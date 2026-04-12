# coding-adventures-java-lexer

A Java lexer for the coding-adventures project. This crate tokenizes Java source code using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads the appropriate `java{version}.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of Java's tokens — keywords, identifiers, numbers, strings, operators, and delimiters — in a declarative format.

## How it fits in the stack

```
java{version}.tokens  (grammar file)
       |
       v
grammar-tools         (parses .tokens into TokenGrammar)
       |
       v
lexer                 (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
java-lexer            (THIS CRATE: wires grammar + lexer together for Java)
       |
       v
java-parser           (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_java_lexer::{create_java_lexer, tokenize_java};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_java("class Hello { }", "21").unwrap();

// Or get the lexer object for more control
let mut lexer = create_java_lexer("public static void main(String[] args) { }", "21").unwrap();
let tokens = lexer.tokenize().expect("tokenization failed");

// Use a specific Java version
let tokens_8 = tokenize_java("int x = 42;", "8").unwrap();
```

## Token types

The Java lexer produces these token categories:

- **NAME** — identifiers like `x`, `MyClass`, `_private`
- **KEYWORD** — reserved words: `class`, `public`, `static`, `void`, `int`, `if`, `else`, `return`, etc.
- **NUMBER** — numeric literals (integers and floats)
- **STRING** — string literals (double-quoted)
- **Operators** — `+`, `-`, `*`, `/`, `==`, `!=`, `>=`, `<=`, `&&`, `||`, etc.
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `,`, `;`, `.`, `:`
- **EOF** — end of file

## Supported Java versions

| Version | Grammar file |
|---------|-------------|
| `"1.0"` | `grammars/java/java1.0.tokens` |
| `"1.1"` | `grammars/java/java1.1.tokens` |
| `"1.4"` | `grammars/java/java1.4.tokens` |
| `"5"` | `grammars/java/java5.tokens` |
| `"7"` | `grammars/java/java7.tokens` |
| `"8"` | `grammars/java/java8.tokens` |
| `"10"` | `grammars/java/java10.tokens` |
| `"14"` | `grammars/java/java14.tokens` |
| `"17"` | `grammars/java/java17.tokens` |
| `"21"` (default) | `grammars/java/java21.tokens` |
