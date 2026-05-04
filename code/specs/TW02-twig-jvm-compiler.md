# TW02 — Twig → JVM (real `java` target)

## Overview

This is the **first non-Python target for Twig** to ship.  Twig
source compiles, via the in-house ``compiler-ir`` +
``ir-to-jvm-class-file`` chain, to a real JVM class file that
executes on the actual ``java`` runtime — not just a simulator.
The repo's JVM stack already produces real-runtime-conformant
class files (``test_oct_8bit_e2e.py`` invokes
``subprocess.run(["java", ...])`` and asserts on real JVM output),
which is why JVM lands ahead of CLR in the roadmap: CLR needs
``cli-assembly-writer`` conformance work first (see ``CLR01-
real-dotnet-conformance.md``).

## Roadmap reorder

| Spec  | Target  | Status                                              |
|-------|---------|-----------------------------------------------------|
| TW00  | vm-core | Merged                                              |
| TW02  | **JVM** | **This spec — runs on real ``java``**               |
| TW03  | CLR     | Blocked on CLR01 (cli-assembly-writer conformance)  |
| TW04  | BEAM    | After TW03; direct .beam bytes (no Erlang source)   |
| TW05  | WASM    | Updates to wasm-runtime + Twig WASM target          |

## What runs in TW02 v1

**Functions are in.**  A functional language without functions is
just a calculator — earlier scoping that excluded them was wrong.

### Surface
- Integer literals; booleans (``#t``/``#f``)
- Arithmetic: ``+``, ``-``, ``*``, ``/``
- Comparison: ``=``, ``<``, ``>``
- Control flow: ``if``, ``let``, ``begin``
- Top-level value defines with **literal RHS**:
  ``(define x 42)`` — accumulates into the synthesised initialiser
- **Top-level function defines:**
  ``(define (f x y) body)`` — emit one JVM static method per
  function; recursion works because methods reach each other via
  ``invokestatic``
- Function application of a top-level name:
  ``(f a b)`` — direct call, args pre-loaded into the
  shared-register convention the IR backend expects
- Builtin application: ``(+ a b)`` etc. — emit the corresponding
  ``IrOp``

### Output convention
The program's *final* top-level expression's value (or the value
of the last ``begin``) is written as a single byte to
``System.out`` via ``SYSCALL 1`` (the JVM backend's stdout-write
helper).  Tests assert on the captured stdout bytes — the same
convention ``test_oct_8bit_e2e.py`` already uses.

This is a v1 simplification — multi-byte / multi-line output and a
proper ``print`` builtin land in TW02.5.

### NOT in TW02 v1
- **`lambda`** — closures need synthetic JVM classes per lambda;
  TW02.5 once the function calling convention is settled.
- **`cons` / `car` / `cdr` / symbols / `nil`** — heap objects.
  TW03 territory once we know how the runtime classes look.
- **`print`** as a multi-arg / formatting builtin.
- **Tail-call optimisation.**  JVM has no proper TCO; the v1
  approach is plain recursion bounded by the JVM stack.

## Architecture

```
Twig source
   ↓  parse_twig + extract_program          (twig package)
typed AST
   ↓  twig_jvm_compiler.compile_to_ir       (this package)
IrProgram
   ↓  ir-optimizer (default passes)
IrProgram (optimised)
   ↓  lower_ir_to_jvm_class_file            (existing)
JVMClassArtifact
   ↓  write_class_file                      (existing)
.class file on disk
   ↓  java -cp <dir> <ClassName>            (real runtime)
program output
```

## Calling convention

The JVM backend emits one ``invokestatic`` per ``IrOp.CALL`` with
descriptor ``()I`` — **no arguments, returns int**.  Caller and
callee share a class-level register array (the runtime's
``_helper_reg_get`` / ``_helper_reg_set`` machinery), so parameter
passing happens by writing args into specific shared registers
before the call.

For Twig:

- Parameter ``i`` of every function lives at register ``2 + i``.
  (``0`` is scratch zero, ``1`` is the HALT-result convention.)
- Caller for ``(f a b)``:
  1. Evaluate ``a`` into a fresh holding register (``≥10``)
  2. Evaluate ``b`` into another fresh holding register
  3. ``ADD_IMM r2, holding_a, 0`` (move into param-0 slot)
  4. ``ADD_IMM r3, holding_b, 0`` (move into param-1 slot)
  5. ``CALL f``
  6. Result is in register 1; copy to a fresh register if the
     calling expression continues
- Callee for ``f``: parameter ``x`` resolves to register 2,
  ``y`` to register 3, etc.  Body computes its result; final
  ``RET`` (or the synthesised ``HALT`` for ``_start``) loads
  register 1 and returns.

The two-step "into holding register, then into param slot"
pattern is what makes nested calls work — without it,
``(f (g x) y)`` would have ``g``'s arg setup clobber ``f``'s
arg-0 slot.

## Tests

- **Compile-only:** golden IR shapes for each surface form;
  rejection paths (lambda, cons, …).
- **Real-JVM end-to-end** (gated behind ``_java_available()``,
  same pattern as ``test_oct_8bit_e2e.py``):
  - Pure arithmetic: ``(+ 1 2)`` writes byte 3.
  - Function call: ``(define (square x) (* x x)) (square 7)``
    writes byte 49.
  - Recursion: ``(define (sum n) (if (= n 0) 0 (+ n (sum (- n 1))))) (sum 10)``
    writes byte 55.
  - Conditional: ``(if (= 1 1) 100 200)`` writes byte 100.
  - ``let``: ``(let ((x 5)) (* x x))`` writes byte 25.
- Coverage target: ≥ 80%.

## Out of scope (TW02.5 / future)

- ``lambda`` and closures (synthetic JVM classes per lambda).
- ``cons`` / ``car`` / ``cdr`` / symbols / ``nil`` (heap objects
  via runtime support classes).
- ``print`` as a multi-arg / formatting builtin (writing strings,
  not just bytes).
- Tail-call optimisation.
- Negative byte ranges via two's-complement / explicit signed
  output handling.
