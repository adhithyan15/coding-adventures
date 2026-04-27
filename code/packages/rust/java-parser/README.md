# coding-adventures-java-parser

A Java parser for the coding-adventures project. This crate parses Java source code into an Abstract Syntax Tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

This crate loads the `java{version}.grammar` file and feeds it, along with tokens from the `java-lexer` crate, to the generic `GrammarParser`. The grammar file defines Java's syntactic structure in a declarative EBNF format.

## How it fits in the stack

```
java{version}.tokens  (grammar file)
       |
       v
java-lexer            (tokenizes Java source -> Vec<Token>)
       |
       v
java{version}.grammar (grammar file)
       |
       v
parser                (GrammarParser: builds AST from tokens + grammar)
       |
       v
java-parser           (THIS CRATE: wires everything together for Java)
```

## Usage

```rust
use coding_adventures_java_parser::{create_java_parser, parse_java};

// Quick parsing — returns a GrammarASTNode
let ast = parse_java("class Hello { }", "21").unwrap();
assert_eq!(ast.rule_name, "program");

// Or get the parser object for more control
let mut parser = create_java_parser("int x = 1 + 2;", "21").unwrap();
let ast = parser.parse().expect("parse failed");
```

## Supported Java versions

| Version | Grammar files |
|---------|--------------|
| `"1.0"` | `grammars/java/java1.0.{tokens,grammar}` |
| `"1.1"` | `grammars/java/java1.1.{tokens,grammar}` |
| `"1.4"` | `grammars/java/java1.4.{tokens,grammar}` |
| `"5"` | `grammars/java/java5.{tokens,grammar}` |
| `"7"` | `grammars/java/java7.{tokens,grammar}` |
| `"8"` | `grammars/java/java8.{tokens,grammar}` |
| `"10"` | `grammars/java/java10.{tokens,grammar}` |
| `"14"` | `grammars/java/java14.{tokens,grammar}` |
| `"17"` | `grammars/java/java17.{tokens,grammar}` |
| `"21"` (default) | `grammars/java/java21.{tokens,grammar}` |

## Grammar rules

The Java grammar covers:

- **program** — the top-level rule, a sequence of statements
- **statement** — variable declarations, expression statements, if/else, while, for, return, class declarations
- **expression** — arithmetic, comparison, logical, assignment, method calls, member access
- **class_declaration** — class definitions with access modifiers and body
- **var_declaration** — variable declarations with type annotations and initializers
