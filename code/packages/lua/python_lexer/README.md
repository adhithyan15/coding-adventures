# coding-adventures-python-lexer (Lua)

A Python lexer that tokenizes Python source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `python.tokens` grammar file.

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

Whitespace is silently consumed (declared as a skip pattern in `python.tokens`).

## Token types

### Literals
| Token type | Example match |
|------------|---------------|
| NAME       | `my_var`, `_private`, `__init__` |
| NUMBER     | `42`, `0`, `100` |
| STRING     | `"hello"`, `"a\nb"` |

### Keywords
| Token type | Keyword   |
|------------|-----------|
| IF         | `if`      |
| ELIF       | `elif`    |
| ELSE       | `else`    |
| WHILE      | `while`   |
| FOR        | `for`     |
| DEF        | `def`     |
| RETURN     | `return`  |
| CLASS      | `class`   |
| IMPORT     | `import`  |
| FROM       | `from`    |
| AS         | `as`      |
| TRUE       | `True`    |
| FALSE      | `False`   |
| NONE       | `None`    |

### Operators
| Token type   | Symbol |
|--------------|--------|
| EQUALS_EQUALS | `==`  |
| EQUALS       | `=`    |
| PLUS         | `+`    |
| MINUS        | `-`    |
| STAR         | `*`    |
| SLASH        | `/`    |

### Delimiters
| Token type | Symbol |
|------------|--------|
| LPAREN     | `(`    |
| RPAREN     | `)`    |
| COMMA      | `,`    |
| COLON      | `:`    |

## Usage

```lua
local py_lexer = require("coding_adventures.python_lexer")

local tokens = py_lexer.tokenize("def foo(x):")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
python.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
python_lexer  ← you are here
    ↓  feeds
python_parser  (future)
```

## Dependencies

- `coding-adventures-grammar-tools` — parses `python.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
