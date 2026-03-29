# coding-adventures-lisp-parser

Lua package — grammar-driven Lisp/Scheme parser that builds Abstract Syntax Trees.

## What it does

Parses Lisp/Scheme source text into an AST using the `lisp.grammar` rules
and the `GrammarParser` engine.

```
(define x 42)
→ program
  └── sexpr
      └── list
          ├── LPAREN
          ├── list_body
          │   ├── sexpr → atom → SYMBOL "define"
          │   ├── sexpr → atom → SYMBOL "x"
          │   └── sexpr → atom → NUMBER "42"
          └── RPAREN
```

## Grammar

```
program   = { sexpr } ;
sexpr     = atom | list | quoted ;
atom      = NUMBER | SYMBOL | STRING ;
list      = LPAREN list_body RPAREN ;
list_body = [ sexpr { sexpr } [ DOT sexpr ] ] ;
quoted    = QUOTE sexpr ;
```

## Usage

```lua
local lisp_parser = require("coding_adventures.lisp_parser")

-- Parse a single expression
local ast = lisp_parser.parse("(+ 1 2)")
print(ast.rule_name)  -- "program"

-- Parse a multi-expression program
local ast = lisp_parser.parse("(define x 42) (display x)")

-- Create a parser without immediately parsing
local p = lisp_parser.create_parser("'(1 2 3)")
local ast, err = p:parse()
```

## How it fits in the stack

```
lisp.tokens  lisp.grammar
     ↓              ↓
lisp_lexer ──→ lisp_parser    ← this package
                    ↓
              (evaluator, macro expander, …)
```

## Dependencies

- `coding-adventures-lisp-lexer` — tokenizes the source
- `coding-adventures-parser` — provides `GrammarParser`
- `coding-adventures-grammar-tools` — parses the `.grammar` file
- `coding-adventures-lexer` (transitive)
- `coding-adventures-directed-graph` (transitive)
- `coding-adventures-state-machine` (transitive)
