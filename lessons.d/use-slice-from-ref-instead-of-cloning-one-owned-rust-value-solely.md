---
category: Rust
---

# Use slice from_ref instead of cloning one owned Rust value solely to call a slice-taking test helper

A Rust test passed `&[value.clone()]` to a helper that only needed a borrowed
one-element slice. Strict Clippy rejected the unnecessary allocation-copy
pattern under `clippy::cloned_ref_to_slice_refs`. Use
`std::slice::from_ref(&value)` when the helper consumes only the slice view and
the owned value must remain available for later cases. Run Clippy with
`--all-targets -- -D warnings` so test helpers receive the same scrutiny as
production code.
