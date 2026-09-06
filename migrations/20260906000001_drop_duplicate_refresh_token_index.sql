-- Remove the redundant manual index on refresh_tokens.token.
-- The `token TEXT NOT NULL UNIQUE` column constraint already maintains an
-- implicit unique index (sqlite_autoindex_refresh_tokens_1) that serves
-- every `WHERE token = ?` lookup, so the manual index only doubled the
-- B-tree write and storage cost of each token insert and delete.
DROP INDEX IF EXISTS idx_refresh_tokens_token;
