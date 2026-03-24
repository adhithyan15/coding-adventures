# SQL Parser

A grammar-driven parser for an ANSI SQL subset that produces an Abstract Syntax Tree (AST). Covers SELECT, INSERT, UPDATE, DELETE, CREATE TABLE, and DROP TABLE statements with full expression support.

## What it does

This crate parses SQL source text into a tree structure (AST) that reflects the syntactic structure of the query. It uses the `sql-lexer` crate for tokenization and loads the `sql.grammar` file for grammar rules, feeding both into the generic `GrammarParser` from the `parser` crate.

## How it fits in the stack

```text
Source text  ("SELECT name FROM users WHERE age > 18")
      |
      v
sql-lexer            → Vec<Token>  (keywords normalized to uppercase)
      |
      v
sql.grammar          → ParserGrammar (~25 rules)
      |
      v
parser::GrammarParser → GrammarASTNode tree
      |
      v
sql-parser           (this crate — thin glue layer)
```

## Grammar rules (summary)

```ebnf
program           = statement { ";" statement } [ ";" ] ;
statement         = select_stmt | insert_stmt | update_stmt
                  | delete_stmt | create_table_stmt | drop_table_stmt ;
select_stmt       = "SELECT" [ "DISTINCT" | "ALL" ] select_list
                    "FROM" table_ref ... ;
insert_stmt       = "INSERT" "INTO" NAME ... "VALUES" row_value { "," row_value } ;
update_stmt       = "UPDATE" NAME "SET" assignment { "," assignment } ... ;
delete_stmt       = "DELETE" "FROM" NAME [ where_clause ] ;
create_table_stmt = "CREATE" "TABLE" [ "IF" "NOT" "EXISTS" ] NAME "(" col_def { "," col_def } ")" ;
drop_table_stmt   = "DROP" "TABLE" [ "IF" "EXISTS" ] NAME ;
expr              = or_expr ;
or_expr           = and_expr { "OR" and_expr } ;
and_expr          = not_expr { "AND" not_expr } ;
not_expr          = "NOT" not_expr | comparison ;
comparison        = additive [ cmp_op additive | "BETWEEN" ... | "IN" ... | "LIKE" ... | "IS" "NULL" ] ;
additive          = multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative    = unary { ( STAR | "/" | "%" ) unary } ;
unary             = "-" unary | primary ;
primary           = NUMBER | STRING | "NULL" | "TRUE" | "FALSE"
                  | function_call | column_ref | "(" expr ")" ;
```

## Usage

```rust
use coding_adventures_sql_parser::parse_sql;

let ast = parse_sql("SELECT id, name FROM users WHERE age > 18").unwrap();
assert_eq!(ast.rule_name, "program");
```

Or use the factory function for fine-grained control:

```rust
use coding_adventures_sql_parser::create_sql_parser;

let mut parser = create_sql_parser("DELETE FROM temp WHERE id = 5").unwrap();
let ast = parser.parse().expect("parse failed");
```

Both functions return `Result<_, String>` so errors (syntax errors, missing grammar file) are propagated cleanly.

## Key differences from json-parser

- **~25 rules vs 4** — SQL has a dramatically richer grammar than JSON.
- **Multiple statement types** — SELECT, INSERT, UPDATE, DELETE, CREATE, DROP.
- **Expression hierarchy** — OR → AND → NOT → comparison → additive → multiplicative → unary → primary.
- **Case-insensitive keywords** — The sql-lexer normalizes keywords to uppercase before the parser sees them.
- **Result-returning API** — Both `create_sql_parser` and `parse_sql` return `Result` rather than panicking.
- **Start symbol is `program`** — not `value` as in json-parser.

## Running tests

```bash
cargo test --package coding-adventures-sql-parser -- --nocapture
```
