# Changelog — mccarthy-lisp-vm

## v0.5.3 — 2026-07-11 — DVAL01-2: dyn_* builtin dispatch names

DVAL01-2: the VM's `call_builtin` dispatch on the tagged-value builtin names
moves `lispy_*` → `dyn_*` in lockstep with the IIR name rename. Pure rename.

## v0.5.2 — 2026-07-11 — DVAL01-1c: dependency renamed `lispy-runtime` → `dynval-runtime`

The shared value-model crate `lispy-runtime` is renamed to `dynval-runtime`
(spec DVAL01 §3.2). The `Cargo.toml` dependency and the `use dynval_runtime::…`
imports in `src/lib.rs` move to the new name. Pure rename — the VM still
executes each `IIRModule` against the same tagged-`i64` value model.

## v0.5.1 — 2026-06-04 — recursive closures (L2c-3c): no new opcode, docs + test

* **`LABEL` recursive closures need no VM change.**  A `LABEL` used as a
  value compiles to a closure `(*CLOSURE* label-fn . env)` whose body
  recurses through an ordinary static `call` to its own (compiler-assigned)
  name; the captured `env` is just the leading `apply` args (L2c-3b
  machinery).  So a recursive `LABEL` value runs, and a non-terminating one
  still terminates with `CallDepthExceeded` rather than a stack overflow.
* Docs: the module-level `apply` notes now state that capture (L2c-3b) and
  recursive closures (L2c-3c) require no further VM machinery.
* Test: added `recursive_closure_value_applied` — a hand-built recursive
  `last` (the shape a recursive `LABEL` lowers to) invoked through a closure
  value via `apply`, returning the right result.
* No source/behaviour change; `lispy-runtime` / `lang-runtime-core` remain
  untouched, so the per-PR Miri obligation still does not apply.

## v0.5.0 — 2026-06-04 — `apply` binds the captured environment (L2c-3b)

* **`apply` now binds the closure's captured environment.**  A closure
  value is `(*CLOSURE* fn-name . env)` where `env = (v1 … vk)` is the list
  of captured free-variable values (L2c-3b).  `apply` flattens `env` into
  the **leading** call arguments (matching the lifted function's parameter
  order `captured ∪ own`), then appends the supplied arguments, then runs
  the callee.  L2c-3a closures (empty `env`) are unchanged.
* New helper `flatten_env` walks the env list with the `lispy-runtime`
  `car`/`cdr` builtins.  It is bounded by `MAX_CALL_ARGS` (defensive: the
  compiler only ever builds finite acyclic envs, and McCarthy source has no
  mutation so cannot create cycles, but a hand-crafted module can't make
  `apply` allocate without limit) and rejects a non-proper-list `env` as
  `NotAClosure`.
* No new opcode and no `lispy-runtime` change; the per-PR Miri obligation
  still does not apply.
* 2 new unit tests (apply binds a captured env value to the leading
  parameter then the supplied arg → `(A . B)`; a closure with a malformed
  non-list env → clean `NotAClosure`, no loop/panic).  The Ω self-apply
  call-depth test remains the recursion DoS regression.

## v0.4.0 — 2026-06-04 — the `apply` opcode: dynamic dispatch on closures (L2c-3a)

* Added the **`apply CLOSURE, args…`** opcode.  Unlike `call` (whose
  `srcs[0]` is a static function *name*), `apply`'s `srcs[0]` is a register
  holding a **closure value** `(*CLOSURE* fn-name . env)`.  It destructures
  the closure, looks the function up by name, and runs it in a fresh frame
  — *dynamic* dispatch (the callee isn't known until run time).  This is
  what a call to a parameter or a returned lambda lowers to (L2c-3a).
* New helper `destructure_closure` walks the value with the `lispy-runtime`
  `car`/`cdr` builtins and validates the head against the reserved
  `CLOSURE_TAG` (`*CLOSURE*`).  The tag is un-lexable McCarthy source, so
  only the compiler can have produced a value that passes — a user program
  cannot forge a closure via `QUOTE`.
* New error `VmError::NotAClosure(String)` — applying anything that isn't a
  `(*CLOSURE* …)` pair (a symbol, an integer, nil, or a wrong-tag pair) is
  a clean error, never a panic.
* **Same DoS guards as `call`:** `apply` recurses through `run_function`
  bounded by `MAX_CALL_DEPTH` + the shared instruction budget and the
  `MAX_CALL_ARGS` cap, so a self-applying closure — the Ω combinator
  `((LAMBDA (X) (X X)) (LAMBDA (X) (X X)))` — terminates with
  `CallDepthExceeded` instead of overflowing the native stack.
* 6 new unit tests (apply runs a closure; apply runs a builtin in the
  callee; apply of a plain symbol / of a wrong-tag pair → `NotAClosure`;
  apply to an unknown function → `UnknownFunction`; self-applying closure →
  call-depth guard, the DoS regression for the new op).
* No `lispy-runtime` / `lang-runtime-core` source is modified, so the
  per-PR Miri obligation still does not apply.

## v0.3.1 — 2026-06-04 — recursion (L2c-2): no new opcode, docs + tests

* **`LABEL` recursion required no VM change.**  A named recursive function
  `(LABEL F (LAMBDA … (F …) …))` compiles to a function whose body simply
  `call`s itself by name, and the existing `call` opcode already resolves
  the callee from the module and runs it in a fresh frame.  Call nesting
  stays bounded by `MAX_CALL_DEPTH` + the shared instruction budget, so a
  non-terminating recursion still errors cleanly (`CallDepthExceeded`)
  rather than overflowing the native stack.
* Docs: the instruction-set table and the `call` notes now describe
  `LABEL` recursion explicitly (through L2c-2).
* Tests: added `terminating_recursion_computes_correctly` — a hand-built
  recursive `last` (the IIR shape a `LABEL` lowers to) that walks a
  cdr-spine to its final element and returns the right value — proving the
  `call` opcode genuinely supports recursion.  The existing
  `unbounded_recursion_hits_call_depth_guard` test is the DoS regression
  for the non-terminating case.
* No source/behaviour change; `lispy-runtime` / `lang-runtime-core` remain
  untouched, so the per-PR Miri obligation still does not apply.

## v0.3.0 — 2026-06-04 — user-function calls (L2c-1)

* Added the `call FN, args…` opcode: looks the callee up in the module
  by name (`srcs[0]` = `Var(name)`), evaluates the argument registers,
  runs the callee in a fresh frame with its parameters bound to the
  argument values, and stores the return value in `dest`.
* `run_function` now takes the module, the call arguments, and a call
  `depth`; the entry point runs at depth 0 with no arguments. Parameters
  are bound by name (the VM frame register named after each parameter).
  Arity must match exactly → `VmError::ArityMismatch`.
* **DoS guards for untrusted IIR:**
  * Call nesting is bounded by `MAX_CALL_DEPTH` (256) — a self-calling
    function trips `VmError::CallDepthExceeded` instead of overflowing
    the native stack. (The instruction budget is shared across the whole
    call tree, so it also bounds total work.)
  * A `call` carrying more than `MAX_CALL_ARGS` (4096) operands is
    rejected before the argument vector is allocated.
* 5 new unit tests (param passthrough, builtin-in-callee, arity mismatch,
  unknown callee, and a self-recursive function hitting the depth guard).

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
