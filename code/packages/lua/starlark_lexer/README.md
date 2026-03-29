# coding-adventures-starlark-lexer (Lua)

A Starlark lexer that tokenizes Starlark source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `starlark.tokens` grammar file.

## What is Starlark?

Starlark is a deterministic subset of Python designed for use as a configuration language. It is used in Bazel BUILD files and many other build systems. Key differences from Python:
- No `while` loops or general iteration constructs
- No `class` definitions
- No `try`/`except`/`raise`
- Significant indentation (like Python)
- Certain Python keywords are reserved but disallowed (`class`, `import`, `while`, etc.)

## What it does

Given the input `def foo(x):`, the lexer produces:

| # | Type   | Value |
|---|--------|-------|
| 1 | DEF    | `def` |
| 2 | NAME   | `foo` |
| 3 | LPAREN | `(`   |
| 4 | NAME   | `x`   |
| 5 | RPAREN | `)`   |
| 6 | COLON  | `:`   |
| 7 | EOF    |       |

Whitespace and comments are silently consumed (declared as skip patterns in `starlark.tokens`). Because `starlark.tokens` uses `mode: indentation`, the lexer also emits `INDENT`, `DEDENT`, and `NEWLINE` tokens for indented blocks.

## Token types

### Literals
| Token type | Example match |
|------------|---------------|
| NAME       | `my_var`, `_private`, `abc123` |
| INT        | `42`, `0xFF`, `0o77` |
| FLOAT      | `3.14`, `1e10`, `.5` |
| STRING     | `"hello"`, `'hi'`, `r"raw"`, `b"bytes"`, `"""triple"""` |

### Keywords
| Token type | Keyword    |
|------------|------------|
| AND        | `and`      |
| BREAK      | `break`    |
| CONTINUE   | `continue` |
| DEF        | `def`      |
| ELIF       | `elif`     |
| ELSE       | `else`     |
| FOR        | `for`      |
| IF         | `if`       |
| IN         | `in`       |
| LAMBDA     | `lambda`   |
| LOAD       | `load`     |
| NOT        | `not`      |
| OR         | `or`       |
| PASS       | `pass`     |
| RETURN     | `return`   |
| TRUE       | `True`     |
| FALSE      | `False`    |
| NONE       | `None`     |

### Three-character operators
| Token type          | Symbol |
|---------------------|--------|
| DOUBLE_STAR_EQUALS  | `**=`  |
| LEFT_SHIFT_EQUALS   | `<<=`  |
| RIGHT_SHIFT_EQUALS  | `>>=`  |
| FLOOR_DIV_EQUALS    | `//=`  |

### Two-character operators
| Token type    | Symbol |
|---------------|--------|
| DOUBLE_STAR   | `**`   |
| FLOOR_DIV     | `//`   |
| LEFT_SHIFT    | `<<`   |
| RIGHT_SHIFT   | `>>`   |
| EQUALS_EQUALS | `==`   |
| NOT_EQUALS    | `!=`   |
| LESS_EQUALS   | `<=`   |
| GREATER_EQUALS| `>=`   |
| PLUS_EQUALS   | `+=`   |
| MINUS_EQUALS  | `-=`   |
| STAR_EQUALS   | `*=`   |
| SLASH_EQUALS  | `/=`   |
| PERCENT_EQUALS| `%=`   |
| AMP_EQUALS    | `&=`   |
| PIPE_EQUALS   | `\|=`  |
| CARET_EQUALS  | `^=`   |

### Single-character operators
| Token type   | Symbol |
|--------------|--------|
| PLUS         | `+`    |
| MINUS        | `-`    |
| STAR         | `*`    |
| SLASH        | `/`    |
| PERCENT      | `%`    |
| EQUALS       | `=`    |
| LESS_THAN    | `<`    |
| GREATER_THAN | `>`    |
| AMP          | `&`    |
| PIPE         | `\|`   |
| CARET        | `^`    |
| TILDE        | `~`    |

### Delimiters
| Token type | Symbol |
|------------|--------|
| LPAREN     | `(`    |
| RPAREN     | `)`    |
| LBRACKET   | `[`    |
| RBRACKET   | `]`    |
| LBRACE     | `{`    |
| RBRACE     | `}`    |
| COMMA      | `,`    |
| COLON      | `:`    |
| SEMICOLON  | `;`    |
| DOT        | `.`    |

### Indentation tokens (from `mode: indentation`)
| Token type | Meaning |
|------------|---------|
| NEWLINE    | Logical line boundary |
| INDENT     | Indentation level increased |
| DEDENT     | Indentation level decreased |

## Usage

```lua
local starlark_lexer = require("coding_adventures.starlark_lexer")

local tokens = starlark_lexer.tokenize("def foo(x):\n    return x + 1\n")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
starlark.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
starlark_lexer  ← you are here
    ↓  feeds
starlark_parser  (future)
```

## Dependencies

- `coding-adventures-grammar-tools` — parses `starlark.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
