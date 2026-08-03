# Changelog

All notable changes to the `coding-adventures-ruby-parser` crate will be documented in this file.

## [0.6.0] - 2026-08-03

### Fixed — bare comparison/logical statement mis-parsed as a paren-less call

`<`, `>`, `<=`, `>=`, `!=`, `&&`, `||` have no dedicated lexer token type
(`classify_op_token` in `ruby-lexer` deliberately leaves every operator
lexeme without one on `TokenType::Name` — "the parser dispatches by
value"). `factor`'s bare `NAME` alternative doesn't check a Name token's
VALUE, so `method_call_no_paren = ( NAME | ... ) expression { ... }` — the
paren-less "command call" production (`puts "hi"`) — could match a bare
statement like `x > 2` as `x` (callee) applied to `>` itself, swallowed
whole as an ordinary name-shaped argument, leaving `2` behind as an
unrelated second statement. Lowering the malformed "argument" then emitted
a call to whatever that bare name resolved to, which downstream validation
correctly rejected. Found while writing block-predicate tests for the SIR
Collections cascade (`[1,2,3].select { |x| x > 2 }` failed identically —
every backend consumes this same grammar, so the bug wasn't C-specific).

Fixed with a negative lookahead in `method_call_no_paren` (`ruby.grammar`)
for these seven operators, so the rule fails to match on them and
`expression_stmt` (`comparison`/`logical_and`/`logical_or`) parses the
whole expression correctly instead. `==` was accidentally already immune
(`classify_op_token` gives it its own dedicated `EqualsEquals` type).
`**`/bitwise `<<`/`>>`/`^`/`&`/`|` are deliberately NOT included: this
grammar has no binary-operator rule for them at all, so there is no
correct fallback parse to preserve — pinned by a regression test that they
remain unchanged.

### Added

- `src/bin/regen_grammars.rs` — a `cargo run -p coding-adventures-ruby-parser
  --bin regen_grammars` binary that regenerates `src/_grammar.rs` from
  `ruby.grammar` (mirrors `adj-lang`'s binary of the same name). Previously
  this crate had no committed regeneration tool despite `_grammar.rs`'s own
  header directing readers to one.

## [0.5.0] - 2026-07-01

(Cargo manifest minor bump 0.4.0 → 0.5.0.  Note: the older CHANGELOG headers
use a separate 0.8x sequence that had drifted from the manifest version; this
entry tracks the actual `Cargo.toml` version, matching the convention adopted
by `ruby-to-semantic-ir`.)

### Added (Issue #59 — class-method defs `def self.m` + `super` as an expression)

Two grammar limitations that blocked the just-merged OOP cascade (O1–O5) are
lifted. Both required `ruby.grammar` changes and a regenerated
`src/_grammar.rs` (via `grammar-tools generate-rust-compiled-grammars
ruby-parser`).

- **Class-method / singleton-method definitions `def self.m` / `def Recv.m`.**
  `def_statement` and `endless_def_statement` gained an OPTIONAL leading
  `def_receiver` prefix:
  `def_statement = "def" [ def_receiver ] NAME [ LPAREN [ params ] RPAREN ] … "end" ;`
  with `def_receiver = singleton_receiver "." ;` (reusing Phase 14e's
  `singleton_receiver = self | NAME`). So `def self.zero; end` and
  `def Foo.bar(x); end` now parse. The prefix is optional, so every prior
  `def m` parses byte-for-byte as before (the optional group matches nothing
  unless a `.` follows the receiver token).

- **`super` as an EXPRESSION.** A dedicated `super_expr = "super" [ super_args ]`
  production is spliced into `factor` (FIRST, before the bare `NAME`/guarded
  `KEYWORD` atoms) so `x = super`, `super + 1`, and `puts(super)` parse.
  Previously `super` was statement-only (`super_statement`), so any
  `super`-in-expression form parse-panicked.

### Changed

- **`super_statement` removed from the `statement` alternation.** All `super`
  forms now route through `factor`'s `super_expr` (a stand-alone `super` line
  parses as `expression_stmt → … → super_expr`). Keeping the old statement
  rule would have shadowed the expression form — a PEG match of
  `super_statement` consumes only `super` and cannot backtrack, orphaning a
  trailing `+ 1`. `super_args` is retained (reused by `super_expr`).
- **`method_call` / `method_call_no_paren` KEYWORD head guarded with
  `!"super"`.** A statement `super(x, y)` / `super x` must NOT match these
  rules (which would mislower it as an ordinary call to a method literally
  named `super`); the guard hands `super` to `super_expr` instead. Every other
  keyword-led call (`puts(1)`, value-keyword receivers) is unchanged.

## [0.82.0] - 2026-06-30

### Added (KW7 — keyword parameters & arguments, the Ruby-1.0 unblock)

- The `param` rule now accepts a KEYWORD parameter:
  `param = [ "*" | "**" ] NAME [ COLON [ expression ] | EQUALS expression ] ;`.
  This makes `def f(a:)` (required keyword) and `def f(a: 1)` (optional
  keyword) parse — core Ruby 2.0+ syntax that previously parse-panicked
  because `param` had no colon branch. The suffix is a three-way choice keyed
  on the token after `NAME`: `COLON` → keyword param, `EQUALS` → positional
  optional/default (P7), nothing → positional required. The two suffix
  branches are disjoint on their opening token, so lowering is unambiguous.
- The `call_arg` rule now accepts a KEYWORD argument:
  `call_arg = NAME COLON expression | [ "*" | "**" | "&" ] expression ;`.
  This makes `f(a: 1)` and `f(1, y: 2)` parse a keyword argument. The
  `NAME COLON expression` branch is listed FIRST so a bare `NAME` immediately
  followed by a colon is captured as a keyword; every other argument shape
  (positional, splat, block-pass, or an explicit brace hash `f({ a: 1 })`)
  falls through to the second branch unchanged.
- **Grammar disambiguation.** `NAME COLON` also opens a `hash_entry` (`k: v`)
  and a `hash_pattern_pair` (`k:`), but those rules are only reachable inside
  a `{ … }` hash literal or a `case/in` pattern — never inside a
  parenthesised parameter list or a call-argument position. So the new
  keyword `NAME COLON` forms never collide with hash-entry parsing.
- Updated both `code/grammars/ruby.grammar` and the embedded `src/_grammar.rs`
  (regenerated via `grammar-tools generate-rust-compiled-grammars ruby-parser`).
- Five regression tests added (279 total, up from 274) pinning the parse-tree
  shape of required/optional/mixed keyword params and keyword call args.
- Cargo crate version bumped `0.3.0` → `0.4.0`.

## [0.81.0] - 2026-06-30

### Fixed (bare-identifier block bodies swallowed their `end`)

- A block whose final statement was a bare identifier mis-parsed: the
  identifier became a `method_call_no_paren` callee and the block's closing
  `end` (a `KEYWORD` token) was consumed as that call's *argument*. The
  enclosing construct then never closed, so its node vanished entirely —
  `def f(a)\n a\nend` produced **no** `def_statement`, and the same swallow
  hit `while`/`until`/`class`/`module` bodies and `if` arms, plus the optional
  argument of `return`/`break`/`next`/`yield`.
- Root cause: the `factor` atom `KEYWORD` matched *any* reserved word, including
  structural terminators. Fix guards the bare-`KEYWORD` atom with negative
  lookaheads so it can no longer match `end`/`rescue`/`ensure`/`else`/`elsif`/
  `when`/`then`/`in`/`do`. Value keywords (`nil`/`true`/`false`/`self`) are
  untouched, so `x = nil` and `puts nil` still parse. One guard repairs every
  block body at the root. Updated `code/grammars/ruby.grammar` and the embedded
  `src/_grammar.rs` (regenerated via `grammar-tools
  generate-rust-compiled-grammars ruby-parser`).
- Six regression tests added (274 total, up from 268).
- Cargo crate version bumped `0.2.0` → `0.3.0`.

## [0.80.0] - 2026-06-30

### Added (P7 — default / optional parameters in `param`)

- The `param` rule now accepts an optional default value:
  `param = [ "*" | "**" ] NAME [ EQUALS expression ] ;`. This makes
  `def f(a = 1)`, `def f(a, b = a + 1)`, and `->(a = 1) { … }` parse — they
  previously parse-panicked at the `=` because `param` had no default branch.
  The default is an ordinary `expression`, which the PEG parser stops at the
  next `COMMA` / `RPAREN`, so `def f(a, b = a + 1, c)` does not greedily
  swallow `c`. Updated both `code/grammars/ruby.grammar` and the embedded
  `src/_grammar.rs` (regenerated via `grammar-tools
  generate-rust-compiled-grammars ruby-parser`). The `ruby-to-semantic-ir`
  frontend lowers the default subtree into `Param.default`.
- Cargo crate version bumped `0.1.0` → `0.2.0`.

## [0.79.0] - 2026-06-19

### Added (RB1 — trailing block on receiver/dotted method calls)

- `dot_call` now accepts an optional trailing `block`:
  `dot_call = "." (NAME|KEYWORD) [ LPAREN [args] RPAREN ] [ block ] ;`.
  This makes the dominant Ruby iterator idiom parse — `[1, 2].each { |x|
  … }` and `foo.bar do … end` — which previously parse-panicked because a
  block could only follow a *bare-name* call (`method_with_block`), never
  a receiver/dotted call. `_grammar.rs` regenerated from `ruby.grammar`
  via `grammar-tools`. Full parser suite unchanged (268 tests); the new
  optional block is greedy but does not regress existing hash-literal or
  chain parses.

## [0.78.0] - 2026-06-03

### Added (FC — array splat and find patterns)

The `array_pattern` grammar rule now admits splat elements (`_grammar.rs`
regenerated):

- `array_pattern = LBRACKET [ ( splat_pattern | pattern ) { COMMA
  ( splat_pattern | pattern ) } ] RBRACKET` — every position may now be a
  fixed sub-pattern **or** a splat.
- `splat_pattern = "*" [ NAME ]` — a named (`*rest`) or anonymous (`*`)
  rest element. One splat is the standard `[a, *mid, b]` form; two splats
  (`[*, x, *]`) is the *find* pattern.

New parser pins: `test_parse_array_pattern_with_named_splat`,
`test_parse_array_find_pattern_two_splats`,
`test_parse_array_anonymous_splat`. (Lowering lives in
`ruby-to-semantic-ir` 0.89.0.)

## [0.77.0] - 2026-06-03

### Added (FC — pin `^x` and class `Foo(x)` patterns)

Two new `case/in` pattern forms in the grammar (`_grammar.rs`
regenerated):

- `pin_pattern = "^" NAME` — `in ^x` matches when the scrutinee equals
  the value of an already-bound local.
- `class_pattern = NAME LPAREN [ pattern { COMMA pattern } ] RPAREN` —
  `in Foo(a, b)` matches an instance of `Foo` whose deconstructed
  positional elements match the inner patterns. Placed in the `pattern`
  alternation **before** `binding_pattern` so the `NAME LPAREN` form wins
  while a bare constant `Foo` still parses as a binding.

New parser pins: `test_parse_pin_pattern`, `test_parse_class_pattern`,
`test_parse_bare_constant_is_binding_not_class_pattern`. (Lowering lives
in `ruby-to-semantic-ir` 0.88.0.)

## [0.76.0] - 2026-06-01

### Added (Phase 26b (FC) — `refine Class do … end` parse pins)

Regression pins confirming Ruby's `refine(Class) do … end` (refinement
body definition) parses with **no grammar change**: `refine` is an
ordinary block-taking method, so `refine(String) do … end` parses as a
`method_with_block` with the target class as a parenned argument and the
refinement body as a `block` subnode.  The feature's semantics (lowering
to a PURE `BuiltinCall` with the block hoisted to a `MakeClosure`, instead
of an undeclared `DirectCall`) are supplied entirely in the lowerer (see
the `ruby-to-semantic-ir` 0.84.0 entry); this crate's grammar is
unchanged, so these are pure parse-shape pins.  New tests:
`test_parse_refine_is_method_with_block`,
`test_parse_refine_carries_callee_and_class`,
`test_parse_refine_has_block_subnode`.

