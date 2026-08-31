# `coding_adventures_oauth_credential_custody`

Audit-first, storage-agnostic custody for provider-neutral OAuth credentials.
The crate owns no filesystem, vault, database, network, browser, or clock
authority. A composition root injects the small `CredentialStore` compare-and-
swap contract and maps it to encrypted local vault storage or another trusted
secret store.

Account identities are opaque 32-byte values, provider behavior remains data,
and credential records retain access, refresh, and untrusted ID tokens only in
wipe-on-drop storage. Every create, disclosure, rotation, and delete is tagged
with the exact provider, opaque account, and caller trace. Durable audit intent
is required before storage access; durable success is required before a token,
revision, or write result is released.

Refresh rotation is one atomic compare-and-swap. Omitted refresh tokens retain
the current credential, new refresh tokens replace it, and a raced revision
fails closed. Raw token bytes, labels, scopes, provider errors, backend errors,
and storage coordinates never enter audit records or diagnostics.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_oauth_credential_custody --all-targets -- -D warnings
cargo doc -p coding_adventures_oauth_credential_custody --no-deps
```
