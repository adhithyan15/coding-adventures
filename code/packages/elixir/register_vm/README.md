# register_vm

A standalone register-based virtual machine for Elixir, implementing the V8 Ignition-style execution model: accumulator register + per-function register file + inline type feedback vectors.

## What is this?

This is an educational implementation of a **register-based VM** — the architecture used by production engines like V8 (JavaScript), CPython (Python), and Lua. It differs from stack-based VMs (like the JVM or .NET CLR) in one key way: instead of pushing and popping values on an operand stack, most instructions read from and write to a single **accumulator register**, with additional named registers for intermediate values.

This design mirrors how real CPUs work. It is the same model used by V8's Ignition bytecode interpreter, introduced in 2016 to replace V8's older "full-codegen" direct-to-machine-code approach.

## Architecture

### Accumulator-centric execution

Most instructions implicitly read from and write to a single register called the **accumulator** (acc). Binary operations take one explicit register operand:

```
Add r0    # acc = acc + registers[0]
```

Compare this to a stack VM:

```
Push r0   # stack = [r0, ...]
Push r1   # stack = [r1, r0, ...]
Add       # stack = [r0 + r1, ...]  (pops 2, pushes 1)
```

The accumulator model needs fewer instructions (no redundant push/pop), produces a more compact bytecode stream, and is easier for a JIT compiler to analyse.

### Register file

Each call frame has a **fixed-size register file** — a tuple of `N` slots where `N` is determined at compile time by the code object. Registers are indexed by small non-negative integers (r0, r1, r2, ...). They hold:

- Function arguments (passed in r0, r1, ...)
- Local variables that need to survive across instruction boundaries
- Temporary intermediate values

### Feedback vectors

Every function has a **feedback vector**: a flat array of observation slots, one per dynamic-dispatch instruction (arithmetic, property access, call site). Each time the interpreter executes such an instruction, it records what types it saw:

```
# After executing Add with int+int once:
[{:monomorphic, [{:integer, :integer}]}]

# After seeing int+string too:
[{:polymorphic, [{:integer, :string}, {:integer, :integer}]}]
```

A JIT compiler would read this feedback to decide whether to emit a specialised integer fast path, a more general polymorphic dispatch, or an unoptimised slow path.

### Slot state machine

Each feedback slot passes through four states:

```
:uninitialized
    │ (first observation)
    ▼
{:monomorphic, [type_pair]}
    │ (different type pair)
    ▼
{:polymorphic, [pair1, pair2, ...]}   (2–4 distinct pairs)
    │ (5th distinct pair)
    ▼
:megamorphic   ◄── terminal, never changes
```

### Call frames

Each function invocation creates a `%CallFrame{}` holding:
- `code` — the `%CodeObject{}` being executed
- `ip` — instruction pointer (index into the instruction list)
- `accumulator` — the current working value
- `registers` — tuple of `register_count` slots
- `feedback_vector` — list of `feedback_slot_count` observation slots
- `caller_frame` — link to the frame that made the call

## Usage

```elixir
alias CodingAdventures.RegisterVM
alias CodingAdventures.RegisterVM.Types.{CodeObject, RegisterInstruction}
alias CodingAdventures.RegisterVM.Opcodes

# 1 + 2 = 3
code = %CodeObject{
  instructions: [
    %RegisterInstruction{opcode: Opcodes.lda_smi(), operands: [1]},   # acc = 1
    %RegisterInstruction{opcode: Opcodes.star(), operands: [0]},       # r0 = acc
    %RegisterInstruction{opcode: Opcodes.lda_smi(), operands: [2]},   # acc = 2
    %RegisterInstruction{opcode: Opcodes.add(), operands: [0, 0]},    # acc = acc + r0
    %RegisterInstruction{opcode: Opcodes.halt(), operands: []}
  ],
  constants: [],
  names: [],
  register_count: 1,
  feedback_slot_count: 1,
  name: "add_example"
}

{:ok, result} = RegisterVM.execute(code)
result.return_value   # => 3
result.error          # => nil
result.final_feedback_vector  # => [{:monomorphic, [{:integer, :integer}]}]
```

### Tracing execution

```elixir
{:ok, result, trace} = RegisterVM.execute_with_trace(code)

# Each step shows before/after state:
Enum.each(trace, fn step ->
  IO.puts("ip=#{step.ip} acc: #{inspect(step.acc_before)} → #{inspect(step.acc_after)}")
end)
```

### Global variables

```elixir
%CodeObject{
  instructions: [
    %RegisterInstruction{opcode: Opcodes.lda_smi(), operands: [42]},
    %RegisterInstruction{opcode: Opcodes.sta_global(), operands: [0]},   # globals["x"] = 42
    %RegisterInstruction{opcode: Opcodes.lda_global(), operands: [0]},   # acc = globals["x"]
    %RegisterInstruction{opcode: Opcodes.halt(), operands: []}
  ],
  constants: [],
  names: ["x"],
  ...
}
```

## Opcode categories

| Range  | Category              | Example opcodes                        |
|--------|-----------------------|----------------------------------------|
| 0x00–  | Accumulator loads     | LdaConstant, LdaZero, LdaSmi, LdaNull |
| 0x10–  | Register moves        | Ldar, Star, Mov                        |
| 0x20–  | Variable access       | LdaGlobal, StaGlobal, LdaContextSlot  |
| 0x30–  | Arithmetic            | Add, Sub, Mul, Div, Mod, Pow, Negate   |
| 0x40–  | Comparisons           | TestEqual, TestLessThan, LogicalNot    |
| 0x50–  | Control flow          | Jump, JumpIfFalse, JumpIfTrue, JumpLoop|
| 0x60–  | Calls / returns       | CallAnyReceiver, Return, Construct     |
| 0x70–  | Property access       | LdaNamedProperty, StaNamedProperty     |
| 0x80–  | Object/array creation | CreateObjectLiteral, CreateClosure     |
| 0x90–  | Iteration             | GetIterator, CallIteratorStep          |
| 0xA0–  | Exceptions            | Throw, Rethrow                         |
| 0xB0–  | Context/scope         | PushContext, PopContext, LdaContextSlot|
| 0xF0–  | VM control            | StackCheck, Debugger, Halt             |

## Layer position

This VM sits at the "language runtime" layer of the computing stack:

```
─────────────────────────────────────────
  Source code (e.g., JavaScript)
─────────────────────────────────────────
  Lexer / Parser → AST
─────────────────────────────────────────
  Bytecode compiler → CodeObject
─────────────────────────────────────────
  register_vm ← YOU ARE HERE
─────────────────────────────────────────
  Elixir BEAM (host VM)
─────────────────────────────────────────
  Operating system
─────────────────────────────────────────
```

Above this layer: a bytecode compiler that translates ASTs into `%CodeObject{}` structs.
Below this layer: the Elixir BEAM VM, which handles memory management, scheduling, and GC.

## Running tests

```bash
mix deps.get && mix test --cover
```
