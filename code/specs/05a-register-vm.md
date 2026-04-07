# 05a — Generic Register-Based Virtual Machine

## Overview

The register-based VM is an alternative execution engine for bytecode. Where the stack-based VM (spec 05) manipulates an implicit operand stack, the register-based VM operates on a fixed set of named virtual registers and a single implicit **accumulator** register that serves as the source and destination for most operations.

This is the execution model used by V8's Ignition interpreter, Lua's VM, and Dalvik (Android's original JVM replacement). It tends to produce fewer instructions than a stack VM for the same program — at the cost of more complex instruction encoding (each instruction must name its operand registers).

This spec also introduces the **feedback vector** — a per-function array of type-recording slots that the VM populates at runtime. The feedback vector is the mechanism by which a JIT compiler learns what types flow through each operation without modifying the instructions themselves.

## Layer Position

```
Logic Gates → Arithmetic → CPU → ARM → Assembler → Lexer → Parser → Compiler → [YOU ARE HERE]
```

**Input from:** Bytecode compiler targeting register bytecode format.
**Output to:** The user — program output, return values.
**Sibling:** Stack-based VM (spec 05) — same layer, different execution model.

## Concepts

### The Accumulator

The accumulator (`acc`) is an implicit register. Most instructions read from `acc`, write to `acc`, or both:

```
LdaConstant 0       # acc = constants[0]
Star r0             # r0  = acc
LdaConstant 1       # acc = constants[1]
Add r0              # acc = acc + r0
Star r1             # r1  = acc
```

This contrasts with the stack VM where `ADD` implicitly pops two values and pushes one. Here, `Add r0` means "add register r0 to the accumulator and store the result back in the accumulator." The operand register is always named explicitly; the accumulator is always implied.

### The Register File

The VM maintains a fixed-size array of virtual registers (`r0`, `r1`, `r2`, ...). These are per-call-frame — each function invocation gets its own register file. The number of registers needed for a given function is known at compile time and encoded in the function's `CodeObject`.

```
Frame layout:
┌─────────────┐
│ accumulator │   implicit, one per frame
├─────────────┤
│ r0          │
│ r1          │   general-purpose registers
│ r2          │   count fixed at compile time
│ ...         │
├─────────────┤
│ context     │   pointer to enclosing scope chain
└─────────────┘
```

### Feedback Slots

A **feedback vector** is an array allocated alongside each function's `CodeObject`. Each slot corresponds to a specific instruction in the bytecode — typically one that does dynamic dispatch (property load, call, arithmetic on unknown types).

The VM writes to a feedback slot every time it executes the corresponding instruction:

| Slot type | What gets recorded |
|---|---|
| `CallSite` | callee shape (monomorphic / polymorphic / megamorphic) |
| `PropertyLoad` | object hidden class seen at this load site |
| `BinaryOp` | pair of operand types seen (e.g., `int × int`, `string × int`) |
| `CompareOp` | operand types seen |
| `InstanceOf` | constructor seen |

A JIT compiler reads this feedback to make type-specialized decisions. If a `BinaryOp` slot has only ever seen `int × int`, the JIT emits an integer add with no type checks. If the slot has seen mixed types, the JIT emits a slower generic path.

The bytecode compiler encodes a **feedback slot index** as an operand on every instruction that needs one. The VM uses that index to read/write the feedback vector at runtime. The compiler does not need to know what the VM records — it just assigns indices.

```
# Instruction format for a property load:
LdaNamedProperty  r0  [name_idx: 3]  [feedback_slot: 7]
                  ↑        ↑                 ↑
               object    name pool      feedback vector
               register    index            index
```

### Hidden Classes (Shapes)

To make property access fast, the VM assigns every object a **hidden class** (also called a shape or map). All objects with the same set of properties in the same order share a hidden class. Property access becomes an array index lookup rather than a hash table lookup:

```
obj = { x: 1, y: 2 }
# hidden class: Shape_A → { x: offset 0, y: offset 1 }
# obj.x → obj.fields[0]   (no hash lookup needed)
```

When a property is added or deleted, the object transitions to a new hidden class. The feedback vector records which hidden classes were seen at each property access site — if only one shape is ever seen (monomorphic), the JIT can hardcode the offset.

### Call Frames

Each function call pushes a new **call frame** onto the call stack. A frame contains:

- The function's register file
- The function's accumulator
- The return address (instruction pointer of the caller)
- A pointer to the caller's frame
- The function's feedback vector
- The context register (scope chain)

When `Return` executes, the current frame is popped and the accumulator value is transferred to the caller's accumulator.

### The Eval Loop

```elixir
def run(frame) do
  instruction = fetch(frame)
  case instruction.opcode do
    :lda_constant ->
      frame |> set_acc(constants(frame, instruction.arg))
            |> advance()
            |> run()

    :star ->
      frame |> set_register(instruction.arg, acc(frame))
            |> advance()
            |> run()

    :add ->
      left  = acc(frame)
      right = register(frame, instruction.reg)
      slot  = instruction.feedback_slot
      result = do_add(left, right)
      frame |> set_acc(result)
            |> record_binary_op(slot, left, right)
            |> advance()
            |> run()

    :return ->
      {acc(frame), caller_frame(frame)}
  end
end
```

## Instruction Set

Instructions are grouped by opcode prefix. Each instruction encodes its operands explicitly. The accumulator is never named — it is always implied.

### 0x0_ — Accumulator Loads (constants and immediates)

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0x00 | `LdaConstant` | `idx` | `acc = constants[idx]` |
| 0x01 | `LdaZero` | — | `acc = 0` |
| 0x02 | `LdaSmi` | `value` | `acc = small_int(value)` (inline integer, no pool) |
| 0x03 | `LdaUndefined` | — | `acc = undefined` |
| 0x04 | `LdaNull` | — | `acc = null` |
| 0x05 | `LdaTrue` | — | `acc = true` |
| 0x06 | `LdaFalse` | — | `acc = false` |

### 0x1_ — Register Moves

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0x10 | `Ldar` | `reg` | `acc = reg` |
| 0x11 | `Star` | `reg` | `reg = acc` |
| 0x12 | `Mov` | `src, dst` | `dst = src` |

### 0x2_ — Variable Access

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0x20 | `LdaGlobal` | `name_idx, slot` | `acc = globals[name_idx]`; record shape in `slot` |
| 0x21 | `StaGlobal` | `name_idx, slot` | `globals[name_idx] = acc` |
| 0x22 | `LdaLocal` | `reg` | alias for `Ldar` — named for readability |
| 0x23 | `StaLocal` | `reg` | alias for `Star` — named for readability |
| 0x24 | `LdaContextSlot` | `depth, idx` | `acc = scope_chain[depth].slots[idx]` |
| 0x25 | `StaContextSlot` | `depth, idx` | `scope_chain[depth].slots[idx] = acc` |
| 0x26 | `LdaCurrentContextSlot` | `idx` | `acc = current_context.slots[idx]` |
| 0x27 | `StaCurrentContextSlot` | `idx` | `current_context.slots[idx] = acc` |

### 0x3_ — Arithmetic

All binary arithmetic reads one operand from a register, the other from `acc`, writes result to `acc`, and records operand types in `feedback_slot`.

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0x30 | `Add` | `reg, slot` | `acc = acc + reg` |
| 0x31 | `Sub` | `reg, slot` | `acc = acc - reg` |
| 0x32 | `Mul` | `reg, slot` | `acc = acc * reg` |
| 0x33 | `Div` | `reg, slot` | `acc = acc / reg` |
| 0x34 | `Mod` | `reg, slot` | `acc = acc % reg` |
| 0x35 | `Pow` | `reg, slot` | `acc = acc ** reg` |
| 0x36 | `AddSmi` | `value, slot` | `acc = acc + small_int(value)` (inline operand, common fast path) |
| 0x37 | `SubSmi` | `value, slot` | `acc = acc - small_int(value)` |
| 0x38 | `BitwiseAnd` | `reg, slot` | `acc = acc & reg` |
| 0x39 | `BitwiseOr` | `reg, slot` | `acc = acc \| reg` |
| 0x3A | `BitwiseXor` | `reg, slot` | `acc = acc ^ reg` |
| 0x3B | `BitwiseNot` | `slot` | `acc = ~acc` |
| 0x3C | `ShiftLeft` | `reg, slot` | `acc = acc << reg` |
| 0x3D | `ShiftRight` | `reg, slot` | `acc = acc >> reg` |
| 0x3E | `ShiftRightLogical` | `reg, slot` | `acc = acc >>> reg` (unsigned) |
| 0x3F | `Negate` | `slot` | `acc = -acc` |

### 0x4_ — Comparisons

All comparisons write a boolean to `acc` and record operand types in `feedback_slot`.

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0x40 | `TestEqual` | `reg, slot` | `acc = (acc == reg)` abstract equality |
| 0x41 | `TestNotEqual` | `reg, slot` | `acc = (acc != reg)` |
| 0x42 | `TestStrictEqual` | `reg, slot` | `acc = (acc === reg)` no coercion |
| 0x43 | `TestStrictNotEqual` | `reg, slot` | `acc = (acc !== reg)` |
| 0x44 | `TestLessThan` | `reg, slot` | `acc = (acc < reg)` |
| 0x45 | `TestGreaterThan` | `reg, slot` | `acc = (acc > reg)` |
| 0x46 | `TestLessThanOrEqual` | `reg, slot` | `acc = (acc <= reg)` |
| 0x47 | `TestGreaterThanOrEqual` | `reg, slot` | `acc = (acc >= reg)` |
| 0x48 | `TestIn` | `reg` | `acc = (acc in reg)` property existence |
| 0x49 | `TestInstanceOf` | `reg, slot` | `acc = (acc instanceof reg)` |
| 0x4A | `TestUndetectable` | — | `acc = (acc == null \|\| acc == undefined)` |
| 0x4B | `LogicalNot` | — | `acc = !acc` |
| 0x4C | `TypeOf` | — | `acc = typeof acc` (string) |

### 0x5_ — Control Flow

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0x50 | `Jump` | `offset` | `ip += offset` |
| 0x51 | `JumpIfTrue` | `offset` | `if acc then ip += offset` |
| 0x52 | `JumpIfFalse` | `offset` | `if !acc then ip += offset` |
| 0x53 | `JumpIfNull` | `offset` | `if acc == null then ip += offset` |
| 0x54 | `JumpIfUndefined` | `offset` | `if acc == undefined then ip += offset` |
| 0x55 | `JumpIfNullOrUndefined` | `offset` | `if acc == null \|\| acc == undefined then ip += offset` |
| 0x56 | `JumpIfToBooleanTrue` | `offset` | `if ToBoolean(acc) then ip += offset` |
| 0x57 | `JumpIfToBooleanFalse` | `offset` | `if !ToBoolean(acc) then ip += offset` |
| 0x58 | `JumpLoop` | `offset` | backward jump (separate opcode for JIT loop detection) |

### 0x6_ — Function Calls

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0x60 | `CallAnyReceiver` | `callable, first_arg, argc, slot` | call with any receiver (undefined if none) |
| 0x61 | `CallProperty` | `callable, receiver, first_arg, argc, slot` | call with explicit receiver (method call) |
| 0x62 | `CallUndefinedReceiver` | `callable, first_arg, argc, slot` | call with `undefined` as receiver (function call) |
| 0x63 | `Construct` | `constructor, first_arg, argc, slot` | `new constructor(...args)` |
| 0x64 | `ConstructWithSpread` | `constructor, first_arg, argc, slot` | `new constructor(...spread)` |
| 0x65 | `CallWithSpread` | `callable, first_arg, argc, slot` | `fn(...spread)` |
| 0x66 | `Return` | — | return `acc` to caller |
| 0x67 | `SuspendGenerator` | `reg` | save frame state into generator object at `reg`, yield `acc` |
| 0x68 | `ResumeGenerator` | `reg` | restore frame from generator at `reg`, `acc = sent_value` |

### 0x7_ — Property Access

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0x70 | `LdaNamedProperty` | `obj, name_idx, slot` | `acc = obj.names[name_idx]`; record shape in `slot` |
| 0x71 | `StaNamedProperty` | `obj, name_idx, slot` | `obj.names[name_idx] = acc` |
| 0x72 | `LdaKeyedProperty` | `obj, slot` | `acc = obj[acc]`; key from acc, record in `slot` |
| 0x73 | `StaKeyedProperty` | `obj, key, slot` | `obj[key] = acc` |
| 0x74 | `LdaNamedPropertyNoFeedback` | `obj, name_idx` | `acc = obj.names[name_idx]`; no slot (builtins fast path) |
| 0x75 | `StaNamedPropertyNoFeedback` | `obj, name_idx` | `obj.names[name_idx] = acc` |
| 0x76 | `DeletePropertyStrict` | `reg` | `delete reg[acc]` in strict mode |
| 0x77 | `DeletePropertySloppy` | `reg` | `delete reg[acc]` in sloppy mode |

### 0x8_ — Object and Array Creation

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0x80 | `CreateObjectLiteral` | `template_idx, slot, flags` | create `{}` from boilerplate in constants; record shape |
| 0x81 | `CreateArrayLiteral` | `template_idx, slot, flags` | create `[]` from boilerplate |
| 0x82 | `CreateRegExpLiteral` | `pattern_idx, flags_idx, slot` | create `RegExp` |
| 0x83 | `CreateClosure` | `code_idx, slot, flags` | wrap `CodeObject` into a callable closure |
| 0x84 | `CreateContext` | `scope_info_idx, reg` | push new scope context |
| 0x85 | `CloneObject` | `src, flags, slot` | shallow clone for object spread `{ ...src }` |

### 0x9_ — Iteration

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0x90 | `GetIterator` | `slot` | `acc = acc[Symbol.iterator]()`; record in `slot` |
| 0x91 | `GetAsyncIterator` | `slot` | async iteration protocol |
| 0x92 | `CallIteratorStep` | `iter, slot` | `acc = iter.next()`; record shape in `slot` |
| 0x93 | `GetIteratorDone` | `iter_result` | `acc = iter_result.done` |
| 0x94 | `GetIteratorValue` | `iter_result` | `acc = iter_result.value` |

### 0xA_ — Exceptions

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0xA0 | `Throw` | — | throw `acc` as exception |
| 0xA1 | `ReThrow` | — | rethrow current exception without changing stack trace |
| 0xA2 | `ThrowIfNotSuperAlreadyCalled` | — | guard for `super()` call check |
| 0xA3 | `SetPendingMessage` | — | set pending exception message |
| 0xA4 | `GetPendingMessage` | — | `acc = current pending message` |

### 0xB_ — Context and Scope

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0xB0 | `PushContext` | `reg` | push new context, store old in `reg` |
| 0xB1 | `PopContext` | `reg` | restore context from `reg` |
| 0xB2 | `LdaLookupSlot` | `name_idx` | dynamic lookup (slow path for `eval` and `with`) |
| 0xB3 | `StaLookupSlot` | `name_idx, flags` | dynamic store |
| 0xB4 | `LdaModuleVariable` | `cell_idx, depth` | load from module-level variable |
| 0xB5 | `StaModuleVariable` | `cell_idx, depth` | store to module-level variable |

### 0xF_ — VM Control

| Opcode | Mnemonic | Operands | Effect |
|---|---|---|---|
| 0xF0 | `StackCheck` | — | check for stack overflow; trigger interrupt checks |
| 0xF1 | `Debugger` | — | breakpoint trap |
| 0xFF | `Halt` | — | stop execution |

## Feedback Vector

Each `CodeObject` carries a `feedback_slot_count` field. At function instantiation time, the VM allocates a `FeedbackVector` of that size, initialized to `:uninitialized`.

### Slot States

```
:uninitialized   — slot has never been reached
:monomorphic     — one type/shape seen so far      (fast JIT path)
:polymorphic     — 2–4 types/shapes seen           (slower JIT path)
:megamorphic     — 5+ types/shapes seen            (JIT gives up, generic path)
```

### Recording (Interpreter)

The interpreter updates slots on every execution:

```elixir
def record_binary_op(frame, slot, left, right) do
  type_pair = {value_type(left), value_type(right)}
  feedback  = get_slot(frame, slot)

  new_state = case feedback do
    :uninitialized          -> {:monomorphic, [type_pair]}
    {:monomorphic, [^type_pair]} -> feedback          # same type, no change
    {:monomorphic, types}   -> {:polymorphic, [type_pair | types]}
    {:polymorphic, types} when length(types) < 4 ->
                               {:polymorphic, [type_pair | types]}
    _                       -> :megamorphic
  end

  set_slot(frame, slot, new_state)
end
```

### Reading (JIT — future)

The JIT reads the feedback vector when compiling a function and uses it to choose which code path to emit. This is out of scope for the interpreter implementation but the feedback vector must be populated correctly so it is ready when the JIT is added.

## CodeObject Format

```elixir
defstruct [
  :instructions,        # list of RegisterInstruction
  :constants,           # constant pool (numbers, strings, nested CodeObjects)
  :names,               # name pool (variable and property names)
  :register_count,      # number of registers this function needs
  :feedback_slot_count, # size of the feedback vector to allocate
  :parameter_count,     # number of named parameters
  :name,                # debug name (function name or "<anonymous>")
  :source_position_table  # maps instruction index → source line/col (for stack traces)
]
```

## RegisterInstruction Format

```elixir
defstruct [
  :opcode,      # atom or integer
  :operands,    # list — number and meaning depend on opcode
  :feedback_slot  # integer | nil — index into feedback vector
]
```

## Call Frame Format

```elixir
defstruct [
  :code,            # CodeObject being executed
  :ip,              # instruction pointer (index into code.instructions)
  :accumulator,     # current accumulator value
  :registers,       # array of size code.register_count
  :feedback_vector, # array of size code.feedback_slot_count
  :context,         # current scope chain (linked list of context frames)
  :caller_frame,    # reference to enclosing call frame | nil if top level
  :return_value     # set by Return, read by caller
]
```

## Public API

```elixir
defmodule RegisterVM do
  # Execute a compiled CodeObject from the top level.
  # Returns {:ok, result} | {:error, VMError}
  @spec execute(CodeObject.t()) :: {:ok, VMResult.t()} | {:error, VMError.t()}
  def execute(code)

  # Execute with full execution trace for debugging.
  @spec execute_with_trace(CodeObject.t()) :: {:ok, VMResult.t(), [TraceStep.t()]} | {:error, VMError.t()}
  def execute_with_trace(code)

  # Read the feedback vector for a function after execution.
  # Used by tests to verify type recording.
  @spec feedback_vector(FunctionValue.t()) :: FeedbackVector.t()
  def feedback_vector(fn_value)
end

defmodule VMResult do
  defstruct [:output, :return_value, :error]
  # output:       list of strings (captured print/console.log output)
  # return_value: final accumulator value of the top-level frame
  # error:        VMError | nil
end

defmodule VMError do
  defstruct [:message, :instruction_index, :instruction, :stack_trace]
end

defmodule TraceStep do
  defstruct [
    :frame_depth,       # call depth (0 = top level)
    :ip,                # instruction pointer before this step
    :instruction,       # RegisterInstruction
    :acc_before,        # accumulator value before execution
    :acc_after,         # accumulator value after execution
    :registers_before,  # register file snapshot before
    :registers_after,   # register file snapshot after
    :feedback_delta     # slots that changed this step
  ]
end
```

## Comparison with Stack VM

| Aspect | Stack VM (spec 05) | Register VM (spec 05a) |
|---|---|---|
| Operand model | implicit stack | explicit registers + implicit accumulator |
| Instruction count for `a + b` | 3 (push a, push b, add) | 3 (lda a, star r0, lda b, add r0) — similar |
| Instruction size | smaller (no register operands) | larger (register indices in operands) |
| Type recording | not built in | feedback vector per function |
| JIT friendliness | moderate | high — feedback slots are already wired in |
| Implementation complexity | lower | higher |
| Real-world examples | CPython, JVM, .NET CLR | V8 Ignition, Lua VM, Dalvik |

## Comparison with V8 Ignition

This VM is designed to be structurally similar to V8 Ignition so that a bytecode compiler targeting V8 can be validated by comparing output against `node --print-bytecode`. Key similarities:

- Accumulator-centric instruction set
- Feedback slots encoded in instructions
- `LdaSmi` for inline small integers
- `JumpLoop` as a distinct opcode for loop detection
- `Star` / `Ldar` as the register transfer pair
- Hidden class recording at property access sites

Key intentional simplifications (for learnability):
- No bytecode serialization format (V8 uses a custom binary format)
- No OSR (on-stack replacement) — the JIT path is a future extension
- No concurrent marking or write barriers — GC is out of scope for this spec
- Feedback vector uses atoms instead of V8's encoded SMI tags

## Test Strategy

- `LdaConstant` + `Return`: verify `acc` gets the constant value
- `Star` / `Ldar`: verify register move in both directions
- `Add r0 [slot=0]`: verify arithmetic result in `acc` and that `slot 0` records `{:monomorphic, [{:integer, :integer}]}`
- `Add` with mixed types: verify slot transitions from monomorphic → polymorphic → megamorphic
- `LdaNamedProperty` with consistent shape: verify monomorphic feedback
- `LdaNamedProperty` with two different shapes: verify polymorphic feedback
- Nested function call: verify new frame pushed, registers isolated, return value lands in caller's `acc`
- `JumpIfFalse`: verify IP advances normally when acc is truthy, jumps when falsy
- `JumpLoop`: verify backward jump works (loop terminates correctly)
- `SuspendGenerator` / `ResumeGenerator`: verify generator frame saved and restored
- Stack overflow: verify `StackCheck` triggers error at configurable depth
- End-to-end: compile `x = 1 + 2` to register bytecode, execute, verify result
- End-to-end: compile a function with a call, verify call frame pushed and popped correctly
- Trace mode: verify each step records correct `acc_before`, `acc_after`, `feedback_delta`

## Future Extensions

- **JIT compiler**: reads feedback vector, compiles hot functions to native code
- **OSR (on-stack replacement)**: replace an executing function mid-run with compiled version
- **Hidden class transitions**: full object shape tracking for property access optimization
- **Inline caches**: cache the last-seen shape at each property access site in the frame
- **Type-specialized arithmetic**: when feedback is monomorphic `int×int`, skip boxing checks
