//! # SPARC V8 (1987) behavioral simulator
//!
//! Rust port of
//! `code/packages/python/sparc-v8-simulator/src/sparc_v8_simulator/{state,simulator}.py`
//! (Layer 07r).  SPARC (Scalable Processor ARChitecture) was designed by
//! Sun Microsystems and first shipped in 1987 — the first **open** RISC
//! instruction-set standard (as opposed to a single vendor's
//! proprietary chip family), later powering Sun's SPARCstation
//! workstations and Solaris servers for two decades.
//!
//! Module split mirrors [`mips_r2000_simulator`], plus one SPARC-specific
//! addition (`registers.rs`) for the windowed register file:
//!
//! ```text
//! opcodes.rs   -- op / op2 / op3 field constant tables (Formats 1/2/3)
//! encoding.rs  -- encode_* helpers to construct machine code words
//! decode.rs    -- instruction decoder for all four instruction shapes
//! registers.rs -- windowed register file (8 globals + NWINDOWS x 16) + CWP
//! execute.rs   -- instruction executor + big-endian memory accessors
//! simulator.rs -- top-level SparcV8Simulator with fetch-decode-execute
//! ```
//!
//! ## What makes SPARC V8 different from every other simulator here
//!
//! - **Overlapping register windows.**  32 logical registers (8 shared
//!   globals + 24 windowed outs/locals/ins) map onto a larger physical
//!   file via a Current Window Pointer.  `SAVE`/`RESTORE` rotate the
//!   window; see `registers.rs` for the full `virt_to_phys` derivation.
//! - **A condition-code register**, not compare-into-GPR.  Unlike MIPS
//!   R2000 (`SLT`/`SLTU` write 0/1 to a GPR), SPARC has traditional
//!   PSR N/Z/V/C flags that `*cc`-suffixed ALU ops update and `Bicc`
//!   branches consume.
//! - **Big-endian memory** (same as MIPS R2000; unlike RISC-V/ARM/x86).
//! - **No branch-delay slots modeled** — matches the Python original's
//!   explicit simplification.
//! - **Fail-closed halting instead of exceptions** for `UDIV`/`SDIV`
//!   by zero, register-window overflow, and non-`TA` `Ticc` traps — see
//!   `execute.rs` module docs.
//!
//! ## Register-window scoping in this crate vs. `sparc-v8-backend`
//!
//! This simulator ports the **full** register-window machinery
//! (`SAVE`/`RESTORE`, `virt_to_phys`, `%i`/`%o`/`%l` addressing) —
//! nothing is stubbed here.  `sparc-v8-backend` (the `Backend`-trait
//! implementation one layer up) is the crate that scopes its v0.1.0
//! CIR lowering to globals-only (`%g0`/`%o0`), since the minimal-viable
//! `const_*`/`ret_*` program never needs `SAVE`/`RESTORE` — see that
//! crate's docs for the rationale.
//!
//! ## Usage
//!
//! ```rust
//! use sparc_v8_simulator::SparcV8Simulator;
//! use sparc_v8_simulator::encoding::*;
//!
//! let mut sim = SparcV8Simulator::new(65536);
//! sim.run_instructions(&[
//!     encode_add_imm(8, 0, 1),   // %o0 = %g0 + 1
//!     encode_add_imm(9, 0, 2),   // %o1 = %g0 + 2
//!     encode_add(10, 8, 9),      // %o2 = %o0 + %o1
//!     encode_ta(0),               // ta 0 -- halt
//! ]);
//! assert_eq!(sim.regs.read(10), 3);
//! ```

pub mod decode;
pub mod encoding;
pub mod execute;
pub mod opcodes;
pub mod registers;
pub mod simulator;

pub use simulator::{ExecutionResult, SparcV8Simulator};
