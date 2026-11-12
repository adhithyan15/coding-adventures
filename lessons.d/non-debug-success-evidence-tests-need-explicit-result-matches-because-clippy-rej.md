---
category: Cryptography & security review
---

# Non-Debug success evidence tests need explicit Result matches because Clippy rejects err expect

Security evidence deliberately omits `Debug`, so `Result::expect_err` cannot be
used without weakening the evidence type. The older `.err().expect(...)`
workaround now fails this repository's `cargo clippy --all-targets -- -D
warnings` gate under `clippy::err_expect`.

Use an explicit `match`: return the error from `Err`, and panic with a static
message from `Ok(_)` without formatting the success value. This preserves the
non-disclosure boundary and passes the current lint policy.
