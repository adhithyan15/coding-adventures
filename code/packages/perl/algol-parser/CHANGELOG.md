# Changelog — CodingAdventures::AlgolParser (Perl)

All notable changes to this package are documented here.

## [0.01] — 2026-04-06

### Added

- Initial implementation of `CodingAdventures::AlgolParser`.
- `parse($source)` — parses an ALGOL 60 string and returns the root ASTNode
  (rule_name = "program").
- `CodingAdventures::AlgolParser::ASTNode` — blessed hashref AST node with
  accessors `rule_name`, `children`, `is_leaf`, `token`.
- Hand-written recursive-descent parser following `algol.grammar`.
- Grammar productions implemented:
  - `_parse_program` — top-level entry point
  - `_parse_block` — `BEGIN {decl SEMI} stmt {SEMI stmt} END`
  - `_parse_declaration` — dispatches to type_decl, array_decl, switch_decl,
    procedure_decl based on leading token
  - `_parse_type_decl` — `type ident_list`
  - `_parse_array_decl` — optional type prefix + `ARRAY array_segment+`
  - `_parse_switch_decl` — `SWITCH IDENT ASSIGN desig_expr+`
  - `_parse_procedure_decl` — optional type, PROCEDURE, formal_params,
    value_part, spec_parts, proc_body
  - `_parse_statement` — optional label + (cond_stmt | unlabeled_stmt)
  - `_parse_cond_stmt` — `IF bool_expr THEN unlabeled_stmt [ELSE statement]`
  - `_parse_unlabeled_stmt` — dispatches to block, for_stmt, goto_stmt,
    assign_stmt, proc_stmt, or empty_stmt
  - `_parse_assign_stmt` — `left_part+ expression`
  - `_parse_for_stmt` — `FOR IDENT ASSIGN for_list DO statement`
  - `_parse_for_elem` — step/until, while, or simple value forms
  - `_parse_expression` — delegates to arith_expr or bool_expr
  - `_parse_arith_expr` — conditional form or simple_arith
  - `_parse_simple_arith` — `[unary sign] term {(+|-) term}`
  - `_parse_term` — `factor {(*|/|div|mod) factor}`
  - `_parse_factor` — `primary {(^|**) primary}` (LEFT-associative)
  - `_parse_primary` — literals, variable, proc_call, parenthesized expr
  - `_parse_bool_expr` — conditional form or simple_bool
  - `_parse_simple_bool` — `implication {eqv implication}`
  - `_parse_implication` — `bool_term {impl bool_term}`
  - `_parse_bool_term` — `bool_factor {or bool_factor}`
  - `_parse_bool_factor` — `bool_secondary {and bool_secondary}`
  - `_parse_bool_secondary` — `not bool_secondary | bool_primary`
  - `_parse_bool_primary` — TRUE/FALSE/relation/parenthesized
  - `_parse_relation` — `simple_arith relop simple_arith`
  - `_parse_variable` — `IDENT [LBRACKET subscripts RBRACKET]`
  - `_parse_proc_call` — `IDENT LPAREN actual_params RPAREN`
  - `_parse_desig_expr` / `_parse_simple_desig` — for goto targets
- Two-token lookahead (_peek2) to disambiguate:
  - IDENT ASSIGN → assign_stmt
  - IDENT LPAREN → proc_call / proc_stmt
  - IDENT COLON → labeled statement
- Declaration-start predicate `_is_declaration_start` for block phase separation.
- Dangling-else resolved at grammar level: then-branch is unlabeled_stmt.
- Left-associative exponentiation per ALGOL 60 report.
- Descriptive die messages with line/col for all parse errors.
- `t/00-load.t` — smoke test that module loads and has a VERSION.
- `t/01-basic.t` — comprehensive test suite covering root node structure,
  minimal programs, type declarations, assignments, if/then/else, arithmetic
  expressions, for loops, ASTNode accessors, block structure, comment handling
  (via lexer), labeled statements, and error handling.
- `BUILD` and `BUILD_windows` scripts.
- `Makefile.PL` and `cpanfile`.
