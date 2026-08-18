# `coding_adventures_vault_pm_audit`

Pure, storage-neutral operation-audit primitives for the local-first password
manager. Each event binds one high-level authenticated action to its vault,
certified device, device counter, random operation trace, redacted resource
identity, selected/result revision, prior per-device audit event, observed
repository heads, outcome, and advisory time.

Events use a closed canonical V1 encoding and are signed by the acting device.
The crate reads no clock or entropy and performs no persistence. Application
layers can seal events at rest and publish them atomically with the commit that
contains the operation, allowing filesystem, Google Drive, WebDAV, S3, and
future stores to remain opaque byte stores. Conflict choice and authored
conflict merge are distinct actions so a merge never invents one selected
parent. Portable import and independent portable-restore verification are also
distinct so the audit chain records mutation and post-reopen comparison as
separate operations. Passphrase rotation is its own vault-scoped action so that
a change of master credential is visible in the chain as itself rather than
inferred from an absence.

The event contains no title, username, URL, query text, password, notes body,
TOTP seed, attachment name, provider identity, path, or arbitrary detail field.
`PassphraseRotate` adds nothing to that list either: no salt, no KDF parameter,
no generation number, no bootstrap identifier. An audit chain records that a
rotation happened, not the shape of the credential it produced.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_audit --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding_adventures_vault_pm_audit --no-deps
```
