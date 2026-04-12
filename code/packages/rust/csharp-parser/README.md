# coding-adventures-csharp-parser

A C# parser for the coding-adventures project. This crate parses C# source code into an Abstract Syntax Tree (AST) using the grammar-driven parser from the `parser` crate.

## How it works

This crate loads the `csharp{version}.grammar` file and feeds it, along with tokens from the `csharp-lexer` crate, to the generic `GrammarParser`. The grammar file defines C#'s syntactic structure in a declarative EBNF format.

## How it fits in the stack

```
csharp{version}.tokens  (grammar file)
       |
       v
csharp-lexer             (tokenizes C# source -> Vec<Token>)
       |
       v
csharp{version}.grammar  (grammar file)
       |
       v
parser                   (GrammarParser: builds AST from tokens + grammar)
       |
       v
csharp-parser            (THIS CRATE: wires everything together for C#)
```

## Usage

```rust
use coding_adventures_csharp_parser::{create_csharp_parser, parse_csharp};

// Quick parsing — returns a GrammarASTNode
let ast = parse_csharp("class Hello { }", "12.0").unwrap();
assert_eq!(ast.rule_name, "program");

// Or get the parser object for more control
let mut parser = create_csharp_parser("int x = 1 + 2;", "12.0").unwrap();
let ast = parser.parse().expect("parse failed");

// Use a specific C# version
let ast_8 = parse_csharp("int x = 42;", "8.0").unwrap();
```

## Supported C# versions

| Version | Grammar files | .NET era |
|---------|--------------|----------|
| `"1.0"` | `grammars/csharp/csharp1.0.{tokens,grammar}` | .NET Framework 1.0 (2002) |
| `"2.0"` | `grammars/csharp/csharp2.0.{tokens,grammar}` | .NET Framework 2.0 (2005) — generics |
| `"3.0"` | `grammars/csharp/csharp3.0.{tokens,grammar}` | .NET Framework 3.5 (2007) — LINQ |
| `"4.0"` | `grammars/csharp/csharp4.0.{tokens,grammar}` | .NET Framework 4.0 (2010) — dynamic |
| `"5.0"` | `grammars/csharp/csharp5.0.{tokens,grammar}` | .NET Framework 4.5 (2012) — async/await |
| `"6.0"` | `grammars/csharp/csharp6.0.{tokens,grammar}` | .NET Framework 4.6 (2015) — string interpolation |
| `"7.0"` | `grammars/csharp/csharp7.0.{tokens,grammar}` | .NET Framework 4.7 (2017) — tuples, patterns |
| `"8.0"` | `grammars/csharp/csharp8.0.{tokens,grammar}` | .NET Core 3.0 (2019) — nullable refs |
| `"9.0"` | `grammars/csharp/csharp9.0.{tokens,grammar}` | .NET 5 (2020) — records, top-level statements |
| `"10.0"` | `grammars/csharp/csharp10.0.{tokens,grammar}` | .NET 6 (2021) — global using, file-scoped namespaces |
| `"11.0"` | `grammars/csharp/csharp11.0.{tokens,grammar}` | .NET 7 (2022) — raw strings, generic math |
| `"12.0"` (default) | `grammars/csharp/csharp12.0.{tokens,grammar}` | .NET 8 (2023) — primary constructors, collection expressions |

## Grammar rules

The C# grammar covers:

- **program** — the top-level rule, a sequence of statements
- **statement** — variable declarations, expression statements, if/else, while, for, foreach, return, class declarations, namespace declarations, using directives
- **expression** — arithmetic, comparison, logical, assignment, method calls, member access, null coalescing (`??`), null conditional (`?.`), lambda (`=>`)
- **class_declaration** — class definitions with access modifiers, base class, interface list, and body
- **var_declaration** — variable declarations with type annotations and optional initializers
