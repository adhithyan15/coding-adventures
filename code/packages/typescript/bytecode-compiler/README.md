# @coding-adventures/bytecode-compiler

Compiles Abstract Syntax Trees into stack-machine bytecode instructions. Layer 4a of the computing stack.

## What Is This?

The bytecode compiler is the bridge between human-readable syntax (parsed into ASTs by the parser) and machine-executable instructions (run by a virtual machine). It walks the tree and emits a flat sequence of stack operations.

This is exactly what real compilers do:

| Compiler | Input | Output |
|----------|-------|--------|
| `javac` | Java source | JVM bytecode (.class files) |
| `csc` | C# source | CLR IL (.dll files) |
| `cpython` | Python source | Python bytecode (.pyc files) |
| **Ours** | AST | CodeObject / JVM / CLR / WASM bytes |

## Multiple Backends

This package includes four compilation backends that all compile the same AST:

- **BytecodeCompiler** -- Targets our custom VM with high-level instructions (LOAD_CONST, ADD, etc.)
- **JVMCompiler** -- Emits real JVM bytecode bytes with tiered encoding (iconst, bipush, ldc)
- **CLRCompiler** -- Emits real CLR IL bytes with wider short forms (ldc.i4.0 through ldc.i4.8)
- **WASMCompiler** -- Emits real WebAssembly bytes with uniform encoding (i32.const + 4 bytes)

## Installation

```bash
npm install @coding-adventures/bytecode-compiler
```

## Usage

### Quick Start (end-to-end)

```typescript
import { compileSource, VirtualMachine } from "@coding-adventures/bytecode-compiler";

const code = compileSource("x = 2 + 3 * 4");
const vm = new VirtualMachine();
vm.execute(code);
console.log(vm.variables["x"]); // 14
```

### Step by Step (AST to CodeObject)

```typescript
import { tokenize } from "@coding-adventures/lexer";
import { Parser } from "@coding-adventures/parser";
import { BytecodeCompiler } from "@coding-adventures/bytecode-compiler";

const tokens = tokenize("x = 1 + 2");
const ast = new Parser(tokens).parse();
const compiler = new BytecodeCompiler();
const code = compiler.compile(ast);
// code.instructions, code.constants, code.names
```

### JVM Backend

```typescript
import { JVMCompiler } from "@coding-adventures/bytecode-compiler";

const jvmCode = new JVMCompiler().compile(ast);
// jvmCode.bytecode is a Uint8Array of real JVM opcodes
```

### CLR Backend

```typescript
import { CLRCompiler } from "@coding-adventures/bytecode-compiler";

const clrCode = new CLRCompiler().compile(ast);
// clrCode.bytecode is a Uint8Array of real CLR IL opcodes
```

### WASM Backend

```typescript
import { WASMCompiler } from "@coding-adventures/bytecode-compiler";

const wasmCode = new WASMCompiler().compile(ast);
// wasmCode.bytecode is a Uint8Array of real WASM opcodes
```

## How It Works

The compiler performs a **post-order traversal** of the AST, emitting stack-machine instructions. For the expression `1 + 2 * 3`:

```
    AST:         +
               / \
              1   *
                 / \
                2   3

    Output:  LOAD_CONST 1
             LOAD_CONST 2
             LOAD_CONST 3
             MUL
             ADD
```

This is Reverse Polish Notation (RPN) -- the natural output format for stack-machine compilation.

## Dependencies

- `@coding-adventures/lexer` -- Tokenizer (for `compileSource` convenience function)
- `@coding-adventures/parser` -- AST node types and parser

## How It Fits in the Stack

```
Layer 1: Logic Gates
Layer 2: Lexer (source -> tokens)
Layer 3: Parser (tokens -> AST)
Layer 4a: Bytecode Compiler (AST -> bytecode)  <-- THIS PACKAGE
Layer 4b: Virtual Machine (bytecode -> execution)
```

## Development

```bash
npm install
npm test                    # Run tests
npm run test:coverage       # Run tests with coverage
```

## License

MIT
