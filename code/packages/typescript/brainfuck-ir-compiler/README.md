# @coding-adventures/brainfuck-ir-compiler

Brainfuck AOT compiler frontend — translates Brainfuck ASTs into general-purpose IR. TypeScript port of `code/packages/go/brainfuck-ir-compiler`.

## What this does

This package is the Brainfuck-specific frontend of the AOT compiler pipeline. It knows Brainfuck semantics — tape, cells, pointer arithmetic, loops, I/O — and translates them into target-independent IR instructions.

## Usage

```typescript
import { parseBrainfuck } from "@coding-adventures/brainfuck";
import {
  compile,
  releaseConfig,
  debugConfig,
} from "@coding-adventures/brainfuck-ir-compiler";
import { printIr } from "@coding-adventures/compiler-ir";

const ast = parseBrainfuck("++[-].");

// Release build: no bounds checks, byte masking on
const { program, sourceMap } = compile(ast, "hello.bf", releaseConfig());

// Debug build: bounds checks on, source locs on
const { program: dbgProg } = compile(ast, "hello.bf", debugConfig());

// Print the IR to text
console.log(printIr(program));
```

## BuildConfig

```typescript
const config = {
  insertBoundsChecks: false,  // emit tape pointer range checks
  insertDebugLocs: false,     // emit COMMENT source location markers
  maskByteArithmetic: true,   // AND 0xFF after every cell mutation
  tapeSize: 30000,            // number of tape cells
};
```

Presets:
- `releaseConfig()` — bounds checks off, debug locs off, masking on
- `debugConfig()` — all checks on

## Command → IR mapping

| Command | IR Output |
|---------|-----------|
| `>` | ADD_IMM v1, v1, 1 |
| `<` | ADD_IMM v1, v1, -1 |
| `+` | LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, 1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1 |
| `-` | LOAD_BYTE v2, v0, v1; ADD_IMM v2, v2, -1; AND_IMM v2, v2, 255; STORE_BYTE v2, v0, v1 |
| `.` | LOAD_BYTE v2, v0, v1; ADD_IMM v4, v2, 0; SYSCALL 1 |
| `,` | SYSCALL 2; STORE_BYTE v4, v0, v1 |

## Stack position

Layer 6 — Compiler Frontend. Depends on brainfuck (parser), compiler-ir (IR types), compiler-source-map (source maps). Produces output consumed by the optimizer and backend.
