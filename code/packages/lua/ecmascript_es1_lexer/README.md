# coding-adventures-ecmascript-es1-lexer (Lua)

An ECMAScript 1 (1997) lexer that tokenizes ES1 source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `ecmascript/es1.tokens` grammar file.

## What it does

Given the input `var x = 42;`, the lexer produces:

| # | Type      | Value   |
|---|-----------|---------|
| 1 | VAR       | `var`   |
| 2 | NAME      | `x`     |
| 3 | EQUALS    | `=`     |
| 4 | NUMBER    | `42`    |
| 5 | SEMICOLON | `;`     |
| 6 | EOF       |         |

Whitespace and comments are silently consumed (declared as skip patterns in `es1.tokens`).

## ES1 characteristics

ECMAScript 1 (ECMA-262, 1st Edition, June 1997) was the first standardized JavaScript. Notable limitations:
- No `===` or `!==` (strict equality was added in ES3)
- No `try`/`catch`/`finally`/`throw` (added in ES3)
- No regex literals (formalized in ES3)
- No `let`, `const`, `class`, arrow functions (added in ES2015)

## Token types

### Literals
| Token type | Example match |
|------------|---------------|
| NAME       | `myVar`, `$el`, `_priv` |
| NUMBER     | `42`, `0xFF`, `3.14`, `1e10` |
| STRING     | `"hello"`, `'world'` |

### Keywords (23 total)
`break`, `case`, `continue`, `default`, `delete`, `do`, `else`, `for`, `function`, `if`, `in`, `new`, `return`, `switch`, `this`, `typeof`, `var`, `void`, `while`, `with`, `true`, `false`, `null`

### Operators
| Token type         | Symbol |
|--------------------|--------|
| EQUALS_EQUALS      | `==`   |
| NOT_EQUALS         | `!=`   |
| LESS_EQUALS        | `<=`   |
| GREATER_EQUALS     | `>=`   |
| AND_AND            | `&&`   |
| OR_OR              | `\|\|` |
| PLUS_PLUS          | `++`   |
| MINUS_MINUS        | `--`   |
| LEFT_SHIFT         | `<<`   |
| RIGHT_SHIFT        | `>>`   |
| UNSIGNED_RIGHT_SHIFT | `>>>` |
| EQUALS             | `=`    |
| PLUS, MINUS, STAR, SLASH, PERCENT | `+`, `-`, `*`, `/`, `%` |
| LESS_THAN, GREATER_THAN, BANG | `<`, `>`, `!` |
| AMPERSAND, PIPE, CARET, TILDE | `&`, `\|`, `^`, `~` |
| QUESTION           | `?`    |

### Delimiters
`(`, `)`, `{`, `}`, `[`, `]`, `;`, `,`, `:`, `.`

## Usage

```lua
local es1_lexer = require("coding_adventures.ecmascript_es1_lexer")

local tokens = es1_lexer.tokenize("var x = 1;")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
ecmascript/es1.tokens  (code/grammars/)
    |  parsed by grammar_tools
TokenGrammar
    |  drives
GrammarLexer  (coding-adventures-lexer)
    |  wrapped by
ecmascript_es1_lexer  <-- you are here
```

## Dependencies

- `coding-adventures-grammar-tools` -- parses `es1.tokens`
- `coding-adventures-lexer` -- provides `GrammarLexer`
- `coding-adventures-state-machine` -- used internally by the lexer
- `coding-adventures-directed-graph` -- used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
