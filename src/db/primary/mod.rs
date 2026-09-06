pub(crate) mod connection;
pub mod error;
pub mod models;
pub(crate) mod repository;
pub mod service;
#[cfg(test)]
// The core pagination tests exercise the shared pagination infrastructure,
// which only exists under the `sensors` or `users` features (see
// `repository`), so the module carries the same gate: both conditions must
// hold for it to compile.
#[cfg(any(feature = "sensors", feature = "users"))]
pub mod tests;
