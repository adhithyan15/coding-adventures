# coding-adventures-python-parser

A grammar-driven Python parser for the coding-adventures monorepo. It takes Python source text, tokenizes it with `python_lexer`, loads the `python.grammar` specification, and produces an Abstract Syntax Tree (AST) using the `GrammarParser` engine from the `parser` package.

## What it does

Given input `x = 5`, the parser produces:

```
program
└── statement
    └── assignment
        ├── NAME    "x"
        ├── EQUALS  "="
        └── expression
            └── term
                └── factor
                    └── NUMBER  "5"
```

For `r = 1 + 2 * 3`, operator precedence is correctly encoded in the tree:

```
program
└── statement
    └── assignment
        ├── NAME    "r"
        ├── EQUALS  "="
        └── expression
            ├── term → factor → NUMBER "1"
            ├── PLUS "+"
            └── term
                ├── factor → NUMBER "2"
                ├── STAR "*"
                └── factor → NUMBER "3"
```

The root node always has `rule_name == "program"` (the entry point of the Python grammar).

## Supported Python constructs

- Assignments: `x = 5`  `name = "Alice"`
- Expression statements: `42`  `x`
- Arithmetic with correct precedence: `+` and `-` at expression level, `*` and `/` at term level
- Parenthesized groups: `(a + b) * c`
- Multiple statements

## How it fits in the stack

```
python_parser  ← this package
      ↓
parser (GrammarParser)
      ↓
grammar_tools (parse_parser_grammar)
      ↓
python_lexer → lexer → grammar_tools (parse_token_grammar)
```

## Usage

```lua
local python_parser = require("coding_adventures.python_parser")

-- Parse and get the AST root
local ast = python_parser.parse("x = 5\ny = x + 1")
print(ast.rule_name)  -- "program"

-- Walk the tree looking for assignments
local function find_node(node, rule_name)
    if type(node) ~= "table" then return nil end
    if node.rule_name == rule_name then return node end
    if node.children then
        for _, child in ipairs(node.children) do
            local found = find_node(child, rule_name)
            if found then return found end
        end
    end
    return nil
end

local assign = find_node(ast, "assignment")
-- assign.children[1] is the NAME token (x)
-- assign.children[2] is the EQUALS token
-- assign.children[3] is the expression subtree
```

## Grammar

The Python grammar (`code/grammars/python.grammar`) defines a focused subset:

```
program      = { statement } ;
statement    = assignment | expression_stmt ;
assignment   = NAME EQUALS expression ;
expression_stmt = expression ;
expression   = term { ( PLUS | MINUS ) term } ;
term         = factor { ( STAR | SLASH ) factor } ;
factor       = NUMBER | STRING | NAME | LPAREN expression RPAREN ;
```

The two-level `expression`/`term` structure encodes operator precedence:
multiplication and division bind tighter than addition and subtraction.

## API

### `python_parser.parse(source) → ASTNode`

Parse a Python string and return the root ASTNode (`rule_name == "program"`). Raises an error on invalid input.

### `python_parser.create_parser(source) → GrammarParser`

Tokenize the source and return an initialized `GrammarParser` without parsing. Useful for trace-mode debugging.

### `python_parser.get_grammar() → ParserGrammar`

Return the cached `ParserGrammar` loaded from `python.grammar`.

## Version

0.1.0
