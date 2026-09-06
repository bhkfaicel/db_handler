use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommonError {
    /// Generic not found error that carries the entity name and its ID.
    #[error("{0} not found with ID: {1}")]
    NotFound(String, u32),

    /// A unique-constraint violation occurred (e.g. duplicate key).
    ///
    /// The `constraint` field carries the constraint identity as reported by
    /// SQLite (for example `keys.mac`, or a comma-separated column list such
    /// as `nonces.value, nonces.mac` for a composite unique index).
    #[error("Unique constraint violated: {constraint}")]
    UniqueViolation {
        /// Name of the violated unique constraint (table.column form or the
        /// explicit index/column list reported by SQLite).
        constraint: String,
    },

    /// Database error occurred.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Generic error with context.
    #[error("Operation failed: {0}")]
    Operation(String),
}

impl CommonError {
    /// Converts a `sqlx::Error` into a `CommonError`.
    ///
    /// This is the single place where unique violations are detected and
    /// decoded into the structured `UniqueViolation` variant: every
    /// `sqlx::Error::Database` whose sqlx-reported kind is
    /// `ErrorKind::UniqueViolation` is matched here, so services never need
    /// to sniff error messages themselves. All other sqlx errors keep the
    /// generic `Database` variant.
    pub fn from_sqlx(error: sqlx::Error) -> Self {
        if let sqlx::Error::Database(ref db_error) = error {
            if db_error.kind() == sqlx::error::ErrorKind::UniqueViolation {
                // The sqlx kind is authoritative for the violation itself;
                // the constraint name comes from the message extraction below
                // and falls back to "unknown" if the message is malformed.
                let constraint = extract_unique_constraint(db_error.message())
                    .unwrap_or_else(|| "unknown".to_string());
                return CommonError::UniqueViolation { constraint };
            }
        }
        CommonError::Database(error)
    }
}

/// Prefix SQLite uses to report unique-constraint violations.
const SQLITE_UNIQUE_PREFIX: &str = "UNIQUE constraint failed: ";

/// Extracts the violated constraint name from a SQLite error message.
///
/// SQLite reports a violated unique constraint ONLY through the error
/// message text (for example `UNIQUE constraint failed: keys.mac`); unlike
/// other engines it exposes no structured metadata field (constraint name,
/// table or columns) on the error object. The message is therefore the only
/// source SQLite provides, and this function is the single place in the
/// codebase that decodes it, so the parsing rule lives in exactly one tested
/// location instead of being duplicated per service.
///
/// Returns `None` when the message is not a unique-violation report or when
/// the report is malformed (prefix present but no constraint name after it).
/// For composite constraints SQLite reports the full column list, which is
/// returned as is (for example `nonces.value, nonces.mac`).
fn extract_unique_constraint(message: &str) -> Option<String> {
    let constraint = message.strip_prefix(SQLITE_UNIQUE_PREFIX)?;
    let constraint = constraint.trim();
    if constraint.is_empty() {
        return None;
    }
    Some(constraint.to_string())
}

pub type Result<T> = std::result::Result<T, CommonError>;

#[cfg(test)]
mod tests;
