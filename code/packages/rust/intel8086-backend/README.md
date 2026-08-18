# Intel 8086 Backend (Rust)

Implements the `jit-core`/`aot-core` `Backend` trait for the Intel 8086
(1978) — the ninth and **final** lane of the 9-architecture expansion
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

Lowers a `Vec<CIRInstr>` into Intel 8086 machine code bytes via
`intel8086-encoder`. Emit-only — `Backend::run` panics; bytes are meant
to be loaded into `intel8086-simulator` (or any compatible external
8086/8088 emulator).

## Scope (v0.1.0 — minimal viable)

| CIR op | Lowering |
|--------|----------|
| `const_*` (16-bit unsigned literal, `[0, 65535]`) | `MOV AX, #imm16` |
| `ret_*`, `ret_void` | `HLT` |
| Anything else | `None` (compile failure — same graceful AOT/JIT fallback every other backend gets) |

A trivial "last const var" single-register (`AX`) allocator — the same
scheme `mips-r2000-backend`/`arm1-backend`/`mos6502-backend` use. Full op
coverage (arithmetic, register-to-register moves, control flow) is
intentionally not wired into this backend yet, even though
`intel8086-simulator` implements a curated core of them — a future
increment can extend `compile_to_bytes`.

## Why `HLT`, not a pseudo-halt?

The Intel 8086 has a genuine, single-byte hardware halt instruction
(`0xF4`) — real silicon behaviour, not a simulator-level convention this
lane invented (ARM1's `SWI`) or inherited (MOS 6502's repurposed `BRK`).
See this crate's `src/lib.rs` module doc and
`code/specs/intel8086-backend.md` for the full comparison across all
nine lanes.

## The `terminated: bool` pattern

A real bug class was found and fixed in **four** prior lanes of this
campaign (Intel 8051, Intel 8080, MOS 6502, Zilog Z80): the defensive
"is the program already terminated?" check was written as a trailing-
byte-value comparison (`bytes.last() == Some(&HALT_BYTE)`) or an
`is_empty()` check. Both are unsound — a legitimate `const_*`
immediate's encoded bytes can numerically collide with the halt
opcode's byte value (`MOV AX,0xF400` encodes with `0xF4` as its
trailing byte, identical to `HALT_BYTE`, despite never having executed
a real halt).

This backend tracks an explicit `terminated: bool` local instead:

- Starts `false`.
- Set `true` **only** when a genuine `ret_*`/`ret_void` arm pushes a
  real `HLT`.
- Reset to `false` whenever any further `const_*` is emitted afterward.
- A real `HLT` is appended at the end if `terminated` is still `false`
  — regardless of what byte value happens to sit last in the buffer.

See `tests/test_backend.rs`'s
`const_whose_encoded_high_byte_collides_with_halt_opcode_still_gets_real_terminator`
for the regression test that would fail against a naive trailing-byte-
comparison implementation.

## Tests (19 tests across `tests/test_backend.rs`)

Byte-for-byte parity for the canonical `const 42; ret` program
(`[0xB8, 0x2A, 0x00, 0xF4]`), verified both as a hand-derived byte array
and by actually executing the emitted bytes in `intel8086-simulator`
(through non-zero-`CS` segmented addressing) and asserting `sim.ax == 42`
and `sim.halted == true` — plus the byte-collision regression test above
and its sibling covering the low-byte-collision case, immediate-range
validation, `Backend` trait conformance, and the `run()` emit-only panic.
