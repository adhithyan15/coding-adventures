# coding-adventures-ruby-lexer (Lua)

A Ruby lexer that tokenizes Ruby source text into a flat stream of typed tokens. It is a thin wrapper around the grammar-driven `GrammarLexer` from `coding-adventures-lexer`, configured by the shared `ruby.tokens` grammar file.

## What it does

Given the input `def greet(name)`, the lexer produces:

| # | Type   | Value   |
|---|--------|---------|
| 1 | DEF    | `def`   |
| 2 | NAME   | `greet` |
| 3 | LPAREN | `(`     |
| 4 | NAME   | `name`  |
| 5 | RPAREN | `)`     |
| 6 | EOF    |         |

Whitespace is silently consumed (declared as a skip pattern in `ruby.tokens`).

## Token types

### Literals
| Token type | Example match |
|------------|---------------|
| NAME       | `my_var`, `_private`, `MyClass` |
| NUMBER     | `42`, `0`, `100` |
| STRING     | `"hello"`, `"a\nb"` |

### Keywords
| Token type | Keyword   |
|------------|-----------|
| DEF        | `def`     |
| END        | `end`     |
| CLASS      | `class`   |
| MODULE     | `module`  |
| IF         | `if`      |
| ELSIF      | `elsif`   |
| ELSE       | `else`    |
| UNLESS     | `unless`  |
| WHILE      | `while`   |
| UNTIL      | `until`   |
| FOR        | `for`     |
| DO         | `do`      |
| RETURN     | `return`  |
| BEGIN      | `begin`   |
| RESCUE     | `rescue`  |
| ENSURE     | `ensure`  |
| REQUIRE    | `require` |
| PUTS       | `puts`    |
| YIELD      | `yield`   |
| THEN       | `then`    |
| TRUE       | `true`    |
| FALSE      | `false`   |
| NIL        | `nil`     |
| AND        | `and`     |
| OR         | `or`      |
| NOT        | `not`     |

### Operators
| Token type     | Symbol |
|----------------|--------|
| EQUALS_EQUALS  | `==`   |
| DOT_DOT        | `..`   |
| HASH_ROCKET    | `=>`   |
| NOT_EQUALS     | `!=`   |
| LESS_EQUALS    | `<=`   |
| GREATER_EQUALS | `>=`   |
| EQUALS         | `=`    |
| PLUS           | `+`    |
| MINUS          | `-`    |
| STAR           | `*`    |
| SLASH          | `/`    |
| LESS_THAN      | `<`    |
| GREATER_THAN   | `>`    |

### Delimiters
| Token type | Symbol |
|------------|--------|
| LPAREN     | `(`    |
| RPAREN     | `)`    |
| COMMA      | `,`    |
| COLON      | `:`    |

## Usage

```lua
local rb_lexer = require("coding_adventures.ruby_lexer")

local tokens = rb_lexer.tokenize("def greet(name)")
for _, tok in ipairs(tokens) do
    print(tok.type, tok.value, tok.line, tok.col)
end
```

## How it fits in the stack

```
ruby.tokens  (code/grammars/)
    ↓  parsed by grammar_tools
TokenGrammar
    ↓  drives
GrammarLexer  (coding-adventures-lexer)
    ↓  wrapped by
ruby_lexer  ← you are here
    ↓  feeds
ruby_parser  (future)
```

## Dependencies

- `coding-adventures-grammar-tools` — parses `ruby.tokens`
- `coding-adventures-lexer` — provides `GrammarLexer`
- `coding-adventures-state-machine` — used internally by the lexer
- `coding-adventures-directed-graph` — used internally by grammar tools

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```
