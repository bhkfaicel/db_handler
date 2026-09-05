#!/usr/bin/env bash
# Local / CI test gate (in-memory SQLite, sqlx offline mode).
set -euo pipefail
cd "$(dirname "$0")/.."
export CONFIG_FILE="${CONFIG_FILE:-config.test.json}"
export SQLX_OFFLINE=true
cargo test --locked --features sensors,users,utoipaDoc -- --test-threads=1
cargo test --doc --locked --features sensors,users,utoipaDoc
