# register-vm

A standalone register-based bytecode VM for Swift, modelled after V8's Ignition interpreter. No dependencies.

## What it does

`register-vm` implements a complete register-based virtual machine with:

- **80+ opcodes** spanning accumulator loads, register moves, variable access,
  arithmetic, bitwise operations, comparisons, control flow, function calls,
  property access, object/array creation, iteration helpers, exceptions,
  context/scope management, and VM control.
- **Accumulator register**: a single implicit register that most opcodes read
  from or write to, avoiding the need to encode two register fields for common
  binary operations.
- **Type-feedback inline caches**: every arithmetic and property-access
  instruction has an optional feedback slot that records the runtime types seen
  (uninitialized → monomorphic → polymorphic → megamorphic). This simulates
  how V8 collects profiling information to drive JIT compilation.
- **Call frames**: each function invocation gets a `CallFrame` with its own
  register file, accumulator, feedback vector, and lexical context.
- **Lexical contexts**: closure variables are stored in a `Context` chain
  (depth-indexed), matching the scope-chain model used by JavaScript engines.
- **Native function hooks**: register Swift closures under a string name and
  call them from bytecode exactly like compiled functions.

## How it fits in the stack

```
logic gates → arithmetic → CPU pipeline → assembler → virtual-machine
                                                          ↓
                                                    register-vm  ← you are here
```

`register-vm` sits above `virtual-machine` (a generic stack-based interpreter)
and demonstrates how a *register* layout trades instruction width for fewer
instructions and a more cache-friendly dispatch loop.

## Usage

```swift
import RegisterVM

// Build a tiny program: return 3 + 4
let code = CodeObject(
    instructions: [
        RegisterInstruction(opcode: .ldaSmi,  operands: [3]),   // acc = 3
        RegisterInstruction(opcode: .star,    operands: [0]),   // r0  = 3
        RegisterInstruction(opcode: .ldaSmi,  operands: [4]),   // acc = 4
        RegisterInstruction(opcode: .add,     operands: [0], feedbackSlot: 0),  // acc = r0 + acc
        RegisterInstruction(opcode: .return_),
    ],
    constants: [],
    names: [],
    registerCount: 1,
    feedbackSlotCount: 1
)

var vm = RegisterVM()
let result = vm.execute(code)
// result.returnValue == .integer(7)
```

### Calling a function

```swift
let inner = CodeObject(
    instructions: [
        RegisterInstruction(opcode: .ldaSmi,   operands: [42]),
        RegisterInstruction(opcode: .return_),
    ],
    constants: [], names: [], registerCount: 1, feedbackSlotCount: 0, name: "answer"
)

let outer = CodeObject(
    instructions: [
        RegisterInstruction(opcode: .ldaConstant,     operands: [0]),  // acc = func
        RegisterInstruction(opcode: .callAnyReceiver, operands: [1, 0]), // call()
        RegisterInstruction(opcode: .halt),
    ],
    constants: [.function(inner, nil)],
    names: [], registerCount: 2, feedbackSlotCount: 0
)

var vm = RegisterVM()
let result = vm.execute(outer)
// result.returnValue == .integer(42)
```

### Inspecting type feedback

```swift
var fv = FeedbackSlot.newVector(size: 1)
recordBinaryOp(vector: &fv, slot: 0, left: .integer(1), right: .integer(2))
// fv[0] == .monomorphic(types: [("Smi", "Smi")])

recordBinaryOp(vector: &fv, slot: 0, left: .float(1.5), right: .float(2.5))
// fv[0] == .polymorphic(types: [("Smi","Smi"), ("Number","Number")])
```

## Running tests

```bash
swift test
# or on macOS:
xcrun swift test
```

## Key types

| Type | Description |
|------|-------------|
| `Opcode` | 80+ bytecode operations as a `UInt8` enum |
| `VMValue` | Universal value type (int, float, string, bool, null, undefined, object, array, function) |
| `CodeObject` | Compiled bytecode unit: instructions + constants + names + register count |
| `RegisterInstruction` | Single instruction: opcode + operands + optional feedback slot |
| `CallFrame` | Live execution state: IP, accumulator, registers, feedback vector |
| `Context` | Lexical scope node for closure variables |
| `FeedbackSlot` | IC state: uninitialized / monomorphic / polymorphic / megamorphic |
| `RegisterVM` | The interpreter struct with `execute(_:) -> VMResult` |

## Learning notes

- **Register vs stack VMs**: stack VMs are simpler to implement (every operand
  is implicit) but require more instructions. Register VMs like Lua 5 and V8's
  Ignition use wider instructions to reduce instruction count and improve cache
  utilisation.
- **Inline caches**: the feedback vector is the foundation of adaptive
  optimisation. Without it, a JIT must generate generic (slow) code; with it,
  a JIT can specialise for the types actually seen.
- **Hidden classes**: JavaScript objects are dynamic, so a naive property
  lookup walks a hash map. Hidden classes let an engine track *which objects
  share the same layout*, enabling fixed-offset property access.
