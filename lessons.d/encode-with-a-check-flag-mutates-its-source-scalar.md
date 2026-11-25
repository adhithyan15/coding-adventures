---
category: Perl
---

# Encode with a check flag mutates its source scalar

When `Encode::encode` or `Encode::decode` receives a `CHECK` flag such as
`FB_CROAK`, it may consume the supplied scalar. Copy caller-owned text before
encoding it for a byte-length check, or the later parser may see an empty
string even though the size check succeeded.
