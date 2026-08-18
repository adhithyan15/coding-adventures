# Changelog — derive-vm

## v0.1.0 — 2026-08-18 — initial release (derive-iir-vm.md, Wave 5 item 2)

Derive's own dedicated VM interpreter for the v0 arithmetic/assignment
IIR subset — a direct structural port of `macsyma-vm`'s dispatch loop
(only `const`/`call_builtin`/`ret`, over the shared `dynval-runtime`
tagged-value model), per this rollout's explicit VM-sharing decision
(`macsyma-iir-vm.md` §6): every language gets its own dedicated VM crate.

* `run`/`run_with_budget` executing an `interpreter_ir::IIRModule`,
  returning a `dynval_runtime::LispyValue`.
* `resolve_builtin` maps `+`/`-`/`*`/`/`/`cons`/`car`/`cdr` to
  `dynval-runtime`'s existing builtins — no new runtime logic.
* 16 unit tests over hand-built IIR: every opcode, every error path
  (missing entry point, unknown function, fell off end, unsupported op,
  unknown builtin, undefined register, division by zero, out-of-range
  integer literal), and the instruction budget.
