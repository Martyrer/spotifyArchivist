#!/usr/bin/env bash
# Fails if any committed sqlx migration file is modified or deleted.
#
# sqlx checksums every migration and refuses to open a user database whose
# stored checksum no longer matches the file ("migration N was previously
# applied but has been modified"). Once a migration is on main it must stay
# byte-identical forever. Adding new NNNN_*.sql files is fine.
#
# Compares the working diff against a base ref (BASE env var, default origin/main).

set -euo pipefail

MIG_DIR="src-tauri/migrations"
BASE="${BASE:-origin/main}"

# Resolve a usable base ref; fall back to merge-base with HEAD.
if ! git rev-parse --verify --quiet "$BASE" >/dev/null; then
  echo "check-migrations: base ref '$BASE' not found, skipping" >&2
  exit 0
fi
MERGE_BASE="$(git merge-base "$BASE" HEAD 2>/dev/null || echo "$BASE")"

# Status letters: M=modified, D=deleted, R=renamed. Any of these on an
# existing migration changes or removes its checksummed content.
violations="$(git diff --name-status --diff-filter=MDR "$MERGE_BASE" HEAD -- "$MIG_DIR" || true)"

if [ -n "$violations" ]; then
  echo "ERROR: committed migration files were modified, renamed, or deleted." >&2
  echo "Migrations are immutable once on main — add a new NNNN_*.sql instead." >&2
  echo >&2
  echo "$violations" >&2
  exit 1
fi

echo "check-migrations: OK (no committed migrations changed)"
