# coding-adventures-algol-parser

A grammar-driven ALGOL 60 parser for the coding-adventures monorepo. It takes ALGOL 60 source text, tokenizes it with `algol_lexer`, loads the `algol.grammar` specification, and produces an Abstract Syntax Tree (AST) using the `GrammarParser` engine from the `parser` package.

## What it does

Given input `begin integer x; x := 42 end`, the parser produces:

```
program
└── block
    ├── BEGIN          "begin"
    ├── declaration
    │   └── type_decl
    │       ├── type       INTEGER "integer"
    │       └── ident_list IDENT   "x"
    ├── SEMICOLON      ";"
    ├── statement
    │   └── unlabeled_stmt
    │       └── assign_stmt
    │           ├── left_part  variable ASSIGN
    │           └── expression arith_expr INTEGER_LIT "42"
    └── END            "end"
```

The root node always has `rule_name == "program"`.

## How it fits in the stack

```
algol_parser  ← this package
     ↓
parser (GrammarParser)
     ↓
grammar_tools (parse_parser_grammar)
     ↓
algol_lexer → lexer → grammar_tools (parse_token_grammar)
```

## Usage

```lua
local algol_parser = require("coding_adventures.algol_parser")

-- Parse and get the AST root
local ast = algol_parser.parse("begin integer x; x := 42 end")
print(ast.rule_name)         -- "program"

-- Walk the tree recursively
local function walk(node, depth)
    local indent = string.rep("  ", depth)
    if node.rule_name then
        print(indent .. node.rule_name)
        for _, child in ipairs(node.children or {}) do
            walk(child, depth + 1)
        end
    else
        -- token node
        print(indent .. node.type_name .. " " .. tostring(node.value))
    end
end
walk(ast, 0)
```

## Grammar overview

The ALGOL 60 grammar (`code/grammars/algol.grammar`) entry point is `program`, which is a single `block`.

```
program      = block ;
block        = BEGIN { declaration SEMICOLON } statement { SEMICOLON statement } END ;
declaration  = type_decl | array_decl | switch_decl | procedure_decl ;
type_decl    = type ident_list ;
statement    = [ label COLON ] unlabeled_stmt | [ label COLON ] cond_stmt ;
cond_stmt    = IF bool_expr THEN unlabeled_stmt [ ELSE statement ] ;
assign_stmt  = left_part { left_part } expression ;
for_stmt     = FOR IDENT ASSIGN for_list DO statement ;
```

Key design decisions:
- **Dangling else** is resolved at the grammar level: the then-branch uses `unlabeled_stmt` (which excludes conditionals), forcing `begin...end` for nested ifs.
- **Exponentiation is left-associative**: `2^3^4 = (2^3)^4 = 4096` (per the ALGOL 60 report; differs from most modern languages).
- **Call-by-name** is the default parameter passing mode; `value` declares call-by-value params.

## API

### `algol_parser.parse(source) → ASTNode`

Parse an ALGOL 60 string and return the root ASTNode (`rule_name == "program"`). Raises an error on invalid input.

### `algol_parser.create_parser(source) → GrammarParser`

Tokenize the source and return an initialized `GrammarParser` without parsing. Useful for trace-mode debugging.

### `algol_parser.get_grammar() → ParserGrammar`

Return the cached `ParserGrammar` loaded from `algol.grammar`.

## Dependencies

- `coding-adventures-algol-lexer` — tokenizes ALGOL 60 source text
- `coding-adventures-parser` — provides `GrammarParser`
- `coding-adventures-grammar-tools` — parses `algol.grammar`
- `coding-adventures-state-machine` — used internally
- `coding-adventures-directed-graph` — used internally

## Running tests

```bash
cd tests
busted . --verbose --pattern=test_
```

## Version

0.1.0
