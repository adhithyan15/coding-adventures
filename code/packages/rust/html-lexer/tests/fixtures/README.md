# HTML Lexer Conformance Fixtures

These JSON files are the repo-native conformance layer for the Rust HTML lexer.
They are intentionally separate from the authored state-machine TOML so we can:

- test the public Rust wrapper against a shared corpus
- run the same cases against bootstrap and generated constructors
- grow coverage without making production code load text fixtures at runtime

## Format

Each file is a JSON object with this shape:

```json
{
  "format": "venture-html-lexer-fixtures/v1",
  "suite": "html1",
  "description": "human-readable summary",
  "cases": [
    {
      "id": "stable-case-id",
      "description": "human-readable case summary",
      "input": "<P>Hello</P>",
      "tokens": [
        "StartTag(name=p, attributes=[], self_closing=false)",
        "Text(data=Hello)",
        "EndTag(name=p)",
        "EOF"
      ],
      "initial_state": "optional tokenizer state context",
      "last_start_tag": "optional tokenizer tag context",
      "current_start_tag": "optional in-progress start-tag token context",
      "current_end_tag": "optional in-progress end-tag token context",
      "current_doctype": "optional in-progress doctype token context",
      "temporary_buffer": "optional tokenizer temporary-buffer context",
      "return_state": "optional tokenizer return-state context",
      "diagnostics": ["optional-diagnostic-code"]
    }
  ]
}
```

`tokens` and `diagnostics` are summarized strings so the corpus stays portable
across generated constructors and future language ports. Rust tests deserialize
these files with `include_str!`, so the fixtures are compiled into the test
binary while production code continues to link only static Rust source.

## Current Suites

- `html-skeleton.json`: narrow bootstrap regression cases
- `html1.json`: Mosaic-era compatibility-floor cases for the current default wrapper
- `whatwg-entities.json`: generated HTML Standard named character reference
  table used by Rust tests to exercise every entry in data, attribute, and
  RCDATA contexts
- `whatwg-numeric-references.json`: generated HTML Standard numeric character
  reference edge table used by Rust tests to exercise decimal, hexadecimal,
  semicolon, and missing-semicolon forms across data, attribute, and RCDATA
  contexts
- `whatwg-character-reference-boundaries.json`: generated HTML tokenizer
  character-reference boundary cases used by Rust tests to pin text,
  attribute, RCDATA, and seeded named/numeric continuation recovery
- `whatwg-attribute-boundaries.json`: generated HTML tokenizer attribute
  boundary cases used by Rust tests to pin seeded start-tag and
  current-attribute continuation recovery
- `whatwg-input-stream.json`: generated HTML Standard input-stream
  preprocessing cases used by Rust tests to exercise CRLF and bare-CR
  normalization across tokenizer contexts and chunk boundaries
- `whatwg-chunk-boundaries.json`: generated HTML tokenizer streaming cases
  used by Rust tests to prove tokens, diagnostics, and positions stay stable
  across every character chunk split point
- `whatwg-eof-recovery.json`: generated HTML tokenizer EOF recovery cases used
  by Rust tests to pin unfinished tags, comments, doctypes, character
  references, text modes, and seeded continuation states
- `whatwg-text-mode-delimiters.json`: generated HTML tokenizer text-mode
  delimiter cases used by Rust tests to pin RCDATA, RAWTEXT, script-data,
  escaped-script, and seeded end-tag continuation recovery
- `whatwg-text-mode-boundaries.json`: generated HTML tokenizer text-mode
  boundary cases used by Rust tests to pin parser-seeded RCDATA, RAWTEXT,
  PLAINTEXT, and text-mode continuation recovery
- `whatwg-script-escape-boundaries.json`: generated HTML tokenizer script
  escape boundary cases used by Rust tests to pin escaped/double-escaped
  script data, NULL, EOF, and seeded continuation recovery
- `whatwg-cdata-boundaries.json`: generated HTML tokenizer CDATA boundary
  cases used by Rust tests to pin foreign-content delimiter recovery, NULL,
  EOF, HTML-content fallback, and seeded bracket/end states
- `whatwg-markup-declarations.json`: generated HTML tokenizer markup
  declaration cases used by Rust tests to pin comment, bogus-comment, CDATA,
  DOCTYPE, and seeded declaration continuation recovery
- `whatwg-comment-boundaries.json`: generated HTML tokenizer comment boundary
  cases used by Rust tests to pin nested-comment recovery, pending dash and
  end-bang handling, bogus comments, EOF, NULL replacement, and seeded comment
  continuations
