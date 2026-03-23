# VHDL Parser (TypeScript)

Parses VHDL (IEEE 1076-2008) source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It loads `vhdl.grammar` and delegates all parsing to the generic engine.

VHDL (VHSIC Hardware Description Language) is a Hardware Description Language for designing digital circuits. Unlike Verilog, which is terse and C-like, VHDL is verbose and Ada-like, with strong typing, explicit declarations, and case-insensitive identifiers. The interface (entity) is separate from the implementation (architecture).

## Usage

```typescript
import { parseVhdl } from "@coding-adventures/vhdl-parser";

// Parse a simple AND gate:
const ast = parseVhdl(`
  entity and_gate is
    port (a, b : in std_logic; y : out std_logic);
  end entity and_gate;

  architecture rtl of and_gate is
  begin
    y <= a and b;
  end architecture rtl;
`);
console.log(ast.ruleName); // "design_file"
```

## Supported Constructs

- **Entity declarations** with ports and generics
- **Architecture bodies** with signal, constant, and type declarations
- **Concurrent signal assignments** (`y <= a and b;`)
- **Process statements** with sensitivity lists and variable declarations
- **If/elsif/else** statements (no dangling else ambiguity)
- **Case/when** statements with `when others` coverage
- **Component instantiation** with named port maps
- **Library and use clauses** (`library IEEE; use IEEE.std_logic_1164.all;`)
- **Type declarations** — enumerations, arrays, records
- **Full expression grammar** with operator precedence (logical, relational, shift, adding, multiplying, unary, power)
- **Generate statements** (for-generate and if-generate)
- **Package declarations** and package bodies
- **Function and procedure** declarations and bodies

## VHDL vs Verilog

| VHDL              | Verilog            |
|-------------------|--------------------|
| entity            | module (interface) |
| architecture      | module (body)      |
| signal            | wire/reg           |
| variable          | (inside process)   |
| process           | always block       |
| port map          | instance ports     |
| generic           | parameter          |
| `<=`              | `<=` (non-blocking)|
| `:=`              | `=` (blocking)     |

## Dependencies

- `@coding-adventures/vhdl-lexer` -- tokenizes VHDL source code (with case normalization)
- `@coding-adventures/parser` -- provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` -- parses `.grammar` files
- `@coding-adventures/lexer` -- token types
- `@coding-adventures/directed-graph` -- transitive dependency
- `@coding-adventures/state-machine` -- transitive dependency
