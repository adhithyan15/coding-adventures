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

Persistent codecs must enumerate `retained_values` and each value's retained
add and removal operations. The present-only `values` projection deliberately
cannot be used for lossless persistence because it would discard tombstones
and could resurrect removed membership after a merge.

## Attachments are two facts that must agree

`ItemDocument` carries both `attachments`, the observed-remove set of
`AttachmentId`s, and `attachment_manifests`, a map from each of those ids to
the `AttachmentManifestId` naming where its bytes are. Those are one fact
stored twice, so the only legal relation between them is equality, and
`validate` enforces it in both directions: membership with no manifest names
bytes nobody can find, and a manifest with no membership points at bytes
nothing claims. Neither is a state with a meaning, so both are
`AttachmentManifestMismatch`.

The key set is `retained_values()` rather than `values()`. An observed-remove
set keeps a removed value on the wire so a later merge cannot resurrect it
silently; if the manifest reference were dropped at removal, the resurrected
attachment would name bytes nothing could find.

Concurrent auto-merge unions the two maps. A disagreement about one id is a
fault rather than a conflict a person could resolve: an `AttachmentId` is a
random 128-bit value drawn once and the manifest it names is an immutable
content address, so two replicas that both know the id necessarily know the
same manifest, and one of them is simply wrong.

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_domain --all-targets -- -D warnings
```

The Phase 0 suite contains 30 unit tests and covers 537/542 executable crate
lines (99.08%) under Tarpaulin's LLVM engine.
