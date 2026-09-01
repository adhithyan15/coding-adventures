//! DFF-backed x86-64 gate-level simulator.
//!
//! The architectural state is stored in repository flip-flop primitives while
//! instruction behavior is selected by explicit decode, arithmetic, shift,
//! rotate, multiply, divide, address, and condition gate networks.

pub mod bits;
mod cpu;
mod execution;
mod gate_flags;
mod gate_networks;
mod register_file;
mod state;

pub use cpu::{X86GateSimulator, FLIP_FLOP_COUNT};
pub use state::DffMemory;
pub use x86_simulator::functional::{
    X86Error, X86ExecutionResult, X86State, X86StepTrace, MEMORY_SIZE,
};
