# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `lisp-vm` crate: a bytecode
  virtual machine (`ca::lisp_vm`) that executes the
  `ca::lisp_compiler::CodeObject` produced by `lisp-compiler`.
- Stack machine with a value stack, a global variable table (`define`), indexed
  local slots, and a grow-only heap modelled as
  `std::vector<std::variant<ConsCell, HeapSymbol, LispClosure>>`.
- Closures capturing their defining environment; tail-call optimisation so
  tail-recursive programs run in constant stack.
- Idiomatic C++ ownership via `std::vector` / `std::string` /
  `std::unordered_map` / `std::variant` / `std::shared_ptr<CodeObject>`; runtime
  and compile errors surfaced as `ca::lisp_vm::VmError` exceptions.
- Public API: `class LispVm` (with `execute`, `format_value`, and inspectable
  `stack`/`variables`/`heap`/`output`) and free functions `run` /
  `run_with_output`.
- Native-recursion guard (`kMaxCallDepth`): a deep *non-tail* closure call chain
  throws `VmError` instead of overflowing the C++ stack. Tail calls loop and are
  unaffected.
- 83 checks across low-level opcode tests and end-to-end programs (arithmetic,
  `cond`, `define`, `lambda`, higher-order functions, `quote`, cons/car/cdr,
  symbols, `factorial`, `fib`, tail-recursive `countdown 10000`, and the
  non-tail recursion depth guard).
