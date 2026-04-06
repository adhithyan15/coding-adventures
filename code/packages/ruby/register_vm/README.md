# coding_adventures_register_vm

A generic register-based virtual machine with an accumulator model and per-function
feedback vectors, modeled closely on [V8's Ignition bytecode interpreter][ignition].

This is an educational implementation — every line of code is commented to explain
*why*, not just *what*, so that someone learning how VMs work for the first time can
follow along.

## What is a register-based VM?

Most beginner VM implementations use a **stack machine**: every operation pushes and
pops a shared operand stack. Python's CPython and the JVM are classic examples.

A **register machine** is different. Each call frame owns a small array of numbered
registers. Instructions explicitly name their source and destination registers instead
of relying on stack order. This makes the bytecode slightly more verbose but
significantly easier for a JIT compiler to optimize — it can see data flow directly
from the instruction stream rather than tracking the stack.

### The accumulator twist

Ignition (V8's interpreter) goes one step further with an **accumulator register**.
Most arithmetic and comparison results go into a single implicit register called
`acc`. Many instructions only need one explicit operand (the right-hand side); the
left-hand side is always `acc`. This keeps the bytecode compact:

```
LDA_CONSTANT 0    ; acc = constants[0]  (e.g. 3)
STAR r0            ; r0  = acc           (save 3)
LDA_CONSTANT 1    ; acc = constants[1]  (e.g. 4)
ADD  r0            ; acc = acc + r0      (= 7)
HALT               ; return acc
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Interpreter                                                 │
│                                                              │
│  @globals  ─────────────────────────────────────────────►  │
│  @output   (PRINT accumulates here)                          │
│  @call_depth (stack-overflow guard)                          │
│                                                              │
│  ┌─────────────────┐   caller_frame ptr   ┌──────────────┐ │
│  │  CallFrame (top)│ ─────────────────►   │  CallFrame   │ │
│  │  code           │                      │  (parent)    │ │
│  │  ip             │                      └──────────────┘ │
│  │  accumulator    │                                        │
│  │  registers[]    │                                        │
│  │  feedback_vec[] │                                        │
│  │  context ──────────────────────────────► Context chain  │
│  └─────────────────┘                                        │
└─────────────────────────────────────────────────────────────┘
```

### Key modules

| File | Responsibility |
|------|---------------|
| `opcodes.rb` | ~60 opcode constants + name table |
| `types.rb` | `CodeObject`, `RegisterInstruction`, `CallFrame`, `VMObject`, `VMFunction`, `VMResult`, `VMError`, `Context`, `TraceStep` |
| `feedback.rb` | IC (inline cache) state machine: `uninitialized → monomorphic → polymorphic → megamorphic` |
| `scope.rb` | Lexical scope chain — `Context` linked list, depth/index slot access |
| `interpreter.rb` | Main `run_frame` dispatch loop |

## Feedback vectors and inline caches

Every `CodeObject` carries a `feedback_slot_count`. When the interpreter executes the
code, it allocates a feedback vector of that size — one slot per instrumented operation.

Each slot tracks the runtime types it has seen:

```
:uninitialized                 # never executed
{ kind: :monomorphic, types: [["number","number"]] }   # one type pair
{ kind: :polymorphic, types: [["number","number"],["string","string"]] }
:megamorphic                   # >4 distinct pairs — JIT gives up
```

This mirrors V8's IC system. A future JIT tier can read the feedback vector to emit
specialized machine code for the most common types.

## Value representation

| JavaScript concept | Ruby representation |
|--------------------|---------------------|
| `undefined` | `UNDEFINED` sentinel object (not `nil`) |
| `null` | `nil` |
| number | `Integer` or `Float` |
| string | `String` |
| boolean | `true` / `false` |
| object | `VMObject` struct with `hidden_class_id` and `properties` |
| function | `VMFunction` struct with `code` and captured `context` |
| array | Ruby `Array` |

## Usage

```ruby
require "coding_adventures_register_vm"

include CodingAdventures::RegisterVM

# Build a simple program: return 6 * 7
code = CodeObject.new(
  name: "multiply",
  instructions: [
    RegisterInstruction.new(opcode: Opcodes::LDA_CONSTANT, operands: [0]),  # acc = 6
    RegisterInstruction.new(opcode: Opcodes::STAR,         operands: [0]),  # r0  = 6
    RegisterInstruction.new(opcode: Opcodes::LDA_CONSTANT, operands: [1]),  # acc = 7
    RegisterInstruction.new(opcode: Opcodes::MUL,          operands: [0]),  # acc = 42
    RegisterInstruction.new(opcode: Opcodes::HALT,         operands: []),
  ],
  constants:          [6, 7],
  names:              [],
  register_count:     1,
  feedback_slot_count: 1,
  parameter_count:    0
)

result = CodingAdventures::RegisterVM.execute(code)
puts result.return_value  # => 42
```

### Tracing execution

```ruby
steps = CodingAdventures::RegisterVM.execute_with_trace(code)
steps.each do |step|
  puts "ip=#{step.ip} #{step.opcode_name} acc: #{step.accumulator_before} → #{step.accumulator_after}"
end
```

## Opcodes (summary)

| Range | Category |
|-------|----------|
| 0x00–0x06 | Accumulator loads (`LDA_CONSTANT`, `LDA_ZERO`, `LDA_TRUE`, …) |
| 0x10–0x11 | Register moves (`STAR`, `MOV`) |
| 0x20–0x2F | Arithmetic and bitwise (`ADD`, `SUB`, `MUL`, `DIV`, `BIT_AND`, …) |
| 0x30–0x3A | Comparison (`CMP_EQ`, `CMP_LT`, `TEST_NULL`, …) |
| 0x40–0x45 | Control flow (`JUMP`, `JUMP_IF_TRUE`, `LOOP`, …) |
| 0x50–0x53 | Function calls (`CALL`, `RETURN`, `CALL_BUILTIN`, `CREATE_CLOSURE`) |
| 0x60–0x65 | Scope chain (`LOAD_GLOBAL`, `STORE_GLOBAL`, `LOAD_CONTEXT_SLOT`, …) |
| 0x70–0x74 | Objects (`CREATE_OBJECT`, `LOAD_PROPERTY`, `STORE_PROPERTY`, …) |
| 0x80–0x84 | Arrays (`CREATE_ARRAY`, `LOAD_ELEMENT`, `PUSH_ELEMENT`, …) |
| 0x90–0x93 | Type coercion (`TYPEOF`, `TO_NUMBER`, `TO_STRING`, `TO_BOOLEAN`) |
| 0xA0–0xA3 | Logical (`LOGICAL_OR`, `LOGICAL_AND`, `LOGICAL_NOT`, `NULLISH_COALESCE`) |
| 0xB0 | I/O (`PRINT`) |
| 0xFF | VM control (`HALT`) |

## Where this fits in the computing stack

This VM sits in the **language runtime** layer. It depends on nothing below it in
the stack, but it is a building block for:

- A bytecode compiler (e.g. `starlark_ast_to_bytecode_compiler`) that emits
  `CodeObject` + `RegisterInstruction` records from a parsed AST.
- A JIT compiler that reads the feedback vectors and emits native machine code.

## Running the tests

```
bundle install
bundle exec rake test
```

## References

- [V8 Ignition — an interpreter for V8][ignition]
- [Benedikt Meurer — "What's up with Monomorphism?"][mono]
- Knuth, *The Art of Computer Programming* — MIX machine (the original register VM)

[ignition]: https://v8.dev/blog/ignition-interpreter
[mono]: https://mrale.ph/blog/2015/01/11/whats-up-with-monomorphism.html
