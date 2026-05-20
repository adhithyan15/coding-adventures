# html-parser fixtures

`html5lib-tree-construction-smoke.dat` is the checked-in Venture smoke corpus
mirrored from the currently audited
`html5lib/html5lib-tests/tree-construction/*.dat` sources.

The format is the upstream tree-construction test format documented in
`html5lib/html5lib-tests/tree-construction/README.md`. Keeping the fixture in
that format lets this crate grow toward WHATWG tree-construction compliance
without inventing a Rust-only test schema.

`audit_html5lib_coverage.py` compares this fixture and the sibling lexer
html5lib fixtures against an upstream html5lib-tests checkout by source
signature:

```bash
HTML5LIB_TESTS_ROOT=/path/to/html5lib-tests \
  python3 code/packages/rust/html-parser/tests/fixtures/audit_html5lib_coverage.py
```

The command exits nonzero if any upstream tree-construction or tokenizer source
case is missing, or if the normalized tokenizer fixture still has skipped
runtime gaps.

Use the expectation flags when the audit should also pin the exact upstream and
checked-in corpus sizes used by the current compliance snapshot:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/audit_html5lib_coverage.py \
  /path/to/html5lib-tests \
  --expect-tree-upstream-cases 1778 \
  --expect-tree-local-cases 2485 \
  --expect-tokenizer-upstream-cases 6806 \
  --expect-tokenizer-local-raw-cases 7015 \
  --expect-normalized-cases 7242 \
  --expect-normalized-skipped 0
```

The stable JSON report is checked in as `html5lib-coverage-audit.json`.
Regenerate or check it with:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/audit_html5lib_coverage.py \
  /path/to/html5lib-tests --write-report
python3 code/packages/rust/html-parser/tests/fixtures/audit_html5lib_coverage.py \
  /path/to/html5lib-tests --check-report
```

`whatwg-tree-insertion-audit.json` is a generated index over the high-signal
tree-construction families inside `html5lib-tree-construction-smoke.dat`:
adoption-agency formatting recovery, table insertion and foster parenting,
template insertion, foreign-content fragments, and HTML fragment shells.
Regenerate or check it with:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_tree_insertion_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_tree_insertion_audit_fixture.py \
  --check
```

`whatwg-frameset-audit.json` is a generated index over tree-construction cases
that stress frameset, frame, noframes, body-compatibility, foreign-content, and
template-boundary recovery:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_frameset_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_frameset_audit_fixture.py \
  --check
```

`whatwg-table-audit.json` is a generated index over tree-construction cases
that stress table shells, row groups, cells, captions/colgroups,
foster-parenting, select-in-table recovery, and table fragment contexts:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_table_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_table_audit_fixture.py \
  --check
```

`whatwg-form-interactive-audit.json` is a generated index over
tree-construction cases that stress anchor/nobr recovery, button boundaries,
form-associated controls, select/option handling, textarea RCDATA handoff,
interactive fragment contexts, and stray interactive end tags:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_form_interactive_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_form_interactive_audit_fixture.py \
  --check
```
