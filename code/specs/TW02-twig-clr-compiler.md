# TW02 — Twig → CLR (CIL bytecode + PE assembly)

## Overview

This spec defines the first non-Python target for Twig: the .NET
Common Language Runtime.  Twig source compiles, via the in-house
``compiler-ir`` + ``ir-to-cil-bytecode`` + ``clr-pe-file`` chain, to
a real PE/CLI assembly that runs on ``clr-vm-simulator`` (and, in
principle, on ``mono`` / ``dotnet`` since the assembly format is
standard ECMA-335).

CLR is the *first* native target Twig gets, ahead of BEAM (TW03)
and JVM (TW04), because it's the lightest lift: the CLR has GC,
delegates (first-class functions), tail-call IL prefix, and runtime
generics — every facility a dynamically-typed Lisp wants, available
without inventing host-side machinery.

## Roadmap update

After this spec, the Twig roadmap is:

| Spec | Scope                                                   |
|------|---------------------------------------------------------|
| TW00 | Language base on ``vm-core`` (refcounted heap)          |
| TW01 | Mark-sweep GC + ``letrec`` (deferred)                   |
| TW02 | **This spec** — Twig → CIL.  V1 ships an arithmetic     |
|      | floor; closures and heap objects come in TW02.5/TW03.   |
| TW03 | Twig → BEAM bytecode (direct, no Erlang source)         |
| TW04 | Twig → JVM class files                                  |

## Why a tight v1 surface

A full Lisp compilation to CIL needs:

1. **Boxing** for dynamic values (everything fits in `object`).
2. **Closure objects** — synthetic CLR classes per ``lambda`` with
   captured fields and an ``Apply`` virtual method.
3. **Cons / Symbol / Nil** — runtime classes provided as CIL
   alongside the user code (a ``TwigRuntime`` class library).
4. **Print** — emit ``[mscorlib]System.Console::WriteLine``.
5. **Tail-call** — emit ``tail.`` IL prefix at recursive call sites.

That's a multi-week effort at the level of detail the existing
``brainfuck-clr-compiler`` operates at.  Trying to ship all of it in
one PR would mean either a giant un-reviewable diff or skipping the
hard parts.  **Instead, TW02 v1 ships a deliberately tiny floor**
that proves the wiring end-to-end:

### TW02 v1 surface

- **Integers** (boxed as CLR `int64`).
- **Booleans** (`bool`).
- **Top-level value defines**: ``(define x expr)`` — compile to
  static fields on the program class.
- **Top-level function defines**: ``(define (f x y) body)`` —
  compile to static methods.  Each method takes its parameters as
  ``int64`` (the only type in v1) and returns ``int64``.  No
  closures yet.
- **`if`** — emits ``brfalse`` / ``br`` over labeled blocks.
- **`let`** — emits assignments to local variables.
- **`begin`** — sequential evaluation, last expression's value is
  the result.
- **Arithmetic** (`+ - * / = < >`) — emits the CIL ops `add`, `sub`,
  `mul`, `div`, `ceq`, `clt`, `cgt`.  All operands are `int64`.
- **Top-level expression** at the end of a program becomes the
  return value of `Main`.

### TW02 v1 NON-surface

- **No closures**.  ``lambda`` is rejected at compile time with a
  clear error (``TwigCompileError: lambdas not yet supported by the
  CLR backend — see TW02.5``).
- **No cons cells / symbols / nil**.  ``cons``, ``car``, ``cdr``,
  ``null?``, ``pair?``, etc. are not registered as builtins on the
  CLR path.  Programs that use them are rejected.
- **No `print`**.  TW02 v1 returns values via ``Main``'s exit code
  rather than emitting ``Console.WriteLine``.  ``print`` lands in
  TW02.5 at the same time as cons cells (so we can render lists).
- **No tail-call optimisation**.  Recursion is bounded by the CLR's
  call stack.  Programs that need deep recursion would need TCO
  (TW03 territory).

## Architecture

```
Twig source
   │
   ▼  parse_twig + extract_program       (existing — TW00)
typed AST (twig.ast_nodes)
   │
   ▼  twig_clr_compiler.compile_to_ir    (NEW — this spec)
IrProgram (compiler-ir)
   │
   ▼  ir-optimizer (default passes)
IrProgram (optimised)
   │
   ▼  lower_ir_to_cil_bytecode           (existing)
CILProgramArtifact
   │
   ▼  write_cli_assembly                 (existing)
PE/CLI assembly bytes
   │
   ▼  clr_vm_simulator.run_clr_entry_point   (existing)
program return value
```

