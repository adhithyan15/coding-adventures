# Changelog — `coding-adventures-macrooct-iir-compiler`

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
