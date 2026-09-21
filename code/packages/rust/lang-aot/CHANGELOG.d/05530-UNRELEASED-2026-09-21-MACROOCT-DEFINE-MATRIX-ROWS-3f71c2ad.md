## Unreleased — MacroOct `@define` matrix rows (PREP01 slice 2)

Five new `Language::MacroOct` corpus rows exercising macro expansion on all
eight backends, taking MacroOct from 9 rows / 72 declared cells to 14 / 112.
Oct's own pinned tuple is deliberately unchanged: PREP01 holds Oct fixed as the
reference MacroOct is checked against, so a slice that moved it would have
invalidated its own oracle.

The rows cover object-like expansion (the shape PREP01's worked example uses),
a function-like macro whose argument is used twice, argument pre-expansion (a
macro invoked inside another macro's argument), the two termination cases in one
program (a self-referential `@define counter counter` plus a function-like name
that must survive without parentheses) over Oct's u8 wrap, and a `@define`
inside a conditional — which proves the engine treats a definition in a skipped
group as inert, since otherwise the untaken branch's value would win.

Each row is paired in `MACROOCT_EXPANSIONS` with the Oct source a reader would
hand-expand it to, and every right-hand side is the **textual** expansion
(`21 + 21`, never `42`). That is deliberate: a twin that folded the constant
would satisfy `macrooct_rows_lower_to_iir_identical_to_hand_expanded_oct`
regardless of what the macro expander actually did.

The rows above only ever asked the preprocessor to *delete* text; these ask it
to synthesise text, so every token Oct's parser sees came out of the expander —
a class of bug that produces a program which still compiles and still prints
something plausible.

`LANG-VM-FEATURE-COVERAGE.md`'s MacroOct row and the pinned tuple in
`feature_coverage_doc_counts_match_programs_source` are updated together, as
VM-061 requires. No new test target and no new `#[test]`, so `BUILD` needs no
change: the existing `macrooct` name filter and the `-- --exact` pinned-counts
filter already run everything added here (VM-065).
