# Changelog

All notable changes to this project will be documented in this file.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

## [0.3.2] - 2026-09-07

### Updated

- Crago.lock

## [0.3.1] - 2026-09-07

### Updated

- README.md

## [0.3.0] - 2026-09-07

### Breaking

- The refresh-token consumption API is replaced by atomic rotation
  (APPLIED): `RefreshTokenService::is_valid` and
  `RefreshTokenService::update_flag_by_token` are removed, along with the
  repository lookup and flag-update methods that only served them
  (`get_by_user_id_and_token`, `update_flag_by_token`). The new
  `RefreshTokenService::rotate(user_id, old_token, new_token, expire_at,
created_at)` consumes the presented token and inserts its replacement in
  ONE database transaction and returns a typed `RotationOutcome`
  (`Rotated(RefreshToken)`, `NotFound`, `Expired`, `ReuseDetected`). The
  former read-then-flag two-step flow left a replay window between the
  validity check and the consumption; a replay of an already-consumed token
  now atomically deletes every token of the user and reports
  `ReuseDetected`. The verified caller (the `backend` refresh controller)
  is migrated to the single `rotate` call; refresh-token issuers keep using
  `add` unchanged.
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
- Refresh tokens are stored as SHA-256 digests, never as plaintext
  (APPLIED): the repository boundary hashes every caller-supplied token
  value before it reaches SQL on every store and lookup path (insertion,
  rotation consumption, rotation classification and replacement insert), so
  a database read alone no longer yields a usable session. The `token`
  field of the returned `RefreshToken` model keeps echoing the
  caller-supplied value; no public getter returns stored digests, and
  callers keep passing the plaintext token string to `add` and `rotate`
  (their signatures are unchanged). **Existing sessions are invalidated:
  every currently issued refresh token stops authenticating after this
  change and all users must log in again.** There is no legacy-row
  migration (a database row holding a plaintext token can no longer match
  its digest); a one-shot legacy-row hashing sweep remains available as a
  follow-up for deployments that want to preserve sessions. Presenting a
  digest read from the database as a token fails (it is hashed again before
  matching), and token issuers MUST supply values with at least 256 bits of
  cryptographic unpredictability (documented contract on
  `RefreshTokenService::add`/`rotate`; the backend's signing-key entropy
  and token uniqueness remain a coordination item to qualify).

### Added

- Optional `history_db_allow_insecure` configuration field (serde default
  false): the history database client now enforces encrypted transport —
  `https://` URLs are always accepted, and plain `http://` is only accepted
  for loopback hosts (`localhost`, `127.0.0.1`, `::1`) unless this flag is
  explicitly set true (for Docker-internal hostnames or private-LAN
  gateways where unencrypted traffic is understood and accepted; traffic
  then carries the bearer token in cleartext on the wire). Any other URL
  scheme, or a malformed URL, is rejected with an actionable error before
  any client is built. URLs are parsed with the `url` crate. The default
  (flag absent) rejects non-loopback `http` — no existing configuration
  silently downgrades. `config.example.json` documents the field, and the
  transport policy is covered by dedicated tests (rejection without opt-in,
  acceptance with opt-in, loopback and https acceptance, malformed URL
  rejection).
- Public `RefreshTokenService::cleanup_expired()` hook that deletes every
  expired refresh token across all users in one call, for boot-time or
  operator-triggered sweeps. It is deliberately caller-dependent — the
  library schedules no background work on its own — because token issuance
  only purges the logging-in user's rows, so tokens of users who never log
  in again are reachable only through this explicit sweep.
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
- Deterministic regression guard for the password-hashing offload: a unit
  test runs the async hash helper on a single-threaded runtime while a
  counter-ticking progress task observes scheduling, and asserts that the
  worker makes progress during the hashing await — possible only while
  bcrypt executes on the blocking pool. An inline (non-offloaded) bcrypt
  call would block the worker with no yield point and yield exactly zero
  progress, failing the test. Binary, wall-clock-free and
  core-count-independent by design.

### Changed

- The canonical test configuration (`config.test.json`) sets
  `history_db_allow_insecure` true: its history URL targets the
  Docker-internal `influxdb` hostname, which is not a loopback host and is
  therefore subject to the new encrypted-transport policy. The flag is
  scoped to this test-only configuration file and does not change the
  library's default behavior (test-only configuration change, deliberate).
- The refresh-token retention purge is now expired-only (APPLIED): the
  per-user purge on the issuance path and at the end of the rotation
  transaction deletes rows whose `expire_at` passed, and no longer deletes
  already-used rows. Consumed tokens are retained until their natural
  expiry, so a replay keeps classifying as `ReuseDetected` for the token's
  full lifetime, while per-user storage stays bounded by the expiry window
  instead of growing with the number of rotations. The global
  `cleanup_expired` sweep was already expired-only and is unchanged.
