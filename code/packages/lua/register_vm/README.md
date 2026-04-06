# register_vm

A generic register-based virtual machine with an accumulator model and feedback vectors, modeled after V8's Ignition interpreter.

## What is this?

This package implements a complete bytecode interpreter for a fictional programming language. It's designed to teach how modern JavaScript engines (like V8) execute code under the hood.

### Key Concepts

**Register VM vs Stack VM**

Most teaching examples use stack-based VMs (like the Java Virtual Machine). This VM uses a _register_ model, like V8's Ignition interpreter:

- Stack VM: operands are pushed to/popped from a stack — `PUSH 3, PUSH 5, ADD`
- Register VM: operands live in named registers — `LDAR r0, ADD r1, STAR r2`

Register VMs tend to be faster in practice because fewer instructions are needed, and the operand locations are explicit.

**Accumulator Model**

Most operations read from / write to a special register called the **accumulator** (`acc`). You load a value _into_ the accumulator, then operate on it:

```
LDA_SMI  5     ; acc = 5
STAR     r0    ; r0 = acc = 5
LDA_SMI  3     ; acc = 3
ADD      r0    ; acc = acc + r0 = 8
RETURN         ; return acc
```

**Feedback Vectors (Inline Caches)**

Each instruction that touches a value has an optional **feedback slot**. The slot records which _types_ have been seen at that operation site:

| State | Meaning |
|-------|---------|
| `uninitialized` | Never executed yet |
| `monomorphic` | Always seen the same type pair (fast path!) |
| `polymorphic` | Seen 2–4 distinct type pairs (medium) |
| `megamorphic` | Seen 5+ distinct pairs — give up optimizing |

This simulates V8's inline caches (ICs), which let the JIT compile specialized machine code for the most common type combinations.

**Hidden Classes**

Every object in the VM has a **hidden class ID** — an integer that identifies the object's "shape" (its set of property names). Two objects with the same properties in the same insertion order share a hidden class. This allows property lookups to be compiled into constant-offset reads instead of hash-table lookups.

## Opcodes

The VM supports ~70 opcodes grouped by category:

| Range | Category |
|-------|----------|
| `0x00–0x06` | Load immediates into accumulator |
| `0x10–0x12` | Move values between accumulator and registers |
| `0x20–0x25` | Global and context (scope) variable access |
| `0x30–0x3F` | Arithmetic and bitwise operations |
| `0x40–0x4C` | Comparison and logical operations |
| `0x50–0x58` | Control flow (jumps) |
| `0x60–0x66` | Function calls and returns |
| `0x70–0x77` | Property access (named and keyed) |
| `0x80–0x85` | Object/array/closure creation |
| `0x90–0x93` | Iterator protocol |
| `0xA0–0xA1` | Exception handling |
| `0xB0–0xB3` | Context and module variable access |
| `0xF0, 0xFE, 0xFF` | VM meta (stack check, debugger, halt) |

## Usage

```lua
local VM      = require("coding_adventures.register_vm")
local Opcodes = VM.Opcodes

-- Build a code object (the VM's "compiled function"):
local code = VM.make_code_object({
  name           = "add_two_numbers",
  register_count = 1,
  instructions   = {
    VM.make_instruction(Opcodes.LDA_SMI, {5}, -1),  -- acc = 5
    VM.make_instruction(Opcodes.STAR,    {0}, -1),  -- r0 = 5
    VM.make_instruction(Opcodes.LDA_SMI, {3}, -1),  -- acc = 3
    VM.make_instruction(Opcodes.ADD,     {0}, -1),  -- acc = acc + r0 = 8
    VM.make_instruction(Opcodes.RETURN,  {}, -1),   -- return acc
  },
})

-- Execute it:
local result = VM.execute(code, {})
print(result.value)  -- 8
print(result.error)  -- nil

-- Execute with trace (for debugging):
local result, trace = VM.execute_with_trace(code, {})
for _, step in ipairs(trace) do
  print(string.format("ip=%d opcode=0x%02X acc=%s",
    step.ip, step.opcode, tostring(step.accumulator)))
end
```

