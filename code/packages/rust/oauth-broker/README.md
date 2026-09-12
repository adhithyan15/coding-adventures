# `coding_adventures_oauth_broker`

Provider-neutral, audit-first orchestration above the repository's pure OAuth
protocol core and storage-agnostic credential custody.

The broker registers any number of validated `ProviderConfig` values as data.
It contains no Google, Microsoft, GitHub, Dropbox, or other provider branch.
Static public- and confidential-client registrations may be decoded from exact,
versioned `BrokerProvider` schemas. Deployment-specific client IDs and redirect
URIs stay outside those files and are supplied by the caller; unknown fields,
duplicate fields, unsafe endpoints, implicit mix-up defenses, and unsupported
response formats fail closed. Confidential profiles must select exactly one
closed implemented method (`client_secret_basic`, `client_secret_post`, or
`private_key_jwt`); public `none`, missing, case-variant, and extension methods
are rejected. JWT profiles additionally require a bounded, unique, non-`none`
advertised algorithm set, while secret methods reject algorithm data. Exact
selection is retained as non-secret provider data without claiming a concrete
algorithm or acquiring credential or signer authority. The broker can compose
both public- and confidential-profile decoding and registration through
separate injected data-source contracts. It durably records the requested
provider and trace before the source read, rejects a profile that names another
provider before registry mutation, and records the closed read/decode result
before proceeding to separately audited registration. Confidential source
loading reads only provider policy and never acquires client-secret or
signing-key access. No file path, backend diagnostic, or profile byte enters
broker audit data.
For retained `client_secret_basic` and `client_secret_post` profiles, the
validated provider data can bind an exact provider-matched opaque secret key
into the existing client-secret adapter without letting the caller choose or
default the wire method. Public profiles, `private_key_jwt` profiles, and keys
for another provider fail before any credential access.
For retained `private_key_jwt` profiles, the broker similarly binds its exact
client ID, token-endpoint audience, advertised algorithm set, and a
provider-matched opaque private-key reference into the existing assertion
profile. The selected algorithm must appear exactly in provider data; this
pure construction invokes no signer and enables no concrete algorithm.
For client-secret profiles, prepared authorization-code exchange, refresh, and
RFC 7009 revocation requests can now cross the complete broker boundary: the
registered provider, client ID, exact operation endpoint, and retained
Basic/Post method are checked before audited secret custody access; custody
constructs zeroizing wire material; and broker-audited injected transport plus
bounded response decoding completes before result release. Revocation accepts
only exact HTTP 200 as confirmation, preserves HTTP 503 as a closed retryable
failure, and never deletes a local credential. These boundaries add no concrete
network implementation;
exchange can either return its audit-gated response or compose it directly into
an exact provider-bound opaque account key, crossing the OAuth credential
release and custody-create audit gates without credential disclosure. Refresh
does not rotate stored OAuth credentials.
Registration enforces exclusive
redirect ownership whenever either provider relies on distinct-redirect mix-up
defense; providers that both validate RFC 9207 issuers may share a redirect.
For one provider/account key it can store an already audited initial token
response, release a still-usable access token to exactly one closure, or refresh
an expiring token through an injected transport. It also initiates an RFC 8628
device authorization request and composes exactly one externally scheduled
device-token poll through separate injected transports. An opaque caller-timed
sequence additionally enforces the provider interval and relative expiry on an
arbitrary monotonic timeline, performs at most one poll per step, applies each
`slow_down` increase, and schedules from the actual attempt time so delayed
callers cannot trigger catch-up polling. It preserves the opaque polling state
after a transient transport failure. A caller may supply an exact opaque
account key so an authorized device response is stored through the existing
credential-custody boundary and only its opaque revision is released; token
bytes do not need to cross back into orchestration code. Refresh-token
disclosure, request preparation, transport attempt/result, response decoding,
credential release, compare-and-swap rotation, metadata reads, and final result
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
- account identity proof and key selection, listing, detach/revocation, device
  authorization UI, timing/sleep authority, and full device-flow loops;
- full confidential credential-refresh rotation, post-revocation local
  credential deletion, private-key signing orchestration, OIDC validation, and
  DPoP.

Those are independently reviewable follow-up slices in `code/specs/oauth.md`.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_oauth_broker --all-targets -- -D warnings
cargo doc -p coding_adventures_oauth_broker --no-deps
```
