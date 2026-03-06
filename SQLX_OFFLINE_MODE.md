# Documentation: SQLX Offline Mode and `.sqlx/` Files

## 📋 Overview

`.sqlx/` files contain metadata for SQL queries used with the `sqlx::query!` macro.  
They allow SQLX to compile the project **without needing a live database connection**.

---

## 🔍 What is SQLX offline mode?

### Problem it solves

The `sqlx::query!` macro is a **compile‑time verification macro** that:

- Checks that SQL queries are syntactically correct
- Verifies that the referenced columns exist in the database
- Validates data types (nullability, column types)
- Generates type‑safe Rust code based on the actual database schema

**Without offline mode**, SQLX must connect to the database at compile time, which can be problematic:

- ❌ Impossible to compile without access to the database
- ❌ Painful in CI/CD environments where a DB is not configured
- ❌ You must configure the DB before every build
- ❌ Slower builds (DB round‑trips on each compilation)

### Solution: Offline mode

Offline mode lets SQLX use **pre‑generated metadata** stored in `.sqlx/` files instead of connecting to the database at compile time.

---

## 📁 Structure of `.sqlx/` files

### Location

Files are stored in the `.sqlx/` folder at the project root:

```text
db_handler/
├── .sqlx/
│   ├── query-065faf22c005f1aa3e67ff06e1502f82e073ad11eb20fcd80c09bd6db944483f.json
│   ├── query-28a550c39a4a3c96525a2d5a2b5abb3244b96774580e281a63b7a1893c1aa035.json
│   └── ... (one file per unique query)
├── src/
├── Cargo.toml
└── ...
```

### Naming

Each file name is based on the SHA256 hash of the SQL query:

- Format: `query-{hash}.json`
- Example: `query-28a550c39a4a3c96525a2d5a2b5abb3244b96774580e281a63b7a1893c1aa035.json`

The hash guarantees uniqueness: identical queries share the same file.

### File structure

Each JSON file contains:

```json
{
  "db_name": "SQLite", // Database type
  "query": "SELECT id, username FROM users WHERE email = ?", // Original SQL query
  "describe": {
    "columns": [
      // Returned columns metadata
      {
        "name": "id", // Column name
        "ordinal": 0, // Column position (index)
        "type_info": "Integer" // SQL type
      },
      {
        "name": "username",
        "ordinal": 1,
        "type_info": "Text"
      }
    ],
    "parameters": {
      "Right": 1 // Number of bind parameters (placeholders ?)
    },
    "nullable": [
      // For each column: true = nullable, false = NOT NULL
      false,
      false
    ]
  },
  "hash": "28a550c39a4a3c96525a2d5a2b5abb3244b96774580e281a63b7a1893c1aa035" // Query hash
}
```

### Concrete example

**SQL query**:

```rust
sqlx::query!("SELECT id, username, email, password_hash FROM users WHERE email = ?", email)
```

**`.sqlx/query-28a55....json` file**:

```json
{
  "db_name": "SQLite",
  "query": "SELECT id, username, email,password_hash FROM users WHERE email = ?",
  "describe": {
    "columns": [
      { "name": "id", "ordinal": 0, "type_info": "Integer" },
      { "name": "username", "ordinal": 1, "type_info": "Text" },
      { "name": "email", "ordinal": 2, "type_info": "Text" },
      { "name": "password_hash", "ordinal": 3, "type_info": "Text" }
    ],
    "parameters": { "Right": 1 },
    "nullable": [true, false, false, false]
  },
  "hash": "28a550c39a4a3c96525a2d5a2b5abb3244b96774580e281a63b7a1893c1aa035"
}
```

**DELETE query with no returned rows**:

```rust
sqlx::query!("DELETE FROM keys WHERE mac = ?", mac)
```

**Corresponding file**:

```json
{
  "db_name": "SQLite",
  "query": "DELETE FROM keys WHERE mac = ?",
  "describe": {
    "columns": [], // No columns for DELETE
    "parameters": { "Right": 1 },
    "nullable": []
  },
  "hash": "7ded542cdb426c2ee3ecad1c49753be69325a549fd0d33fa09072b0dc6e9ba62"
}
```

---

## 🛠️ Generating `.sqlx/` files

### Prerequisites

1. A **database with all migrations applied**
2. **`sqlx-cli` installed**:

   ```bash
   cargo install sqlx-cli
   ```

### Generation steps

#### 1. Prepare the database

Make sure your database has all migrations applied:

```bash
# Option 1: Use an existing DB
export DATABASE_URL="sqlite:./path/to/your/database.db"

# Option 2: Create a temporary DB and apply migrations
sqlite3 /tmp/sqlx_prepare.db < migrations/20250226015426_init.sql
sqlite3 /tmp/sqlx_prepare.db < migrations/20250610_init.sql
export DATABASE_URL="sqlite:/tmp/sqlx_prepare.db"
```

#### 2. Generate `.sqlx/` files

```bash
cargo sqlx prepare --database-url "$DATABASE_URL"
```

This command:

- Scans all source code for `sqlx::query!` calls
- Connects to the database
- Runs `DESCRIBE` on each query to get metadata
- Writes JSON files into `.sqlx/`
- Validates that all queries are correct

#### 3. Verify

After generation you should see:

```text
query data written to .sqlx in the current directory; please check this into version control
```

---

## ⚙️ Configuring offline mode

### Option 1: Environment variable (temporary)

```bash
export SQLX_OFFLINE=true
cargo build
```

### Option 2: `.cargo/config.toml` file (recommended)

