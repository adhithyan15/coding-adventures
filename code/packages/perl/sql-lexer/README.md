# CodingAdventures::SqlLexer (Perl)

A grammar-driven SQL tokenizer. Reads the shared `sql.tokens` grammar file, compiles the token definitions into Perl regexes, and tokenizes SQL source into a flat list of typed tokens.

## What it does

Given `SELECT * FROM users WHERE id = 1`, produces:

| type          | value    | line | col |
|---------------|----------|------|-----|
| SELECT        | `SELECT` | 1    | 1   |
| STAR          | `*`      | 1    | 8   |
| FROM          | `FROM`   | 1    | 10  |
| NAME          | `users`  | 1    | 15  |
| WHERE         | `WHERE`  | 1    | 21  |
| NAME          | `id`     | 1    | 27  |
| EQUALS        | `=`      | 1    | 30  |
| NUMBER        | `1`      | 1    | 32  |
| EOF           |          | 1    | 33  |

Whitespace, line comments (`-- ...`), and block comments (`/* ... */`) are consumed silently. The last token is always `EOF`.

## Token types

### Keywords (case-insensitive)

`SELECT`, `FROM`, `WHERE`, `GROUP`, `BY`, `HAVING`, `ORDER`, `LIMIT`, `OFFSET`, `INSERT`, `INTO`, `VALUES`, `UPDATE`, `SET`, `DELETE`, `CREATE`, `DROP`, `TABLE`, `IF`, `EXISTS`, `NOT`, `AND`, `OR`, `NULL`, `IS`, `IN`, `BETWEEN`, `LIKE`, `AS`, `DISTINCT`, `ALL`, `UNION`, `INTERSECT`, `EXCEPT`, `JOIN`, `INNER`, `LEFT`, `RIGHT`, `OUTER`, `CROSS`, `FULL`, `ON`, `ASC`, `DESC`, `TRUE`, `FALSE`, `CASE`, `WHEN`, `THEN`, `ELSE`, `END`, `PRIMARY`, `KEY`, `UNIQUE`, `DEFAULT`

### Values

| Token  | Example |
|--------|---------|
| NAME   | `users`, `id`, `department` |
| NUMBER | `42`, `3.14` |
| STRING | `'hello'`, `'it\'s'` |

### Operators

| Token           | Symbol  |
|-----------------|---------|
| EQUALS          | `=`     |
| NOT_EQUALS      | `!=`, `<>` |
| LESS_THAN       | `<`     |
| GREATER_THAN    | `>`     |
| LESS_EQUALS     | `<=`    |
| GREATER_EQUALS  | `>=`    |
| PLUS            | `+`     |
| MINUS           | `-`     |
| STAR            | `*`     |
| SLASH           | `/`     |
| PERCENT         | `%`     |

### Delimiters

| Token     | Symbol |
|-----------|--------|
| LPAREN    | `(`    |
| RPAREN    | `)`    |
| COMMA     | `,`    |
| SEMICOLON | `;`    |
| DOT       | `.`    |

## Usage

```perl
use CodingAdventures::SqlLexer;

my $tokens = CodingAdventures::SqlLexer->tokenize('SELECT * FROM users');
for my $tok (@$tokens) {
    printf "%s  %s  (line %d, col %d)\n",
        $tok->{type}, $tok->{value}, $tok->{line}, $tok->{col};
}
```

## How it fits in the stack

```
sql.tokens  (code/grammars/)
    ↓  parsed by CodingAdventures::GrammarTools
TokenGrammar
    ↓  compiled to Perl qr// rules
CodingAdventures::SqlLexer  ← you are here
    ↓  feeds
sql_parser  (future)
```

## SQL-specific notes

**Case-insensitive keywords** — The `sql.tokens` grammar is marked `@case_insensitive true`. Keywords like `SELECT`, `select`, and `Select` all produce a `SELECT` token. Original casing is preserved in the token value.

**Operator precedence** — Longer operators match before shorter ones: `<=` before `<`, `>=` before `>`, `!=` before no match.

**NEQ_ANSI alias** — Both `!=` and `<>` produce a `NOT_EQUALS` token.

**STRING alias** — Single-quoted strings (`'...'`) produce `STRING` tokens. Backtick-quoted identifiers produce `NAME` tokens.

## Dependencies

- `CodingAdventures::GrammarTools` — parses `sql.tokens`
- `CodingAdventures::Lexer` — general-purpose rule-driven lexer (transitive)

## Running tests

```bash
prove -l -v t/
```
