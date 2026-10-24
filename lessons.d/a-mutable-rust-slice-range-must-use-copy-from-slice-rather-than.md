---
category: Rust
---

# A mutable Rust slice range must use copy_from_slice rather than whole-slice assignment

Indexing a `Vec<u8>` with a `Range<usize>` yields an unsized `[u8]` place. Even
when the range is known to contain one byte, assigning `[value]` to that place
fails with E0308/E0277 because the left side is a slice and the right side is an
array. Validate the range first, then call
`bytes[range].copy_from_slice(&[value])`; for a single index, assign through
`bytes[index]` instead. Run the first compile immediately after adding memory
helpers so this mechanical error does not obscure instruction-level failures.
