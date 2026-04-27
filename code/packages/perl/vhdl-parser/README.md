# CodingAdventures::VhdlParser

Hand-written recursive-descent VHDL parser for the coding-adventures monorepo.

The parser core is still hand-written for historical reasons: this package was
implemented before the generic grammar-driven Perl parser stack landed, and it
has not been migrated over yet. It now accepts versioned lexer frontends
(`1987`, `1993`, `2002`, `2008`, `2019`) so the public API stays aligned with
the shared HDL model.

## What it does

This module parses the synthesizable subset of VHDL (IEEE 1076-2008) into an
Abstract Syntax Tree (AST) using the recursive-descent technique.

VHDL (VHSIC Hardware Description Language) is used to describe digital
circuits. Unlike Verilog's module-centric model, VHDL separates the external
interface (entity) from the internal implementation (architecture). This
separation enables multiple architectures for a single entity — for example,
a behavioral simulation model and a gate-level structural model can both
implement the same entity interface.

## Usage

```perl
use CodingAdventures::VhdlParser;

# Object-oriented
my $parser = CodingAdventures::VhdlParser->new(<<'VHDL');
entity half_adder is
  port (a, b : in std_logic; sum, carry : out std_logic);
end entity half_adder;

architecture rtl of half_adder is
begin
  sum   <= a xor b;
  carry <= a and b;
end architecture rtl;
VHDL
my $ast = $parser->parse();
print $ast->rule_name;   # "design_file"

# Convenience class method
my $ast = CodingAdventures::VhdlParser->parse_vhdl("entity empty is end entity;");
```

## Supported constructs

- Library and use clauses (context items)
- Entity declarations with generics and ports
- Architecture bodies with declarative regions and concurrent statements
- Signal, constant, variable, and type declarations
- Enumeration, array, and record type definitions
- Component declarations
- Concurrent signal assignments (with waveforms and `after` delays)
- Component instantiations (named and positional port maps)
- Process statements with sensitivity lists
- Generate statements (for-generate and if-generate)
- Sequential statements: signal assignment (`<=`), variable assignment (`:=`),
  if/elsif/else, case/when, for loops, return, null
- Package declarations and package bodies
- Function and procedure declarations
- Full expression grammar with correct VHDL operator precedence

## VHDL vs Verilog

| Feature              | VHDL                      | Verilog              |
|----------------------|---------------------------|----------------------|
| Interface            | `entity` (separate)       | `module` (combined)  |
| Implementation       | `architecture`            | `module` body        |
| Signal assignment    | `<=` (concurrent/seq)     | `<=` (non-blocking)  |
| Variable assignment  | `:=`                      | `=` (blocking)       |
| Case-sensitivity     | Case-insensitive           | Case-sensitive       |
| Concatenation        | `&`                       | `{a, b}`             |
| Logical operators    | Words: `and`, `or`, `xor` | Symbols: `&&`, `||`  |

## Building and testing

```bash
cd code/packages/perl/vhdl-parser
cat BUILD | bash
```

## Version

0.01
