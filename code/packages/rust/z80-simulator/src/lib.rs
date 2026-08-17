//! # Zilog Z80 (1976) behavioral simulator
//!
//! Rust port of `code/packages/python/z80-simulator` — see
//! [`code/specs/z80-encoder.md`](../../../specs/z80-encoder.md) /
//! [`code/specs/z80-backend.md`](../../../specs/z80-backend.md) for the
//! encoder/backend writeups this crate feeds.  Seventh lane of the
//! 9-architecture expansion documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! Module split mirrors [`mips_r2000_simulator`] / [`intel8080_simulator`]:
//!
//! ```text
//! opcodes.rs   -- opcode / register / condition-code constant tables
//! encoding.rs  -- encode_* helpers to construct machine code byte sequences
//! decode.rs    -- variable-length instruction decoder (1-4 bytes)
//! execute.rs   -- instruction executor + register/flag state
//! simulator.rs -- top-level Z80Simulator with fetch-decode-execute
//! ```
//!
//! ## The Z80 is an Intel 8080 superset
//!
//! Every valid 8080 opcode is a valid Z80 opcode with **identical**
//! semantics and **identical** byte encoding.  This crate's base
//! (unprefixed, non-`EX`/`EXX`/`DJNZ`/`JR`) instruction set is therefore a
//! direct structural port of `intel8080_simulator` (same register/pair/
//! ALU/condition-code field encodings), renamed to Zilog's assembler
//! mnemonics (`LD` instead of `MOV`/`MVI`/`LXI`/…, `JP` instead of `JMP`,
//! `CP` instead of `CMP`, …).  See `code/specs/z80-encoder.md` for the
//! full byte-identity table, and
//! `code/packages/rust/z80-backend/tests/test_backend.rs` for a direct
//! cross-architecture assertion that `z80-backend`'s minimal-viable
//! `LD A,n; HALT` output matches `intel8080-backend`'s `MVI A,n; HLT`
//! output byte-for-byte.
//!
//! ## What's new relative to `intel8080_simulator`
//!
//! - **Alternate register bank** (`A'/F'/B'/C'/D'/E'/H'/L'`) — swapped in
//!   via `EX AF,AF'` / `EXX`.
//! - **Index registers** `IX`/`IY` — this v0.1.0 ports only `LD IX/IY,nn`
//!   and `INC IX/IY` (the "IX/IY basics" the migration guidance calls
//!   for); full displacement addressing (`LD r,(IX+d)`, ALU-via-`(IX+d)`,
//!   …) is not ported.
//! - **`CB`-prefix** — bit manipulation (`BIT`/`SET`/`RES`) and extended
//!   rotate/shift (`RLC`/`RRC`/`RL`/`RR`/`SLA`/`SRA`/`SLL`/`SRL`) against
//!   any of the 8 `r`-coded operands.  Fully ported.
//! - **Relative jumps** — `DJNZ e`, `JR e`, and the four conditional `JR`
//!   forms.  Fully ported.
//! - **An extra flag** — `N` (add/subtract), needed for correct `DAA`
//!   behaviour; the `P/V` flag is dual-purpose (parity after logical ops,
//!   signed overflow after arithmetic ops) rather than 8080's
//!   parity-only `P`.
//! - **`ED`-prefix is NOT ported** in this v0.1.0 — see the module docs
//!   on `decode::decode_ed` for the deliberate scope cut (16-bit
//!   `ADC`/`SBC HL,rp`, the block-transfer/compare/I-O instruction
//!   families, `LD A,I`/`LD A,R`, `NEG`, interrupt-mode selection).  Every
//!   `ED`-prefixed opcode decodes to `"undefined"`, which `execute.rs`
//!   treats as a fail-closed halt.
//!
//! ## Usage
//!
//! ```rust
//! use z80_simulator::Z80Simulator;
//! use z80_simulator::encoding::*;
//! use z80_simulator::opcodes::*;
//!
//! let mut sim = Z80Simulator::new(65536);
//! sim.run_instructions(&[
//!     encode_ld_r_n(REG_B, 1),   // B = 1
//!     encode_ld_a_n(2),          // A = 2
//!     vec![encode_alu_reg(ALU_ADD, REG_B)], // A = A + B = 3
//!     vec![HALT],
//! ]);
//! assert_eq!(sim.regs.a, 3);
//! ```

pub mod decode;
pub mod encoding;
pub mod execute;
pub mod opcodes;
pub mod simulator;

pub use simulator::{ExecutionResult, Z80Simulator};
