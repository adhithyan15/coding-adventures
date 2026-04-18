# LP07 — Standardized Logic Instructions

## Overview

The current Python logic stack has two strong layers already:

- `logic-core` gives us terms, substitutions, and unification
- `logic-engine` gives us relations, clauses, programs, and solving

That is enough to write relational programs directly in Python.

There is still an important missing layer if we want to move toward a VM-backed
execution model:

- there is no standardized instruction stream for logic programs
- there is no package that treats facts, rules, and queries as executable data
- there is no shared contract that both the current direct solver and a future
  logic VM can target

This milestone introduces that missing layer.

## Design Goal

Add a package that standardizes logic programs as an ordered list of
instructions.

The first version should not pretend to be low-level bytecode yet. Instead, it
should be a clean instruction format that is:

- explicit
- serializable-in-spirit
- easy to validate
- easy to lower into the current `logic-engine`
- easy to compile into a future VM backend

## Why Instructions Before Bytecode

Jumping directly to a WAM-like bytecode would be educationally premature.

We first want a stable contract for:

- relation declarations
- fact emission
- rule emission
- query emission

That gives us a durable "logic assembly" layer. A future VM can then lower that
instruction stream into lower-level opcodes such as:

- `CALL`
- `TRY_ME_ELSE`
- `PROCEED`
- `UNIFY_VAR`
- `UNIFY_CONST`
- `FAIL`
- `BACKTRACK`

In other words:

```text
Python relational API
        ↓
Standardized logic instructions   ← this milestone
        ↓
Current direct lowering to logic-engine
        ↓
Future logic VM / bytecode backend
```

## Package

Add a new Python package:

```text
code/packages/python/logic-instructions
```

This package is responsible for:

- defining the instruction data model
- validating instruction streams
- lowering those instruction streams into `logic-engine` programs and queries
- running queries through the current engine backend

## Layer Position

```text
SYM00 Symbol Core
    ↓
LP00 Logic Core
    ↓
LP01 Logic Engine
    ↓
LP02 Disequality Constraints
    ↓
LP03 / LP04 / LP05 / LP06  Relational helper layers
    ↓
LP07 Logic Instructions   ← this milestone
    - standardized executable logic program format
    - current engine lowering
    - future VM target
```

## Scope

The first version should standardize *program-level* instructions, not
execution-level bytecode.

Required instruction kinds:

- `DEF_REL`
- `FACT`
- `RULE`
- `QUERY`

The first version should reuse `logic-engine` terms and goal expressions as the
operands stored inside those instructions. That keeps the package practical
immediately while still giving us a durable instruction stream.

## API

The package should export:

### Instruction data types

```python
InstructionOpcode
RelationDefInstruction
FactInstruction
RuleInstruction
QueryInstruction
InstructionProgram
AssembledInstructionProgram
```

### Helper constructors

```python
defrel(name: str | Symbol | Relation, arity: int | None = None) -> RelationDefInstruction
fact(head: RelationCall) -> FactInstruction
rule(head: RelationCall, body: GoalExpr) -> RuleInstruction
query(goal: GoalExpr, outputs: tuple[object, ...] | None = None, label: str | None = None) -> QueryInstruction
instruction_program(*instructions: LogicInstruction) -> InstructionProgram
```

### Validation and lowering

```python
validate(program: InstructionProgram) -> None
assemble(program: InstructionProgram) -> AssembledInstructionProgram
run_query(program: InstructionProgram, query_index: int = 0, limit: int | None = None) -> list[object]
run_all_queries(program: InstructionProgram, limit: int | None = None) -> list[list[object]]
```

## Instruction Semantics

### `DEF_REL`

Declares that a relation exists and fixes its arity.

Examples:

```python
defrel("parent", 2)
defrel(relation("ancestor", 2))
```

This declaration should be required before later `FACT`, `RULE`, or `QUERY`
 instructions refer to that relation.

### `FACT`

Emits one fact into the assembled program.

Example:

```python
fact(parent("homer", "bart"))
```

The first version should reject facts that contain logic variables in their
head, because facts are meant to represent ground truths in the standardized
instruction stream.

### `RULE`

Emits one rule into the assembled program.

Example:

```python
rule(
    ancestor(X, Y),
    conj(parent(X, Z), ancestor(Z, Y)),
)
```

The body may use any supported `logic-engine` goal expression, including:

- relation calls
- `eq(...)`
- `neq(...)`
- `conj(...)`
- `disj(...)`
- `succeed()`
- `fail()`
- `fresh(...)`
- `defer(...)`

### `QUERY`

Stores one runnable query in the instruction stream.

Example:

```python
query(ancestor("homer", Who), outputs=(Who,))
```

Queries are not part of the assembled clause database. They are execution
requests attached to the instruction program.

The first version should support:

- explicit `outputs`
- optional automatic output inference when `outputs` is omitted
- optional labels for query selection in tools later

## Validation Rules

The package should validate:

- all referenced relations were declared by `DEF_REL`
- duplicate relation declarations are rejected
- `FACT` heads are ground
- `FACT` and `RULE` heads are relation calls
- relation arity matches the declared arity
- query indices are in range when running

The validator should recursively inspect rule and query goals so undeclared
relations are caught even when nested inside conjunctions or disjunctions.

## Lowering

`assemble(...)` should lower the instruction stream into:

- one immutable `logic-engine` `Program`
- zero or more stored query plans
- a relation registry keyed by `(symbol, arity)`

This lowering should be deterministic and preserve source order.

## Running

`run_query(...)` should execute one stored query through the current
`logic-engine`.

If the query has:

- zero outputs, return success tuples such as `[()]`
- one output, return a flat list of reified terms
- multiple outputs, return tuples matching `logic-engine.solve_all(...)`

`run_all_queries(...)` should run every stored query in order.

## Usage Example

```python
from logic_engine import atom, conj, relation, var
from logic_instructions import defrel, fact, instruction_program, query, rule, run_query

parent = relation("parent", 2)
ancestor = relation("ancestor", 2)

X = var("X")
Y = var("Y")
Z = var("Z")
Who = var("Who")

program_ops = instruction_program(
    defrel(parent),
    defrel(ancestor),
    fact(parent("homer", "bart")),
    fact(parent("homer", "lisa")),
    rule(ancestor(X, Y), parent(X, Y)),
    rule(ancestor(X, Y), conj(parent(X, Z), ancestor(Z, Y))),
    query(ancestor("homer", Who), outputs=(Who,)),
)

assert run_query(program_ops) == [atom("bart"), atom("lisa")]
```

## Why This Matters

This milestone gives us a shared contract between:

- the current direct Python solver
- future logic-program compilers
- future VM backends
- future Prolog lowering

It also gives us a much clearer answer to:

> "What does a logic program look like as executable data?"

That question becomes increasingly important once we want:

- tracing
- serialization
- bytecode compilation
- alternate runtimes
- cross-language frontends

## Test Strategy

Required tests:

- declare relations, facts, rules, and a query, then run them end to end
- reject undeclared relation use
- reject facts with variables in the head
- infer outputs from a query when `outputs` is omitted
- support multiple stored queries in one instruction stream

## Future Extensions

Later milestones may add:

- lower-level VM opcodes
- explicit clause labels and branch targets
- instruction serialization formats such as JSON
- a compiler from instruction programs into a logic VM
- a VM backend that replaces direct lowering to `logic-engine`

## Why This Milestone Matters

LP07 is the first step toward making the logic system executable in more than
one way.

Right now we solve directly from host-language data structures.

After LP07, we also have a standardized instruction stream. That becomes the
bridge between today's direct engine and tomorrow's VM.
