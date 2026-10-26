# Changelog

## Unreleased

- Adopt the sixteen-case portable TE04 extension corpus through the public
  parser and HTML renderer APIs. Compare complete AST and HTML output with a
  dynamic reader, add independent source/AST/HTML corruption witnesses, and
  route exact corpus changes into the package's native CI build closure.
- Bound the native fixture read to 131,072 bytes, reject a UTF-8 byte-order
  mark and malformed UTF-8, and run the digest/depth/duplicate-key-aware Python
  validator plus its tests in the repository's always-on contract job.
- Preserve literal Unicode line/paragraph separators and opposite fence characters in info by normalizing the already-sliced opener suffix. Add independent fixture regressions and a one-blank-line payload witness.
- GFM fences now produce `fenced_block` with the entire normalized `info`, its first-token `name`, and literal payload. Info is decoded once, interior whitespace is retained, and absent or empty normalized info produces null hints; empty payloads remain valid. Indented code remains `code_block`.
