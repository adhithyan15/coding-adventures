---
category: Rust
---

# A new GrammarElement variant must be added to every exhaustive match in downstream tests too

**What went wrong.** PR #17824 (E6) added
`GrammarElement::AdjacencyPredicate` to grammar-tools. Meanwhile Ruby 5a and
Python 7a landed on main, and each added a `tests/cst_contract.rs` whose
`referenced_tokens` matches `GrammarElement` exhaustively, with no `_` arm.
Once main was merged in, both test crates stopped compiling, and the CI
shard holding them failed (`lang-aot` was DEP-SKIPped). The PR's own tests
passed because neither test file existed when the variant was written.

**Fix.** Add the variant to the no-token arm in both files.

**Do differently.** After merging main into a branch that adds an enum
variant to a shared crate, grep the whole tree for exhaustive matches on
that enum, tests included:

    grep -rln 'GrammarElement::RuleReference' --include=*.rs code | grep -v _grammar

Then compile the matches' crates with `cargo test --no-run`.