This completes the Ruby 3.4 refinement surface (`using` + `refine`) and
the full-coverage frontend convergence.

## [0.75.0] - 2026-06-01

### Added (Phase 26a (FC) — `using Mod` parse pins)

Regression pins confirming Ruby's `using Mod` refinement-activation
statement parses with **no grammar change**: `using` is an ordinary
method (not a keyword), so `using Foo` / `using Foo::Bar` parse as a
`method_call_no_paren` with the refinement module as the sole argument.
The feature's semantics (lowering to a PURE `BuiltinCall` instead of an
undeclared `DirectCall`) are supplied entirely in the lowerer (see the
`ruby-to-semantic-ir` 0.83.0 entry); this crate's grammar is unchanged,
so these are pure parse-shape pins.  New tests:
`test_parse_using_is_method_call_no_paren`,
`test_parse_using_carries_callee_and_module`,
`test_parse_using_scoped_module`.

## [0.74.0] - 2026-06-01

### Added (Phase 23d (FC) — `__dir__` parse pins)

Regression pins confirming Ruby's `__dir__` pseudo-variable parses with
**no grammar change** (sibling of Phase 23a `__FILE__` / 23c `__LINE__`):
`__dir__` is not a lexer keyword — it arrives as an ordinary `NAME` token
matched by `factor`'s existing bare-`NAME` alternative in every
expression position (standalone statement, call argument, assignment
RHS).  The feature's semantics are supplied entirely at lowering time
(see the `ruby-to-semantic-ir` 0.82.0 entry); this crate's grammar is
unchanged, so these are pure parse-shape pins.  New tests:
`test_parse_dir_keyword_as_factor`, `test_parse_dir_keyword_in_call_arg`,
`test_parse_dir_keyword_in_assignment_rhs`.

## [0.73.0] - 2026-06-01

### Added (Phase 23c (FC) — `__LINE__` parse pins)

Regression pins confirming Ruby's `__LINE__` pseudo-variable parses with
**no grammar change** (sibling of Phase 23a `__FILE__`): because
`__LINE__` begins with `_` it is not a lexer keyword — it arrives as an
ordinary `NAME` token matched by `factor`'s existing bare-`NAME`
alternative in every expression position (standalone statement, call
argument, assignment RHS).  The feature's semantics are supplied entirely
at lowering time (see the `ruby-to-semantic-ir` 0.81.0 entry); this
crate's grammar is unchanged, so these are pure parse-shape pins.  New
tests: `test_parse_line_keyword_as_factor`,
`test_parse_line_keyword_in_call_arg`,
`test_parse_line_keyword_in_assignment_rhs`.

## [0.72.0] - 2026-06-01

### Added (Phase 23a (FC) — `__FILE__` parse pins)

Regression pins confirming Ruby's `__FILE__` pseudo-variable parses with
**no grammar change**: because `__FILE__` begins with `_` it is not a
lexer keyword — it arrives as an ordinary `NAME` token and is matched by
`factor`'s existing bare-`NAME` alternative in every expression position
(standalone statement, call argument, assignment RHS).  The feature's
semantics are supplied entirely at lowering time (see the
`ruby-to-semantic-ir` 0.80.0 entry); this crate's grammar is unchanged,
so these are pure parse-shape pins.  New tests:
`test_parse_file_keyword_as_factor`, `test_parse_file_keyword_in_call_arg`,
`test_parse_file_keyword_in_assignment_rhs`.

## [0.71.0] - 2026-06-01

### Added (Phase 24b (FC) — `undef name` method removal)

New grammar rule `undef_statement = "undef" NAME`, added to the
`statement` alternation right after `alias_statement` (i.e. BEFORE
`method_call`/`expression_stmt`).  Like `alias`, the lexer already
classifies `undef` as a Ruby `KEYWORD` (matched here by value), so no
lexer change was needed.  Placement matters for the same reason the
Phase 23b `defined?` and Phase 24a `alias` rules document: a bare
leading `undef` would otherwise be matched by the `KEYWORD` alternative
of `factor` (via `expression_stmt`), consuming only `undef` and leaving
the name operand dangling.  `_grammar.rs` regenerated.

This first slice covers the canonical single-bare-name form (`undef
foo`); the symbol form (`undef :name`) and the multi-name form (`undef
a, b`) are deliberate follow-ups.  New parser pins:
`test_parse_undef_basic`, `test_parse_undef_carries_name`,
`test_parse_undef_not_shadowed_by_method_call`.

## [0.70.0] - 2026-06-01

### Added (Phase 24a (FC) — `alias new old` method aliasing)

New grammar rule `alias_statement = "alias" NAME NAME`, added to the
`statement` alternation BEFORE `method_call`/`expression_stmt`.  The
lexer already classifies `alias` as a Ruby `KEYWORD` (matched here by
value), so no lexer change was needed.  Placement matters for the same
reason the Phase 23b `defined?` rule documents: a bare leading `alias`
would otherwise be matched by the `KEYWORD` alternative of `factor` (via
`expression_stmt`), consuming only `alias` and leaving the two name
operands dangling.  `_grammar.rs` regenerated.

This first slice covers the canonical two-bare-name form (`alias foo
bar`); the symbol operand forms (`alias :new :old`) are a deliberate
follow-up.  New parser pins: `test_parse_alias_basic`,
`test_parse_alias_carries_both_names`,
`test_parse_alias_not_shadowed_by_method_call`.

## [0.69.0] - 2026-06-01

### Added (Phase 23b (FC) — `defined?` operator)

New grammar rule `defined_expression = "defined?" factor`, added as the
first alternative of `factor` (so the `defined?` keyword wins over the
bare `KEYWORD` alternative that would otherwise leave the operand
unconsumed) and also to the `statement` alternation before `method_call`
(so a bare `defined?(x)` statement parses as `defined_expression` rather
than being swallowed as a `method_call` to a callee named `defined?`).
The lexer already emits `defined?` as a single `KEYWORD` token (trailing
`?` included), matched here by value. `_grammar.rs` regenerated.

Covers both `defined?(x)` (operand = `LPAREN expression RPAREN`) and the
bare tight form `defined? x` (operand = NAME). New parser pins:
`test_parse_defined_with_parens`, `test_parse_defined_without_parens`,
`test_parse_defined_statement_position`. Test count: 238 → 241.

## [0.68.0] - 2026-05-31

### Added (Phase 21c (FC) — implicit `it` block parameter, Ruby 3.4)

No grammar change.  A header-less block may use a bare `it` in its body
as the first block argument.  Parser-side, `it` lexes as a plain `Name`
token (it is not a Ruby keyword), so such blocks already parse — these
pins confirm it.

New parse pins (+3): `test_parse_block_with_implicit_it_parses`
(`each { puts(it) }` — no block_params, `it` token present),
`test_parse_do_block_with_implicit_it_parses`
(`each do\n puts(it)\nend`), `test_parse_block_with_it_dot_method_parses`
(`each { puts(it.foo) }` — `it` as receiver).  Test count: 235 → 238.

## [0.67.0] - 2026-05-31

### Added (Phase 21b (FC) — implicit numbered block parameters `_1`..`_9`)

No grammar change.  A block with NO explicit `|...|` header may use
`_1`..`_9` in its body as positional parameters.  Parser-side, `_1`
lexes as a plain `Name` token (the lexer flags it with
`NUMBERED_BLOCK_PARAM_FLAG` but keeps the type), so such blocks already
parse — these pins confirm it.

New parse pins (+3): `test_parse_block_with_numbered_param_parses`
(`each { puts(_1) }` — no block_params, `_1` token present),
`test_parse_block_with_two_numbered_params_parses`
(`each { puts(_1 + _2) }`), `test_parse_do_block_with_numbered_param_parses`
(`each do\n puts(_1)\nend`).  Test count: 232 → 235.

## [0.66.0] - 2026-05-31

### Added (Phase 21a (FC) — block-local variables `{ |x; y| … }`)

The `block_params` rule now accepts an optional `;`-introduced list of
block-local variables after the regular parameters:

```
block_params = "|" NAME { COMMA NAME } [ ";" NAME { COMMA NAME } ] "|" ;
```

`{ |x; y, z| … }` declares `x` as a block parameter and `y`, `z` as
fresh block-local variables scoped to the body.  The `;` is a
`Semicolon` token; the lexer already emits it, so only the grammar rule
(and a regen of `_grammar.rs`) was needed on the parser side.

New parse pins (+3): `test_parse_brace_block_with_one_block_local`
(`{ |x; y| x }` — Semicolon token + names `x`,`y`),
`test_parse_do_block_with_two_block_locals` (`do |a; b, c|` — names
`a`,`b`,`c`), `test_parse_block_without_locals_has_no_semicolon`
(regression: plain `|x, y|` has no Semicolon).  Test count: 229 → 232.

## [0.65.0] - 2026-05-31

### Added (Phase 11d (FC) — `return` WITH VALUE, coverage-confirmation)

No grammar change.  The Phase 6j rule already accepts an optional
trailing expression after `return`:

```
return_statement = "return" [ expression ] ;
```

The pre-existing pins covered `return 42`, bare `return`, and
`return x + 1` inside a def.  These pins add new payload shapes from
fresh angles so a future grammar edit cannot silently drop
value-carrying returns.

New parse pins (+3): `test_parse_return_with_array_value` (`return [1, 2]`),
`test_parse_return_with_string_value` (`return "ok"`),
`test_parse_return_with_paren_value` (`return (1 + 2)`, `+` token
survives).  Test count: 226 → 229.

## [0.64.0] - 2026-05-31

### Added (Phase 11c (FC) — `retry` keyword)

New `retry_statement` rule, added to the `statement` alternation right
after `redo_statement`:

```
retry_statement = "retry" ;
```

`retry` is a Ruby keyword (lexer-tagged KEYWORD) that re-executes the
enclosing `begin` block from the top, inside a `rescue` clause.  Like
`redo`, it is a bare keyword that never carries a value.  `_grammar.rs`
regenerated.

New parse pins (+3): `test_parse_retry_bare` (`retry` → `retry_statement`),
`test_parse_retry_has_no_expression_child` (no `expression` subnode),
`test_parse_retry_inside_begin_rescue_body`
(`begin; x = 1; rescue; retry; end` — nests in a rescue clause).
Test count: 223 → 226.

## [0.63.0] - 2026-05-31

### Added (Phase 11b (FC) — `redo` keyword)

New `redo_statement` rule, added to the `statement` alternation right
after `next_statement`:

```
redo_statement = "redo" ;
```

`redo` is a Ruby keyword (lexer-tagged KEYWORD) that restarts the
current loop iteration WITHOUT re-evaluating the loop condition or
advancing the iterator.  Unlike `break`/`next`, it never carries a
value — it is a bare keyword with no optional trailing expression.
`_grammar.rs` regenerated.

