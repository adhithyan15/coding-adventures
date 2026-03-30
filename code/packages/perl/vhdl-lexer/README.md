# CodingAdventures::VhdlLexer

A Perl module that tokenizes VHDL (IEEE 1076-2008) source code using the
grammar-driven lexer infrastructure from the coding-adventures monorepo.

## What is VHDL?

VHDL (VHSIC Hardware Description Language) was designed by the US Department
of Defense. Where Verilog is terse and C-like, VHDL is verbose and Ada-like:
strongly typed, explicitly declared, and — uniquely among HDLs — case-insensitive.
`ENTITY`, `Entity`, and `entity` are all the same identifier.

A VHDL design separates interface from implementation: the `entity` declares
the ports, and the `architecture` describes the logic. Concurrent statements
describe hardware that runs in parallel; `process` blocks provide sequential
control flow.

## How it fits in the stack

```
vhdl.tokens          (grammar definition)
      ↓
GrammarTools         (parse_token_grammar)
      ↓
VhdlLexer            (this module — thin wrapper)
      ↓
Token array          [{ type=>"ENTITY", value=>"entity", line=>1, col=>1 }, ...]
```

All token values are **lowercase** because `vhdl.tokens` sets `case_sensitive: false`.

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
use CodingAdventures::VhdlLexer;

my $tokens = CodingAdventures::VhdlLexer->tokenize('entity adder is port (a : in std_logic);');
for my $tok (@$tokens) {
    printf "%s  %s\n", $tok->{type}, $tok->{value};
}
# ENTITY    entity
# NAME      adder
# IS        is
# PORT      port
# ...
# EOF
```

## Token types

Key types: `ENTITY`, `ARCHITECTURE`, `IS`, `OF`, `BEGIN`, `END`, `PORT`,
`GENERIC`, `COMPONENT`, `PACKAGE`, `USE`, `LIBRARY`; `SIGNAL`, `VARIABLE`,
`CONSTANT`, `TYPE`, `SUBTYPE`, `IN`, `OUT`, `INOUT`, `BUFFER`; `IF`, `ELSIF`,
`ELSE`, `THEN`, `CASE`, `WHEN`, `OTHERS`, `FOR`, `WHILE`, `LOOP`, `PROCESS`,
`WAIT`; `AND`, `OR`, `NOT`, `NAND`, `NOR`, `XOR`, `XNOR`; `LESS_EQUALS`,
`VAR_ASSIGN`, `ARROW`, `NOT_EQUALS`, `POWER`, `GREATER_EQUALS`; `BIT_STRING`,
`CHAR_LITERAL`, `NUMBER`, `STRING`, `NAME`.

## Running tests

```sh
prove -l -v t/
```

## API

### `CodingAdventures::VhdlLexer->tokenize($source)`

Tokenize a VHDL string. Returns an arrayref of hashrefs, each with:
- `type`  — token type string
- `value` — matched text, **lowercased** (due to `case_sensitive: false`)
- `line`  — 1-based line number
- `col`   — 1-based column number

The last element always has `type => 'EOF'`. Dies on unexpected input.

## Version

0.01
