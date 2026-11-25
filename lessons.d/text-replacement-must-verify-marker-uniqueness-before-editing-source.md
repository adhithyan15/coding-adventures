---
category: Rust
---

# Text replacement must verify marker uniqueness before editing source

When adding portable fixture replay to the WASM guest tests, a global replacement
matched the same malformed-input assertion in two test functions. The second
insertion referenced a fixture variable outside its scope, and compilation failed.

Before a scripted source replacement, count the expected matches and fail unless
the count is exact. Bound edits by the surrounding function when an assertion is
reused. Inspect the diff and compile the affected target immediately; a successful
text-edit command establishes neither scope nor compilation correctness.
