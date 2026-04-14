# CodingAdventures::VerilogParser

Hand-written recursive-descent Verilog parser for the coding-adventures monorepo.

The parser core is still hand-written for historical reasons: this package was
implemented before the generic grammar-driven Perl parser stack landed, and it
has not been migrated over yet. It now accepts versioned lexer frontends
(`1995`, `2001`, `2005`) so the public API stays aligned with the shared HDL
model.

## What it does

This module parses the synthesizable subset of Verilog (IEEE 1364-2005) into an
Abstract Syntax Tree (AST) using the recursive-descent technique.

Verilog is a Hardware Description Language. A Verilog "program" describes
circuits, not computations — modules with ports (inputs/outputs), wires,
registers, and behavioral or structural logic descriptions.

## Usage

```perl
use CodingAdventures::VerilogParser;

# Object-oriented
my $parser = CodingAdventures::VerilogParser->new(<<'VERILOG');
module and_gate(input a, input b, output y);
  assign y = a & b;
endmodule
VERILOG
my $ast = $parser->parse();
print $ast->rule_name;   # "source_text"

# Convenience class method
my $ast = CodingAdventures::VerilogParser->parse_verilog("module empty; endmodule");
```

## Supported constructs

- Module declarations with ports and parameters
- Wire/reg/integer declarations with bit widths (`[7:0]`)
- Continuous assignments: `assign y = a & b;`
- Always blocks: `always @(posedge clk) begin … end`
- Initial blocks
- If/else, case/casex/casez statements
- For loops
- Module instantiation (named `.a(sig)` and positional)
- Generate blocks (for-generate, if-generate)
- Functions and tasks
- Full expression grammar with correct Verilog operator precedence

## Building and testing

```bash
cd code/packages/perl/verilog-parser
cat BUILD | bash
```

## Version

0.01
