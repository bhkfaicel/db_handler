CREATE TABLE IF NOT EXISTS sensor_battery (
    sensor_id INTEGER PRIMARY KEY REFERENCES sensors(id) ON DELETE CASCADE,
    resting_mv INTEGER NOT NULL CHECK (resting_mv BETWEEN 0 AND 65535),
    loaded_mv INTEGER NOT NULL CHECK (loaded_mv BETWEEN 0 AND 65535),
    received_at DATETIME NOT NULL
);
