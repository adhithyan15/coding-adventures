//! SPARC V8 gate-level simulator.
//!
//! Persistent architectural state is clocked through D flip-flops. Arithmetic,
//! logic, shifts, multiplication, division, decode, and register writes are
//! built from repository gates and ripple-carry networks. Host integers remain
//! at the simulator boundary for addresses, instruction fields, lifecycle
//! metadata, and bit-vector conversion.
//!
//! # Architecture overview
//!
//! ```text
//!  ┌─────────────────────────────────────────────────┐
//!  │                   SparcCpu                      │
//!  │  ┌────────────┐  ┌──────────┐  ┌────────────┐  │
//!  │  │ RegisterFile│  │  Alu     │  │ DffMemory │  │
//!  │  │ 56 phys regs│  │ gate ops │  │ 64 KiB    │  │
//!  │  │ PSR,PC/nPC,Y│  └──────────┘  └───────────┘  │
//!  │  └────────────┘                                  │
//!  └─────────────────────────────────────────────────┘
//! ```

pub mod alu;
pub mod bits;
pub mod cpu;
pub mod decoder;
pub mod register_file;
mod state;

pub use cpu::{SparcCpu, FLIP_FLOP_COUNT};
pub use sparc_v8_simulator::{ExecutionResult, SparcError, SparcState, StepTrace};
pub use state::DffMemory;
