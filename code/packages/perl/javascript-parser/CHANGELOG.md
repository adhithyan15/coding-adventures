# Changelog — CodingAdventures::JavascriptParser

## [0.01] - 2026-03-29

### Added

- Initial implementation of the hand-written recursive-descent JavaScript parser.
- `CodingAdventures::JavascriptParser` — main parser module.
  - `new($source)` — tokenizes source with `JavascriptLexer` and returns a parser.
  - `parse()` — parses the token stream and returns the root AST node (rule_name "program").
  - `parse_js($source)` — convenience class method combining new + parse.
- `CodingAdventures::JavascriptParser::ASTNode` — AST node class.
  - `new($rule_name, $children)` — inner node constructor.
  - `new_leaf($token)` — leaf node wrapping a lexer token.
  - Accessors: `rule_name`, `children`, `is_leaf`, `token`.
- Supported statement types:
  - `var_declaration` — var/let/const NAME = expression ;
  - `assignment_stmt` — NAME = expression ;
  - `expression_stmt` — expression ;
  - `function_decl` — function NAME(params) { body }
  - `if_stmt` — if (cond) block [else block/if_stmt]
  - `for_stmt` — for (init; cond; update) block
  - `return_stmt` — return [expression] ;
  - `block` — { statement* }
- Full expression support with correct operator precedence:
  - equality → comparison → additive → multiplicative → unary → primary
  - Equality: `===`, `!==`, `==`, `!=`
  - Comparison: `<`, `>`, `<=`, `>=`
  - Arithmetic: `+`, `-`, `*`, `/`
  - Unary: `!`, `-`
  - Primary: NUMBER, STRING, TRUE, FALSE, NULL literals; NAME; call_expr; grouped (expr); arrow_expr
  - `call_expr` — function call with arg_list
  - `arrow_expr` — (params) => expression or (params) => block
  - `binary_expr` — left op right for all binary operators
  - `unary_expr` — op expr for unary operators
- Uses correct token types from `CodingAdventures::JavascriptLexer`:
  - Keywords: VAR, LET, CONST, FUNCTION, IF, ELSE, FOR, RETURN, TRUE, FALSE, NULL
  - Operators: STRICT_EQUALS, STRICT_NOT_EQUALS, EQUALS_EQUALS, NOT_EQUALS,
    LESS_THAN, GREATER_THAN, LESS_EQUALS, GREATER_EQUALS, PLUS, MINUS, STAR, SLASH, BANG, ARROW
  - Punctuation: LPAREN, RPAREN, LBRACE, RBRACE, SEMICOLON, COMMA
- Lookahead-based disambiguation for arrow functions vs. grouped expressions.
- Full test suite (`t/01-basic.t`) covering:
  - ASTNode inner and leaf node construction
  - Empty program
  - var/let/const declarations
  - Assignments
  - Expression statements
  - Function declarations with param_list, block, return_stmt
  - If/else statements including nested if-else
  - For loops with for_init, for_condition, for_update
  - Expression precedence (`1 + 2 * 3` → 2 binary_expr nodes)
  - Comparison, equality, unary expressions
  - Parenthesized expressions
  - Arrow functions with expression body and block body
  - Function calls with and without arguments
  - Mixed multi-statement programs
  - Error handling: garbage input, incomplete declaration, missing semicolon
- `BUILD` with transitive dependency installation: state-machine → directed-graph
  → grammar-tools → lexer → javascript-lexer → javascript-parser.
- `BUILD_windows` skipping Perl (not supported on Windows).
- `cpanfile` and `Makefile.PL` with all PREREQ_PM dependencies.
- `README.md` with architecture description, supported constructs,
  precedence table, and usage examples.
