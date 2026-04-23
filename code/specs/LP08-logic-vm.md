# LP08 — Logic VM

## Overview

LP07 gave the logic stack a standardized instruction stream:

- `DEF_REL`
- `FACT`
- `RULE`
- `QUERY`

That was the first half of the VM story. We can now describe logic programs as
explicit executable data, but we still execute them by lowering the whole
instruction stream directly into `logic-engine`.

LP08 introduces the next layer:

- a small dispatch-table virtual machine
- a mutable runtime state for logic instructions
- step-by-step execution over the LP07 instruction stream

The first version is intentionally simple. It is not a WAM. It does not expose
low-level unification opcodes yet. Instead, it executes the existing
program-level instructions directly and uses `logic-engine` as the proof-search
backend once the program has been loaded.

## Design Goal

Add a Python package that executes `logic-instructions` through a real VM
interface.

That VM should:

- walk an `InstructionProgram` one instruction at a time
- dispatch each opcode to a handler
- accumulate runtime state incrementally
- surface runtime snapshots for tracing and debugging
- execute stored queries after the instruction stream has been loaded

This gives us a proper execution chassis now, while leaving room for a future
lowering step from LP07 instructions into lower-level bytecode.

## Package

Add a new Python package:

```text
code/packages/python/logic-vm
```

## Layer Position

```text
SYM00 Symbol Core
    ↓
LP00 Logic Core
    ↓
LP01 Logic Engine
    ↓
LP07 Logic Instructions
    ↓
LP08 Logic VM   ← this milestone
```

## Why A VM At This Layer

The first VM does not need to know how to perform unification instruction by
instruction.

That lower-level VM will come later.

Right now we want a runtime that answers:

- what state exists while a logic program is loading?
- how do relation declarations, facts, rules, and queries become live runtime
  objects?
- how can we single-step and trace that process?

This mirrors the "framework VM" pattern used elsewhere in the repo: the VM owns
the fetch/dispatch/state loop, while handlers own opcode semantics.

## Scope

LP08 should execute the existing LP07 instruction kinds directly:

- `DEF_REL`
- `FACT`
- `RULE`
- `QUERY`

The VM should not invent new low-level opcodes yet.

It should use a dispatch table keyed by `InstructionOpcode`.

## Runtime Model

The VM runtime state should at minimum track:

- the current `InstructionProgram`
- the instruction pointer
- whether the VM has halted
- a relation registry keyed by `(symbol, arity)`
- the loaded clause list
- the stored query list

The state should be explicit and inspectable so tools can trace or visualize it
later.

## Public API

The package should export:

```python
LogicVM
LogicVMState
LogicVMTraceEntry
LogicVMError
LogicVMValidationError
UnknownInstructionOpcodeError
create_logic_vm
execute
execute_all
```

### Core VM surface

```python
class LogicVM:
    def register(opcode: InstructionOpcode, handler: LogicInstructionHandler) -> None: ...
    def load(program: InstructionProgram) -> None: ...
    def reset() -> None: ...
    def step() -> LogicVMTraceEntry: ...
    def run() -> list[LogicVMTraceEntry]: ...
    def assembled_program() -> Program: ...
    def run_query(query_index: int = 0, limit: int | None = None) -> list[object]: ...
    def run_all_queries(limit: int | None = None) -> list[list[object]]: ...
```

### Convenience helpers

```python
def create_logic_vm() -> LogicVM: ...
def execute(program: InstructionProgram, query_index: int = 0, limit: int | None = None) -> list[object]: ...
def execute_all(program: InstructionProgram, limit: int | None = None) -> list[list[object]]: ...
```

## Handler Semantics

### `DEF_REL`

Registers one relation in the runtime registry.

It should reject duplicate declarations at runtime.

### `FACT`

Checks that the referenced relation has already been declared, checks that the
fact head is ground, then appends one fact clause to the runtime clause list.

### `RULE`

Checks that the rule head relation was declared and that all explicit relation
calls inside the rule body refer to declared relations, then appends one rule
clause to the runtime clause list.

### `QUERY`

Checks that all explicit relation calls inside the query refer to declared
relations, then appends the query to the runtime query list.

## Tracing

Each successful `step()` should return a trace entry describing:

- the instruction index that just ran
- the opcode that ran
- counts after execution:
  - number of declared relations
  - number of loaded clauses
  - number of stored queries

This is intentionally lightweight. We want enough information for tests and
debugging without building a full debugger UI yet.

## Execution Semantics

`load(program)` should:

- reset the VM state
- install the new instruction stream
- set the instruction pointer to the first instruction

`step()` should:

- fail if no program is loaded
- fail if the VM is already halted
- dispatch the current instruction
- advance the instruction pointer unless the handler halts or raises
- mark the VM halted once the instruction pointer moves past the last
  instruction

`run()` should execute `step()` until the VM halts.

Once loading is complete, `run_query(...)` and `run_all_queries(...)` should use
the loaded runtime state to build a `logic-engine.Program` and execute the
stored queries.

## Error Model

Use explicit VM-specific errors for:

- stepping with no loaded program
- stepping after halt
- unknown opcode handlers
- duplicate declarations
- facts that are not ground
- use of undeclared relations
- query index out of range

## Usage Example

```python
from logic_engine import atom, conj, relation, var
from logic_instructions import defrel, fact, instruction_program, query, rule
from logic_vm import create_logic_vm

parent = relation("parent", 2)
ancestor = relation("ancestor", 2)

X = var("X")
Y = var("Y")
Z = var("Z")
Who = var("Who")

program_value = instruction_program(
    defrel(parent),
    defrel(ancestor),
    fact(parent("homer", "bart")),
    fact(parent("homer", "lisa")),
    rule(ancestor(X, Y), parent(X, Y)),
    rule(ancestor(X, Y), conj(parent(X, Z), ancestor(Z, Y))),
    query(ancestor("homer", Who), outputs=(Who,)),
)

vm = create_logic_vm()
vm.load(program_value)
trace = vm.run()

assert trace[-1].query_count == 1
assert vm.run_query() == [atom("bart"), atom("lisa")]
```

## Test Strategy

Required tests:

- load and run a full ancestor program end to end through the VM
- single-step a short program and verify instruction pointer and runtime counts
- reject duplicate relation declarations
- reject facts or rules that reference undeclared relations
- reject facts containing logic variables
- reset the VM and prove state is cleared
- run multiple stored queries from one loaded program

## Future Extensions

LP08 is the bridge VM, not the final VM.

Later milestones can add:

- lower-level bytecode opcodes
- choice-point frames
- trail stacks
- explicit backtracking instructions
- compiled clause layouts
- a compiler from LP07 instructions into low-level VM bytecode

## Why This Milestone Matters

LP07 answered:

> "What does a logic program look like as executable data?"

LP08 answers:

> "What does the runtime that consumes that data look like?"

That runtime boundary matters because it gives us one place to add:

- tracing
- debugging
- alternate execution strategies
- future bytecode lowering
