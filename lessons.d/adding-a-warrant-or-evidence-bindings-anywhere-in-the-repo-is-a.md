---
category: Testing & coverage
---

# Adding a .warrant or evidence.bindings anywhere in the repo is a warrant-grammar change, so run its full test suite

The wasm-leb128 adoption added `code/specs/wasm-leb128.warrant` (the first
document with the `v1` header) and `rust/wasm-leb128/evidence.bindings`.
Every wasm-leb128 BUILD line passed, but `warrant-grammar`'s
`tests/v0_compat.rs` failed: it scans *every* `code/specs/*.warrant` and every
`code/packages/rust/*/evidence.bindings` and asserted each document is v0
("none of today's documents carries the `v1` header"). Nothing in the
adopter's own BUILD runs that test, and the CI build tool maps neither a
`.warrant` nor another package's bindings file to `warrant-grammar`, so on a
branch that did not also touch `warrant-grammar` the break would have reached
main unseen and failed the next unrelated warrant-grammar PR.

The fix kept the v0 guarantee and gave v1 adopters their own lane: the
pre-WX-B documents (`ct-compare`, `gf256`) are pinned by name and must stay v0;
any other document must type-check, and a v1 adopter's harness must
regenerate byte for byte with its host lines, as `gen_evidence --check` does.

Do this: when you add or edit any `.warrant` file or `evidence.bindings`, run
`cargo test -p coding_adventures_warrant_grammar` (all of it, not the regen
tests you think are relevant) and `cargo test -p
coding_adventures_ca_assurance_report`, whatever the build tool selects. Tests
that scan the repository are invisible to path-based change detection.
