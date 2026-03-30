# CodingAdventures::LatticeParser

Hand-written recursive-descent parser for the Lattice CSS superset language.
Consumes a token stream from `CodingAdventures::LatticeLexer` and produces
a concrete syntax tree (AST).

## What is Lattice?

Lattice extends CSS3 with the following features:

| Feature | Example |
|---------|---------|
| Variables | `$primary: #4a90d9;` |
| Mixins | `@mixin flex { display: flex; }` |
| @include | `@include flex;` |
| @if / @else | `@if $dark { background: #1a1a1a; }` |
| @for | `@for $i from 1 through 12 { .col-#{$i} { ... } }` |
| @each | `@each $c in red, green { .text-#{$c} { color: $c; } }` |
| @function | `@function spacing($n) { @return $n * 8px; }` |
| @use | `@use "colors";` |
| Nesting | `.nav { .item { color: white; } }` |
| Placeholder | `%flex-center { display: flex; }` |
| @extend | `.hero { @extend %flex-center; }` |
| @content | `@mixin respond($bp) { @media ($bp) { @content; } }` |
| @at-root | `.parent { @at-root .child { color: red; } }` |
| Map literals | `$colors: (primary: #4a90d9, secondary: #e74c3c);` |

Every valid CSS file is also valid Lattice.

## Usage

```perl
use CodingAdventures::LatticeParser;

my $ast = CodingAdventures::LatticeParser->parse(<<'LATTICE');
$primary: #4a90d9;

@mixin center {
  display: flex;
  align-items: center;
}

.hero {
  @include center;
  color: $primary;
}
LATTICE

print $ast->rule_name;  # "stylesheet"
```

## Architecture

This is a hand-written recursive-descent parser.  Unlike the Lua `lattice_parser`
which delegates to the grammar-driven `GrammarParser`, this Perl implementation
encodes each grammar production as a dedicated `_parse_RULENAME` subroutine.

Perl's `CodingAdventures::GrammarTools` only provides `parse_token_grammar`
(for lexer grammars), not `parse_parser_grammar`, so a grammar-driven approach
is not available in the Perl layer.

```
lattice source  →  CodingAdventures::LatticeLexer  →  token array
                                                              │
                                              LatticeParser (this module)
                                           (recursive descent, hand-written)
                                                              │
                                                   ASTNode tree
```

## Stack position

```
CodingAdventures::LatticeParser        ← this package
CodingAdventures::LatticeLexer
CodingAdventures::Lexer / GrammarTools
CodingAdventures::StateMachine / DirectedGraph
```

## Installation

```bash
cpanm --notest .
```

## Tests

```bash
prove -l -v t/
```
