# Changelog — `coding-adventures-macrooct-iir-compiler`

## 0.2.0 — 2026-09-21 (PREP01 slice 2)

`@define` works. MacroOct gains a full macro facility — object-like and
function-like, argument pre-expansion, hide-set termination — and gains it
**without this crate learning how to expand anything**: every line of the
expander lives in `source-preprocessor`, and what changed here is one function
that answers "what did the author write".

That division is the slice's result, not its implementation detail. The dialect
grew ~180 lines; the backends grew none.

### Added

- `MacroOctDialect::classify` now returns a complete `Directive::Define`:
  - `@define NAME body…` → object-like (`params: None`). The body may be empty,
    which expands the name to nothing — the idiomatic way to compile a token
    out.
  - `@define NAME(a, b) body…` → function-like (`params: Some([…])`), including
    the zero-parameter `NAME()` form, which is genuinely different from the
    object-like `NAME`: a bare `NAME` does not expand.
  - **C's adjacency rule decides between them.** The `(` must touch the name.
    `@define X (1+2)` is an OBJECT-like macro whose body is `(1+2)`, which is
    the case the rule exists for: a parenthesised body is the commonest macro
    body there is, written precisely so `X + 1` cannot reassociate at the use
    site. Measured on `Token::column`, the same mechanism the slice-1
    misspelled-directive check already used.
- Located `PpError` refusals — never a panic — for a missing name, a
  non-identifier name, an unterminated parameter list, a non-identifier
  parameter, two parameters with no `,` between them, and a **duplicate
  parameter name**. The last is not merely redundant: substitution resolves a
  parameter by position, so `@define F(x, x) x + x` would let the first `x` win
  both slots and `F(1, 2)` would quietly mean `1 + 1`.
- 14 new `@define` program pairs in `tests/iir_identity.rs` (25 total) and a
  sixth `@include` pair covering a "header" of definitions used by its
  includer. Every right-hand side is the **textual** expansion — `21 + 21`,
  never `42` — so a constant-folding frontend cannot satisfy the oracle.
- Six new `lang-aot` matrix rows (15 total, 120 declared cells), each with a
  hand-expanded twin: object-like, an argument used twice, argument
  pre-expansion, the self-referential / bare-function-like-name termination
  pair over Oct's u8 wrap, and a definition inside a conditional.

### Changed

- `is_identifier` is now one shared predicate rather than an inline closure,
  and deliberately checks the **spelling** rather than `TokenType`. MacroOct's
  lexer promotes `in`, `out`, `loop` and friends from `NAME` to keywords; that
  is Oct's business, not the preprocessor's, and C likewise accepts
  `#define F(int) int`. A `TokenType` check would have refused
  `@define WRAP(in) in`.
- The grammar is unchanged. `macrooct.tokens` already had `AT_DEFINE`, and a
  parameter list needs only the existing `LPAREN` / `RPAREN` / `COMMA` / `NAME`
  rules — verified by regenerating `macrooct-lexer/src/_grammar.rs` and
  confirming a byte-identical result, since CI has no Rust grammar
  regeneration check (VM-067).

### The refusal that moved, and one test that had gone quiet

Slice 1 refused **every** `@define` with a located "not supported yet"
diagnostic, and `iir_identity.rs`'s diagnostic-hygiene guard leaned on that: it
drove `@define X 1` expecting a message. As of this slice that program compiles
cleanly, so the case proved nothing — it would have failed loudly on the
"expected to fail but compiled" arm rather than gone silently green, but it
still had to be replaced rather than deleted, because `@define` gained *more*
diagnostic surface here, not less. Four malformed shapes driven with hostile
text took its place (a control character via a string literal, a 5 000-byte
identifier), and **both limbs of the guard were re-verified by mutation**:
disabling escaping in `PpError::quote` fires the control-character assertion on
`define-param-not-identifier-esc`; disabling truncation fires the 2 048-byte
ceiling on `define-unterminated-params-long`. The Display-not-Debug assertion
and the 2 048 ceiling are unchanged.

### VM-068: controlling expressions are macro-expanded (found here, fixed in the engine)

After `@define LED_PORT 1`, the line `@if LED_PORT == 1` now takes the true
branch. Until this slice it read `LED_PORT` as an *undefined* name, took the
`@else` branch, and compiled to the wrong thing — silently, with no
diagnostic. PREP01 §7's own worked example is exactly that shape, so the
spec's canonical illustration of the feature was broken.

Fixed in the engine rather than the dialect, because no dialect could fix it:
`eval_condition` receives a bare token slice with neither the macro table nor
any expansion applied. The grouping-depth pre-scan now runs twice — once on
the raw tokens, once after expansion, since a macro body can introduce
grouping the source text did not have.

Proved by a matched *pair* of matrix rows, because either alone is satisfied
by a broken implementation: an always-zero evaluator passes the undefined-name
row, and an always-truthy one passes the defined-name row.

### Tests

57 unit tests plus 7 integration tests, all passing. New load-bearing ones:

- `a_glued_paren_makes_a_function_like_macro_and_a_detached_one_does_not` —
  written as a pair on purpose: an implementation that ignored adjacency
  satisfies the first half, and one that never recognised a parameter list
  satisfies the second.
- `every_malformed_parameter_list_is_a_diagnostic_and_never_a_panic` — ten
  shapes, each asserted against the *reason* it was refused, not merely that it
  was.
- `a_duplicate_parameter_is_refused_rather_than_silently_dropping_an_argument`.
- `a_keyword_spelling_is_still_a_legal_parameter_name`.
- `a_define_inside_a_skipped_group_never_becomes_visible` — the engine's guard,
  which became load-bearing the moment the `Define` arm started installing a
  definition instead of refusing one. Without it `@if 0` would define.
