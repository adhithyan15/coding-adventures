# PR17: Prolog VM Compiler

## Summary

This batch adds the first compiler bridge from loaded Prolog artifacts into the
standardized Logic VM instruction stream.

The Prolog frontend stack can now produce executable `logic-instructions`
programs that run through `logic-vm`, instead of only running loaded clauses
directly through `logic-engine`.

## Goals

- keep dialect parsing and loading separate from VM lowering
- compile loaded Prolog sources and linked projects into `InstructionProgram`
- emit relation declarations for every relation referenced by clauses and
  queries
- preserve `dynamic/1` declarations as `DYNAMIC_REL` instructions
- compile ground head-only clauses as `FACT`
- compile rules and variable-bearing head-only clauses as `RULE`
- preserve initialization goals and source queries as labeled `QUERY`
  instructions
- expose helpers for loading compiled programs into `logic-vm`
- expose helpers for running source-level queries while hiding initialization
  query slots from callers

## Package

Add a new Python package:

```text
code/packages/python/prolog-vm-compiler
```

## Layer Position

```text
Prolog dialect parser
    ↓
prolog-loader
    ↓
prolog-vm-compiler
    ↓
logic-instructions
    ↓
logic-vm
    ↓
logic-engine
```

## Public API

The package exposes:

```python
CompiledPrologVMProgram
compile_loaded_prolog_source(...)
compile_loaded_prolog_project(...)
compile_swi_prolog_source(...)
compile_swi_prolog_project(...)
load_compiled_prolog_vm(...)
run_compiled_prolog_query(...)
run_compiled_prolog_queries(...)
```

`CompiledPrologVMProgram` keeps the VM instruction stream plus query metadata:

```python
@dataclass(frozen=True)
class CompiledPrologVMProgram:
    instructions: InstructionProgram
    initialization_query_count: int
    source_query_count: int
```

Initialization goals are emitted before source queries with labels like
`initialization:1`. Source queries are emitted afterwards with labels like
`query:1`.

## Clause Lowering

The compiler lowers clauses as follows:

- `parent(homer, bart).` becomes a `FACT`
- `ancestor(X, Y) :- parent(X, Y).` becomes a `RULE`
- `same(X, X).` becomes a `RULE` with a `true` body

The last case matters because the VM-level `FACT` instruction intentionally
requires ground facts. Prolog allows variable-bearing head-only clauses, so the
compiler preserves that semantics by compiling them as rules.

## Builtin Adaptation

By default, the compiler applies the shared `prolog-loader` builtin adapter to
clause bodies, initialization goals, and source queries before instruction
validation.

This allows supported Prolog builtins such as `not/1`, `once/1`, `call/1`, and
term-inspection predicates to execute through the VM.

Unsupported builtins remain ordinary relation calls. That keeps the compiler
lossless while leaving room for future builtin coverage batches.

## Non-goals

- low-level WAM-style opcode generation
- bytecode VM parity for dynamic declarations
- stateful execution of initialization directives across later source queries
- full ISO unknown-predicate error semantics
- broad arithmetic, CLP, or dialect-specific builtin lowering beyond the
  current shared adapter
