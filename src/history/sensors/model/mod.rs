/// Models for time-series sensor data points.
///
/// This module contains data structures for representing sensor data
/// in time-series database format, including tags, fields, and complete data points.
#[cfg(any(test, doc))]
pub mod fields;
#[cfg(not(any(test, doctest)))]
pub mod fields;
#[cfg(any(test, doc))]
pub mod point;
#[cfg(not(any(test, doctest)))]
pub(crate) mod point;
#[cfg(any(test, doc))]
pub mod tags;
#[cfg(not(any(test, doctest)))]
pub(crate) mod tags;

#[cfg(test)]
mod tests;

pub use fields::{SensorFields, VibrationCount};
pub use point::*;
pub use tags::*;
