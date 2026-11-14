---
category: Cryptography & security review
---

# Explicit Result matching satisfies Clippy without adding Debug to sensitive evidence

Security-boundary tests often avoid `Result::unwrap_err()` because it requires
the success evidence to implement `Debug`. Replacing it with
`result.err().expect(...)` avoids that disclosure pressure, but the repository's
`-D warnings` Clippy gate rejects the pattern under `clippy::err_expect`.

Use an explicit `match`: return the error arm and panic with a fixed message in
the success arm. This neither formats the retained evidence nor suppresses a
lint, and it works whether the non-copyable success type intentionally omits
`Debug` or provides only a redacted implementation.