- `whatwg-attribute-edges.json`: generated HTML tokenizer attribute-edge cases
  used by Rust tests to pin quoted/unquoted values, duplicate attributes,
  missing whitespace recovery, NULL replacement, self-closing delimiters, and
  end-tag attribute diagnostics
- `whatwg-tag-open-recovery.json`: generated HTML tokenizer tag-open recovery
  cases used by Rust tests to pin ordinary tags, ASCII casing, tag whitespace,
  invalid openers, NULL replacement in tag names, and EOF partial-token drops
- `whatwg-doctype-boundaries.json`: generated HTML tokenizer DOCTYPE boundary
  cases used by Rust tests to pin keyword/name whitespace, PUBLIC/SYSTEM
  identifier recovery, force-quirks transitions, EOF, NULL replacement, and
  seeded continuation contexts
- `html5lib-smoke.json`: generated normalized Venture fixture corpus derived from
  the raw html5lib-style smoke file
- `upstream-html5lib-smoke.test`: raw html5lib-style tokenizer cases used to
  exercise the normalization path toward broader upstream corpora
- `normalize_html5lib_fixtures.py`: importer that lowers supported raw
  html5lib-style tokenizer cases into Venture's portable fixture schema
- `check_html5lib_tokenizer_coverage.py`: checker that proves every raw local
  html5lib tokenizer smoke case maps to the expected normalized fixture case
  or skipped-case marker, with no stale or duplicate normalized IDs
- `check_whatwg_lexer_fixture_metadata.py`: checker that keeps the generated
  WHATWG lexer fixture files, generator pairs, metadata formats, and
  stale-check manifest wiring aligned
- `check_html_fixture_case_ids.py`: checker that keeps checked-in lexer and
  parser fixture case identities unique, stable, and aligned with parser audit
  count metadata
- `check_html_fixture_schemas.py`: checker that makes the checked-in lexer and
  parser fixture JSON schema contracts explicit for tokenizer, input-stream,
  chunk-boundary, numeric-reference, and parser-audit corpora
- `check_html_fixture_format_registry.py`: checker that keeps every
  format-bearing lexer/parser fixture JSON file explicitly registered with its
  expected format string and fixture category
- `check_html_fixture_readme_inventory.py`: checker that keeps the lexer and
  parser fixture READMEs aligned with user-facing fixture data and command
  scripts
- `check_whatwg_lexer_rust_tests.py`: checker that keeps every focused WHATWG
  lexer fixture paired with a Rust test that parses the fixture and exercises
  the lexer harness
- `check_generated_html_fixtures.py`: manifest check that runs every
  self-contained generated lexer/parser fixture stale check from checked-in
  inputs, with optional flags for upstream-only source inputs
- `test_generated_html_fixture_manifest.py`: regression test that keeps the
  stale-check manifest aligned with the self-contained WHATWG fixture
  generators in this directory
- the parser fixture audit can pin the current html5lib tokenizer counts with
  `--expect-tokenizer-upstream-cases 6806`,
  `--expect-tokenizer-local-raw-cases 7015`,
  `--expect-normalized-cases 7242`, and `--expect-normalized-skipped 0`
- `generate_whatwg_entities_fixture.py`: importer that lowers the HTML
  Standard's `entities.json` table into the checked-in `whatwg-entities.json`
  fixture
- `generate_whatwg_numeric_references_fixture.py`: importer that generates the
  finite numeric character reference edge table for the checked-in
  `whatwg-numeric-references.json` fixture
- `generate_whatwg_character_reference_boundaries_fixture.py`: importer that
  generates focused character-reference boundary cases for the checked-in
  `whatwg-character-reference-boundaries.json` fixture
- `generate_whatwg_attribute_boundaries_fixture.py`: importer that generates
  focused attribute boundary cases for the checked-in
  `whatwg-attribute-boundaries.json` fixture
- `generate_whatwg_input_stream_fixture.py`: importer that generates finite
  CRLF/bare-CR preprocessing cases for the checked-in
  `whatwg-input-stream.json` fixture
- `generate_whatwg_chunk_boundaries_fixture.py`: importer that generates
  streaming chunk-boundary invariance cases for the checked-in
  `whatwg-chunk-boundaries.json` fixture
- `generate_whatwg_eof_recovery_fixture.py`: importer that generates EOF
  recovery cases for the checked-in `whatwg-eof-recovery.json` fixture
