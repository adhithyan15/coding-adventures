---
category: Perl
---

# Validate array elements before checked native addition

A checksum helper that accepts a caller-owned Perl array must validate every
element, not only its operation count. Perl arithmetic silently coerces values:
`1.5` can produce a fractional checksum, and a nonnumeric string can become
zero after a warning. Require scalar integer values and check their signed
64-bit range before either traversal begins, including when the operation count
is zero.

For two already validated signed 64-bit integers `a` and `b`, check the relevant
boundary before adding: positive `b` overflows when `a > I64_MAX - b`, and
negative `b` underflows when `a < I64_MIN - b`. The safe native addition then
remains exact and avoids allocating a big integer for every element in a hot
benchmark loop. Test both traversal variants with fractional, nonnumeric,
nonfinite, out-of-range, positive-overflow, negative-overflow, and mutated
caller-owned arrays.
