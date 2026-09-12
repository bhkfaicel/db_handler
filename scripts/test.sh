#!/usr/bin/env bash
# Local / CI test gate (in-memory SQLite, sqlx offline mode).
set -euo pipefail
cd "$(dirname "$0")/.."

# SQLx's "sqlite" feature compiles SQLite from its bundled sources, so the
# only real build-time requirement here is a C compiler (the same one the
# TLS/crypto build dependencies already need). Fail early with a clear
# message instead of a confusing linker error deep inside cargo.
if ! command -v cc >/dev/null 2>&1; then
  echo "error: no C compiler (cc) found; the bundled SQLite build requires one" >&2
  exit 1
fi

export CONFIG_FILE="${CONFIG_FILE:-config.test.json}"
export SQLX_OFFLINE=true
cargo test --locked --features sensors,users,utoipaDoc -- --test-threads=1
cargo test --doc --locked --features sensors,users,utoipaDoc
