---
category: Rust
---

# A returned borrow prevents restoring its output before a final audit

When a success wrapper borrows caller-owned output for its full return
lifetime, matching on that wrapper and then restoring the output in an audit-
failure branch still violates the borrow rules: the success arm may return the
borrow. Use a bounded two-pass construction instead. Construct and immediately
discard the proof in a helper returning only `()`, restore output on validation
or audit failure, publish the terminal success event, and then reconstruct from
the same immutable inputs for return. This preserves fail-closed output without
unsafe code or `drop` calls that trigger Clippy's `drop_non_drop` lint.
