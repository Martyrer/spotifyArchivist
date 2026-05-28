use std::path::Path;

use serde::Serialize;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::store::{MembershipFilter, Source, Store};

#[derive(Debug, Serialize)]
struct ExportRow<'a> {
    source_id: i64,
    source_name: &'a str,
    track_id: &'a str,
    uri: &'a str,
    name: &'a str,
    artists: serde_json::Value,
    album: &'a str,
    added_at: &'a str,
    position: i64,
    is_removed: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("store error: {0}")]
    Store(#[from] crate::store::StoreError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("source not found: {0}")]
    SourceNotFound(i64),
}

pub async fn export_jsonl(
    store: &Store,
    sources: &[Source],
    path: &Path,
) -> Result<usize, ExportError> {
    let f = File::create(path).await?;
    let mut w = BufWriter::new(f);
    let mut written = 0usize;
    for source in sources {
        let rows = store.list_rows(source.id, MembershipFilter::All).await?;
        for r in rows {
            let artists: serde_json::Value =
                serde_json::from_str(&r.artists).unwrap_or_else(|_| serde_json::json!([]));
            let line = serde_json::to_string(&ExportRow {
                source_id: source.id,
                source_name: &source.name,
                track_id: &r.track_id,
                uri: &r.uri,
                name: &r.name,
                artists,
                album: &r.album,
                added_at: &r.added_at,
                position: r.position,
                is_removed: r.is_removed,
            })
            .expect("ExportRow serializes");
            w.write_all(line.as_bytes()).await?;
            w.write_all(b"\n").await?;
            written += 1;
        }
    }
    w.flush().await?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{Membership, SourceKind, Track};
    use tokio::io::AsyncReadExt;

    async fn seed(store: &Store) -> Source {
        let id = store
            .upsert_source(SourceKind::Playlist, Some("p"), "Mix")
            .await
            .unwrap();
        store
            .upsert_track(&Track {
                id: "t1".into(),
                uri: "spotify:track:t1".into(),
                name: "One".into(),
                artists: r#"[{"id":"a","name":"A"}]"#.into(),
                album: "Alb".into(),
                first_seen_at: "2026-01-01T00:00:00Z".into(),
            })
            .await
            .unwrap();
        store
            .upsert_membership(&Membership {
                source_id: id,
                track_id: "t1".into(),
                added_at: "2026-01-01T00:00:00Z".into(),
                position: 0,
                is_removed: false,
                pending_vanish: false,
            })
            .await
            .unwrap();
        Source {
            id,
            kind: SourceKind::Playlist,
            spotify_id: "p".into(),
            name: "Mix".into(),
            enabled: true,
        }
    }

    #[tokio::test]
    async fn writes_one_jsonl_line_per_membership() {
        let store = Store::open_in_memory().await.unwrap();
        let s = seed(&store).await;
        let dir = tempdir();
        let path = dir.join("out.jsonl");
        let n = export_jsonl(&store, &[s], &path).await.unwrap();
        assert_eq!(n, 1);
        let mut buf = String::new();
        tokio::fs::File::open(&path)
            .await
            .unwrap()
            .read_to_string(&mut buf)
            .await
            .unwrap();
        let line = buf.trim_end();
        assert!(line.contains("\"track_id\":\"t1\""));
        assert!(line.contains("\"source_name\":\"Mix\""));
        assert!(line.contains("\"is_removed\":false"));
        let parsed: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(parsed["artists"][0]["name"], "A");
    }

    #[tokio::test]
    async fn empty_source_list_writes_empty_file() {
        let store = Store::open_in_memory().await.unwrap();
        let dir = tempdir();
        let path = dir.join("out.jsonl");
        let n = export_jsonl(&store, &[], &path).await.unwrap();
        assert_eq!(n, 0);
        let meta = tokio::fs::metadata(&path).await.unwrap();
        assert_eq!(meta.len(), 0);
    }

    fn tempdir() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        p.push(format!("archivist-test-{nanos}"));
        std::fs::create_dir_all(&p).unwrap();
        p
    }
}
