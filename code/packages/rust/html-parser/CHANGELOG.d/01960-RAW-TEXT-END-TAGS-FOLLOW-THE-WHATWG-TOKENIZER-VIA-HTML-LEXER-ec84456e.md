- **Raw-text end tags follow the WHATWG tokenizer** (via `html-lexer`):
  `</` + non-letter is text, and a quoted `>` inside end-tag attributes no
  longer ends the tag. `tests/raw_text_end_tags_test.rs` pins that the
  hidden-`<img onerror>` payloads from the security review produce no `img`.
