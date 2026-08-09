# vault-pm-format

Canonical repository bytes for the local-first password manager specified by
VLT-PM01.

The crate owns strict V1 codecs for bootstraps, device certificates, commits,
signed announcements, and encrypted object frames. It also derives
domain-separated IDs and signing preimages. It deliberately has no storage,
randomness, key derivation, encryption, or signature implementation.

```text
domain model -> canonical unsigned bytes -> signer
                                             |
                                             v
storage <- opaque frame <- repository encryption <- signed structured value
```

Run:

```sh
cargo test -p coding_adventures_vault_pm_format -- --nocapture
```

Wire details and bounds are normative in `code/specs/VLT-PM01-format.md`.
