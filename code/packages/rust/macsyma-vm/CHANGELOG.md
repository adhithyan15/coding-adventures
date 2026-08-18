# Changelog — macsyma-vm

## v0.1.0 — 2026-08-14 — initial release (macsyma-iir-vm.md, v0)

Macsyma's own VM — a small interpreter built directly on `dynval-runtime`,
deliberately independent of `twig-vm`/`mccarthy-lisp-vm` (which are
Twig's/McCarthy Lisp's own). All three languages share only the
`dynval-runtime` value model.

* `run(&IIRModule) -> Result<LispyValue, VmError>` and
  `run_with_budget(&IIRModule, budget)` — execute the module's
  `entry_point` function and return the resulting `LispyValue`.
* Executes the v0 instruction set: `const` (int / nil-sentinel /
  interned-symbol), `call_builtin` (dispatched to `dynval-runtime`
  builtins `+`/`-`/`*`/`/`/`cons`/`car`/`cdr`), and `ret`. No
  branches/calls/closures — v0's accepted grammar needs none.
* `VmError` covers every failure mode (no entry point, unknown function,
  fell off end, unsupported opcode, malformed instruction, undefined
  register, unknown builtin, runtime trap, budget exhaustion, integer out
  of `dynval-runtime`'s tagged range) — builtin traps never panic.
* 15 unit tests over hand-built IIR modules (literals, symbol interning,
  concrete `+`/unary `-`, the `cons`-chain shape a symbolic `Apply` node
  uses, every error path) plus a doctest.
* Known, disclosed gap: `/` is `dynval-runtime`'s C-style truncating
  integer division, not Macsyma's exact rational. `macsyma-iir-compiler`
  never emits a `/` call unless it has verified the division is exact at
  compile time, so this is not reachable through the compiler — see
  `macsyma-iir-vm.md` §3 (division landmine) and §6 (Wave 2: bignum).
