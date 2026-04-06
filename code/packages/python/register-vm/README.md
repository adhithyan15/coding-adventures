# register-vm

A standalone register-based virtual machine for Python, inspired by V8's Ignition interpreter.

## What is a register VM?

Most teaching VMs (including this repo's `virtual-machine` package) use a *stack machine*: values are pushed onto a value stack and operations pop their operands from there. A *register machine* is different: values live in a fixed array of named registers, and each instruction names its operand registers explicitly.

V8's Ignition bytecode adds an **accumulator** — a single hidden register that is the implicit source and destination for most operations. This hybrid model:

- Reduces instruction size (most ops only need one explicit operand index)
- Maps naturally to hardware CPUs (which also have accumulator/flags registers)
- Simplifies JIT compilation (the accumulator's type is known at any monomorphic site)

```
Stack machine:          Register machine (Ignition-style):
  PUSH 1                  LDA_SMI 1      ; acc = 1
  PUSH 2                  STAR r0        ; r0  = acc
  ADD                     LDA_SMI 2      ; acc = 2
  ← result on stack       ADD r0         ; acc = acc + r0
```

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                   RegisterVM                        │
│                                                     │
│  _globals: {print: fn, ...}                         │
│  _output:  ["line 1", ...]                          │
│                                                     │
│  ┌──────────────────────────────────────────────┐   │
│  │              CallFrame (current)             │   │
│  │  ip: 4                                       │   │
│  │  accumulator: 42                             │   │
│  │  registers: [10, 32, undefined, ...]         │   │
│  │  feedback_vector: [Mono, Uninit, ...]        │   │
│  │  context: ─────→ Context                    │   │
│  │  caller_frame: ──→ CallFrame                │   │
│  └──────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────┘
```

### Opcode groups

| Range  | Category         | Examples                              |
|--------|------------------|---------------------------------------|
| `0x0_` | Accumulator loads | `LDA_CONSTANT`, `LDA_SMI`, `LDA_TRUE` |
| `0x1_` | Register moves   | `LDAR`, `STAR`, `MOV`                 |
| `0x2_` | Variable access  | `LDA_GLOBAL`, `LDA_CONTEXT_SLOT`      |
| `0x3_` | Arithmetic       | `ADD`, `SUB`, `MUL`, `BITWISE_AND`    |
| `0x4_` | Comparisons      | `TEST_EQUAL`, `TEST_LESS_THAN`        |
| `0x5_` | Control flow     | `JUMP`, `JUMP_IF_FALSE`, `JUMP_LOOP`  |
| `0x6_` | Calls            | `CALL_ANY_RECEIVER`, `RETURN`         |
| `0x7_` | Property access  | `LDA_NAMED_PROPERTY`, `STA_KEYED_PROPERTY` |
| `0x8_` | Creation         | `CREATE_OBJECT_LITERAL`, `CREATE_CLOSURE`  |
| `0x9_` | Iteration        | `GET_ITERATOR`, `CALL_ITERATOR_STEP`  |
| `0xA_` | Exceptions       | `THROW`                               |
| `0xB_` | Context/module   | `PUSH_CONTEXT`, `POP_CONTEXT`         |
| `0xF_` | VM control       | `STACK_CHECK`, `HALT`                 |

### Feedback vectors

Each call frame has a `FeedbackSlot` list. Slots progress through a state machine as the VM records what types flow through each operation site:

```
Uninitialized → Monomorphic (1 type pair)
             → Polymorphic  (2–4 pairs)
             → Megamorphic  (5+ pairs, gives up optimizing)
```

A JIT optimizer would use monomorphic slots to emit specialized machine code. Here the data is recorded for educational purposes.

## Quick start

```python
from register_vm import CodeObject, RegisterInstruction, Opcode, execute

# Compute 3 + 4 and return the result.
code = CodeObject(
    instructions=[
        RegisterInstruction(Opcode.LDA_SMI, [3]),   # acc = 3
        RegisterInstruction(Opcode.STAR, [0]),       # r0 = acc
        RegisterInstruction(Opcode.LDA_SMI, [4]),   # acc = 4
        RegisterInstruction(Opcode.ADD, [0]),        # acc = acc + r0
        RegisterInstruction(Opcode.RETURN),
    ],
    constants=[],
    names=[],
    register_count=1,
    feedback_slot_count=0,
)
result = execute(code)
assert result.return_value == 7
```

## Execution trace

```python
from register_vm import execute_with_trace, Opcode

result, trace = execute_with_trace(code)
for step in trace:
    name = Opcode(step.instruction.opcode).name
    print(f"ip={step.ip:2d}  {name:<30s}  acc: {step.acc_before!r:10} → {step.acc_after!r}")
```

Output:
```
ip= 0  LDA_SMI                         acc: undefined   → 3
ip= 1  STAR                            acc: 3           → 3
ip= 2  LDA_SMI                         acc: 3           → 4
ip= 3  ADD                             acc: 4           → 7
ip= 4  RETURN                          acc: 7           → 7
```

## Calling functions

```python
inner = CodeObject(
    instructions=[
        RegisterInstruction(Opcode.LDA_SMI, [7]),
        RegisterInstruction(Opcode.RETURN),
    ],
    constants=[], names=[], register_count=0, feedback_slot_count=0,
)

outer = CodeObject(
    instructions=[
        RegisterInstruction(Opcode.CREATE_CLOSURE, [0]),         # acc = closure
        RegisterInstruction(Opcode.STAR, [0]),                   # r0 = closure
        RegisterInstruction(Opcode.CALL_ANY_RECEIVER, [0, 1, 0, 0]),  # call r0()
        RegisterInstruction(Opcode.RETURN),
    ],
    constants=[inner], names=[], register_count=2, feedback_slot_count=1,
)
result = execute(outer)
assert result.return_value == 7
```

## Where this fits in the stack

This package lives in the **language runtime layer**. It depends on nothing else in this monorepo. A compiler front-end (parser → AST → bytecode emitter) would produce `CodeObject` instances that this VM executes.

```
Source code
    ↓  (lexer + parser)
Abstract Syntax Tree
    ↓  (compiler / emitter)
CodeObject  ←─ this package executes it
    ↓  (RegisterVM._run_frame)
Result / side-effects
```

## Running tests

```bash
uv venv
uv pip install -e ".[dev]"
.venv/bin/python -m pytest tests/ -v
```
