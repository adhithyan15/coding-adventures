# CodingAdventures::SqlParser

A hand-written recursive-descent SQL parser for the coding-adventures monorepo. It tokenizes SQL source with `CodingAdventures::SqlLexer` and builds an Abstract Syntax Tree (AST) of `CodingAdventures::SqlParser::ASTNode` nodes.

## What it does

Given input `SELECT name FROM users WHERE age > 18`, the parser produces:

```
program
└── statement
    └── select_stmt
        ├── SELECT token
        ├── select_list
        │   └── select_item
        │       └── expr → or_expr → and_expr → not_expr → comparison
        │           └── additive → multiplicative → unary → primary
        │               └── column_ref → NAME "name"
        ├── FROM token
        ├── table_ref → table_name → NAME "users"
        └── where_clause
            ├── WHERE token
            └── expr → or_expr → and_expr → not_expr → comparison
                ├── additive → … → column_ref → NAME "age"
                ├── cmp_op → GREATER_THAN ">"
                └── additive → … → primary → NUMBER "18"
```

## Supported SQL constructs

- `SELECT [DISTINCT|ALL] col1, col2 FROM table` — with `WHERE`, `GROUP BY`, `HAVING`, `ORDER BY`, `LIMIT`, `OFFSET`, `JOIN`
- `INSERT INTO table [(col, …)] VALUES (…)`
- `UPDATE table SET col = expr [, …] [WHERE …]`
- `DELETE FROM table [WHERE …]`
- Multiple statements separated by semicolons

## Expression support

Full operator precedence (lowest to highest):

| Level | Operators |
|-------|-----------|
| `or_expr` | `OR` |
| `and_expr` | `AND` |
| `not_expr` | `NOT` |
| `comparison` | `=` `!=` `<` `>` `<=` `>=` `BETWEEN` `IN` `LIKE` `IS NULL` |
| `additive` | `+` `-` |
| `multiplicative` | `*` `/` `%` |
| `unary` | unary `-` |
| `primary` | literals, column refs, function calls, `(expr)` |

## How it fits in the stack

```
CodingAdventures::SqlParser  ← this package (hand-written)
         ↓ uses
CodingAdventures::SqlLexer   → tokenizes SQL
         ↓ uses
CodingAdventures::GrammarTools / Lexer
```

## Usage

```perl
use CodingAdventures::SqlParser;

# Object-oriented
my $parser = CodingAdventures::SqlParser->new("SELECT * FROM users WHERE id = 1");
my $ast    = $parser->parse();
print $ast->rule_name;   # "program"

# Convenience class method
my $ast = CodingAdventures::SqlParser->parse_sql("DELETE FROM t WHERE expired = TRUE");

# Walk the AST
sub find_node {
    my ($node, $rule_name) = @_;
    return $node if $node->rule_name eq $rule_name;
    for my $child (@{ $node->children }) {
        my $found = find_node($child, $rule_name);
        return $found if $found;
    }
    return undef;
}

my $where = find_node($ast, 'where_clause');
```

## AST Node Format

Each node is a `CodingAdventures::SqlParser::ASTNode`:

```perl
$node->rule_name   # e.g. "select_stmt", "where_clause", "comparison"
$node->children    # arrayref of child ASTNode objects
$node->is_leaf     # 1 for leaf (token) nodes, 0 for inner nodes
$node->token       # token hashref (leaf nodes only): {type, value, line, col}
```

## Version

0.01
