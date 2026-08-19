# `coding_adventures_vault_pm_config`

This crate defines the host-neutral configuration contract shared by vault-pm
clients. It remembers the opaque locator needed to reopen each vault, maps each
vault to named local and remote stores, and carries bounded auto-lock and
clipboard-clear policy.

The V1 decoder accepts only the closed schema in `VLT-PM07-config.md`. It
rejects duplicate or unknown declarations, unknown versions and storage kinds,
invalid references, duplicate locators, unbounded input, and non-canonical
locator encodings. The renderer produces deterministic TOML with sorted table
names, so a host adapter can persist and compare exact bytes.

This package does not read paths, open files, resolve platform directories,
load credentials, or instantiate a storage provider. Filesystem, removable,
Google Drive, WebDAV, and S3 are typed adapter selections; the opaque `path`
string is interpreted only by the selected host adapter. `removable` (VLT-PM00
§12, §23 item 14) is a variant of `filesystem` sharing the identical on-disk
object format — `StorageKind::is_local_directory` reports both as one group so
a host adapter can route them to the same filesystem code path and reserve the
distinction for `vault-pm-storage-removable`'s conflict-copy detection.

Sensitive or identifying values use redacted `Debug` output, and all errors
are stable and payload blind. Nineteen tests cover the closed schema,
cross-references, canonical escaping, all supported storage kinds, bounds,
malformed input, and redaction.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_config --all-targets -- -D warnings
```
