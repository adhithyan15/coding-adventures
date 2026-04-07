# @coding-adventures/register-vm

A standalone register-based virtual machine modelled on V8's Ignition interpreter. This package is **not** built on top of the stack-based `virtual-machine` package — it is an independent implementation of the accumulator + register file + feedback-vector execution model.

## What is a register VM?

Traditional stack-based VMs (JVM, .NET CLR, the `virtual-machine` package) store intermediate values on a stack. Every instruction implicitly pops inputs and pushes results.

Register-based VMs (V8 Ignition, Lua 5, Dalvik/ART) name registers explicitly. V8 uses a hybrid called the **accumulator model**: one special accumulator register is the implicit input/output for most instructions, and a small register file holds named locals.

```
Stack VM:                     Register VM (accumulator model):

PUSH 10                       LDA_SMI 10        ; acc = 10
PUSH 3                        STAR r0           ; r0  = acc
ADD        ; stack = [13]     LDA_SMI 3         ; acc = 3
PUSH 5                        ADD r0            ; acc = acc + r0 = 13
MUL        ; stack = [65]     STAR r1           ; r1 = 13
                              LDA_SMI 5
                              MUL r1            ; acc = 5 * 13 = 65
```

The accumulator model reduces instruction count (no constant `PUSH`/`POP` noise) and simplifies JIT compilation (the JIT can allocate the accumulator to a hardware register permanently).

## Features

- **70 opcodes** covering loads, arithmetic, bitwise, comparisons, jumps, calls, property access, object/array creation, iterators, exceptions, and scope management
- **Feedback vectors** — every arithmetic and property-access instruction records type information. The state machine transitions: `uninitialized → monomorphic → polymorphic → megamorphic`
- **Hidden classes** — objects carry a monotonically-increasing `hiddenClassId` that changes when new properties are added, enabling inline-cache simulation
- **Closure support** — `CREATE_CLOSURE` captures the current lexical context; calls restore the closure's saved context chain
- **Lexical scope chains** — `Context` objects form a linked list; `LDA_CONTEXT_SLOT [depth, idx]` walks the chain
- **Stack overflow protection** — `STACK_CHECK` opcode throws a VMError if call depth exceeds `maxDepth`
- **Step-by-step tracing** — `executeWithTrace()` returns a `TraceStep[]` with accumulator and register snapshots before/after each instruction, plus feedback deltas

## Quick start

```typescript
import { RegisterVM, Opcode } from '@coding-adventures/register-vm';
import type { CodeObject } from '@coding-adventures/register-vm';

const vm = new RegisterVM();

// Compute 40 + 2 = 42
const code: CodeObject = {
  name: 'main',
  instructions: [
    { opcode: Opcode.LDA_SMI,  operands: [40], feedbackSlot: null },
    { opcode: Opcode.STAR,     operands: [0],  feedbackSlot: null },  // r0 = 40
    { opcode: Opcode.LDA_SMI,  operands: [2],  feedbackSlot: null },
    { opcode: Opcode.ADD,      operands: [0, 0], feedbackSlot: 0 },   // acc = 2 + r0, record in slot 0
    { opcode: Opcode.HALT,     operands: [],   feedbackSlot: null },
  ],
  constants: [],
  names: [],
  registerCount: 2,
  feedbackSlotCount: 1,
  parameterCount: 0,
};

const result = vm.execute(code);
console.log(result.returnValue);  // 42
console.log(result.error);        // null
```

## Feedback vectors

After executing the code above, the feedback vector at slot 0 is `monomorphic` with type pair `['number', 'number']`:

```typescript
const { result, trace } = vm.executeWithTrace(code);
const addStep = trace.find(s => s.instruction.opcode === Opcode.ADD);
console.log(addStep.feedbackDelta[0].after);
// { kind: 'monomorphic', types: [['number', 'number']] }
```

If the same slot sees different type pairs (e.g. string+string later), it transitions to `polymorphic` and eventually `megamorphic`. A real JIT compiler uses this information to decide which fast path to emit.

## How closures work

```typescript
import { RegisterVM, Opcode, newObject } from '@coding-adventures/register-vm';

// Inner function: return the first parameter + 1
const adderCode = {
  name: 'addOne',
  instructions: [
    { opcode: Opcode.LDAR,    operands: [0], feedbackSlot: null },  // acc = arg0
    { opcode: Opcode.ADD_SMI, operands: [1], feedbackSlot: null },  // acc += 1
    { opcode: Opcode.RETURN,  operands: [],  feedbackSlot: null },
  ],
  constants: [], names: [],
  registerCount: 1, feedbackSlotCount: 0,
  parameterCount: 1,
};

const outerCode = {
  name: 'outer',
  instructions: [
    { opcode: Opcode.CREATE_CLOSURE,    operands: [0], feedbackSlot: null }, // acc = closure(adderCode)
    { opcode: Opcode.STAR,              operands: [0], feedbackSlot: null }, // r0 = closure
    { opcode: Opcode.LDA_SMI,           operands: [41], feedbackSlot: null },
    { opcode: Opcode.STAR,              operands: [1], feedbackSlot: null }, // r1 = 41 (argument)
    { opcode: Opcode.CALL_ANY_RECEIVER, operands: [0, 1, 1, 0], feedbackSlot: null },
    { opcode: Opcode.HALT,              operands: [], feedbackSlot: null },
  ],
  constants: [adderCode], names: [],
  registerCount: 4, feedbackSlotCount: 1,
  parameterCount: 0,
};

const vm = new RegisterVM();
const result = vm.execute(outerCode);
console.log(result.returnValue);  // 42
```

## Where this fits in the computing stack

```
Layer 6: OS Kernel
Layer 5: Virtual Machines  ← this package (register-based, V8 Ignition style)
Layer 4: Bytecode compilers
Layer 3: Language parsers
Layer 2: Language lexers
Layer 1: Data structures
Layer 0: Logic gates / transistors
```

This package provides the execution model that a bytecode compiler (e.g. `starlark-ast-to-bytecode-compiler`) would target when generating register-based bytecode rather than stack-based bytecode.

## API reference

### `RegisterVM`

```typescript
new RegisterVM(options?: { maxDepth?: number })
vm.execute(code: CodeObject): VMResult
vm.executeWithTrace(code: CodeObject): { result: VMResult; trace: TraceStep[] }
```

### Object helpers

```typescript
newObject(): VMObject          // Create a new object with a fresh hiddenClassId
objectWithHiddenClass(obj)     // Create a shape-transitioned copy of an object
```

### Feedback utilities

```typescript
newVector(size: number): FeedbackSlot[]
valueType(v: VMValue): string                                          // 'number' | 'string' | ...
recordBinaryOp(vector, slot, left, right): void
recordPropertyLoad(vector, slot, hiddenClassId): void
recordCallSite(vector, slot, calleeType): void
```

### Scope utilities

```typescript
newContext(parent, slotCount): Context
getSlot(ctx, depth, idx): VMValue
setSlot(ctx, depth, idx, value): void
```

### Opcode utilities

```typescript
Opcode.LDA_CONSTANT  // 0x00
Opcode.HALT          // 0xFF
// ... all 70+ opcodes

opcodeName(op: number): string  // 0x00 → 'LDA_CONSTANT'
```
