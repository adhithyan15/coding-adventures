# CodingAdventures::VerilogLexer

A Perl module that tokenizes Verilog (IEEE 1364-2005) source code using the
grammar-driven lexer infrastructure from the coding-adventures monorepo.

## What is Verilog?

Verilog is a Hardware Description Language (HDL). Unlike software languages
that describe sequential computations on a processor, Verilog describes
physical structures — gates, wires, flip-flops — that exist simultaneously
and operate in parallel. A Verilog module is a blueprint for a hardware
component with named ports (inputs and outputs) and internal logic.

## How it fits in the stack

```
verilog.tokens       (grammar definition)
      ↓
GrammarTools         (parse_token_grammar)
      ↓
VerilogLexer         (this module — thin wrapper)
      ↓
Token array          [{ type=>"MODULE", value=>"module", line=>1, col=>1 }, ...]
```

The module reads `code/grammars/verilog.tokens` once (cached) and compiles
the token definitions to Perl `qr/\G.../` patterns.

## Installation

```sh
cpanm --installdeps .
```

Dependencies (install first):
- `CodingAdventures::StateMachine`
- `CodingAdventures::DirectedGraph`
- `CodingAdventures::GrammarTools`
- `CodingAdventures::Lexer`

## Usage

```perl
use CodingAdventures::VerilogLexer;

my $tokens = CodingAdventures::VerilogLexer->tokenize('module adder(input a, output y);');
for my $tok (@$tokens) {
    printf "%s  %s\n", $tok->{type}, $tok->{value};
}
# MODULE    module
# NAME      adder
# LPAREN    (
# INPUT     input
# NAME      a
# ...
# EOF
```

## Token types

See `lib/CodingAdventures/VerilogLexer.pm` POD for the full list.
Key types: `MODULE`, `ENDMODULE`, `INPUT`, `OUTPUT`, `INOUT`, `WIRE`, `REG`,
`PARAMETER`, `LOCALPARAM`, `ALWAYS`, `INITIAL`, `BEGIN`, `END`, `IF`, `ELSE`,
`CASE`, `CASEX`, `CASEZ`, `ENDCASE`, `FOR`, `AND`, `OR`, `NOT`, `NAND`, `NOR`,
`XOR`, `XNOR`, `BUF`, `POSEDGE`, `NEGEDGE`; `SIZED_NUMBER`, `NUMBER`, `STRING`,
`SYSTEM_ID`, `DIRECTIVE`, `NAME`; operators and delimiters.

## Running tests

```sh
prove -l -v t/
```

## API

### `CodingAdventures::VerilogLexer->tokenize($source)`

Tokenize a Verilog string. Returns an arrayref of hashrefs, each with:
- `type`  — token type string
- `value` — matched text as it appeared in the source
- `line`  — 1-based line number
- `col`   — 1-based column number

The last element always has `type => 'EOF'`. Dies on unexpected input.

## Version

0.01
