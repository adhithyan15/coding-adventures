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
The broker can also load the separate exact-schema ID-token identity policy
through an injected source. It requires an existing provider registration and
an opaque verification context naming that exact provider before the source
effect, derives the deployment client ID only from the registration, keeps
returned bytes zeroizing, and durably records the provider/trace-bound source
intent and closed decode result before releasing the validated identity
profile. No concrete source or verifier is added.
That load can now compose directly into nonce-bound Authorization Code identity
proof. The one-use nonce supplies the only trace; its provider and deployment
client, plus the opaque verification context's provider, must match the
registration before source access. The zeroizing ID token then crosses the
existing separately audited policy-load and trusted-verification gates, and a
final broker audit must be durable before only the provider-scoped opaque
credential key is released. The composition still adds no concrete source,
verifier, JWT/JOSE/JWKS algorithm, clock, storage, transport, or network
authority.
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
Prepared authorization-code exchange, refresh, and RFC 7009 revocation requests
can then cross broker-level assertion boundaries: the registered provider,
client ID, exact operation endpoint, retained method, and exact algorithm are
checked before the existing audited non-exporting signer is invoked; injected
transports are separately audit-gated; and bounded response decoding or
classification completes before release. Revocation requires its explicitly
configured endpoint as the assertion audience and never assumes token-endpoint
acceptance. The caller still owns issued-at time and 256-bit replay entropy, and
no concrete signing or network implementation is added. A separate composition
validates that retained profile before credential access, releases the exact
opaque account record's refresh token and revision, and conditionally deletes
only that revision after exact HTTP 200 crosses signer, transport, protocol,
custody, and broker audit gates. For records without a refresh token, a
separate fallback proves refresh absence inside custody before releasing the
access token and exact revision, then applies the same signing, transport,
response, and conditional-delete gates. Refreshable records and every later
failure retain the local credential.
Authorization-code exchange can likewise compose its bounded response directly
into an exact provider-bound opaque account key. Key mismatch fails before
signing, transport, clock, or custody access; the response crosses the OAuth
credential-release and custody-create audit gates inside the broker, and only
the opaque revision is released. Stored refresh credentials can cross the same
retained profile, audited abstract signer, injected transport, bounded decoder,
caller-owned clock, OAuth credential release, and revision-bound custody
rotation without leaving the broker. Invalid profiles fail before credential
access, and every later failure retains the prior record.
For client-secret profiles, prepared authorization-code exchange, refresh, and
RFC 7009 revocation requests can now cross the complete broker boundary: the
registered provider, client ID, exact operation endpoint, and retained
Basic/Post method are checked before audited secret custody access; custody
constructs zeroizing wire material; and broker-audited injected transport plus
bounded response decoding completes before result release. Revocation accepts
only exact HTTP 200 as confirmation, preserves HTTP 503 as a closed retryable
failure, and the lower-level send never deletes a local credential. A separate
composition reads the exact opaque account record's refresh token and revision,
revokes that token through the retained client-secret method, and conditionally
deletes only that revision after exact HTTP 200 and every intervening audit gate.
Provider failures, transport failures, missing refresh tokens, and binding
failures leave the credential record intact. For records that have no refresh
token, a separate access-token fallback releases that token and exact revision
only after custody proves refresh absence; it then applies the same retained
client-secret, transport, response, and conditional-delete gates. These
boundaries add no concrete network implementation. Exchange can either return
its audit-gated response or compose it directly into
an exact provider-bound opaque account key, crossing the OAuth credential
release and custody-create audit gates without credential disclosure. A stored
refresh token can likewise cross exact retained Basic/Post authentication,
injected transport, bounded response decoding, caller-owned time, OAuth
credential release, and revision-bound custody rotation without leaving the
broker. Authentication mismatch is rejected before credential access, and
transport, provider, clock, release, or compare-and-swap failures retain the
prior record.
Public-client profiles now have the corresponding refresh-token detach
boundary without acquiring an authentication secret or signer. The exact
opaque account key selects the registered public `none` profile and credential
record; that retained profile and its explicit revocation endpoint are checked
before custody releases the stored refresh token and revision into the
zeroizing RFC 7009 request. Injected transport, response classification, and
conditional deletion remain separately audited with the same provider and
trace. Only exact HTTP 200 permits deletion of the original revision, so
confidential-profile mismatch, absent refresh data, transport or provider
failure, and a concurrent revision change retain local credentials. This slice
adds no access-token fallback or concrete network implementation.
The usable-access composition applies that same retained client-secret policy
before even reading credential metadata. A still-fresh access token reaches
only the existing audited custody closure and invokes neither secret custody nor
transport. A token inside the provider-data refresh lead crosses the complete
audited refresh and revision-bound rotation path first, so only the new stored
access token can reach the closure. The clock, secret store, token transport,
credential store, and closure all remain injected or caller-owned.
The symmetric `private_key_jwt` usable-access composition validates the complete
retained provider, client, token-endpoint, method, algorithm, and opaque-key
profile before credential or clock access. Fresh credentials invoke neither the
abstract signer nor transport and discard caller replay entropy through
zeroizing ownership. Due credentials cross the existing separately audited
assertion, transport, response, release, and revision-bound rotation gates
before custody discloses the newly stored access token to one closure. No
concrete signing algorithm or network authority is enabled.

