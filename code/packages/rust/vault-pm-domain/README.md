# `coding_adventures_vault_pm_domain`

The pure product-domain model for the local-first password manager specified by
VLT-PM03.

The crate composes VLT02 records into validated item documents and fixes the
rules every future host shares:

- opaque, explicitly rendered product identifiers;
- observed-remove collection, tag, and attachment membership;
- deterministic favorite-register merge;
- whole-record and delete/edit conflict preservation;
- tombstones and retained conflict resolution state; and
- typed default views that omit plaintext secret fields.

It deliberately owns no clock, entropy, storage, transport, cryptography, or
device keys. Applications supply IDs and timestamps; repositories determine
causal relations; hosts render the returned redacted views.

## Development

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_domain --all-targets -- -D warnings
```
