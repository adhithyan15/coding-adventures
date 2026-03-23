# @coding-adventures/pipeline

**Orchestrator that chains lexer, parser, compiler, and VM into a single execution flow with trace capture for visualization.**

This is the TypeScript port of the Python `coding-adventures-pipeline` package.

## What It Does

The pipeline is the assembly line of our computing stack. Raw source code enters at one end, and fully executed results come out the other. Along the way, every intermediate stage is captured for inspection and visualization:

```
Source code  ->  Lexer  ->  Parser  ->  Compiler  ->  VM
                  |           |            |           |
              tokens        AST       bytecode     traces
```

1. **Lexer** — Breaks raw text into tokens (NAME, NUMBER, PLUS, etc.)
2. **Parser** — Builds an Abstract Syntax Tree encoding precedence and grouping
3. **Compiler** — Walks the AST and emits stack-machine bytecode
4. **Virtual Machine** — Executes bytecode one instruction at a time, recording traces

## Usage

```typescript
import { Pipeline } from "@coding-adventures/pipeline";

const result = new Pipeline().run("x = 1 + 2");

// Inspect each stage:
console.log(result.lexerStage.tokenCount);         // Number of tokens
console.log(result.parserStage.astDict);            // JSON-serializable AST
console.log(result.compilerStage.instructionsText); // Human-readable bytecode
console.log(result.vmStage.finalVariables);         // { x: 3 }
```

### Multiple Statements

```typescript
const result = new Pipeline().run("a = 10\nb = 20\nc = a + b");
console.log(result.vmStage.finalVariables);
// { a: 10, b: 20, c: 30 }
```

### With Custom Keywords

```typescript
const result = new Pipeline().run("if x = 1", ["if", "else"]);
// The word "if" will be classified as a KEYWORD token
```

## How It Fits in the Stack

```
Layer 1:  Logic Gates          (NAND, AND, OR, XOR)
Layer 2:  Arithmetic           (addition, subtraction from gates)
Layer 3:  Floating Point       (IEEE 754 from integer arithmetic)
Layer 4:  Clock                (tick/tock timing)
Layer 5:  Cache                (LRU, direct-mapped, set-associative)
Layer 6:  Lexer                (tokenization) ◄── dependency
Layer 7:  Parser               (AST construction) ◄── dependency
Layer 8:  Compiler             (AST -> bytecode) ◄── included locally
Layer 9:  Virtual Machine      (bytecode execution) ◄── included locally
Layer 10: Pipeline             (THIS PACKAGE — orchestrates layers 6-9)
Layer 11: HTML Renderer        (visualization)
```

## Architecture

The pipeline package includes self-contained implementations of the bytecode compiler and virtual machine. When the dedicated `@coding-adventures/bytecode-compiler` and `@coding-adventures/virtual-machine` TypeScript packages are ported, the pipeline will switch to importing from them.

### Exported Types

- `Pipeline` — The main orchestrator class
- `PipelineResult` — Complete result bundle from all four stages
- `LexerStage`, `ParserStage`, `CompilerStage`, `VMStage` — Per-stage output
- `astToDict()` — Converts AST nodes to JSON-serializable dictionaries
- `instructionToText()` — Converts bytecode instructions to human-readable text
- `BytecodeCompiler` — Compiles AST to bytecode
- `VirtualMachine` — Executes bytecode
- `OpCode`, `CodeObject`, `Instruction`, `VMTrace` — VM types

## Testing

```bash
npm test                    # Run tests
npm run test:coverage       # Run with coverage
```

## Dependencies

- `@coding-adventures/lexer` — Tokenizer
- `@coding-adventures/parser` — AST construction
