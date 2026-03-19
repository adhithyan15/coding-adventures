//! hazard-detection -- Pipeline hazard detection for a classic 5-stage CPU.
//!
//! This crate detects data, control, and structural hazards in a pipelined
//! CPU and determines the appropriate action: forwarding, stalling, or flushing.
//!
//! # Modules
//!
//! - [`types`]: Shared types (`PipelineSlot`, `HazardAction`, `HazardResult`)
//! - [`data_hazard`]: Detects RAW data hazards, resolves via forwarding/stalling
//! - [`control_hazard`]: Detects branch mispredictions, triggers flushes
//! - [`structural_hazard`]: Detects resource conflicts (ALU, FP, memory port)
//! - [`hazard_unit`]: Combined unit running all detectors each cycle

pub mod types;
pub mod data_hazard;
pub mod control_hazard;
pub mod structural_hazard;
pub mod hazard_unit;
