# CLR02 — Closure lowering for the CLR backend

## Why this spec exists

This is the **CLR-side companion** to
[JVM02 Phase 2](JVM02-phase2-multi-class-closure-lowering.md).
Both implement [TW03 Phase 2](TW03-lisp-primitives-and-gc.md)
(closures across all backends) but the lowering strategies differ
significantly because PE/CLI's class model is more flexible than
the JVM's.

**Key advantage for CLR**: a single PE/CLI assembly natively
supports **multiple ``TypeDef`` rows** in one ``.exe``.  No
"JAR equivalent" needed — closure classes live alongside the
program's main type in the same file.  That makes CLR
implementation substantially simpler than JVM Phase 2.

## Acceptance criterion

```python
>>> from twig_clr_compiler import run_source
>>> run_source(
...   "(define (make-adder n) (lambda (x) (+ x n))) ((make-adder 7) 35)",
...   assembly_name="Adder",
... ).returncode
42
```

The Twig program uses a closure (``lambda``) capturing the free
variable ``n`` from ``make-adder``'s scope.  Real ``dotnet``
runs the produced ``.exe`` and exits with code 42.

## Design

### Closure as a CIL TypeDef

Each ``MAKE_CLOSURE fn_label`` site causes us to emit one
additional ``TypeDef`` row in the assembly's metadata:

```
TypeDef row for Closure_<fn_label>:
  Flags         = 0x00100001  (public + auto-layout)
  Name          = "Closure_<fn_label>"
  Namespace     = "" (or the assembly namespace)
  Extends       = TypeRef to System.Object
  FieldList     = first field row for this type's fields
  MethodList    = first method row for this type's methods
```

Fields: one ``int32`` instance field per captured variable
(``capt0``, ``capt1``, ...).

Methods:
- ``ctor(int, int, ...)`` — constructor takes one int32 per
  capture and stores them in the fields.  Standard CIL ctor
  prologue: ``ldarg.0; call System.Object::.ctor()`` followed
  by ``ldarg.0; ldarg.1; stfld capt0`` per capture.
- ``Apply(int[])`` — the lifted lambda body.  Reads captured
  values from instance fields (``ldarg.0; ldfld capt0``),
  reads arguments from the array (``ldarg.1; ldc.i4 i; ldelem.i4``),
  computes, returns ``int``.

### Why an Apply method on each TypeDef instead of an interface?

JVM02 uses a shared ``Closure`` interface to make
``invokeinterface`` monomorphic.  CLR has the same option
(emit an ``IClosure`` ``TypeDef`` with abstract Apply, then
each closure ``Implements`` it), but it's not strictly
necessary for v1 — we can dispatch via ``callvirt`` on a
known-at-call-site closure type.

For v2 we'd add an ``IClosure`` interface to support
"first-class closure values" stored in heterogeneous
collections.  v1 keeps it simple: each closure is a concrete
type, and ``apply_closure`` IR-op lowering tracks the closure's
type at compile time.

Wait — that's actually a problem.  ``APPLY_CLOSURE`` operates
on a closure value held in a register.  We don't always know
the closure's type at the apply site (e.g. when the closure
came from a function call).  So we DO need the interface.

**Revised decision**: emit ``IClosure`` interface as the first
extra TypeDef.  Every closure ``Implements`` it.  ``APPLY_CLOSURE``
lowers to ``callvirt IClosure::Apply(int32[]) int32``.

### MAKE_CLOSURE lowering

```
MAKE_CLOSURE dst, fn_label, num_captured, capt0, capt1, ...
```

Lowers to:

```
ldloc capt0                  ; push captured value 0
ldloc capt1                  ; push captured value 1
...                          ; (one ldloc per capture)
newobj instance void Closure_<fn_label>::.ctor(int32, int32, ...)
                              ; allocate + invoke ctor
stloc dst                    ; store the new reference
```

