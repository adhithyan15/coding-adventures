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
`html-skeleton.lexer.states.toml` remains in the crate as a smaller bootstrap
machine for comparisons and narrow debugging.

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
