//! # Motorola 68000 (1979) behavioral simulator
//!
//! Rust port of `code/packages/python/motorola-68000-simulator` (Layer
//! 07n) — see
//! [`code/specs/07n-motorola-68000-simulator.md`](../../../specs/07n-motorola-68000-simulator.md)
//! for the full ISA writeup (this crate documents the port, not the ISA
//! semantics again).  Eighth lane of the 9-architecture expansion
//! following the pattern documented in
//! [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
//!
//! The Motorola 68000 (1979) is the CPU clean-ISA advocates point to as
//! "what the 8086 should have been" — a landmark 16/32-bit processor with
//! 8 general-purpose 32-bit data registers, 8 address registers, and 14
//! genuinely orthogonal addressing modes.  It powered the original
//! Apple Macintosh (1984), Commodore Amiga (1985), Atari ST (1985), early
//! Sun-1/Sun-2 workstations, and the Sega Genesis/Mega Drive (1988).
//!
//! Module split mirrors [`mos6502_simulator`] (this repo's most recent
//! from-scratch simulator port, and the closer structural template —
//! `mips-r2000-simulator` has not landed in this branch at the time this
//! crate was written):
//!
//! ```text
//! opcodes.rs   -- shared size-code tables, condition-code predicates,
//!                 the HALT sentinel, masks/sign-extension helpers
//! decode.rs    -- effective-address classification/resolution + the
//!                 PC-relative fetch helpers every instruction shares
//! flags.rs     -- N/Z/V/C/X computation (direct port of flags.py)
//! execute.rs   -- one function per opword "line" (the top 4 bits),
//!                 inherent-method-style over &mut M68kSimulator
//! simulator.rs -- top-level M68kSimulator with fetch-decode-execute
//! encoding.rs  -- encode_* helpers (subset used by tests / m68k-encoder)
//! ```
//!
//! ## What makes the 68000 different from every other ISA in this repo
//!
//! - **Bit-field decoding, not a flat opcode table.**  MOS 6502 and
//!   Intel 8080-family CPUs assign each opcode *byte* a fixed meaning —
//!   `opcodes::lookup(u8) -> (mnemonic, mode)` is a complete decode.  The
//!   68000's 16-bit opword instead groups instructions by their top 4
//!   bits ("line 0" through "line F" — even the Python original's own
//!   module doc calls this "a rough category, not a complete opcode"),
//!   and each line further branches on its own bit sub-fields.  There is
//!   no single table to port; `decode.rs`/`execute.rs` mirror the
//!   Python original's per-line dispatch methods directly.
//! - **Orthogonal addressing modes.**  Almost any instruction that reads
//!   an operand can read it from *any* of 11 effective-address variants
//!   (register direct, 6 memory-indirect flavours, 2 absolute forms, 2
//!   PC-relative forms) plus immediate — this crate ports 8 of them (see
//!   `decode.rs`'s module doc for exactly which, and why the 3
//!   indexed/PC-relative modes are deferred).
//! - **Big-endian, unlike every other simulator in this repo.**  MIPS
//!   R2000/ARM1/RV32I/MOS 6502 are all little-endian (or byte-oriented,
//!   for the 6502).  The 68000 stores the most-significant byte at the
//!   lowest address — `decode.rs`'s `mem_read`/`mem_write` and
//!   `fetch_word`/`fetch_long` all assemble bytes big-endian, matching
//!   the Python original exactly.
//! - **A real 24-bit address bus (16 MiB).**  Every computed effective
//!   address is masked to `0x00FFFFFF`
//!   ([`opcodes::ADDR_MASK`](crate::opcodes::ADDR_MASK)) — the backing
//!   `Memory` a caller constructs may be smaller (tests routinely use a
//!   few KiB), same convention every other Rust ISA simulator in this
//!   repo uses.
//! - **`TRAP #15` is the HALT sentinel**, not a MIPS-style `SYSCALL`, an
//!   ARM1-style pseudo-`SWI`, or a MOS-6502-style `BRK`.  See the "Halt
//!   convention" section below for the full derivation.
//!
//! ## Halt convention: why `TRAP #15`, not `STOP #imm`
//!
//! The 68000 has **two** genuine, silicon-real instructions that halt
//! program flow, and the pre-existing Python simulator's own
//! `state.py` documents both: *"halted: True after STOP or TRAP #15
//! executes."*  `STOP #imm` is architecturally the more literal "halt"
//! — a privileged instruction that loads an immediate into the status
//! register and stops the CPU until an interrupt occurs.  `TRAP #15`
//! is architecturally a software-interrupt/trap vector call (trap
//! number 15 specifically, out of 16); the Python simulator special-
//! cases it as a halt rather than modelling trap-vector dispatch.
//!
//! Both are equally "real" per `state.py`'s own docs, so this port
//! follows this repo's own established rule for such ties (see
//! `mos6502-encoder`'s and `arm1-encoder`'s crate docs): **mirror
//! whatever the pre-existing reference already does, don't invent a
//! fresh convention.**  Inspecting the Python original's own test
//! suite settles it — `test_instructions.py` defines a `_stop()`
//! helper (*"TRAP #15 — halts simulation without modifying SR"*) that
//! is `_w(0x4E4F)`, i.e. `TRAP #15`, and every one of that file's 100+
//! test programs (plus `test_programs.py`'s 18) ends its program with
//! that helper.  `STOP #imm` appears exactly once, in the module-level
//! doctest example, and nowhere else — a curiosity, not the
//! established idiom.  `TRAP #15` is therefore the dominant,
//! already-established halt convention this port mirrors; see
//! [`opcodes::TRAP_15_WORD`](crate::opcodes::TRAP_15_WORD) for the
//! encoding and `m68k-backend`'s crate doc for how `ret_*`/`ret_void`
//! lower to it.
//!
//! `STOP #imm` is still ported faithfully in `execute.rs` (any program
//! that happens to use it directly still halts correctly) — it just
//! isn't the convention `m68k-backend` emits.
//!
//! ## Usage
//!
//! ```rust
//! use m68k_simulator::M68kSimulator;
//! use m68k_simulator::encoding::{assemble, encode_move_l_imm_to_dn, encode_trap15};
//!
//! let mut sim = M68kSimulator::new(65536);
//! sim.run(&assemble(&[
//!     encode_move_l_imm_to_dn(0, 42), // MOVE.L #42, D0
//!     encode_trap15(),                 // TRAP #15 (halt)
//! ]));
//! assert_eq!(sim.d[0], 42);
//! assert!(sim.halted);
//! ```

pub mod decode;
pub mod encoding;
pub mod execute;
pub mod flags;
pub mod opcodes;
pub mod simulator;

pub use simulator::{ExecutionResult, M68kSimulator};
