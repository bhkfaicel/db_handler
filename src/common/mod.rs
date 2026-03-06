pub mod helpers;

/// Prelude module that automatically exports common types and traits.
///
/// Import this module with `use db_handler::common::prelude::*;` to automatically
/// have access to the `Service` trait and other common types.
pub mod prelude {
    pub use crate::db::primary::service::map_sqlx_to_common_error;
    pub use crate::db::primary::service::Service;
}