New parse pins (+3): `test_parse_redo_bare` (`redo` → `redo_statement`),
`test_parse_redo_has_no_expression_child` (no `expression` subnode),
`test_parse_redo_inside_while_body` (`while x; redo; end` — nests in a
loop body).  Test count: 220 → 223.

## [0.62.0] - 2026-05-31

### Added (Phase 11a (FC) — `break`/`next` WITH VALUES, coverage-confirmation)

No grammar change.  The Phase 6j rules already accept an optional
trailing expression after the loop-control keywords:

```
break_statement = "break" [ expression ] ;
next_statement  = "next"  [ expression ] ;
```

This release adds parse pins from new angles so a future grammar edit
cannot silently drop value-carrying loop control.  The pre-existing
pins covered `break 1 + 2` and a bare `next`; the new pins cover a bare
integer payload, a value-carrying `next`, and a name-plus-literal binary
payload whose `+` token must survive inside the `break_statement`
subtree.

New parse pins (+3): `test_parse_break_with_int_value` (`break 5`),
`test_parse_next_with_value` (`next 7`),
`test_parse_break_with_binary_name_value` (`break x + 1`).  Test count:
217 → 220.

## [0.61.0] - 2026-05-31

### Added (Phase 22d (FC) — `super` keyword)

New `super_statement` rule (and a `super_args` rule mirroring
`yield_args`), added to the `statement` alternation right after
`yield_statement`:

```
super_statement = "super" [ super_args ] ;
super_args = LPAREN [ call_arg { COMMA call_arg } ] RPAREN
           | call_arg { COMMA call_arg } ;
```

`super` is a Ruby keyword (lexer-tagged KEYWORD), and `super_statement`
is placed before the `method_call*` family so the keyword token gets its
dedicated rule instead of falling through to `method_call_no_paren`.
Three surface forms parse: bare `super` (no `super_args` — the
implicit-forward "zsuper"), `super()` (explicit empty `super_args`), and
`super(x, y)` / `super x` (explicit args).  `super_args` reuses
`call_arg`, so splat / double-splat / block-pass / `...` forwarding ride
through uniformly.  `_grammar.rs` regenerated.

New parse pins (+3): `test_parse_super_bare` (`super`, no `super_args`),
`test_parse_super_empty_parens` (`super()`, 0 call_args),
`test_parse_super_with_args` (`super(x, y)`, 2 call_args).  Test count:
214 → 217.

## [0.60.0] - 2026-05-31

### Added (Phase 22c (FC) — `...` argument forwarding)

Ruby 2.7+ argument forwarding: `def m(...)` declares a method that
forwards every argument, and `n(...)` forwards them to an inner call.

Grammar change — `params` only:

```
params = "..." | param { COMMA param } ;
```

The lexer fuses `...` into a single **Name-typed** token (value `...`,
shared with the exclusive-range operator).  Because `param = [ "*" |
"**" ] NAME` would otherwise match `...` as a parameter *named* `...`,
the bare `"..."` alternative is listed FIRST so it claims the token as a
literal forwarding marker.  Ordinary signatures are unaffected (their
first token is never `...`).  `_grammar.rs` regenerated.

**No `call_arg` grammar change.**  `factor`'s `NAME` alternative already
matches the `...` token as a bare-name expression, so `n(...)` parses as
a call_arg whose expression is the bare name `...`.  A beginless
exclusive-range argument `m(...5)` instead parses as a `range` (the
`... 5` form), keeping the two disjoint at the parse-tree level — a
literal `"..."` call_arg alternative was deliberately NOT added (listing
it first breaks `m(...5)`, since the parser does not backtrack across the
call_arg boundary once `...` is consumed; listing it last is dead code
because `NAME` shadows it).

New parse pins (+4): `test_parse_forward_all_call_arg` (`n(...)`),
`test_parse_forward_all_param` (`def m(...)` → 0 `param` nodes),
`test_parse_forward_all_roundtrip` (`def m(...) ; n(...) ; end`),
`test_parse_beginless_range_arg_still_parses` (`m(...5)` regression —
still a `range`).  Test count: 210 → 214.

## [0.59.0] - 2026-05-31

### Added (Phase 22b (FC) — `&blk` block-pass call argument)

Grammar change: `call_arg` gained a `&` alternative in its optional
prefix —

```
call_arg = [ "*" | "**" | "&" ] expression ;
```

so a block-pass argument (`f(&blk)`, `arr.each(&blk)`, `f(1, &blk)`)
parses in both head `method_call` and `dot_call` argument lists, the
same slots that already carry `*` splat and `**` double-splat. `_grammar.rs`
regenerated.

No ambiguity: the lexer emits a lone `&` as a Name-typed Op token (the
`&.` safe-nav fusion only fires when a `.` immediately follows), and
there is no binary `&` rule anywhere in the expression hierarchy
(`logical_or → logical_and → … → factor`, no bitwise layer), so a
leading `&` in a `call_arg` is unambiguous.

New parse pins (+3): `test_parse_block_pass_call_arg` (`f(&blk)`),
`test_parse_block_pass_after_positional` (`f(1, &blk)`),
`test_parse_block_pass_in_dot_call` (`arr.each(&blk)`).  Test count:
207 → 210.

## [0.58.0] - 2026-05-31

### Added (Phase 22a (FC) — `**` double-splat call argument, coverage)

No grammar change.  The `call_arg = [ "*" | "**" ] expression ;` rule
(Phase 6s) already admits a `**` double-splat prefix in both head
`method_call` argument lists and `dot_call` argument lists.  This phase
locks the parse shape with three new-angle pins that earlier `**`
coverage never exercised:

- `test_parse_double_splat_only_call_arg` (`f(**opts)`) — a lone `**`
  arg with no positional/splat siblings (prior pins only ran `**`
  alongside `f(1, *arr, **hsh)`).
- `test_parse_double_splat_hash_literal_inner` (`f(**{a: 1})`) — the
  double-splat operand is itself a `hash_literal`, confirming the
  `call_arg` expression slot accepts a brace literal after `**`.
- `test_parse_double_splat_in_dot_call` (`obj.merge(**opts)`) — the
  `**` rides through a `dot_call` argument list, a distinct grammar
  path from the head-call args.

Test count: 204 → 207.

## [0.57.0] - 2026-05-31

### Added (Phase 19d (FC) — `%r{...}` regex literal)

No grammar change.  The lexer's `percent_r_body` state emits the whole
`%r{...}` literal as a single `TokenType::String` token carrying the
verbatim source (the `%r` + braces preserved) — the `%`-family
sentinel-by-prefix trick `%w`/`%q`/`%i` use.  The parser routes it
through the ordinary string-literal slot.

New parse pins (+3): `test_parse_percent_r_regex_literal` (`%r{hello}`),
`test_parse_percent_r_regex_empty` (`%r{}`),
`test_parse_percent_r_regex_in_call_argument` (`foo(%r{bar})`).  Test
count: 201 → 204.

## [0.56.0] - 2026-05-31

### Added (Phase 19c (FC) — regex interpolation `/a#{b}c/`)

No grammar or lexer change.  The `regex_body` lexer state does not
special-case `#{...}` — it accumulates the markers verbatim into the
regex body — so an interpolated regex still arrives as ONE
`TokenType::String` token whose value includes the markers
(`/a#{b}c/`).  The parser routes it through the ordinary string-literal
slot exactly as for a non-interpolated regex.

New parse pins (+3): `test_parse_regex_literal_with_interpolation`
(`/a#{b}c/`), `test_parse_regex_interpolation_single_marker` (`/#{b}/`),
`test_parse_regex_interpolation_with_flags` (`/x#{b}/i`) — each confirms
the verbatim interpolated lexeme survives into the parse tree.  Test
count: 198 → 201.

## [0.55.0] - 2026-05-31

### Added (Phase 19b (FC) — regex flags `/r/i` coverage confirmation)

No grammar change.  Regex flags already ride along in the verbatim
`/pattern/flags` lexeme (Phase 19a), so 19b is a coverage-confirmation
phase (cf. 16b/16c) pinning MULTI-flag lexemes (the 19a tests only
covered a single `i`).

New parse pins (+3): `test_parse_regex_literal_multi_flag` (`/foo/im`),
`test_parse_regex_literal_three_flags` (`/a/mix`),
`test_parse_regex_literal_multi_flag_in_call_argument` (`foo(/bar/im)`).
Test count: 195 → 198.

## [0.54.0] - 2026-05-31

### Added (Phase 19a (FC) — regex literal `/pattern/flags`)

No grammar change.  The lexer already resolves the classic
`/`-is-regex-vs-division ambiguity (`should_open_regex` / the
`regex_body` sub-machine) and emits a regex as a `TokenType::String`
token carrying the verbatim `/pattern/flags` source (slashes included)
— the same lexeme-prefix sentinel trick percent literals, heredocs, and
backticks use.  The parser therefore routes a regex through the ordinary
string-literal slot.

New parse pins (+3): `test_parse_regex_literal` (`x = /foo/`),
`test_parse_regex_literal_with_flags` (`x = /foo/i`),
`test_parse_regex_literal_in_call_argument` (`foo(/bar/)`) — each
confirms the verbatim regex lexeme survives into the parse tree.  Test
count: 192 → 195.

## [0.53.0] - 2026-05-31

### Added (Phase 10d (FC) — beginless range `..5` / `...5`)

The `range` rule gained a FIRST alternative that leads with the op,
enabling beginless ranges (an end with no start):

- `range = ( "..." | ".." ) logical_or
          | logical_or [ ( "..." | ".." ) [ logical_or ] ] ;` — the two
  alternatives are disjoint on their first token (`..`/`...` vs a
  `logical_or`), so there is no ambiguity and no backtracking hazard.
  The beginless alt requires a trailing operand (the end).
- Regenerated `_grammar.rs`.

New parse pins (+3): `test_parse_beginless_range_inclusive` (`x = ..5`),
`test_parse_beginless_range_exclusive` (`(...5)`),
`test_parse_beginless_range_parenthesized` (`(..5)`).  (A bare leading
`..` at statement start is a separate dispatch quirk — like the
bare-NAME quirk — so the pins use expression positions.)  Test count:
189 → 192.

## [0.52.0] - 2026-05-31

### Added (Phase 10c (FC) — endless range `1..` / `1...`)

The `range` rule's trailing operand is now **optional**, enabling
endless ranges (a start with no end):

- `range = logical_or [ ( "..." | ".." ) [ logical_or ] ] ;` — when the
  range op is present but the next token is a closer (`)`, `]`, `,`,
  newline, EOF) that cannot begin a `logical_or`, the inner optional
  matches nothing and the node carries one operand plus the op token.
  `1..5` still binds two operands; a bare `logical_or` still passes
  through.  Beginless ranges (`..5`) remain deferred to Phase 10d.
- Regenerated `_grammar.rs`.

New parse pins (+3): `test_parse_endless_range_inclusive` (`1..`),
`test_parse_endless_range_exclusive` (`1...`),
`test_parse_endless_range_parenthesized` (`(1..)`).  Each asserts the
range node carries exactly one `logical_or` operand.  Test count:
186 → 189.

## [0.51.0] - 2026-05-31

### Added (Phase 10a (FC) — inclusive range `1..5` coverage confirmation)

