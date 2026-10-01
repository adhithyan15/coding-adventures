//! Bounded FM02 subprocess runner for Rust-authored Forme stages.

#![forbid(unsafe_code)]

mod context;
mod peer;
mod runner;
mod stage;
mod wire;

pub use context::{
    EnvApi, FilesystemApi, Logger, NetworkApi, NetworkResponse, ShellApi, ShellResult,
    StageContext, StorageApi, TimeApi,
};
pub use runner::{run_plugin, RunnerOptions};
pub use stage::{
    CancellationError, CancellationToken, FromWire, InputStream, Stage, StageError, StageInput,
    StageMetadata, StageOutput,
};
pub use wire::{decode_wire_value, encode_frame, FrameDecoder, ProtocolError, WireValue};
