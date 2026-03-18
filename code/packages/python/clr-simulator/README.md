# CLR IL Simulator

**Layer 4f** of the computing stack — a simulator for Microsoft's Common Language Runtime Intermediate Language (CIL/MSIL).

## What is this?

This package simulates the execution of CLR IL bytecode, the intermediate language that powers the .NET platform. C#, F#, VB.NET, and PowerShell all compile to CIL, which the CLR then JIT-compiles to native code. This simulator executes CIL instructions directly, producing detailed step-by-step traces.

## Where it fits

```
Source -> Lexer -> Parser -> Compiler -> CLR Simulator (you are here)
```

This is one of several VM simulators in the coding-adventures project:

| Simulator | Year | Architecture | Type System |
|-----------|------|-------------|-------------|
| JVM | 1995 | Stack-based | Typed opcodes (iadd, ladd, fadd) |
| **CLR** | **2002** | **Stack-based** | **Type-inferred (add works on any type)** |
| WASM | 2017 | Stack-based | Typed opcodes (i32.add, i64.add) |

## Key differences from JVM

The CLR takes a different approach to type information:

- **JVM**: Type is encoded in the opcode (`iadd` for int, `ladd` for long)
- **CLR**: Type is inferred from the stack (`add` works for any numeric type)

The CLR also has more short-form constant encodings (0-8 vs JVM's 0-5) and uses a 0xFE prefix for extended opcodes.

## Supported instructions

| Category | Instructions |
|----------|-------------|
| Constants | `ldc.i4.0`..`ldc.i4.8`, `ldc.i4.s`, `ldc.i4`, `ldnull` |
| Locals | `ldloc.0`..`ldloc.3`, `stloc.0`..`stloc.3`, `ldloc.s`, `stloc.s` |
| Arithmetic | `add`, `sub`, `mul`, `div` |
| Comparison | `ceq`, `cgt`, `clt` (two-byte opcodes with 0xFE prefix) |
| Branching | `br.s`, `brfalse.s`, `brtrue.s` |
| Control | `nop`, `ret` |

## Usage

```python
from clr_simulator import CLRSimulator, CLROpcode, assemble_clr, encode_ldc_i4, encode_stloc

# Compute x = 1 + 2
sim = CLRSimulator()
sim.load(assemble_clr(
    encode_ldc_i4(1),       # ldc.i4.1 — push 1
    encode_ldc_i4(2),       # ldc.i4.2 — push 2
    (CLROpcode.ADD,),       # add — pop 2 and 1, push 3
    encode_stloc(0),        # stloc.0 — pop 3, store in local 0
    (CLROpcode.RET,),       # ret — halt
))
traces = sim.run()

print(f"Result: {sim.locals[0]}")  # 3

for trace in traces:
    print(f"  PC={trace.pc}: {trace.opcode:12s} stack: {trace.stack_before} -> {trace.stack_after}")
```

## Development

```bash
cd code/packages/python/clr-simulator
uv pip install -e ".[dev]"
pytest
```
