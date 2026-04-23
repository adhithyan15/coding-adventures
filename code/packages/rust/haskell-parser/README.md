# coding-adventures-haskell-parser

A Haskell parser for the coding-adventures project. This crate parses Haskell source code into an Abstract Syntax Tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

This crate loads the `haskell{version}.grammar` file and feeds it, along with tokens from the `haskell-lexer` crate, to the generic `GrammarParser`. The grammar file defines Haskell's syntactic structure in a declarative EBNF format.

## How it fits in the stack

```
haskell{version}.tokens  (grammar file)
       |
       v
haskell-lexer            (tokenizes Haskell source -> Vec<Token>)
       |
       v
haskell{version}.grammar (grammar file)
       |
       v
parser                (GrammarParser: builds AST from tokens + grammar)
       |
       v
haskell-parser           (THIS CRATE: wires everything together for Haskell)
```

## Usage

```rust
use coding_adventures_haskell_parser::{create_haskell_parser, parse_haskell};

// Quick parsing — returns a GrammarASTNode
let ast = parse_haskell("class Hello { }", "21").unwrap();
assert_eq!(ast.rule_name, "program");

// Or get the parser object for more control
let mut parser = create_haskell_parser("int x = 1 + 2;", "21").unwrap();
let ast = parser.parse().expect("parse failed");
```

## Supported Haskell versions

| Version | Grammar files |
|---------|--------------|
| `"1.0"` | `grammars/haskell/haskell1.0.{tokens,grammar}` |
| `"1.1"` | `grammars/haskell/haskell1.1.{tokens,grammar}` |
| `"1.4"` | `grammars/haskell/haskell1.4.{tokens,grammar}` |
| `"5"` | `grammars/haskell/haskell5.{tokens,grammar}` |
| `"7"` | `grammars/haskell/haskell7.{tokens,grammar}` |
| `"8"` | `grammars/haskell/haskell8.{tokens,grammar}` |
| `"10"` | `grammars/haskell/haskell10.{tokens,grammar}` |
| `"14"` | `grammars/haskell/haskell14.{tokens,grammar}` |
| `"17"` | `grammars/haskell/haskell17.{tokens,grammar}` |
| `"21"` (default) | `grammars/haskell/haskell21.{tokens,grammar}` |

## Grammar rules

The Haskell grammar covers:

- **program** — the top-level rule, a sequence of statements
- **statement** — variable declarations, expression statements, if/else, while, for, return, class declarations
- **expression** — arithmetic, comparison, logical, assignment, method calls, member access
- **class_declaration** — class definitions with access modifiers and body
- **var_declaration** — variable declarations with type annotations and initializers
