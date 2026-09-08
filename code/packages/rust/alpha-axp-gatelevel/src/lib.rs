//! DEC Alpha AXP 21064 gate-level simulator.
//!
//! Persistent architectural state is clocked through D flip-flops. The
//! completion target is the full Layer 07s integer surface and checked
//! lifecycle shared with the functional simulator.

pub mod alu;
pub mod bits;
mod cpu;
pub mod decoder;
mod register_file;
mod state;

pub use alpha_axp_simulator::{AlphaError, AlphaState, ExecutionResult, StepTrace};
pub use cpu::{AlphaGateSimulator, FLIP_FLOP_COUNT};
pub use state::DffMemory;