Inclusive ranges were first implemented in **Phase 6n** (the `range`
rule `range = logical_or [ ( "..." | ".." ) logical_or ]`, with the
lexer's `fuse_range_ops` folding two adjacent `Dot` tokens into a single
`Name("..")`).  No grammar change is required for Phase 10a — like
Phases 16b/16c it is a coverage-confirmation phase that pins inclusive
ranges in syntactic positions the original 6n tests did not cover.

New parse pins (+3):

- `test_parse_inclusive_range_string_endpoints` — `"a".."z"` (string
  literal endpoints; the two Dots after a String still fuse to `..`).
- `test_parse_inclusive_range_as_call_argument` — `foo(1..5)` (range as
  a method-call argument).
- `test_parse_inclusive_range_parenthesized` — `(1..5)` (parenthesized
  range still parses to a `range` node carrying `..`).

Test count: 183 → 186.

## [0.50.0] - 2026-05-30

### Added (Phase 16e (FC) — method-level rescue/ensure)

`def_statement` now accepts trailing `rescue`/`ensure` clauses WITHOUT an
explicit `begin` — the whole method body is the protected region:

- `def_statement = "def" NAME [ LPAREN [ params ] RPAREN ]
  { !"rescue" !"ensure" !"end" statement } { rescue_clause }
  [ ensure_clause ] "end" ;` — the body repetition gains the same
  `!"rescue" !"ensure"` negative-lookahead as `begin_statement`, then the
  shared `rescue_clause` / `ensure_clause` rules.
- Regenerated `_grammar.rs`.

New parse pins (+3): `test_parse_def_with_method_level_rescue`,
`test_parse_def_with_method_level_ensure`,
`test_parse_def_with_typed_rescue_and_ensure`.  Test count: 180 → 183.

## [0.49.0] - 2026-05-30

### Added (Phase 16d (FC) — `raise` / `raise Foo` / `raise Foo, "msg"`)

No grammar change — `raise` is not a keyword; `raise Foo, "msg"` parses
as a paren-less method call (`method_call_no_paren`) and bare `raise` as
an expression.  Phase 16d adds a parse pin:

- `test_parse_raise_with_class_and_message` — `raise Foo, "boom"` parses
  as a `method_call_no_paren` carrying the `raise` head and both args.

Test count: 179 → 180 (+1).

## [0.48.0] - 2026-05-30

### Added (Phase 16c (FC) — `ensure` clause coverage)

No grammar change — the `ensure_clause` rule already parses (Phase 6v),
and the Phase 16a `Stmt::TryCatch.ensure_body` lowering consumes it.
Phase 16c adds a parse pin for the multi-statement ensure body:

- `test_parse_begin_ensure_multiple_statements` — an `ensure` clause with
  several statements collects them all under the `ensure_clause` node.

(Ensure-only and rescue+ensure shapes are already covered by
`test_parse_begin_with_ensure` / `test_parse_begin_with_rescue_and_ensure`.)
Test count: 178 → 179 (+1).

## [0.47.0] - 2026-05-30

### Added (Phase 16b (FC) — typed / multi-type / multi-clause rescue)

No grammar change — the `rescue_clause` /  `exception_list` rules
(`rescue ExceptionType[, OtherType] => var`) already parse these forms
(Phase 6v).  Phase 16b adds parse pins for the typed and multi-clause
shapes that the Phase 16a `Stmt::TryCatch` lowering relies on:

- `test_parse_begin_multi_type_rescue` — `rescue Foo, Bar => e` parses
  with an `exception_list` carrying both class Name tokens plus the `=>`
  binding.
- `test_parse_begin_multiple_rescue_clauses` — two `rescue` clauses
  parse as two distinct `rescue_clause` nodes under the begin_statement.

Test count: 176 → 178 (+2).

## [0.46.0] - 2026-05-30

### Added (Phase 15d (FC) — scope-resolution operator `Foo::Bar`)

The grammar gains a `scope_resolution` postfix so constant paths parse:

- New rule `scope_resolution = "::" ( NAME | KEYWORD ) ;`, appended to
  `factor`'s postfix alternation (`{ dot_call | scope_resolution }`), so
  `Foo::Bar` parses as `(Foo)::Bar` and `A::B::C` as a chain of two
  `::` steps.  The `::` literal matches by VALUE (the lexer emits `::`
  as a `Colon`-typed token whose value is `"::"`).
- **Disambiguation fix**: `symbol_literal` previously matched the
  `COLON` token by TYPE, which let it swallow the `::` of a scope
  resolution (so `Foo::Bar` mis-parsed as `Foo(:Bar)`).  It now matches
  the literal `":"` (by value), keeping symbols to the single-colon form
  and freeing `::` for `scope_resolution`.  All existing symbol tests
  still pass (a lone `:` lexes as `Colon` value `":"`).

New parse pins (+3): `test_parse_scope_resolution_foo_bar`,
`test_parse_scope_resolution_chain` (`A::B::C` → two steps),
`test_parse_scope_resolution_then_dot_call` (`Foo::Bar.baz` mixes both
postfix kinds).  Test count: 173 → 176 (+3).

## [0.45.0] - 2026-05-30

### Added (Phase 15c (FC) — constants `FOO` / `MyClass`)

No grammar change — an uppercase-initial identifier already lexes as a
`Name` token, so `MAX = 10` parses as an `assignment` and a bare `MAX`
as an expression.  Phase 15c adds one parse pin for the new lowering
path:

- `test_parse_constant_assignment` — `MAX = 10` parses as an assignment
  carrying the uppercase-initial `MAX` Name token.

Test count: 172 → 173 (+1).

## [0.44.0] - 2026-05-30

### Added (Phase 15b (FC) — class variables `@@x`)

No grammar change — the lexer already emits `@@x` as a `Name` token
(sigil included, since Phase 4i/4j), so `@@x = 1` parses as an
`assignment` and a bare `@@x` as an expression.  Cvar assignment and
expression parsing are already covered by the Phase 6x tests
(`test_parse_class_var_assignment`, etc.).  Phase 15b adds one parse pin
for the new lowering path:

- `test_parse_class_var_compound_assignment` — `@@n += 1` parses as an
  assignment carrying the `@@n` Name token and the fused `+=` operator
  token.

Test count: 171 → 172 (+1).

## [0.43.0] - 2026-05-30

### Added (Phase 15a (FC) — instance variables `@x`)

No grammar change — the lexer already emits `@x` as a `Name` token
(sigil included, since Phase 4i/4j), so `@x = 1` parses as an
`assignment` and a bare `@x` as an expression.  Ivar assignment and
expression parsing are already covered by the Phase 6x tests
(`test_parse_instance_var_assignment`,
`test_parse_instance_var_in_expression`).  Phase 15a adds one parse pin
for the new lowering path:

- `test_parse_instance_var_compound_assignment` — `@n += 1` parses as an
  assignment carrying the `@n` Name token and the fused `+=` operator
  token.

Test count: 170 → 171 (+1).

## [0.42.0] - 2026-05-30

### Added (Phase 14e (FC) — singleton class `class << receiver … end`)

Grammar change: `class_statement` gains a singleton alternative, listed
first so PEG tries it before the ordinary form:

```
class_statement   = "class" "<<" singleton_receiver { !"end" statement } "end"
                  | "class" NAME [ "<" NAME ] { !"end" statement } "end" ;
singleton_receiver = "self" | NAME ;
```

`<<` matches the left-shift Op token by value (`class << self` lexes as
`class`, `<<`, `self` — the space before `self` keeps `<<` a shift
operator, not a heredoc opener).  `_grammar.rs` regenerated via
`grammar-tools compile-grammar`.

