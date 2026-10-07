# coding-adventures-javascript-lexer

A JavaScript lexer for the coding-adventures project. This crate tokenizes JavaScript source code using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads the `javascript.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of JavaScript's tokens — keywords, identifiers, numbers, strings, operators, and delimiters — in a declarative format.

## How it fits in the stack

```
javascript.tokens   (grammar file)
       |
       v
grammar-tools       (parses .tokens into TokenGrammar)
       |
       v
lexer               (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
javascript-lexer    (THIS CRATE: wires grammar + lexer together for JavaScript)
       |
       v
javascript-parser   (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_javascript_lexer::{create_javascript_lexer, tokenize_javascript};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_javascript("var x = 42;");

// Or get the lexer object for more control
let mut lexer = create_javascript_lexer("function add(a, b) { return a + b; }");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Token types

The JavaScript lexer produces these token categories:

- **NAME** — identifiers like `x`, `myFunc`, `_private`
- **KEYWORD** — reserved words: `var`, `let`, `const`, `function`, `return`, `if`, `else`, etc.
- **NUMBER** — numeric literals (integers and floats)
- **STRING** — string literals (single-quoted and double-quoted)
- **Operators** — `+`, `-`, `*`, `/`, `===`, `!==`, `=>`, `&&`, `||`, etc.
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `,`, `;`, `.`, `:`
- **EOF** — end of file

## Checked provenance errors (CV02)

CV02: propagate checked token-identity allocation failures as lexer errors, with an exhausted-node-budget regression. The caller must propagate the error and reject the compilation result.

## Exact lexical source intervals (CV04a)

`tokenize_javascript_typed_with_ranges(source, version, LexicalLimits::default())`
selects the same generated ECMAScript grammar and returns the generic lexer's
source-bound snapshot. Raw ranges include quotes, escape spelling, skipped
trivia and zero-width EOF. Existing Token values, locations, CVs and flags match
`tokenize_javascript_typed`; processed string escape policy varies by edition.
UTF8/scalar/UTF16 coordinates belong to the inspected buffer, separate from
legacy Token positions. Capture is bounded and returns distinct malformed-source,
resource and unsupported errors.

A successful partition covers only lexer input. This API creates no CV graph,
serialized inventory, source authentication or output mapping. The compiler must
explicitly bind buffer identity and connect these tokens to existing CLOC27 leaf
IDs in the next slice. Bare U+2028/U+2029 currently reject between ES2025 tokens;
capture preserves that known grammar parity gap.
