# Changelog — CodingAdventures::SqlParser

## [0.01] - 2026-03-29

### Added

- Initial implementation of the hand-written recursive-descent SQL parser.
- `CodingAdventures::SqlParser` — main parser module.
  - `new($source)` — tokenizes source with `SqlLexer` and returns a parser instance.
  - `parse()` — parses the token stream and returns the root AST node (rule_name "program").
  - `parse_sql($source)` — convenience class method combining new + parse.
- `CodingAdventures::SqlParser::ASTNode` — AST node class.
  - `new($rule_name, $children)` — inner node constructor.
  - `new_leaf($token)` — leaf node wrapping a lexer token.
  - Accessors: `rule_name`, `children`, `is_leaf`, `token`.
- Supported statement types:
  - `SELECT` — with DISTINCT, column lists, STAR, FROM, JOIN (INNER/LEFT/RIGHT/FULL/CROSS),
    WHERE, GROUP BY, HAVING, ORDER BY (ASC/DESC), LIMIT/OFFSET
  - `INSERT INTO … VALUES …` — with optional column list, multiple row values
  - `UPDATE … SET … WHERE …` — single and multiple assignments
  - `DELETE FROM … WHERE …`
- Full expression support with correct operator precedence:
  - OR → AND → NOT → comparison → additive → multiplicative → unary → primary
  - Comparison operators: `=`, `!=`, `<`, `>`, `<=`, `>=`, `BETWEEN AND`,
    `IN (…)`, `NOT IN (…)`, `LIKE`, `NOT LIKE`, `IS NULL`, `IS NOT NULL`
  - Arithmetic: `+`, `-`, `*`, `/`, `%`
  - Unary minus
  - Primary: NUMBER, STRING, NULL, TRUE, FALSE, column_ref, function_call, `(expr)`
  - `column_ref` with optional `table.column` dot notation
  - `function_call` with `*` or expression argument list (e.g., `COUNT(*)`)
- Multiple statements separated by semicolons.
- Full test suite (`t/01-basic.t`) covering:
  - ASTNode inner and leaf node construction
  - Root node rule_name
  - SELECT with *, column list, DISTINCT, WHERE, AND/OR, ORDER BY, LIMIT, GROUP BY,
    HAVING, INNER JOIN, column_ref dot notation, function calls, AS aliases
  - INSERT with and without column list, multiple row values
  - UPDATE with single and multiple assignments, with and without WHERE
  - DELETE with and without WHERE
  - Expression nodes: comparison operators, BETWEEN, IN, LIKE, IS NULL,
    arithmetic, unary minus, NOT, NULL/TRUE/FALSE literals
  - Multiple semicolon-separated statements
  - Error handling: empty input, incomplete SELECT, garbage input
- `BUILD` with transitive dependency installation in leaf-to-root order:
  state-machine → directed-graph → grammar-tools → lexer → sql-lexer → sql-parser.
- `BUILD_windows` skipping Perl (not supported on Windows).
- `cpanfile` and `Makefile.PL` with all PREREQ_PM dependencies.
- `README.md` with architecture description, supported constructs, precedence table,
  and usage examples.