The IR register for ``dst`` needs to hold an object reference
rather than an int.  Same problem JVM02 has — solved the same
way: parallel ``object[]`` register pool alongside the existing
``int32[]``.

### APPLY_CLOSURE lowering

```
APPLY_CLOSURE dst, closure_reg, num_args, arg0, arg1, ...
```

Lowers to:

```
ldloc closure_reg            ; push the closure reference
ldc.i4 num_args              ; push args-array length
newarr int32                 ; allocate int[num_args]
dup                          ; (one dup per arg-store)
ldc.i4 0                     ; index 0
ldloc arg0                   ; value
stelem.i4                    ; arr[0] = arg0
... (repeat for each arg)
callvirt instance int32 IClosure::Apply(int32[])
stloc dst
```

### Cross-spec parity with JVM02

Both backends use the same shape:
- One root interface (``Closure`` on JVM, ``IClosure`` on CLR)
- One subclass per ``MAKE_CLOSURE`` site, with captured fields
  and an ``Apply`` method
- ``APPLY_CLOSURE`` lowers to a virtual call on the interface

The only structural difference: JVM packages everything in a
JAR; CLR packages everything in one ``.exe`` (no archive
container needed).

### The Twig frontend (TW02.5 — same for both backends)

``twig-clr-compiler`` extends to:
1. Discover anonymous lambdas.
2. Lift each to a synthetic top-level region (``_lambda_0``, ...).
3. Run free-variable analysis (re-use ``twig.free_vars``).
4. At the lambda site, emit ``MAKE_CLOSURE``.
5. At apply-of-local sites, emit ``APPLY_CLOSURE``.

## Implementation phases

Same staging as JVM02 Phase 2:

- **CLR02 Phase 2a** — IR ops.  **Shipped** via the
  ``compiler-ir`` v0.4.0 release.
- **CLR02 Phase 2b** — multi-TypeDef metadata in
  ``cli-assembly-writer``.  **Shipped.**  The writer now accepts
  an arbitrary ``extra_types`` list on ``CILProgramArtifact``;
  each entry produces one ``TypeDef`` row plus its ``Field`` /
  ``MethodDef`` / ``InterfaceImpl`` rows.  Method signatures
  support the ``HASTHIS`` flag (instance methods) and an
  abstract-method codepath (RVA=0).  Two real-``dotnet`` tests
  prove that an ``IClosure`` interface and a concrete closure-
  shape class with a field round-trip and load.
- **CLR02 Phase 2c** — ``MAKE_CLOSURE`` / ``APPLY_CLOSURE``
  lowering in ``ir-to-cil-bytecode``.  Pending.
- **CLR02 Phase 2d** — ``twig-clr-compiler`` accepts ``Lambda``
  + real-``dotnet`` factorial-closure test.  Pending.

## Risk register

- **CLR class-name uniqueness across runs.**  Closure class
  names are derived from IR labels (``_lambda_0``, ...).  Two
  separate Twig programs in the same process would conflict if
  the assembly were loaded into a shared AppDomain.  Acceptable
  for v1 (each Twig run produces its own assembly with a
  unique ``assembly_name``).
- **Recursive ``IClosure.Apply`` and stack overflow.**  CLR has
  no proper TCO either.  Document this; recommend top-level
  ``define`` for deep recursion.
- **``int32[]`` allocation per call.**  Same overhead as JVM02.
  Future optimisation: specialise ``Apply0()`` / ``Apply1(int)``
  / ``Apply2(int, int)`` etc. on ``IClosure`` and dispatch by
  arity.

## Out of scope

- **Closure values in heterogeneous collections.**  TW03
  Phase 3 (heap primitives) brings cons cells; only then will
  closures need to live in ``object[]``-shaped data.  v1 only
  flows them through registers.
- **Tail-call elimination.**  CLR ``.tail`` prefix could be
  emitted but most JITs ignore it.  Out of scope for v1.
- **Closure introspection** (``procedure?``, etc.) — TW03
  Phase 3 territory.
