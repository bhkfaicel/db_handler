# SQLite migrations

Migrations are embedded at compile time (`include_dir!`) and applied in
lexicographic filename order on every primary-database connection. Only
files with a `.sql` extension are executed; this document is ignored by
the runner.

## Files (apply in this order)

| File | Purpose |
|------|---------|
| `20250101000000_create_refresh_tokens_table.sql` | Refresh-token storage for auth |
| `20250226015426_init.sql` | Core `users`, `sensors`, `open_close_sensors` (with `ON DELETE CASCADE`) |
| `20250610_init.sql` | `historical_keys`, `keys`, `nonces` with cascade on `sensors(mac)` |
| `20260120_add_unique_to_open_close_sensors.sql` | Unique indexes on `sensor_mac` and `sensor_id` |
| `20260906000001_drop_duplicate_refresh_token_index.sql` | Drops the duplicate manual index on `refresh_tokens.token` (the `UNIQUE` column constraint already indexes it) |
| `20260906000002_rebuild_nonces_drop_composite_unique.sql` | Rebuilds `nonces` to drop the redundant composite `UNIQUE (value, mac)` |
| `20260921000000_sensor_battery.sql` | `sensor_battery`: latest rest/load voltages per sensor, cascade on `sensors(id)`, u16 voltage CHECKs |

## Cascade on sensor deletion

`open_close_sensors` is created in the initial migration with `ON DELETE CASCADE`
on both references to `sensors`. No separate rebuild migration is required for
greenfield deployments.

## Tests

- `src/sensors/repository/tests/delete_cascades_children.rs` — delete by MAC/id purges child rows.
- `src/db/primary/tests/mod.rs::migrations_apply_exactly_once_across_boots` — ledger prevents replay.
