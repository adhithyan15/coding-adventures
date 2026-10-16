---
category: Rust
---

# Count legacy source children independently of the new provenance namespace

Independent D1 test review found that a census filtered to role=piece and the
expected input_id could miss duplicate legacy lexer_token/lexer_trivia children
with no namespace. The semantic red was real, but this no-extra assertion was
incomplete. Before production, specify and test the union of legacy direct root
children and reserved pieces attributed to or parented by that root. Exercise
the helper with explicit namespace-free and misattributed extra children, and
reuse it for unavailable inventories. Keep the original test/red receipt and
withheld review; stronger census coverage does not establish implementation.
