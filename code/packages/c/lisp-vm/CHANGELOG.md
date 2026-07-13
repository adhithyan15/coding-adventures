# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `lisp-vm` crate: a bytecode virtual machine that
  executes the `LcCodeObject` produced by `lisp-compiler`.
- Stack machine with a value stack, a global variable table (`define`), indexed
  local slots, and a grow-only heap for cons cells, interned symbols, and
  closures.
- Closures capturing their defining environment; tail-call optimisation so
  tail-recursive programs run in constant C stack.
- `LcValue`-based memory model: values are cloned onto the stack / into
  variables / onto the heap and freed on consumption (`lc_value_clone` /
  `lc_value_free`). Verified leak-free under ASan + UBSan.
- Public API: `lv_new`/`lv_free`, `lv_execute`, stack/heap/output inspection,
  `lv_format_value`, and the top-level `lv_run` / `lv_run_with_output`.
- Native-recursion guard (`LV_MAX_CALL_DEPTH`): a deep *non-tail* closure call
  chain fails cleanly with a runtime error instead of overflowing the C stack.
  Tail calls loop and are unaffected.
- 86 checks across low-level opcode tests and end-to-end programs (arithmetic,
  `cond`, `define`, `lambda`, higher-order functions, `quote`, cons/car/cdr,
  symbols, `factorial`, `fib`, tail-recursive `countdown 10000`, and the
  non-tail recursion depth guard).

### Fixed

- Bound the embedded compile-error message in `lv_run` / `lv_run_with_output`'s
  `snprintf` with a `%.118s` precision so the total fits the 128-byte buffer;
  the unbounded `%s` tripped gcc's `-Werror=format-truncation` on ubuntu CI
  (both messages are `char[128]`).
