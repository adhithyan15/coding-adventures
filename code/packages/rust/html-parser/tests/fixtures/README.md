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

`whatwg-text-control-audit.json` is a generated index over tree-construction
cases that stress script RAWTEXT, title/textarea RCDATA, RAWTEXT elements,
noscript scripting modes, PLAINTEXT, pre/listing initial newlines, text-control
fragment contexts, and stray text-control end tags:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_text_control_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_text_control_audit_fixture.py \
  --check
```

`whatwg-foreign-audit.json` is a generated index over tree-construction cases
that stress SVG, MathML, foreign-content fragments, HTML integration points,
and table/foreign-content boundaries:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_foreign_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_foreign_audit_fixture.py \
  --check
```

`whatwg-formatting-audit.json` is a generated index over tree-construction
cases that stress active formatting elements, adoption-agency recovery,
paragraph/list implied end tags, ruby scopes, headings, and formatting
reconstruction:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_formatting_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_formatting_audit_fixture.py \
  --check
```

`whatwg-document-shell-audit.json` is a generated index over tree-construction
cases that stress doctypes, comments, `html`/`head`/`body` synthesis, frameset
boundaries, and shell fragment contexts:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_document_shell_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_document_shell_audit_fixture.py \
  --check
```

`whatwg-template-audit.json` is a generated index over tree-construction cases
that stress template insertion modes, nested template stacks, EOF recovery,
and template interactions with tables, selects, framesets, document shells,
text modes, foreign content, and template fragment contexts:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_template_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_template_audit_fixture.py \
  --check
```

`whatwg-select-list-audit.json` is a generated index over tree-construction
cases that stress select shells, adjacent option implied-end recovery, optgroup
boundaries, select-in-table handling, select fragment contexts, and stray
select/list end tags:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_select_list_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_select_list_audit_fixture.py \
  --check
```
