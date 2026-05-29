# Project conventions

## Database migrations are immutable once committed

`src-tauri/migrations/*.sql` files are applied by `sqlx::migrate!` and their
content is checksummed. sqlx stores each checksum in the user's
`_sqlx_migrations` table and **refuses to start** if a previously-applied
migration's file no longer matches ("migration N was previously applied but has
been modified").

Rules:

- **Never edit a migration file that has been committed to `main`.** Not the SQL,
  not whitespace, not comments. The checksum must stay byte-identical forever.
- To change the schema or seed data, **add a new** `NNNN_description.sql` file
  with the next number. Use `INSERT OR IGNORE` / `ALTER TABLE` so it is safe on
  databases that already have prior state.
- Settings rows are read with a default (see `get_typed(key, default)` in
  `src-tauri/src/store/repo.rs`), so a *missing* seed row is harmless — prefer
  relying on the code default over re-seeding in a migration when possible.
- CI enforces this: `scripts/check-migrations.sh` fails the build if any
  committed migration file is modified in a PR. Adding new files is fine.

This protects users' ability to upgrade in place without their local database
failing to open.
