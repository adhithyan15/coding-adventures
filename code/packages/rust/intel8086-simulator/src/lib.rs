//! # Intel 8086 (1978) behavioral simulator
//!
//! Complete Rust port of the repository's Python Intel 8086 oracle. The
//! simulator models the full specified instruction surface, the architectural
//! 1 MiB segmented address space, the complete register/FLAGS state, and two
//! 256-byte I/O port banks.
//!
//! The public checked lifecycle provides atomic program loads, complete owned
//! snapshots, transactional bounded runs, typed failures, and full before/after
//! traces. The historical `step() -> String` and `run(&[u8])` conveniences stay
//! available for existing backend consumers.
//!
//! ```rust
//! use intel8086_simulator::Intel8086Simulator;
//!
//! let mut sim = Intel8086Simulator::new(1 << 20);
//! let result = sim.run_checked(&[0xb8, 42, 0, 0xf4], 10)?;
//! assert!(result.halted);
//! assert_eq!(result.final_state.ax, 42);
//! # Ok::<(), intel8086_simulator::Intel8086Error>(())
//! ```

pub mod decode;
pub mod encoding;
pub mod execute;
pub mod flags;
pub mod opcodes;
pub mod simulator;

pub use simulator::{
    ExecutionResult, Intel8086Error, Intel8086Simulator, Intel8086State, StepTrace,
};
