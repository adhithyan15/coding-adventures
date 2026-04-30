# JVM02 Phase 2 — multi-class closure lowering

## Why this spec exists

[JVM02](JVM02-jar-as-distribution-unit.md) Phase 1 shipped the
JAR writer (``jvm-jar-writer``).  Phase 2 is the implementation
that **uses** the JAR writer: extending ``ir-to-jvm-class-file``
to emit multiple classes from one IR program — specifically, one
class per closure plus the main program class.

This is the JVM-side implementation of [TW03](TW03-lisp-primitives-and-gc.md)
Phase 2 (closures across all backends).  Mirrors the user vision
(quote): "if a JAR is a better abstraction instead of class, we
should also reorient it towards that."

This spec depends on the new IR ops ``MAKE_CLOSURE`` and
``APPLY_CLOSURE`` (added to ``compiler-ir`` alongside this spec).
Those ops are the cross-backend interface; lowering strategies
differ per backend (see TW03 spec for the full table).

## Acceptance criterion

```python
>>> from twig_jvm_compiler import run_source
>>> run_source(
...   "(define (make-adder n) (lambda (x) (+ x n))) ((make-adder 7) 35)",
...   class_name="Adder",  # becomes the JAR's Main-Class
... ).stdout.strip()
b'*'   # (= 42 = 0x2a)
```

The Twig program uses a closure (``lambda``) that captures the
free variable ``n`` from the enclosing ``make-adder`` scope.
Real ``java`` runs the produced JAR and the answer comes out.

## Design

### The Closure base interface

Every closure produced by Twig implements a shared interface
emitted at the root of the JAR:

```java
package coding_adventures.twig.runtime;

public interface Closure {
    int apply(int[] args);
}
```

Why ``int[] args`` and not varargs of ``int``?  Because the
Twig calling convention (today) only handles integer registers,
and ``int[] args`` lets us call any closure of any arity through
a single JVM ``invokeinterface`` site without per-arity
dispatch.  Adds one allocation per call but keeps the invoke
site monomorphic, which matters for HotSpot inlining.

### Per-lambda subclass

Each ``MAKE_CLOSURE fn_label`` site causes us to emit a class:

```java
public class Closure_<fn_label> implements Closure {
    private final int capt0;
    private final int capt1;
    // ... one final field per captured variable

    public Closure_<fn_label>(int capt0, int capt1, /* ... */) {
        this.capt0 = capt0;
        this.capt1 = capt1;
        // ...
    }

    @Override
    public int apply(int[] args) {
        // The lifted lambda body — uses ``capt*`` for captured
        // free variables and ``args[i]`` for the lambda's own
        // params.
        return /* lowered IR body */;
    }
}
```

The class name uses a sanitised version of the IR ``fn_label``
(replacing ``-`` with ``_``, etc.).  All closure classes go
under the package ``coding_adventures.twig.<assembly>.``  to keep
the JAR's class layout predictable.

### MAKE_CLOSURE lowering

```
MAKE_CLOSURE dst, fn_label, num_captured, capt0, capt1, ...
```

Lowers to:

```
new <ClosureClass>           ; allocate the closure object
dup                          ; duplicate the reference for the constructor
ldloc capt0                  ; push captured value 0
ldloc capt1                  ; push captured value 1
...                          ; (one ldloc per capture)
invokespecial Closure_<fn_label>.<init>(I, I, ...)V
                              ; constructor takes one int per capture
astore dst                   ; store the new reference into dst
```

There's a wrinkle: today the JVM backend treats all IR
registers as ``int``.  Closure values are object references,
not ints.  We need to widen the in-class register pool to hold
both ints and references.

**Strategy**: introduce a parallel ``Object[]`` register file
alongside the existing ``int[]``.  Each IR register has both
slots.  Most operations use the int slot; ``MAKE_CLOSURE``
writes the object slot; ``APPLY_CLOSURE`` reads the object slot.
``ADD_IMM`` (used as MOV) copies both slots.

