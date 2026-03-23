# SQL Grammar Specification

## Overview

This document specifies the SQL subset implemented by the `sql-lexer` and
`sql-parser` packages. The grammar targets **ANSI SQL** with the most widely-
used constructs: `SELECT`, `INSERT`, `UPDATE`, `DELETE`, `CREATE TABLE`, and
`DROP TABLE`.

SQL is implemented using the grammar-driven lexer/parser infrastructure:
- `code/grammars/sql.tokens` — token definitions loaded by `GrammarLexer`
- `code/grammars/sql.grammar` — parser rules loaded by `GrammarParser`
- Both files use `# @version 1` and (for `.tokens`) `# @case_insensitive true`

---

## Language Subset

### DQL — Data Query Language

```sql
SELECT [DISTINCT | ALL] ...
FROM table_ref
[JOIN ...]
[WHERE expr]
[GROUP BY ...]
[HAVING expr]
[ORDER BY ...]
[LIMIT n [OFFSET m]]
```

Supported:
- `SELECT *` and `SELECT col1, col2, expr AS alias`
- `DISTINCT` and `ALL` modifiers
- `FROM single_table` (no subqueries in FROM)
- `INNER JOIN`, `LEFT [OUTER] JOIN`, `RIGHT [OUTER] JOIN`,
  `FULL [OUTER] JOIN`, `CROSS JOIN` with `ON` condition
- `WHERE`, `GROUP BY`, `HAVING`, `ORDER BY [ASC|DESC]`, `LIMIT`/`OFFSET`
- Scalar subqueries are NOT supported in this subset

### DML — Data Manipulation Language

```sql
INSERT INTO table [(col1, col2, ...)] VALUES (v1, v2, ...), ...
UPDATE table SET col = expr [WHERE expr]
DELETE FROM table [WHERE expr]
```

### DDL — Data Definition Language

```sql
CREATE TABLE [IF NOT EXISTS] name (col_def, ...)
DROP TABLE [IF EXISTS] name
```

Column definitions:
```sql
col_name type_name [NOT NULL] [NULL] [PRIMARY KEY] [UNIQUE] [DEFAULT value]
```

The `type_name` is parsed as a bare `NAME` token — the grammar does not
enumerate SQL types (`INT`, `VARCHAR`, `TEXT`, etc.) because they are treated
as identifiers by the lexer when not in the keyword list.

---

## Case-Insensitive Keywords

SQL is defined as case-insensitive by the ANSI standard.
`sql.tokens` declares `# @case_insensitive true`.

**Effect:** The `GrammarLexer` normalizes NAME tokens whose value (uppercased)
appears in the keyword set to `KEYWORD` tokens with the uppercase value. So
`select`, `SELECT`, and `Select` all produce `KEYWORD` with value `"SELECT"`.

**Grammar literals** therefore use uppercase: `"SELECT"`, `"FROM"`, `"WHERE"`.

**Identifiers** (table names, column names) that happen to share a name with a
keyword would be reclassified as KEYWORD and fail to parse as NAME — this is
intentional and matches real SQL parsers. Users must quote such identifiers
with backticks (`` `order` ``), which the lexer emits as `NAME` tokens via the
`QUOTED_ID = /\`[^\`]+\`/ -> NAME` rule.

---

## Token Design

### Operator Ordering

Longer operators must come first to avoid prefix matching:

```
LESS_EQUALS    = "<="    # before LESS_THAN
GREATER_EQUALS = ">="    # before GREATER_THAN
NOT_EQUALS     = "!="    # before nothing, but explicit is clear
NEQ_ANSI       = "<>"    # -> NOT_EQUALS alias (ANSI standard)
```

The `NEQ_ANSI` alias means `<>` and `!=` both produce `NOT_EQUALS` tokens.
The grammar references `NOT_EQUALS` in `cmp_op` and the lexer/parser resolve
it via string `TypeName` comparison.

### Custom TypeNames (No Enum Required)

Token types like `NOT_EQUALS`, `LESS_EQUALS`, `GREATER_EQUALS` have no
corresponding enum constant in the lexer's `TokenType` enumeration. They are
matched in the parser via string comparison of `token.TypeName == "NOT_EQUALS"`.
This is the designed extensibility mechanism — grammar-driven parsing does not
need enum changes for new token types.

