//! # MOS 6502 (1975) behavioral simulator
//!
//! Rust port of `code/packages/python/mos6502-simulator` (Layer 07j) — see
//! [`code/specs/07j-mos6502-simulator.md`](../../../specs/07j-mos6502-simulator.md)
//! for the full ISA writeup (this crate documents the port, not the ISA
//! semantics again).
//!
//! The MOS 6502 (Chuck Peddle, MOS Technology, 1975) is one of the most
//! influential 8-bit CPUs ever made: at $25 (versus the Intel 8080's
//! $179), it powered the Apple II, Commodore 64, Atari 2600/8-bit line,
//! BBC Micro, and — via the Ricoh 2A03 variant — the NES/Famicom.
//!
//! Module split mirrors [`mips_r2000_simulator`]:
//!
//! ```text
//! opcodes.rs   -- the 151-opcode table (mnemonic, addressing mode) + the
//!                 BRK halt sentinel
//! decode.rs    -- fetch + addressing-mode resolution (combined, since the
//!                 6502's variable-length encoding makes them inseparable)
//! flags.rs     -- N/Z/V computation, P-byte pack/unpack, BCD add/sub
//! execute.rs   -- instruction executor (inherent-method style over
//!                 &mut Mos6502Simulator, not free functions -- see its
//!                 module doc for why)
//! simulator.rs -- top-level Mos6502Simulator with fetch-decode-execute
//! ```
//!
//! ## What makes the 6502 different from every fixed-width ISA in this repo
//!
//! - **Variable-length instructions.**  MIPS R2000/ARM1/RISC-V are all
//!   fixed 32-bit words.  The 6502 is 1-3 bytes per instruction, so
//!   `decode.rs` combines fetch and addressing-mode resolution into one
//!   step (see its module doc).
//! - **Tiny, irregular register file.**  Just `A` (accumulator), `X`/`Y`
//!   (index registers), an 8-bit stack pointer, and a 16-bit PC — no
//!   general-purpose register bank, unlike MIPS's 32 GPRs or ARM1's 16.
//!   `Mos6502Simulator` therefore exposes these as plain typed fields
//!   rather than reusing `cpu_simulator::RegisterFile` (which assumes a
//!   uniform-width array).
//! - **BCD (decimal-mode) arithmetic.**  `ADC`/`SBC` with the `D` flag set
//!   perform binary-coded-decimal correction — a famous 6502 emulation
//!   gotcha (NMOS leaves `N`/`V`/`Z` reflecting the *binary* result, not
//!   the BCD-corrected one).  See `flags.rs`/`execute.rs` module docs.
//! - **The indirect `JMP` page-wrap bug.**  `JMP ($10FF)` reads its high
//!   byte from `$1000`, not `$1100` — a real NMOS silicon bug this
//!   simulator replicates exactly (see `decode.rs`).
//! - **`BRK` is the HALT sentinel**, not a MIPS-style `SYSCALL`/`BREAK` or
//!   ARM1-style pseudo-`SWI`.  This matches the existing Python
//!   simulator's documented convention: *"BRK (opcode 0x00) is treated as
//!   HALT... matches the convention used throughout the simulator stack
//!   (HLT for 8080, TRAP for IBM 704, etc.)"* — see `opcodes::BRK_OPCODE`.
//!
//! ## Usage
//!
//! ```rust
//! use mos6502_simulator::Mos6502Simulator;
//!
//! let mut sim = Mos6502Simulator::new(65536);
//! sim.run(&[
//!     0xA9, 42,  // LDA #42
//!     0x00,      // BRK (halt)
//! ]);
//! assert_eq!(sim.a, 42);
//! assert!(sim.halted);
//! ```

pub mod decode;
pub mod encoding;
pub mod execute;
pub mod flags;
pub mod opcodes;
pub mod simulator;

pub use simulator::{ExecutionResult, Mos6502Simulator};
