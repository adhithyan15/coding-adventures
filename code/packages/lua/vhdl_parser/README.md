# coding-adventures-vhdl-parser

Grammar-driven VHDL parser for the coding-adventures monorepo.

## What it does

This package parses VHDL (IEEE 1076-2008) source code into an Abstract Syntax
Tree (AST) using the grammar-driven `GrammarParser` engine. It covers the
**synthesizable subset** — constructs that map to real digital hardware.

VHDL is a Hardware Description Language with strong typing and explicit
declarations. Unlike Verilog (which is C-like), VHDL is Ada-like: verbose,
but catches many errors at compile time. The key VHDL concepts are:

- **Entity**: the interface (ports/generics) — like a chip's pin diagram
- **Architecture**: the implementation — behavioral or structural description
- **Process**: sequential region inside the concurrent world
- **Signal**: physical wire or register (assignment takes effect after delta)
- **Variable**: local to a process, assignment is immediate

## Usage

```lua
local vhdl_parser = require("coding_adventures.vhdl_parser")

-- Parse an entity + architecture
local ast = vhdl_parser.parse([[
  entity and_gate is
    port (a, b : in std_logic; y : out std_logic);
  end entity and_gate;

  architecture rtl of and_gate is
  begin
    y <= a and b;
  end architecture rtl;
]])
print(ast.rule_name)  -- "design_file"

-- Create a parser without parsing (for trace/inspection)
local p = vhdl_parser.create_parser("entity empty is end entity;")
local ast, err = p:parse()

-- Inspect the grammar
local g = vhdl_parser.get_grammar()
print(g.rules[1].name)  -- "design_file"
```

## Grammar highlights

The grammar covers:

- **Design files** — multiple design units with context clauses
- **Context clauses** — `library IEEE; use IEEE.std_logic_1164.all;`
- **Entity declarations** — ports, generics, port modes (in/out/inout/buffer)
- **Architecture bodies** — signal declarations, concurrent statements
- **Concurrent statements** — processes, signal assignments, component instantiation, generate
- **Process statements** — sequential logic with sensitivity lists
- **Sequential statements** — `if/elsif/else`, `case/when`, `for/while loop`, `return`, `null`
- **Signal and variable assignments** — `<=` (signal) and `:=` (variable)
- **Type system** — enumeration, array, record types; subtype indications
- **Component declarations and instantiations** — structural connectivity
- **Generate statements** — for-generate and if-generate
- **Package declarations and bodies** — library packages
- **Functions and procedures** — reusable behavioral blocks
- **Full expression hierarchy** — logical (and/or/xor/nand/nor/xnor), relational,
  shift (sll/srl/sla/sra/rol/ror), adding (+/-/&), multiplying (*//, mod, rem),
  unary (abs/not), power (**)

## VHDL peculiarities

```
-- Concurrent signal assignment (always active, like Verilog 'assign'):
y <= a and b;

-- Process (sequential, like Verilog 'always'):
process (clk)
begin
  if rising_edge(clk) then
    q <= d;
  end if;
end process;

-- Logical operators are KEYWORDS, not symbols:
y <= (a and b) or (c xor d);   -- NOT y <= (a & b) | (c ^ d)
```

## Stack position

```
vhdl_parser      ← this package
     ↓
   parser         — GrammarParser engine
     ↓
grammar_tools     — parse .grammar file into ParserGrammar
     ↓
vhdl_lexer        — tokenize VHDL source (case-insensitive)
     ↓
     lexer        — grammar-driven lexer engine
```

## Building and testing

```bash
cd code/packages/lua/vhdl_parser
cat BUILD | bash
```

## Version

0.1.0
