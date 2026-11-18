---
category: Rust
---

# A Rust test adapter should pass a named error mapper directly to map_err

CI's strict Clippy gate rejected `map_err(|error| error_name(error))` in the
new shared-corpus adapter under `clippy::redundant_closure`. The closure only
forwarded its argument to the named function, so it added no behavior and
failed compilation under `-D warnings`.

Pass the function item directly: `map_err(error_name)`. When adapting a Rust
test helper, inspect the named function's signature and prefer it directly to
a forwarding closure; run the package's CI-equivalent Clippy command before
publishing the test.
