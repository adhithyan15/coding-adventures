# coding-adventures-ecmascript-es5-lexer

An ECMAScript 5 (2009) lexer for the coding-adventures project. This crate tokenizes ES5 JavaScript source code using the grammar-driven lexer from the `lexer` crate.

## How it works

Instead of hand-writing tokenization rules, this crate loads the `es5.tokens` grammar file and feeds it to the generic `GrammarLexer`. The grammar file defines all of ES5's tokens in a declarative format.

## What makes ES5 different from ES3

ES5 landed a full decade after ES3 (ES4 was abandoned). The lexical changes are modest:

- **`debugger` keyword** — promoted from future-reserved (ES3) to a full keyword
- **Getter/setter syntax** — `{ get x() {}, set x(v) {} }` in object literals
- **String line continuation** — backslash before newline continues the string
- **Trailing commas** — allowed in object literals

## How it fits in the stack

```
es5.tokens          (grammar file)
       |
       v
grammar-tools       (parses .tokens into TokenGrammar)
       |
       v
lexer               (GrammarLexer: tokenizes source using TokenGrammar)
       |
       v
ecmascript-es5-lexer (THIS CRATE: wires grammar + lexer together for ES5)
       |
       v
ecmascript-es5-parser (consumes tokens to build AST)
```

## Usage

```rust
use coding_adventures_ecmascript_es5_lexer::{create_es5_lexer, tokenize_es5};

// Quick tokenization — returns a Vec<Token>
let tokens = tokenize_es5("debugger;");

// Or get the lexer object for more control
let mut lexer = create_es5_lexer("var obj = { get x() { return 1; } };");
let tokens = lexer.tokenize().expect("tokenization failed");
```

## Token types

The ES5 lexer produces these token categories:

- **NAME** — identifiers like `x`, `myFunc`, `_private`, `$dollar`
- **KEYWORD** — reserved words: `var`, `function`, `debugger`, `try`, `catch`, `instanceof`, etc.
- **NUMBER** — numeric literals (integers, floats, hex)
- **STRING** — string literals (single-quoted and double-quoted)
- **REGEX** — regular expression literals (`/pattern/flags`)
- **Operators** — `+`, `-`, `*`, `/`, `===`, `!==`, `==`, `!=`, `&&`, `||`, etc.
- **Delimiters** — `(`, `)`, `[`, `]`, `{`, `}`, `,`, `;`, `.`, `:`
- **EOF** — end of file
