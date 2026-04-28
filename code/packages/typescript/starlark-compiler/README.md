# @coding-adventures/starlark-compiler

TypeScript opcode metadata for the Coding Adventures Starlark bytecode compiler.

The package mirrors the Rust `starlark-compiler` opcode contract so parser,
compiler, and VM integrations can agree on instruction bytes and operator
mappings.

## Features

- Stable byte values for every Starlark VM opcode.
- Byte-to-opcode round trips.
- Operator maps for binary, comparison, augmented assignment, and unary
  operators.
- Opcode category helpers based on the high nibble.
