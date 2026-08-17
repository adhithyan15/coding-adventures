# Changelog — intel8051-backend

## v0.1.0 — 2026-08-17 — fourth lane of the 9-architecture expansion

Initial release. Minimal viable `Backend` trait impl for the Intel
8051 (MCS-51). Covers `const_*` + `ret_*` -- enough to compile the
canonical Twig `42` program to `[0x74, 0x2A, 0xA5]`
(`MOV A, #42; HALT`), verified byte-for-byte and by actually executing
the emitted bytes in `intel8051-simulator`.

### Public API

- `pub struct Intel8051Backend` with `impl Backend`:
  - `name() -> "intel8051"`
  - `compile(&[CIRInstr]) -> Option<Vec<u8>>`
  - `compile_function(&FunctionContext, &[CIRInstr]) -> Option<Vec<u8>>`
  - `run(_, _)` panics with "intel8051 backend is emit-only; load
    bytes into intel8051-simulator to execute"
- `pub fn compile(ctx, cir) -> Result<Vec<u8>, BackendError>`
- `pub enum BackendError` -- 4 diagnostic variants.

### Covered CIR ops

- `const_*` (unsigned 8-bit range `[0, 255]`) -> `MOV A, #imm`
- `ret_*`, `ret_void` -> the HALT sentinel (`0xA5`)
- Anything else -> `None` (graceful AOT/JIT fallback)

### Why the HALT sentinel instead of self-jump (`SJMP $`) detection

The 8051 has no real HALT instruction; the historically-idiomatic
"program is done" convention for a *real, running* 8051 program is an
infinite self-jump. This backend does not use that convention: its
sibling crate `intel8051-simulator` already ports an established,
tested HALT sentinel (opcode `0xA5`, reserved/undefined on real
silicon) from the existing Python behavioral reference
(`intel8051_simulator.state.HALT_OPCODE`, spec 07p) -- inventing a
second, different halt convention for the same architecture would
fracture parity between the Python and Rust simulators for no benefit,
and the sentinel is strictly simpler for an emit-only backend to
produce and for a simulator to detect (one opcode-equality check vs.
pattern-matching a two-instruction self-loop). See `src/lib.rs`'s
crate-level doc comment for the full derivation.

### What's NOT in this PR

Full op coverage (add/sub/and/or/xor/cmp/branches/calls) is
intentionally out of scope for the minimal-viable lane. Future
increments can add them, along with a real register allocator over
the 8051's `R0`-`R7` working registers.

### Tests

15 unit/integration tests pin the canonical `MOV A, #42; HALT` byte
sequence and edge cases (zero, 8-bit range boundaries -- negative and
>255 -- bool, multi-var fallthrough, unsupported op, empty CIR,
`ret_void`, `Backend::run` panics, `Backend::compile` vs. the free
`compile` function agree), plus:

- one test that loads the compiled bytes into `intel8051-simulator`
  and asserts `acc() == 42` and `halted() == true` after execution in
  exactly 2 steps -- not just a hand-asserted byte array;
- one converse test proving a genuinely unterminated `SJMP $`
  self-loop does NOT halt and DOES exhaust `run_loaded_with_limit`'s
  step budget, so the positive HALT assertion above is meaningful.
