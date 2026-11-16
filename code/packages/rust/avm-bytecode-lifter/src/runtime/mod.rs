//! Flash VM execution runtime and environment module.

#![deny(unsafe_code)]

pub mod environment;
pub mod runner;

pub use environment::AvmEnvironment;
pub use runner::AvmRunner;
