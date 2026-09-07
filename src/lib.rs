// Core layer: always compiled (config helpers, primary/history DB plumbing,
// driver traits). It has no dependency on any feature-gated domain.
pub mod common;
pub mod db;
pub mod drives;

// Test-support shared by every suite that mutates the process-global
// `CONFIG_FILE` variable (see module docs); compiled out of non-test builds.
#[cfg(test)]
pub(crate) mod test_env;

// Sensor domain. These four modules form one dependency closure (`history`
// imports `sensors::model::ProductType`, `keys` imports `history::keys`,
// `nonces` chains through `keys`), so they are gated together behind the
// `sensors` feature: gating any single module alone would not compile.
#[cfg(feature = "sensors")]
pub mod history;
#[cfg(feature = "sensors")]
pub mod keys;
#[cfg(feature = "sensors")]
pub mod nonces;
#[cfg(feature = "sensors")]
pub mod sensors;

// User domain: `users` and `refresh_token` depend only on the core modules,
// so they are gated together behind the `users` feature.
#[cfg(feature = "users")]
pub mod refresh_token;
#[cfg(feature = "users")]
pub mod users;

/// Main prelude module that exports the most commonly used types and traits.
///
/// Import this module with `use db_handler::prelude::*;` to automatically
/// have access to the `Service` trait and other common types. The prelude
/// re-exports straight from the `db` layer: the `common` module must stay a
/// foundation that never depends on a higher layer.
pub mod prelude {
    pub use crate::db::primary::service::{map_sqlx_to_common_error, Service};
}
