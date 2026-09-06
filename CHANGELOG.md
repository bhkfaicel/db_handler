# Changelog

All notable changes to this project will be documented in this file.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

### Breaking

- The previously no-op `sensors` and `users` cargo features are now real
  gates (APPLIED): a default-features build compiles only the core modules
  (`common`, `db`, `drives`). The `sensors` feature gates `sensors`,
  `history`, `keys` and `nonces` (a single dependency closure), and the
  `users` feature gates `users` and `refresh_token`. Dependents must keep
  requesting the features they use: `bridge` (path dependency) is
  validated directly and already requests `sensors`; `backend` and
  `backend_grpc` request `sensors`/`users` and are covered at their next
  version bump. The dead `models` feature is removed alongside (nothing
  gated it and no dependent requests it).
- `ContactSensorHistoryService::new` now takes the InfluxDB bucket name
  alongside the client (`new(client, bucket)`): history writes target the
  bucket configured in the configuration file instead of a name hardcoded
  in source. Callers obtain the value from `Connection::history_db_bucket`
  (filled in by `Connection::new` from configuration); no dependent
  constructs the service directly.
- The crate-internal single-implementation history constructor traits are
  removed (APPLIED): the `db::history` service/repository constructor-trait
  modules and their sole implementations are deleted, and
  `ContactSensorHistoryService` and its repository are now built through
  their inherent `new` constructors. Behavior is unchanged; the traits were
  unreachable from outside the crate and no dependent used them. The
  remaining planned interface changes (repository error-type unification)
  are still to come; all breaking changes from 0.2.x are released together
  as **0.3.0**, never silently inside a 0.2.x patch.
- The `common::prelude` path is removed: the foundation `common` module no
  longer re-exports anything from the higher `db` layer. The main
  `db_handler::prelude` re-exports `Service` and
  `map_sqlx_to_common_error` directly from `db::primary::service`, so
  `use db_handler::prelude::*;` keeps working unchanged. No dependent
  imported `db_handler::common::prelude` directly (verified across
  `bridge`, `backend` and `backend_grpc`), so no deprecation alias is
  kept.
- The `common::helpers::controller` module is deleted: password hashing
  and verification are user-authentication domain logic and now live in
  the `users` module (crate-internal, not part of the public API). Code
  that reached for
  `db_handler::common::helpers::controller::hash_password` must hash
  through its own user-domain stack or the `bcrypt` crate directly; the
  `backend` test helper (`backend/src/helpers/test/test_db.rs`) imports
  this exact path and must migrate to hashing on its side at its next
  `db_handler` version bump.
- The per-domain pagination models are unified into one generic type
  (APPLIED): `sensors::model::PaginationParams` and
  `users::model::PaginationParams` are now type aliases of the new public
  generic `db::primary::models::PaginationParams<K>` instantiated with
  their typed sort keys (`SensorSortKey`, `UserSortKey`). The stringly
  crate-internal duplicate and the field-by-field conversions in
  `SensorService::find_by_string` and `UserService::find_by_string` are
  gone: services forward the typed parameters unchanged. Callers keep
  constructing the same fields (`skip`, `nbr`, `sort_key`, `sort`,
  `searched_value`) with the same typed sort-key enums; only code that
  named the struct type itself (e.g. `impl` blocks or explicit paths to
  `db::primary::models::PaginationParams`) must reference the new generic.
  The core test modules exercising the pagination infrastructure carry the
  same feature gate as the infrastructure itself, so test compilation with
  default features (where the gated items do not exist) skips them.

### Added

- Public generic pagination model `db::primary::models::PaginationParams<K>`
  where `K: AsRef<str> + Display + Serialize` is the sort-key type: domains
  pass their typed sort-key enum, plain `String` works where no typed key
  exists. Under the `utoipaDoc` feature it renders the sort key as a
  string schema/parameter without imposing any utoipa bound on `K`.

