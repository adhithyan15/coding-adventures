# Verilog Parser (TypeScript)

Parses Verilog (IEEE 1364-2005) source code into abstract syntax trees (ASTs) using the grammar-driven parser approach.

## What Is This?

This package is a **thin wrapper** around the generic `GrammarParser` from the `@coding-adventures/parser` package. It loads `verilog.grammar` and delegates all parsing to the generic engine.

Verilog is a Hardware Description Language (HDL) for designing digital circuits. Unlike software languages that describe sequential instructions, Verilog describes physical hardware — gates, wires, registers — that all exist simultaneously.

## Usage

```typescript
import { parseVerilog } from "@coding-adventures/verilog-parser";

// Parse a simple AND gate module:
const ast = parseVerilog(`
  module and_gate(input a, input b, output y);
    assign y = a & b;
  endmodule
`);
console.log(ast.ruleName); // "source_text"
```

## Supported Constructs

- **Module declarations** with ports and parameters
- **Wire/reg/integer declarations** with bit widths
- **Continuous assignments** (`assign y = a & b;`)
- **Always blocks** with sensitivity lists (`always @(posedge clk)`, `always @(*)`)
- **Initial blocks** for simulation
- **If/else** and **case/casex/casez** statements
- **Module instantiation** with positional and named port connections
- **Full expression grammar** with operator precedence (ternary, logical, bitwise, equality, relational, shift, arithmetic, unary)
- **Generate blocks** for parameterized hardware
- **Functions and tasks**

## Dependencies

- `@coding-adventures/verilog-lexer` -- tokenizes Verilog source code (with preprocessor)
- `@coding-adventures/parser` -- provides `GrammarParser` and `ASTNode`
- `@coding-adventures/grammar-tools` -- parses `.grammar` files
- `@coding-adventures/lexer` -- token types
- `@coding-adventures/directed-graph` -- transitive dependency
- `@coding-adventures/state-machine` -- transitive dependency
