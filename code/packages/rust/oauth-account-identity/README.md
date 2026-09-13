# OAuth account identity

This crate is the audit-first account-key selection prerequisite for OAuth
credential custody. It turns bounded, wipe-on-drop ID-token evidence into a
provider-scoped opaque `CredentialKey` only through an injected trusted
identity authority.

The profile binds the exact provider, client audience, HTTPS issuer, allowed
case-sensitive JOSE algorithms, and an opaque provider-bound verification
context. The operation additionally binds the expected transaction nonce,
caller-supplied Unix time, and OAuth trace. Durable audit intent is required
before the authority sees token or nonce bytes, and durable success is required
before the verified opaque key is released.

The authority contract must verify the token signature, reject algorithms not
in the exact allowed set (including `none`), validate `iss`, `aud`, `exp`, and
nonce, and derive a stable provider-scoped opaque account identity without
returning `sub`. This package deliberately implements none of those mechanisms:
JWT parsing, JOSE, JWKS retrieval/cache policy, discovery, clock access,
networking, storage, and concrete cryptography remain separately injected.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_oauth_account_identity --all-targets -- -D warnings
cargo doc -p coding_adventures_oauth_account_identity --no-deps
```
