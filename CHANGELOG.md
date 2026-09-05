# Changelog

All notable changes to this project will be documented in this file.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

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
