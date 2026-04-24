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
      "input": "<P>Hello</P>",
      "tokens": [
        "StartTag(name=p, attributes=[], self_closing=false)",
        "Text(data=Hello)",
        "EndTag(name=p)",
        "EOF"
      ],
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
- `upstream-html5lib-smoke.test`: raw html5lib-style tokenizer cases used to
  exercise the normalization path toward broader upstream corpora

## WPT Path

The next layer normalizes WHATWG/WPT or html5lib-style tokenizer coverage into
this same schema instead of making the Rust test harness understand raw
upstream files directly. That keeps the runtime boundary stable while still
letting us mirror broader living-standard cases.

`upstream-html5lib-smoke.test` is the first concrete step in that direction. It
uses the tokenizer JSON structure documented by the html5lib tokenizer tests:
top-level `tests`, with each test carrying `description`, `input`, `output`,
optional `initialStates`, optional `lastStartTag`, and optional `errors`.
Rust tests parse that raw upstream-style file and lower it into
`venture-html-lexer-fixtures/v1` before running the shared conformance harness.

Planned flow:

1. Import or mirror selected upstream tokenizer cases into a generator script.
2. Lower them into `venture-html-lexer-fixtures/v1` JSON files with stable IDs.
3. Keep provenance metadata alongside the generated fixture file or in the
   import script, rather than coupling the Rust test harness to WPT internals.

This keeps Venture's Mosaic compatibility floor protected while making it easy
to add newer HTML tokenizer behavior as the authored state machine grows.
