# Changelog

All notable changes to the `coding-adventures-ruby-parser` crate will be documented in this file.

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
