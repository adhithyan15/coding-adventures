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
Numeric character references report invalid-code-point diagnostics and recover
with the HTML replacement/remapping rules for null, surrogate, out-of-range,
noncharacter, and Windows-1252 control references.
Duplicate attributes recover with HTML semantics: the first attribute value is
kept, later attributes with the same interpreted name are dropped, and a
`duplicate-attribute` diagnostic is recorded.
Unquoted attribute values also preserve spec-defined unexpected characters
such as `"`, `'`, `<`, `=`, and `` ` `` while reporting
`unexpected-character-in-unquoted-attribute-value`.
NULL characters in data/RCDATA/RAWTEXT/PLAINTEXT/CDATA/script data, script
escaped/double-escaped states, and attribute values recover by reporting
`unexpected-null-character` and appending U+FFFD, matching the replacement
behavior the future parser will expect from the lexer/tokenizer boundary.
Tag names and attribute names now use the same recovery shape, so a raw NULL in
markup names becomes U+FFFD and records `unexpected-null-character` instead of
leaking the raw code point into emitted tokens.
Comments and bogus comments also replace raw NULL characters with U+FFFD while
recording `unexpected-null-character`, including dash-sensitive comment end
substates that must preserve their pending `-` or `--` text first.
DOCTYPE names and quoted public/system identifiers use the same replacement
path, so legacy declarations with embedded NULLs still emit structured
DOCTYPE tokens with U+FFFD in the affected field.
Comment tokenization includes the standard start-dash recovery cases for empty
HTML comments such as `<!-->` and `<!--->`, while still preserving normal
Mosaic-era `<!--note-->` comments. Nested-looking `<!--` sequences inside an
open comment remain literal comment data and surface a recoverable
`nested-comment` diagnostic. Comment endings also recover from `--!>` with an
`incorrectly-closed-comment` diagnostic while preserving non-closing `--!` text.
Processing-instruction-looking markup such as `<?xml ...?>` now follows HTML
bogus-comment recovery instead of being mistaken for a start tag, preserving
legacy document prologs without polluting the tag stream. EOF in that bogus
comment recovery emits the recovered comment without adding an unrelated
`eof-in-comment` diagnostic.
Malformed markup declarations such as `<!foo>` also recover as bogus comments
and report `incorrectly-opened-comment`, matching the tokenizer error shape the
future parser will rely on for compatibility diagnostics. That recovery
reconsumes the first non-matching byte in bogus-comment state, so empty
declarations such as `<!>` emit an empty comment and return to normal data
lexing, and EOF after `<!` emits the same empty bogus-comment recovery instead
of preserving the opener as text. One-dash declaration openers such as `<!->`
and `<!-x>` use that same bogus-comment recovery instead of being mistaken for
normal empty-comment syntax.
The tag-open states now only begin normal tags when the next character is an
ASCII letter; stray less-than signs such as `a < b` remain text, and malformed
end-tag openers such as `</3>` recover as bogus comments with a tokenizer
diagnostic.
DOCTYPE tokenization reports missing names and marks force-quirks mode for
inputs such as `<!DOCTYPE>` and `<!DOCTYPE >`, and EOF recovery after a name
or inside the `DOCTYPE` keyword emits a force-quirks token. Malformed
`DOCTYPE` keyword text also marks force-quirks mode. Mosaic-era `PUBLIC` and
`SYSTEM` identifiers are preserved on emitted DOCTYPE tokens, so legacy
declarations such as `<!DOCTYPE html PUBLIC "...">` keep the information the
future tree-construction/parser layer will need for compatibility decisions.
DOCTYPE system-identifier recovery marks force-quirks mode for missing
identifiers and unexpected trailing junk.
The generated HTML1 machine also exposes `RCDATA`, `RAWTEXT`, `PLAINTEXT`,
`CDATA section`, `script_data`, `script_data_escaped`, and
`script_data_double_escaped` entry states for parser-controlled tokenizer
submodes. The markup declaration path also recognizes `<![CDATA[` and enters
the CDATA section state so the generated lexer can exercise that tokenizer
subflow end to end; a future parser can still decide when that opener is valid
for foreign-content contexts.
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
CDATA section, script data, script data escaped, and script data double escaped
cases can already live in the shared Venture fixture format. Current Rust
conformance tests now seed that context into the generated lexer so the first
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