- Optional `bucket_name` configuration field: names the InfluxDB bucket
  that sensor history writes target. It defaults to `test_bucket`, so
  existing deployments and test configuration files stay valid without any
  edit; production deployments rename the bucket in their configuration
  file instead of in code. The default is pinned by a dedicated
  configuration test, and `config.example.json` plus the README document
  the field.
- Structured unique-violation handling in the shared `CommonError`: the new
  `UniqueViolation { constraint }` variant carries the violated constraint
  identity (for example `keys.mac`, or a composite column list such as
  `nonces.value, nonces.mac`) instead of forcing callers to parse error
  message text. SQLite reports the constraint name only through the error
  message, so a single central extraction function decodes it in one
  tested place, with unit tests pinning well-formed single-column and
  composite reports, non-unique constraint messages, and malformed
  reports. The shared `map_sqlx_to_common_error` conversion point performs
  the detection, replacing per-service message sniffing as modules
  migrate.
- The `sensors::contact::model` module is public, consistent with every
  sibling model module, so external callers can name the `ContactSensor`
  type that the public contact service returns. Additive change: the
  previous crate-internal visibility made the type unreachable from
  outside the crate.
- Minimal behavioral tests for the data models that previously shipped
  without any: the user models (constructor plus serde round-trip, and the
  typed user sort key against its lowercase wire format), the key, nonce,
  refresh token and historical key models (serde round-trip on
  representative values), and the contact sensor view (construction from a
  wrapped sensor plus its accessors). Each model module now follows the
  project test layout with a `tests/` directory next to its source file.

### Changed

- The Docker test runner now mirrors the standard test gate: the compose
  test command runs plain `cargo test` with the same flags as
  `scripts/test.sh` (`--locked --features sensors,users,utoipaDoc`,
  single-threaded, plus the doc-test pass) followed by a second pass with
  `--run-ignored all` that executes the InfluxDB-backed ignored tests; the
  redundant `cargo-nextest` runner and its image-build install step are
  removed. The dead `TIMESERIES_DB_URL`/`TIMESERIES_DB_ORG`/
  `TIMESERIES_DB_TOKEN` environment variables are dropped from
  `docker-compose.test.yml` (the test configuration has always been read
  from the JSON file pointed to by `CONFIG_FILE`) and the Docker test
  documentation no longer mentions them.
- The InfluxDB bucket that sensor history writes target now comes from the
  configuration file instead of a constant hardcoded in source: the
  `Connection` constructor carries the configured bucket alongside the
  time-series client, and it travels down the existing construction chain
  (`ContactSensorService` -> `ContactSensorHistoryService` -> its
  repository) to the write call. The single-variant `BucketName` enum that
  hid the constant is gone; the write path is one configuration-sourced
  string, so deployments rename the bucket by editing configuration, never
  code. Writes keep targeting `test_bucket` when the configuration omits
  the field (see the added optional `bucket_name` field).
- The history/keys module follows the same structured error flow:
  `HistoricalKeyRepository` methods return the shared `CommonError` instead
  of raw `sqlx::Error`, and `HistoricalKeyService` maps failures to
  `HistoricalKeyError` through the existing transparent `Common` variant, so
  the local sqlx-to-domain mapper helper is deleted. A duplicate `value`
  (the column is UNIQUE in the schema) surfaces as the structured
  `UniqueViolation` variant carrying the constraint identity reported by
  SQLite; all other database failures pass through unchanged. No public
  error variant changed; all other historical-key behavior is unchanged.
- The refresh_token module follows the same structured error flow:
  `RefreshTokenRepository` methods return the shared `CommonError` instead
  of raw `sqlx::Error`, and `RefreshTokenService` maps failures to
  `RefreshTokenError` through a typed constraint guard on the structured
  `UniqueViolation` constraint name (`refresh_tokens.token`), so the local
  message-string mapper helper is deleted. Duplicate-token detection keeps
  surfacing as the public `RefreshTokenError::AlreadyExists` variant, now
  pinned by a new typed service test asserting the variant instead of
  message text; every other database failure passes through unchanged in
  the transparent `Common` variant. No public error variant changed; all
  other refresh-token behavior is unchanged.