For an OpenID Connect Authorization Code exchange using retained client-secret
authentication, the broker can instead consume the exact non-cloneable
authorization nonce and route the bounded response through the existing
account-identity authority before custody creation. Provider, client, and trace
bindings are checked before secret access or transport. The response then
crosses the OAuth credential-release audit, its zeroizing ID token is detached
without cloning and consumed by audited verification, and only the authority's
provider-scoped opaque account key selects storage. Missing or rejected ID-token
evidence reaches no credential store, the consumed evidence is omitted from the
stored record, and only the opaque revision is released. Verification, time,
transport, and storage implementations remain injected.

The broker can now load the exact static ID-token policy directly within that
client-secret exchange-to-custody composition. The retained authentication,
request endpoint/client, opaque verification context, and nonce are all bound
to the registration before the injected policy source is read. Policy loading,
secret access, transport, response release, verification, and storage retain
their separate provider/trace audit gates, and a final composite audit precedes
release of only the opaque revision. The caller supplies no decoded identity
profile and gains no concrete source, verifier, clock, transport, or storage
authority.

The same verified-identity custody path is available for retained
`private_key_jwt` profiles through the existing audited abstract signer. Its
provider/client/nonce bindings fail before signing, and the composition adds no
concrete signing or ID-token verification algorithm.

The RFC 8628 initiation path now has an explicit OpenID Connect composition.
It requires the exact case-sensitive `openid` scope, obtains an independent
256-bit nonce from injected entropy, includes it in the audited device request,
and retains it as non-cloneable zeroizing provider/client/trace-bound state
beside the validated display and polling state. Registered profile binding is
checked before entropy or transport access. Identity verification, opaque
account-key selection, polling time, storage, and concrete network authority
remain separate.

That state can now enter an opaque caller-timed OIDC polling sequence without
separating its nonce from the device session. Early waits, pending responses,
provider slow-down, and transient transport failure retain the same
provider/client/trace-bound nonce. Authorization returns the audited token
response only as a nonce-paired value for a later ID-token proof; denial and
expiry discard the nonce. The composition reuses the existing registry,
protocol, transport, and broker audit gates and adds no verifier, account-key
selection, storage, clock, sleep, or concrete transport.

An authorized nonce-paired device response can now cross the existing audited
identity authority and credential custody in one composition. The registered
provider, deployment client, response, nonce, identity profile, and trace must
all match before clock access or credential release. The zeroizing ID token is
detached and consumed by proof rather than stored; only the remaining
credentials are created under the authority-derived opaque provider/account
key, and only the opaque storage revision is returned. Missing or rejected
identity evidence reaches no credential store. The identity profile must
already be validated, and concrete policy loading, verification algorithms,
clock, storage, transport, and waiting remain separate authorities.

The device path can also load the exact static ID-token policy inside that
composition. The opaque verification context, response, retained nonce,
registered deployment client, provider, and trace are bound before source or
later authority access. Policy loading, clock access, credential release,
identity proof, custody creation, and both composite broker results retain
separate audit gates. The caller supplies no decoded identity policy, and this
adds no concrete source, verifier, algorithm, clock, storage, or transport.

One caller-timed step can now carry the nonce-bound polling sequence into that
same static-policy custody path. The verification context is provider-bound
before polling. Early waits, pending, slow-down, and transient transport
failure return only the opaque sequence and reach no policy source, wall clock,
identity authority, or credential store. Authorization continues through the
separate policy-load, proof, credential-release, and custody audit gates and
returns only the stored opaque revision.

That `private_key_jwt` path can also load the exact static ID-token policy
inside the composition. The retained provider/client/endpoint/algorithm/key
profile, opaque verification context, and nonce are validated before the
policy source, signer, transport, clock, verifier, or credential store is used.
Policy loading, signing, transport, response release, identity verification,
and storage keep their separate provider/trace audit gates; only the opaque
credential revision is released.

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
- concrete account-identity verification/JWKS, identity composition for other
  grant paths, account listing, device authorization UI, timing/sleep
  authority, and full device-flow loops;
- concrete private-key algorithms, OIDC validation, and DPoP.

Those are independently reviewable follow-up slices in `code/specs/oauth.md`.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_oauth_broker --all-targets -- -D warnings
cargo doc -p coding_adventures_oauth_broker --no-deps
```
