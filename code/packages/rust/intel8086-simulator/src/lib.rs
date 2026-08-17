//! # Intel 8086 (1978) behavioral simulator
//!
//! Rust port of `code/packages/python/intel-8086-simulator` (Layer 07m)
//! — see
//! [`code/specs/07m-intel-8086-simulator.md`](../../../specs/07m-intel-8086-simulator.md)
//! for the full ISA writeup (this crate documents the port and its
//! deliberately curated scope, not the ISA semantics again).
//!
//! On 8 June 1978, Intel announced the 8086 — a 16-bit extension of the
//! 8080 architecture (NOT source- or binary-compatible with it, despite
//! the lineage) that introduced the segmented memory model and the
//! ModRM addressing byte. The IBM PC (1981) shipped with its cheaper
//! 8-bit-bus sibling, the 8088 — making the 8086 architecture family the
//! ancestor of every x86 CPU made since, and the origin point of the
//! "PC-compatible" industry.
//!
//! Module split mirrors [`mips_r2000_simulator`]/[`mos6502_simulator`]:
//!
//! ```text
//! opcodes.rs   -- curated opcode table (mnemonic, decode Format) +
//!                 register-index constants + the HLT halt opcode
//! flags.rs     -- CF/PF/AF/ZF/SF/OF computation, ported from flags.py
//! decode.rs    -- fetch + operand decode (register-only ModRM; see
//!                 module doc for why memory operands are a decode error
//!                 rather than silently misinterpreted)
//! execute.rs   -- instruction executor (methods over
//!                 &mut Intel8086Simulator, mirroring mos6502-simulator's
//!                 shape, not mips-r2000's decomposed-field style)
//! simulator.rs -- top-level Intel8086Simulator: segmented CS:IP fetch,
//!                 registers, fetch-decode-execute loop, phys_addr()
//! encoding.rs  -- encode_* helpers (subset used by tests /
//!                 intel8086-encoder)
//! ```
//!
//! ## What makes the 8086 structurally different from every other
//! simulator in this repo: segmented memory
//!
//! Every other historical-arch simulator here (MIPS R2000, ARM1,
//! MOS 6502, RISC-V, …) uses **flat** memory: the program counter *is*
//! the address. The 8086 never does — physical addresses are always
//! `(segment_register << 4) + offset`, giving a 1 MiB address space built
//! from 16-bit segment×offset pairs across four segment registers
//! (`CS`/`DS`/`SS`/`ES`). Instruction fetch always goes through `CS:IP`.
//! This is **structural, not deferrable**: even the trivial
//! `MOV AX,imm16; HLT` program `intel8086-backend`'s smoke test compiles
//! has its very first opcode byte read via segmented addressing — see
//! `simulator.rs`'s module doc and [`simulator::phys_addr`] for the full
//! derivation and the exact formula (`code/packages/python/
//! intel-8086-simulator`'s `_phys`, ported faithfully).
//!
//! ## Curated opcode subset (deliberately scoped)
//!
//! This crate does **not** port the Python reference's full ~1670-line
//! instruction set. It covers a curated core: register-immediate data
//! transfer (`MOV reg16/8,imm`), register-to-register data transfer and
//! ALU ops via ModRM (**mod=11 only** — no memory effective-address
//! computation), accumulator-immediate ALU ops, `INC`/`DEC reg16`, and
//! the real `HLT` halt instruction. See `opcodes.rs`'s module doc for the
//! full rationale.
//!
//! **Deferred to a future increment** (all present in the Python
//! reference, none ported here):
//!
//! - Memory-operand addressing (`mod != 11` ModRM forms: `[BX+SI]`,
//!   `[BP+DI+disp8]`, `[disp16]`, …) and therefore all `MOV`/ALU forms
//!   that read or write memory.
//! - Segment-override prefixes (`26`/`2E`/`36`/`3E`), `LOCK` (`F0`),
//!   `REP`/`REPNE` string-op prefixes.
//! - String operations (`MOVS`/`CMPS`/`STOS`/`LODS`/`SCAS`).
//! - Stack operations (`PUSH`/`POP`/`PUSHF`/`POPF`/`CALL`/`RET`).
//! - Control flow (`JMP`, conditional jumps, `LOOP`, interrupts).
//! - `MUL`/`IMUL`/`DIV`/`IDIV`, shift/rotate group (`SHL`/`SHR`/`SAR`/
//!   `ROL`/`ROR`/`RCL`/`RCR`), `NOT`/`NEG`/`TEST`.
//! - BCD adjust instructions (`DAA`/`DAS`/`AAA`/`AAS`/`AAM`/`AAD`).
//! - `XCHG`, `LEA`, `LDS`/`LES`, `LAHF`/`SAHF`, `CBW`/`CWD`, `XLAT`.
//! - `MOV sreg,r/m` / `MOV r/m,sreg` (segment-register data transfer).
//! - I/O port instructions (`IN`/`OUT`) and memory-mapped port
//!   emulation.
//! - `TF`/`IF` flag semantics beyond storage (no interrupt or
//!   single-step machinery).
//!
//! ## Usage
//!
//! ```rust
//! use intel8086_simulator::Intel8086Simulator;
//!
//! let mut sim = Intel8086Simulator::new(65536);
//! sim.run(&[
//!     0xB8, 42, 0x00, // MOV AX, 42
//!     0xF4,           // HLT
//! ]);
//! assert_eq!(sim.ax, 42);
//! assert!(sim.halted);
//! ```

pub mod decode;
pub mod encoding;
pub mod execute;
pub mod flags;
pub mod opcodes;
pub mod simulator;

pub use simulator::{ExecutionResult, Intel8086Simulator};