- The nonces module follows the same structured error flow:
  `NonceRepository` methods return the shared `CommonError` instead of raw
  `sqlx::Error`, and `NonceService` maps failures to `NonceError` through
  the existing conversion, so the local sqlx-to-domain mapper helper is
  deleted. Duplicate-nonce detection surfaces as the structured
  `UniqueViolation` variant (constraint identity as reported by SQLite for
  the violated index) carried by the transparent `Common` variant of
  `NonceError`: new tests pin the typed behavior at both the repository and
  the service level, and the former raw-sqlx duplicate/foreign-key
  assertions now target the structured variants with the original violation
  kinds preserved. No public error variant changed; all other nonce
  behavior is unchanged.
- The users module follows the same structured error flow:
  `UserRepository` methods return the shared `CommonError` instead of
  `anyhow` errors, and `UserService` maps failures to `UserError` without
  parsing error message text. Duplicate-email detection uses the structured
  `UniqueViolation` constraint name (`users.email`): new tests pin the
  constraint SQLite actually reports at the repository level and the typed
  `UserError::EmailAlreadyExists` variant at the service level. The former
  message-string mapper, the dead email-extraction helper and the anyhow
  downcast boilerplate are deleted; the repository-level duplicate-email
   assertion now targets the typed variant instead of matching message text.
   All other user behavior is unchanged.
- Password hashing moved out of the data-access layer into the user
  service: `UserService::update_by_id` now bcrypt-hashes the clear password
  before calling the repository, and the crate-internal
  `UserRepository::update_by_id` persists the hash it is handed (its
  password parameter carries an already-hashed value). A hashing failure
  on the update path now surfaces as the dedicated
  `UserError::PasswordHashing` variant, consistent with user creation,
  instead of a generic operation error. The synchronous call semantics are
  unchanged.
- The sensors module now uses the structured error flow: `SensorRepository`
  and `ContactSensorRepository` methods return the shared `CommonError`
  instead of `anyhow`/`sqlx` errors, and `SensorService` maps failures to
  `SensorError` without parsing error message text. The update business
  rules (a status may not be set back to `InProgress`; an update must
  change at least one field) moved from the repository into `SensorService`,
  which validates them before any SQL is sent and returns the existing
  typed variants. Duplicate-MAC detection uses the structured
  `UniqueViolation` constraint name (`sensors.mac`): a new service test pins
  `SensorError::DuplicateMac`, and the former message-string mappers and
  MAC extraction from error text are deleted. The two former repository
  tests that asserted the update rules through bare `is_err()` moved to the
  service tests with typed variant assertions; all other sensor behavior
  is unchanged.
- The keys module is the first converted to the structured error flow:
  `KeyRepository` methods now return the shared `CommonError` instead of
  raw `sqlx::Error`, and `KeyService` maps failures to `KeyError` through
  the existing conversion without constructing error variants by hand. A
  new repository test pins that inserting a duplicate key value surfaces
  as `CommonError::UniqueViolation` carrying the `keys.value` constraint
  name. No existing test behavior changed.
- The time-series connection factory (`get_time_series_database_client`)
  now returns `anyhow::Result` instead of the sensor history domain error
  type: connection infrastructure must not depend on a domain error type,
  and the configuration loader already reports failures as `anyhow::Error`,
  so the original error chain is preserved instead of being re-wrapped into
  a formatted string. Crate-internal change (the factory is not part of the
  public API); the public `Connection::new` signature is unchanged. This
  decouples the `db` tree from the sensors module and prepares it to
  compile without the `sensors` cargo feature.
- Test placement now follows the project rule (each module = source file
  plus a `tests/` directory containing `mod.rs`): the 17 modules that kept
  their tests in a single `tests.rs` file were converted to a `tests/`
  directory with no code change (Rust module declarations resolve both
forms). The two empty one-line placeholder test files in the
   `db::history` tree (zero tests, fake coverage) are removed with the
   modules they nominally covered (see the deleted `BucketName` enum and
   history constructor traits above); the default-bucket behavior is
   pinned by the dedicated configuration test (see the added optional
   `bucket_name` field). No test was weakened or skipped by this change.
