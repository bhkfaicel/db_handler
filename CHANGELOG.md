# Changelog

All notable changes to this project will be documented in this file.

The format is inspired by [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to semantic versioning.

## [Unreleased]

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
