# Changelog

## Unreleased

- GFM fences now produce `fenced_block` with the entire normalized `info`, its first-token `name`, and literal payload. Info is decoded once, interior whitespace is retained, and absent or empty normalized info produces null hints; empty payloads remain valid. Indented code remains `code_block`.
