-- Rebuild of the nonces table that drops the redundant composite
-- UNIQUE (value, mac) index. The column-level UNIQUE on `value` is strictly
-- stronger: it already rejects a repeated value whatever the MAC, so the
-- composite only added one extra B-tree write per nonce insert with zero
-- additional enforcement.
--
-- There is deliberately no `PRAGMA foreign_keys` statement here: the
-- migration runner wraps every file in one transaction and SQLite ignores
-- foreign-keys pragma changes inside a transaction, so this rebuild is
-- written to be safe with foreign keys ON. The wrapping transaction also
-- makes the whole rebuild atomic: an interrupted boot rolls back to the
-- original table and records no schema-migrations ledger row.
CREATE TABLE nonces_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    value BLOB NOT NULL UNIQUE,
    mac TEXT NOT NULL,
    FOREIGN KEY (mac) REFERENCES keys(mac) ON DELETE CASCADE
);

-- Copy row identifiers over so existing references to them stay valid.
INSERT INTO nonces_new (id, value, mac) SELECT id, value, mac FROM nonces;

-- Dropping the old child table is safe with foreign keys ON: no table
-- references nonces, so the implicit row deletion violates nothing.
DROP TABLE nonces;

ALTER TABLE nonces_new RENAME TO nonces;
