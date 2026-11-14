---
category: Testing & coverage
---

# A Python lane runner must compare results by exact type, because True == 1 and bytearray == bytes

Python's `==` is generous exactly where an evidence runner must be strict: `True == 1`,
`1.0 == 1`, and `bytearray(b"a") == b"a"`. A lane runner that compares `actual == expected` would
pass a `Bool` op that returns `1`, a `U64` op that returns `True`, and a `Bytes` op that returns a
mutable buffer.

The `warrant-corpus` runner therefore checks `type(x) is T` (and the integer range) against the
declared Warrant type first (`type-mismatch`), and only then compares with `==`. It also wraps a
library's plain return value in `("ok", value)` itself instead of trusting a returned tuple, so a
stray `("err", ...)` from the library fails the type check rather than posing as a declared error.
Test these shapes with a deliberately wrong reference library, not only with correct ones.
