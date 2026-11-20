## Unreleased - CV2-S5a: the oracle derives the v2 conformance cases and kills a mutant list

No version bump, no emitted byte changes, no committed file outside this package (S5b commits the fixture).

### Added

- `src/coding_adventures_warrant_oracle/conformance.py`, the derivation module of VER04r section 11 (R19, R25, R26):
  the strict JSON reader of VER04i 9.1, the section 2.1 type text and the section 2.3 value decoder (built on
  `decode_int_text` and the oracle's stream encoder), evaluation with pure stand-in models and the pure call trace
  for `v2-ir`, a descriptor-JSON to `nth` reader (`desc_from_json`) for `v2-domain`, the chunk-file and
  `slim_chunks` checks of sections 5.5 and 14 and single-chunk enumeration for `v2-chunk`, the three fault wrappers
  over `faults.fault_row` for `v2-fault`, the mapping from the oracle's codes to the kit's closed vocabulary
  (`classify_ir_refusal`), the case envelope (`Row`, `finish_case`: id, summary, `requires`, `expect`), and the
  canonical writer and index rows (`build_files`). The module reads no file.
- `tests/fixtures/conformance-v2/`: the hand-written case tables (`cases_value.py`, `cases_arith.py`, `cases_ir.py`,
  `cases_domain.py`, `cases_chunk.py`, `cases_fault.py`; 724 cases in the six categories, every one with `requires`
  `["ir.core"]`), the two kit documents the `v2-chunk` cases come from (`kit.warrant` with a `2**24` suite and
  `kit-small.warrant` at `chunk_size 1024`, with their profiles, which declare no fault), and `make_conformance.py`
  (`--write` and `--check`, `--fixture DIR`; exit 0 identical, 1 differs, 2 a table and its derivation disagree).
  A table intent that disagrees with the oracle fails the build naming the case id.
- `tests/test_conformance.py` (every function and branch of the module), `tests/test_conformance_tables.py` (the
  ten slugs of section 3.4, every Appendix A tag, every `ir.core` leaf descriptor, every code of every closed error
  set, the S5-0 erratum on `unknown-tag` vectors, the CLI exit codes), `tests/test_conformance_cross.py` (second
  opinions: Python integers, `struct.pack`, a naive chunk enumerator, the formulas of section 5.1, the cost model,
  and the v1 checker's reader and canonical writer) and `tests/test_conformance_mutants.py` (the mutant test of
  [D42]: sixteen named wrong evaluators, each changing a case of each category it is named for).
- `tests/conftest.py`: a session fixture `kit` that derives the whole kit once (about a minute).

### Notes

- Nothing is written to `code/specs/fixtures`; the generated files exist only in a scratch directory until S5b.
- `BUILD` and `BUILD_windows` are unchanged (pytest already runs `tests/`); S5b adds `make_conformance.py --check`.
- Open points found while building, recorded in the fixture README: the depth convention of a type expression and of
  a descriptor tree is avoided at its single ambiguous boundary; the static width of a `Bytes` literal is kept out
  of every result type; the second kit document exists because `chunk_size 1024` cannot hold a `2**24` suite.
