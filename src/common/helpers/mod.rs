pub(crate) mod config;
// Shared MAC format validator used at the trust boundaries of every
// MAC-carrying model (constructor arguments and deserialization).
pub mod mac;
pub use config::HomeIotConfig;

#[cfg(test)]
pub mod tests;
