# coding-adventures-javascript-parser

A grammar-driven JavaScript parser for the coding-adventures monorepo. It takes JavaScript source text, tokenizes it with `javascript_lexer`, loads the `javascript.grammar` specification, and produces an Abstract Syntax Tree (AST) using the `GrammarParser` engine from the `parser` package.

## What it does

Given input `var x = 5;`, the parser produces:

```
program
└── statement
    └── var_declaration
        ├── KEYWORD    "var"
        ├── NAME       "x"
        ├── EQUALS     "="
        ├── expression
        │   └── term
        │       └── factor
        │           └── NUMBER  "5"
        └── SEMICOLON  ";"
```

For `var r = 1 + 2 * 3;`, operator precedence is correctly encoded in the tree:

```
program
└── statement
    └── var_declaration
        ├── KEYWORD  "var"
        ├── NAME     "r"
        ├── EQUALS   "="
        ├── expression
        │   ├── term → factor → NUMBER "1"
        │   ├── PLUS "+"
        │   └── term
        │       ├── factor → NUMBER "2"
        │       ├── STAR "*"
        │       └── factor → NUMBER "3"
        └── SEMICOLON ";"
```

The root node always has `rule_name == "program"` (the entry point of the JavaScript grammar).

## Supported JavaScript constructs

- Variable declarations: `var x = 5;`  `let y = "hello";`  `const z = true;`
- Assignments: `x = 10;`
- Expression statements: `42;`  `x;`
- Arithmetic with correct precedence: `+` and `-` at expression level, `*` and `/` at term level
- Parenthesized groups: `(a + b) * c`
- Multiple statements in a single `parse()` call

## How it fits in the stack

```
javascript_parser  ← this package
        ↓
parser (GrammarParser)
        ↓
grammar_tools (parse_parser_grammar)
        ↓
javascript_lexer → lexer → grammar_tools (parse_token_grammar)
```

## Usage

```lua
local javascript_parser = require("coding_adventures.javascript_parser")

-- Parse and get the AST root
local ast = javascript_parser.parse("var x = 5;\nlet y = x + 1;")
print(ast.rule_name)  -- "program"

-- Walk the tree looking for var_declarations
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

local decl = find_node(ast, "var_declaration")
-- decl.children[1] is the KEYWORD token (var/let/const)
-- decl.children[2] is the NAME token
```

## Grammar

The JavaScript grammar (`code/grammars/javascript.grammar`) defines a focused subset:

```
program         = { statement } ;
statement       = var_declaration | assignment | expression_stmt ;
var_declaration = KEYWORD NAME EQUALS expression SEMICOLON ;
assignment      = NAME EQUALS expression SEMICOLON ;
expression_stmt = expression SEMICOLON ;
expression      = term { ( PLUS | MINUS ) term } ;
term            = factor { ( STAR | SLASH ) factor } ;
factor          = NUMBER | STRING | NAME | KEYWORD | LPAREN expression RPAREN ;
```

The two-level `expression`/`term` structure encodes operator precedence:
multiplication and division bind tighter than addition and subtraction.

## API

### `javascript_parser.parse(source) → ASTNode`

Parse a JavaScript string and return the root ASTNode (`rule_name == "program"`). Raises an error on invalid input.

### `javascript_parser.create_parser(source) → GrammarParser`

Tokenize the source and return an initialized `GrammarParser` without parsing. Useful for trace-mode debugging.

### `javascript_parser.get_grammar() → ParserGrammar`

Return the cached `ParserGrammar` loaded from `javascript.grammar`.

## Version

0.1.0
