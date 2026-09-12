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
- `users` – User domain (Argon2id login) and `refresh_token` domain (REST refresh rotation + gRPC session API); see [Authentication and session tokens](#authentication-and-session-tokens)
- `utoipaDoc` – OpenAPI documentation (adds `utoipa` dependency)

Build with the features you need, for example:

```bash
cargo build --features "sensors,users,utoipaDoc"
```

## Authentication and session tokens

Everything in this section requires the `users` feature, which gates the
`users` domain together with the `refresh_token` domain as one closure.

**Passwords (breaking in 0.4.0).** New and reset passwords are hashed with
Argon2id (standard PHC strings, unique OS-CSPRNG salt per hash, default
RustCrypto parameters) and must be 8–128 bytes; anything else is rejected
with a typed error on creation/reset, and login attempts with out-of-range
passwords fail uniformly before any hashing work. **There is no legacy
fallback**: bcrypt hashes are intentionally unsupported in 0.4.0, every
pre-existing account fails closed until its password is reset through an
approved provisioning/reset path. Email stays the sole login identifier and
lookups are byte-exact: emails are never trimmed, lowercased or case-folded.
Login runs exactly one Argon2id verification (unknown emails burn the same
work against a fixed dummy hash, so response timing cannot reveal whether an
account exists).

**Password reset and exact lookup (additive).** `UserService` gains
`get_by_email(email) -> Result<Option<User>>` — one bound, byte-exact SQL
equality lookup (`WHERE email = ?`): no trimming, lowercasing or case
folding, no pagination, no in-memory scan; the returned public model carries
no password hash. It also gains
`reset_password_revoking_tokens(email, new_password) -> Result<User>`: the
new password is validated (8–128 bytes, else the typed `InvalidInput`)
and Argon2id-hashed before the database transaction opens, then ONE
transaction performs the exact-email lookup, the password update and the
deletion of every `refresh_tokens` row of the user. That deletion is
deliberately unscoped by token domain: a password reset is a user-lifecycle
event that must invalidate BOTH the REST refresh tokens and the gRPC
sessions of the account, unlike every domain-scoped sweep above. Any
failure — lookup, update, deletion or commit — rolls the whole transaction
back: the old password keeps authenticating and every credential row
survives. An unknown email returns the typed `UserError::NotFoundByEmail`
and writes nothing; that typed answer is safe to expose on this
provisioning surface, because unlike the login path it does not have to
hide account existence from callers that explicitly target one account.

**gRPC sessions (additive).** `RefreshTokenService` gains `add_session`,
`validate_session`, `revoke_session`, `revoke_all_sessions` and
`cleanup_expired_sessions`. The library never generates tokens: the caller
supplies an opaque value that MUST carry at least 256 bits of OS-CSPRNG
unpredictability (empty, oversized or inverted-window values are rejected
before any SQL). Validation is **non-consuming** — it may repeat per request
and fails closed (`None` for unknown, expired and revoked alike, with no way
to distinguish them). `revoke_session` and `revoke_all_sessions` are single
atomic idempotent statements (logout and logout-everywhere); expired session
rows leave only through the explicit `cleanup_expired_sessions` sweep.

**Token-domain separation.** Sessions are stored under the tagged digest
`grpc-session:<sha256(raw)>` while REST refresh tokens keep their bare
64-hex digests. Every refresh SQL path matches only refresh-domain rows and
every session path only session-domain rows, so no refresh rotation, reuse
purge or deletion can consume or revoke a gRPC session (nor the reverse),
and the same raw value can exist in both domains as two distinct rows.
**No SQL migration was needed**: the tagged value fits the existing
`TEXT NOT NULL UNIQUE` column and existing refresh digests stay valid.

**Coordinated rollout.** Publish 0.4.0 first. Then `backend/`
(`rest_api_server`) bumps to it, adapts its email login and re-verifies REST
refresh rotation in its own mini-cycle. Then `backend_grpc` enables `users`
and resumes its gated authentication work, consuming
`UserService::get_by_email` and `UserService::reset_password_revoking_tokens`
instead of composing a password update with a separate revocation call or
filtering paginated listings in memory. **Neither consumer is compatible
with 0.4.0 until its own mini-cycle completes**, and deployment waits for
both.

## License

Personal use only, non-commercial. See [LICENSE](LICENSE).
