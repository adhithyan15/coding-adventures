# coding-adventures-csharp-lexer

A C# lexer for the coding-adventures project. This crate tokenizes C# source code using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads the appropriate `csharp{version}.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of C#'s tokens — keywords, identifiers, numbers, strings, operators, and delimiters — in a declarative format.

## How it fits in the stack

```
csharp{version}.tokens  (grammar file)
       |
       v
grammar-tools            (parses .tokens into TokenGrammar)
       |
       v
lexer                    (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
csharp-lexer             (THIS CRATE: wires grammar + lexer together for C#)
       |
       v
csharp-parser            (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_csharp_lexer::{create_csharp_lexer, tokenize_csharp};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_csharp("class Hello { }", "12.0").unwrap();

// Or get the lexer object for more control
let mut lexer = create_csharp_lexer("public static void Main(string[] args) { }", "12.0").unwrap();
let tokens = lexer.tokenize().expect("tokenization failed");

// Use a specific C# version
let tokens_8 = tokenize_csharp("int x = 42;", "8.0").unwrap();
```

## Token types

The C# lexer produces these token categories:

- **NAME** — identifiers like `x`, `MyClass`, `_private`
- **KEYWORD** — reserved words: `class`, `public`, `static`, `void`, `int`, `string`, `bool`, `namespace`, `using`, `if`, `else`, `return`, `new`, `this`, `true`, `false`, `null`, etc.
- **NUMBER** — numeric literals (integers and floats)
- **STRING** — string literals (double-quoted)
- **Operators** — `+`, `-`, `*`, `/`, `==`, `!=`, `>=`, `<=`, `&&`, `||`, `??`, `?.`, `=>`, etc.
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `,`, `;`, `.`, `:`
- **EOF** — end of file

## Supported C# versions

| Version | Grammar file | .NET era |
|---------|-------------|----------|
| `"1.0"` | `grammars/csharp/csharp1.0.tokens` | .NET Framework 1.0 (2002) |
| `"2.0"` | `grammars/csharp/csharp2.0.tokens` | .NET Framework 2.0 (2005) — generics |
| `"3.0"` | `grammars/csharp/csharp3.0.tokens` | .NET Framework 3.5 (2007) — LINQ |
| `"4.0"` | `grammars/csharp/csharp4.0.tokens` | .NET Framework 4.0 (2010) — dynamic |
| `"5.0"` | `grammars/csharp/csharp5.0.tokens` | .NET Framework 4.5 (2012) — async/await |
| `"6.0"` | `grammars/csharp/csharp6.0.tokens` | .NET Framework 4.6 (2015) — string interpolation |
| `"7.0"` | `grammars/csharp/csharp7.0.tokens` | .NET Framework 4.7 (2017) — tuples, patterns |
| `"8.0"` | `grammars/csharp/csharp8.0.tokens` | .NET Core 3.0 (2019) — nullable refs |
| `"9.0"` | `grammars/csharp/csharp9.0.tokens` | .NET 5 (2020) — records, top-level statements |
| `"10.0"` | `grammars/csharp/csharp10.0.tokens` | .NET 6 (2021) — global using, file-scoped namespaces |
| `"11.0"` | `grammars/csharp/csharp11.0.tokens` | .NET 7 (2022) — raw strings, generic math |
| `"12.0"` (default) | `grammars/csharp/csharp12.0.tokens` | .NET 8 (2023) — primary constructors, collection expressions |
