# @coding-adventures/clr-simulator

CLR IL (Common Language Runtime Intermediate Language) simulator -- Layer 4f of the computing stack.

## What is this?

A TypeScript implementation of a CLR IL bytecode simulator. The CLR is Microsoft's virtual machine at the heart of .NET, executing CIL bytecode produced by C#, F#, VB.NET, and other .NET languages. Unlike the JVM, the CLR uses type-inferred arithmetic (one `add` opcode for all types) and a 0xFE prefix for extended opcodes.

## Supported Instructions

| Instruction | Encoding | Description |
|------------|----------|-------------|
| `ldc.i4.0..8` | 1 byte | Push small int32 constants |
| `ldc.i4.s V` | 2 bytes | Push signed int8 as int32 |
| `ldc.i4 V` | 5 bytes | Push int32 |
| `ldloc.N / stloc.N` | 1 byte | Load/store local (slots 0-3) |
| `ldloc.s / stloc.s` | 2 bytes | Load/store local (any slot) |
| `add/sub/mul/div` | 1 byte | Type-inferred arithmetic |
| `ceq/cgt/clt` | 2 bytes | Comparison (0xFE prefix) |
| `br.s/brfalse.s/brtrue.s` | 2 bytes | Short branch instructions |
| `nop/ldnull/ret` | 1 byte | Miscellaneous |

## Usage

```typescript
import { CLRSimulator, CLROpcode, assembleClr, encodeLdcI4, encodeStloc } from "@coding-adventures/clr-simulator";

const sim = new CLRSimulator();
sim.load(assembleClr(
  encodeLdcI4(1),
  encodeLdcI4(2),
  [CLROpcode.ADD],
  encodeStloc(0),
  [CLROpcode.RET],
));
const traces = sim.run();
console.log(sim.locals[0]); // => 3
```

## How it fits in the stack

This is a TypeScript port of the Python clr-simulator package. It sits at Layer 4f, demonstrating the CLR's type-inferred stack machine with two-byte opcode extensions.
