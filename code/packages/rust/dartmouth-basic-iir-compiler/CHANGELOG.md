# Changelog — `dartmouth-basic-iir-compiler`

## 0.1.0 — 2026-05-20 (PL05 initial release)

Initial release.  Compiles Dartmouth BASIC source to
`interpreter_ir::IIRModule`, unlocking the LANG VM AOT chain
(twig-aot / lang-aot → x86_64-backend / aarch64-backend → object →
system linker → native executable) for BASIC programs.

Distinct from the existing `dartmouth-basic-ir-compiler` crate, which
targets the GE-225 simulator's custom `compiler_ir::IrProgram` shape
and is not pluggable into the LANG VM chain.

### V1 coverage (integer programs)

| Statement | Status |
|-----------|--------|
| `LET A = expr` | ✓ |
| `PRINT expr`   | ✓ (numeric only — strings deferred to LANG77) |
| `INPUT X`      | ✓ |
| `IF cond THEN m` | ✓ |
| `GOTO m`       | ✓ |
| `FOR I = a TO b STEP s` / `NEXT I` | ✓ (positive STEP) |
| `END` / `STOP` | ✓ |
| `REM …`        | ✓ (no-op) |
| `GOSUB` / `RETURN` | **deferred** — V1 errors with `UnsupportedStatement` |
| `READ` / `DATA` / `RESTORE` | deferred — needs data pool |
| `DIM` / arrays | deferred — needs LANG76-based byte arrays |
| `DEF`          | deferred |

### Expression coverage

- Integer literals (floats truncate to i64; explicit float support
  deferred until backends grow SSE2).
- Variables (scalar `A..Z`, `A0..Z9` — array access `A(I)` deferred).
- Arithmetic: `+`, `-`, `*`, `/` with standard precedence.
- Unary minus.
- Exponentiation (`^`): deferred — needs a runtime helper.
- Built-in / user-defined functions (`SIN`, `FNA`, …): deferred.

### IIR shape

The whole program becomes a single function `main` returning `i64`.
Every BASIC line gets a label `line_<n>`; flow-control statements
jump between those labels.  FOR/NEXT loops use per-loop synthetic
labels `for_<id>_test` / `for_<id>_end`.

### Tests

11 unit tests cover each supported statement plus the deferred
`UnsupportedStatement` paths.  End-to-end smoke tests in
`lang-aot/tests/end_to_end_smoke.rs` compile BASIC programs all the
way to native executables on Windows + Linux and assert stdout:

- `10 PRINT 42 / 20 END` → stdout `"42\n"`.
- `10 FOR I = 1 TO 3 / 20 PRINT I / 30 NEXT I / 40 END` → stdout
  `"1\n2\n3\n"`.

Spec: `code/specs/PL05-dartmouth-basic-iir-compiler.md`.
