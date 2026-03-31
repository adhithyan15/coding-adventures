# coding-adventures-json-parser

A grammar-driven JSON parser for the coding-adventures monorepo. It takes JSON source text, tokenizes it with `json_lexer`, loads the `json.grammar` specification, and produces an Abstract Syntax Tree (AST) using the `GrammarParser` engine from the `parser` package.

## What it does

Given input `{"key": 42}`, the parser produces:

```
value
└── object
    ├── LBRACE  "{"
    ├── pair
    │   ├── STRING  '"key"'
    │   ├── COLON   ":"
    │   └── value
    │       └── NUMBER  "42"
    └── RBRACE  "}"
```

The root node always has `rule_name == "value"` (the entry point of the JSON grammar).

## How it fits in the stack

```
json_parser  ← this package
     ↓
parser (GrammarParser)
     ↓
grammar_tools (parse_parser_grammar)
     ↓
json_lexer → lexer → grammar_tools (parse_token_grammar)
```

## Usage

```lua
local json_parser = require("coding_adventures.json_parser")

-- Parse and get the AST root
local ast = json_parser.parse('{"name": "Alice", "age": 30}')
print(ast.rule_name)         -- "value"
print(ast.children[1].rule_name)  -- "object"

-- Walk the tree
local function walk(node, depth)
    local indent = string.rep("  ", depth)
    if node.rule_name then
        print(indent .. node.rule_name)
        for _, child in ipairs(node.children or {}) do
            walk(child, depth + 1)
        end
    else
        -- token
        print(indent .. node.type_name .. " " .. tostring(node.value))
    end
end
walk(ast, 0)
```

## Grammar

The JSON grammar (`code/grammars/json.grammar`) has four rules:

```
value  = object | array | STRING | NUMBER | TRUE | FALSE | NULL ;
object = LBRACE [ pair { COMMA pair } ] RBRACE ;
pair   = STRING COLON value ;
array  = LBRACKET [ value { COMMA value } ] RBRACKET ;
```

## API

### `json_parser.parse(source) → ASTNode`

Parse a JSON string and return the root ASTNode. Raises an error on invalid input.

### `json_parser.create_parser(source) → GrammarParser`

Tokenize the source and return an initialized `GrammarParser` without parsing. Useful for trace-mode debugging.

### `json_parser.get_grammar() → ParserGrammar`

Return the cached `ParserGrammar` loaded from `json.grammar`.

## Version

0.1.0
