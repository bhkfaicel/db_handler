/// Module for managing sensor historical data in time-series databases.
///
/// This module provides functionality to store and retrieve time-series data
/// from sensors. It follows the architecture pattern with:
/// - `model`: Data structures for time-series data points
/// - `repository`: Data access layer with generic time-series database abstraction
/// - `service`: Business logic layer
///
/// The repository uses a trait-based approach (`TimeSeriesDatabase`) allowing
/// easy switching between different backends (InfluxDB, TimescaleDB, QuestDB, etc.).
///
/// Note: Database connection management is in `crate::connection`
#[cfg(any(test, doc))]
pub mod error;
#[cfg(not(any(test, doctest)))]
pub(crate) mod error;
#[cfg(any(test, doc))]
pub mod model;
#[cfg(not(any(test, doctest)))]
pub(crate) mod model;
#[cfg(any(test, doc))]
pub mod repository;
#[cfg(not(any(test, doctest)))]
mod repository;
#[cfg(any(test, doc))]
pub mod service;
#[cfg(not(any(test, doctest)))]
pub(crate) mod service;
