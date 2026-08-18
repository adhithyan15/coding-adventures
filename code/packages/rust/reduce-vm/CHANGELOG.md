# Changelog — reduce-vm

## v0.1.0 — 2026-08-18 — initial release (reduce-iir-vm.md, Wave 5 item 3)

Reduce's own dedicated VM interpreter for the v0 arithmetic/assignment
IIR subset — a direct structural port of `macsyma-vm`'s/`derive-vm`'s
dispatch loop, per this rollout's explicit VM-sharing decision
(`macsyma-iir-vm.md` §6): every language gets its own dedicated VM crate.

* `run`/`run_with_budget` executing an `interpreter_ir::IIRModule`,
  returning a `dynval_runtime::LispyValue`.
* `resolve_builtin` maps `+`/`-`/`*`/`/`/`cons`/`car`/`cdr` to
  `dynval-runtime`'s existing builtins — no new runtime logic.
* 15 unit tests over hand-built IIR covering every opcode, every error
  path, and the instruction budget.
