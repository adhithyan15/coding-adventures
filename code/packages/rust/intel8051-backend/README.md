# intel8051-backend

Intel 8051 (MCS-51) backend for `jit-core` / `aot-core`. Fourth lane
of the 9-architecture expansion following the pattern documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

**v0.1.0 scope**: minimal viable -- covers `const_*` (unsigned 8-bit
imm into the accumulator, `A`) + `ret_*` (the HALT sentinel `0xA5`).
Enough to compile the canonical Twig `42` program to
`lang-aot --emit=intel8051` byte-for-byte, verified against the
in-tree `intel8051-simulator`.

## Why the HALT sentinel, not self-jump detection?

The 8051 has no real HALT instruction -- a genuine running program
that's done spins forever (`SJMP $`) or waits for an interrupt, since
there's no OS to hand control back to. Self-jump detection (recognise
a fixed `SJMP $` pattern as "the program is done") is the historically
idiomatic convention and was seriously considered.

It was **not** used: this architecture already has a tested, shipped
HALT convention -- opcode `0xA5` (reserved/undefined on real
silicon), defined by the Python behavioral reference this crate's
sibling `intel8051-simulator` was ported from
(`intel8051_simulator.state.HALT_OPCODE`, spec 07p). Reusing it keeps
the Python and Rust simulators in agreement about what "done" means,
and is strictly simpler for an emit-only backend to produce and detect
than pattern-matching a self-loop. See `src/lib.rs`'s crate-level doc
comment for the full derivation, and
[`code/specs/intel8051-backend.md`](../../../specs/intel8051-backend.md)
for the spec writeup.

Full op coverage (add/sub/cmp/branches/calls) is **not** ported in
this PR -- future increments can add them, along with a real register
allocator over the 8051's `R0`-`R7` working registers and direct/
indirect RAM addressing.

See [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
