# html-parser fixtures

`html5lib-tree-construction-smoke.dat` is the checked-in Venture smoke corpus
mirrored from the currently audited
`html5lib/html5lib-tests/tree-construction/*.dat` sources.

The format is the upstream tree-construction test format documented in
`html5lib/html5lib-tests/tree-construction/README.md`. Keeping the fixture in
that format lets this crate grow toward WHATWG tree-construction compliance
without inventing a Rust-only test schema.

`html-browser-readiness.json` is a checked parser acceptance corpus for
browser-facing extraction. It verifies that broad HTML documents produce stable
title, base URL/target, metadata, resource, anchor, body text, heading, link,
image, form, and table facts that a browser pipeline can consume before layout
and Paint VM rendering. The fixture also pins resolved URL metadata derived
from `base` hrefs while preserving raw authored link and resource attributes,
and document/body identity and language metadata such as `lang`, `dir`, body
`id`, and tokenized body classes. Document metadata cases pin charset,
viewport, description, application name, referrer/robots/color-scheme policy,
theme colors, refresh URL resolution, and canonical/manifest URLs. Form cases
also pin labels, derived accessible names, owner references,
placeholder/autocomplete hints, and required/readonly/multiple control state.
Global-state descriptor cases pin document/body shell state plus non-form
inert/hidden, editing, drag, spellcheck, translate, accesskey, autofocus, and
related focus metadata.
ARIA descriptor cases pin collection/range/live-region semantics plus details,
error-message, and flow relationship target text.
Inline semantic cases pin machine-readable values, edits, quotes, phrase-level
annotations, ruby annotation nodes, and bidi overrides.
Media cases pin audio/video playback flags and preload/poster metadata.
Script/style cases pin module/classic script kind, async/defer/nomodule flags,
inline script/style text, and loading-policy hints such as integrity,
crossorigin, referrer policy, fetch priority, blocking, disabled state, and
alternate stylesheets. Responsive image cases pin `srcset`/`sizes`, resolved
candidate URLs, `picture/source` media/type hints, lazy loading, decoding, fetch
priority, CORS/referrer policy, usemap, and server-side image-map state. Link
resource cases pin relation-derived resource kinds, `as` hints,
integrity/CORS/referrer policy, fetch priority, blocking, and responsive image
preload hints.

`html-browser-content-tree.json` is a checked parser acceptance corpus for the
browser-facing content-tree projection. It verifies that broad HTML body
content can be filtered into renderable structural nodes such as headings,
blocks, inline runs, links, images, form controls, lists, and tables while
skipping parser shell, head metadata, comments, scripts, styles, and templates.
Where a document base is present, link and replaced-resource nodes also expose
resolved URL fields for downstream navigation and fetch planning. Renderable
nodes may also pin `id`, tokenized `class`, `title`, `lang`, and `dir`
metadata for selector matching and UI policy. Embedded and replaced resources
such as frames, objects, embeds, and media elements also carry resource kind,
resolved source, type hints, media attributes, and authored dimensions. Table
nodes preserve row-group identity, column groups/columns, column and cell spans,
header associations, scopes, and abbreviated header labels for layout and
accessibility code. Text-flow cases also pin paragraph/preformatted roles,
preserved preformatted text runs, list numbering metadata, quote citations, and
line/word/thematic break kinds. Document-outline cases pin heading levels,
sectioning roles, and landmark-like regions such as `main`, `nav`, `aside`,
`header`, and `footer`. Form cases also preserve label associations, accessible
names, owner references, and control-state hints for downstream accessibility
and layout code. Media nodes also preserve playback flags, preload/poster
metadata for fetch and layout planning.

`html-browser-render-tree.json` is a checked parser acceptance corpus for the
browser-facing render-tree input projection. It verifies that the content tree
is converted into stable default display categories such as block, inline,
inline-replaced, list-item, and table display nodes for early layout work. The
browser-facing fixture set also pins control metadata such as value,
disabled/checked/selected state, select option labels, textarea values, and
resolved URL metadata for link and replaced nodes, plus render-node identity
metadata that layout and styling code can carry forward. Replaced resource
nodes are pinned as inline-replaced render inputs with their fetch and
dimension metadata intact. Table render inputs also keep column hints, row-group
identity, spans, scopes, and header metadata intact while mapping to stable table
display categories. Text-flow render inputs preserve list markers, quote
citations, preformatted whitespace policy, and break kinds while mapping to
stable display categories. Document-outline render inputs keep heading levels
and section/landmark metadata while mapping those structural regions to block
display categories. Form accessibility metadata is preserved while controls map
to inline-replaced render inputs. Media playback metadata is preserved while
audio/video nodes map to inline-replaced render inputs.

