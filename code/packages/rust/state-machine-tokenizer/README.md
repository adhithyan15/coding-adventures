# state-machine-tokenizer

Tokenizer profile runtime on top of `state-machine` effectful transducers.

This crate is the first Rust wrapper layer for declarative tokenizer machines.
The core `state-machine` crate executes ordered transitions and emits portable
effect names. This crate interprets those effect names into tokenizer buffers,
tokens, diagnostics, source positions, and traces.

The package does not read TOML, JSON, or any other definition file at runtime.
Production wrappers should link statically generated machine constructors, then
pass the resulting `EffectfulStateMachine` into `Tokenizer::new`.

## Example

```rust
use state_machine_tokenizer::{html_skeleton_tokenizer, Token};

let mut tokenizer = html_skeleton_tokenizer().unwrap();
tokenizer.push("<p>Hello</p>").unwrap();
tokenizer.finish().unwrap();

assert_eq!(
    tokenizer.drain_tokens(),
    vec![
        Token::StartTag { name: "p".into(), attributes: vec![], self_closing: false },
        Token::Text("Hello".into()),
        Token::EndTag { name: "p".into() },
        Token::Eof,
    ]
);
```

## Current Scope

The first runtime supports the small fixed action vocabulary needed by the HTML
skeleton:

- `append_text(current)`
- `append_text(literal)`
- `append_text_replacement`
- `flush_text`
- `emit_current_as_text`
- `create_start_tag`
- `create_end_tag`
- `append_tag_name(current)`
- `append_tag_name(current_lowercase)`
- `emit_current_token`
- `emit(EOF)`
- `parse_error(code)`

It also enforces a per-input step limit so malformed reconsume-style machines
cannot spin forever on one code point.

## Development

```bash
bash BUILD
```
