# m68k-backend

Motorola 68000 implementation of the `jit_core::backend::Backend` trait.
Lowers typed, monomorphised `Vec<CIRInstr>` to `Vec<u8>` of big-endian
Motorola 68000 machine code via [`m68k-encoder`](../m68k-encoder).
Emit-only — bytes go to [`m68k-simulator`](../m68k-simulator) for
execution.

Eighth lane of the [9-architecture expansion](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md),
mirroring [`mos6502-backend`](../mos6502-backend) / [`arm1-backend`](../arm1-backend)
/ [`armv7-backend`](../armv7-backend) / [`intel8008-backend`](../intel8008-backend)
in shape and intent.

## Scope (v0.1.0 — minimal viable)

| CIR op | Lowering |
|--------|----------|
| `const_*` (32-bit literal, `[i32::MIN, u32::MAX]`) | `MOVE.L #imm, D0` |
| `ret_*`, `ret_void` | `TRAP #15` (the pre-existing HALT convention) |
| Anything else | `UnsupportedOp` from `compile()`; `None` from `Backend::compile` |

No real register allocator — a trivial "last const var" scheme tracks
which single variable the most recent `const_*` wrote into `D0`; `ret_*`
only succeeds if it returns exactly that variable. Programs needing more
than one live value fall through to `UnsupportedOp`.

## Why `ret_*` lowers to `TRAP #15`, not `STOP #imm`

See `m68k-simulator`'s crate doc ("Halt convention") for the full
derivation. Short version: the pre-existing Python simulator's own
`state.py` documents both `STOP` and `TRAP #15` as halting conditions,
but its test suite's `_stop()` helper — used 100+ times — is `TRAP #15`,
making it the dominant, already-established idiom this lane mirrors
rather than inventing a fresh convention (the same rule
`mos6502-backend`'s `BRK` and `arm1-backend`'s pseudo-halt `SWI` each
followed for their own ISAs).

## Security: the termination check is a `bool`, not a byte comparison

A prior lane (Intel 8051) shipped a defensive "did I already emit a
terminator?" check that compared the *trailing emitted byte* against the
halt sentinel's byte value — unsound, because the sentinel's byte value
was also a valid data-immediate byte, so a `const_*` whose immediate
happened to end in that byte coincidentally produced a trailing byte
identical to the sentinel, fooling the check into skipping the real
terminator (fixed in `intel8051-backend` commit `19e360d`).

`TRAP #15`'s low byte (`0x4F`) is equally reachable as the low byte of a
`MOVE.L #imm, D0` immediate (`const_i64 79` → `...0x4F`, with no
following `ret`), so the same trap applies here. `compile_to_bytes`
tracks an explicit `terminated: bool` instead: set `true` only when a
real `ret_*`/`ret_void` arm pushes `encode_trap15()`, and reset to
`false` whenever a further `const_*` is emitted — never a byte/word-value
comparison against the halt encoding. See
`tests/test_backend.rs::const_ending_in_halt_low_byte_with_no_ret_still_appends_real_halt`
for the regression test.

## Wire format

Each instruction is 2-6 bytes, big-endian (the 68000's native byte
order). `m68k-encoder`'s bytes are already the wire format — unlike
`arm1-backend` (which flattens little-endian ARM1 words), there is no
endianness-conversion step here.

## Pinned byte sequence

| Program | CIR | Emitted bytes |
|---------|-----|----------------|
| IIR `42` | `const_i64 v=42; ret_i64 v` | `[0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A, 0x4E, 0x4F]` |
| `ret_void` only | `ret_void` | `[0x4E, 0x4F]` |
| Empty CIR | (none) | `[0x4E, 0x4F]` |

`MOVE.L #42, D0` = `[0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A]`; `TRAP #15` =
`[0x4E, 0x4F]`.

## Backend trait surface

| Trait method | Behaviour |
|---------------|-----------|
| `name()` | returns `"m68k"` |
| `compile(ir)` | returns `Some(bytes)` for supported CIR ops; `None` otherwise |
| `compile_function(ctx, ir)` | ignores `FunctionContext`; delegates to `compile` |
| `run(binary, args)` | **panics** with `"m68k backend is emit-only; load bytes into m68k-simulator to execute"` |

## Tests

17 unit/integration tests in `tests/test_backend.rs` pin the canonical
byte sequence and edge cases (zero, 32-bit range boundaries, bool,
multi-var fallthrough, unsupported op, empty CIR, `ret_void`,
`Backend::run` panics, `Backend::compile` vs the free `compile` function
agree, and the halt-lookalike-byte security regression above).

One test additionally loads the compiled bytes into `m68k-simulator`,
runs it, and asserts `D0 == 42` and `halted == true` after execution —
byte-for-byte parity is necessary but not sufficient; the emitted bytes
must actually execute correctly (and actually halt) in the new
simulator.

## Backlog

1. [ ] Real register allocator using the 68000's other 7 data registers
   and 8 address registers.
2. [ ] Arithmetic/logical CIR ops via `ADD`/`SUB`/`AND`/`OR`/`EOR`/`CMP`
   (already implemented in `m68k-simulator`; only the backend-side
   lowering is missing).
3. [ ] Comparisons and conditional branches via `Bcc`/`Scc`/`DBcc`
   (already implemented in `m68k-simulator`).
4. [ ] Direct calls (`JSR`/`RTS` pairing) — once this lands, `ret_*`
   could switch from `TRAP #15` to `RTS` for called functions (the
   `TRAP #15` halt would remain for the outermost program-exit case).
5. [ ] `Backend::run` wired to `m68k-simulator` for JIT execution
   (best-effort per the migration spec — "no working JIT" is an
   acceptable outcome for a historical-arch target).
