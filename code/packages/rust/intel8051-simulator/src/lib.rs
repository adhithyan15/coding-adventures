//! # `intel8051-simulator` — Intel 8051 (MCS-51) behavioral simulator.
//!
//! ## Historical context
//!
//! The **Intel 8051** (MCS-51, 1980) is Intel's first single-chip
//! microcontroller — CPU, RAM, ROM, timers, a UART, and parallel I/O
//! ports on one die.  It defined the microcontroller as a product
//! category and went on to become, by unit count, **the most-
//! manufactured CPU architecture in history**: over 20 billion units,
//! still fabricated today by dozens of licensees (Atmel/Microchip's
//! AT89 family, Philips/NXP's 80C51, Silicon Labs' EFM8, and many
//! more). It underpins decades of embedded controllers, keyboards,
//! modems, printers, medical devices, and automotive ECUs.
//!
//! Rust port of `code/packages/python/intel8051-simulator` (Layer
//! 07p — see `code/specs/07p-intel-8051-simulator.md` for the full
//! architecture writeup and instruction-set tables); fourth lane of
//! the 9-architecture expansion described in
//! `code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md`.
//!
//! ## Harvard architecture — four independent memory spaces
//!
//! Unlike every other historical-arch simulator in this codebase
//! (RISC-V/MIPS/8080/ARM1 are all flat-memory), the 8051 has
//! genuinely separate address spaces:
//!
//! ```text
//! code   (64 KiB) — program memory: fetched by PC, read-only via MOVC
//! iram  (256 B)   — internal RAM (0x00-0x7F) + SFRs (0x80-0xFF)
//! xdata  (64 KiB) — external data memory: read/write only via MOVX
//! ```
//!
//! `code/specs/07p-intel-8051-simulator.md`'s "Architecture" section
//! has the full memory-map diagram, register-bank layout, and
//! bit-addressable-area details this module split implements.
//!
//! ## Module layout
//!
//! | Module | Contents |
//! |--------|----------|
//! | [`opcodes`] | Every memory-size, SFR-address, PSW-bit-mask, and instruction-opcode constant. |
//! | [`encoding`] | Pure `encode_*` helpers — what [`intel8051-encoder`](../intel8051-encoder) re-exports. |
//! | [`decode`] | Pure opcode → operand-length + operand-byte decoding, no CPU-state access. |
//! | [`execute`] | Instruction semantics — mutates simulator state given a decoded instruction. |
//! | [`simulator`] | The public [`Intel8051Simulator`] struct tying decode+execute together. |
//!
//! ## HALT convention
//!
//! The real 8051 has no HALT instruction — the era's idiom for "the
//! program is done" is either an infinite self-jump (`SJMP $`) or a
//! wait for the next interrupt.  This simulator (matching its already-
//! shipped Python reference, `intel8051_simulator.state.HALT_OPCODE`)
//! uses opcode `0xA5` — reserved/undefined in every MCS-51 opcode map
//! — as a HALT sentinel: executing it sets [`Intel8051Simulator::
//! halted`] and stops the fetch-decode-execute loop.  See
//! `code/specs/intel8051-backend.md` for the full rationale behind
//! reusing this convention (rather than inventing self-jump detection)
//! for `intel8051-backend`'s `ret_*` lowering.
//!
//! ## Quick start
//!
//! ```
//! use intel8051_simulator::Intel8051Simulator;
//!
//! // MOV A, #42 ; HALT
//! let mut sim = Intel8051Simulator::new();
//! sim.load_program(&[0x74, 42, 0xA5], 0);
//! let result = sim.run_loaded_with_limit(100);
//! assert!(result.halted);
//! assert_eq!(sim.acc(), 42);
//! ```

pub mod decode;
pub mod encoding;
pub mod execute;
pub mod opcodes;
pub mod simulator;

pub use simulator::{ExecutionResult, Intel8051Simulator};