Create or edit `.cargo/config.toml` at the project root:

```toml
[env]
SQLX_OFFLINE = "true"
```

This automatically enables offline mode for every build.

### Current project configuration

This project uses **Option 2** with `.cargo/config.toml`:

```toml
[env]
SQLX_OFFLINE = "true"
```

That means offline mode is **enabled by default** for all builds.

---

## 📝 Usage in development

### Normal workflow

1. **Development**: Change code, add/modify SQL queries
2. **Compilation**: `cargo build` or `cargo check` uses the existing `.sqlx/` files
3. **Tests**: `cargo test` works as usual

### When to regenerate `.sqlx/` files?

Regenerate files in these situations:

✅ **After modifying an existing SQL query**  
✅ **After adding a new `sqlx::query!`**  
✅ **After changing the database schema** (migrations)  
✅ **When you get SQLX‑related compile errors**  
✅ **After upgrading SQLX** to a new version

### Useful commands

```bash
# Check that queries are up to date (without regenerating)
cargo sqlx prepare --check --database-url "$DATABASE_URL"

# Regenerate all files
cargo sqlx prepare --database-url "$DATABASE_URL"

# Build in offline mode (automatic with .cargo/config.toml)
cargo build

# Build in online mode (temporarily disable offline)
SQLX_OFFLINE=false cargo build
```

---

## ✅ Best practices

### 1. Version `.sqlx/` files

**IMPORTANT**: `.sqlx/` files **must be committed to Git**!

```bash
git add .sqlx/
git commit -m "Update SQLX offline query metadata"
```

**Why?**

- Allows all developers to compile without a database
- Ensures consistency across environments
- Required for CI/CD pipelines

### 2. Do not ignore `.sqlx/` in `.gitignore`

❌ **Do NOT** do this:

```text
# .gitignore
.sqlx/
```

✅ **Correct**: `.sqlx/` files must be tracked in Git.

### 3. Keep in sync with the schema

`.sqlx/` files must match the real database schema:

- **Before committing**: Check that `.sqlx/` is up to date
- **After a migration**: Regenerate files
- **After a pull**: Ensure `.sqlx/` matches the new code

### 4. Database for generation

Use a database with **all migrations applied** to generate `.sqlx/` files.  
An incomplete DB will produce incorrect metadata.

---

## 🔄 Updating files after changes

### Scenario: Adding a new query

```rust
// src/users/repository/mod.rs

// New function using sqlx::query!
pub(crate) async fn get_by_username(&self, username: &str) -> Result<Option<User>> {
    let result = sqlx::query!(
        "SELECT id, username, email FROM users WHERE username = ?",
        username
    )
    .fetch_optional(&*self.pool)
    .await?;
    // ...
}
```

**Steps**:

1. Write the code with the new query
2. Make sure the database is up to date with migrations
3. Generate the new metadata files:

   ```bash
   cargo sqlx prepare --database-url "$DATABASE_URL"
   ```

4. Verify the build:

   ```bash
   cargo check
   ```

5. Commit the new `.sqlx/` files

### Scenario: Modifying an existing query

If you change an existing SQL query:

```rust
// Before
sqlx::query!("SELECT id, username FROM users WHERE email = ?", email)

// After
sqlx::query!("SELECT id, username, email FROM users WHERE email = ?", email)
```

**Steps**:

1. Modify the query in the code
2. Regenerate files (the hash changes, a new file is created)
3. The old file becomes unused (you can delete it manually)
4. Build and test
5. Commit the changes

---

## ❓ Troubleshooting

### Error: "unable to open database file"

**Cause**: SQLX is trying to connect to the database in online mode.

**Solution**:

- Check that `SQLX_OFFLINE=true` is set
- Verify that `.cargo/config.toml` exists with the right configuration
- Regenerate `.sqlx/` files if needed

### Error: "query metadata not found"

**Cause**: A query was added/changed but `.sqlx/` files are not up to date.

**Solution**:

```bash
cargo sqlx prepare --database-url "$DATABASE_URL"
```

### Error: "column not found" or "type mismatch"

**Cause**: Metadata in `.sqlx/` no longer matches the real schema.

**Solution**:

1. Apply all migrations
2. Regenerate files:

   ```bash
   cargo sqlx prepare --database-url "$DATABASE_URL"
   ```

### Slow compilation

**Cause**: SQLX is trying to connect to the DB on each build.

**Solution**: Ensure offline mode is enabled in `.cargo/config.toml`.

---

## 📊 Summary

| Aspect            | Details                                                    |
| ----------------- | ---------------------------------------------------------- |
| **Folder**        | `.sqlx/` at the project root                               |
| **Format**        | JSON files (one per unique query)                          |
| **Naming**        | `query-{hash}.json` (SHA256 of the query)                  |
| **Generation**    | `cargo sqlx prepare --database-url "$DATABASE_URL"`        |
| **Configuration** | `.cargo/config.toml` with `SQLX_OFFLINE = "true"`          |
| **Versioning**    | ✅ Must be committed to Git                                |
| **Content**       | Column metadata, types, parameters, nullability, etc.      |

---

## 🔗 Resources

- [Official SQLX docs – Offline Mode](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md#enable-building-in-offline-mode-with-query)
- [SQLX Query Macros](https://docs.rs/sqlx/latest/sqlx/macro.query.html)
- [SQLX CLI](https://github.com/launchbadge/sqlx/blob/main/sqlx-cli/README.md)

---

## 📅 History

- **December 2024**: Offline mode configured for the `db_handler` project  
- Initial `.sqlx/` files generated after resolving compilation errors
