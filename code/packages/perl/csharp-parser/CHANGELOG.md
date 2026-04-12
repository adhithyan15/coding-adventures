# Changelog — CodingAdventures::CSharpParser (Perl)

All notable changes to this package are documented here.

## [0.01] — 2026-04-11

### Added

- Initial implementation of the hand-written recursive-descent C# parser.
- `CodingAdventures::CSharpParser` — main parser module.
  - `new($source, $version)` — tokenizes source with `CSharpLexer` and returns a parser.
  - `parse()` — parses the token stream and returns the root AST node (rule_name "program").
  - `parse_csharp($source, $version)` — convenience class method combining new + parse.
  - `new_csharp_parser($source, $version)` — alias for `new()`.
- `CodingAdventures::CSharpParser::ASTNode` — AST node class.
  - `new($rule_name, $children)` — inner node constructor.
  - `new_leaf($token)` — leaf node wrapping a lexer token.
  - Accessors: `rule_name`, `children`, `is_leaf`, `token`.
- Supported statement types:
  - `var_declaration` — type NAME = expression ;
  - `assignment_stmt` — NAME = expression ;
  - `expression_stmt` — expression ;
  - `if_stmt` — if (cond) block [else block/if_stmt]
  - `for_stmt` — for (init; cond; update) block
  - `foreach_stmt` — foreach (type name in expr) block  ← C# exclusive
  - `return_stmt` — return [expression] ;
  - `block` — { statement* }
- Full expression support with correct operator precedence:
  - null_coalesce → equality → comparison → additive → multiplicative → unary → primary
  - Null-coalescing: `??` (C# 2.0+)  ← C# exclusive
  - Equality: `==`, `!=`
  - Comparison: `<`, `>`, `<=`, `>=`
  - Arithmetic: `+`, `-`, `*`, `/`
  - Unary: `!`, `-`
  - Primary: NUMBER, STRING, TRUE, FALSE, NULL literals; NAME; call_expr; grouped (expr)
  - `call_expr` — method call with arg_list
  - `binary_expr` — left op right for all binary operators
  - `unary_expr` — op expr for unary operators
  - `null_coalesce` — left ?? right
- Version-aware parsing: optional `$version` argument threads through to
  `CodingAdventures::CSharpLexer`.
- Valid version strings: `"1.0"`, `"2.0"`, `"3.0"`, `"4.0"`, `"5.0"`,
  `"6.0"`, `"7.0"`, `"8.0"`, `"9.0"`, `"10.0"`, `"11.0"`, `"12.0"`.
  Default is `"12.0"`.
- Full test suite (`t/01-basic.t`) covering:
  - ASTNode inner and leaf node construction
  - Empty program
  - Basic C# class/variable declaration parsing
  - Variable declarations
  - Assignments
  - Expression statements
  - Expression precedence
  - C# null-coalescing operator `??`
  - Method calls
  - If/else statements
  - For loops
  - Return statements
  - Mixed multi-statement programs
  - All 12 version strings
  - Version-aware parsing
  - Error handling (unexpected tokens, missing semicolons, unknown version)
- `BUILD` with transitive dependency paths (grammar-tools + csharp-lexer).
- `BUILD_windows` skipping Perl.
- `cpanfile` and `Makefile.PL` with all dependencies.
- `README.md` with architecture, supported constructs, and usage examples.
- `required_capabilities.json`.
