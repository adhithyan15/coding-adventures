# Changelog — sparc-v8-backend

## v0.1.0 — 2026-08-17 — sixth lane of the 9-architecture expansion

Initial release.  Minimal viable `Backend` trait impl for SPARC V8.
Covers `const_*` + `ret_*` — enough to compile the canonical Twig
`42` program to
`[0x90, 0x00, 0x20, 0x2A, 0x91, 0xD0, 0x20, 0x00]`
(`ADD %g0, 42, %o0; ta 0`), verified byte-for-byte and by actually
executing the emitted bytes in `sparc-v8-simulator`.

### Public API

- `pub struct SparcV8Backend` with `impl Backend`:
  - `name() -> "sparc-v8"`
  - `compile(&[CIRInstr]) -> Option<Vec<u8>>` (big-endian flattened)
  - `compile_function(&FunctionContext, &[CIRInstr]) -> Option<Vec<u8>>`
  - `run(_, _)` panics with "sparc-v8 backend is emit-only; load bytes
    into sparc-v8-simulator to execute"
- `pub fn compile(ctx, cir) -> Result<Vec<u8>, BackendError>`
- `pub enum BackendError` — 4 diagnostic variants.

### Covered CIR ops

- `const_*` (13-bit signed range `[-4096, 4095]`) → `ADD %g0, imm, %o0`
- `ret_*`, `ret_void` → `ta 0` (trap always, HALT)
- Anything else → `None` (graceful AOT/JIT fallback)

### Why `%o0`, not a `%g` register

`%o0` is the real SPARC ABI's integer return-value register. This
backend never emits `SAVE`/`RESTORE`, so the Current Window Pointer
never moves, and `%o0` resolves to a fixed physical register for the
whole program — no window-rotation risk. See `src/lib.rs`'s
crate-level doc comment for the full derivation.

### Why `ta 0` instead of `RESTORE` + `JMPL`

Real SPARC subroutine return needs a live caller context (`%i7` set by
a preceding `CALL`) the minimal-viable scope never establishes.
`sparc-v8-simulator` already defines `ta 0` as its HALT convention
(matching the Python original's `state.HALT_WORD`), so `ret_*`/
`ret_void` lower to it.

### What's NOT in this PR

Full op coverage (add/sub/and/or/xor/cmp/branches/calls) is
intentionally out of scope for the minimal-viable lane. Future
increments can add them, along with a real register allocator over
SPARC V8's other windowed registers and `SAVE`/`RESTORE` support for
real function calls — `sparc-v8-simulator` already implements the full
register-window machinery this would need.

### Tests

14 unit/integration tests pin the canonical `ADD %g0, 42, %o0; ta 0`
byte sequence and edge cases (zero, 13-bit range boundaries, bool,
multi-var fallthrough, unsupported op, empty CIR, `ret_void`,
`Backend::run` panics, `Backend::compile` vs the free `compile`
function agree), plus one test that loads the compiled bytes into
`sparc-v8-simulator` and asserts `%o0 == 42` and `halted() == true`
after execution — not just a hand-asserted byte array.
