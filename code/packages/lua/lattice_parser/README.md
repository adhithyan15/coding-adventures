# coding-adventures-lattice-parser

Grammar-driven parser for the Lattice CSS superset language. Consumes a
token stream from `lattice_lexer` and produces a concrete syntax tree (AST)
using the shared `parser` and `grammar_tools` packages.

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

Every valid CSS file is also valid Lattice.

## Usage

```lua
local lattice_parser = require("coding_adventures.lattice_parser")

-- Parse a Lattice source string
local ast = lattice_parser.parse([[
$primary: #4a90d9;

@mixin center {
  display: flex;
  align-items: center;
}

.hero {
  @include center;
  color: $primary;
}
]])
-- ast.rule_name == "stylesheet"

-- Inspect the grammar
local grammar = lattice_parser.get_grammar()
print(grammar.rules[1].name)  -- "stylesheet"

-- Create a parser without running it
local p = lattice_parser.create_parser("h1 { color: red; }")
local ast2, err = p:parse()
```

## Architecture

The parser is grammar-driven: it reads `code/grammars/lattice.grammar` and
delegates to the generic `GrammarParser` from the `parser` package. This
means the parser automatically stays in sync when the grammar is updated.

```
lattice source  →  lattice_lexer  →  token stream
                                           │
                               parser.GrammarParser
                                           │
                              lattice.grammar (loaded once)
                                           │
                                    ASTNode tree
```

## Stack position

```
lattice_parser        ← this package
lattice_lexer
lexer / grammar_tools
state_machine / directed_graph
```

## Installation

```bash
luarocks make --local coding-adventures-lattice-parser-0.1.0-1.rockspec
```

## Tests

```bash
cd tests && busted . --verbose --pattern=test_
```
