---
category: Compiler / VM / language pipeline
---

# A table keyed by a lossy UTF-8 decode of bytes collides as soon as the bytes need not be UTF-8

**What went wrong.** `iir-to-wasm` deduplicated its promoted string-literal
blocks in a `HashMap<String, u32>` keyed by `String::from_utf8_lossy(bytes)`.
A comment argued the decode was lossless because every frontend emits UTF-8.
LANG83 PR5b added raw-byte literals (`Operand::Bytes`), and the one-byte
strings `A9` and `FF` both decode to U+FFFD, so they shared one block. A
function that returned `A9` on one branch and `FF` on the other returned `A9`
on both. Nothing failed at compile time.

**Fix.** Key the table by the bytes themselves (`HashMap<Vec<u8>, u32>`).
Test `runtime_blocks_are_keyed_by_bytes_not_by_lossy_text`; it fails with only
the lossy key put back.

**Do differently.** Never use a lossy conversion as an identity. If a key must
be text, use an injective encoding (hex, `escape_ascii`). When a change widens
what a value can hold (UTF-8 text to arbitrary bytes), grep for every
`from_utf8_lossy`, `to_string_lossy` and `String::from_utf8(...).ok()` on that
value's path. Each one is either a display (fine) or a bug.
