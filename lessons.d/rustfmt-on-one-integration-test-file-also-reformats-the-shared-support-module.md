---
category: Rust
---

# rustfmt on one integration-test file also reformats the shared support module it declares

`rustfmt tests/new_test.rs` formats the file **and every module it declares**, so a test that says
`mod support;` rewrote `tests/support/minijson.rs`, a file the change never touched (caught only
by `git status`). The tree is not rustfmt-clean everywhere either (`cli.rs`, `cost.rs` already
differ at HEAD), so `cargo fmt` over the crate would have been worse.

Format a new test with `rustfmt --edition 2021 --config skip_children=true tests/new_test.rs`, or
format only the files you own, and read `git status` before staging: the diff of an untouched file
is the symptom. To tell your own formatting debt from pre-existing debt, run
`git show HEAD:path | rustfmt --check` on the old text.
