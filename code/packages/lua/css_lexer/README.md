# coding-adventures-css-lexer (Lua)

A CSS lexer that tokenizes CSS3 source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `css.tokens` grammar file.

## What it does

Given the input `h1 { color: red; }`, the lexer produces:

| # | Type      | Value   |
|---|-----------|---------|
| 1 | IDENT     | `h1`    |
| 2 | LBRACE    | `{`     |
| 3 | IDENT     | `color` |
| 4 | COLON     | `:`     |
| 5 | IDENT     | `red`   |
| 6 | SEMICOLON | `;`     |
| 7 | RBRACE    | `}`     |
| 8 | EOF       |         |

Whitespace and `/* ... */` comments are silently consumed (declared as skip patterns in `css.tokens`).

## Why CSS tokenization is harder than most languages

CSS uses *compound tokens* — single lexical units made from multiple character classes:

| Input     | Wrong (two tokens)          | Right (one token) |
|-----------|-----------------------------|-------------------|
| `10px`    | NUMBER(`10`) IDENT(`px`)    | DIMENSION(`10px`) |
| `50%`     | NUMBER(`50`) PERCENT        | PERCENTAGE(`50%`) |
| `rgba(`   | IDENT(`rgba`) LPAREN(`(`)   | FUNCTION(`rgba(`) |
| `url(./x)`| FUNCTION(`url(`) …          | URL_TOKEN(`url(./x)`) |
| `::`      | COLON(`:`) COLON(`:`)       | COLON_COLON(`::`) |

The `css.tokens` grammar uses first-match-wins priority ordering to get this right.

## Token types

### Compound tokens (CSS-unique)
| Token type      | Example match                          |
|-----------------|----------------------------------------|
| DIMENSION       | `10px`, `1.5em`, `100vh`, `360deg`     |
| PERCENTAGE      | `50%`, `100%`, `0.5%`                  |
| AT_KEYWORD      | `@media`, `@import`, `@keyframes`      |
| HASH            | `#333`, `#ff0000`, `#header`           |
| FUNCTION        | `rgba(`, `calc(`, `linear-gradient(`   |
| URL_TOKEN       | `url(./img.png)`, `url(data:...)`      |
| CUSTOM_PROPERTY | `--main-color`, `--bg`                 |

### Simple tokens
| Token type    | Example match                     |
|---------------|-----------------------------------|
| NUMBER        | `42`, `3.14`, `-0.5`              |
| STRING        | `"hello"`, `'world'`              |
| IDENT         | `color`, `sans-serif`, `-webkit-transform` |
| UNICODE_RANGE | `U+0025-00FF`, `U+4??`            |
| CDO / CDC     | `<!--` / `-->`                    |

### Operators and delimiters
| Token type    | Symbol |
|---------------|--------|
| COLON_COLON   | `::`   |
| TILDE_EQUALS  | `~=`   |
| PIPE_EQUALS   | `\|=`  |
| CARET_EQUALS  | `^=`   |
| DOLLAR_EQUALS | `$=`   |
| STAR_EQUALS   | `*=`   |
| LBRACE        | `{`    |
| RBRACE        | `}`    |
| LPAREN        | `(`    |
| RPAREN        | `)`    |
| LBRACKET      | `[`    |
| RBRACKET      | `]`    |
| SEMICOLON     | `;`    |
| COLON         | `:`    |
| COMMA         | `,`    |
| DOT           | `.`    |
| PLUS          | `+`    |
| GREATER       | `>`    |
| TILDE         | `~`    |
| STAR          | `*`    |
| PIPE          | `\|`   |
| BANG          | `!`    |
| SLASH         | `/`    |
| EQUALS        | `=`    |
| AMPERSAND     | `&`    |
| MINUS         | `-`    |

### Error tokens (graceful degradation)
| Token type | Description                              |
|------------|------------------------------------------|
| BAD_STRING | Unclosed string: `"hello` (no closing ") |
| BAD_URL    | Unclosed url(): `url(./path` (no `)`)    |

## Usage

```lua
local css_lexer = require("coding_adventures.css_lexer")

local tokens = css_lexer.tokenize("h1 { color: red; }")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
css.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
css_lexer  ← you are here
    ↓  feeds
css_parser  (coding-adventures-css-parser)
```

## Dependencies

- `coding-adventures-grammar-tools` — parses `css.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
