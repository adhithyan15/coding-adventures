---
category: Compiler / VM / language pipeline
---

# When an inventory column changes, run the test suite of every crate that reads that column

**What went wrong.** LANG84 Python 7c slice 3 (py-float-print) changed the
`grammar_subset` column of `code/grammars/python/releases.csv` from
`py-int-print` to `py-float-print` for six releases. It ran the
python-iir-compiler, lang-aot and inventory checks, but not the full
python-parser suite. `python-parser/tests/release_probes.rs` hard-codes the
subset each paired row must claim, so
`every_paired_row_claims_stage_p_for_its_subset` failed on all three CI
operating systems. The same test file had even been extended with float
probes, and its header comment updated, but its assertion had not.

**Fix.** The assertion now derives the expected subset per release: Python 2
releases expect `py2-int-print`, 3.0a1 expects `py-int-print`, and the rest
expect `py-float-print`.

**Next time.** Before pushing a change to an inventory column, grep for the
column name and for its old value (`grammar_subset`, `py-int-print`) across
`code/packages/rust/*/tests`, and run every crate whose tests read it, the
lexer and parser crates included.
