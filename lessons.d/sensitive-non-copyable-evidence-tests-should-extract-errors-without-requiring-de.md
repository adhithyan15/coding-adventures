---
category: Cryptography & security review
---

# Sensitive non-copyable evidence tests should extract errors without requiring Debug

`Result::unwrap_err()` requires the success type to implement `Debug`. A test
used it on non-copyable security evidence whose deliberate lack of `Debug`
prevents accidental disclosure of retained request data. Do not add or derive a
debug representation just to satisfy the assertion helper. Use
`result.err().expect("...")` or an explicit match so the test exercises the
failure while preserving the evidence type's disclosure boundary.
