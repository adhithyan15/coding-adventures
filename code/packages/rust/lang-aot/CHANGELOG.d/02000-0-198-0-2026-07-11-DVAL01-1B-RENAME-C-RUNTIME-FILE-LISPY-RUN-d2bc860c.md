## 0.198.0 - 2026-07-11 (DVAL01-1b: rename C runtime file lispy_runtime.c -> dynval_runtime.c)

DVAL01-1b: the shared C runtime file is renamed `lispy_runtime.c` -> `dynval_runtime.c` (and the golden test `lispy_runtime_golden.rs` -> `dynval_runtime_golden.rs`), continuing the de-lisp of the generic dynamic-value substrate (spec DVAL01). Pure file/path rename -- no symbol, ABI, or behaviour change; the link/build path strings that reference the runtime are updated to match. The `lispy-runtime` Rust crate rename follows in DVAL01-1c.

