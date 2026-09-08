//! PowerPC 601 gate-level simulator.
//!
//! Persistent architectural state is clocked through D flip-flops. The
//! completion target is the full Layer 07u integer surface and the checked
//! lifecycle shared with the functional simulator.

pub mod alu;
pub mod bits;
mod cpu;
pub mod decoder;
mod register_file;
mod state;

pub use cpu::{PowerPc601GateSimulator, FLIP_FLOP_COUNT};
pub use powerpc601_simulator::{ExecutionResult, PowerPcError, PowerPcState, StepTrace};
pub use state::DffMemory;
