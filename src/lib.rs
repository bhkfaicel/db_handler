pub mod common;
pub mod db;
pub mod drives;
pub mod history;
// pub(crate) mod sensor_history;
// #[cfg(feature = "sensors")]
pub mod sensors;
// #[cfg(feature = "users")]
pub mod users;

//pub mod historical_keys;
pub mod keys;
pub mod nonces;

pub mod refresh_token;

/// Main prelude module that exports the most commonly used types and traits.
///
/// Import this module with `use db_handler::prelude::*;` to automatically
/// have access to the `Service` trait and other common types.
pub mod prelude {
    pub use crate::common::prelude::*;
}
