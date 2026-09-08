# `coding_adventures_oauth_client_secret_custody`

Audit-first, storage-agnostic custody for OAuth confidential-client shared
secrets. Provider configuration stores only an opaque 32-byte reference; raw
secret bytes remain in wipe-on-drop ownership behind an injected atomic store.

Every create, access, rotation, and deletion publishes a durable attempted
event before storage access and a closed result event before returning a
revision or disclosing a secret to one closure. Events carry only the validated
provider, opaque reference, caller trace, action, and closed outcome.

This primitive is the prerequisite for data-driven `client_secret_basic` and
`client_secret_post` request authentication. Both methods are implemented for
authorization-code exchange, refresh, and revocation requests. Basic form
credentials are encoded before standard Base64 and are not duplicated in the
body; Post credentials remain only in the form body. Returned headers and
bodies are wipe-on-drop, and every request is matched to the exact provider and
client identity before access while inheriting the trace used for the custody
audit. Private-key JWT uses a separate non-exporting signing authority so
private key bytes never cross this boundary.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_oauth_client_secret_custody --all-targets -- -D warnings
cargo doc -p coding_adventures_oauth_client_secret_custody --no-deps
```