- `generate_whatwg_text_mode_delimiters_fixture.py`: importer that generates
  text-mode end-tag delimiter cases for the checked-in
  `whatwg-text-mode-delimiters.json` fixture
- `generate_whatwg_text_mode_boundaries_fixture.py`: importer that generates
  focused text-mode boundary cases for the checked-in
  `whatwg-text-mode-boundaries.json` fixture
- `generate_whatwg_script_escape_boundaries_fixture.py`: importer that
  generates focused script escape boundary cases for the checked-in
  `whatwg-script-escape-boundaries.json` fixture
- `generate_whatwg_cdata_boundaries_fixture.py`: importer that generates
  focused CDATA boundary cases for the checked-in
  `whatwg-cdata-boundaries.json` fixture
- `generate_whatwg_markup_declarations_fixture.py`: importer that generates
  markup declaration recovery cases for the checked-in
  `whatwg-markup-declarations.json` fixture
- `generate_whatwg_comment_boundaries_fixture.py`: importer that generates
  focused comment and bogus-comment boundary cases for the checked-in
  `whatwg-comment-boundaries.json` fixture
- `generate_whatwg_attribute_edges_fixture.py`: importer that generates
  attribute-edge recovery cases for the checked-in
  `whatwg-attribute-edges.json` fixture
- `generate_whatwg_tag_open_recovery_fixture.py`: importer that generates
  tag-open recovery cases for the checked-in
  `whatwg-tag-open-recovery.json` fixture
- `generate_whatwg_doctype_boundaries_fixture.py`: importer that generates
  focused DOCTYPE boundary cases for the checked-in
  `whatwg-doctype-boundaries.json` fixture

Regenerate or verify the WHATWG entity fixture with:

```bash
curl -L https://html.spec.whatwg.org/entities.json -o /tmp/entities.json
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_entities_fixture.py \
  /tmp/entities.json
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_entities_fixture.py \
  /tmp/entities.json --check
```

Verify local html5lib tokenizer normalization coverage with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/check_html5lib_tokenizer_coverage.py \
  --check
```

Verify local WHATWG lexer fixture metadata and manifest coverage with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/check_whatwg_lexer_fixture_metadata.py \
  --check
python3 code/packages/rust/html-lexer/tests/fixtures/check_html_fixture_case_ids.py \
  --check
python3 code/packages/rust/html-lexer/tests/fixtures/check_html_fixture_schemas.py \
  --check
python3 code/packages/rust/html-lexer/tests/fixtures/check_html_fixture_format_registry.py \
  --check
python3 code/packages/rust/html-lexer/tests/fixtures/check_html_fixture_readme_inventory.py \
  --check
python3 code/packages/rust/html-lexer/tests/fixtures/check_whatwg_lexer_rust_tests.py \
  --check
```

Regenerate or verify the WHATWG numeric-reference fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_numeric_references_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_numeric_references_fixture.py \
  --check
```

Regenerate or verify the WHATWG character-reference boundary fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_character_reference_boundaries_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_character_reference_boundaries_fixture.py \
  --check
```

Regenerate or verify the WHATWG input-stream preprocessing fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_input_stream_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_input_stream_fixture.py \
  --check
```

Regenerate or verify the WHATWG chunk-boundary fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_chunk_boundaries_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_chunk_boundaries_fixture.py \
  --check
```

Regenerate or verify the WHATWG EOF recovery fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_eof_recovery_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_eof_recovery_fixture.py \
  --check
```

Regenerate or verify the WHATWG text-mode delimiter fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_text_mode_delimiters_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_text_mode_delimiters_fixture.py \
  --check
```

Regenerate or verify the WHATWG script escape boundary fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_script_escape_boundaries_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_script_escape_boundaries_fixture.py \
  --check
```

Regenerate or verify the WHATWG CDATA boundary fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_cdata_boundaries_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_cdata_boundaries_fixture.py \
  --check
```

Regenerate or verify the WHATWG markup declaration fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_markup_declarations_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_markup_declarations_fixture.py \
  --check
```

Regenerate or verify the WHATWG comment boundary fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_comment_boundaries_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_comment_boundaries_fixture.py \
  --check
```

Regenerate or verify the WHATWG attribute-edge fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_attribute_edges_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_attribute_edges_fixture.py \
  --check
```

Regenerate or verify the WHATWG tag-open recovery fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_tag_open_recovery_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_tag_open_recovery_fixture.py \
  --check
```

