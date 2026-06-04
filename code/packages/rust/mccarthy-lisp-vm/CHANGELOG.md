# Changelog — mccarthy-lisp-vm

## v0.2.0 — 2026-06-04 — control flow for COND (L2b)

* Added the control-flow opcodes the `COND` lowering needs:
  * `jmp NAME` — unconditional branch (`srcs[0]` = label `Var`).
  * `jmp_if_false COND, NAME` — branch to `NAME` when `COND` is falsy
    (`#f` or `nil`, via `LispyValue::is_truthy`); otherwise fall through.
  * `mov dest, src` — copy a register (funnels each `COND` clause's value
    into one result register).
* `label NAME` instructions are pre-scanned into an O(1) name→index table
  at function entry; branch targets resolve through it. A branch to an
  undefined label is a clean `VmError::UnknownLabel` (never a panic).
* The interpreter stays a flat dispatch loop — jumps only move `pc`
  within one function, so there is still no native recursion over the
  program, and the instruction budget bounds any loop a future phase
  might introduce.
* 5 new unit tests (`mov`, unconditional `jmp`, `jmp_if_false` on
  truthy/falsy/nil, undefined-label error) — 19 unit + 1 doctest total.

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
