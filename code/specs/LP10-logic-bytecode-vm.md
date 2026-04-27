# LP10 — Logic Bytecode VM

## Overview

The logic stack now has:

- `logic-instructions` — a standardized program-level instruction stream
- `logic-vm` — a VM that executes those high-level instructions directly
- `logic-bytecode` — a compact loader-bytecode representation of that stream

That gives us a good compiler boundary, but the bytecode itself is still only a
data format. We cannot yet point a VM at `LogicBytecodeProgram` and run it end
to end.

LP10 adds that missing piece.

## Design Goal

Add a Python package that executes LP09 loader bytecode directly.

This VM should:

- accept a `LogicBytecodeProgram`
- walk its raw opcode stream one instruction at a time
- resolve operands against the bytecode pools
- accumulate runtime state incrementally
- surface trace snapshots for debugging
- execute stored queries after loading completes

The goal is not to become a WAM yet. The goal is to make the first bytecode
format *real* by giving it a direct runtime.

## Why This Milestone Matters

LP08 proved that standardized logic instructions can drive a VM.

LP09 proved that those instructions can be lowered into a compact bytecode
object.

LP10 closes the loop:

- compile structured instructions into bytecode
- execute that bytecode through a VM
- run relational programs without first decoding back to LP07

That is a strong stepping stone toward future clause-level and unification-level
bytecode.

## Package

Add a new Python package:

```text
code/packages/python/logic-bytecode-vm
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
LP08 Logic VM
    ↓
LP09 Logic Bytecode
    ↓
LP10 Logic Bytecode VM   ← this milestone
```

## Scope

LP10 should execute the existing LP09 loader-bytecode instructions directly:

- `EMIT_RELATION`
- `EMIT_FACT`
- `EMIT_RULE`
- `EMIT_QUERY`
- `HALT`

This is still a **loader VM**. It does not yet execute low-level unification
opcodes or explicit backtracking instructions.

## Public API

The package should export:

```python
LogicBytecodeVM
LogicBytecodeVMState
LogicBytecodeVMTraceEntry
LogicBytecodeVMError
LogicBytecodeVMValidationError
UnknownLogicBytecodeOpcodeError
create_logic_bytecode_vm
execute
execute_all
compile_and_execute
compile_and_execute_all
```

## Runtime Model

The runtime state should explicitly track:

- the current `LogicBytecodeProgram`
- the instruction pointer
- whether the VM has halted
- a relation registry keyed by `(symbol, arity)`
- the loaded clause list
- the stored query list

The state model should look familiar to LP08, but the fetch/decode step should
start from `LogicBytecodeInstruction` and the operand pools rather than from
`InstructionProgram`.

## Core VM Surface

```python
class LogicBytecodeVM:
    def register(opcode: LogicBytecodeOp, handler: LogicBytecodeHandler) -> None: ...
    def load(program: LogicBytecodeProgram) -> None: ...
    def reset() -> None: ...
    def step() -> LogicBytecodeVMTraceEntry: ...
    def run() -> list[LogicBytecodeVMTraceEntry]: ...
    def assembled_program() -> Program: ...
    def run_query(query_index: int = 0, limit: int | None = None) -> list[object]: ...
    def run_all_queries(limit: int | None = None) -> list[list[object]]: ...
```

## Convenience Helpers

```python
def create_logic_bytecode_vm() -> LogicBytecodeVM: ...
def execute(program: LogicBytecodeProgram, query_index: int = 0, limit: int | None = None) -> list[object]: ...
def execute_all(program: LogicBytecodeProgram, limit: int | None = None) -> list[list[object]]: ...

def compile_and_execute(
    instruction_program: InstructionProgram,
    query_index: int = 0,
    limit: int | None = None,
) -> list[object]: ...

def compile_and_execute_all(
    instruction_program: InstructionProgram,
    limit: int | None = None,
) -> list[list[object]]: ...
```

The compile helpers are important because they let users treat LP10 as a
drop-in bytecode path for the existing instruction layer.

## Handler Semantics

### `EMIT_RELATION`

- require an operand
- resolve the operand through `relation_pool`
- reject duplicate declarations
- register the relation in runtime state

### `EMIT_FACT`

