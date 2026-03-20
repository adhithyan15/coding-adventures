//! # GPU Core -- a pluggable, educational GPU processing element simulator.
//!
//! This crate implements a generic GPU core (processing element) that can simulate
//! any vendor's hardware by plugging in different instruction sets. It is a Rust port
//! of the Python `gpu-core` package, designed for learning how GPUs execute programs.
//!
//! ## Architecture
//!
//! A GPU core is the smallest independently programmable compute unit on a GPU.
//! Unlike CPU cores (which are complex, out-of-order, speculative), GPU cores are
//! simple in-order processors that achieve performance through massive parallelism.
//!
//! ```text
//! +-------------------------------------------+
//! |              GPU Core                      |
//! |                                            |
//! |  +---------+    +-----------------+        |
//! |  | Program |---→|   Fetch         |        |
//! |  | Memory  |    |   instruction   |        |
//! |  +---------+    |   at PC         |        |
//! |                 +-------+---------+        |
//! |                         |                  |
//! |                 +-------v---------+        |
//! |  +-----------+  |   ISA.execute() |        |
//! |  | Register  |<-|   (pluggable!)  |-->Trace|
//! |  | File      |->|                 |        |
//! |  +-----------+  +-------+---------+        |
//! |                         |                  |
//! |  +-----------+  +-------v---------+        |
//! |  |  Local   |<--|  Update PC      |        |
//! |  |  Memory  |   +-----------------+        |
//! |  +-----------+                             |
//! +-------------------------------------------+
//! ```
//!
//! ## Modules
//!
//! - **[`protocols`]** -- The pluggable interfaces: `ProcessingElement` and
//!   `InstructionSet` traits, plus the `ExecuteResult` type.
//! - **[`registers`]** -- `FPRegisterFile`: configurable floating-point register storage.
//! - **[`memory`]** -- `LocalMemory`: byte-addressable scratchpad with FP load/store.
//! - **[`opcodes`]** -- `Opcode` enum, `Instruction` struct, and helper constructors
//!   (`fadd`, `fmul`, `limm`, `halt`, etc.).
//! - **[`generic_isa`]** -- `GenericISA`: the default, vendor-neutral instruction set
//!   with 16 opcodes covering arithmetic, memory, data movement, and control flow.
//! - **[`core`]** -- `GPUCore`: the main simulator that ties everything together.
//! - **[`trace`]** -- `GPUCoreTrace`: execution trace records for observability.
//!
//! ## Quick Start
//!
//! ```
//! use gpu_core::{GPUCore, GenericISA};
//! use gpu_core::opcodes::{limm, fadd, halt};
//!
//! // Create a core with the generic ISA
//! let mut core = GPUCore::new(Box::new(GenericISA));
//!
//! // Write a simple program: compute 3.0 + 4.0
//! core.load_program(vec![
//!     limm(0, 3.0),   // R0 = 3.0
//!     limm(1, 4.0),   // R1 = 4.0
//!     fadd(2, 0, 1),  // R2 = R0 + R1
//!     halt(),          // stop
//! ]);
//!
//! // Run the program
//! let traces = core.run(1000).unwrap();
//!
//! // Check the result
//! assert_eq!(core.registers.read_float(2), 7.0);
//!
//! // Inspect the execution trace
//! for trace in &traces {
//!     println!("{}", trace.format());
//! }
//! ```
//!
//! ## Pluggable ISA Design
//!
//! The key insight is that all GPU cores do the same basic thing (fetch, execute,
//! update PC), but the *instruction set* varies by vendor. By making the ISA a
//! trait, we can simulate any vendor's hardware:
//!
//! ```text
//! impl InstructionSet for PtxIsa { ... }   // NVIDIA
//! impl InstructionSet for GcnIsa { ... }   // AMD
//! impl InstructionSet for XeIsa  { ... }   // Intel
//! impl InstructionSet for MaliIsa { ... }  // ARM
//! ```

pub mod protocols;
pub mod registers;
pub mod memory;
pub mod opcodes;
pub mod generic_isa;
pub mod core;
pub mod trace;

// Re-export the most commonly used types for convenience.
pub use crate::core::GPUCore;
pub use crate::generic_isa::GenericISA;
pub use crate::protocols::{ExecuteResult, InstructionSet, ProcessingElement};
pub use crate::registers::FPRegisterFile;
pub use crate::memory::LocalMemory;
pub use crate::trace::GPUCoreTrace;
pub use crate::opcodes::{Opcode, Instruction};
