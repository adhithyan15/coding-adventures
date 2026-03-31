# coding-adventures-sql-parser

A grammar-driven SQL parser for the coding-adventures monorepo. It takes SQL source text, tokenizes it with `sql_lexer`, loads the `sql.grammar` specification, and produces an Abstract Syntax Tree (AST) using the `GrammarParser` engine from the `parser` package.

## What it does

Given input `SELECT name FROM users WHERE age > 18`, the parser produces:

```
program
└── statement
    └── select_stmt
        ├── SELECT  "SELECT"
        ├── select_list
        │   └── select_item
        │       └── expr → … → column_ref → NAME "name"
        ├── FROM    "FROM"
        ├── table_ref → table_name → NAME "users"
        └── where_clause
            ├── WHERE  "WHERE"
            └── expr → comparison
                ├── additive → … → column_ref → NAME "age"
                ├── cmp_op → GREATER_THAN ">"
                └── additive → … → NUMBER "18"
```

The root node always has `rule_name == "program"` (the entry point of the SQL grammar).

## Supported SQL statements

- `SELECT` — column lists, `*`, `DISTINCT`, `WHERE`, `GROUP BY`, `HAVING`, `ORDER BY`, `LIMIT`, `JOIN`
- `INSERT INTO … VALUES (…)` — with optional column list
- `UPDATE … SET … WHERE …` — single or multiple assignments
- `DELETE FROM … WHERE …`
- `CREATE TABLE … (col_def, …)`
- `DROP TABLE [IF EXISTS] …`

## How it fits in the stack

```
sql_parser  ← this package
     ↓
parser (GrammarParser)
     ↓
grammar_tools (parse_parser_grammar)
     ↓
sql_lexer → lexer → grammar_tools (parse_token_grammar)
```

## Usage

```lua
local sql_parser = require("coding_adventures.sql_parser")

-- Parse and get the AST root
local ast = sql_parser.parse("SELECT name, age FROM users WHERE age > 18")
print(ast.rule_name)  -- "program"

-- Find specific nodes
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

local select_stmt = find_node(ast, "select_stmt")
local where_clause = find_node(ast, "where_clause")
```

## Grammar

The SQL grammar (`code/grammars/sql.grammar`) defines the full ANSI SQL subset. Key rules:

```
program      = statement { ";" statement } [ ";" ] ;
statement    = select_stmt | insert_stmt | update_stmt | delete_stmt | … ;
select_stmt  = "SELECT" select_list "FROM" table_ref … ;
select_list  = STAR | select_item { "," select_item } ;
expr         = or_expr ;
or_expr      = and_expr { "OR" and_expr } ;
comparison   = additive [ cmp_op additive | "BETWEEN" … | "IN" … | "LIKE" … ] ;
```

Keywords are case-insensitive (handled by the SQL lexer).

## API

### `sql_parser.parse(source) → ASTNode`

Parse a SQL string and return the root ASTNode (`rule_name == "program"`). Raises an error on invalid input.

### `sql_parser.create_parser(source) → GrammarParser`

Tokenize the source and return an initialized `GrammarParser` without parsing. Useful for trace-mode debugging.

### `sql_parser.get_grammar() → ParserGrammar`

Return the cached `ParserGrammar` loaded from `sql.grammar`.

## Version

0.1.0
