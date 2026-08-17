# Changelog — mips-r2000-backend

## v0.1.0 — 2026-08-17 — first lane of the 9-architecture expansion

Initial release.  Minimal viable `Backend` trait impl for MIPS R2000.
Covers `const_*` + `ret_*` — enough to compile the canonical Twig `42`
program to `[0x24, 0x02, 0x00, 0x2A, 0x03, 0xE0, 0x00, 0x08]`
(`ADDIU $v0, $zero, 42; JR $ra`), verified byte-for-byte and by actually
executing the emitted bytes in `mips-r2000-simulator`.

### Public API

- `pub struct MipsR2000Backend` with `impl Backend`:
  - `name() -> "mips-r2000"`
  - `compile(&[CIRInstr]) -> Option<Vec<u8>>` (big-endian flattened)
  - `compile_function(&FunctionContext, &[CIRInstr]) -> Option<Vec<u8>>`
  - `run(_, _)` panics with "mips-r2000 backend is emit-only; load bytes
    into mips-r2000-simulator to execute"
- `pub fn compile(ctx, cir) -> Result<Vec<u8>, BackendError>`
- `pub enum BackendError` — 4 diagnostic variants.

### Covered CIR ops

- `const_*` (16-bit signed immediate range) → `ADDIU $v0, $zero, imm`
- `ret_*`, `ret_void` → `JR $ra`
- Anything else → `None` (graceful AOT/JIT fallback)

### What's NOT in this PR

Full op coverage (add/sub/and/or/xor/cmp/branches/calls) is intentionally
out of scope for the minimal-viable lane.  Future increments can add
them, along with a real register allocator (the `TEMP_REGISTERS` pool
`mips-r2000-encoder` already declares).

### Tests

11 unit tests pin the canonical `ADDIU $v0, $zero, 42; JR $ra` byte
sequence and edge cases (zero, 16-bit range boundaries, immediate
overflow, bool, multi-var fallthrough, unsupported op, empty CIR,
`ret_void`), plus one test that loads the compiled bytes into
`mips-r2000-simulator` and asserts `$v0 == 42` after execution — not
just a hand-asserted byte array.
