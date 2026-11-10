---
category: Rust
---

# Adding a field to a public struct whose Debug output is hashed breaks the v0 byte-identity gate; expose new checker data through a separate accessor

WX-A needed the type checker to remember its records and variants, and the
natural place was a new `adts` field on `CheckedWarrantModule`. The very next
test run failed `v0_compat::the_repository_specifications_hash_as_they_did_before_wx_b_and_wx_c`:
that test pins the SHA-256 of the `Debug` rendering of the checker's output for
`ct-compare.warrant` and `gf256.warrant`, so *any* new field, even an empty map,
changes the bytes and looks exactly like a v0 meaning change.

Fix: the new data stays inside the private `Checker` (the declarations are
checked there and the evidence generator re-derives what it needs from the
AST), and `CheckedWarrantModule` is untouched. The pinned hash is a feature, not
an obstacle: it is what proves a v1 slice leaves v0 documents alone.

Do differently: before adding a field to a public struct, grep the tests for a
hash of its `Debug` output (`sha256_hex(format!("{:?}", ..))`). If one exists,
either put the data somewhere else or deliberately re-pin the hash in the same
commit *and* say why in the commit message and the spec's compatibility section.
The same applies to any `#[derive(Debug)]` enum that gains a variant: a variant
that no v0 document can reach is fine, but a changed `Debug` of an existing one
is not.
