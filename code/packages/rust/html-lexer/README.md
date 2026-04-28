# coding-adventures-html-lexer

Rust HTML lexer for Venture.

The HTML standard calls this phase tokenization, while this repository uses the
package term **lexer** for source-to-token frontends. This crate is therefore
the Rust HTML lexer package, even when comments or specs reference the WHATWG
HTML tokenizer states by their standard names.

## How it works

The generic `state-machine` crate executes ordered transitions. The
`state-machine-tokenizer` crate owns the portable lexer runtime state: text
buffers, current token construction, diagnostics, source positions, and traces.
This crate owns the HTML-specific state machine and exposes a Rust lexer API.

The TOML files in this package are authoring artifacts. Production code links
checked-in generated Rust modules built to match the output shape of
`state-machine-source-compiler`, so the runtime never loads TOML or JSON.

`html1.lexer.states.toml` is the current package default and the broader
compatibility floor for Venture's Mosaic-era target: it is not the end state of
the project, but the first real HTML authoring artifact that must keep HTML
1.0-era content working as the lexer grows forward toward newer standards.
The default lexer already resolves the core named character references and the
classic Latin-1 entity set, preserving entity-name case so legacy names such as
`Agrave` and `agrave` remain distinct.
The generated HTML1 machine also exposes `RCDATA`, `RAWTEXT`, `PLAINTEXT`,
`script_data`, `script_data_escaped`, and `script_data_double_escaped` entry
states for parser-controlled tokenizer submodes.
`html-skeleton.lexer.states.toml` remains in the crate as a smaller bootstrap
machine for comparisons and narrow debugging.

## Conformance

Repo-native conformance fixtures live under
[`tests/fixtures`](/tmp/coding-adventures-html-conformance/code/packages/rust/html-lexer/tests/fixtures).
They use a documented JSON schema that Rust tests load with `include_str!`, so
the test corpus is checked in and shared while production code still links only
static Rust modules.

Today the package carries two suites:

- `html-skeleton.json` for narrow bootstrap regression coverage
- `html1.json` for the current Mosaic-era compatibility floor

There is also an `upstream-html5lib-smoke.test` file that mirrors the raw
html5lib tokenizer JSON shape in a small supported subset. The Rust test
harness now targets a checked-in generated `html5lib-smoke.json` corpus, which
is produced by `tests/fixtures/normalize_html5lib_fixtures.py` from that raw
upstream-style file. This makes the future WPT/html5lib import path concrete
without coupling the shared harness to upstream file formats or requiring raw
fixture normalization logic to live forever inside the Rust tests.

The normalized corpus now carries optional tokenizer-context metadata such as
`initial_state` and `last_start_tag`, so upstream RCDATA, RAWTEXT, PLAINTEXT,
script data, script data escaped, and script data double escaped cases can
already live in the shared Venture fixture format. Current Rust conformance
tests now seed that context into the generated lexer so the first
non-data-state cases execute through the same static Rust wrapper as the
data-state corpus.

The intended WHATWG/WPT path is to normalize upstream tokenizer cases into this
same schema rather than teaching the Rust harness to parse raw upstream files
directly. That gives us a clean expansion path from the HTML 1.x floor toward
the living standard without reopening the runtime trust boundary.

## Usage

```rust
use coding_adventures_html_lexer::{lex_html, Token};

let tokens = lex_html("<p>Hello</p>").unwrap();

assert_eq!(
    tokens,
    vec![
        Token::StartTag { name: "p".into(), attributes: vec![], self_closing: false },
        Token::Text("Hello".into()),
        Token::EndTag { name: "p".into() },
        Token::Eof,
    ]
);
```

## Development

```bash
bash BUILD
```
