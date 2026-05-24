# Changelog

All notable changes to the `coding-adventures-ruby-parser` crate will be documented in this file.

## [0.14.0] - 2026-05-24

### Added (Phase 6m — logical operators `&&`, `||`, `and`, `or`, `not`, `!`)

New grammar layer inserted above comparison:

```
expression   = logical_or ;
logical_or   = logical_and { ( "||" | "or" ) logical_and } ;
logical_and  = logical_not { ( "&&" | "and" ) logical_not } ;
logical_not  = { ( "!" | "not" ) } comparison ;
comparison   = sum { CMP_OP sum } ;    # was the old `expression` body
```

The pre-6m `expression` rule body (the comparison-operator chain) was moved into a new `comparison` rule.  Every previous reference to `expression` (assignment RHS, method args, if/unless cond, etc.) now automatically picks up the broader logical layer because they reference `expression`.

**Precedence**: `||`/`or` < `&&`/`and` < `!`/`not` < comparison.  This matches the *symbol* form precedence.  v0 collapses keyword forms (`and`/`or`/`not`) onto the same precedence level (real Ruby gives them lower precedence than even assignment — uncommon in modern code; if any test program depends on the difference it'll be flagged in a follow-up).

**Left-associativity**: `a || b || c` folds as `(a || b) || c`.

**`logical_not` shape**: the rule uses `{ "!" | "not" }` (zero-or-more leading operators) instead of right-recursive alternation.  Equivalent semantics (`!!x` parses as two leading `!` then a comparison), more parser-friendly.

### Parser-framework limitation discovered

`method_call_no_paren = (NAME|KEYWORD) expression …` in the `statement` alternation can swallow a `def`'s body tail expression when the body is a bare logical/binary chain like `a || b`.  Wrapping the tail in parens (`(a || b)`) is the v0 workaround.  See lessons.md for the full diagnosis and follow-up plan.

### Tests (+5 new, total 64)
- `test_parse_logical_or_symbol_form`, `test_parse_logical_and_symbol_form`, `test_parse_logical_keyword_form`, `test_parse_logical_not_prefix`, `test_parse_logical_chain_and_then_or_precedence`.
- 4 existing comparison tests (`test_parse_simple_comparison_has_sum_subnodes`, `test_parse_equality_in_assignment`, `test_parse_comparison_in_if_condition`, `test_parse_plus_has_lower_precedence_than_comparison`) updated to walk the new `comparison` subnode instead of the old `expression` body.
- `test_parse_logical_or_inside_def_body` pins the parens workaround for the def-body issue.

## [0.13.0] - 2026-05-23

### Added (Phase 6l — method receiver chains `foo.bar.baz`, `foo.bar(args)`, `foo(1).bar`)

Grammar additions in `code/grammars/ruby.grammar`:
- `dot_call = "." ( NAME | KEYWORD ) [ LPAREN [ expression { COMMA expression } ] RPAREN ] ;` — one step of a receiver chain.
- `factor`'s atom alternation is wrapped in `( … ) { dot_call }` so chains apply anywhere an expression goes.
- `method_call` grew a trailing `{ dot_call }` so statement-level `foo(1).bar` works without a parser-position trick.
- `method_call` was promoted to the **first** atom alternative inside `factor`.  Without this, `foo(1).bar` in expression position (e.g. RHS of an assignment) leaves `(1).bar` unconsumed because the bare-NAME branch only matches `foo`.

### Tests (+5 new, total 59)
- `test_parse_single_dot_call` — `foo.bar` produces one `dot_call` subnode.
- `test_parse_chained_dot_calls` — `foo.bar.baz` produces two `dot_call`s.
- `test_parse_method_call_with_dot_chain` — `foo(1).bar` has a `method_call` head and one `dot_call` tail.
- `test_parse_dot_call_with_args` — `foo.bar(1, 2)` has two `expression` direct children under the `dot_call`.
- `test_parse_chain_inside_assignment_rhs` — `x = a.b.c` parses two `dot_call`s in the RHS.

Out of scope for this chunk (will get their own phases):
- Setter calls `foo.bar = 1` (assignment LHS stays as bare NAME).
- Bracket access `arr[0]`.

## [0.12.0] - 2026-05-22

### Added (Phase 6k — unary minus `-5`, `-x`, `-(1+2)`)
- New `factor` alternative `unary_minus = MINUS factor`.  Right-recursive (`--5` parses fine); precedence tighter than binary `+ -`.

### Tests (+5 new, total 54)
- `test_parse_unary_minus_on_number`, `test_parse_unary_minus_on_name`, `test_parse_unary_minus_on_parenthesised_expression`, `test_parse_double_unary_minus_nests`, `test_parse_unary_minus_with_binary_addition`.

## [0.11.0] - 2026-05-22

### Added (Phase 6j — control-flow keywords `return` / `break` / `next`)
- `return_statement = "return" [ expression ] ;`
- `break_statement  = "break"  [ expression ] ;`
- `next_statement   = "next"   [ expression ] ;`

### Tests (+5 new, total 49)
- `test_parse_return_with_value`, `test_parse_bare_return`, `test_parse_break_with_value`, `test_parse_next_keyword`, `test_parse_return_inside_def_body`.

## [0.10.0] - 2026-05-22

### Added (Phase 6i — comparison operators `==`, `!=`, `<`, `>`, `<=`, `>=`)

Inserted a new precedence level into the expression hierarchy:
```
expression  =  sum { ( "==" | "!=" | "<=" | ">=" | "<" | ">" ) sum } ;
sum         =  term { ( PLUS | MINUS ) term } ;
term        =  factor { ( STAR | SLASH ) factor } ;
```

### Tests (+5 new, total 44)
- `test_parse_simple_comparison_has_sum_subnodes`, `test_parse_equality_in_assignment`, `test_parse_comparison_in_if_condition`, `test_parse_chained_inequality_left_associative`, `test_parse_plus_has_lower_precedence_than_comparison`.

## [0.9.0] - 2026-05-22

### Added (Phase 6h — no-paren method calls `puts 1` / `puts 1, 2`)
- `method_call_no_paren = ( NAME | KEYWORD ) expression { COMMA expression } ;`

### Tests (+5 new, total 39)
- `test_parse_no_paren_single_arg`, `test_parse_no_paren_multiple_args`, `test_paren_form_still_wins_over_no_paren`, `test_bare_name_falls_through_to_expression_stmt`, `test_no_paren_with_binary_arg_is_single_call`.

## [0.8.0] - 2026-05-22

### Added (Phase 6g — blocks `do … end` and `method { … }`)
- `method_with_block = ( NAME | KEYWORD ) [ LPAREN [ expression { COMMA expression } ] RPAREN ] block ;`
- `block = do_block | brace_block ;`
- `do_block = "do" [ block_params ] { !"end" statement } "end" ;`
- `brace_block = LBRACE [ block_params ] { statement } RBRACE ;`
- `block_params = "|" NAME { COMMA NAME } "|" ;`

### Tests (+6 new, total 34)
- `test_parse_method_with_do_block_no_params`, `test_parse_method_with_brace_block`, `test_parse_do_block_with_pipe_params`, `test_parse_brace_block_with_two_pipe_params`, `test_parse_method_call_with_args_and_block`, `test_parse_hash_literal_still_works_at_statement_position`.

## [0.7.0] - 2026-05-22

### Added (Phase 6f — `class Foo … end` / `module Foo … end` namespace declarations)
- `class_statement  = "class"  NAME { !"end" statement } "end" ;`
- `module_statement = "module" NAME { !"end" statement } "end" ;`
- Added as alternatives in `statement` right after `def_statement`, so the dispatch order is: def → class → module → if → unless → while → until → assignment → method_call → expression_stmt.
- The body's `Repetition { statement }` uses the same negative-lookahead `!"end"` as `def_statement` so the closing `end` doesn't get eaten by `expression_stmt → factor → KEYWORD`.
- Regenerated `src/_grammar.rs` via `grammar-tools compile-grammar`.

### Tests (+4 new, total 28)
- `test_parse_empty_class` — `class Foo\nend` parses and the first Name token is `"Foo"`.
- `test_parse_class_with_method_body` — `class Foo\n  def bar\n  end\nend` produces a class with at least one body `statement` (the nested `def`).
- `test_parse_empty_module` — `module M\nend` parses to a `module_statement` subnode.
- `test_parse_module_with_assignment_body` — `module M\n  x = 1\nend` keeps the body assignment under the module.

## [0.6.0] - 2026-05-20

### Added (Phase 6e — symbol literals `:foo` / `:"bar"`)
- `symbol_literal = COLON ( NAME | KEYWORD | STRING ) ;`
- Added as a `factor` alternative so symbols can appear wherever an expression is valid.
- Quoted symbols (`:"hello world"`) reuse the existing STRING token shape — the lexer strips the quotes, so the symbol's name is the inner content directly.

### Tests (+3 new, total 24)
- `:foo` (name), `:def` (keyword name — names can be Ruby keywords), `:"hello world"` (quoted).

## [0.5.0] - 2026-05-20

### Added (Phase 6d — array and hash literals)
- `array_literal = LBRACKET [ expression { COMMA expression } ] RBRACKET`
- `hash_literal  = LBRACE [ hash_entry { COMMA hash_entry } ] RBRACE`
- `hash_entry    = NAME COLON expression | expression "=>" expression`
- `factor` now accepts `array_literal` and `hash_literal` as alternatives, so they can appear anywhere an expression is valid.

### Tests (+4 new, total 21)
- Array literal `[1, 2, 3]`, empty array `[]`, hash shorthand `{a: 1, b: 2}`, hash rocket `{a => 1}`.

## [0.4.0] - 2026-05-20

### Added (Phase 6c — `while … end` / `until … end` loops)
- `while_statement = "while" expression { !"end" statement } "end"`
- `until_statement = "until" expression { !"end" statement } "end"`
- Added as alternatives in `statement` after the conditionals.
- Body uses the same `!"end"` negative-lookahead trick from Phase 6a/6b.

### Tests (+3 new, total 17)
- `while` with body, `until`, `while` with empty body.

## [0.3.0] - 2026-05-20

### Added (Phase 6b — `if … else … end` and `unless`)
- New grammar rules in `code/grammars/ruby.grammar`:

      if_statement     = "if" expression { !"else" !"elsif" !"end" statement }
                           { elsif_clause } [ else_clause ] "end"
      elsif_clause     = "elsif" expression { !"else" !"elsif" !"end" statement }
      else_clause      = "else" { !"end" statement }
      unless_statement = "unless" expression { !"else" !"end" statement }
                           [ else_clause ] "end"

- `if_statement` and `unless_statement` are added as alternatives in `statement` right after `def_statement`, so the dispatch order is: def → if → unless → assignment → method_call → expression_stmt.
- The body repetitions all use negative lookaheads (`!"end"`, `!"else"`, `!"elsif"`) so they stop short of the closing keyword — same trick as `def_statement` in Phase 6a.
- Regenerated `src/_grammar.rs`.

### Tests (+4 new, total 14)
- `if` with body, `if`/`else`, `if`/`elsif`/`else`, `unless`.

## [0.2.0] - 2026-05-20

### Added (Phase 6a — `def name(params) … end` method definitions)
- New `def_statement` grammar rule in `code/grammars/ruby.grammar`:
  - `def_statement = "def" NAME [ LPAREN [ params ] RPAREN ] { !"end" statement } "end"`
  - `params = NAME { COMMA NAME }`
- Added as the first alternative in `statement`, so `def`-led statements dispatch ahead of the `assignment` / `method_call` / `expression_stmt` branches.
- The body's `Repetition { statement }` uses a **negative lookahead** `!"end"` so it stops greedy-matching once the trailing `end` keyword comes into view.  Without the lookahead, `expression_stmt → factor → KEYWORD` would happily consume `end` itself as part of the body and leave nothing for the closing literal.
- Regenerated `src/_grammar.rs` via `grammar-tools compile-grammar`.

### Tests (+4 new, total 10)
- `test_parse_def_no_params_no_body` — `def foo()\nend`.
- `test_parse_def_with_params` — `def add(x, y)\nend` produces a `params` subnode with names `["x", "y"]`.
- `test_parse_def_with_body` — `def add(x, y)\n  x + y\nend` carries at least one body `statement`.
- `test_parse_def_without_parens` — `def foo\nend` works (parens optional per Ruby).

## [0.1.0] - 2026-03-21

### Added
- `create_ruby_parser(source)` — factory function that loads `ruby.grammar` and returns a configured `GrammarParser`.
- `parse_ruby(source)` — convenience function that parses Ruby source and returns a `GrammarASTNode`.
- Loads grammar from `ruby.grammar` using `env!("CARGO_MANIFEST_DIR")` for reliable path resolution.
- Test suite covering assignments, expressions, method definitions, if/else, while loops, multiple statements, empty programs, class definitions, method calls, and the factory function.
