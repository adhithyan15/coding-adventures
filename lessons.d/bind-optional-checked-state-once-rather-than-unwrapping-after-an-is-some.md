---
category: Rust
---

# Bind optional checked state once rather than unwrapping after an is_some guard

The first strict CV02 lint run rejected an `is_some()` branch followed by
`self.checked.as_ref().unwrap()` with `clippy::unnecessary_unwrap`. Tests had
passed, but the repository's warnings-as-errors contract had not.

Bind the state with `if let Some(state) = &self.checked` and compare the usage
ledger through that binding. Run strict lint before committing the foundation;
passing tests do not establish the required lint result.