### STAR Disambiguation

`STAR = "*"` appears in three contexts:
1. `SELECT *` — matched as `STAR` in `select_list`
2. `COUNT(*)` — matched as `STAR` in `function_call`
3. `a * b` — matched as `STAR` in `multiplicative`

The grammar disambiguates by position. There is no ambiguity because the parser
is deterministic (PEG ordered choice): `select_list = STAR | select_item { "," select_item }` tries `STAR` first; if we are already inside `multiplicative`,
the outer rule structure determines context.

### PERCENT Token

SQL's modulo operator `%` is included for completeness, even though it is not
standard ANSI SQL in some dialects. It is defined as `PERCENT = "%"` and
referenced in `multiplicative` as `"%" unary`.

### Block Comment Escaping

Inside `.tokens` regex patterns, `/` is the pattern delimiter. To match the
`/*` opening of a SQL block comment:

```
BLOCK_COMMENT = /\x2f\*([^*]|\*[^\x2f])*\*\x2f/
```

`\x2f` is the Unicode escape for `/`. This is required by all implementations
of the grammar-driven lexer.

---

## Grammar Design

### Expression Precedence

Expressions are encoded as a chain of rules, from lowest to highest precedence:

```
expr → or_expr → and_expr → not_expr → comparison → additive
     → multiplicative → unary → primary
```

This mirrors standard SQL precedence:
1. `OR` (lowest)
2. `AND`
3. `NOT` (unary)
4. Comparison operators (`=`, `<>`, `<`, `>`, `<=`, `>=`, `BETWEEN`, `IN`, `LIKE`, `IS NULL`)
5. `+`, `-`
6. `*`, `/`, `%`
7. Unary `-`
8. Atoms: literals, names, function calls, parenthesized expressions (highest)

### BETWEEN...AND Ambiguity

`BETWEEN x AND y` uses the keyword `AND`, which is also the boolean `AND`
operator. The grammar handles this without ambiguity because `comparison` is
tried before `and_expr` in the rule chain.

In `comparison`:
```
comparison = additive [ ... | "BETWEEN" additive "AND" additive | ... ] ;
```

The `"BETWEEN" additive "AND" additive` alternative consumes the `AND` token
as part of the BETWEEN syntax. By the time `and_expr` evaluates its
`{ "AND" not_expr }` loop, the `AND` has already been consumed.

Example: `a BETWEEN 1 AND 10 AND b > 0`
- `comparison` matches `a BETWEEN 1 AND 10`
- `and_expr` matches `<comparison> AND <not_expr>` where the second `AND` is
  the boolean operator

### NOT Placement

`NOT` appears in multiple contexts:

1. **Unary `NOT`** (boolean negation): `not_expr = "NOT" not_expr | comparison`
2. **`NOT IN`**: `comparison` includes `"NOT" "IN" "(" value_list ")"`
3. **`NOT BETWEEN`**: `comparison` includes `"NOT" "BETWEEN" additive "AND" additive`
4. **`NOT LIKE`**: `comparison` includes `"NOT" "LIKE" additive`
5. **`IS NOT NULL`**: `comparison` includes `"IS" "NOT" "NULL"`
6. **`NOT NULL` column constraint**: `col_constraint` includes `"NOT" "NULL"`

Because `comparison` handles all the compound `NOT X` forms before `not_expr`'s
fallthrough to `comparison`, there is no ambiguity. The grammar uses PEG ordered
choice: longer/more-specific alternatives come first.

### Function Calls vs Column References

```
primary = NUMBER | STRING | "NULL" | "TRUE" | "FALSE"
        | function_call | column_ref | "(" expr ")" ;

function_call = NAME "(" ( STAR | [ value_list ] ) ")" ;
column_ref    = NAME [ "." NAME ] ;
```

`function_call` is tried before `column_ref` in `primary`. Both start with
`NAME`. The parser uses packrat memoization — `function_call` will fail and
backtrack if the next token after `NAME` is not `(`, then `column_ref` succeeds.

