# JVM01 — Per-method JVM locals for `ir-to-jvm-class-file`

> **Status (2026-04-28):** Landed via the *minimal-diff* path
> described in "Implementation as landed" below.  The bigger
> per-method-locals refactor in the original plan is no longer
> required — keeping the spec for context, with a note on what
> was actually shipped.

## Implementation as landed

Two cooperating changes made recursion correct on real `java`
without rewriting every callable's descriptor:

1. **Caller-saves at the JVM backend** —
   `ir-to-jvm-class-file/backend.py` now snapshots every register
   slot of the static `__ca_regs` array into JVM locals
   immediately before each `IrOp.CALL`, then restores them
   immediately after (skipping `r1`, the return-value slot).  This
   gives every CALL the *effect* of per-method locals without
   changing method descriptors or the helper-array contract.
   Each callable's `max_locals` is bumped to `reg_count`.
2. **Compiler-side param copy** — `twig-jvm-compiler/compiler.py`
   `_emit_function` now copies each parameter out of its arrival
   register (`r2`, `r3`, …) into a fresh body-local holding
   register at function entry.  Without this, the body's read of
   `n` happens against the same register the upcoming call's
   arg-marshalling overwrites — the caller-save then snapshots
   the *already-clobbered* value, defeating the fix.

Regression tests:
- `ir-to-jvm-class-file/tests/test_oct_8bit_e2e.py::test_call_preserves_caller_registers`
- `twig-jvm-compiler/tests/test_real_jvm.py::test_recursion_factorial`

The full per-method-locals refactor below is preserved as the
"clean" target if we ever want to retire the static `__ca_regs`
array entirely.

## Why this spec exists

`ir-to-jvm-class-file` produces real-`java`-compatible class files,
unlike the CLR equivalent (which has its own conformance gap, see
CLR01).  But it has a **calling-convention bug** that breaks
recursion: every "IR register" is stored in a class-level
``__ca_regs`` static int array shared across every method
invocation.

Concrete failure: `(define (fact n) (if (= n 0) 1 (* n (fact (- n
1))))) (fact 5)` should output byte 120, but outputs 0.  When
`fact(5)` calls `fact(4)`, `fact(5)`'s own parameter register r2
(holding 5) gets overwritten with 4 by the call setup.  After
`fact(4)` returns, the outer multiplication reads r2 as 4 and
short-circuits the result chain.

The CLR backend doesn't have this problem because each invokestatic
takes parameters via the JVM operand stack, so each method gets a
fresh local frame.  JVM01 brings the JVM backend up to the same
model.

## Sister track

This spec is the JVM-side companion to CLR01:

| Spec   | Backend | Gap                                                 |
|--------|---------|-----------------------------------------------------|
| CLR01  | CLR     | `cli-assembly-writer` lacks ECMA-335 essentials     |
| JVM01  | JVM     | `ir-to-jvm-class-file` uses class-level static reg  |
|        |         | array → recursion is broken                         |

Both track to the same outcome — real-runtime correctness for
languages compiled through the in-house IR pipeline.

## Detailed plan

### What changes

The JVM backend currently emits each callable method with descriptor
``()I`` (no args, returns int) and uses two helper static methods —
`__ca_regGet(int) -> int` and `__ca_regSet(int, int) -> void` —
that read and write a class-level static int array.  Caller and
callee communicate by writing to specific shared register indices
before/after the call.

**The fix:**

1. Each callable method's descriptor becomes ``(I*N)I`` where N
   is the program-wide maximum register count.  N is already
   computed as `reg_count` at backend-entry time.
2. Method body: every `_emit_reg_get(idx)` call becomes an `iload
   idx` (raw JVM instruction).  Every `_emit_reg_set(idx, value)`
   pattern becomes "value-on-stack + `istore idx`".
3. CALL emission: caller pushes `iload 0`, `iload 1`, …, `iload
   N-1` onto the JVM operand stack, invokestatic with the new
   descriptor, then `istore 1` to land the return value into local
   1 (preserving the existing "register 1 == HALT result"
   convention).
4. Main wrapper: pushes `N` zeros (`iconst_0` × N) before
   invokestatic on `_start`.
5. The static `__ca_regs` field and the `__ca_regGet` / `__ca_regSet`
   helper methods become unused.  We can either remove them
   (cleaner) or leave them in place as dead code (smaller diff).
   Remove.

### Why this works

JVM method frames are per-invocation: each invokestatic creates a
fresh local-variable array on the JVM call stack.  So when
`fact(5)` calls `fact(4)`, `fact(4)`'s locals are independent of
`fact(5)`'s.  After `fact(4)` returns, `fact(5)`'s locals (including
its r2 = 5) are intact.

