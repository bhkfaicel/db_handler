-- Users table
CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL
);
-- Sensors table
CREATE TABLE IF NOT EXISTS sensors (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    sensor_type TEXT,
    mac TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL
);

-- Sensor-specific data
CREATE TABLE IF NOT EXISTS open_close_sensors (
    sensor_id INTEGER,
    sensor_mac TEXT NOT NULL,
    is_open BOOLEAN  NOT NULL,
    updated_at DATETIME NOT NULL,
    FOREIGN KEY (sensor_id) REFERENCES sensors(id) ON DELETE CASCADE,
    FOREIGN KEY (sensor_mac) REFERENCES sensors(mac) ON DELETE CASCADE
);