Why parallel arrays instead of an `Object[]` everywhere?
Boxing every int would explode allocation pressure and tank
arithmetic-heavy programs.  Parallel arrays let us pay the
boxing cost only at closure boundaries.

### APPLY_CLOSURE lowering

```
APPLY_CLOSURE dst, closure_reg, num_args, arg0, arg1, ...
```

Lowers to:

```
aload closure_reg            ; push the closure object reference
ldc num_args                 ; push the args-array length
newarray int                 ; allocate int[num_args]
dup                          ; (one dup per arg-store, see below)
ldc 0                        ; index 0
iload arg0                   ; value
iastore                      ; arr[0] = arg0
... (repeat for each arg)
invokeinterface Closure.apply([I)I
istore dst                   ; the int return goes into dst
```

The ``int[]`` allocation per call is the cost we pay for the
monomorphic invoke site.  Future optimisation: specialise apply
to common arities (apply0, apply1, apply2) on the Closure
interface and dispatch on `args.length`.

### The Twig frontend (TW02.5 work)

``twig-jvm-compiler`` extends to:

1. Discover anonymous lambdas in the AST.
2. Lift each to a synthetic top-level region (``_lambda_0``,
   ``_lambda_1``, ...).
3. Run free-variable analysis (already done in
   [twig/free_vars.py](../packages/python/twig/src/twig/free_vars.py)
   for the vm-core path) to find captured names.
4. At the lambda-creation site, emit ``MAKE_CLOSURE``.
5. At apply sites where the function position is a local (not a
   top-level name), emit ``APPLY_CLOSURE``.

The existing ``_compile_apply`` already dispatches on whether
the function is a top-level name; the new branch handles
"closure-valued local variable" by emitting ``APPLY_CLOSURE``.

## Implementation phases

This is a substantial change; suggested PR breakdown:

1. **Phase 2a** (this spec) — add IR ops + scaffolding.
   - Add ``MAKE_CLOSURE`` / ``APPLY_CLOSURE`` to compiler-ir
     ✓ shipped alongside this spec.
   - Backend stubs that raise ``NotImplementedError`` with a
     clear message pointing at the relevant spec phase.
   - This unblocks parallel work on JVM, CLR, BEAM.

2. **Phase 2b** — the Closure base interface + multi-class
   plumbing in ``ir-to-jvm-class-file``.  **Shipped.**
   - ``JVMMultiClassArtifact`` dataclass.
   - ``build_closure_interface_artifact()`` returns the
     ``coding_adventures.twig.runtime.Closure`` interface
     ``.class`` bytes (ACC_PUBLIC | ACC_INTERFACE | ACC_ABSTRACT,
     one abstract method ``int apply(int[])``).
   - ``lower_ir_to_jvm_classes(program, config, *,
     include_closure_interface=False)`` — multi-class API.
   - Real-``java`` JAR conformance test packs main + interface
     and proves the JVM loads both without VerifyError.