The package is **`twig-clr-compiler`**, mirroring
``brainfuck-clr-compiler``.  It exports:

```python
class TwigClrCompiler:
    def compile_source(self, source: str) -> PackageResult: ...
    def write_assembly_file(self, source: str, path: Path) -> Path: ...

def compile_source(source: str) -> PackageResult: ...
def run_source(source: str) -> ExecutionResult: ...
```

## The Twig → CompilerIR transform

The compiler walks the typed AST and emits ``IrInstruction``s using
the existing IR opcode set.  Mapping for the v1 surface:

| Twig form              | CompilerIR shape                            |
|------------------------|---------------------------------------------|
| `IntLit(n)`            | `LOAD_IMM dest, n`                          |
| `BoolLit(b)`           | `LOAD_IMM dest, 1` or `0`                   |
| `(+ a b)`              | `ADD dest, a_reg, b_reg`                    |
| `(- a b)`              | `SUB dest, a_reg, b_reg`                    |
| `(* a b)`              | `MUL dest, a_reg, b_reg`                    |
| `(/ a b)`              | `DIV dest, a_reg, b_reg`                    |
| `(= a b)`              | `CMP_EQ dest, a_reg, b_reg`                 |
| `(< a b)`              | `CMP_LT dest, a_reg, b_reg`                 |
| `(> a b)`              | `CMP_GT dest, a_reg, b_reg`                 |
| `(if c t e)`           | `BRANCH_Z c_reg, else_label` etc.           |
| `(let ((x e)) body)`   | bind `e_reg` to local-name `x`; emit body   |
| `(begin e1 …)`         | emit each e in order                        |
| `(define x e)`         | top-level binding (see below)               |
| `(define (f x) body)`  | one IrProgram function (see below)          |
| `(f a b)` (top-level)  | `CALL f_label, a_reg, b_reg → dest`         |
| top-level expr (last)  | `LOAD into HALT_RESULT`                     |

The IR uses only existing opcodes — no new IR shapes are needed
for v1, which is precisely why the CLR backend is the lightest lift.

## Top-level value defines

`(define x 42)` becomes a static field initialiser run before any
function body executes.  We accumulate value defines into a
synthetic `_init` IrProgram function that runs ahead of `main`.
References to `x` inside other functions emit a `LOAD_IMM` of the
*compile-time-known constant* if the RHS is a literal; otherwise
they emit a `LOAD_FROM_GLOBAL` (CIL `ldsfld`) — the lowering pass
handles this via the existing global-variable hooks in
`ir-to-cil-bytecode`.

For TW02 v1 we restrict value defines to **literal RHSs**
(`(define x 42)` is fine, `(define x (+ 1 2))` is not).  This
sidesteps the need to wire the CLR static-constructor mechanism
in the v1 PR.  The full case is straightforward but adds 100+
lines of bytecode emission.

## Tests

- Compile + run integer-arithmetic programs:
  - `(+ 1 2)` → 3
  - `(let ((x 5)) (* x x))` → 25
  - `(if (= 1 1) 100 200)` → 100
- Top-level functions:
  - `(define (square x) (* x x))` + `(square 7)` → 49
  - `(define (fact n) (if (= n 0) 1 (* n (fact (- n 1)))))` +
    `(fact 6)` → 720 *(if recursion through CIL `call` works)*
- Defensive: `lambda`, `cons`, `print` raise clear errors.
- Coverage ≥ 95%.

## Out of scope (TW02.5 / future)

- `lambda` + closure objects (synthetic CLR types per lambda).
- Cons / Symbol / Nil runtime library (`TwigRuntime.dll`).
- `print` via `Console.WriteLine`.
- Tail-call optimisation via the IL `tail.` prefix.
- `(define x (computation))` — non-literal value defines.
- Boxing / unboxing for mixed-type expressions.

These are real-language features and matter — but each is its own
chunk of bytecode emission work.  Shipping them in separate PRs
keeps the diffs reviewable and lets us course-correct on the CIL
layout choices before the surface area gets too wide.
