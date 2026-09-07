# db_handler

> [!IMPORTANT]
> **The canonical repository for this project lives on Forgejo**: <https://code.bhk-itsolutions.com/homeiot/db_handler.git>.
> This GitHub repository is only a mirror and is **not** the primary git remote.

Rust library for the Home IoT backend: primary database (SQLite), time-series history (InfluxDB 2.x), and domain services (sensors, users, keys, refresh tokens, nonces).

## Features

- **Primary DB**: SQLite with [sqlx](https://github.com/launchbadge/sqlx), migrations, connection pooling
- **History DB**: InfluxDB 2.x client (rustls), sensor history (e.g. contact sensors)
- **Domain**: Sensors (typed models, contact sensors), users (Argon2id), keys, refresh tokens, nonces
- **Optional**: OpenAPI docs via `utoipaDoc` feature

## Prerequisites

- Rust 1.75+ (2021 edition)
- SQLite
- InfluxDB 2.x (for history features; optional for tests via Docker)
- **Dependency**: `db_derive` — not on crates.io; use path, Git, or the Forgejo package registry (see [Building](#building)).

## Building

`db_derive` is required.

**Git dependency:**  
In `Cargo.toml`, replace the path with one of:

```toml
# Default branch (e.g. main)
db_derive = { git = "https://code.bhk-itsolutions.com/homeiot/db_derive.git" }

# Pin to a tag (recommended for reproducible builds)
db_derive = { git = "https://code.bhk-itsolutions.com/homeiot/db_derive.git", tag = "v0.1.0" }

# Or pin to a specific commit
db_derive = { git = "https://code.bhk-itsolutions.com/homeiot/db_derive.git", rev = "abc1234" }
```

With Git, `tag` and `rev` are **exact** (one tag = one version). For **version ranges** (e.g. accept all `0.2.x`), use the Forgejo package registry below.

**Option C — Forgejo package registry (version ranges):**  
If `db_derive` is published on your Forgejo instance, you can use Cargo’s package registry and semver ranges.

1. **One-time setup** — in `~/.cargo/config.toml` add (replace URL and owner if needed):

```toml
[registries.forgejo]
index = "sparse+https://code.bhk-itsolutions.com/api/packages/homeiot/cargo/"
```

2. **In `Cargo.toml`**, use the registry and a version requirement:

```toml
db_derive = { version = "0.2", registry = "forgejo" }
```

This uses Cargo semver (e.g. `"0.2"` ⇒ `^0.2`, so any `0.2.x`). For publishing `db_derive` to Forgejo, see [Forgejo Cargo registry](https://forgejo.org/docs/latest/user/packages/cargo/).

Then:

```bash
cd db_handler
cargo build --features "sensors,users,utoipaDoc"
```

## Configuration

The application reads configuration from a JSON file. Set the path via the `CONFIG_FILE` environment variable (default: `config.json` in the current directory).

**Example config** (copy from `config.example.json` and fill in):

```json
{
  "primary_db_url": "sqlite:data/home_iot.db",
  "history_db_url": "http://localhost:8086",
  "history_db_org": "my-org",
  "history_db_token": "your-influxdb-token",
  "bucket_name": "my-bucket"
}
```

- `primary_db_url`: SQLite connection URL (e.g. `sqlite:path/to/file.db` or `:memory:` for tests).
- `history_db_url`: InfluxDB 2.x URL (e.g. `http://localhost:8086`).
- `history_db_org`: InfluxDB organization name.
- `history_db_token`: InfluxDB API token.
- `bucket_name` (optional): InfluxDB bucket that history writes target. Defaults to `test_bucket`; set it to your production bucket per deployment.

Do not commit real credentials. Use `config.example.json` as a template and keep `config.json` or `config.local.json` local (they are in `.gitignore`).

**At-rest protection of the SQLite database is the deployer's responsibility**: the primary database file is plain SQLite (no encryption backend is bundled). Restrict file permissions (e.g. user-only access to the data directory) and/or rely on system-level encryption (LUKS, FileVault, BitLocker) as appropriate for your deployment.

## Running tests

Run tests in Docker (recommended; includes InfluxDB). If `Cargo.toml` uses the Forgejo registry for `db_derive`, set a read-only token (e.g. in `.env`): `CARGO_REGISTRIES_FORGEJO_TOKEN=your_token`. See `.env.example`.

```bash
./scripts/test-docker.sh
```

See [README.DOCKER.md](README.DOCKER.md) for options (`--keep`, manual commands, etc.).

**Troubleshooting (Docker tests):** If the 4 `set_history` tests fail with InfluxDB error **"no space left on device"**, Docker has run out of disk space. Free space and retry: `docker system prune -a` (removes unused images/containers), and/or increase the Docker Desktop virtual disk size in settings.

To run tests locally without Docker (in-memory SQLite, no InfluxDB):

```bash
./scripts/test.sh
```

This runs the default suite with `--test-threads=1` (four `set_history` tests are
`#[ignore]`). To run InfluxDB integration tests locally:

```bash
cargo test -- --ignored set_history
```

Or use `./scripts/test-docker.sh` for the full suite including InfluxDB.

## Project layout

- `src/` – Library code
  - `db/` – Primary DB connection, migrations; history DB connection
  - `sensors/` – Sensor models, repository, service; contact sensor and history
  - `history/` – InfluxDB-backed history (sensors, keys)
  - `users/`, `keys/`, `refresh_token/`, `nonces/` – Domain services
  - `common/` – Config, helpers
- `migrations/` – SQLite migrations (sqlx)
- `scripts/` – Test and Docker helper scripts
- `config.test.json` – Test config (safe to commit; uses test token)
- `config.example.json` – Example config (no secrets)

## Features (cargo)

- `sensors` – Sensor and contact sensor support
- `users` – User and auth-related services
- `utoipaDoc` – OpenAPI documentation (adds `utoipa` dependency)
- `models` – Optional model/API extras

Build with the features you need, for example:

```bash
cargo build --features "sensors,users,utoipaDoc"
```

## License

Personal use only, non-commercial. See [LICENSE](LICENSE).
