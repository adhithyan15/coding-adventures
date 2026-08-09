# `coding_adventures_vault_pm_domain`

The pure product-domain model for the local-first password manager specified by
VLT-PM03.

The crate composes VLT02 records into validated item documents and fixes the
rules every future host shares:

- opaque, explicitly rendered product identifiers;
- bounded observed-remove collection, tag, and attachment membership;
- deterministic favorite-register merge;
- whole-record and delete/edit conflict preservation;
- tombstones and retained conflict resolution state; and
- typed default views that omit plaintext secret fields.

It deliberately owns no clock, entropy, storage, transport, cryptography, or
device keys. Applications supply IDs and timestamps; repositories determine
causal relations; hosts render the returned redacted views.

Every observed set retains at most 256 distinct values, 1,024 add operations,
and 1,024 removal tombstones. Its fallible mutation, exact-removal
reconstruction, and merge APIs enforce those limits before insertion. Removed
pairs remain until a repository supplies causal-stability proof to
`compact_stable_removals`; elapsed time or observation by one head is not
sufficient proof.

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_domain --all-targets -- -D warnings
```

The Phase 0 suite contains 29 unit tests and covers 527/532 executable crate
lines (99.06%) under Tarpaulin's LLVM engine.