- The redundant composite `UNIQUE (value, mac)` constraint on the `nonces`
  table is removed by a new migration that rebuilds the table: the
  column-level `UNIQUE` on `value` is strictly stronger (it rejects a
  repeated value whatever the MAC, which the pair constraint would accept),
  so the composite only added one extra B-tree write to every nonce insert
  with zero additional enforcement. The rebuild is safe with foreign keys
  enabled (no foreign-keys pragma inside the migration: SQLite ignores it
  inside the runner's per-file transaction, which also records the ledger
  row and makes the whole rebuild atomic), preserves row identifiers, and
  leaves `PRAGMA foreign_key_check` empty. Tests pin the fresh-database
  behavior (uniques, cascade, foreign-key integrity), a legacy-deployment
  replay (old schema plus pre-filled ledger migrated through the production
  boot: data preserved, constraints and cascade intact, ledger row added)
  and a full rollback on an interrupted rebuild (original table restored,
  no residue, no ledger row).
- The redundant manual index on `refresh_tokens.token` is dropped by a new
  migration: the `token TEXT NOT NULL UNIQUE` column constraint already
  maintains an implicit unique index that serves every token-based lookup,
  so the manual index only doubled the B-tree write and storage cost of
  each token insert and delete. Token lookups and updates keep resolving
  through an index search after the migration (pinned by a repository
  test asserting the query plan).
- Bounded retention for the two previously insert-only tables, piggybacked
  on their guaranteed write paths: key rotation (`KeyService::add`) now
  purges historical keys past their 30-day retention window inside the same
  transaction that records the rotated-out key — steady state stays around
  720 rows per sensor at one rotation per hour, and the purge is a full
  scan of that bounded table, so no index on `expire_at` is added on
  purpose (one extra B-tree write per insert to accelerate a negligible
  scan is a documented ceiling) — and refresh-token issuance
  (`RefreshTokenService::add`) now deletes the user's expired tokens
  before inserting the new one, riding the existing
  per-user index. Tokens of users who never log in again are covered by
  the added `cleanup_expired` hook instead (see above). No existing
  rotation or issuance semantics changed beyond the added purges.
- The time-series client is now built with connect and request timeouts
  (5 s and 30 s in production): a hung or unreachable InfluxDB server used
  to stall history writes indefinitely — the HTTP call never returned
  while the corresponding SQLite update was already committed — and the
  write now fails within the configured windows instead. The factory takes
  the two durations as parameters so tests can use short values. The
  per-event success log line on the history write path is demoted from
  info to debug level: it executed on every sensor event, and success
  details are not worth a format plus I/O per event on the gateway
  (failures remain error-level).
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
  the transparent `Common` variant. This intermediate structured
  conversion introduced no public variant change of its own; this
  release's broader refresh-token changes (the removed two-step
  consumption flow and the atomic `rotate` API) appear under the breaking
  section above.
- The nonces module follows the same structured error flow:
  `NonceRepository` methods return the shared `CommonError` instead of raw
  `sqlx::Error`, and `NonceService` maps failures to `NonceError` through
  the existing conversion, so the local sqlx-to-domain mapper helper is
  deleted. Duplicate-nonce detection first surfaced as the structured
  `UniqueViolation` variant (constraint identity as reported by SQLite for
  the violated index) carried by the transparent `Common` variant of
  `NonceError`, and this release's final state consolidates it further
  into the public `NonceError::ReplayDetected` variant (see the changed
  entry below): the former raw-sqlx duplicate/foreign-key assertions now
  target the typed variants with the original violation kinds preserved.
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
- The file-based primary SQLite database now runs in WAL journal mode: the
  connect options set `journal_mode` to WAL, so readers work on snapshots
  and no longer queue behind a committing writer (in the previous
  rollback-journal mode every commit needed exclusive access while active
  readers drained, turning concurrent sensor bursts, key rotations and
  nonce inserts into latency spikes bounded only by the 5 s busy timeout).
  The `shared_cache` option is removed from the file branch on purpose: a
  pool of connections sharing one page cache contends on table-level locks
  (`SQLITE_LOCKED`) instead of progressing concurrently, and WAL plus the
  busy timeout cover residual write contention; the single shared
  in-memory database keeps its `cache=shared` URI, where shared cache is
  the right setup. Durability is unchanged: `synchronous` stays at the
  SQLite default FULL (pinning a deliberate decision; NORMAL could roll
  back security-relevant commits such as anti-replay nonces after a power
  loss and remains an option for the user to request explicitly). A
  dedicated connection test opens a temporary file database through the
  production factory and asserts `PRAGMA journal_mode = wal` and
  `PRAGMA synchronous = 2`, and re-checks that the `:memory:` branch still
  yields a working pool.
