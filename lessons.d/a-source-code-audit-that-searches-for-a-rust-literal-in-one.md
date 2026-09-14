---
category: Testing & coverage
---

# A source-code audit that searches for a Rust literal in one escaping form silently reports guarded code as unguarded

A mutant survived the `water-movement-route` suite: truncating ONE row's copy of a
`source` span that three rows shared left every test green, because the other two
copies kept the whole string alive in stdout. Reasonable next question — which other
tables are in that position?

An audit was written to answer it: flag any `.adj` with a span carried by 2+ rows whose
test file has no set-wise read of the shipped file. It flagged **ten** tables.

All ten were wrong. The audit looked for `strip_prefix("        source \"")`, but the
shipped guards are written with a **raw** string literal —
`line.strip_prefix(r#"        source ""#)`. Same bytes on disk in the `.adj`, entirely
different bytes in the `.rs`. Every table carrying
`the_shared_spans_are_exactly_the_declared_ones` was classified as unguarded. A second
bug in the same script located test files by transforming the `.adj` filename
(`body-counts.adj` → `facts_bodycounts_e2e.rs`), so two tables whose tests live under
other names (`facts_anatomy_e2e.rs`, `facts_aqi_range_e2e.rs`) were reported as having
no test at all.

It was caught by **running the prediction instead of publishing it.** Three flagged
tables were mutated for real — truncate one copy of a span shared by 7, 6 and 4 rows —
and all three suites went red, naming `the_shared_spans_are_exactly_the_declared_ones`
as the killer. After fixing both instrument bugs the audit flagged exactly one table;
mutating that one killed it too. Final count: 22 tables with per-row spans, zero
unguarded. Had the ten-table census been posted first, it would have been a fabricated
defect report against four other agents' work.

**Rule:** a classifier over source code needs a positive AND a negative control before
its output is believed — the fixed audit ends by asserting that a file known to carry
the guard is reported guarded, and that the same text with the guard idiom deleted is
reported unguarded. The first version could only ever return "unguarded" and had no way
to notice. Search for a literal in every escaping the language admits (Rust alone has
`"...\"..."`, `r"..."`, `r#"..."#`, and continuation backslashes that swallow the
following indent), or better, match on a token containing no quote at all. Locate
related files by content — ask which test file *names* the artifact — because naming
conventions have exceptions and the exceptions are silent. And treat a static
prediction as a list of candidates to measure, never as findings: run the mutant on at
least one before reporting a count anywhere a person will read it.

**Corollary, same session, from the CI failure this file itself caused:** a lesson shard
carries exactly ONE heading. Its `#` title becomes `###` in the rendered aggregate, and
a shard's own `##` sub-heading demotes to `###` too — indistinguishable from a lesson
title. Three `##` sections here made `test_render_has_exactly_one_heading_per_lesson`
report 515 headings against 512 lessons. Use bold lead-ins and flowing prose, the way
every other shard does.

Related: `assert-structure-not-substrings` — four tests once passed while proving
nothing, for the same underlying reason: one side of the comparison was another test
literal rather than the artifact.
