# `coding_adventures_vault_pm_audit`

Pure, storage-neutral operation-audit primitives for the local-first password
manager. Each event binds one high-level authenticated action to its vault,
certified device, device counter, random operation trace, redacted resource
identity, selected/result revision, prior per-device audit event, observed
repository heads, outcome, and advisory time.

Events use a closed canonical V1 encoding and are signed by the acting device.
The crate reads no clock or entropy and performs no persistence. The application
layer will seal events at rest and publish them atomically with the commit that
contains the operation, allowing filesystem, Google Drive, WebDAV, S3, and
future stores to remain opaque byte stores.

The event contains no title, username, URL, query text, password, notes body,
TOTP seed, attachment name, provider identity, path, or arbitrary detail field.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_vault_pm_audit --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p coding_adventures_vault_pm_audit --no-deps
```
