# Changelog

All notable changes to the `coding-adventures-ruby-parser` crate will be documented in this file.

## [0.9.0] - 2026-05-22

### Added (Phase 6h — no-paren method calls `puts 1` / `puts 1, 2`)
- `method_call_no_paren = ( NAME | KEYWORD ) expression { COMMA expression } ;`
- Inserted in `statement` AFTER `method_call` and BEFORE `expression_stmt`.  The parenned form still wins for `puts(1)` (priority of `method_call`); bare `puts` (no args) falls through to `expression_stmt → factor → NAME`.
- Requires at least one `expression` argument so it can't shadow `expression_stmt`.
- Regenerated `src/_grammar.rs` via `grammar-tools compile-grammar`.

### Disambiguation invariants (regression-tested)
- `puts(1)` keeps matching `method_call` (parens take priority).
- `puts 1` matches `method_call_no_paren`.
- `puts 1, 2, 3` matches `method_call_no_paren` with three args.
- `puts` alone (no args) falls through to `expression_stmt`.
- `puts 1 + 2` resolves as `puts(1+2)` — the inner `expression` rule greedy-grabs the binary chain.  Matches real Ruby.

### Tests (+5 new, total 39)
- `test_parse_no_paren_single_arg` — `puts 1`.
- `test_parse_no_paren_multiple_args` — `puts 1, 2, 3` (3 args).
- `test_paren_form_still_wins_over_no_paren` — `puts(1)` matches `method_call`, NOT `method_call_no_paren`.
- `test_bare_name_falls_through_to_expression_stmt` — `puts` alone falls through.
- `test_no_paren_with_binary_arg_is_single_call` — `puts 1 + 2` is a single call with one expression arg.

## [0.8.0] - 2026-05-22

### Added (Phase 6g — blocks `do … end` and `method { … }`)
- `method_with_block = ( NAME | KEYWORD ) [ LPAREN [ expression { COMMA expression } ] RPAREN ] block ;`
- `block = do_block | brace_block ;`
- `do_block = "do" [ block_params ] { !"end" statement } "end" ;`
- `brace_block = LBRACE [ block_params ] { statement } RBRACE ;`
- `block_params = "|" NAME { COMMA NAME } "|" ;`
- `method_with_block` is inserted in `statement` **before** `method_call` and `expression_stmt` so the parser commits to the longer prefix match (call + trailing block) when a block is present, and falls through to `method_call` / `expression_stmt` otherwise.

### Disambiguation rules
- Bare `LBRACE … RBRACE` at statement position remains a **hash literal** (handled inside `expression_stmt → factor → hash_literal`), NOT a block — blocks always attach to a preceding method-name token.  Test `test_parse_hash_literal_still_works_at_statement_position` pins this invariant.
- `do_block` requires the same `!"end"` negative-lookahead as `def_statement` because `end` is a KEYWORD token and `expression_stmt → factor → KEYWORD` would otherwise greedy-match it.
- `brace_block` does NOT need an analogous `!"}"` lookahead because RBRACE isn't a `factor` alternative — the body repetition naturally stops at the closing brace.

### Tests (+6 new, total 34)
- `test_parse_method_with_do_block_no_params` — `each do\n  puts 1\nend`.
- `test_parse_method_with_brace_block` — `each { puts 1 }`.
- `test_parse_do_block_with_pipe_params` — `each do |x|\n  puts x\nend` extracts param name `x` (filter excludes `|` tokens which the lexer's `classify_op_token` reclassifies as `Name`).
- `test_parse_brace_block_with_two_pipe_params` — `each { |x, y| x + y }` extracts both params.
- `test_parse_method_call_with_args_and_block` — `each(1, 2) { puts 1 }` has both expression args and a brace_block subnode.
- `test_parse_hash_literal_still_works_at_statement_position` — `x = {a: 1}` remains a hash literal (no `brace_block` subnode appears).

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
