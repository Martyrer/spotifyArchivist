use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{AssertSqlSafe, Pool, Sqlite};
use std::path::Path;
use std::str::FromStr;

use super::error::Result;
use super::models::*;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct Store {
    pool: Pool<Sqlite>,
}

impl Store {
    pub async fn open(path: &Path) -> Result<Self> {
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await?;
        MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn open_in_memory() -> Result<Self> {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        MIGRATOR.run(&pool).await?;
        Ok(Self { pool })
    }

    pub fn pool(&self) -> &Pool<Sqlite> {
        &self.pool
    }

    pub async fn upsert_source(
        &self,
        kind: SourceKind,
        spotify_id: Option<&str>,
        name: &str,
    ) -> Result<i64> {
        let kind_str = match kind {
            SourceKind::LikedSongs => "liked_songs",
            SourceKind::Playlist => "playlist",
        };
        let sid = spotify_id.unwrap_or("__self__");
        let row: (i64,) = sqlx::query_as(
            r#"
            INSERT INTO sources (kind, spotify_id, name, enabled)
            VALUES (?, ?, ?, 1)
            ON CONFLICT (kind, spotify_id) DO UPDATE SET name = excluded.name
            RETURNING id
            "#,
        )
        .bind(kind_str)
        .bind(sid)
        .bind(name)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    pub async fn list_sources(&self) -> Result<Vec<Source>> {
        let rows = sqlx::query_as::<_, Source>(
            r#"SELECT id, kind, spotify_id, name, enabled FROM sources ORDER BY id"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn delete_source(&self, id: i64) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM syncs WHERE source_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM memberships WHERE source_id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        let n = sqlx::query("DELETE FROM sources WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        tx.commit().await?;
        if n == 0 {
            return Err(super::error::StoreError::SourceNotFound(id));
        }
        Ok(())
    }

    pub async fn set_source_enabled(&self, id: i64, enabled: bool) -> Result<()> {
        let n = sqlx::query(r#"UPDATE sources SET enabled = ? WHERE id = ?"#)
            .bind(enabled)
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if n == 0 {
            return Err(super::error::StoreError::SourceNotFound(id));
        }
        Ok(())
    }

    pub async fn upsert_track(&self, t: &Track) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO tracks (id, uri, name, artists, album, first_seen_at)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO UPDATE SET
                uri = excluded.uri,
                name = excluded.name,
                artists = excluded.artists,
                album = excluded.album
            "#,
        )
        .bind(&t.id)
        .bind(&t.uri)
        .bind(&t.name)
        .bind(&t.artists)
        .bind(&t.album)
        .bind(&t.first_seen_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn upsert_membership(&self, m: &Membership) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO memberships (source_id, track_id, added_at, position, is_removed, pending_vanish)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT (source_id, track_id) DO UPDATE SET
                added_at = excluded.added_at,
                position = excluded.position,
                is_removed = excluded.is_removed,
                pending_vanish = excluded.pending_vanish
            "#,
        )
        .bind(m.source_id)
        .bind(&m.track_id)
        .bind(&m.added_at)
        .bind(m.position)
        .bind(m.is_removed)
        .bind(m.pending_vanish)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_rows(&self, source_id: i64, filter: MembershipFilter) -> Result<Vec<Row>> {
        let base = r#"
            SELECT m.source_id, m.track_id, t.uri, t.name, t.artists, t.album,
                   m.added_at, m.position, m.is_removed, m.pending_vanish
            FROM memberships m
            JOIN tracks t ON t.id = m.track_id
            WHERE m.source_id = ?
        "#;
        let where_extra = match filter {
            MembershipFilter::All => "",
            MembershipFilter::Present => " AND m.is_removed = 0",
            MembershipFilter::Removed => " AND m.is_removed = 1",
        };
        let sql = format!("{base}{where_extra} ORDER BY m.position");
        let rows = sqlx::query_as::<_, Row>(AssertSqlSafe(sql))
            .bind(source_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let r: Option<(String,)> = sqlx::query_as(r#"SELECT value FROM settings WHERE key = ?"#)
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(r.map(|x| x.0))
    }

    pub async fn put_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO settings (key, value) VALUES (?, ?)
            ON CONFLICT (key) DO UPDATE SET value = excluded.value
            "#,
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Read a typed setting, returning `default` when the key is missing and
    /// an `InvalidSetting` error when the stored string fails to parse. One
    /// recovery policy across every kv-bag setting (was previously 3 different
    /// ad-hoc fallbacks per accessor).
    async fn get_typed<T: std::str::FromStr>(&self, key: &str, default: T) -> Result<T> {
        let Some(v) = self.get_setting(key).await? else {
            return Ok(default);
        };
        v.parse()
            .map_err(|_| super::error::StoreError::InvalidSetting {
                key: key.into(),
                value: v,
            })
    }

    async fn put_typed<T: std::fmt::Display>(&self, key: &str, value: T) -> Result<()> {
        self.put_setting(key, &value.to_string()).await
    }

    pub async fn sync_interval_hours(&self) -> Result<u32> {
        Ok(self
            .get_typed::<u32>("sync_interval_hours", 6)
            .await?
            .clamp(1, 24))
    }

    pub async fn set_sync_interval_hours(&self, h: u32) -> Result<()> {
        self.put_typed("sync_interval_hours", h.clamp(1, 24)).await
    }

    pub async fn unseen_losses(&self) -> Result<u32> {
        self.get_typed::<u32>("unseen_losses_total", 0).await
    }

    pub async fn add_unseen_losses(&self, n: u32) -> Result<u32> {
        let next = self.unseen_losses().await? + n;
        self.put_typed("unseen_losses_total", next).await?;
        Ok(next)
    }

    pub async fn clear_unseen_losses(&self) -> Result<()> {
        self.put_typed("unseen_losses_total", 0u32).await
    }

    pub async fn is_onboarded(&self) -> Result<bool> {
        Ok(self.get_typed::<u32>("onboarded", 0).await? != 0)
    }

    pub async fn set_onboarded(&self, v: bool) -> Result<()> {
        self.put_typed("onboarded", if v { 1u32 } else { 0 }).await
    }

    /// Wipe every row of user data and restore settings to first-run defaults.
    /// Run in one transaction so a partial failure cannot leave a half-cleared db.
    pub async fn reset(&self) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        for table in ["syncs", "memberships", "sources", "tracks"] {
            sqlx::query(AssertSqlSafe(format!("DELETE FROM {table}")))
                .execute(&mut *tx)
                .await?;
        }
        sqlx::query("DELETE FROM settings")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            r#"INSERT INTO settings (key, value) VALUES
                ('sync_interval_hours', '6'),
                ('consecutive_failures', '0'),
                ('unseen_losses_total', '0')"#,
        )
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }
}
