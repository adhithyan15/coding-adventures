# `coding_adventures_oauth_broker`

Provider-neutral, audit-first orchestration above the repository's pure OAuth
protocol core and storage-agnostic credential custody.

The broker registers any number of validated `ProviderConfig` values as data.
It contains no Google, Microsoft, GitHub, Dropbox, or other provider branch.
Static public-client registrations may be decoded from the exact, versioned
`BrokerProvider::from_public_provider_data` schema. Deployment-specific client
IDs and redirect URIs stay outside that file and are supplied by the caller;
unknown fields, duplicate fields, unsafe endpoints, implicit mix-up defenses,
and unsupported response formats fail closed. The broker can compose decoding
and registration through an injected `PublicProviderDataSource`: it durably
records the requested provider and trace before the source read, rejects a
profile that names another provider before registry mutation, and records the
closed read/decode result before proceeding to separately audited registration.
No file path, backend diagnostic, or profile byte enters broker audit data.
Registration enforces exclusive
redirect ownership whenever either provider relies on distinct-redirect mix-up
defense; providers that both validate RFC 9207 issuers may share a redirect.
For one provider/account key it can store an already audited initial token
response, release a still-usable access token to exactly one closure, or refresh
an expiring token through an injected transport. It also initiates an RFC 8628
device authorization request and composes exactly one externally scheduled
device-token poll through separate injected transports while preserving the
opaque polling session after a transient poll transport failure. Refresh-token disclosure,
request preparation, transport attempt/result, response decoding, credential
release, compare-and-swap rotation, metadata reads, and final access-token
release are all durable-audit gates.

The transport trait is deliberately an authority seam, not an HTTP library.
Concrete HTTPS and capability authorization remain separate packages. The
clock is injected, provider response format and refresh lead time are provider
data, and credential persistence remains an injected `CredentialStore`.

This crate adds no external library. Its bounded JSON decoder and every other
dependency are sibling packages in this repository.

## Deliberate exclusions

- browser and loopback-host orchestration;
- concrete HTTPS, TLS, or socket authority;
- concrete filesystem, vault, or embedded-resource provider-data sources;
- concrete encrypted-vault credential storage;
- account identity proof, listing, detach/revocation, device authorization UI,
  polling schedules, and full device-flow loops;
- confidential-client secrets, OIDC validation, and DPoP.

Those are independently reviewable follow-up slices in `code/specs/oauth.md`.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_oauth_broker --all-targets -- -D warnings
cargo doc -p coding_adventures_oauth_broker --no-deps
```
