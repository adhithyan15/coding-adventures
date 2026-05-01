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
- `html5lib-smoke.json`: generated normalized Venture fixture corpus derived from
  the raw html5lib-style smoke file
- `upstream-html5lib-smoke.test`: raw html5lib-style tokenizer cases used to
  exercise the normalization path toward broader upstream corpora
- `normalize_html5lib_fixtures.py`: importer that lowers supported raw
  html5lib-style tokenizer cases into Venture's portable fixture schema

## WPT Path

The next layer normalizes WHATWG/WPT or html5lib-style tokenizer coverage into
this same schema instead of making the Rust test harness understand raw
upstream files directly. That keeps the runtime boundary stable while still
letting us mirror broader living-standard cases.

`upstream-html5lib-smoke.test` is the first concrete step in that direction. It
uses the tokenizer JSON structure documented by the html5lib tokenizer tests:
top-level `tests`, with each test carrying `description`, `input`, `output`,
optional `initialStates`, optional `lastStartTag`, and optional `errors`.

`normalize_html5lib_fixtures.py` is the checked-in importer for this shape. It
currently supports:

- default data-state cases
- explicit `initialStates: ["Data state"]`
- `initialStates: ["RCDATA state"]` together with `lastStartTag`
- `initialStates: ["RAWTEXT state"]` together with `lastStartTag`
- `StartTag`, `EndTag`, `Character`, `Comment`, and `DOCTYPE` output tokens
- html5lib start-tag self-closing booleans
- named character references in data, RCDATA, and attribute values for the
  current shared entity subset
- legacy named character references `nbsp`, `copy`, and `reg` in data, RCDATA,
  and attribute values
- HTML4 math/symbol named character references such as `alefsym`, `oline`,
  `sum`, and `notin`
- missing-semicolon recovery for legacy named character references `nbsp`,
  `copy`, and `reg` before delimiters and EOF
- generic named-character-reference scanning with literal fallback for unknown
  names
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
keep the intake path visible. Tokenizer-context cases such as RCDATA and
RAWTEXT stay in the generated corpus with `initial_state` / `last_start_tag`
metadata and are seeded into the Rust wrapper at test time, while still
unsupported upstream states remain recorded under `skipped` instead of being
discarded.

To regenerate the normalized corpus:

```bash
python3 code/packages/rust/html-lexer/tests/fixtures/normalize_html5lib_fixtures.py \
  code/packages/rust/html-lexer/tests/fixtures/upstream-html5lib-smoke.test \
  code/packages/rust/html-lexer/tests/fixtures/html5lib-smoke.json
```

Planned flow:

1. Import or mirror selected upstream tokenizer cases into a generator script.
2. Lower them into `venture-html-lexer-fixtures/v1` JSON files with stable IDs.
3. Keep provenance metadata alongside the generated fixture file or in the
   import script, rather than coupling the Rust test harness to WPT internals.

This keeps Venture's Mosaic compatibility floor protected while making it easy
to add newer HTML tokenizer behavior as the authored state machine grows.
