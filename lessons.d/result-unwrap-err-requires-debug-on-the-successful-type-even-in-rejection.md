---
category: Rust
---

# Result unwrap_err requires Debug on the successful type even in rejection tests

A PREP01 rejection test used `preprocess(...).unwrap_err()`, but its successful
`Preprocessed` type has no `Debug` implementation. Rust requires `T: Debug`
for `Result<T, E>::unwrap_err()` so it can display an unexpected success. The
test could not compile before reaching the intended negative case.

Use an explicit `match` or `.err().expect("expected rejection")` when only the
error is needed; do not add a production `Debug` implementation solely to
satisfy a test convenience method. First compile the rejection suite, then
verify the acceptance regression fails for the missing behavior.