- Sort-key validation in the crate-internal pagination helper
  (`get_paginated_results_by_string`) no longer issues a per-call
  `PRAGMA table_info` round-trip: the sort key is checked against the
  model's column map (generated at compile time by `db_derive`), which is
  authoritative because the schema is fixed once the boot migrations have
  run. The generated SELECT column list is now sorted, so query column
  order is deterministic instead of following `HashMap` iteration order.
  One behavioral consequence: sorting by a column that exists in the table
  but is absent from the model's mapped columns is now rejected instead of
  being silently allowed, and the dedicated core test pins that rejection.
  A dedicated core test passes a sort key absent from the model's columns
  (`not_a_column`) over an otherwise valid model and asserts the explicit
  "Invalid sort key" error, directly proving the `ORDER BY` whitelist.
  The now-unused PRAGMA-based column introspection helper is deleted
  together with its core-only dead-code warning.

### Removed

- The dead SQLite `.pragma("key", ...)` from the primary database connect
  options: no SQLCipher backend exists anywhere in the dependency graph, so
  SQLite silently ignored the pragma and a configured key would have been
  a false guarantee. No key option is introduced; at-rest protection of the
  SQLite file (file permissions, system-level encryption) is the
  deployer's responsibility, now documented in the README configuration
  section.
- The single-variant `BucketName` enum whose `Display` rendered the
  hardcoded `test_bucket` value that history writes used to target: it is
  replaced by the configuration-sourced bucket name threaded through the
  construction chain (see the changed and added entries above).
