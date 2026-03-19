//! # Cache — Configurable CPU cache hierarchy simulator
//!
//! The same implementation serves as L1, L2, or L3 by configuring
//! size, associativity, and latency differently.

pub mod cache_line;
pub mod cache_set;
pub mod cache;
pub mod stats;
