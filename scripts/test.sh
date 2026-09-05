#!/usr/bin/env bash
# Local / CI test gate (in-memory SQLite, sqlx offline mode).
set -euo pipefail
cd "$(dirname "$0")/.."

# The test suite links against system SQLite (sqlx), so the dev headers must
# be present. CI containers may ship without them; install on the fly when
# possible instead of failing with a confusing compiler error.
if ! pkg-config --exists sqlite3 2>/dev/null; then
  if command -v apt-get >/dev/null 2>&1 && [ "$(id -u)" -eq 0 ]; then
    apt-get update
    apt-get install -y --no-install-recommends libsqlite3-dev
  elif ! [ -e /usr/include/sqlite3.h ]; then
    echo "error: SQLite dev files not found and cannot install libsqlite3-dev (need root + apt-get)" >&2
    exit 1
  fi
fi

export CONFIG_FILE="${CONFIG_FILE:-config.test.json}"
export SQLX_OFFLINE=true
cargo test --locked --features sensors,users,utoipaDoc -- --test-threads=1
cargo test --doc --locked --features sensors,users,utoipaDoc