New parser tests (+3): `test_parse_singleton_class_of_self`,
`test_parse_singleton_class_body_with_def`,
`test_parse_ordinary_class_has_no_singleton_receiver` (regression guard
that the singleton alternative doesn't shadow `class Foo`).  Test
count: 167 → 170 (+3).

## [0.41.0] - 2026-05-30

### Added (Phase 14d (FC) — `module M … end` parser coverage)

No grammar change — `module_statement = "module" NAME { !"end"
statement } "end"` already accepts a module body.  Phase 14d adds
parser coverage for the parse properties the lowerer relies on (name
extraction, def/non-def body children):

- `test_parse_module_name_is_first_name_token` — the module name is the
  first Name token (the `module` keyword is a Keyword-type token).
- `test_parse_module_body_with_def` — a `def` inside a module parses as
  a `def_statement` body child.
- `test_parse_module_body_mixes_def_and_assignment` — the body holds
  both an `assignment` and a `def_statement`.

Test count: 164 → 167 (+3).

## [0.40.0] - 2026-05-30

### Added (Phase 14c (FC) — inheritance `class Foo < Bar`)

Grammar change: `class_statement` gains an optional superclass clause:

```
class_statement = "class" NAME [ "<" NAME ] { !"end" statement } "end" ;
```

The `"<"` literal matches by *value* — the lexer reclassifies `<` to a
`Name`-type token (the comparison-operator trick), and the grammar's
literal matcher compares the token value, so `"<"` matches
transparently.  `packages/rust/ruby-parser/src/_grammar.rs` was
regenerated via `grammar-tools compile-grammar`.

New parser tests (+3):

- `test_parse_class_with_superclass` — `class Dog < Animal; end` parses
  with the `<` separator and superclass `Animal` tokens in the class
  header, and zero body statements.
- `test_parse_base_class_has_no_superclass_separator` — a base class
  `class Widget; end` carries no `<` token.
- `test_parse_subclass_with_method_body` — inheritance composes with a
  non-empty body (`<` separator + a `def_statement` body child).

A `body_has_token_value` test helper checks for a direct child token by
value.  Test count: 161 → 164 (+3).

## [0.39.0] - 2026-05-30

### Added (Phase 14b (FC) — class body with method defs + statements)

No grammar changes — the `class_statement` body
(`{ !"end" statement }`) already accepts any statement, so a class
mixing method definitions and executable statements parses without
a grammar edit.  Phase 14b adds parser coverage pinning the body
shape the 14b lowerer walks (one `statement` child per source line,
each wrapping its own inner rule):

- `test_parse_class_body_mixes_def_and_assignment` — `class Foo;
  MAX = 10; def bar; end; end` parses to a body holding both an
  `assignment` and a `def_statement`.
- `test_parse_class_body_multiple_assignments_preserved` — two
  consecutive constant assignments parse as two distinct body
  `statement` children, in source order.
- `test_parse_nested_class_inside_class_body` — a `class` declared
  inside another class parses as a nested `class_statement` body
  child (the shape the lowerer recurses through).

A `body_inner_rule_names` test helper collects the inner-rule name
of each direct body statement (one level deep).

Test count: 158 → 161 (+3).

## [0.38.0] - 2026-05-29

### Added (Phase 14a (FC) — empty `class Foo; end` grammar coverage)

No grammar changes — the existing `class_statement` rule
(`"class" NAME { !"end" statement } "end"`) already accepts an
empty body (the repetition matches zero statements).  Phase 14a
adds parser coverage for the exact parse properties the lowerer
depends on:

- `test_parse_empty_class_camelcase_name` — a multi-character
  CamelCase class name is extracted whole from the first Name token
  (the `class` keyword token is not mistaken for it).
- `test_parse_empty_class_has_zero_body_statements` — the empty
  class has zero `statement` children in its body.
- `test_parse_empty_class_followed_by_top_level_stmt` — an empty
  class does not swallow a following top-level statement; the
  `!"end"` boundary keeps `x = 1` a sibling assignment.

Tests: 155 → 158 (+3).

## [0.37.0] - 2026-05-28

### Added (Phase 9c (FC) — single-RHS tuple destructure grammar coverage)

No grammar changes — the existing `multi_assignment` rule already
accepts the 1-RHS shape (`expression { COMMA expression }` allows the
trailing repetition group to be empty).  Phase 9c enables the lowering
for that shape; this version bump tracks the new parser-level tests
that pin grammar behaviour:

- `test_parse_multi_assignment_single_rhs_two_lhs` (`a, b = arr`)
- `test_parse_multi_assignment_single_rhs_three_lhs` (`a, b, c = arr`)
- `test_parse_multi_assignment_single_rhs_keeps_one_rhs_expression`
  (asserts the RHS list has exactly one expression node)

### Tests

- `coding-adventures-ruby-parser`: 152 → **155** (+3)

## [0.36.0] - 2026-05-28

### Added (Phase 9b (FC) — splat target in multi-assignment LHS)

`multi_assignment` rule extended to allow an optional `*` prefix on
each LHS target via a new `mlhs_target` rule:

```ebnf
multi_assignment = mlhs_target COMMA mlhs_target { COMMA mlhs_target }
                   EQUALS expression { COMMA expression } ;
mlhs_target      = [ "*" ] NAME ;
```

The grammar allows the splat at any LHS position; the lowerer
enforces "at most one splat".  Single-LHS forms (`a = 1`) still fall
through to the existing `assignment` rule unchanged because
`multi_assignment` requires at least two `mlhs_target`s.

`_grammar.rs` regenerated via `grammar-tools compile-grammar`.

### Tests

- `coding-adventures-ruby-parser`: 149 → **152** (+3):
  - `test_parse_multi_assignment_splat_at_end`
  - `test_parse_multi_assignment_splat_at_start`
  - `test_parse_multi_assignment_splat_in_middle`

## [0.35.0] - 2026-05-26

### Added (Phase 8a-2 (FC) — `>>=` right-shift compound assign)

`assignment` rule extended with `">>="` alongside the Phase 8a additions:

```ebnf
assignment = NAME ( EQUALS | "+=" | "-=" | "*=" | "/=" | "%=" | "**=" | "<<=" | ">>=" | "&=" | "|=" | "^=" | "||=" | "&&=" ) expression ;
```

The lexer's new `fuse_right_shifts()` pass folds `>>=` into a single `Name(">>=")` token before this rule runs, so parsing is mechanical.  Combined with Phase 8a, the parser now accepts Ruby's **complete** compound-assignment family on local variables (no further deferrals).

Regenerated `_grammar.rs` via `grammar-tools compile-grammar`.

### Tests

- `coding-adventures-ruby-parser`: 147 → **149** (+2):
  - `test_parse_right_shift_op_assign`
  - `test_parse_left_and_right_shift_op_assigns_round_trip`

## [0.34.0] - 2026-05-26

### Added (Phase 8a (FC) — additional arithmetic / bitwise / shift op-assigns)

Extended the `assignment` rule to recognise six more compound-assignment operators:

```ebnf
assignment = NAME ( EQUALS | "+=" | "-=" | "*=" | "/=" | "%=" | "**=" | "<<=" | "&=" | "|=" | "^=" | "||=" | "&&=" ) expression ;
```

Combined with the lexer's `fuse_compound_assigns` companion pass (extended in this phase to recognise `%`, `**`, `<<`, `&`, `|`, `^` as left operands), Ruby's complete compound-assign family on local variables is now parseable — minus `>>=`, which is tracked as a follow-up.

Regenerated `_grammar.rs` via `grammar-tools compile-grammar`.

### Tests

- `coding-adventures-ruby-parser`: 143 → **147** (+4):
  - `test_parse_modulo_op_assign`
  - `test_parse_power_and_shift_op_assigns`
  - `test_parse_bitwise_op_assigns`
  - `test_parse_plain_assignment_still_works_after_8a` (regression)

### v0 deferred limitations

- `>>=` is NOT yet accepted because the 1.8-era lexer state machine splits `>>` into two `>` tokens.  A dedicated `>>` pre-fusion pass is the natural follow-up; it slots in cleanly because the post-`>>=` lowering reuses the same Phase 8a desugar path.

## [0.33.0] - 2026-05-26

### Added (Phase 7f — Ruby 3.1 hash value-omitted shorthand `{x:, y:}`)

Extended grammar rule:

```ebnf
hash_entry = NAME COLON expression | NAME COLON | expression "=>" expression ;
```

The new `NAME COLON` alternative (no trailing expression) is placed AFTER `NAME COLON expression` so PEG ordered-choice tries the longer form first.  When the longer form fails (e.g. `x:` followed by `,` or `}`), the parser cleanly backtracks to the value-omitted shape.

The grammar change is purely additive — every prior shape (`{x: 1}`, `{a => b}`, `{x: 1, y: 2}`) parses identically.

Regenerated `_grammar.rs` via `grammar-tools compile-grammar`.

### Tests

- `coding-adventures-ruby-parser`: 140 → **143** (+3):
  - `test_parse_hash_value_shorthand_pure` — `{x:, y:}` produces two `hash_entry` subnodes each with ZERO `expression` children.
  - `test_parse_hash_value_shorthand_mixed` — `{x:, y: 5}` produces one entry with 0 expression children and one with 1.
  - `test_parse_hash_value_shorthand_regression_existing_form` — `{x: 1, y: 2}` still produces entries with 1 expression child each.

## [0.32.0] - 2026-05-25

### Added (Phase 7e — Ruby 3.0 rightward assignment `expr => var`)

New grammar rule:

```ebnf
rightward_assignment = expression "=>" NAME ;
```

Placed AFTER `modifier_statement` (so `x = 1 if cond` still wins) and BEFORE `assignment` in the `statement` alternation.

The `=>` token is shared with hash literals (`{ "a" => 1 }`), but the rightward-assignment rule only matches at the statement level — PEG cleanly backtracks when `=>` sits inside `{...}`.

Regenerated `_grammar.rs` via `grammar-tools compile-grammar`.

### Lowering (in `ruby-to-semantic-ir`)

A new helper `lower_rightward_assignment` lowers identically to a regular `assignment`:

| Source              | SIR shape                                                |
|---------------------|----------------------------------------------------------|
| `1 + 2 => sum`      | `LetBinding(sum, BuiltinCall("+", [IntLit 1, IntLit 2]))` |
| `42 => x`           | `LetBinding(x, IntLit 42)`                               |
| (re-bind) `5 => x`  | `Assign(x, IntLit 5)` + `Feature::MutableBindings`       |

Rightward assignment is purely syntactic — `expr => var` and `var = expr` produce identical SIR — so downstream emitters need no new code paths.

### v0 deferred limitations

- Compound rightward forms (`expr += var`, etc.) are NOT supported in Ruby itself.
- Modifier-suffix combinations like `expr => var if cond` are deferred (modifier_statement only accepts `assignment | method_call_no_paren | method_call | expression_stmt` as its LHS).

### Tests

- `coding-adventures-ruby-parser`: 136 → **140** (+4 grammar tests):
  - `test_parse_rightward_assignment_with_literal` — `1 => x`.
  - `test_parse_rightward_assignment_with_binary_expression` — `1 + 2 => sum`.
  - `test_parse_rightward_assignment_with_call` — `foo(1, 2) => result`.
  - `test_parse_rightward_assignment_does_not_break_normal_assignment` — regression.

## [0.31.0] - 2026-05-25

### Added (Phase 7d — Ruby 3.0 `case/in` pattern matching)

The existing `case_statement` rule is extended to accept either `when_clause` or `in_clause` repetitions in any source order:

```ebnf
case_statement = "case" expression { when_clause | in_clause } [ else_clause ] "end" ;
in_clause      = "in" pattern { !"when" !"in" !"else" !"end" statement } ;
pattern        = array_pattern | hash_pattern | literal_pattern | binding_pattern ;
literal_pattern   = NUMBER | STRING | symbol_literal | KEYWORD ;
binding_pattern   = NAME ;
array_pattern     = LBRACKET [ pattern { COMMA pattern } ] RBRACKET ;
hash_pattern      = LBRACE [ hash_pattern_pair { COMMA hash_pattern_pair } ] RBRACE ;
hash_pattern_pair = NAME COLON [ pattern ] ;
```

Order in `pattern`: array/hash come first so their `[`/`{` openers shadow the literal/binding forms; literal comes before binding so concrete constants don't get swallowed as bindings.

Regenerated `_grammar.rs` via `grammar-tools compile-grammar`.

The `when_clause` body lookaheads were extended with `!"in"` so `when`/`in` can interleave cleanly in the rule's repetition (even though semantically Ruby disallows mixing — this is a grammar concern, not a semantic one).

### Lowering (in `ruby-to-semantic-ir`)

`lower_case_statement` now collects both `when_clause` and `in_clause` subnodes in source order.  A new helper `lower_in_clause_pattern` dispatches on pattern kind:

| Pattern        | cond                                            | body-prefix stmts        |
|----------------|-------------------------------------------------|--------------------------|
| `in 1`         | `BuiltinCall("==", [scrut, IntLit(1)])`         | `[]`                     |
| `in nil`       | `BuiltinCall("==", [scrut, NilLit])`            | `[]`                     |
| `in y`         | `BoolLit(true)`                                 | `[LetBinding(y, scrut)]` |
| `in [1, 2]`    | `BuiltinCall("__pattern_match__", [scrut, raw])` | `[]`                    |
| `in {name: y}` | `BuiltinCall("__pattern_match__", [scrut, raw])` | `[]`                    |

The `__pattern_match__` marker carries the raw source text of the pattern (joined Token values via a depth-first walk) so downstream emitters can re-derive the structural matching at codegen time.  Same marker-builtin pattern as Phase 6v rescue/ensure, Phase 6y `__interp__`, Phase 7a `backtick`.

### v0 deferred limitations

- Array / hash patterns lower as a `__pattern_match__` marker — no structural decomposition or sub-bindings yet.  Downstream emitters can re-parse the marker's StrLit raw text.
- Hash pattern shorthand `{name:}` doesn't bind `name` at SIR level (deferred along with structural decomposition).
- Pin operators (`^x`), find patterns (`[…, *, …]`), and class patterns (`SomeClass(x)`) are not yet parsed.
- A bare-name body inside `in` (e.g. `in y; y; end`) hits a pre-existing grammar quirk where `method_call_no_paren` greedily consumes the closing `end` as a one-arg method-call argument — unrelated to Phase 7d, observable since Phase 6h.  Workaround: wrap the body in a paren-call (`puts(y)`) or any non-bare form.

### Tests

- `coding-adventures-ruby-parser`: 131 → **136** (+5 grammar tests):
  - `test_parse_case_in_with_literal_pattern` — `in 1`.
  - `test_parse_case_in_with_binding_pattern` — `in y`.
  - `test_parse_case_in_with_array_pattern` — `in [1, 2]`.
  - `test_parse_case_in_with_hash_pattern` — `in {name: y}`.
  - `test_parse_case_when_still_works_after_in_clause_addition` — regression.

## [0.30.0] - 2026-05-25

### Added (Phase 7c — Ruby 3.0 endless method definitions `def foo = expr`)

New grammar rule:

```
endless_def_statement = "def" NAME [ LPAREN [ params ] RPAREN ] EQUALS expression ;
```

Placed **before** `def_statement` in the `statement` alternation so PEG tries the endless form first; when the `=` isn't present the parser falls through to the block-bodied `def`.  The two forms cannot conflict because the endless form requires `=` immediately after the signature, while the block form expects a newline + body statements + `end`.

Regenerated `_grammar.rs` via `grammar-tools compile-grammar`.

### Lowering (in `ruby-to-semantic-ir`)

A new helper `lower_endless_def_statement` mirrors `lower_def_statement`'s parameter extraction and scope-isolation logic, but the body is the single trailing `expression` Node:

```
endless_def_statement → Function {
    name,
    params,
    body: Block { stmts: [], value: <lowered expression> },
    ...
}
```

Both the program-level pre-pass (`collect_def_statements`) and the class/module nested pass (`collect_def_statements_from_body`) now dispatch on rule name to handle both `def_statement` and `endless_def_statement`.

### v0 deferred limitations

- Inherits Phase 6s's lossy-splat limitation: `def foo(*args) = args.sum` parses but the `*` prefix on the param is dropped at SIR level (`Param` has no variadic flag).
- The endless form is not nested under classes / modules in any SIR-significant way (matches the block-bodied def's v0 limitation).

### Tests

- `coding-adventures-ruby-parser`: 128 → **131** (+3 grammar tests):
  - `test_parse_endless_def_no_params` — `def hello = 1`.
  - `test_parse_endless_def_with_params` — `def add(x, y) = x + y`.
  - `test_parse_endless_def_does_not_break_block_def` — regression for the block-bodied form.

## [0.29.0] - 2026-05-25

### Added (Phase 7b — heredocs in parser)

The lexer's Phase-3c body capture + Phase-4o opener-variant handling (`<<EOF`, `<<-EOF`, `<<~EOF`) finalise every heredoc into a single `TokenType::String` token whose value is the verbatim canonical form `<<TAG\n<body>TAG` (with `<<~TAG`'s common-indent stripping pre-applied).  Phase 7b is therefore **grammar-zero**: the existing STRING token at factor / assignment-RHS positions already accepts heredocs.  This PR adds the SIR lowering dispatch + explicit test coverage.

### Lowering (in `ruby-to-semantic-ir`)

A new helper `lower_heredoc_literal` strips the opener prefix and the closing tag, emitting:

```
StrLit(body)
```

The `String` case of `lower_factor_atom` now dispatches in lexeme-prefix priority order:
- starts with `` ` `` → backtick command literal (Phase 7a)
- starts with `<<` → heredoc (Phase 7b)
- otherwise → string interpolation lowering (Phase 6y)

The synthetic `StrLit` triggers `Feature::Strings`.

### v0 deferred limitations

- Interpolation inside the body (`#{name}`) is NOT split — the body lowers as a single `StrLit` with `#{...}` markers preserved verbatim.  Follow-up will reuse the Phase 6y splitter.
- Non-interpolating heredocs (`<<'TAG'`) and the `<<"TAG"` form are not yet distinguished — the lexer doesn't carry the quote state, so every heredoc is treated the same.
- Escape sequences inside the body are kept literal (the lexer's heredoc capture does not unescape; same v0 stance as backticks).

### Tests

- `coding-adventures-ruby-parser`: 125 → **128** (+3 grammar tests):
  - `test_parse_plain_heredoc_assignment` — `<<EOF`.
  - `test_parse_dash_indent_heredoc_assignment` — `<<-EOF` with indented closer.
  - `test_parse_tilde_indent_heredoc_assignment` — `<<~EOF` with indent stripping.

## [0.28.0] - 2026-05-25

### Added (Phase 7a — backtick command literals in parser)

The lexer's Phase-4m `backtick_body` state emits `` `cmd args` `` as a single `TokenType::String` token whose value is the verbatim source **including the surrounding backticks** (`` `cmd args` ``) — the same lexeme-prefix sentinel trick used by percent literals and heredocs.  Phase 7a is therefore **grammar-zero**: the existing STRING token at factor / call-arg / assignment-RHS positions already accepts backtick literals.  This phase adds the SIR lowering dispatch + explicit test coverage on both sides of the pipeline.

### Lowering (in `ruby-to-semantic-ir`)

A new helper `lower_backtick_command_literal` strips the wrapping backticks and emits:

```
BuiltinCall {
    name: "backtick",
    args: [StrLit(body)],
    effects: MayBlock | MayPrint | MayThrow,
}
```

The triple-effect set reflects that command execution may **block** on the child process, **print** stdout/stderr, and **throw** if the command can't be invoked (`Errno::ENOENT` and friends).

The `String` case of `lower_factor_atom` dispatches by lexeme prefix:
- starts with `` ` `` → backtick command literal (Phase 7a)
- otherwise → string interpolation lowering (Phase 6y)

### v0 deferred limitations

- Interpolation inside the body (`` `echo #{name}` ``) is NOT split — the body lowers as a single `StrLit` with `#{...}` markers preserved verbatim.  A future phase will reuse the Phase 6y interpolation splitter inside the body.
- Escape sequences inside the body (`` \` ``, `\n`, `\t`) are already resolved by the lexer (Phase 4m's body state).

### Tests

- `coding-adventures-ruby-parser`: 122 → **125** (+3 grammar tests):
  - `test_parse_backtick_command_literal_assignment` — `` x = `ls -la` ``.
  - `test_parse_backtick_command_literal_in_call_arg` — `` puts(`pwd`) ``.
  - `test_parse_empty_backtick_command_literal` — `` x = `` ``.

## [0.27.0] - 2026-05-25

### Added (Phase 6z — float / hex / bin / oct numeric literal parsing)

The lexer's Phase-4k float fusion (`1.5`, `1e10`, `1.5e-3`) and Phase-4l radix-prefix fusion (`0x1F`, `0b1010`, `0o17`, `0d42`) emit each literal as a single `TokenType::Number` token whose value is the verbatim source text.  Phase 6z is therefore **grammar-zero**: the existing NUMBER token at factor position already accepts every shape.  This phase adds the SIR lowering dispatch + explicit test coverage on both sides of the pipeline.

### Lowering (in `ruby-to-semantic-ir`)

A new helper `lower_numeric_literal` dispatches on shape:

| Source       | SIR shape                                |
|--------------|------------------------------------------|
| `42`         | `IntLit { value: 42 }`                   |
| `1_000_000`  | `IntLit { value: 1000000 }`              |
| `0x1F`       | `IntLit { value: 31 }` (radix 16)        |
| `0xDEAD_BEEF`| `IntLit { value: 3735928559 }`           |
| `0b1010`     | `IntLit { value: 10 }` (radix 2)         |
| `0o17`       | `IntLit { value: 15 }` (radix 8)         |
| `0d42`       | `IntLit { value: 42 }` (radix 10 explicit) |
| `1.5`        | `FloatLit { value: 1.5 }` + Feature::Floats |
| `1e10`       | `FloatLit { value: 1e10 }` + Feature::Floats |
| `1.5e-3`     | `FloatLit { value: 0.0015 }` + Feature::Floats |

Underscore separators (`_`) are stripped before parsing.  Radix detection checks `bytes[1] ∈ {x,X,b,B,o,O,d,D}` after a leading `0`.  Float detection is a single scan for `.` or `e`/`E` in the cleaned digit string; the two checks are mutually exclusive in the Ruby grammar.

### v0 deferred limitations

- Ruby's Rational (`r`) / Complex (`i`) numeric suffixes (lexed by Phase 4f) are rejected — a future phase will route those into `BuiltinCall("rational", ...)` / `BuiltinCall("complex", ...)` markers.
- Negative literals are still handled by the unary-minus path (Phase 6k); this routine sees only the magnitude.
- Ruby's legacy octal syntax (`017` without `0o` prefix) is not supported; use `0o17`.

### Tests

- `coding-adventures-ruby-parser`: 117 → **122** (+5 grammar tests):
  - `test_parse_float_literal_assignment` — `x = 1.5`.
  - `test_parse_float_literal_with_exponent` — `x = 1.5e-3`.
  - `test_parse_hex_integer_literal` — `x = 0xDEAD_BEEF`.
  - `test_parse_binary_integer_literal` — `x = 0b1010`.
  - `test_parse_octal_integer_literal` — `x = 0o17`.

## [0.26.0] - 2026-05-25

### Added (Phase 6y — string interpolation expression parsing)

The lexer's Phase-3b state machine captures `"foo#{x}bar"` as a single `TokenType::String` token whose value carries the `#{...}` markers verbatim, with `{`/`}` already brace-balanced by the lexer's `interp_brace_depth` tracking.  Phase 6y is therefore **grammar-zero**: the existing STRING token at factor / call-arg / assignment-RHS positions already accepts interpolated forms.  This phase adds explicit test coverage for the parser side and the SIR lowering for the interpolation split.

### Lowering (in `ruby-to-semantic-ir`)

The SIR lowerer's String case now scans the raw content for `#{...}` segments and emits:

| Source              | SIR shape                                                                                       |
|---------------------|-------------------------------------------------------------------------------------------------|
| `"plain"`           | `StrLit("plain")` (zero-cost fast path — unchanged)                                             |
| `"#{x}"`            | `VarRef("x")` (single non-literal segment, no wrapper)                                          |
| `"hi #{name}"`      | `BuiltinCall("string_concat", [StrLit("hi "), VarRef("name")])`                                 |
| `"sum=#{1+2}"`      | `BuiltinCall("string_concat", [StrLit("sum="), BuiltinCall("__interp__", [StrLit("1+2")])])`    |

Bare-identifier interp bodies lower to a `VarRef` with the same `Scope::Param` / `Scope::Local` dispatch as the regular factor-atom Name case.  More complex interp bodies (arithmetic, calls, nested strings) emit a marker `BuiltinCall("__interp__", [StrLit(raw_body)])`, matching the marker pattern used by Phase 6v rescue/ensure — downstream Ruby emitters can re-emit the marker verbatim as `#{<raw>}`.

### v0 deferred limitations

- Complex interp bodies are carried as the raw `__interp__` marker rather than being recursively parsed — a future phase will invoke the parser/lowerer on the body so the SIR carries semantic info.
- Escape sequences inside the string literal pass through unchanged (the lexer hasn't unescaped them yet).

### Tests

- `coding-adventures-ruby-parser`: 113 → **117** (+4):
  - `test_parse_interpolated_string_assignment` — `x = "hello #{name}"`.
  - `test_parse_interpolated_string_in_call_arg` — `puts("sum=#{1+2}")`.
  - `test_parse_interpolated_string_only_interp` — `x = "#{name}"`.
  - `test_parse_interpolated_string_multiple_segments` — `x = "a=#{a}, b=#{b}"`.

## [0.25.0] - 2026-05-25

### Added (Phase 6x — instance var `@x`, class var `@@x`, global var `$x` refs)

The lexer (Phase 4i/4j) emits these as a SINGLE Name-typed token whose value carries the leading sigil (`@a`, `@@all`, `$c`).  The parser sees them as bare `NAME` tokens at the factor and assignment-LHS levels — **no new grammar rules required**.  This phase adds explicit test coverage and the SIR lowerer's documentation of how sigil-preserved names flow through.

### Lowering (in `ruby-to-semantic-ir`)

- All sigil-prefixed names route to `Scope::Local` for v0.
- The sigil stays in the bound name (`@a`, `@@all`, `$config`), so downstream emitters can detect the form and route assignment/read appropriately.
- The validator-correct `$x` → `Scope::Global` mapping would require auto-emitting matching `Global` declarations on the module; that's deferred to a follow-up phase.

### v0 deferred limitations

- SIR has no `IVar` / `CVar` / `GVar` scope.  Using `Scope::Global` for `$x` would require module-level `Global` declarations that the validator enforces.  Until those are auto-generated, all sigil vars sit on `Scope::Local` with the sigil preserved in `name`.
- Downstream emitters targeting Ruby (or any language with similar sigil semantics) can still distinguish by checking for the leading `@` / `@@` / `$`.

### Tests

- `coding-adventures-ruby-parser`: 109 → **113** (+4):
  - `test_parse_instance_var_assignment` — `@a = 1`.
  - `test_parse_class_var_assignment` — `@@all = 0`.
  - `test_parse_global_var_assignment` — `$config = 1`.
  - `test_parse_instance_var_in_expression` — `puts(@a)`.

## [0.24.0] - 2026-05-25

### Added (Phase 6w — arrow-lambda literal `->(params){body}`)

New grammar rule:

```
factor         = ( lambda_literal | method_call | NUMBER | ... ) { dot_call } ;
lambda_literal = "->" [ LPAREN [ params ] RPAREN ] block ;
```

Placed BEFORE `method_call` in factor so the `->` literal wins.  Without this, `->` (a Name-typed Op token from the lexer) would be mis-matched by `method_call`'s `(NAME|KEYWORD) LPAREN` prefix when followed by parens.

`lambda { … }` and `proc { … }` continue to lower via `method_with_block` (no new grammar — they're regular keyword-led calls).  The SIR lowerer now recognises `lambda` and `proc` as builtins so both shapes produce the same `BuiltinCall("lambda", …)` form.

#### Parser default era bumped to "3.0"

`create_ruby_parser` now invokes the lexer with `tokenize_ruby_for_version(source, "3.0")` so era-gated lexer fusions are visible to the parser by default.  Most importantly, `->` is now fused into a single Op token (1.9.1+ behaviour) so `lambda_literal` can match it.  Era-specific lexer tests still use the lower-level `tokenize_ruby_for_version` directly.

New public constant: `DEFAULT_RUBY_ERA: &str = "3.0"`.

### Lowering (in `ruby-to-semantic-ir`)

Arrow lambda → `BuiltinCall("lambda", [MakeClosure { fn_name: "__block_<n>", captures: [] }])`.

Body is hoisted to a top-level `Function` (named `__block_<n>`, reusing Phase 6g's counter).  Params are extracted from the parens-list (Phase 6s — splat supported) rather than from the block's `block_params` pipe header.

### v0 deferred limitations

- Block bodies that reference outer locals lose them — captures are NOT computed (same as Phase 6g).
- If the user writes both `->(x) { |y| … }` (parens-params AND block_params), the latter is silently ignored.
- `lambda { ... }` / `proc { ... }` works only at statement position, not as an expression RHS (because `method_with_block` is not in `factor`).  Arrow form (`->(...) { ... }`) is the recommended form for expression-position closures.

### Tests

- `coding-adventures-ruby-parser`: 105 → **109** (+4):
  - `test_parse_arrow_lambda_no_params` — `-> { 1 }`.
  - `test_parse_arrow_lambda_with_params` — `->(x, y) { x + y }`.
  - `test_parse_arrow_lambda_inside_call` — `each(->(x) { x })`.
  - `test_parse_lambda_keyword_with_brace_block` — `lambda { |x| x + 1 }` (regression: NOT a lambda_literal).

## [0.23.0] - 2026-05-25

### Added (Phase 6v — `begin … rescue … ensure … end`)

New grammar rules:

```
statement       = ... | case_statement | begin_statement | return_statement | ... ;
begin_statement = "begin"
                  { !"rescue" !"ensure" !"end" statement }
                  { rescue_clause }
                  [ ensure_clause ]
                  "end" ;
rescue_clause   = "rescue" [ exception_list "=>" NAME ]
                       { !"rescue" !"ensure" !"end" statement } ;
exception_list  = NAME { COMMA NAME } ;
ensure_clause   = "ensure" { !"end" statement } ;
```

#### Grammar design decision

`rescue_clause` accepts the optional exception header ONLY when paired with `=>` binding (`rescue StandardError => e`).  Bare `rescue ExceptionType` (no `=>`) would create grammar ambiguity with the rescue body's first NAME-led statement — `rescue x = 2` would otherwise parse `x` as an exception type and fail at `=`.  Workaround for users wanting bare-type rescue: write `rescue ExceptionType => _`.

#### Lowering (in `ruby-to-semantic-ir`)

SIR has no try/catch primitive.  v0 lowering is lossy: body, rescue, and ensure stmts emit inline with synthetic marker `BuiltinCall`s:

```
begin body
rescue StandardError, IOError => e rescue_body
ensure ensure_body
end
```

→

```
body_stmts...
ExprStmt(BuiltinCall("__rescue_marker__", [StrLit("StandardError,IOError"), StrLit("e")]))
rescue_stmts...
ExprStmt(BuiltinCall("__ensure_marker__", []))
ensure_stmts...
```

Markers carry the `Effect::MayThrow` tag.  Downstream emitters targeting languages with real exceptions can re-stitch the form via marker detection; for v0 the rescue body is unreachable in SIR's effect model.

`lower_statement_inner_multi` (already routing `multi_assignment` to a Vec<Stmt> path from Phase 6r) is extended to dispatch `begin_statement` to a new `lower_begin_statement` helper.

#### v0 deferred limitations

- Bare `rescue ExceptionType` (no `=>`) — see grammar design note.
- `else` clause inside begin (executes when no exception raised) — not supported.
- Real try/catch propagation through SIR's effect lattice — markers only.
- `retry` / `raise` inside rescue body lower like any other call.

### Tests

- `coding-adventures-ruby-parser`: 101 → **105** (+4):
  - `test_parse_begin_with_rescue` — bare `rescue` body.
  - `test_parse_begin_with_rescue_typed_and_var` — `rescue StandardError => e`.
  - `test_parse_begin_with_ensure` — `begin … ensure … end`.
  - `test_parse_begin_with_rescue_and_ensure` — full three-section form.

## [0.22.0] - 2026-05-25

### Added (Phase 6u — `case … when … else … end`)

New grammar rules:

```
statement      = ... | until_statement | case_statement | return_statement | ... ;
case_statement = "case" expression { when_clause } [ else_clause ] "end" ;
when_clause    = "when" expression { COMMA expression }
                      { !"when" !"else" !"end" statement } ;
```

`else_clause` is reused from the existing `if_statement` definition — the else body parses identically to `if … else … end`.

Multi-value `when` lists are supported (`when 1, 2, 3` parses with three expression children inside the clause).  Each when_clause's body uses the same negative-lookahead repetition (`{ !"when" !"else" !"end" statement }`) as if/else, so it stops cleanly at the next clause boundary.

### Lowering (in `ruby-to-semantic-ir`)

Chained `Expr::If` with `==` comparisons:

```
case x
when v1, v2 then a
when v3     then b
else c
end
```

becomes

```
if ((x == v1) || (x == v2)) then a
else if (x == v3) then b
else c
```

- Each when_clause becomes one nested `If` step.
- Multi-value `when 1, 2, 3` lists OR-fold left-to-right using `BuiltinCall("or", ...)`.
- The else_clause (or implicit `NilLit` block) caps the chain.

### v0 deferred limitations

- `when` uses `==` instead of Ruby's `===` (case-equality, class-aware).  Phase 7d will add full `case/in` pattern matching with proper match semantics.
- Range/Regex/Class values in `when` lists parse syntactically but don't match Ruby's case-equality semantics under v0's `==` lowering.
- Splat in `when` argument lists (`when *arr`) is not supported.

### Tests

- `coding-adventures-ruby-parser`: 98 → **101** (+3):
  - `test_parse_case_single_when` — one when, no else.
  - `test_parse_case_multiple_whens_and_else` — two whens + else.
  - `test_parse_when_with_multiple_values` — `when 1, 2, 3`.

## [0.21.0] - 2026-05-25

### Added (Phase 6t — `yield` keyword with optional args)

New grammar rule:

```
statement       = ... | next_statement | yield_statement | multi_assignment | ... ;
yield_statement = "yield" [ yield_args ] ;
yield_args      = LPAREN [ call_arg { COMMA call_arg } ] RPAREN
                | call_arg { COMMA call_arg } ;
```

All three surface forms supported:

| Source | AST |
|---|---|
| `yield` | yield_statement with no `yield_args` wrapper |
| `yield(x, y)` | yield_statement → yield_args → LPAREN call_arg COMMA call_arg RPAREN |
| `yield x, y` | yield_statement → yield_args → call_arg COMMA call_arg |
| `yield(*arr)` | yield_statement → yield_args → LPAREN call_arg(*arr) RPAREN |

Placed AFTER `next_statement` and BEFORE the catch-all `multi_assignment` / `modifier_statement` / `assignment` / `method_with_block` / `method_call` family so `yield` (which the lexer reclassifies as a KEYWORD token) doesn't fall through to `method_call_no_paren` (which would lose the yield-specific lowering).

Args reuse Phase 6s's `call_arg`, so splat (`yield *arr`) and double-splat (`yield **hsh`) work uniformly with method-call args.

### Lowering (in `ruby-to-semantic-ir`)

`yield ...` → `Stmt::ExprStmt(Expr::BuiltinCall("yield", lowered_args, EffectSet::PURE))`.

Effects are PURE — the block's effects bubble up through its construction site (via the `MakeClosure` effect set) rather than via the `yield` call.

### Tests

- `coding-adventures-ruby-parser`: 94 → **98** (+4):
  - `test_parse_bare_yield` — `yield` alone.
  - `test_parse_yield_with_paren_args` — `yield(x, y)`.
  - `test_parse_yield_with_parenless_args` — `yield x, y`.
  - `test_parse_yield_with_splat_arg` — `yield(*arr)`.

## [0.20.0] - 2026-05-25

### Added (Phase 6s — splat / double-splat in params and call args)

Grammar additions:

```
params   = param { COMMA param } ;
param    = [ "*" | "**" ] NAME ;
method_call = ( NAME | KEYWORD ) LPAREN [ call_arg { COMMA call_arg } ] RPAREN { dot_call } ;
dot_call    = "." ( NAME | KEYWORD ) [ LPAREN [ call_arg { COMMA call_arg } ] RPAREN ] ;
call_arg = [ "*" | "**" ] expression ;
```

`params` previously bound bare `NAME` tokens; each parameter is now wrapped in a `param` rule that admits the optional splat / double-splat prefix.  Similarly, `method_call` and `dot_call` argument slots are wrapped in a `call_arg` rule.

`method_call_no_paren` intentionally keeps bare `expression` args — wrapping it in `call_arg` would create a grammar ambiguity with binary `*` at expression-start position (`a * b` would parse as `a(splat b)`).  Paren-less splat (`puts *arr`) is therefore a v0 deferred limitation; users can always fall back to the parenned form `puts(*arr)`.

The lexer already emits `**` as one Name-typed Op token (1.8-baseline state machine coalesces it); `*` is a Star token.  Both match by literal value in the grammar.

### Lowering (in `ruby-to-semantic-ir`)

- **Call args**: `f(*arr)` → `BuiltinCall("splat", [VarRef(arr)])`; `f(**hsh)` → `BuiltinCall("double_splat", [VarRef(hsh)])`.  Bare args are unchanged.
- **Params**: v0 lossy — `*args` and `**kwargs` lower to regular `Param { name: "args" / "kwargs" }`.  SIR's `Param` has no variadic flag, so splat-ness is dropped at the SIR level.  Downstream emitters treat the parameter as positional; correctness for variadic call sites is a deferred limitation.

### Tests

- `coding-adventures-ruby-parser`: 88 → **94** (+6):
  - `test_parse_splat_param` — `def f(*args) end`.
  - `test_parse_double_splat_param` — `def f(**kwargs) end`.
  - `test_parse_mixed_params_with_splats` — `def f(a, *rest, **opts) end`.
  - `test_parse_splat_call_arg` — `f(*arr)`.
  - `test_parse_mixed_call_args_with_splats` — `f(1, *arr, **hsh)`.
  - `test_parse_binary_star_still_parses_as_expression` — regression: `a * b` stays binary.

Also updates two pre-existing tests to walk the new `param` / `call_arg` wrappers (`test_parse_def_with_params`, `test_parse_dot_call_with_args`).

## [0.19.0] - 2026-05-25

### Added (Phase 6r — multiple assignment `a, b = 1, 2`)

New grammar rule:
```
statement        = ... | next_statement | multi_assignment | modifier_statement |
                   assignment | method_with_block | method_call | method_call_no_paren |
                   expression_stmt ;
multi_assignment = NAME COMMA NAME { COMMA NAME }
                   EQUALS
                   expression { COMMA expression } ;
```

Placed BEFORE `modifier_statement` and `assignment` so `NAME COMMA NAME ... =` parses as a multi-assignment.  Requires at least two LHS names; single-LHS forms (`a = 1`) still flow through the existing `assignment` rule unchanged because `multi_assignment` fails immediately when only one NAME is present.

#### Lowering (in `ruby-to-semantic-ir`)

Each `(lhs[i], rhs[i])` pair lowers to its own `Stmt::LetBinding` / `Stmt::Assign` — exactly as `lhs[i] = rhs[i]` would.  The lowerer's new `lower_statement_inner_multi` wrapper returns a `Vec<Stmt>` so a single `multi_assignment` source node fans out to N SIR statements at every call site (`lower_program`, `lower_clause_statements`, `lower_def_statement` body, `lower_method_with_block` body).

#### v0 restrictions (deferred)

- LHS count must equal RHS count.  Mismatched arities are rejected with a `RubyLowerError`; the more permissive Ruby semantics (excess LHS gets `nil`, excess RHS dropped) ride with a future phase.
- Single-RHS auto-unpack `a, b = arr` is NOT supported.
- Splat targets `a, *b = 1, 2, 3` ride with Phase 6s.
- Multi-assignment LHS inside a `modifier_statement` (`a, b = 1, 2 if cond`) is NOT supported.

### Tests

- `coding-adventures-ruby-parser`: 84 → **88** (+4):
  - `test_parse_multi_assignment_two_names` — basic `a, b = 1, 2`.
  - `test_parse_multi_assignment_three_names` — three LHS / three RHS.
  - `test_parse_multi_assignment_with_complex_rhs` — `a, b = x + 1, y * 2`.
  - `test_parse_single_assignment_not_consumed_by_multi` — regression: `a = 1` stays an `assignment`.

## [0.18.0] - 2026-05-24

### Added (Phase 6q — modifier conditionals/loops `x if y`, `x unless y`, `x while y`, `x until y`)

Trailing-modifier surface syntax for one-line `if`/`unless`/`while`/`until`.

#### Grammar change

```
statement          = ... | return_statement | break_statement | next_statement |
                     modifier_statement | assignment | method_with_block | method_call |
                     method_call_no_paren | expression_stmt ;
modifier_statement = ( assignment | method_call_no_paren | method_call | expression_stmt )
                     ( "if_modifier" | "unless_modifier" | "while_modifier" | "until_modifier" )
                     expression ;
```

Placement: AFTER the keyword-led statements (so `if y ... end` still wins) and BEFORE the bare statement forms (so we try the modifier wrapper before committing to a plain statement).  PEG-style alternation backtracking unwinds cleanly when the trailing keyword isn't present, falling through to the plain forms.

#### Lexer-side disambiguation

The trailing keyword tokens use special values (`if_modifier` etc., not bare `if`/etc.) because ruby-lexer's `tag_modifier_keywords` post-pass re-tags `if`/`unless`/`while`/`until` Keyword tokens to `*_modifier` when they follow an expression-ending token on the same line.  Leading-keyword forms keep the bare values and continue to match the existing `if_statement` / `unless_statement` / `while_statement` / `until_statement` rules.

This sidesteps the grammar's newline-insensitive default mode (modifier syntax is intrinsically a same-line construct in Ruby).  Without the lexer split, two-line programs like `x = 1\nif y ... end` would mis-parse as `(x = 1) if y` followed by orphaned `end`.

#### Lowering (in `ruby-to-semantic-ir`)

| Source              | Lowered SIR                                              |
|---------------------|----------------------------------------------------------|
| `lhs if cond`       | `Stmt::ExprStmt(Expr::If(cond, [lhs], Nil))`             |
| `lhs unless cond`   | `Stmt::ExprStmt(Expr::If(not(cond), [lhs], Nil))`        |
| `lhs while cond`    | `Stmt::While(cond, [lhs])`                               |
| `lhs until cond`    | `Stmt::While(not(cond), [lhs])`                          |

Same `Expr::If` / `Stmt::While` shapes as the leading-keyword forms — downstream emitters (semantic-ir-to-python, -rust, -typescript, -go) need zero new code paths.

### Tests

- `coding-adventures-ruby-parser`: 78 → **84** (+6):
  - `test_parse_if_modifier_simple` — `puts "hi" if cond` parses with `if_modifier`.
  - `test_parse_unless_modifier_with_assignment_lhs` — `x = 1 unless cond`.
  - `test_parse_while_modifier` — `puts "tick" while cond`.
  - `test_parse_until_modifier_with_assignment_lhs` — `x = 1 until cond`.
  - `test_parse_leading_if_not_tagged_as_modifier` — regression: `if y ... end` stays `if_statement`.
  - `test_parse_two_statements_across_newline` — regression: `x = 1\nif y ... end` stays two statements.

## [0.17.0] - 2026-05-24

### Added (Phase 6p — compound assignment `+=`, `-=`, `*=`, `/=`, `||=`, `&&=`)

Grammar change:

```
assignment = NAME ( EQUALS | "+=" | "-=" | "*=" | "/=" | "||=" | "&&=" ) expression ;
```

The lexer's companion `fuse_compound_assigns` post-pass folds adjacent `Op` + `Equals` token pairs into single Name-typed tokens carrying the fused operator value (`+=`, etc.), so the grammar matches by literal value — same convention as `"=>"`, `"<="`, `"&&"`.

Excludes `==`, `!=`, `<=`, `>=`, `===` — those are comparison operators handled at the `comparison` layer.  The fusion pass deliberately only runs on Op tokens (`Plus`/`Minus`/`Star`/`Slash`) and the two Name-typed logicals (`||`/`&&`) to avoid colliding with comparison ops which the lexer fuses directly.

### Tests (+4 new, total 78)
- `test_parse_plus_equals_assignment` — `x += 1` carries `+=` token.
- `test_parse_all_arithmetic_compound_operators` — `+=`, `-=`, `*=`, `/=` all parse.
- `test_parse_logical_compound_operators` — `||=`, `&&=` parse.
- `test_parse_compound_assign_with_complex_rhs` — `x += 1 + 2` parses with one `+=` and one `+`.

## [0.16.0] - 2026-05-24

### Added (Phase 6o — ternary `cond ? a : b`)

New grammar layer inserted between `expression` and `range`:

```
expression = ternary ;
ternary    = range [ "?" expression ":" expression ] ;
```

**Precedence**: ternary sits above range (looser than `..`/`...`) and below assignment.  So `a..b ? c : d` parses as `(a..b) ? c : d`, and `x = b ? c : d` parses as `x = (b ? c : d)` (since `assignment` is statement-level, outside the expression hierarchy).

**Right-associativity**: `a ? b : c ? d : e` parses as `a ? b : (c ? d : e)`.  The false-branch recurses through `expression` (→ ternary at the top), so the inner ternary nests inside the outer's else.

**Colon ambiguity**: `:foo` opens a `symbol_literal` in `factor`.  But by the time the ternary's `:` is reached, the true-branch's factor has already completed and control has unwound back to the ternary rule.  The grammar parser consumes the `:` as the ternary separator, not as a symbol opener.

**Token typing**: `?` lexes as a `Name`-typed Op token with value `?` (catch-all in `classify_op_token`); `:` lexes as `TokenType::Colon`.  Grammar matches by value (`"?"`, `":"`) — same trick as `"=>"`, `"<="`, etc.

### Tests (+3 new, total 74)
- `test_parse_simple_ternary` — `x = 1 ? 2 : 3` produces a ternary node carrying `?` and `:`.
- `test_parse_ternary_right_associative` — `x = a ? b : c ? d : e` carries two `?` and two `:` tokens.
- `test_parse_ternary_inside_array_literal` — `[1 ? 2 : 3]` works as an array element.

All tests wrap the ternary in an assignment to dodge the bare-NAME-led statement ambiguity (lessons.md).

## [0.15.0] - 2026-05-24

### Added (Phase 6n — range expressions `..` and `...`)

New grammar layer inserted between `expression` and `logical_or`:

```
expression = range ;
range      = logical_or [ ( "..." | ".." ) logical_or ] ;
```

`expression` is now a transparent wrapper over `range`.  When no `..`/`...` token follows the first `logical_or`, the `range` rule passes through the single operand unchanged — so existing parses for non-range expressions continue to produce the same shape (modulo an extra `range` node in the AST).

**Precedence**: range sits OUTER over `logical_or` because Ruby's `..`/`...` bind *looser* than `||`.  So `a || b .. c || d` parses as `(a || b) .. (c || d)`.  (Test that exercises this with explicit parens — see Test notes below.)

**Non-chainable**: the optional (zero-or-one) repetition is intentional; `1..5..10` is a parse error in Ruby and our grammar matches that by not allowing chaining.

**Token typing**: the lexer's `fuse_range_ops` (Phase 4e) pre-fuses two consecutive `.` tokens into a single `Name`-typed token with value `..` (three → `...`).  The grammar matches these by literal *value* (`"..."`, `".."`), the same trick used for `"=>"`, `"<="`, `"&&"`, etc.

**Endless / beginless ranges**: out of scope for this phase.  The lexer already flags `..`/`...` followed by a closer (era ≥ 2.6), but the parser-side support — `(1..)`, `arr[2..]`, `(..5)` — gets its own phase.

### Tests (+6 new, total 71)
- `test_parse_inclusive_range` — `1..5` produces a `range` node carrying a `..` token and two `logical_or` operands.
- `test_parse_exclusive_range` — `1...5` carries `...`.
- `test_parse_range_in_assignment_rhs` — `x = 1..10` nests the range in an assignment.
- `test_parse_range_with_arithmetic_endpoints` — `1 + 2 .. 10 - 3` proves range binds looser than `+`/`-`.
- `test_parse_range_inside_array_literal` — `[1..5]` works as an array element.
- `test_parse_range_with_paren_logical_operands` — `(a || b)..(c || d)` proves operands can be full logical chains.

Note: a precedence test for the unparenthesised form (`a || b .. c || d`) was attempted but hit the v0 `method_call_no_paren` framework ambiguity (lessons.md).  The arithmetic-endpoints and paren-operand tests cover the precedence ladder around range without tripping that issue.

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