- Manifest cleanups: the unused sqlx `migrate` feature (migration tracking
  is the crate's own tested ledger and no `sqlx::migrate!` call exists),
  the duplicate `futures` entry under `[dev-dependencies]` (already a
  regular dependency), and the commented-out alternative `db_derive` path
  dependency line.
- Dead `sensor_history` module tree (model, repository, service, traits,
  about 1,300 lines): its module declaration was commented out, the tree was
  never compiled, and it duplicated the live `history/sensors`
  implementation, including a superseded time-series backend abstraction.
  The `async-trait` dependency, used only by that dead tree, is removed
  from `Cargo.toml` and `Cargo.lock`. Two dead commented module
  declarations are removed from `src/lib.rs`. No compiled code path
  changes; the full gate set passes unchanged.
- Dead `tests1/` suite and `src/tests.sh` runner: `tests1/` was never a
  Cargo test target (not compiled, not tracked) and its files imported an
  API that no longer exists. `src/tests.sh` was a divergent duplicate test
  runner (nextest, no strict error handling) inside `src/`; the canonical
  test gate is `scripts/test.sh`. The stale `tests1/*` entry is removed
  from `.gitignore`.

### Fixed

- Test code cleanup so the full lint gate passes on all targets
  (`cargo clippy --locked --all-targets --features sensors,users,utoipaDoc
  -- -D warnings`): compound assignment operators replace manual
  add/subtract operations, `clone()` calls removed on `Copy` key arrays,
  shorthand struct field initialization, `assert!` instead of
  `assert_eq!(..., true/false)`, unit-value `let` bindings removed,
  fixed-size arrays instead of `vec!` where no heap vector is needed,
  needless borrows and `to_string()` calls inside `format!` arguments
  removed. Test behavior is unchanged; the suite passes with the same
  results as before the cleanup.
- Error attribution and swallowed-error fixes. The contact-sensor service
  no longer misreports database/operation failures as "not found": the
  repository signals a missing row through `Ok` (an `Option` or an upsert
  row count), so every `Err` is a real database/operation failure and now
  propagates as the structured common error instead of being remapped to
  `ContactSensorError::NotFound`. `KeyService::add` no longer swallows a
  failed DELETE inside its transaction behind a warning: the absence of a
  previous key was already a success (zero rows deleted), so the
  warn-and-continue branch is removed and real failures abort the
  transaction. Contact-sensor history writes report out-of-range
  timestamps as a structured invalid-format error instead of silently
  recording epoch zero, for both the sensor-side timestamp field and the
  data point timestamp. New unit tests pin each behavior.
- Stale documentation corrected so it matches the actual code. The
  time-series connection docs described `TIMESERIES_DB_*` environment
  variables the code never read (configuration is a JSON file through
  `CONFIG_FILE`) and referenced a nonexistent example path; they now
  document the real configuration fields. The primary connection docs
  named a `db_url` configuration field (actual field: `primary_db_url`).
  The sensor model docs referenced a `HistoryCapable` trait that does not
  exist; they now describe the real mechanism (exhaustive `ProductType`
  match guarding new sensor types, `TypedSensorTrait` delegation for the
  accessors). The documentation examples of the shared error conversion
  and the `Service` trait now compile in a core-only build too: the
  conversion example sticks to core types and the `Service` examples are
  feature-conditional.
- Library code logs migration progress with `log::info!` instead of
  printing to standard output, so applications decide where the output
  goes.
- Remaining French source comments translated to English (the
  repository note about `query_as` versus `query!` and the test comments
  in the keys module).
- French SQL comments in the migration files translated to English
  (table and index descriptions in the three oldest migrations). A note
  documents the intentional ordering: the refresh-token migration sorts
  before the users-table migration, which is safe because SQLite
  resolves foreign keys lazily at statement time. Filenames and SQL
  statements are unchanged (the migration ledger is keyed by filename).
- The migrations test index pointed at the pre-reorganization single-file
  location (`src/db/primary/tests.rs`); it now points at the actual
  `tests/mod.rs` layout.

## [0.2.0] - 2026-09-06

### Added

- `SensorService::list_by_status` (+ repository counterpart): lists every
  sensor currently holding a given status. Gateway boot logic uses it to
  detect commissioning runs left unfinished (`InProgress`) and grant them a
  bounded pairing retry opportunity.
- Forgejo CI workflows (fmt, clippy, cargo-deny, machete, version sync,
  `scripts/test.sh` gate) and supporting scripts under `scripts/` and
  `.forgejo/workflows/`.

### Changed

- `ContactSensorService::set_history` integration tests are marked
  `#[ignore]` (require InfluxDB). CI runs `scripts/test.sh` (SQLite only);
  Docker test runner executes ignored tests via `nextest --run-ignored all`.
- Forgejo workflows are now thin callers of the shared `faicel/central_ci`
  reusable workflows (checks, tagging, publishing, PR direction), matching
  the other aligned repositories; the local shared gate workflow was
  removed. `scripts/test.sh` installs `libsqlite3-dev` on the fly when the
  CI image lacks SQLite dev files.
- Local path dependency on the derive crate restored under its real package
  name (`db_derive`, crate at `../derive`) so the workspace resolves again.
- `cargo fmt` applied to `src/db/primary/tests.rs` (formatting drift that
  would fail the CI format gate).
- Gate cleanup on the inherited codebase: 36 clippy lints fixed (mostly
  blank lines after doc comments), unused dependencies removed
  (`dotenvy`, `env_logger`, `once_cell`), `db_derive` path dependency now
  declares its version (cargo-deny wildcard ban), MPL-2.0 allowed and
  `db_derive` license clarified in `deny.toml`, and the advisories flowing
  exclusively through the `influxdb2 0.5 -> reqwest 0.11 -> rustls 0.21`
  stack are documented as accepted risks (no upgrade path until influxdb2
  moves to reqwest 0.12). Transitive dependencies refreshed via
  `cargo update` (bcrypt 0.19.3 resolves the `bcrypt::verify` panic
  advisory).

### Fixed

- Deleting a sensor no longer fails with a foreign key constraint error
  when it owns `open_close_sensors` state: the initial schema declares
  `ON DELETE CASCADE` on both foreign keys of that table, so removal
  cascades through state rows, keys and nonces on every deletion path.
- Test-suite hygiene: the config error-path test no longer deletes the
  global `CONFIG_FILE` variable, which poisoned every later test in a
  serialized full-suite run.

## [0.1.0]

- Initial published release.