- require an operand
- resolve the operand through `fact_pool`
- require that the fact's relation was declared
- require that the fact head is ground
- append the fact clause to runtime state

### `EMIT_RULE`

- require an operand
- resolve the operand through `rule_pool`
- require that the rule head relation was declared
- require that all explicit relation calls in the body were declared
- append the rule clause to runtime state

### `EMIT_QUERY`

- require an operand
- resolve the operand through `query_pool`
- require that all explicit relation calls in the goal were declared
- append the query to runtime state

### `HALT`

- halt the VM
- require that `HALT` is the final instruction in the bytecode stream

## Execution Semantics

`load(program)` should:

- reset the VM state
- store the incoming `LogicBytecodeProgram`
- set the instruction pointer to `0`
- mark the VM as not halted

`step()` should:

- fail if no program is loaded
- fail if the VM is already halted
- fetch the current bytecode instruction
- normalize the opcode
- dispatch to the registered handler
- advance the instruction pointer unless the handler halts the VM

`run()` should execute `step()` until the VM halts.

Once loading is complete, `run_query(...)` and `run_all_queries(...)` should
use the assembled runtime state to build a `logic-engine.Program` and execute
stored queries.

## Tracing

Each successful `step()` should return a trace entry with:

- the bytecode instruction index that ran
- the normalized opcode
- counts after execution:
  - number of declared relations
  - number of loaded clauses
  - number of stored queries

This keeps the first bytecode VM inspectable without introducing a full
debugger.

## Error Model

Use explicit VM-specific errors for:

- stepping with no loaded program
- stepping after halt
- unknown bytecode opcode handlers
- duplicate declarations
- facts that are not ground
- use of undeclared relations
- malformed or invalid bytecode operands
- query index out of range

Malformed bytecode should fail closed. The VM must not silently tolerate:

- unknown opcode integers
- missing operands
- negative pool indexes
- out-of-range pool indexes
- `HALT` in the middle of the program

## Relationship To Existing Packages

`logic-bytecode-vm` should depend on:

- `logic-engine`
- `logic-instructions`
- `logic-bytecode`

It may share validation patterns with `logic-vm`, but it should not simply
decode back into an `InstructionProgram` and hand off execution. The bytecode VM
exists to exercise LP09 directly.

## Usage Example

```python
from logic_engine import conj, relation, var
from logic_instructions import defrel, fact, instruction_program, query, rule
from logic_bytecode import compile_program
from logic_bytecode_vm import create_logic_bytecode_vm

parent = relation("parent", 2)
ancestor = relation("ancestor", 2)

X = var("X")
Y = var("Y")
Z = var("Z")
Who = var("Who")

instruction_stream = instruction_program(
    defrel(parent),
    defrel(ancestor),
    fact(parent("homer", "bart")),
    fact(parent("homer", "lisa")),
    rule(ancestor(X, Y), parent(X, Y)),
    rule(ancestor(X, Y), conj(parent(X, Z), ancestor(Z, Y))),
    query(ancestor("homer", Who), outputs=(Who,)),
)

bytecode = compile_program(instruction_stream)

vm = create_logic_bytecode_vm()
vm.load(bytecode)
trace = vm.run()
answers = vm.run_query()

assert len(trace) == 8
assert answers == [("bart",), ("lisa",)]
```

## Test Strategy

Required tests:

- load and run a bytecode program compiled from a real instruction stream
- verify `step()` returns useful trace entries
- verify recursive programs execute correctly through bytecode
- verify `compile_and_execute(...)` matches `execute(...)`
- reject duplicate declarations
- reject undeclared relations in facts, rules, and queries
- reject non-ground facts
- reject unknown opcode integers
- reject missing operands
- reject negative and out-of-range pool indexes
- reject `HALT` before the final instruction
- reject stepping after halt

## Future Extensions

LP10 prepares the stack for more serious execution formats later:

- clause-entry bytecode
- explicit call and return opcodes
- choice-point and trail instructions
- low-level unification bytecode
- serialized bytecode bundles
- alternate runtimes in other languages

## Summary

LP10 makes bytecode execution real.

Instead of treating LP09 as a passive compact representation, we now have a VM
that can load, trace, and run it directly. That keeps the engine-first roadmap
moving and gives the future Prolog frontend a stronger reusable backend.
