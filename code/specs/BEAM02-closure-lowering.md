# BEAM02 — Closure lowering for the BEAM backend

## Why this spec exists

This is the **BEAM-side companion** to
[JVM02 Phase 2](JVM02-phase2-multi-class-closure-lowering.md)
and [CLR02](CLR02-closure-lowering.md), implementing
[TW03 Phase 2](TW03-lisp-primitives-and-gc.md) (closures across
all backends).

BEAM's strategy is **completely different** from JVM/CLR
because BEAM has a first-class notion of "fun objects" baked
into the file format and instruction set:

- A new ``FunT`` chunk in the ``.beam`` file describes each
  lifted lambda — atom, arity, list of free variables, etc.
- A new ``make_fun2`` opcode at the closure-creation site
  packages captured values into a fun handle.
- A ``call_fun`` opcode at the apply site invokes it.

No new classes, no JAR — just one extra chunk plus two new
opcode types.  This is the **simplest** of the three backend
strategies.

## Acceptance criterion

```python
>>> from twig_beam_compiler import run_source
>>> run_source(
...   "(define (make-adder n) (lambda (x) (+ x n))) ((make-adder 7) 35)",
...   module_name="adder",
... ).stdout.strip()
b'42'
```

The Twig program uses a closure capturing the free variable
``n``.  Real ``erl`` runs it and prints ``42``.

## Design

### The FunT chunk

ECMA-style BEAM file format reference §FunT.  One row per lifted
lambda::

    {function_atom_idx, arity, code_label, index, num_free, old_uniq}

- ``function_atom_idx`` — atom-table index of the lifted
  lambda's name (e.g. ``_lambda_0`` interned as an atom).
- ``arity`` — the lambda's parameter count (does not include
  captured free variables).
- ``code_label`` — BEAM label number where the lambda body
  starts (the lifted region's call-target label).
- ``index`` — fun-table entry index, sequential from 0.
- ``num_free`` — count of captured free variables.
- ``old_uniq`` — a 32-bit hash of the function body for
  versioning.  We can compute it as a CRC32 of the body bytes
  or use a fixed value for v1 (Erlang accepts both).

The ``beam-bytecode-encoder`` package needs a new
``BEAMFun`` dataclass and an ``encode_funt(funs)`` helper.  The
``FunT`` chunk goes after ``ExpT`` in the chunk order.

### make_fun2 opcode

BEAM opcode 103 (per ``beam-opcode-metadata`` catalog).  Single
operand: an unsigned integer index into the FunT table.

```
make_fun2 fun_index
```

Pre-conditions: the captured values must be in the ``x0..xN-1``
registers in declaration order.  ``make_fun2`` reads them,
allocates a fun heap object, and stores the resulting fun
reference in ``x0``.

So the lowering of ``MAKE_CLOSURE`` becomes:

```
; First, move the captured values from their current
; (y-register) IR slots into BEAM x-registers x0..xN-1
move {y, capt0_reg}, {x, 0}
move {y, capt1_reg}, {x, 1}
...
make_fun2 fun_index
move {x, 0}, {y, dst_reg}    ; store fun ref into IR dst register
```

### call_fun opcode

BEAM opcode 75 (per the catalog: arity 1).  Single operand: the
arity of the call (number of args being passed, not counting
the closure value itself).

Pre-conditions:
- The closure (fun) reference must be in ``x{arity}``.
- The arguments must be in ``x0..x{arity-1}``.

So the lowering of ``APPLY_CLOSURE`` becomes:

```
move {y, arg0_reg}, {x, 0}
move {y, arg1_reg}, {x, 1}
...
move {y, closure_reg}, {x, arity}   ; closure goes in x{arity}
call_fun arity
move {x, 0}, {y, dst_reg}            ; result lands in x0
```

### Closure body emission

The lifted lambda body is emitted as a regular BEAM function
**but with extra implicit parameters for the captured
variables**.  Inside the body, captured variables are accessed
as ``x{arity..arity+num_free-1}`` (BEAM's calling convention
for fun-internal access — the runtime restores the captures
from the fun object into x-registers before invoking the body).

The Twig frontend already tracks free vars per lambda (via
``twig.free_vars``); the BEAM lowering reads that info and
generates the right ``allocate K, arity+num_free`` at function
entry plus the right ``move {x, captN}, {y, slot}`` shuffle.

### Operand encoding

The ``z`` extended-tag in BEAM compact-term encoding handles
some operand types we haven't touched yet.  For ``make_fun2``
the operand is just a ``u`` (small unsigned int) — no new
encoding needed.

For ``call_fun`` similarly: just a ``u`` for arity.

## Implementation phases

Same staging as JVM02 Phase 2 / CLR02:

- **BEAM02 Phase 2a** — IR ops (already shipped via the
  ``compiler-ir`` v0.4.0 release alongside the JVM02 spec).
- **BEAM02 Phase 2b** — ``FunT`` chunk encoding in
  ``beam-bytecode-encoder``: new ``BEAMFun`` dataclass,
  ``encode_funt`` helper, ``FunT`` added to the standard chunk
  order.
- **BEAM02 Phase 2c** — ``MAKE_CLOSURE`` / ``APPLY_CLOSURE``
  lowering in ``ir-to-beam``: new opcode constants for
  ``make_fun2``/``call_fun``, register-shuffle logic at both
  sites, FunT-row population.
- **BEAM02 Phase 2d** — ``twig-beam-compiler`` accepts
  ``Lambda`` + real-``erl`` closure test.

## Risk register

- **``old_uniq`` hash mismatch.**  BEAM's loader caches funs
  across module reloads keyed on ``old_uniq``.  A hand-rolled
  hash that collides with a real ``erlc`` output could cause
  surprising behavior in mixed-source environments (test
  harnesses that load both our modules and ``erlc``-produced
  ones).  Mitigation: use a deterministic CRC32 of the body
  bytes; document the choice.
- **``allocate`` framing across ``call_fun``.**  BEAM funs may
  themselves call other funs.  The y-register frame allocated
  by the caller must be re-established after the call returns.
  ``call_fun`` saves/restores the frame automatically (same
  semantics as ``call``), so no special handling needed —
  just keeping the same y-register convention works.
- **``num_free`` parameter ordering.**  The convention
  (captured free vars come AFTER explicit args in
  ``x0..x{arity+num_free-1}``) is non-obvious.  Mitigation:
  document inline; tests assert on exact register-shuffle
  bytecode.

## Cross-spec parity

| Aspect | JVM02 | CLR02 | BEAM02 |
|--------|-------|-------|--------|
| Closure container | per-lambda class | per-lambda TypeDef | FunT chunk row |
| Apply dispatch | ``invokeinterface Closure.apply`` | ``callvirt IClosure.Apply`` | ``call_fun`` opcode |
| Capture storage | object fields | object fields | fun heap object slots |
| Packaging | JAR (multi-class) | single ``.exe`` | single ``.beam`` |
| Difficulty | High (JAR + multi-class plumbing) | Medium (multi-TypeDef in one file) | Low (one new chunk + two opcodes) |

BEAM is the easiest of the three, JVM the hardest.  Suggested
implementation order: **BEAM Phase 2 first** (smallest scope
to validate the full flow) → CLR Phase 2 → JVM Phase 2.

## Out of scope

- **Tail calls into closures.**  BEAM has ``call_fun_last`` for
  tail calls; we'll wire that as an optimisation later.  v1
  uses plain ``call_fun``.
- **Closures captured in lists / cons cells.**  Heap primitives
  are TW03 Phase 3.
- **First-class fun introspection** (``is_function/1``,
  ``fun_info/2``) — out of scope.
