# Changelog — CodingAdventures::JavaParser

## [0.01] — 2026-04-11

### Added

- Initial implementation of the hand-written recursive-descent Java parser.
- `CodingAdventures::JavaParser` — main parser module.
  - `new($source, $version)` — tokenizes source with `JavaLexer` and returns a parser.
  - `parse()` — parses the token stream and returns the root AST node (rule_name "program").
  - `parse_java($source, $version)` — convenience class method combining new + parse.
- `CodingAdventures::JavaParser::ASTNode` — AST node class.
  - `new($rule_name, $children)` — inner node constructor.
  - `new_leaf($token)` — leaf node wrapping a lexer token.
  - Accessors: `rule_name`, `children`, `is_leaf`, `token`.
- Supported statement types:
  - `var_declaration` — type NAME = expression ;
  - `assignment_stmt` — NAME = expression ;
  - `expression_stmt` — expression ;
  - `if_stmt` — if (cond) block [else block/if_stmt]
  - `for_stmt` — for (init; cond; update) block
  - `return_stmt` — return [expression] ;
  - `block` — { statement* }
- Full expression support with correct operator precedence:
  - equality → comparison → additive → multiplicative → unary → primary
  - Equality: `==`, `!=`
  - Comparison: `<`, `>`, `<=`, `>=`
  - Arithmetic: `+`, `-`, `*`, `/`
  - Unary: `!`, `-`
  - Primary: NUMBER, STRING, TRUE, FALSE, NULL literals; NAME; call_expr; grouped (expr)
  - `call_expr` — method call with arg_list
  - `binary_expr` — left op right for all binary operators
  - `unary_expr` — op expr for unary operators
- Version-aware parsing: optional `$version` argument threads through to
  `CodingAdventures::JavaLexer`.
- Valid version strings: `"1.0"`, `"1.1"`, `"1.4"`, `"5"`, `"7"`, `"8"`,
  `"10"`, `"14"`, `"17"`, `"21"`. Default is `"21"`.
- Full test suite (`t/01-basic.t`) covering:
  - ASTNode inner and leaf node construction
  - Empty program
  - Variable declarations
  - Assignments
  - Expression statements
  - Expression precedence
  - Method calls
  - If/else statements
  - For loops
  - Return statements
  - Mixed multi-statement programs
  - Version-aware parsing
  - Error handling
- `BUILD` with transitive dependency paths.
- `BUILD_windows` skipping Perl.
- `cpanfile` and `Makefile.PL` with all dependencies.
- `README.md` with architecture, supported constructs, and usage examples.
