# coding-adventures-verilog-parser

Grammar-driven Verilog parser for the coding-adventures monorepo.

## What it does

This package parses Verilog (IEEE 1364-2005) source code into an Abstract
Syntax Tree (AST) using the grammar-driven `GrammarParser` engine. It covers
the **synthesizable subset** — constructs that map to real digital hardware.

Verilog is a Hardware Description Language. A Verilog "program" describes
circuits — modules with ports (inputs/outputs), wires, registers, and logic.
Unlike software, all concurrent statements execute simultaneously, modelling
the parallel operation of actual hardware.

## Usage

```lua
local verilog_parser = require("coding_adventures.verilog_parser")

-- Parse a simple AND gate module
local ast = verilog_parser.parse([[
  module and_gate(input a, input b, output y);
    assign y = a & b;
  endmodule
]])
print(ast.rule_name)  -- "source_text"

-- Create a parser without parsing (for trace/inspection)
local p = verilog_parser.create_parser("module empty; endmodule")
local ast, err = p:parse()

-- Inspect the grammar
local g = verilog_parser.get_grammar()
print(g.rules[1].name)  -- "source_text"
```

## Grammar highlights

The grammar covers:

- **Module declarations** — `module NAME [#(params)] [(ports)]; … endmodule`
- **Port declarations** — `input [7:0] data`, `output reg y`
- **Wire/reg declarations** — `wire [31:0] bus`, `reg [7:0] counter`
- **Continuous assignments** — `assign y = a & b` (combinational logic)
- **Always blocks** — `always @(posedge clk) begin … end` (sequential logic)
- **Sensitivity lists** — `@(posedge clk or negedge rst)`, `@(*)`
- **If/else, case/casex/casez** — behavioral branching
- **For loops** — primarily for generate-time unrolling
- **Module instantiation** — `adder #(.WIDTH(8)) u1 (.a(x), .b(y));`
- **Generate blocks** — parameterized hardware replication
- **Functions and tasks** — reusable behavioral blocks
- **Full expression hierarchy** — ternary, logical, bitwise, shift, arithmetic

## Two Verilog design styles

**Structural** (connect modules like wiring chips):
```verilog
and_gate u1 (.a(sig_a), .b(sig_b), .y(out));
```

**Behavioral** (describe what the hardware does):
```verilog
always @(posedge clk)
  if (reset) q <= 0;
  else q <= d;
```

## Stack position

```
verilog_parser    ← this package
     ↓
   parser          — GrammarParser engine
     ↓
grammar_tools      — parse .grammar file into ParserGrammar
     ↓
verilog_lexer      — tokenize Verilog source
     ↓
     lexer         — grammar-driven lexer engine
```

## Building and testing

```bash
cd code/packages/lua/verilog_parser
cat BUILD | bash
```

## Version

0.1.0
