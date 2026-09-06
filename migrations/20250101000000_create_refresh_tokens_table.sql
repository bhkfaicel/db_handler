-- Refresh tokens table: enables token rotation and reuse detection.
-- Ordering note: this file sorts BEFORE the users table migration
-- (20250226015426_init.sql) in lexicographic order. This is safe: SQLite
-- resolves foreign keys lazily at statement time, not at CREATE TABLE.
CREATE TABLE IF NOT EXISTS refresh_tokens (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    token TEXT NOT NULL UNIQUE,
    created_at DATETIME NOT NULL,
    expire_at DATETIME NOT NULL,
    is_used BOOLEAN NOT NULL DEFAULT FALSE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index to speed up lookups by token
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_token ON refresh_tokens(token);

-- Index to speed up lookups by user_id
CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user_id ON refresh_tokens(user_id);

