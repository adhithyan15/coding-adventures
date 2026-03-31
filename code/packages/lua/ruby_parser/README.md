# coding-adventures-ruby-parser

A grammar-driven Ruby parser for the coding-adventures monorepo. It takes Ruby source text, tokenizes it with `ruby_lexer`, loads the `ruby.grammar` specification, and produces an Abstract Syntax Tree (AST) using the `GrammarParser` engine from the `parser` package.

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

For `puts("hello")`, the parser produces:

```
program
└── statement
    └── method_call
        ├── KEYWORD  "puts"
        ├── LPAREN   "("
        ├── expression → term → factor → STRING "hello"
        └── RPAREN   ")"
```

The root node always has `rule_name == "program"` (the entry point of the Ruby grammar).

## Supported Ruby constructs

- Assignments: `x = 5`  `name = "Alice"`
- Method calls: `puts("hello")`  `foo(1 + 2)`
- Expression statements: `42`  `x`
- Arithmetic with correct precedence: `+` and `-` at expression level, `*` and `/` at term level
- Parenthesized groups: `(a + b) * c`
- Keyword expressions: `true`, `false`, `nil`
- Multiple statements

## How it fits in the stack

```
ruby_parser  ← this package
      ↓
parser (GrammarParser)
      ↓
grammar_tools (parse_parser_grammar)
      ↓
ruby_lexer → lexer → grammar_tools (parse_token_grammar)
```

## Usage

```lua
local ruby_parser = require("coding_adventures.ruby_parser")

-- Parse and get the AST root
local ast = ruby_parser.parse('x = 5\nputs("hello")')
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
local call   = find_node(ast, "method_call")
```

## Grammar

The Ruby grammar (`code/grammars/ruby.grammar`) defines a focused subset:

```
program      = { statement } ;
statement    = assignment | method_call | expression_stmt ;
assignment   = NAME EQUALS expression ;
method_call  = ( NAME | KEYWORD ) LPAREN [ expression { COMMA expression } ] RPAREN ;
expression_stmt = expression ;
expression   = term { ( PLUS | MINUS ) term } ;
term         = factor { ( STAR | SLASH ) factor } ;
factor       = NUMBER | STRING | NAME | KEYWORD | LPAREN expression RPAREN ;
```

Ruby differs from Python: keywords such as `puts`, `true`, `false`, and `nil`
appear as KEYWORD tokens and are valid in expression and method call contexts.
Blocks and `end`-delimited constructs are parsed by the hand-written Perl
parser in the sibling `ruby-parser` package.

## API

### `ruby_parser.parse(source) → ASTNode`

Parse a Ruby string and return the root ASTNode (`rule_name == "program"`). Raises an error on invalid input.

### `ruby_parser.create_parser(source) → GrammarParser`

Tokenize the source and return an initialized `GrammarParser` without parsing. Useful for trace-mode debugging.

### `ruby_parser.get_grammar() → ParserGrammar`

Return the cached `ParserGrammar` loaded from `ruby.grammar`.

## Version

0.1.0
