# 04a — Bytecode Compiler Backends

## Overview

The bytecode compiler currently targets our custom VM. We need additional
backends that compile the same AST to JVM bytecode, CLR IL, and WASM
bytecode. This demonstrates a key compiler design principle:

**One frontend (lexer + parser), multiple backends (code generators).**

```
                    ┌──→ VM Bytecode      (existing)
Source → AST ──────┼──→ JVM Bytecode     (new)
                    ├──→ CLR IL Bytecode  (new)
                    └──→ WASM Bytecode   (new)
```

## Architecture

Each backend is a module in the `bytecode-compiler` package:

```
bytecode-compiler/
  src/bytecode_compiler/
    compiler.py          ← existing (targets our VM)
    jvm_compiler.py      ← new (targets JVM)
    clr_compiler.py      ← new (targets CLR)
    wasm_compiler.py     ← new (targets WASM)
```

All backends share the same interface:

```python
class JVMCompiler:
    def compile(self, program: Program) -> JVMCodeObject:
        ...

class CLRCompiler:
    def compile(self, program: Program) -> CLRCodeObject:
        ...

class WASMCompiler:
    def compile(self, program: Program) -> bytes:
        ...
```

## JVM Backend

Walks the AST and emits JVM bytecode bytes:

```python
# x = 1 + 2
# AST: Assignment(Name("x"), BinaryOp(Number(1), "+", Number(2)))

# JVM output:
bytecode = bytes([
    0x04,       # iconst_1
    0x05,       # iconst_2
    0x60,       # iadd
    0x3B,       # istore_0
    0xB1,       # return
])
```

For constants > 5, uses `bipush N` (0x10, N) or `ldc` with a constant pool.

## CLR Backend

Walks the AST and emits CLR IL bytes:

```python
# x = 1 + 2
bytecode = bytes([
    0x17,       # ldc.i4.1
    0x18,       # ldc.i4.2
    0x58,       # add
    0x0A,       # stloc.0
    0x2A,       # ret
])
```

For constants > 8, uses `ldc.i4.s N` (0x1F, N) or `ldc.i4` with 4-byte encoding.

## WASM Backend

Walks the AST and emits WASM bytecode:

```python
# x = 1 + 2
bytecode = bytes([
    0x41, 0x01, 0x00, 0x00, 0x00,  # i32.const 1
    0x41, 0x02, 0x00, 0x00, 0x00,  # i32.const 2
    0x6A,                           # i32.add
    0x21, 0x00,                     # local.set 0
    0x0B,                           # end
])
```

## Compilation mapping

| AST Node | Our VM | JVM | CLR | WASM |
|----------|--------|-----|-----|------|
| Number(1) | LOAD_CONST 0 | iconst_1 | ldc.i4.1 | i32.const 1 |
| Number(42) | LOAD_CONST 0 | bipush 42 | ldc.i4.s 42 | i32.const 42 |
| BinaryOp(+) | ADD | iadd | add | i32.add |
| BinaryOp(-) | SUB | isub | sub | i32.sub |
| BinaryOp(*) | MUL | imul | mul | i32.mul |
| BinaryOp(/) | DIV | idiv | div | i32.div |
| Assignment(x, ...) | STORE_NAME 0 | istore_0 | stloc.0 | local.set 0 |
| Name(x) | LOAD_NAME 0 | iload_0 | ldloc.0 | local.get 0 |

## End-to-end flow

```python
from bytecode_compiler import JVMCompiler
from jvm_simulator import JVMSimulator

# Compile
code = JVMCompiler().compile(ast)

# Execute
sim = JVMSimulator()
sim.load(code.bytecode, code.constants, num_locals=code.num_locals)
traces = sim.run()
# Local variable 0 now contains 3
```
