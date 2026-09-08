# `coding_adventures_oauth_credential_sealed_store`

Concrete encrypted storage for `oauth-credential-custody` over any backend
accepted by `vault-sealed-store`. Provider behavior remains data: one fixed
namespace contains records keyed by validated provider ID and a hex-encoded
opaque account ID.

Credential plaintext uses a small versioned, length-prefixed binary envelope
with strict bounds. Secret strings and the encoded/decrypted envelope are held
in wipe-on-drop storage. Credential revisions are SHA-256 bindings to the
backend's opaque revision; conditional changes recover the exact current
backend revision and pass it to the underlying atomic CAS operation.

The adapter intentionally owns no audit sink. `CredentialCustody` durably
records provider, opaque account, trace, action, and closed outcome before
calling the adapter or releasing any secret or result.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_oauth_credential_sealed_store --all-targets -- -D warnings
cargo doc -p coding_adventures_oauth_credential_sealed_store --no-deps
```
