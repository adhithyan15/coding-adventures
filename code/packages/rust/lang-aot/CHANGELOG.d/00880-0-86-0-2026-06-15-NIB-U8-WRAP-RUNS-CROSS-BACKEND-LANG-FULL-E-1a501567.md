## 0.86.0 — 2026-06-15 — Nib u8 wrap runs cross-backend (LANG-FULL E2 / N6)

`tests/lang_matrix.rs` gains two new **Nib integer-wrap** programs, both run
across native / LLVM / WASM / JVM / CLR / VM / JIT:

**Wrap proof** (LANG-FULL N6 — u8 wrap semantics):
```nib
fn main() -> u8 { let x: u8 = 200 + 100; if x == 44 { return 1; } return 0; }
```
→ exit **1**. `200 + 100 = 300` wraps mod-256 to **44** in every backend;
comparing the in-register value (not just the exit-code low byte) proves the
wrap happened *before* the comparison.  Exercises the full E2 stack:
`nib-type-checker` 0.3.0 annotates the `add_expr` node as `u8` (bidirectional
context flows from the `let x: u8` declaration); `nib-iir-compiler` 0.14.0
emits `add` with `type_hint = "u8"`; each backend masks — vm-core and jit-core
by `result & 0xFF`, iir-to-wasm by `i32.and 255`, iir-to-jvm/cil by `and`,
iir-to-llvm by `and i64 ..., 255`, native-AOT by `and X0, X0, #0xFF` /
`and rax, 0xFF`.

**Magnitude regression guard** (confirms bidirectional typing):
```nib
fn main() -> u8 { return 6 * 7; }
```
→ exit **42**. Without `nib-type-checker` 0.3.0, `6` and `7` would infer as
`u4` (magnitude ≤ 15), the backend would mask `42 & 0xF = 10`, and the test
would fail.  Passing proves literals in a `u8` return context adopt `u8`, and
`6 * 7 = 42` is within the u8 range so no wrap occurs.

### Crates updated

- `nib-type-checker` 0.3.0 — bidirectional typing
- `nib-iir-compiler` 0.14.0 — narrow type_hints + unary `~` lowering
- `aot-core` 0.2.2 — `u4` added to CIR type pipeline

