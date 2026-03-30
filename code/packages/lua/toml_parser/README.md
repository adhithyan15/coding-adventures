# coding-adventures-toml-parser

A grammar-driven TOML parser for the coding-adventures monorepo. It takes TOML source text, tokenizes it with `toml_lexer`, loads the `toml.grammar` specification, and produces an Abstract Syntax Tree (AST) using the `GrammarParser` engine from the `parser` package.

## What it does

Given input:

```toml
[server]
host = "localhost"
port = 8080
```

The parser produces:

```
document
├── expression
│   └── table_header
│       ├── LBRACKET "["
│       ├── key → simple_key → BARE_KEY "server"
│       └── RBRACKET "]"
├── NEWLINE
├── expression
│   └── keyval
│       ├── key → simple_key → BARE_KEY "host"
│       ├── EQUALS "="
│       └── value → BASIC_STRING '"localhost"'
├── NEWLINE
└── expression
    └── keyval
        ├── key → simple_key → BARE_KEY "port"
        ├── EQUALS "="
        └── value → INTEGER "8080"
```

The root node always has `rule_name == "document"`.

## How it fits in the stack

```
toml_parser  ← this package
     ↓
parser (GrammarParser)
     ↓
grammar_tools (parse_parser_grammar)
     ↓
toml_lexer → lexer → grammar_tools (parse_token_grammar)
```

## TOML-specific considerations

**Newlines are significant** in TOML — key-value pairs are terminated by newlines. The `toml.grammar` references NEWLINE, so the `GrammarParser` automatically preserves NEWLINE tokens (rather than skipping them as it does for JSON).

## Usage

```lua
local toml_parser = require("coding_adventures.toml_parser")

-- Parse and get the AST root
local ast = toml_parser.parse('[server]\nhost = "localhost"\nport = 8080\n')
print(ast.rule_name)  -- "document"

-- Traverse to find keyval nodes
local function find_keyvals(node, results)
    results = results or {}
    if type(node) ~= "table" then return results end
    if node.rule_name == "keyval" then
        results[#results + 1] = node
    end
    for _, child in ipairs(node.children or {}) do
        find_keyvals(child, results)
    end
    return results
end

local kvs = find_keyvals(ast)
print(#kvs)  -- 2
```

## Grammar

The TOML grammar (`code/grammars/toml.grammar`) has ~12 rules. Key ones:

```
document           = { NEWLINE | expression } ;
expression         = array_table_header | table_header | keyval ;
keyval             = key EQUALS value ;
key                = simple_key { DOT simple_key } ;
table_header       = LBRACKET key RBRACKET ;
array_table_header = LBRACKET LBRACKET key RBRACKET RBRACKET ;
value              = BASIC_STRING | … | array | inline_table ;
array              = LBRACKET array_values RBRACKET ;
inline_table       = LBRACE [ keyval { COMMA keyval } ] RBRACE ;
```

## API

### `toml_parser.parse(source) → ASTNode`

Parse a TOML string and return the root ASTNode. Raises an error on invalid input.

### `toml_parser.create_parser(source) → GrammarParser`

Tokenize the source and return an initialized `GrammarParser` without parsing.

### `toml_parser.get_grammar() → ParserGrammar`

Return the cached `ParserGrammar` loaded from `toml.grammar`.

## Version

0.1.0
