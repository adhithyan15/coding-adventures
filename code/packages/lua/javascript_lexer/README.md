# coding-adventures-javascript-lexer (Lua)

A JavaScript lexer that tokenizes JavaScript source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `javascript.tokens` grammar file.

## What it does

Given the input `const x = 42;`, the lexer produces:

| # | Type      | Value   |
|---|-----------|---------|
| 1 | CONST     | `const` |
| 2 | NAME      | `x`     |
| 3 | EQUALS    | `=`     |
| 4 | NUMBER    | `42`    |
| 5 | SEMICOLON | `;`     |
| 6 | EOF       |         |

Whitespace is silently consumed (declared as a skip pattern in `javascript.tokens`).

## Token types

### Literals
| Token type | Example match |
|------------|---------------|
| NAME       | `myVar`, `$el`, `_priv` |
| NUMBER     | `42`, `0`, `255` |
| STRING     | `"hello"`, `"a\nb"` |

### Keywords
| Token type  | Keyword      |
|-------------|--------------|
| LET         | `let`        |
| CONST       | `const`      |
| VAR         | `var`        |
| IF          | `if`         |
| ELSE        | `else`       |
| WHILE       | `while`      |
| FOR         | `for`        |
| DO          | `do`         |
| FUNCTION    | `function`   |
| RETURN      | `return`     |
| CLASS       | `class`      |
| IMPORT      | `import`     |
| EXPORT      | `export`     |
| FROM        | `from`       |
| AS          | `as`         |
| NEW         | `new`        |
| THIS        | `this`       |
| TYPEOF      | `typeof`     |
| INSTANCEOF  | `instanceof` |
| TRUE        | `true`       |
| FALSE       | `false`      |
| NULL        | `null`       |
| UNDEFINED   | `undefined`  |

### Operators
| Token type         | Symbol |
|--------------------|--------|
| STRICT_EQUALS      | `===`  |
| STRICT_NOT_EQUALS  | `!==`  |
| EQUALS_EQUALS      | `==`   |
| NOT_EQUALS         | `!=`   |
| LESS_EQUALS        | `<=`   |
| GREATER_EQUALS     | `>=`   |
| ARROW              | `=>`   |
| EQUALS             | `=`    |
| PLUS               | `+`    |
| MINUS              | `-`    |
| STAR               | `*`    |
| SLASH              | `/`    |
| LESS_THAN          | `<`    |
| GREATER_THAN       | `>`    |
| BANG               | `!`    |

### Delimiters
| Token type | Symbol |
|------------|--------|
| LPAREN     | `(`    |
| RPAREN     | `)`    |
| LBRACE     | `{`    |
| RBRACE     | `}`    |
| LBRACKET   | `[`    |
| RBRACKET   | `]`    |
| COMMA      | `,`    |
| COLON      | `:`    |
| SEMICOLON  | `;`    |
| DOT        | `.`    |

## Usage

```lua
local js_lexer = require("coding_adventures.javascript_lexer")

local tokens = js_lexer.tokenize("const x = 1;")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
javascript.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
javascript_lexer  ← you are here
    ↓  feeds
javascript_parser  (future)
```

## Dependencies

- `coding-adventures-grammar-tools` — parses `javascript.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