### Calling a Function

```lua
-- Inner function: returns 77
local inner_code = VM.make_code_object({
  name         = "inner",
  instructions = {
    VM.make_instruction(Opcodes.LDA_SMI, {77}, -1),
    VM.make_instruction(Opcodes.RETURN,  {},   -1),
  },
})

local inner_fn = { kind = "function", code = inner_code, context = nil }

-- Outer: call the inner function
local outer_code = VM.make_code_object({
  constants = { inner_fn },
  instructions = {
    VM.make_instruction(Opcodes.LDA_CONSTANT,      {0},    -1),
    VM.make_instruction(Opcodes.CALL_ANY_RECEIVER, {0, 0}, -1),
    VM.make_instruction(Opcodes.RETURN,            {},     -1),
  },
})

local result = VM.execute(outer_code, {})
print(result.value)  -- 77
```

### Working with Objects

```lua
-- Create a VM object (simulates a JS object)
local obj = VM.new_vm_object({ x = 10, y = 20 })
print(obj.properties.x)         -- 10
print(obj.__hidden_class_id)     -- some integer (e.g. 1)

-- Two objects with the same shape share a hidden class
local obj2 = VM.new_vm_object({ x = 99, y = 0 })
print(obj.__hidden_class_id == obj2.__hidden_class_id)  -- true
```

### Feedback Slots

```lua
local slot = VM.new_feedback_slot()
print(slot.kind)  -- "uninitialized"

VM.record_feedback(slot, "int:int")
print(slot.kind)  -- "monomorphic"

VM.record_feedback(slot, "float:int")
print(slot.kind)  -- "polymorphic"
```

## Architecture

```
execute(code_object, globals)
  └── run_frame(vm, frame)
        ├── Dispatch table: handlers[opcode](vm, frame, instr)
        ├── Feedback recording: record_feedback(slot, type_pair)
        └── For CALL_ANY_RECEIVER:
              └── call_vm_function(vm, fn_table, args)
                    └── run_frame(vm, inner_frame)  [recursive]
```

## Data Structures

### CodeObject
```lua
{
  name               = "function_name",
  instructions       = { ... },  -- array of RegisterInstruction
  constants          = { ... },  -- constant pool (1-indexed)
  names              = { ... },  -- variable name strings (1-indexed)
  register_count     = 0,        -- how many registers to allocate
  feedback_slot_count = 0,       -- how many feedback slots to allocate
  parameter_count    = 0,        -- number of function parameters
}
```

### RegisterInstruction
```lua
{
  opcode        = 0x00,    -- one of M.Opcodes.*
  operands      = { ... }, -- array of integer operands (0-based indices)
  feedback_slot = -1,      -- index into feedback_vector (-1 = no feedback)
}
```

### CallFrame
```lua
{
  code            = CodeObject,
  ip              = 1,         -- instruction pointer (1-based Lua index)
  accumulator     = nil,       -- current accumulator value
  registers       = { ... },   -- array of register values
  feedback_vector = { ... },   -- array of FeedbackSlot tables
  context         = Context,   -- current scope chain
  caller_frame    = Frame,     -- nil for top-level
}
```

### FeedbackSlot
```lua
{ kind = "uninitialized" }
{ kind = "monomorphic", types = {"int:int"} }
{ kind = "polymorphic", types = {"int:int", "float:int"} }
{ kind = "megamorphic" }
```

## Dependencies

- Lua >= 5.3 (requires integer bitwise operators: `&`, `|`, `~`, `<<`, `>>`)

## Development

```bash
# Run tests (from package root)
bash BUILD

# Run tests manually (from tests/ directory)
busted . --verbose --pattern=test_
```

## Further Reading

- [V8 Ignition Design](https://v8.dev/blog/ignition-interpreter) — the real-world inspiration
- [Lua 5.1 VM Instructions](http://luaforge.net/docman/83/98/ANoFrillsIntroToLua51VMInstructions.pdf) — another excellent register VM
- [Crafting Interpreters](https://craftinginterpreters.com/) — free online book that builds a VM step by step
