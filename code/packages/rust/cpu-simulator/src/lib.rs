//! # CPU Simulator -- Layer 3 of the computing stack.
//!
//! Simulates the core of a processor: registers, memory, program counter,
//! and the fetch-decode-execute cycle that drives all computation.
//!
//! This is a generic CPU model -- not tied to any specific architecture.
//! The ISA simulators (RISC-V, ARM, WASM, Intel 4004) build on top of this
//! by providing their own instruction decoders and executors.
//!
//! ## Architecture
//!
//! The crate is organized into four modules that mirror the physical
//! components of a real CPU:
//!
//! - [`registers`] -- Fast, small storage (like a whiteboard on your desk)
//! - [`memory`] -- Large, slow storage (like a filing cabinet across the room)
//! - [`pipeline`] -- The fetch-decode-execute cycle and tracing infrastructure
//! - [`cpu`] -- The CPU itself, tying registers, memory, and pipeline together
//!
//! ## Trait-based ISA abstraction
//!
//! The CPU uses two traits to remain architecture-independent:
//!
//! - [`InstructionDecoder`] -- translates raw bits into structured decode results
//! - [`InstructionExecutor`] -- performs the decoded operation on registers/memory
//!
//! To simulate a specific ISA (like RISC-V), implement these two traits
//! and pass them to [`CPU::new`].

pub mod cpu;
pub mod memory;
pub mod pipeline;
pub mod registers;

// Re-export the main types for convenient access.
pub use cpu::{CPUState, InstructionDecoder, InstructionExecutor, CPU};
pub use memory::Memory;
pub use pipeline::{
    format_pipeline, DecodeResult, ExecuteResult, FetchResult, PipelineStage, PipelineTrace,
};
pub use registers::RegisterFile;
