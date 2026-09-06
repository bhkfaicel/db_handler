mod repository;

mod error;
// Public like every sibling model module, so external callers can name the
// `ContactSensor` type that the public service returns.
pub mod model;
pub mod service;