Regenerate or verify the WHATWG DOCTYPE boundary fixture with:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_doctype_boundaries_fixture.py
python3 code/packages/rust/html-lexer/tests/fixtures/generate_whatwg_doctype_boundaries_fixture.py \
  --check
```

## Conformance Audit

The parser package owns a shared upstream source-coverage audit that checks WPT
tree-construction resources, this raw html5lib tokenizer mirror, and the
generated normalized tokenizer corpus together:

```bash
HTML5LIB_TESTS_ROOT=/path/to/html5lib-tests \
WPT_ROOT=/path/to/wpt \
  python3 code/packages/rust/html-parser/tests/fixtures/audit_html5lib_coverage.py \
  --expect-tree-missing 156 \
  --expect-tokenizer-missing 0
```

Without exact missing-case expectations, the command exits nonzero if an
upstream tokenizer or tree-construction case is absent locally. The current
checked debt is 156 WPT tree cases and zero html5lib tokenizer cases.
`html5lib-smoke.json` runtime skips always fail.

The same audit can verify the checked
`html-parser/tests/fixtures/html5lib-coverage-audit.json` report with
`--check-report`, so fixture-count and missing-source drift stays visible in CI.

For a single local stale check over all checked-in generated HTML fixtures that
do not require fresh upstream downloads, run:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/check_generated_html_fixtures.py
```

Pass `--html5lib-tests /path/to/html5lib-tests --wpt-tests /path/to/wpt` to
include the parser coverage audit report and pinned-count checks, or
`--entities-json /path/to/entities.json` to include the generated WHATWG
named-character-reference table.

