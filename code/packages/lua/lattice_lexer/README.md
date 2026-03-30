# coding-adventures-lattice-lexer (Lua)

A Lattice lexer that tokenizes Lattice (a CSS superset) source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `lattice.tokens` grammar file.

## What is Lattice?

Lattice is a CSS superset language that adds powerful features on top of standard CSS:
- **Variables**: `$color: #ff0000;`
- **Mixins**: `@mixin` and `@include`
- **Control flow**: `@if`, `@else`, `@for`, `@each`
- **Functions**: `@function` and `@return`
- **Modules**: `@use`
- **Nesting**: `.parent { .child { ... } }`
- **Placeholder selectors**: `%button-base` (used with `@extend`)
- **Single-line comments**: `// to end of line`
- **Comparison operators**: `==`, `!=`, `>=`, `<=` (in `@if` conditions)
- **Variable flags**: `!default`, `!global`

Every valid CSS file is valid Lattice.

## What it does

Given the input `$color: #ff0000;`, the lexer produces:

| # | Type      | Value     |
|---|-----------|-----------|
| 1 | VARIABLE  | `$color`  |
| 2 | COLON     | `:`       |
| 3 | HASH      | `#ff0000` |
| 4 | SEMICOLON | `;`       |
| 5 | EOF       |           |

Whitespace and comments are silently consumed. Because `lattice.tokens` uses `escapes: none`, string token values include their surrounding quotes and any backslash sequences as raw text (CSS escape decoding is a semantic concern handled post-parse).

## Token types

### Lattice-specific tokens
| Token type    | Example match                      |
|---------------|------------------------------------|
| VARIABLE      | `$color`, `$font-size`             |
| PLACEHOLDER   | `%button-base`, `%flex-center`     |
| EQUALS_EQUALS | `==`                               |
| NOT_EQUALS    | `!=`                               |
| GREATER_EQUALS| `>=`                               |
| LESS_EQUALS   | `<=`                               |
| BANG_DEFAULT  | `!default`                         |
| BANG_GLOBAL   | `!global`                          |

### Numeric tokens (priority: DIMENSION > PERCENTAGE > NUMBER)
| Token type | Example match |
|------------|---------------|
| DIMENSION  | `10px`, `1.5em`, `-2rem` |
| PERCENTAGE | `50%`, `100%` |
| NUMBER     | `0`, `3.14`, `-1` |

### Other tokens
| Token type     | Example match               |
|----------------|-----------------------------|
| STRING         | `"hello"`, `'world'`        |
| HASH           | `#ff0000`, `#abc`           |
| AT_KEYWORD     | `@media`, `@mixin`, `@if`   |
| URL_TOKEN      | `url(/path/to/file.png)`    |
| FUNCTION       | `rgb(`, `calc(`, `var(`     |
| CUSTOM_PROPERTY| `--primary-color`           |
| IDENT          | `red`, `serif`, `auto`      |
| COLON_COLON    | `::`                        |
| TILDE_EQUALS   | `~=`                        |
| PIPE_EQUALS    | `\|=`                       |
| CARET_EQUALS   | `^=`                        |
| DOLLAR_EQUALS  | `$=`                        |
| STAR_EQUALS    | `*=`                        |
| BANG           | `!`                         |
| LBRACE/RBRACE  | `{`, `}`                    |
| LPAREN/RPAREN  | `(`, `)`                    |
| LBRACKET/RBRACKET | `[`, `]`               |
| SEMICOLON      | `;`                         |
| COLON          | `:`                         |
| COMMA          | `,`                         |
| DOT            | `.`                         |
| AMPERSAND      | `&`                         |
| PLUS/MINUS     | `+`, `-`                    |
| GREATER/LESS   | `>`, `<`                    |
| STAR/SLASH     | `*`, `/`                    |
| TILDE/PIPE     | `~`, `\|`                   |
| EQUALS         | `=`                         |

## Usage

```lua
local lattice_lexer = require("coding_adventures.lattice_lexer")

local tokens = lattice_lexer.tokenize("$color: #ff0000;")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
lattice.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
lattice_lexer  ← you are here
    ↓  feeds
lattice_parser  (future)
```

## Dependencies

- `coding-adventures-grammar-tools` — parses `lattice.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