Validate the checked-in smoke fixture's case boundaries and metadata with:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/check_html5lib_tree_construction_smoke.py \
  --check
```

Validate that every focused WHATWG parser audit fixture has a matching Rust
test that parses the fixture and replays the cases through the DOM-dump harness:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/check_whatwg_audit_rust_tests.py \
  --check
```

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

`whatwg-ruby-audit.json` is a generated index over tree-construction cases
that stress `ruby`, `rb`, `rt`, `rtc`, and `rp` implied-end-tag recovery,
including block descendants and nested ruby containers:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_ruby_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_ruby_audit_fixture.py \
  --check
```

`whatwg-noscript-audit.json` is a generated index over tree-construction cases
that stress `noscript` behavior with scripting enabled and disabled, head
insertion-mode boundaries, comment-looking text, RAWTEXT/PLAINTEXT
descendants, paragraph integration, and stray noscript end tags:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_noscript_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_noscript_audit_fixture.py \
  --check
```

`whatwg-head-body-audit.json` is a generated index over tree-construction
cases that stress `html`/`head`/`body` shell transitions, head metadata
relocation, title/style/script handoff, frameset/body compatibility, and late
shell tags:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_head_body_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_head_body_audit_fixture.py \
  --check
```

`whatwg-void-element-audit.json` is a generated index over tree-construction
cases that stress void element insertion, stray void end tags, table/select
contexts, fragment contexts, foreign-content boundaries, and legacy void-like
elements:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_void_element_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_void_element_audit_fixture.py \
  --check
```

`whatwg-list-item-audit.json` is a generated index over tree-construction
cases that stress `li`, `dt`, and `dd` implied-end-tag recovery, nested list
boundaries, paragraph/list interactions, formatting reconstruction, and
list-in-table recovery:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_list_item_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_list_item_audit_fixture.py \
  --check
```

`whatwg-paragraph-audit.json` is a generated index over tree-construction
cases that stress paragraph implied-end tags, formatting reconstruction,
table/foster-parenting boundaries, form controls, text modes, headings,
special end-tag recovery, and fragment contexts:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_paragraph_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_paragraph_audit_fixture.py \
  --check
```

`whatwg-block-boundary-audit.json` is a generated index over tree-construction
cases that stress grouping, sectioning, list-container, heading, formatting,
table/foster-parenting, foreign-content, template, form, text-mode, ruby,
select/list, and fragment-context block boundaries:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_block_boundary_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_block_boundary_audit_fixture.py \
  --check
```

`whatwg-fragment-context-audit.json` is a generated index over
tree-construction cases that stress table, shell, block, foreign-content,
text-mode, select/list, template, and ordinary fragment parsing contexts:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_fragment_context_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_fragment_context_audit_fixture.py \
  --check
```

`whatwg-character-reference-audit.json` is a generated index over
tree-construction cases that stress named references, numeric references,
ambiguous ampersands, attribute references, parser-driven RCDATA references,
and fragment-context character-reference handling:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_character_reference_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_character_reference_audit_fixture.py \
  --check
```

`whatwg-legacy-element-audit.json` is a generated index over
tree-construction cases that stress `isindex`, obsolete `menuitem`,
`main`/`search`, pending-spec, tricky recovery, and namespace-sensitivity
cases:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_legacy_element_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_legacy_element_audit_fixture.py \
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

`whatwg-misc-recovery-audit.json` is a generated index over tree-construction
cases that stress XML/processing-instruction-looking markup, bogus comments,
CDATA-as-text, malformed tag opens, plain text and whitespace shells, duplicate
doctypes, unknown elements, and legacy compatibility elements:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_misc_recovery_audit_fixture.py
python3 code/packages/rust/html-parser/tests/fixtures/generate_whatwg_misc_recovery_audit_fixture.py \
  --check
```

`check_whatwg_audit_coverage.py` verifies that every source marker in
`html5lib-tree-construction-smoke.dat` is indexed by at least one focused
`whatwg-*-audit.json` parser fixture, and that those audit fixtures do not
contain stale or duplicate source references:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/check_whatwg_audit_coverage.py \
  --check
```

`check_whatwg_audit_manifest.py` verifies that every focused
`whatwg-*-audit.json` parser fixture has a matching generator, that those
generators are wired into the shared generated-fixture manifest, and that audit
metadata such as `format`, `source_fixture`, `case_count`, case fields, and
axis counts stay internally consistent:

```bash
python3 code/packages/rust/html-parser/tests/fixtures/check_whatwg_audit_manifest.py \
  --check
```
