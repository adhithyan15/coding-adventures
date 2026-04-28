# state-machine-tokenizer

Tokenizer profile runtime on top of `state-machine` effectful transducers.

This crate is the first Rust wrapper layer for declarative tokenizer machines.
The core `state-machine` crate executes ordered transitions and emits portable
effect names. This crate interprets those effect names into tokenizer buffers,
tokens, diagnostics, source positions, and traces.

The package does not read TOML, JSON, or any other definition file at runtime.
Production wrappers should link statically generated machine constructors, then
pass the resulting `EffectfulStateMachine` into `Tokenizer::new`.

HTML-specific machine constructors live in `coding-adventures-html-lexer`. This
package stays generic: it interprets lexer actions for any statically linked
machine that uses the portable action vocabulary below.

## Lexer Action Vocabulary

The runtime interprets a bounded portable action vocabulary. Definitions stay
serializable because actions are simple strings, and unknown actions still fail
closed at runtime.

Text buffering:

- `append_text(current)`
- `append_text(literal)`
- `append_text_replacement`
- `flush_text`
- `emit_current_as_text`

Tag tokens:

- `create_start_tag`
- `create_end_tag`
- `append_tag_name(current)`
- `append_tag_name(current_lowercase)`
- `start_attribute`
- `append_attribute_name(current)`
- `append_attribute_name(current_lowercase)`
- `append_attribute_name(literal)`
- `append_attribute_value(current)`
- `append_attribute_value(literal)`
- `commit_attribute`
- `mark_self_closing`
- `emit_current_token`

Comments and doctypes:

- `create_comment`
- `append_comment(current)`
- `append_comment(current_lowercase)`
- `append_comment(literal)`
- `create_doctype`
- `append_doctype_name(current)`
- `append_doctype_name(current_lowercase)`
- `append_doctype_name(literal)`
- `mark_force_quirks`

Temporary buffers and controlled state changes:

- `clear_temporary_buffer`
- `append_temporary_buffer(current)`
- `append_temporary_buffer(current_lowercase)`
- `append_temporary_buffer(literal)`
- `append_temporary_buffer_to_text`
- `append_temporary_buffer_to_attribute_value`
- `discard_current_token`
- `set_return_state(state)`
- `switch_to(state)`
- `switch_to_return_state`
- `emit_rcdata_end_tag_or_text`

Diagnostics and stream control:

- `emit(EOF)`
- `parse_error(code)`

It also enforces a per-input step limit so malformed reconsume-style machines
cannot spin forever on one code point.

The runtime also exposes context-seeding helpers such as
`Tokenizer::set_initial_state` and `Tokenizer::set_last_start_tag` so wrapper
packages can execute HTML submodes like RCDATA without loading definition files
at runtime.

## Development

```bash
bash BUILD
```