This ordering handles `COUNT(*)`, `SUM(salary)`, `COALESCE(a, b)`, as well as
`table.column` and `schema.table.column`-style references (the latter via two
`"." NAME` expansions from `column_ref = NAME [ "." NAME ]`).

Note: Only two-part names are supported (`table.column`). Three-part names
(`schema.table.column`) are not in this subset.

### Subquery Limitation

Subqueries (`SELECT ... FROM (SELECT ...)`) are not supported in this grammar
subset. Adding them would require:
- `table_ref` to also accept `"(" select_stmt ")" NAME`
- `primary` to accept `"(" select_stmt ")"` alongside `"(" expr ")"`

These would require disambiguating `(expr)` vs `(SELECT ...)` — possible but
out of scope for this initial implementation.

### CASE Expressions

`CASE WHEN ... THEN ... ELSE ... END` is listed in the keywords section but
not in the grammar. This is intentional — CASE expressions are complex and
out of scope for the current subset. The keywords are reserved to prevent them
from being parsed as identifiers.

---

## sql.tokens

```
# SQL Token Grammar — ANSI SQL subset
# @version 1
# @case_insensitive true
#
# Note: keyword values are normalized to uppercase when @case_insensitive
# is true. Grammar literals like "SELECT" match select/SELECT/Select.

NAME          = /[a-zA-Z_][a-zA-Z0-9_]*/
NUMBER        = /[0-9]+(\.[0-9]+)?/
STRING_SQ     = /'([^'\\]|\\.)*'/ -> STRING
QUOTED_ID     = /`[^`]+`/ -> NAME

LESS_EQUALS    = "<="
GREATER_EQUALS = ">="
NOT_EQUALS     = "!="
NEQ_ANSI       = "<>" -> NOT_EQUALS

EQUALS        = "="
LESS_THAN     = "<"
GREATER_THAN  = ">"
PLUS          = "+"
MINUS         = "-"
STAR          = "*"
SLASH         = "/"
PERCENT       = "%"

LPAREN        = "("
RPAREN        = ")"
COMMA         = ","
SEMICOLON     = ";"
DOT           = "."

keywords:
  SELECT FROM WHERE GROUP BY HAVING ORDER LIMIT OFFSET
  INSERT INTO VALUES UPDATE SET DELETE
  CREATE DROP TABLE IF EXISTS NOT
  AND OR NULL IS IN BETWEEN LIKE
  AS DISTINCT ALL UNION INTERSECT EXCEPT
  JOIN INNER LEFT RIGHT OUTER CROSS FULL ON
  ASC DESC TRUE FALSE
  CASE WHEN THEN ELSE END
  PRIMARY KEY UNIQUE DEFAULT

skip:
  WHITESPACE    = /[ \t\r\n]+/
  LINE_COMMENT  = /--[^\n]*/
  BLOCK_COMMENT = /\x2f\*([^*]|\*[^\x2f])*\*\x2f/
```

---

## sql.grammar

```
# Parser grammar for SQL (ANSI SQL subset)
# @version 1
#
# UPPERCASE identifiers reference token types from sql.tokens.
# Quoted strings match keyword values (normalized to uppercase by the lexer
# because sql.tokens uses @case_insensitive true).

program           = statement { ";" statement } [ ";" ] ;

statement         = select_stmt | insert_stmt | update_stmt
                  | delete_stmt | create_table_stmt | drop_table_stmt ;

# ── SELECT ───────────────────────────────────────────────────────────────────

select_stmt       = "SELECT" [ "DISTINCT" | "ALL" ] select_list
                    "FROM" table_ref { join_clause }
                    [ where_clause ] [ group_clause ] [ having_clause ]
                    [ order_clause ] [ limit_clause ] ;

select_list       = STAR | select_item { "," select_item } ;
select_item       = expr [ "AS" NAME ] ;

table_ref         = table_name [ "AS" NAME ] ;
table_name        = NAME [ "." NAME ] ;

join_clause       = join_type "JOIN" table_ref "ON" expr ;
join_type         = "CROSS" | "INNER" | ( "LEFT" [ "OUTER" ] )
                  | ( "RIGHT" [ "OUTER" ] ) | ( "FULL" [ "OUTER" ] ) ;

where_clause      = "WHERE" expr ;
group_clause      = "GROUP" "BY" column_ref { "," column_ref } ;
having_clause     = "HAVING" expr ;
order_clause      = "ORDER" "BY" order_item { "," order_item } ;
order_item        = expr [ "ASC" | "DESC" ] ;
limit_clause      = "LIMIT" NUMBER [ "OFFSET" NUMBER ] ;

# ── INSERT / UPDATE / DELETE ─────────────────────────────────────────────────

insert_stmt       = "INSERT" "INTO" NAME
                    [ "(" NAME { "," NAME } ")" ]
                    "VALUES" row_value { "," row_value } ;
row_value         = "(" expr { "," expr } ")" ;

update_stmt       = "UPDATE" NAME "SET" assignment { "," assignment }
                  [ where_clause ] ;
assignment        = NAME "=" expr ;

delete_stmt       = "DELETE" "FROM" NAME [ where_clause ] ;

# ── CREATE TABLE / DROP TABLE ─────────────────────────────────────────────────

create_table_stmt = "CREATE" "TABLE" [ "IF" "NOT" "EXISTS" ] NAME
                    "(" col_def { "," col_def } ")" ;
col_def           = NAME NAME { col_constraint } ;
col_constraint    = ( "NOT" "NULL" ) | "NULL" | ( "PRIMARY" "KEY" )
                  | "UNIQUE" | ( "DEFAULT" primary ) ;

drop_table_stmt   = "DROP" "TABLE" [ "IF" "EXISTS" ] NAME ;

# ── Expressions ───────────────────────────────────────────────────────────────

expr              = or_expr ;
or_expr           = and_expr { "OR" and_expr } ;
and_expr          = not_expr { "AND" not_expr } ;
not_expr          = "NOT" not_expr | comparison ;
comparison        = additive [ cmp_op additive
                  | "BETWEEN" additive "AND" additive
                  | "NOT" "BETWEEN" additive "AND" additive
                  | "IN" "(" value_list ")"
                  | "NOT" "IN" "(" value_list ")"
                  | "LIKE" additive
                  | "NOT" "LIKE" additive
                  | "IS" "NULL"
                  | "IS" "NOT" "NULL" ] ;

cmp_op            = "=" | NOT_EQUALS | "<" | ">" | "<=" | ">=" ;
additive          = multiplicative { ( "+" | "-" ) multiplicative } ;
multiplicative    = unary { ( STAR | "/" | "%" ) unary } ;
unary             = "-" unary | primary ;
primary           = NUMBER | STRING | "NULL" | "TRUE" | "FALSE"
                  | function_call | column_ref | "(" expr ")" ;

column_ref        = NAME [ "." NAME ] ;
function_call     = NAME "(" ( STAR | [ value_list ] ) ")" ;
value_list        = expr { "," expr } ;
```

---

## Known Limitations

| Feature | Status |
|---------|--------|
| Subqueries in FROM | Not supported |
| CASE expressions | Keywords reserved; no grammar rule |
| Window functions | Not supported |
| CTEs (`WITH`) | Not supported |
| Three-part names (`schema.table.column`) | Not supported |
| `UNION` / `INTERSECT` / `EXCEPT` | Keywords reserved; no grammar rule |
| `INSERT ... SELECT` | Not supported |
| Transactions (`BEGIN`, `COMMIT`, `ROLLBACK`) | Not supported |
| Schema-qualified object names | Not supported |
| Double-quoted string literals (`"value"`) | Not supported (only single-quoted) |

These limitations are intentional for the initial implementation scope. They
can be added by extending `sql.grammar` without changing `sql.tokens` (except
for `WITH`, which would require a new keyword).

---

## Relationship to Other Specs

| Spec | Relationship |
|------|-------------|
| `tokens-format.md` | `.tokens` file format — covers `# @case_insensitive true` |
| `grammar-format.md` | `.grammar` file format — PEG semantics, quantifiers, etc. |
| `02-lexer.md` | Base lexer infrastructure |
| `03-parser.md` | Base parser infrastructure |
| `F04-lexer-pattern-groups.md` | Pattern groups (not used by SQL, but available) |
