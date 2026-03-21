# starlark-vm

Executes compiled Starlark bytecode on the pluggable GenericVM.

## Architecture

This crate ties everything together:

1. All ~50 opcodes have registered handlers
2. All ~25 built-in functions are registered
3. Starlark-specific restrictions are configured (recursion limits, etc.)

## Usage

```rust
use starlark_vm::*;

// Quick start -- one call does everything:
let result = execute_starlark("x = 1 + 2\nprint(x)\n");
assert_eq!(result.variables["x"], StarlarkValue::Int(3));
assert_eq!(result.output, vec!["3"]);
```

## Built-in Functions

The VM includes all standard Starlark built-in functions:

- Type functions: type, bool, int, float, str
- Collection functions: len, list, dict, tuple, range, sorted, reversed, enumerate, zip
- Logic and math: min, max, abs, all, any
- String/utility: repr, hasattr, getattr
- I/O: print

## Dependencies

- `starlark-compiler` -- opcode definitions
- `virtual-machine` -- generic VM framework
- `bytecode-compiler` -- compilation framework
- `starlark-lexer` -- tokenization
- `starlark-parser` -- parsing
