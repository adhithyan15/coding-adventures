//! In-memory data store composition crate.
//!
//! This crate wires the generic protocol and engine packages
//! together into a reusable pipeline. The reusable building blocks 
//! live in the engine and protocol crates so future transports
//! and extensions can compose them differently.

mod pipeline;

pub use in_memory_data_store_engine::{
    current_time_ms, Database, DataStoreBackend, DataStoreEngine, Entry, EntryType, EntryValue,
    OrderedF64, SortedSet, Store,
};
pub use in_memory_data_store_protocol::{CommandFrame, EngineResponse};
pub use pipeline::DataStoreManager;