- `a_macro_defined_in_an_included_file_is_visible_to_the_includer` — the table
  is per translation unit, which is what makes a "header" of definitions work.
- `a_definition_does_not_reach_back_up_its_own_line_or_the_lines_before_it`.


## 0.1.0 — 2026-09-21 (PREP01 slice 1)

First release. The MacroOct frontend for the LANG VM AOT chain, and the
composition PREP01 slice 1 exists to test: the generic `source-preprocessor`
engine in front of **Oct's unchanged parser grammar, type checker and
`compile_ast`**.

Nothing in Oct or Nib changed. Oct is the reference MacroOct is checked
against, and the slice's acceptance criterion — identical IIR — is a claim you
cannot make about a language you are also editing.

### Added

- `MacroOctDialect`, implementing `source_preprocessor::Dialect`:
  - `classify` — `@include "path"` (quotes stripped here, since they are
    lexical and the engine knows no language's string syntax), `@if <expr>`,
    `@else`, `@end`, `@define NAME body`.
  - `eval_condition` — integers `0`–`255` in Oct's decimal/hex/binary
    spellings, `true`/`false`, names (undefined → 0), the six comparisons,
    `&&`, `||` and parentheses, with Oct's own precedence. Anything else is a
    diagnostic rather than a guess; an out-of-range literal is refused rather
    than truncated.
  - `lex` — included text through MacroOct's own grammar, so directives inside
    an include are directives.
  - `stringize` / `paste` left at the trait's `None` defaults. MacroOct has
    neither, and declining them is a positive test that the engine does not
    assume a C-shaped macro facility.
- `compile_source`, `compile_source_with_includes` and `preprocess_source`.
- `src/_oct_grammar.rs` — a compiled copy of `oct.grammar`, and
  `MAX_RULE_DEPTH` — a copy of `oct-parser`'s constant. Both needed because
  `oct-parser` keeps them private and PREP01 forbids editing Oct to expose
  them; both guarded by a test so the copies cannot drift.
- `tests/iir_identity.rs` — **the acceptance criterion**: 11 directive program
  pairs and 5 `@include` pairs, each compiled as MacroOct and as hand-expanded
  Oct and compared field for field via `{:#?}` (not a hand-written field walk,
  which would silently stop covering any field added to `IIRModule` later).

### Two design decisions worth recording

**The evaluator is iterative, not recursive.** A dialect parses
attacker-influenced text, and in Rust a stack overflow is an *abort* —
uncatchable, fatal to an embedding host. The engine does pre-scan a condition's
grouping depth before calling `eval_condition`, but a dialect whose safety
depends on a check in another crate is one refactor away from unsafe.
`deep_grouping_terminates_without_touching_the_machine_stack` feeds it 100 000
nested parentheses.

**The lexer's `Eof` sentinel is stripped before preprocessing and restored
after.** `GrammarLexer` appends one to every stream, and the engine — correctly
— knows nothing about that. Left in place it does two kinds of damage: a
sentinel sharing a line with a closing `@end` (which any file without a final
newline produces) joins that directive's logical line and makes it look like
`@end` plus trailing junk; and a sentinel arriving from an `@include`d file is
spliced into the **middle** of the stream, where Oct's parser stops early and
silently compiles a truncated program. Both have regression tests.

### Two real bugs this slice found and fixed

1. **Misspelled directives were read as correct ones.** The directive rules are
   literal patterns and the lexer matches a literal by prefix with no
   word-boundary requirement, so `@endif` lexes as `@end` + `if` and `@ifdef`
   as `@if` + `def`. Most such spellings landed somewhere safe by luck, but
   `@ifdef` alone is the condition `def` — one undefined name, which evaluates
   to 0 — so the group was **silently skipped with no diagnostic**, and `@if1`
   read as `@if 1` and silently *took* the branch. `classify` now refuses a
   token glued directly to a directive with no space, naming the misspelling
   and (for `@endif` specifically) the right spelling. Only a glued
   alphanumeric counts, so `@if(1)` and `@include"x"` keep working.
2. **A precedence test that could not fail.** The first
   `parentheses_override_precedence` compared two expressions that evaluate the
   same under either reading. Replaced with a pair that actually distinguishes
   them.

### Tests

49 unit tests plus 6 integration tests. The load-bearing ones:

- `macrooct_and_oct_agree_on_everything_except_instruction_provenance` — the
  criterion, over 11 program pairs.
- `included_macrooct_and_hand_expanded_oct_agree` — the same, over 5 include
  shapes (single, conditional content, nested chain, two siblings, and an
  include selected by a conditional).
- `line_aligned_sources_produce_byte_identical_iir_including_provenance` —
  removes the provenance exclusion by padding the Oct source until the line
  numbers coincide, then compares everything. This is what turns the exclusion
  above from a loophole into a measurement; it also asserts that *without* the
  padding the maps do differ, so the exclusion is never silently unnecessary.
- `a_directive_free_program_compiles_byte_identically_to_oct` — the degenerate
  case, provenance included.
- `an_include_inside_a_skipped_group_is_never_resolved` and
  `a_condition_inside_a_skipped_group_is_never_evaluated` — the engine's rule
  that a skipped group gets nesting tracking only. Proven positively: the
  skipped include names a file that does not exist, so compiling cleanly is
  evidence it was never attempted.
- `an_include_cycle_is_refused_rather_than_looping_forever`.
- `the_embedded_oct_grammar_is_byte_identical_to_oct_parsers` and
  `the_parser_depth_cap_matches_the_one_oct_parser_documents` — the two
  Oct-is-frozen copies.
