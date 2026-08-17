# Changelog — arm1-backend

## v0.1.0 — 2026-08-17 — second lane of the 9-architecture expansion

Initial release.  Minimal viable `Backend` trait impl for ARM1
(ARMv1).  Covers `const_*` + `ret_*` — enough to compile the
canonical Twig `42` program to
`[0x2A, 0x00, 0xA0, 0xE3, 0x56, 0x34, 0x12, 0xEF]`
(`MOV R0, #42; SWI #0x123456`), verified byte-for-byte and by
actually executing the emitted bytes in `arm1-simulator`.

### Public API

- `pub struct Arm1Backend` with `impl Backend`:
  - `name() -> "arm1"`
  - `compile(&[CIRInstr]) -> Option<Vec<u8>>` (little-endian flattened)
  - `compile_function(&FunctionContext, &[CIRInstr]) -> Option<Vec<u8>>`
  - `run(_, _)` panics with "arm1 backend is emit-only; load bytes
    into arm1-simulator to execute"
- `pub fn compile(ctx, cir) -> Result<Vec<u8>, BackendError>`
- `pub enum BackendError` — 4 diagnostic variants.

### Covered CIR ops

- `const_*` (unrotated 8-bit range `[0, 255]`) → `MOV R0, #imm`
- `ret_*`, `ret_void` → pseudo-halt `SWI #0x123456`
- Anything else → `None` (graceful AOT/JIT fallback)

### Why a pseudo-halt instead of `BX LR`

ARM1/ARMv1 predates the link-register-return convention `armv7-backend`
uses — there is no `BX` instruction, and the era's `MOVS PC, R14`
return idiom needs a live `R14` set by a preceding `BL` (i.e. a
caller), which the minimal-viable scope never establishes.
`arm1-simulator` already defines a pseudo-halt (`SWI #0x123456`,
intercepted by `execute_swi` to set `halted() == true`) for exactly
this "the program is done" signal, so `ret_*`/`ret_void` lower to it.
See `src/lib.rs`'s crate-level doc comment for the full derivation.

### What's NOT in this PR

Full op coverage (add/sub/and/or/xor/cmp/branches/calls) is
intentionally out of scope for the minimal-viable lane.  Future
increments can add them, along with a real register allocator over
ARM1's other 14 general-purpose registers.

### Tests

14 unit/integration tests pin the canonical
`MOV R0, #42; SWI #0x123456` byte sequence and edge cases (zero,
8-bit range boundaries — negative and >255 — bool, multi-var
fallthrough, unsupported op, empty CIR, `ret_void`, `Backend::run`
panics, `Backend::compile` vs the free `compile` function agree),
plus one test that loads the compiled bytes into `arm1-simulator`
and asserts `R0 == 42` and `halted() == true` after execution — not
just a hand-asserted byte array.
