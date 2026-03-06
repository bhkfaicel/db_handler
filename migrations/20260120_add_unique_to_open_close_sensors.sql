-- Add unique constraints (via unique indexes) on open_close_sensors

CREATE UNIQUE INDEX IF NOT EXISTS idx_open_close_sensors_sensor_mac
ON open_close_sensors(sensor_mac);

CREATE UNIQUE INDEX IF NOT EXISTS idx_open_close_sensors_sensor_id
ON open_close_sensors(sensor_id);