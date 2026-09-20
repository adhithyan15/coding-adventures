---
category: Rust
---

# Rust vec macro elements do not splice fixed byte arrays

`vec![prefix, *b"bytes"]` treats the fixed byte array as one element and fails
because that element is not a `u8`; the macro has no spread form. Build test
fixtures from slices with `[prefix_slice, b"bytes"].concat()`, or initialize a
vector and call `extend_from_slice`, whenever a literal header and byte array
must become one flat buffer.