To verify that new self-contained WHATWG fixture generators are added to the
manifest, run:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/test_generated_html_fixture_manifest.py
```

## WPT Path

The next layer normalizes WHATWG/WPT or html5lib-style tokenizer coverage into
this same schema instead of making the Rust test harness understand raw
upstream files directly. That keeps the runtime boundary stable while still
letting us mirror broader living-standard cases.

`upstream-html5lib-smoke.test` is the first concrete step in that direction. It
uses the tokenizer JSON structure documented by the html5lib tokenizer tests:
top-level `tests`, with each test carrying `description`, `input`, `output`,
optional `initialStates`, optional `lastStartTag`, and optional `errors`.
For seeded continuation states that upstream fixtures cannot fully describe,
this smoke file also accepts `currentEndTag` and `temporaryBuffer` extension
fields; the normalizer lowers them to `current_end_tag` and `temporary_buffer`.
Comment continuation fixtures likewise accept a `currentComment` extension
field, lowered to `current_comment`, so html5lib-style comment substates can
resume with an already-created comment token.
DOCTYPE continuation fixtures accept a `currentDoctype` extension object,
lowered to `current_doctype`, with optional `name`, `public_identifier`,
`system_identifier`, and `force_quirks` fields for the partial doctype token.
Character-reference continuation fixtures accept `temporaryBuffer` plus
`returnState`, lowered to `temporary_buffer` and `return_state`, so seeded
named/numeric reference substates can recover back into data or RCDATA.
Start-tag and attribute continuation fixtures accept a `currentStartTag`
extension object, lowered to `current_start_tag`, with a required `name`, an
optional `attributes` list, an optional `current_attribute`, and an optional
`self_closing` flag.

`normalize_html5lib_fixtures.py` is the checked-in importer for this shape. It
currently supports:

- default data-state cases
- explicit `initialStates: ["Data state"]`
- `initialStates: ["RCDATA state"]` together with `lastStartTag`
- `initialStates: ["RAWTEXT state"]` together with `lastStartTag`
- `initialStates: ["PLAINTEXT state"]`
- `initialStates: ["CDATA section state"]`
- CDATA section `bracket` and `end` substates
- character-reference, named-reference, numeric-reference, decimal-reference,
  and hexadecimal-reference substates together with `temporaryBuffer` and
  `returnState`
- comment `start`, `start dash`, body, less-than-sign, less-than-sign bang,
  less-than-sign bang dash, less-than-sign bang dash dash, end-dash, end,
  end-bang, and bogus-comment substates together with `currentComment`
- DOCTYPE keyword/name, public/system keyword, public/system identifier,
  after-identifier, and bogus-doctype substates together with `currentDoctype`
- `initialStates: ["Script data state"]` together with `lastStartTag`
- script `less-than sign`, `escape start`, and `escape start dash` substates
  together with `lastStartTag`
- `initialStates: ["Script data escaped state"]` together with `lastStartTag`
- script escaped `dash`, `dash dash`, and `less-than sign` substates together
  with `lastStartTag`
- script double-escape `start` and `end` substates together with
  `lastStartTag`
- `initialStates: ["Script data double escaped state"]` together with
  `lastStartTag`
- script double-escaped `dash`, `dash dash`, and `less-than sign` substates
  together with `lastStartTag`
- RCDATA, RAWTEXT, script-data, and script-data-escaped end-tag `name`,
  `whitespace`, `attributes`, and `self-closing` continuation substates
  together with `lastStartTag`, `currentEndTag`, and `temporaryBuffer`
- generic `tag open` and `end tag open` states, plus seeded start-tag `name`,
  attribute name/value, after-attribute-value, and `self-closing` continuation
  substates together with `currentStartTag`
- multi-state html5lib fixture entries for supported states, expanded into
  stable per-state Venture fixture cases
- `StartTag`, `EndTag`, `Character`, `Comment`, and `DOCTYPE` output tokens
- html5lib start-tag self-closing booleans
- named character references in data, RCDATA, and attribute values for the
  current shared entity subset
- legacy named character references `nbsp`, `copy`, and `reg` in data, RCDATA,
  and attribute values
- HTML4 math/symbol named character references such as `alefsym`, `oline`,
  `sum`, and `notin`
- WHATWG named character references for spacing, invisible operators,
  punctuation aliases, and math constants
- WHATWG relation/operator named character references, including negated
  aliases with combining-overlay replacements
- WHATWG extended greater-than and less-than comparison named character
  references, including `gl*`, `gn*`, `gtr*`, `less*`, `ln*`, and nested
  negated aliases
- WHATWG equality, congruence, similarity, vertical-bar, and parallel named
  character references, including `eq*`, `sim*`, `mid*`, and `par*` aliases
- WHATWG precedence and successor relation named character references, including
  uppercase, `pr*`, `prec*`, `sc*`, and `succ*` aliases
- WHATWG arrow and vector named character references, including long, bar, tee,
  map, and vector-bar aliases
- extended WHATWG arrow aliases, including short/capital arrows, lowercase long
  arrows, hooks, tails, loops, harpoons, negated arrows, diagonals, and mapsto
  forms
- WHATWG Greek variant and letter-like named character references, including
  epsilon, kappa, phi, rho, sigma, theta, digamma, beth, gimel, and daleth
- WHATWG set, membership, subset/superset, square-set, and n-ary logic named
  character references
- WHATWG operator, square, lozenge, star, suit, and symbol named character
  references
- WHATWG box-drawing named character references, including double-line,
  mixed-line, light-line, crossing, and corner aliases
- WHATWG angle, bracket, floor/ceiling, triangle, corner, and over/under fence
  named character references
- WHATWG Latin Extended and diacritic named character references, including
  macron, breve, ogonek, caron, cedilla, circumflex, dot, ring, and tilde forms
- WHATWG mathematical alphabet named character references, including open-face,
  script, and fraktur uppercase/lowercase aliases
- WHATWG Cyrillic named character references, including core and extended
  uppercase/lowercase aliases
- remaining WHATWG arrow, vector, harpoon, fish-tail, and negated arrow named
  character references
- remaining WHATWG set-algebra named character references, including cap/cup,
  square-set, subset/superset, and negated aliases
- remaining WHATWG operator, integral, dot, plus/times, and circled operator
  named character references
- final generated coverage for all remaining semicolon-terminated WHATWG named
  character references
- missing-semicolon recovery for legacy named character references `nbsp`,
  `copy`, and `reg` before delimiters and EOF
- missing-semicolon named-reference recovery limited to WHATWG legacy
  no-semicolon aliases, including fallback from longer modern names to shorter
  legacy prefixes such as `not`
- form-feed handling as an HTML ASCII-whitespace delimiter for script double
  escape and semicolonless legacy named character references
- CRLF and bare-CR input-stream newline preprocessing before tokenization
- generated CRLF and bare-CR input-stream preprocessing coverage across data,
  markup, attributes, comments, doctypes, character references, RCDATA,
  RAWTEXT, script data, PLAINTEXT, CDATA, seeded comment continuations, seeded
  DOCTYPE continuations, all chunk split points, and diagnostic line/column
  positions
- generated chunk-boundary invariance coverage across data, Unicode text, tags,
  attributes, comments, doctypes, named/numeric character references, RCDATA,
  RAWTEXT, script data, PLAINTEXT, CDATA, and seeded continuation states
- generated EOF recovery coverage across partial tags, attributes, comments,
  bogus comments, doctypes, named/numeric character references, RCDATA,
  RAWTEXT, script data, PLAINTEXT, CDATA, and seeded continuation states
- generated text-mode delimiter coverage across RCDATA, RAWTEXT, script data,
  escaped script data, matching/mismatched apparent end tags, recoverable
  whitespace/attribute/solidus delimiters, and seeded end-tag continuations
- generated text-mode boundary coverage across parser-seeded RCDATA, RAWTEXT,
  and PLAINTEXT less-than recovery, end-tag-open/name continuations, NULL/EOF
  recovery, character-reference differences, and literal markup preservation
- generated attribute boundary coverage across seeded start-tag and
  current-attribute continuation states, quoted/unquoted values, EOF recovery,
  missing-whitespace recovery, and self-closing boundaries
- generated attribute-edge coverage across quoted/unquoted values, duplicate
  attributes, missing whitespace recovery, unexpected attribute characters,
  NULL replacement, self-closing delimiters, unexpected solidus recovery, and
  end-tag attributes
- generated tag-open recovery coverage across ordinary tags, ASCII casing,
  HTML whitespace delimiters, invalid tag openers, NULL replacement in tag
  names, and EOF partial-token drops
- EOF recovery for unfinished ordinary start and end tags, ensuring partial
  tokens are dropped before the parser sees them
- EOF recovery for unfinished attribute character references, including named
  and numeric forms
- generic named-character-reference scanning with literal fallback for unknown
  names
- PUBLIC/SYSTEM DOCTYPE recovery diagnostics for missing whitespace, missing
  identifier quotes, and abrupt identifier termination
- longest-prefix named-character-reference recovery for text and RCDATA, with
  ambiguous ampersand preservation in attributes
- semicolon-terminated decimal and hexadecimal numeric character references in
  data, RCDATA, and attribute values
- missing-semicolon decimal and hexadecimal numeric character reference
  recovery in data, RCDATA, and attribute values
- tokenizer error codes lowered into Venture diagnostics

Unsupported raw cases are skipped into metadata in the generated file rather
than silently disappearing. Rust conformance tests execute the generated
`html5lib-smoke.json` corpus and separately parse the raw upstream-style file to
keep the intake path visible. Tokenizer-context cases such as RCDATA, RAWTEXT,
script submodes, CDATA bracket/end states, resumable end-tag-open states, and
seeded end-tag, comment, and DOCTYPE continuation states stay in the generated
corpus with context metadata and are seeded into the Rust wrapper at test time, while still
unsupported upstream states remain recorded under `skipped` instead of being
discarded. End-tag continuation coverage intentionally exercises matching,
mismatched, EOF, and diagnostic recovery paths so parser/tokenizer handoff
regressions show up in the shared fixture corpus instead of only in narrow unit
tests. Comment continuation coverage does the same for pending dash/bang and
bogus-comment recovery paths. Character-reference continuation coverage
exercises seeded named/numeric reference recovery returning to data and RCDATA.
DOCTYPE continuation coverage exercises partial keyword/name, identifier,
diagnostic, and bogus-doctype recovery paths.

To regenerate the normalized corpus:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/normalize_html5lib_fixtures.py \
  code/packages/rust/html-lexer/tests/fixtures/upstream-html5lib-smoke.test \
  code/packages/rust/html-lexer/tests/fixtures/html5lib-smoke.json
```

To check whether the normalized corpus is stale:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/normalize_html5lib_fixtures.py \
  code/packages/rust/html-lexer/tests/fixtures/upstream-html5lib-smoke.test \
  code/packages/rust/html-lexer/tests/fixtures/html5lib-smoke.json \
  --check
```

To check that all lexer/parser fixture helper and generator Python scripts still
compile without writing bytecode into the worktree, and that the checked-in
script inventory is current:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/check_html_fixture_scripts_compile.py \
  --check
```

To update the checked-in script inventory after adding or removing a helper:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/check_html_fixture_scripts_compile.py \
  --write-inventory
```

Planned flow:

1. Import or mirror selected upstream tokenizer cases into a generator script.
2. Lower them into `venture-html-lexer-fixtures/v1` JSON files with stable IDs.
3. Keep provenance metadata alongside the generated fixture file or in the
   import script, rather than coupling the Rust test harness to WPT internals.

This keeps Venture's Mosaic compatibility floor protected while making it easy
to add newer HTML tokenizer behavior as the authored state machine grows.