- Password hashing and verification no longer run inline on the async
  runtime: the crate-internal password helpers gain `*_async` wrappers that
  move the password (and stored hash) into `tokio::task::spawn_blocking`,
  so the ~100-300 ms of pure CPU a cost-12 bcrypt call burns executes on
  the dedicated blocking pool instead of stalling the calling tokio worker
  and starving every other task it carries (concurrent logins in
  particular). All async call sites are routed through the wrappers: login
  credential verification in the user repository, and password hashing in
  user creation and update in the user service. Return values and error
  variants are unchanged; the synchronous primitives remain for non-async
  reuse and unit tests.
- Duplicate nonce inserts now surface as a dedicated
  `NonceError::ReplayDetected` variant instead of
  `NonceError::Common(CommonError::UniqueViolation)` (APPLIED): replay
  detection is enforced by the database itself — the column-level
  `UNIQUE(value)` constraint rejects an already-accepted value inside the
  INSERT, so concurrent writers get exactly one acceptance with no racy
  SELECT-then-INSERT probe. `NonceService::add` keeps its signature and
  still returns the stored record on success; only the duplicate error
  path changes shape, so callers matching on the wrapped unique violation
  for duplicates must match `NonceError::ReplayDetected` instead. The
  ledger contract is now documented on the module and the service: a
  nonce value is burned GLOBALLY once accepted (the same value is refused
  whatever sensor presents it — the validated strictly-stronger
  anti-replay semantics of the single `UNIQUE(value)` schema; with 96-bit
  values a cross-sensor collision is practically unreachable), accepted
  rows are never deleted on verification, and they leave the ledger only
  through the foreign-key cascade when the sensor's key row is deleted at
  key rotation, keeping ledger growth bounded by the key-rotation window;
  there is deliberately no explicit purge method.

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
- The hand-written InfluxDB line-protocol formatters
  (`SensorDataPoint::to_line_protocol`, `SensorTags::to_line_protocol`,
  `SensorFields::to_line_protocol`): they escaped the `:` separator of MAC
  addresses, which line protocol does not treat as special, while leaving
  the actually special characters (`\`, `,`, `=`, space) unescaped, so any
  value carrying those would have corrupted the written line. The live
  write path never used them: it builds points through
  `influxdb2::models::DataPoint::builder`, which performs the correct
  escaping. The formatters were reachable only from documentation examples
  (their modules are crate-private in release builds), and those examples
  now demonstrate `DataPoint::builder` instead. No in-tree or out-of-tree
  caller existed.

### Fixed

- Flaky parallel test runs eliminated by isolating the process-global
  `CONFIG_FILE` environment variable in tests: a std-only mutex-guarded
  helper (`src/test_env.rs`, compiled only for tests) serializes every test
  that points the config loader at a temporary file and restores the
  previous value automatically on drop, so running
  `cargo test --locked --no-default-features` with the default parallel
  threads no longer races between the file-pragma and history-timeout /
  policy tests. The history-policy tests additionally use distinct
  temporary config file names per security-flag variant so parallel
  executions cannot overwrite each other's configuration files. No
  production code is affected; the core-only suite now passes
  deterministically in parallel mode.
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
- The migrations README file table now lists the two migrations added by the
  performance work (duplicate refresh-token index drop, nonces rebuild).
- Key-rotation denial of service through the retention purge: a sensor
  offline for more than the 30-day historical-key window kept an expired
  key row whose historical row had also expired, and the historical purge
  inside the rotation transaction deleted that referenced row, violating
  the `keys.value` -> `historical_keys.value` foreign key and aborting
  every subsequent rotation of every sensor. The rotation transaction now
  purges expired key rows first (they are unusable, and their anti-replay
  nonces leave through the `nonces.mac` -> `keys.mac` cascade), then
  purges expired historical keys with a `NOT EXISTS` guard that never
  deletes a row still referenced by a live key, so no statement order can
  orphan a referenced row. The resulting retention contract is documented
  on the rotation path: expired key rows leave at the next rotation of
  any sensor, historical keys follow once unreferenced, nonces cascade
  when their key row goes. Regression tests cover the offline-sensor
  scenario, the cascade and the guard.
- Secret redaction for the secret-bearing data types: the derived `Debug`
  output printed secrets in full and derived serialization carried them
  into any serialized form. `HomeIotConfig` (history-database token),
  `RefreshToken` (token value), `Key` and `HistoricalKey` (raw key bytes)
  and the crate-internal `InternalUser` (password hash) now render the
  secret through a hand-written `Debug` impl as a fixed `[REDACTED]`
  marker: every other field keeps its readable rendering, and the presence
  of a password hash stays visible. Serialization skips the secret field
  while deserialization keeps accepting a complete payload, so
  serializing these models is intentionally lossy for the secret (no
  serialize -> deserialize round trip reproduces it); no caller serializes
  any of these models. The documentation examples no longer demonstrate
  printing whole secret-bearing values. Per-type tests pin that neither
  `Debug` output nor the serialized form contains the secret.
- Login timing no longer leaks which emails are registered (timing-based
  user enumeration, CWE-208): `UserRepository::get_by_email_and_password`
  now runs the bcrypt verification on every login attempt. When the email
  is unknown, the presented password is verified against a fixed dummy
  bcrypt hash generated at the same default cost as real stored hashes, so
  the unknown-email branch burns the same CPU as the wrong-password branch
  and response time cannot distinguish the two. The verification keeps
  running off the async runtime through the shared password helper, and the
  function still returns no user for any failing combination. A dedicated
  test pins that the dummy constant is a real cost-12 hash rejecting
  passwords, and the unknown-email path is covered deterministically.
- Registry token handling in the Docker test image: the preferred
  mechanism is a BuildKit secret mount (`docker-compose.test.yml` passes
  the token from the environment as a build-time secret and the dependency
  prebuild consumes it through a scoped `RUN --mount=type=secret`), an
  ephemeral per-build value that never lands in image layers or build
  history. Compose no longer passes the token through `build.args` by
  default: a plain build argument would preserve the exposure in build
  metadata and provenance. The Dockerfile `ARG
CARGO_REGISTRIES_FORGEJO_TOKEN` remains ONLY as a MANUAL non-BuildKit
  fallback, a human passing it explicitly (e.g. `docker build
--build-arg ...`) for legacy runners outside Compose — an explicitly
  UNRESOLVED residual risk: the token scope in the Forgejo UI is still
  UNVERIFIED (no privilege assumption is documented); severity stays LOW
  only once the user confirms a package-read-only token.
- The Docker test stack now runs as a dedicated non-root user instead of
  root: `Dockerfile.test` creates the user, aligns file ownership of the
  working directory and of the directories the compose named volumes seed
  (the cargo registry cache and the build target), and switches to the
  non-root user well before the dependency prebuild, so the container
  command executes unprivileged. The cargo home configuration is also
  unified: the image no longer points `CARGO_HOME` at `/root/.cargo`
  while the compose cache volume mounted `/usr/local/cargo/registry`; the
  registry config now lives under the rust image's own
  `/usr/local/cargo`, matching the cache mount.

- MAC addresses were accepted free-form, so a malformed or line-protocol-hostile
  value could flow into key/nonce records and time-series tags. One shared
  validator (six hexadecimal pairs separated by `:`, case-insensitive) is
  now enforced where MACs enter the system: `SensorTags::new` (which now
  returns a `Result` and reports
  `SensorHistoryError::InvalidFormat`), `KeyService::add`
  (`KeyError::InvalidMacFormat`) and `NonceService::add` (new
  `NonceError::InvalidMacFormat` variant) refuse a malformed address
  before any database work, and the `mac` fields of `SensorTags`, `Key`
  and `Nonce` validate through a serde deserialization hook, closing the
  bypass where the public fields plus derived `Deserialize` would have
  skipped constructor checks. The sensor registration boundary is covered
  too: the `mac` field of the `Sensor` model validates through the same
  serde hook, and `SensorService::add` replaces its non-empty check with
  the full format validation (`SensorError::InvalidMac`), since
  `sensors.mac` roots the foreign-key chain (`keys.mac` ->
  `nonces.mac`) and a hostile value there would flow into every
  downstream record. No backend production endpoint inserts or
  deserializes a `Sensor` payload (the backend exposes no sensor-creation
  endpoint, and no API payload deserializes this model); the bridge
  production paths DO call `SensorService::add`, with well-formed LoRa
  MACs that already match the format, so they are unaffected. Out-of-tree
  test fixtures
  that register fake values (the backend sensor tests' placeholder MACs
  and the bridge SIL tests' `SL:`-prefixed placeholders) will be refused
  by the new contract and must switch to well-formed addresses at their
  next update. Tests pin the rejections at construction and
  deserialization (including a MAC carrying `,`, `=` and spaces), and that
  lowercase addresses stay accepted.

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