Parameter passing becomes "values on the JVM operand stack at the
invokestatic site" — exactly what the JVM was designed for.

### Sites that change in `backend.py`

| Site                              | Today                       | After |
|-----------------------------------|-----------------------------|-------|
| Callable method descriptor        | `_DESC_NOARGS_INT` (`()I`)  | `(I*N)I` |
| `_emit_reg_get(builder, idx)`     | invokestatic helper         | `iload idx` |
| `_emit_reg_set(builder, idx, v)`  | invokestatic helper         | `bipush v; istore idx` |
| Compute-then-store patterns       | push idx, value, invokestatic helper | `value-on-stack; istore idx` |
| `IrOp.CALL` emission              | invokestatic ()I; helper_set 1 | iload 0..N-1; invokestatic (I*N)I; istore 1 |
| `_build_main_method`              | invokestatic _start ()I; pop; return | iconst_0 × N; invokestatic _start (I*N)I; pop; return |
| `_helper_reg_get` / `_helper_reg_set` | full method bodies      | remove |
| `__ca_regs` field                 | int[] static                | remove |
| `_build_class_initializer`        | allocates `__ca_regs`       | drop the alloc; keep memory init |

The compute-then-store patterns are the bulk of the work.  There
are 44 occurrences in `backend.py`.  Each follows a pattern roughly
like:

```python
self._emit_push_int(builder, dst.index)         # push dst index
self._emit_reg_get(builder, lhs.index)          # push lhs value
self._emit_reg_get(builder, rhs.index)          # push rhs value
builder.emit_opcode(_OP_IADD)                   # compute on stack
builder.emit_u2_instruction(                    # invoke helper_set
    _OP_INVOKESTATIC,
    self._method_ref(self._helper_reg_set, _DESC_INT_INT_TO_VOID),
)
```

Translates to:

```python
self._emit_iload(builder, lhs.index)            # iload lhs
self._emit_iload(builder, rhs.index)            # iload rhs
builder.emit_opcode(_OP_IADD)                   # compute on stack
self._emit_istore(builder, dst.index)           # istore dst
```

A helper `_emit_compute_into(builder, dst, op_emitter)` could
collapse the pattern, but isn't required.

### Max-locals / max-stack

After this change, every callable method has `max_locals = N` (the
program-wide register count, also the parameter count).
`max_stack` is dictated by the deepest expression — the existing
computation should keep working since the JVM-stack effects of
`iload`/`istore` match the helper-method effects (push int / pop
int).

The class file's CodeAttribute for each method needs the descriptor
update reflected in its `local_variable_table` if one is emitted
(today probably not — verify when implementing).

### Test plan

- Existing `test_oct_8bit_e2e.py` tests must keep passing — they
  cover non-recursive arithmetic + I/O programs, and the new model
  doesn't change semantics for those.
- New test in `ir-to-jvm-class-file/tests/`:
  - `test_recursion.py::test_simple_recursion` — a hand-built IR
    program that recursively halves an input and verifies the
    output matches.
- New test in `twig-jvm-compiler/tests/test_real_jvm.py`:
  - Re-add `test_recursion_factorial_small` — `(fact 5) == 120`.
  - Add `test_mutual_recursion_even_odd` — `(even? 4) == 1`.

When JVM01 lands, those tests pass without any change to
`twig-jvm-compiler` (it produces the same IR; only the backend's
emission of that IR changes).

## Out of scope for JVM01

- **Closures.**  TW02.5 will add lambda support to twig — that's
  separate language-frontend work.  JVM01 only fixes the calling
  convention so closures CAN work later.
- **Tail-call optimisation.**  JVM has no proper TCO.  Programs
  that need deep recursion will still blow the JVM stack.  TCO via
  trampolining or `recur`-style explicit-loop is a TW02.5 / TW03
  language-level decision, not a JVM backend issue.
- **Generics, exceptions, virtual dispatch.**  Not needed for the
  current language frontends.

## Risk register

- **Method-body verification.**  Real `java` runs class-file
  verification at load time — bytecode that worked under the
  static-helper model may need different `max_stack` /
  `max_locals` claims after the rewrite.  Mitigation: run all
  existing JVM tests after each chunk; verifier errors will be
  specific and pointable.
- **Unintentional behavioural change.**  Switching to per-method
  locals means cross-method state via the static array is gone.
  If any test or user-program relies on writing to a register in
  one method and reading it in another (via direct `_helper_reg_get`
  call, not through CALL), that breaks.  Mitigation: grep the
  repo for direct calls to `__ca_regGet` / `__ca_regSet` from
  outside the backend.
- **Class file size.**  Heavier method descriptors mean slightly
  larger constant-pool entries.  Negligible — N is small (currently
  bounded by the IR's `_max_register_index() + 1`).
