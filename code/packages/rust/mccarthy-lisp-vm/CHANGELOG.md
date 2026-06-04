# Changelog — mccarthy-lisp-vm

## v0.1.0 — 2026-06-04 — initial release (L2a)

McCarthy 1960 Lisp's own VM — a small interpreter built directly on
`lispy-runtime`, deliberately independent of `twig-vm` (which is
Twig-specific). Both languages share only the `lispy-runtime` value
model.

* `run(&IIRModule) -> Result<LispyValue, VmError>` and
  `run_with_budget(&IIRModule, budget)` — execute the module's
  `entry_point` function and return the resulting `LispyValue`.
* Executes the L2a instruction set: `const` (int / nil-sentinel /
  interned-symbol / bool), `call_builtin` (dispatched to `lispy-runtime`
  builtins `cons`/`car`/`cdr`/`pair?`/`not`/`equal?`), `ret`, and `label`
  (a no-op marker until L2b adds jumps).
* Flat dispatch loop with a per-run instruction budget
  (`DEFAULT_INSTRUCTION_BUDGET = 10_000_000`) — no native recursion over
  the program, so untrusted IIR cannot overflow the stack; the budget
  backstops runaway loops once control flow lands in L2b.
* `VmError` covers every failure mode (no entry point, unknown
  function, unsupported opcode, unknown builtin, undefined register,
  builtin trap, budget exhaustion) — builtin traps never panic.
* 13 unit tests over hand-built IIR modules (literals, symbol interning,
  `cons`/`car`/`cdr`, `(not (pair? …))`, `equal?`, and every error path)
  plus a doctest.
