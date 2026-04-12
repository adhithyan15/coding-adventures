# coding-adventures-dartmouth-basic-parser (Lua)

A grammar-driven parser for 1964 Dartmouth BASIC. Accepts source text
and returns an Abstract Syntax Tree (AST) built by the `coding-adventures`
grammar engine.

## What is Dartmouth BASIC?

Dartmouth BASIC was invented by John Kemeny and Thomas Kurtz at Dartmouth
College in 1964. It was the first time-sharing BASIC, designed to give
non-science students access to computing via the college's GE-225 mainframe.

Key features of the 1964 specification:
- Every statement is **line-numbered** (`10 LET X = 5`)
- All source is **uppercase** (teletypes had no lowercase)
- Simple **scalar variables**: single letter (`X`) or letter+digit (`A1`)
- **Arrays** dimensioned with `DIM`; accessed as `A(I)`
- **17 statement types**: LET, PRINT, INPUT, IF/THEN, GOTO, GOSUB, RETURN,
  FOR/NEXT, END, STOP, REM, READ, DATA, RESTORE, DIM, DEF
- **11 built-in math functions**: SIN, COS, TAN, ATN, EXP, LOG, ABS, SQR,
  INT, RND, SGN
- **User-defined functions**: `DEF FNA(X) = ...` through `DEF FNZ(X) = ...`

## How it fits in the stack

```
dartmouth_basic_parser   ← this package
       ↓ uses
  dartmouth_basic_lexer  ← tokenizes source text
       ↓ uses
    grammar_tools         ← loads .grammar files
    parser                ← GrammarParser engine
    directed_graph        ← dependency of grammar_tools
    state_machine         ← dependency of lexer
```

The parser reads `code/grammars/dartmouth_basic.grammar` at startup (once,
cached). The grammar has 29 rules covering the full 1964 specification.

## Installation

```bash
luarocks make --local coding-adventures-dartmouth-basic-parser-0.1.0-1.rockspec
```

## Usage

```lua
local bp = require("coding_adventures.dartmouth_basic_parser")

-- Parse a BASIC program
local ast = bp.parse("10 LET X = 5\n20 PRINT X\n30 END\n")
print(ast.rule_name)   -- "program"
print(#ast.children)   -- number of top-level nodes

-- Access the grammar for introspection
local g = bp.get_grammar()
print(#g.rules)        -- 29 rules

-- Get a GrammarParser for manual control
local p = bp.create_parser("10 END\n")
local ast2, err = p:parse()
```

## AST structure

```
program
└── line
    ├── LINE_NUM "10"
    ├── statement
    │   └── let_stmt
    │       ├── token(KEYWORD, "LET")
    │       ├── variable
    │       │   └── token(NAME, "X")
    │       ├── token(EQ, "=")
    │       └── expr → term → power → unary → primary
    │           └── token(NUMBER, "5")
    └── token(NEWLINE, "\n")
```

Node fields:
- `node.rule_name` — grammar rule that produced this node
- `node.children`  — array of child ASTNodes
- `node:is_leaf()` — true when wrapping a single token
- `node:token()`   — the wrapped token (only when `is_leaf()` is true)

Token fields: `type`, `value`, `line`, `col`

## Running tests

```bash
cd tests && busted . --verbose --pattern=test_
```

## Version

0.1.0
