---
category: Testing & coverage
---

# A release-class checker that asserts one subset per language breaks the moment a language gets a second subset

`lang-aot`'s `lang84_rows_run_their_corpus_case` read
`code/grammars/<lang>/release_classes.csv` and asserted every line's `subset`
equals the one under test. That was true while each language had one IIR
subset. LANG84 phase 7c slice 1 gave Python a second subset
(`py2-int-print`), whose sweep rows live in the same CSV, and the helper would
have failed on the first `py2-int-print` line even for the untouched
`py-int-print` rows. The same single-subset assumption sat in
`python-iir-compiler`'s `tests/common` (a `SUBSET` constant), the sweep's
renderer, and the matrix tests that matched every Python row against the
`py-int-print` corpus and the CPython 3 oracle.

**Fix:** the helper skips other subsets' lines; the Python test readers take
the subset by name; matrix checks go through `python_rows_of(subset)`
(by the `case` prefix), with one assertion that every Python row belongs to
exactly one subset.

**Do differently:** a per-language checker over a shared inventory file should
key everything by subset from the start and filter, not assert; adding a
subset (or a profile) should be a data change, not a test rewrite.
