# Changelog — maple-vm

## v0.1.0 — 2026-08-18 — initial release (maple-iir-vm.md, Wave 5 item 4)

Maple's own dedicated VM interpreter for the v0 arithmetic/assignment
IIR subset — a direct structural port of the sibling VMs in this
rollout, per the explicit VM-sharing decision (`macsyma-iir-vm.md` §6).

* `run`/`run_with_budget` executing an `interpreter_ir::IIRModule`,
  returning a `dynval_runtime::LispyValue`.
* `resolve_builtin` maps `+`/`-`/`*`/`/`/`cons`/`car`/`cdr` to
  `dynval-runtime`'s existing builtins.
* 15 unit tests over hand-built IIR.
