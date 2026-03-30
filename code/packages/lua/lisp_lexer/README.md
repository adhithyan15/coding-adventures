# coding-adventures-lisp-lexer

Lua package — Lisp/Scheme tokenizer using the grammar-driven infrastructure.

## What it does

Converts Lisp source text into a flat stream of typed tokens:

```
(define x 42)
→ LPAREN SYMBOL("define") SYMBOL("x") NUMBER("42") RPAREN EOF
```

Powered by the shared `lisp.tokens` grammar file and the `GrammarLexer` from
`coding-adventures-lexer`.  Whitespace and line comments (`;...`) are silently
consumed.

## Token types

| Type     | Pattern                                           | Example      |
|----------|---------------------------------------------------|--------------|
| NUMBER   | `/-?[0-9]+/`                                      | `42`, `-7`   |
| SYMBOL   | `/[a-zA-Z_+\-*\/=<>!?&][a-zA-Z0-9_+\-*\/=<>!?&]*/` | `define`, `+`, `null?` |
| STRING   | `/"([^"\\]|\\.)*"/`                               | `"hello"`    |
| LPAREN   | `(`                                               | `(`          |
| RPAREN   | `)`                                               | `)`          |
| QUOTE    | `'`                                               | `'`          |
| DOT      | `.`                                               | `.`          |
| EOF      | (sentinel)                                        | (end)        |

## Usage

```lua
local lisp_lexer = require("coding_adventures.lisp_lexer")

local tokens = lisp_lexer.tokenize("(+ 1 2)")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
-- LPAREN    (    1  1
-- SYMBOL    +    1  2
-- NUMBER    1    1  4
-- NUMBER    2    1  6
-- RPAREN    )    1  7
-- EOF            1  8
```

## How it fits in the stack

```
lisp.tokens  (grammar file)
     ↓
lisp_lexer   (this package)
     ↓
lisp_parser  (builds ASTs from the token stream)
```

## Dependencies

- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-grammar-tools` — parses the `.tokens` file
- `coding-adventures-directed-graph` (transitive)
- `coding-adventures-state-machine` (transitive)
