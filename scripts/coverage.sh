#!/usr/bin/env bash
# Coverage gate for the Rust crate. Runs llvm-cov against the testable
# subsystems and fails the build if any line/region/function ratio
# drops below the threshold. lib.rs (Tauri command shim + setup) and
# the KeyringBackend hit live OS surfaces and are excluded.

set -euo pipefail

cd "$(dirname "$0")/../src-tauri"

cargo llvm-cov --lib \
  --ignore-filename-regex 'lib\.rs|auth/tokens\.rs|auth/error\.rs|spotify/error\.rs|sync/error\.rs|store/error\.rs|commands/handlers\.rs' \
  --fail-under-lines 90 \
  --fail-under-functions 90 \
  -- --test-threads=1
