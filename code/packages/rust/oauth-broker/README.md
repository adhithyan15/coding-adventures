# `coding_adventures_oauth_broker`

Provider-neutral, audit-first orchestration above the repository's pure OAuth
protocol core and storage-agnostic credential custody.

The broker registers any number of validated `ProviderConfig` values as data.
It contains no Google, Microsoft, GitHub, Dropbox, or other provider branch.
For one provider/account key it can store an already audited initial token
response, release a still-usable access token to exactly one closure, or refresh
an expiring token through an injected transport. Refresh-token disclosure,
request preparation, transport attempt/result, response decoding, credential
release, compare-and-swap rotation, metadata reads, and final access-token
release are all durable-audit gates.

The transport trait is deliberately an authority seam, not an HTTP library.
Concrete HTTPS and capability authorization remain separate packages. The
clock is injected, provider response format and refresh lead time are provider
data, and credential persistence remains an injected `CredentialStore`.

This crate adds no external library. Its dependencies are sibling packages in
this repository.

## Deliberate exclusions

- browser and loopback-host orchestration;
- concrete HTTPS, TLS, or socket authority;
- concrete encrypted-vault credential storage;
- account identity proof, listing, detach/revocation, and device flow;
- confidential-client secrets, OIDC validation, and DPoP.

Those are independently reviewable follow-up slices in `code/specs/oauth.md`.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_oauth_broker --all-targets -- -D warnings
cargo doc -p coding_adventures_oauth_broker --no-deps
```
