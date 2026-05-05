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

The current built-in named-character-reference table covers the core HTML
entities plus the classic Latin-1 set. The scanner preserves case, so callers
can model legacy names like `Agrave` separately from `agrave` while still adding
explicit aliases such as `AMP` where the HTML layer needs them.

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
- `append_tag_name_replacement`
- `start_attribute`
- `append_attribute_name(current)`
- `append_attribute_name(current_lowercase)`
- `append_attribute_name(literal)`
- `append_attribute_name_replacement`
- `append_attribute_value(current)`
- `append_attribute_value(literal)`
- `append_attribute_value_replacement`
- `commit_attribute`
- `commit_attribute_dedup`
- `mark_self_closing`
- `emit_current_token`

Comments and doctypes:

- `create_comment`
- `append_comment(current)`
- `append_comment(current_lowercase)`
- `append_comment(literal)`
- `append_comment_replacement`
- `create_doctype`
- `append_doctype_name(current)`
- `append_doctype_name(current_lowercase)`
- `append_doctype_name(literal)`
- `append_doctype_name_replacement`
- `set_doctype_public_identifier_empty`
- `append_doctype_public_identifier(current)`
- `append_doctype_public_identifier(literal)`
- `append_doctype_public_identifier_replacement`
- `set_doctype_system_identifier_empty`
- `append_doctype_system_identifier(current)`
- `append_doctype_system_identifier(literal)`
- `append_doctype_system_identifier_replacement`
- `mark_force_quirks`

Temporary buffers and controlled state changes:

- `clear_temporary_buffer`
- `append_temporary_buffer(current)`
- `append_temporary_buffer(current_lowercase)`
- `append_temporary_buffer(literal)`
- `append_temporary_buffer_to_text`
- `append_temporary_buffer_to_attribute_value`
- `append_numeric_character_reference_to_text`
- `append_numeric_character_reference_to_attribute_value`
- `append_named_character_reference_to_text`
- `append_named_character_reference_to_attribute_value`
- `append_named_character_reference_or_temporary_buffer_to_text`
- `append_named_character_reference_or_temporary_buffer_to_attribute_value`
- `recover_named_character_reference_to_text`
- `recover_named_character_reference_to_attribute_value`
- `discard_current_token`
- `set_return_state(state)`
- `switch_to(state)`
- `switch_to_if_temporary_buffer_equals(value, equal_state, fallback_state)`
- `switch_to_return_state`
- `emit_rcdata_end_tag_or_text`
- `emit_rcdata_end_tag_with_trailing_solidus_or_text`
- `emit_rcdata_end_tag_with_whitespace_or_text`
- `emit_rcdata_end_tag_with_attributes_or_text`

Diagnostics and stream control:

- `emit(EOF)`
- `parse_error(code)`

It also enforces a per-input step limit so malformed reconsume-style machines
cannot spin forever on one code point.

Named character reference actions consume the longest matching known entity
prefix in text-like contexts and preserve ambiguous ampersands in attribute
contexts when a missing-semicolon reference would be followed by an ASCII
alphanumeric character or `=`.
Missing-semicolon recovery is limited to the WHATWG legacy no-semicolon names;
newer names must include `;` or fall back to a shorter legacy prefix/literal
text instead of being over-accepted.

`commit_attribute_dedup` commits the current attribute only when the current
start tag does not already have an attribute with the same interpreted name. If
there is already a matching attribute, the runtime drops the current attribute
and records a `duplicate-attribute` diagnostic. Definitions that want to keep
duplicates can continue using plain `commit_attribute`.

The runtime also exposes context-seeding helpers such as
`Tokenizer::set_initial_state`, `Tokenizer::set_last_start_tag`,
`Tokenizer::set_current_end_tag`, and `Tokenizer::set_temporary_buffer` so
wrapper packages can execute HTML submodes and continuation states like RCDATA
end-tag-name without loading definition files at runtime.

Wrapper packages can opt into HTML-style input-stream newline preprocessing
with `Tokenizer::with_normalized_carriage_returns`. That maps CRLF pairs and
bare carriage returns to a single LF before transition matching while keeping
raw byte/scalar offsets moving across skipped LF bytes.

## Development

```bash
bash BUILD
```
