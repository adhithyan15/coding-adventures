//! In-memory data store composition crate.
//!
//! This crate wires the generic transport, protocol, and engine packages
//! together into a Redis-compatible single-node server for the current DT
//! milestone. The reusable building blocks live in the engine and protocol
//! crates so future transports and extensions can compose them differently.

mod server;

pub use in_memory_data_store_engine::{
    current_time_ms, Database, DataStoreBackend, DataStoreEngine, Entry, EntryType, EntryValue,
    OrderedF64, SortedSet, Store,
};
pub use in_memory_data_store_protocol::{command_frame_from_resp, CommandFrame};
pub use server::DataStoreServer;