3. **Phase 2c** — ``MAKE_CLOSURE`` / ``APPLY_CLOSURE`` lowering
   in ``ir-to-jvm-class-file``.  **Shipped (structural).**
   - Per-lambda ``Closure_<name>.class`` artifacts via the new
     ``build_closure_subclass_artifact()`` — fields per capture,
     ``.ctor`` chains into ``Object::.ctor()`` and stores
     captures, ``apply([I)I`` is a placeholder ``iconst_0;
     ireturn``.
   - ``MAKE_CLOSURE`` lowers to ``new Closure_<fn>; dup; iload
     caps; invokespecial ctor`` then **pops the reference** —
     int[] register convention can't hold object refs.
   - ``APPLY_CLOSURE`` lowers to ``aconst_null; build int[] args;
     invokeinterface Closure.apply([I)I; pop`` — the closure
     ref isn't retrievable yet, so the placeholder null would
     NPE at runtime.
   - The full ``((make-adder 7) 35) → 42`` test ships as
     ``xfail(strict=True)`` so the structural shape stays right
     and it'll auto-flip to passing the moment the typed pool
     lands.

4. **Phase 2c.5** — Object register pool + lifted-lambda
   forwarder.  **Shipped.**  The headline ``((make-adder 7) 35)
   → 42`` test now runs end-to-end on real ``java -jar``
   (was previously ``xfail``).  Implementation choices:
   - Parallel ``Object[] __ca_objregs`` static field on the
     main class, initialized in ``<clinit>`` only when at
     least one closure is declared.
   - MAKE_CLOSURE stores the new ref into
     ``__ca_objregs[dst]`` via ``getstatic + aastore``.
   - APPLY_CLOSURE reads via ``getstatic + aaload + checkcast
     Closure``, then builds the int[] args, invokeinterface,
     and stores the int return into ``__ca_regs[dst]``.
   - **Lifted lambdas live on the main class as PUBLIC static
     methods** with widened arity (``num_free + explicit_arity``).
     The closure subclass's ``apply`` body forwards to them
     via ``invokestatic`` — pushes captures from ``this.captI``
     fields, pushes explicit args from the ``int[]`` parameter,
     ireturn the int.  This avoids the alternative of giving
     the subclass cross-class access to ``__ca_regs``.
   - The lifted lambda's prologue copies its JVM args into
     ``__ca_regs[REG_PARAM_BASE+i]`` so the existing IR-body
     emitter runs unchanged.
   Limitation: caller-saves still only cover ``__ca_regs``
   (not ``__ca_objregs``); fine for straight-line closure-flow
   code.  ADD_IMM-0 obj-mov + multi-closure-in-flight scenarios
   land in a follow-up if twig-jvm-compiler exercises them.

5. **Phase 2d** — ``twig-jvm-compiler`` accepts ``Lambda``.
   Pending — now **unblocked**.
   - Lambda lifting.
   - Free-var analysis (re-use ``twig.free_vars``).
   - JAR packaging via ``jvm-jar-writer``.
   - Real-``java`` test: ``((make-adder 7) 35) → 42``.

## Risk register

- **Object register pool overhead.**  Doubling the per-method
  local-variable-table size hurts class-file footprint slightly.
  Mitigation: only allocate the parallel ``Object[]`` slot if
  the function actually uses ``MAKE_CLOSURE`` or
  ``APPLY_CLOSURE``.  ``int``-only methods stay unchanged.
- **JVM verifier strictness on ``invokespecial`` constructor
  arity.**  Each closure class's constructor signature depends
  on capture count; getting the descriptor wrong produces
  ``VerifyError`` at load time.  Mitigation: unit tests assert
  on the exact descriptor strings emitted.
- **Cross-PR coordination.**  Phases 2b/2c/2d are sequenced;
  each blocks the next.  Mitigation: open them as stacked
  branches with explicit dependency notes.
- **Sister-spec drift.**  CLR Phase 2 and BEAM Phase 2 will
  use different lowering strategies (CLR does multi-class
  natively, BEAM uses ``FunT`` + ``make_fun2``).  Each gets its
  own implementation spec; this spec is **JVM-only**.

## Out of scope

- **Recursive closures inside their own lambda body.**  Today's
  Twig has no ``letrec``; recursion only works through
  top-level ``define``.  Closures that recursively reference
  themselves (e.g. ``(define f (lambda (n) (if ... (f ...))))``)
  need ``letrec`` or Y-combinator-style indirection, both of
  which are out of scope for this phase.
- **Tail-call elimination across ``apply``.**  JVM has no proper
  TCO.  Closures that recurse via ``apply`` will blow the JVM
  stack on deep recursion.  Acceptable for v1.
- **Closure introspection** (``procedure?``, etc.) — TW03
  Phase 3 territory.
