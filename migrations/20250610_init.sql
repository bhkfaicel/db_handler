-- Table des données spécifiques cles historique
CREATE TABLE IF NOT EXISTS historical_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    value BLOB NOT NULL UNIQUE,
    expire_at DATETIME NOT NULL
);

-- Table des données spécifiques aux cles
CREATE TABLE IF NOT EXISTS keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    value BLOB NOT NULL UNIQUE,
    expire_at DATETIME NOT NULL,
    mac TEXT NOT NULL UNIQUE,
    FOREIGN KEY (mac) REFERENCES sensors(mac) ON DELETE CASCADE,
    FOREIGN KEY (value) REFERENCES historical_keys(value)
);

-- Table des données spécifiques aux nonce
CREATE TABLE IF NOT EXISTS nonces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    value BLOB NOT NULL UNIQUE,
    mac TEXT NOT NULL,
    UNIQUE (value, mac),
    FOREIGN KEY (mac) REFERENCES keys(mac) ON DELETE CASCADE
);
