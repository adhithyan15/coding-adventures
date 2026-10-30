# OAuth

**Status:** Phase 2 provider-neutral broker boundary implemented —
installed-app Authorization Code + PKCE, RFC 8628 device-flow initiation and
caller-driven polling classification and sequencing,
token/error codecs, refresh rotation, revocation request/response
classification and exact refresh- or access-token detach, RFC 8414
metadata validation, and an audited
literal-loopback callback host plus storage-agnostic audit-before-disclosure
credential custody now compose through data-driven provider registration,
expiry policy, injected transport, and compare-and-swap refresh orchestration;
the custody contract has a bounded zeroizing encrypted-store adapter over any
`vault-sealed-store` backend; concrete transport, complete account lifecycle,
and verified live provider snapshots remain prioritized below.

## Overview

`oauth` is the reusable OAuth 2.0 client primitive, updated by RFC 9700's
OAuth Security Best Current Practice, for the
agent system. Agents that need to talk to a third-party service —
Gmail, GitHub, Google Calendar, Slack, Spotify, anything that
speaks OAuth — go through this crate. Tokens live in the vault
under a stable namespace; the broker handles refresh transparently;
the agent's manifest gates which providers it can talk to.

The spec covers the parts of OAuth that the agent system actually
needs:

- **Authorization Code with PKCE (RFC 7636)** — the standard flow
  for user-facing apps. Browser opens, user logs in, redirects back
  with an auth code, broker exchanges the code for an access +
  refresh token.
- **Device Authorization Grant (RFC 8628)** — the headless-friendly
  fallback. The user goes to a URL on their phone and types a code
  the broker displays. Useful for the orchestrator when no browser
  is available, and for mobile / TV / IoT clients.
- **Refresh Token Grant** — exchange a refresh token for a new
  access token without bothering the user. Auto-fired when an
  access token has less than a configurable lead time remaining.
- **Token Revocation (RFC 7009)** — when an account is detached or
  a user explicitly revokes consent, the broker calls the
  provider's revocation endpoint.
- **OpenID Connect ID tokens** — when present, retained as opaque OAuth token
  material until a later OIDC layer verifies signature, issuer, audience,
  expiry, and transaction nonce before exposing `sub`. After the decoder's
  credential-release audit gate, ownership can be split without cloning into
  zeroizing ID-token evidence and a remaining access/refresh credential bundle
  for later verified-identity custody composition.

We deliberately exclude:

- **Implicit Grant** (not recommended for native apps by RFC 8252 and omitted
  by the RFC 9700 security baseline).
- **Resource Owner Password Credentials** (deprecated; insecure).
- **Token Introspection (RFC 7662)** as a client — agents don't
  introspect their own tokens.
- **Dynamic Client Registration (RFC 7591)** — providers we care
  about all use static client IDs.

The pattern mirrors the rest of the substrate: one trait
(`OAuthClient`) with a higher-level broker (`OAuthBroker`) that
adds vault storage, auto-refresh, and account management. Provider
specifics (URLs, scopes, quirks) live in `ProviderConfig` data, not
in code, so adding a new provider means writing a config — not a
new client.

## Implementation Boundary and Prioritized Backlog

OAuth and remote storage are independent reusable layers. The OAuth core never
imports a Google Drive, OneDrive, Dropbox, GitHub, or other provider SDK; the
vault storage contract never imports OAuth. A composition root joins a generic
OAuth credential to one storage adapter.

The delivery order is:

1. **Shipped by the first `coding_adventures_oauth` slice:** strict public-client
   provider configuration; caller-injected 256-bit entropy; mandatory PKCE
   `S256`; authorization URL construction; exact redirect, state, and explicit
   RFC 9207 issuer-or-distinct-redirect mix-up defense; one-use callback
   completion; secret-bearing token-exchange preparation; zeroizing ownership;
   closed diagnostics; caller-owned trace correlation; and privacy-safe audit
   descriptors for successful, denied, and failed begin/completion attempts.
   The result wrapper exposes no bypass around durable
   `publish_then_release`, so audit failure fails closed.
2. **Shipped OAuth token boundary:** bounded JSON and form token/error
   responses, refresh grant requests, explicit rotating refresh-token
   semantics, and RFC 7009 revocation preparation plus response classification.
   Only exact HTTP 200 confirms revocation and its bounded body is ignored;
   bounded HTTP 400/401 OAuth errors and retryable HTTP 503 are closed
   classifications. Token or raw provider response bytes never enter errors or
   audit data, and requests, success, and error results are audit-gated before
   release. Retry timing, transport, and credential removal remain caller-owned.
3. **Shipped authorization-server metadata:** audited RFC 8414 well-known
   request preparation, bounded JSON decoding, exact issuer validation, strict
   endpoint policy, explicit Authorization Code + PKCE `S256` negotiation,
   immutable public and confidential authentication-capability retention, and
   public provider-config derivation that separately requires an explicitly
   advertised `none` method and never silently widens metadata-derived
   behavior. Unverified
   `signed_metadata` and unknown fields are ignored and scrubbed, not trusted.
   RFC 9207 response-issuer support is retained exactly; providers that omit it
   require the existing registry-owned distinct-redirect defense.
   Metadata and token responses now use the repository-owned zero-dependency
   `bounded-json` parser: exact RFC 8259 grammar, duplicate preservation for
   protocol-level rejection, closed input-free errors, and depth rejection
   during parsing rather than after an unbounded recursive walk. The OAuth
   normal dependency graph contains only workspace packages.
4. **Shipped installed-app host:** audit-bracketed external-browser release and
   a transient listener bound only to an IP-literal loopback redirect, with
   provider/trace/redirect binding, exact aggregate/request-line/header/Host
   bounds, zeroizing callback ownership, closed diagnostics, and the listener
   closed immediately after the first accepted connection. Browser launch is
   an injected platform adapter; provider behavior remains data in the pure
   OAuth core.
5. **Shipped credential custody and provider-neutral broker:** opaque account
   keys, injected compare-and-swap storage, zeroizing credential records,
   no-clone codec handoff, atomic refresh-token retain/rotation, conditional
   deletion, and audit-before-access-token or refresh-token disclosure. Audit
   publication failure fails the operation closed. The broker registers any
   number of providers as validated data, applies caller-injected clock and
   expiry policy, audit-brackets its injected token transport, and rotates
   credentials with the custody layer's exact revision. The concrete
   `oauth-credential-sealed-store` adapter maps provider/opaque-account keys to
   bounded, versioned, zeroizing encrypted records over any
   `vault-sealed-store` backend. Backend revision tokens are cryptographically
   bound to fixed-size custody revisions while the exact backend token is used
   for every atomic compare-and-swap or delete. HTTPS authority remains a
   separate adapter. An audit-first account-identity proof boundary is also
   shipped as a prerequisite for selecting those opaque account keys. It
   accepts only bounded zeroizing ID-token evidence plus exact provider,
   client, issuer, nonce, caller-time, allowed-algorithm, and opaque
   verification-context data; an injected trusted authority must verify the
   signature and all bound claims before returning a provider-scoped opaque
   account identity. Verification intent and its closed result are durably
   provider/trace audited before the authority effect and result release. For
   static provider policy, a bounded exact-schema decoder retains only the
   provider, HTTPS issuer, and case-sensitive non-`none` ID-token signing
   algorithms while deployment client identity and the opaque verification
   context remain injected. It performs no source read and enables no
   verification algorithm. A broker boundary can read that policy through an
   injected source only after exact registered-provider and opaque-context
   binding, auditing the provider and trace before the read and its closed
   decode result before profile release; concrete source and verification
   authority remain absent. A broker composition can now carry that exact
   source load directly into Authorization Code identity proof. It derives the
   trace only from the one-use nonce, rejects registered-provider, deployment-
   client, nonce, or opaque-context mismatch before the source read, preserves
   the source and trusted-verifier audit gates, and withholds the verified
   provider-scoped opaque credential key until a final broker result audit is
   durable. It adds no concrete source or verification authority. For
   Authorization Code identity proof, the underlying boundary consumes the core's
   non-cloneable nonce object, validates its exact provider and client, and
   derives the audit trace from that browser ceremony before any authority
   effect. The boundary exposes no subject claim or token bytes and implements
   no JWT, JOSE, JWKS, discovery, network, clock, storage, or concrete cryptographic
   authority. The authorization core now also provides an explicit OIDC begin
   path: exact `openid` scope is mandatory, caller-injected entropy supplies an
   independent 256-bit `nonce` beside state and PKCE, provider extras cannot
   override it, and a non-cloneable zeroizing provider/client/trace-bound nonce
   is retained for the later proof. A broker composition now carries one
   client-secret-authenticated Authorization Code response through the OAuth
   credential-release gate, detaches its zeroizing ID-token evidence without a
   clone, verifies that evidence with the exact retained nonce, and creates the
   credential record only under the resulting provider-scoped opaque account
   key. Provider, client, and trace mismatches fail before secret access or
   transport; missing or rejected identity evidence fails before credential
   storage; the consumed ID token is not retained in the stored credential.
   Concrete JWT, JOSE, JWKS, discovery, clock, storage, and network authorities
   remain injected or out of scope. That complete client-secret exchange path
   can now load the exact static identity policy internally: retained
   authentication, request endpoint/client, opaque verification context, and
   nonce are bound to the registration before the source read; the source,
   secret, transport, verifier, and storage effects retain separate
   provider/trace audit gates; and only the final opaque credential revision is
   released. The caller no longer supplies a decoded identity profile.
   The symmetric `private_key_jwt` exchange can now load that exact static
   identity policy internally after the retained provider/client/endpoint/
   algorithm/key profile, opaque verification context, and nonce are bound to
   the registration. Policy loading precedes signing, transport, clock,
   verification, and credential storage, while each authority keeps its own
   provider/trace audit gates and only the opaque credential revision is
   released. This enables no concrete signing or verification algorithm.
6. **Confidential-client authentication:** web-service profiles for
   `client_secret_basic`, `client_secret_post`, and `private_key_jwt`, using
   opaque custody references and audit-before-release rather than secrets in
   provider configuration. The storage-agnostic client-secret custody
   prerequisite is shipped: create, one-closure access, atomic rotation, and
   conditional deletion are all audit-bracketed with provider and trace while
   secret ownership remains zeroizing. Provider-driven `client_secret_basic`
   and `client_secret_post` are now bound through that custody boundary into
   authorization-code exchange, refresh, and revocation requests. Basic
   credentials are form-encoded before standard Base64 and omitted from the
   body; Post credentials remain only in the form body. Returned headers and
   bodies are zeroizing, reject provider or client-identity mismatches before
   secret access, and retain the provider/trace audit binding. Validated RFC
   8414 metadata now selects these methods only after exact provider and
   case-sensitive method matching, without inventing a default. The same
   metadata can now derive a PKCE-enforced confidential `ProviderConfig` only
   for the closed implemented set (`client_secret_basic`, `client_secret_post`,
   or `private_key_jwt`) and only when the selected method was advertised
   exactly. RFC 9207 or registry-owned distinct-redirect mix-up defense remains
   mandatory, while both public derivation paths still require explicit
   `none`. A
   non-exporting signer boundary for `private_key_jwt` is shipped: opaque
   provider-bound key references, algorithm-as-data, bounded zeroizing
   signature ownership, and durable trace-correlated audit before signing and
   before result release. RFC 8414 signing algorithms are retained exactly as
   provider data, required when JWT client authentication is advertised, and
   reject `none` without inventing defaults. No concrete software algorithm is
   implied. Bounded RFC 7523 assertion construction is now shipped through
   that boundary: exact provider-advertised method and algorithm checks,
   `iss`/`sub`/`aud`/`exp`/`iat` claims, caller-entropy replay-resistant `jti`,
   zeroizing JWS material, and provider/client/audience/trace bindings retained
   for request binding. Provider-driven binding into authorization-code
   exchange, refresh, and revocation requests is now shipped: exact
   provider/client/endpoint-audience checks happen before signing, the request
   trace drives signer and later transport audit, secret-bearing form bodies
   remain zeroizing, and exchange/refresh response contexts are preserved.
   Revocation uses an explicitly configured revocation-endpoint audience rather
   than inventing provider acceptance of the token endpoint as that audience.
   Its request-bound response classifier now treats only exact HTTP 200 as
   revoked, ignores the bounded success body, validates bounded JSON errors for
   HTTP 400/401, and retains HTTP 503 as a closed retryable error before any
   later broker transport or local credential-removal authority is added.
   Broker-level client-secret authorization-code exchange, refresh, and RFC
   7009 revocation sends are also shipped: the registered provider, client ID,
   exact operation endpoint, and retained Basic/Post method are checked before
   audited client-secret access; custody constructs the zeroizing request; and
   broker-audited injected transport plus bounded response decoding completes
   before release. Revocation releases success only for exact HTTP 200 and keeps
   HTTP 503 retryable. The composed detach path reads the refresh token and
   revision from the exact opaque account record, then conditionally deletes
   only that revision after remote success and every audit gate; binding,
   custody, transport, and provider failures retain local credentials. Initial
   client-secret exchange can now compose directly into an exact opaque account
   key: key/provider mismatch fails before secret, transport, clock, or storage
   access; OAuth credential release and custody creation remain separately
   audited; and only the opaque revision is returned. Confidential refresh now
   composes the exact stored refresh token through retained Basic/Post policy,
   audited secret access and transport, bounded response decoding, caller-owned
   time, OAuth credential release, and revision-bound atomic retain/rotation.
   Binding and all later failures preserve the prior record. A client-secret
   access-token-only fallback now releases the token and exact revision only
   when custody proves that no refresh token exists, then conditionally deletes
   after exact HTTP 200 and every audit gate. Refreshable credentials, retryable
   provider failures, and stale revisions retain local state. Client-secret
   Authorization Code exchange can additionally derive its opaque account key
   through the audited ID-token identity boundary before initial custody
   creation, eliminating caller key selection for that path. The exact static
   identity policy can now be loaded inside that same client-secret composition
   after registered request/authentication/context/nonce binding and before any
   credential or transport effect, eliminating caller-owned decoded policy as
   well. Other grant and authentication paths still keep identity proof and key
   selection as separate composition steps. The corresponding `private_key_jwt` exchange
   can now use the same nonce-bound identity-to-custody path through the
   existing abstract signer, without enabling any concrete signing algorithm.
   Other grants still keep identity proof and key selection separate. Prepared
   `private_key_jwt` authorization-code exchange, refresh, and
   RFC 7009 revocation requests can now cross the broker through the existing
   abstract signer: exact retained provider, client, operation endpoint,
   method, and case-sensitive algorithm checks precede signing; signer and
   injected transport effects/results are separately audited with the same
   provider and trace; and bounded response decoding or classification
   completes before release. Revocation derives a separate profile from the
   explicitly configured revocation endpoint and never invents token-endpoint
   audience acceptance. An exact opaque account record can now compose its
   stored refresh token and revision through that path: the complete profile is
   revalidated before credential access, and local deletion is conditional on
   exact HTTP 200 plus every signer, transport, protocol, custody, and broker
   audit gate. An access-token-only fallback additionally requires custody to
   prove refresh-token absence before the access token and exact revision are
   released; refreshable records fail before signing or transport, and every
   later failure retains the record. Private-key-JWT authorization-code
   exchange can also route its bounded response directly into an exact opaque
   account key: key/provider mismatch fails before signer, transport, clock, or
   custody access; OAuth credential release and custody creation remain
   separately audited; and only the opaque revision returns. Issued-at time and
   256-bit replay entropy remain caller-owned. Stored refresh credentials can
   now compose through the same exact profile, audited signer and transport,
   bounded decoding, caller-owned clock, OAuth credential release, and
   revision-bound atomic rotation. Profile mismatch fails before credential
   access, while signing, transport, clock, release, and compare-and-swap
   failures preserve the prior record. This enables neither a concrete signing
   algorithm nor network authority. Authorization Code exchange for the
   retained public `none` profile can now cross an injected transport and
   persist directly under one caller-selected opaque account key. The exact key
   provider, registered client ID, token endpoint, and public authentication
   profile are validated before transport, clock, credential release, or
   storage access. The zeroizing PKCE exchange request stays inside the
   transport boundary; bounded response decoding, OAuth credential release,
   and custody creation retain their separate provider/trace audit gates, and
   only the opaque revision is returned. Wrong binding, transport or provider
   failure, audit failure, clock failure, and an existing key create no new
   credential record. Account identity proof and key selection remain
   caller-owned for that entry point, and this adds no concrete network
   authority. The public exchange can now instead consume a nonce-bound ID
   token through the existing audited identity authority and create custody
   only under the resulting provider-scoped opaque account key. Exact identity
   profile, request provider/client/trace, and one-use nonce binding is checked
   before transport, clock, verification, or credential storage; missing or
   rejected identity evidence reaches no custody, and the consumed ID token is
   never stored. A static-policy variant can now instead select the registered
   provider's exact decoded identity profile through an opaque verification
   context before exchange. The registered public profile, client, token
   endpoint, verification context, and nonce binding are validated before the
   policy source, transport, clock, verifier, or custody is reached. Policy
   loading, public transport, identity proof, credential release, and custody
   creation retain separate provider/trace audit gates; only the opaque
   credential revision leaves the composition, and the consumed ID token is
   never retained. The policy source remains injected and no concrete identity
   verifier, network, clock, storage, provider data, or signing algorithm is
   enabled. Public-client
   refresh-token detach is now shipped for the retained `none` profile: the
   exact opaque account key binds
   the registered provider and credential record, and the retained public
   authentication profile plus explicit revocation endpoint are validated
   before credential access. Custody releases that record's refresh token and
   revision into a zeroizing RFC 7009 request containing the registered client
   ID; injected transport and bounded response classification remain separately
   audited with the same provider and trace. Local deletion is conditional on
   exact HTTP 200 and the original revision, so confidential-profile mismatch,
   missing data, transport or provider failure, and stale revision all retain
   the credential. An access-token-only fallback now uses the same retained
   public profile and audited transport path, but custody releases the access
   token and exact revision only after proving that no refresh token exists.
   Refreshable credentials fail before transport, while exact HTTP 200 alone
   permits revision-bound local deletion; retryable provider failures and stale
   revisions retain local state. These boundaries add no concrete transport,
   provider-specific data, or external dependency. The original usable-access
   and forced-refresh entry points are now explicitly public-client paths: the
   exact opaque account key must select a registered provider retaining the
   public `none` profile before credential metadata, refresh material, clock,
   transport, or token-use closure access. A still-fresh public credential
   crosses only the existing audited custody closure; a due or explicitly
   refreshed credential crosses the existing audited refresh, response,
   credential-release, and revision-bound atomic-rotation gates. Confidential
   provider bindings fail before custody and retain their credential record.
   This binding adds no new transport, storage, clock, provider data, or
   external dependency. A client-secret
   usable-access composition
   now validates the exact retained Basic/Post method and provider-bound secret
   key before credential or clock access. Still-fresh credentials invoke no
   secret or transport authority; due credentials cross the existing audited
   refresh and revision-bound atomic rotation path before the resulting access
   token is disclosed through the custody closure. Clock, transport, secret
   storage, credential storage, and token use remain injected authorities. The
   symmetric `private_key_jwt` usable-access path validates the full retained
   provider/client/endpoint/method/algorithm/key profile before credential or
   clock access, invokes neither signer nor transport for a fresh credential,
   and carries a due credential through the existing separately audited
   assertion, transport, response, release, and atomic-rotation gates before
   custody disclosure. Unused replay entropy stays zeroizing, and no concrete
   signing algorithm is enabled.
   The repository-owned Ed25519 implementation must first replace
   secret-dependent scalar branches/loops and scrub key-derived temporaries
   before an `EdDSA` authority can be enabled; `RS256` requires a separate
   repository-owned RSA primitive. The RFC 8414 decoder now retains exact
   confidential-only authentication capabilities even when public-client
   `none` is omitted, while public `ProviderConfig` derivation continues to
   require an explicit `none`. Public native clients remain `none` + PKCE.
7. **Device Authorization Grant:** metadata-bound RFC 8628 public-client
   initiation and polling classification are shipped: exact device endpoint,
   grant, and `none` authentication capability checks; bounded zeroizing
   response ownership; strict HTTPS verification URIs and bounded
   lifetime/interval values; audited device-code poll preparation; exact
   `authorization_pending`, `slow_down`, denial, expiry, and token-success
   transitions; and persistent five-second slow-down increases. The device code
   remains inside an opaque provider/client/token-endpoint/trace-bound session,
   each sent request becomes one response context, and each transient wire body
   is zeroizing. The broker can initiate device authorization and execute one
   externally scheduled poll through separate injected transports after exact
   registry binding, returning the opaque session on transient poll transport
   failure and auditing each effect and result before release. An opaque
   caller-timed sequence now rejects early and locally expired steps before
   request preparation or transport, performs at most one poll per call,
   reschedules from the actual attempt time, and retains cumulative
   `slow_down`. The caller still supplies monotonic observations and owns
   waiting and UI. A caller-selected exact opaque account key can now compose
   an authorized poll directly into the existing audited credential-custody
   boundary; only the opaque storage revision is released, while continuation
   outcomes access neither the injected wall clock nor credential storage.
   Account identity and key selection remain caller-owned. These boundaries
   add no sleep or concrete network authority. An explicit OpenID Connect
   device-initiation prerequisite now requires the exact case-sensitive
   `openid` scope, obtains an independent 256-bit nonce from caller-injected
   entropy, includes that nonce in the audited device-authorization request,
   and returns it as non-cloneable zeroizing provider/client/trace-bound state
   beside the validated verification and polling state. Registered profile
   binding is checked before entropy or transport access. This adds no identity
   verifier or account-key selection yet; it prepares a later authorized poll
   to cross the existing audited ID-token proof boundary without accepting a
   caller-invented or replay-ambiguous nonce. The nonce-bound polling
   prerequisite is also explicit: one opaque OIDC sequence carries the exact
   device session and nonce through early waits, `authorization_pending`,
   `slow_down`, and retryable transport failure. Only an authorized token
   response is paired with that nonce for a later identity proof; denial and
   expiry discard it. This adds no verifier, account-key selection, storage,
   clock, sleep, or concrete transport authority. An authorized nonce-paired
   response can then cross the existing audited ID-token authority and
   credential-custody gates using one already validated, data-driven identity
   profile. Exact provider, deployment-client, and trace binding is checked
   before clock access or credential release. The zeroizing ID token is
   detached and consumed by identity proof rather than stored; only the
   remaining credentials are created under the proof-derived opaque account
   key, and only the storage revision is released. Missing or rejected identity
   evidence reaches no credential store. The same composition can load the
   exact static identity policy internally after binding the opaque verification
   context, response, nonce, registered deployment client, provider, and trace,
   so the caller supplies no decoded identity policy. Policy-source,
   verification, clock, credential-release, custody, and broker audits remain
   separate. One caller-timed step can now preserve that opaque OIDC sequence
   across early waits, pending, slow-down, and retryable transport failure, then
   carry an authorized response directly through the same static-policy,
   identity-proof, and derived-key custody path. The opaque verification context
   is provider-bound before polling, continuation outcomes reach no policy,
   clock, verifier, or credential authority, and only a stored revision is
   released after authorization. Concrete sources, verification algorithms,
   clock/storage implementations, transport, waiting, and UI remain separate
   injected or caller-owned authorities.
8. **HTTPS transport:** provider-neutral request/response types over the
   repository's TLS and HTTP primitives, with endpoint/capability authorization
   before any socket is opened.
9. **Provider data fixtures:** a language-neutral, zero-dependency conformance
   contract now drives synthetic public RFC 9207, distinct-redirect, device,
   confidential, user-defined, and fail-closed metadata profiles through the
   same decoder and audited derivation paths. The synthetic corpus explicitly
   claims no live provider values. A bounded, exact-schema static public-client
   profile decoder now creates broker registrations from caller-owned client
   IDs and redirect URIs; endpoint, mix-up-defense, response-format,
   refresh-lead, and authorization-parameter policy remain data. An injected
   provider-data source now composes public profile loading with registration:
   the broker durably audits the exact requested provider and caller trace
   before the source read, rejects a returned profile naming another provider
   before registry mutation, keeps profile bytes zeroizing, and records a
   closed source/decode result before separately audited registration. A
   parallel confidential-only source contract now provides the same gates for
   confidential static profiles while reading provider policy only, never a
   client secret or signing key. Concrete file, vault, or embedded-resource
   lookup remains host-owned. An exact-schema confidential profile variant
   retains one selected closed implemented method
   (`client_secret_basic`, `client_secret_post`, or `private_key_jwt`) as
   provider data. It rejects public `none`, absence, case variants, and
   user-defined methods. JWT profiles require a bounded, unique, non-`none`
   advertised algorithm set; secret methods reject algorithm data. Exact
   algorithms remain provider data without claiming an implementation, and the
   decoder acquires no secret, key, signer, or storage authority. The broker
   can now bind a retained `client_secret_basic` or `client_secret_post`
   selection directly to an exact provider-matched opaque secret key and the
   existing custody adapter. Public and `private_key_jwt` profiles fail closed,
   and no caller-selected or default secret method reaches credential access.
   A retained `private_key_jwt` selection can likewise bind the exact provider,
   client ID, token-endpoint audience, advertised algorithm set, and an opaque
   provider-matched key into the existing assertion profile. Public and
   client-secret profiles plus unadvertised algorithms fail closed before any
   signer access; constructing this profile enables no concrete algorithm. The
   broker rejects redirect reuse when either registration relies on
   distinct-redirect mix-up defense while allowing shared redirects only when
   both providers validate RFC 9207 response issuers.
   Verified Google, Microsoft, GitHub, Dropbox, Slack, and Spotify snapshots
   remain to be added through this same suite; provider quirks remain data and
   none creates a new OAuth implementation.
10. **Later hardening:** DPoP and full OpenID
   Connect discovery/JWKS/ID-token validation.

The sealed store's legacy JSON chain is now removed from its normal dependency
graph: shared storage metadata uses the repository-owned bounded JSON value
model, `storage-fs` parses and serializes through that primitive, and the legacy
`json-value` facade re-exports the same types for source-compatible consumers.
Metadata is rejected before backend dispatch when it is non-finite or exceeds
the shared depth limit. This enabled the shipped concrete encrypted OAuth
credential-store adapter. The
cross-platform kernel-entropy prerequisite is shipped: `csprng` now reads
`/dev/urandom` through safe standard-library I/O on Unix and confines Windows
CNG FFI to one documented `BCryptGenRandom` call, with no normal dependencies.
OAuth production dependency graphs must contain only workspace-owned
libraries; operating-system APIs and wire protocols are the lowest trusted
boundary.

Before the concrete HTTPS adapter, replace `tls-platform`'s `rustls` dependency
chain with repository-owned TLS, certificate parsing, path validation,
hostname verification, and signature verification. `webpki-roots` is the sole
approved external production data dependency and may supply only a reviewed
Mozilla public-Web-PKI trust-root snapshot. Its integration must pin one exact
reviewed release, record the official upstream tag or commit, crates.io
checksum, root count, and root-set digest, and add CI that rejects unreviewed
root-set changes. The integration must also document that OS enterprise and
local roots are excluded and define a deployed-client update cadence.
`webpki-roots` supplies no TLS, certificate parsing, path or hostname
validation, signature verification, revocation, or platform-trust logic. This
is prerequisite work, not permission to weaken HTTPS or certificate checks.
The first independently reviewable prerequisite is now shipped as
`tls-server-identity`: a zero-external-dependency RFC 9525 matcher for already
decoded DNS-ID and IP-ID subjectAltName values. It provides bounded ASCII DNS
validation, exact typed IP comparison, case-insensitive label matching, strict
one-label wildcards, and no Common Name input. It does not parse certificates,
validate paths, load roots, open sockets, or integrate with `tls-platform`, so
the concrete OAuth HTTPS boundary remains blocked on all remaining TLS work.
The next prerequisite is also shipped as `der-tlv`: a zero-external-dependency,
allocation-free decoder for canonical DER identifier and definite-length
framing with explicit input, value, tag, and sibling limits. It rejects BER
indefinite lengths, non-minimal encodings, overflow, truncation, universal EOC,
and trailing data in exact mode. It does not interpret ASN.1 types, recurse
through constructed values, parse X.509, validate certificate paths, verify
signatures, or select trust roots, so it does not unblock concrete HTTPS by
itself. The typed `der-asn1` layer is now shipped above that framing primitive:
it provides allocation-free canonical BOOLEAN, INTEGER, ENUMERATED, BIT STRING,
OCTET STRING, NULL, and OBJECT IDENTIFIER decoding plus exact SEQUENCE, SET, and
explicit context-wrapper traversal under one shared depth and total-element
budget. It also validates universal IA5String and schema-selected implicit NULL,
BIT STRING, OCTET STRING, IA5String, and OBJECT IDENTIFIER values under exact
primitive context-specific tags, providing typed value prerequisites for
later X.509 schemas without claiming their semantics. The implicit NULL helper
requires the exact caller-selected primitive context-specific tag and empty
contents; it does not select a schema tag or grant CertStatus, OCSP, revocation,
freshness, or authentication semantics. The generic ENUMERATED
decoder requires the exact primitive universal tag and canonical non-empty
two's-complement contents, preserves signed values as borrowed bytes, and
offers checked conversion to `u64`; it does not define an enum registry or
grant CRL, OCSP, revocation, or freshness semantics. A separate
`x509-time` profile layer now validates exact RFC 5280
UTCTime and GeneralizedTime tags, fixed Zulu forms, the 1950/2050 century
split, and Gregorian fields without a clock or platform calendar. A separate
tag-strict entry point in that same repository-owned layer validates the RFC
5280 GeneralizedTime wire form used by RFC 6960 OCSP fields without applying
the certificate-validity rule that reserves GeneralizedTime for 2050 and
later. It accepts only the fixed `YYYYMMDDHHMMSSZ` form and returns the same
clock-free calendar value; it does not parse an OCSP container, read a clock,
or decide freshness. A bounded
`x509-validity` layer is now shipped above them: it decodes the exact two-field
`Validity` sequence, rejects inverted endpoints, and classifies an explicitly
supplied validated time against inclusive endpoints. These layers still
provide no certificate-wide schema, clock authority, name semantics, signature
verification, path construction, revocation, or trust-root source, so concrete
HTTPS remains blocked. A bounded `x509-algorithm-identifier` prerequisite is
now shipped: it decodes a generic validated algorithm OID plus one optional
opaque canonical DER parameter element. It intentionally adds no algorithm
registry, parameter policy, key access, signing, or verification.
A bounded `x509-subject-public-key-info` composition is now shipped above that
syntax: it decodes the exact two-field algorithm-and-key-bit-string container
under the same shared budgets. It deliberately does not recognize algorithms,
interpret public keys, or grant key, signing, or verification authority.
A bounded `x509-extension` prerequisite now decodes one exact generic extension
OID, canonical omitted-default critical flag, and opaque OCTET STRING under the
same shared budgets. It does not recognize extension OIDs, parse encapsulated
values, process unknown critical extensions, or decode an extension list.
A bounded `x509-extensions` composition now decodes the non-empty certificate
extension sequence into at most 64 allocation-free borrowed values, preserves
wire order, and rejects duplicate extension OIDs under the same shared budgets.
It still does not recognize OIDs, parse encapsulated values, or decide whether
an unknown critical extension is supported.
A bounded `x509-basic-constraints` semantic layer now recognizes only the
RFC 5280 Basic Constraints OID and decodes its exact encapsulated sequence. It
enforces canonical omission of the default false CA flag, non-negative path
lengths, and the rule that a path length cannot appear unless CA is true while
preserving arbitrarily large canonical integers as borrowed bytes. It does not
grant CA authority, enforce certificate-wide criticality or Key Usage policy,
build paths, verify signatures, or select trust roots.
A bounded `x509-key-usage` semantic layer now recognizes only the RFC 5280 Key
Usage OID and decodes its exact encapsulated BIT STRING. It requires at least
one asserted usage, exposes only the nine defined flags, and rejects undefined
or non-minimal named-bit encodings. It does not authorize a key operation,
assign a certificate role, enforce outer criticality, or combine the result
with Basic Constraints or Extended Key Usage policy.
A bounded `x509-extended-key-usage` semantic layer now recognizes only the RFC
5280 Extended Key Usage OID and decodes its exact non-empty sequence under a
fixed 64-purpose bound. It preserves arbitrary canonical purpose OIDs in wire
order without embedding a purpose registry. It does not authorize a purpose,
assign a certificate role, enforce outer criticality, or intersect the result
with Key Usage or Basic Constraints policy.
A bounded `x509-subject-alt-name` semantic layer now recognizes only the RFC
5280 Subject Alternative Name OID and decodes at most 256 non-empty GeneralName
envelopes. It retains allocation-free DNS-ID strings and typed IPv4/IPv6 IP-ID
values for the separate server-identity matcher, validates every primitive
choice, and keeps constructed choices bounded and opaque for later schema
packages. It does not validate DNS syntax, establish identity, interpret
constructed names, parse a certificate, build paths, verify signatures, or
select trust roots.
A bounded `x509-serial-number` semantic layer now decodes one canonical RFC
5280 certificate serial-number INTEGER. It requires a positive nonzero value,
enforces the 20-content-octet ceiling, and retains only borrowed canonical and
normalized magnitude views. It does not generate values, prove per-issuer
uniqueness, parse a complete certificate, validate paths, or verify signatures.
A bounded `x509-certificate-version` semantic layer now decodes the optional
constructed `[0] EXPLICIT Version` field under shared depth and element limits.
It exposes omitted `v1` as the caller's default, rejects an explicitly encoded
DER default, and accepts only explicit `v2` or `v3`. It does not parse the
surrounding certificate or enforce version-dependent field policy.
A bounded `x509-name` semantic layer now decodes exact RFC 5280 RDN sequences,
non-empty DER-ordered RDN sets, and OID-plus-opaque-value attributes under
shared limits. It retains exact borrowed values and RDN boundaries without
claiming DirectoryString normalization, semantic name equality, issuer/subject
relationship decisions, or path validation.
A bounded `x509-name-comparison` semantic layer now compares two decoded RDN
sequences in order, matches each RDN's attributes without depending on their
DER SET order, requires exact canonical attribute OIDs, and delegates only the
already framed attribute-value comparison to caller policy. The callback must
be a stable equivalence relation and may run more than once for repeated
attribute types. This structure is not DirectoryString decoding,
normalization, complete RFC 5280 name equality, issuer selection, or path
validation.
A bounded `x509-directory-string` value layer now recognizes all five
primitive, non-empty RFC 5280 DirectoryString alternatives. It validates
UTF8String, PrintableString, UniversalString, and BMPString into
allocation-free Unicode-scalar views while retaining TeletexString as opaque
bytes, and it can compare exact scalar sequences across the four supported
representations. It refuses Teletex comparison rather than inventing a T.61
mapping. It performs no RFC 4518 StringPrep, case folding, normalization,
insignificant-space handling, or attribute-OID schema selection, so it is not
yet a complete value policy for issuer linking or path validation.
A bounded `x509-certificate` structural composition now decodes complete RFC
5280 `Certificate` and `TBSCertificate` values under one shared DER budget. It
retains the exact signed bytes, composes the shipped field decoders, validates
the required non-empty issuer and canonical implicit unique identifiers,
enforces optional-field ordering and version constraints, and requires exact
inner/outer signature-algorithm encoding agreement. It does not recognize or
enable algorithms, verify signatures, interpret extension policy, compare
names, validate paths or
hostnames, check revocation, select trust roots, perform TLS, read a clock, or
open a network connection.
A bounded `x509-certificate-policy` gate now enforces the certificate-wide
unknown-critical-extension rule over that complete decoded structure. It calls
a caller-supplied, data-driven OID recognizer exactly once for each critical
extension in wire order, accepts unrecognized non-critical extensions, and
fails closed with a redacted sequence index at the first unsupported critical
extension. Recognition does not decode or validate the extension value, grant
a certificate role, or establish trust. A second bounded gate now composes the
shipped Basic Constraints, Key Usage, and Extended Key Usage decoders under one
explicit semantic budget. It requires critical `cA = TRUE` plus
`keyCertSign` for an intended issuing CA, rejects CA or certificate-signing
assertions for an intended end entity, and returns only borrowed decoded role
evidence. It deliberately leaves key-usage selection, EKU purpose
authorization, signatures, names, paths, trust, clocks, revocation, and
transport to later policy layers. A third bounded gate selects exactly one
caller-supplied signature/key AlgorithmIdentifier pair, including exact
parameter presence or DER bytes, then checks inclusive encoded public-key and
certificate-signature BIT STRING length ranges. The public-key bound is a
wire-size policy, not algorithm-specific strength such as an RSA modulus size
or EC group. It embeds no algorithm registry, parses no key or signature,
chooses no verifier, and grants no cryptographic authority. Repository-owned
key parsing and strength validation remain separate. A fourth bounded gate now
requires and decodes the complete certificate's Subject Alternative Name under
an explicit caller budget, then applies the shipped RFC 9525 matcher to only
typed DNS-ID and IP-ID values. It has no Common Name fallback and returns only
the matched kind plus the extension's wire index. This proves only a name match
for one caller-selected reference; it does not authenticate the certificate or
replace critical-extension policy. A fifth bounded gate now classifies the
already decoded inclusive certificate validity interval at one explicit
validated caller-owned observation. It distinguishes not-yet-valid from
expired without reading an ambient clock, applying tolerance, or claiming the
observation's source or freshness; clock acquisition, conversion, and audit
remain separate caller authority. A sixth bounded gate now authorizes one
caller-supplied non-empty Extended Key Usage purpose over already validated
end-entity role evidence. It fails closed when the role is not an end entity,
the certificate omitted Extended Key Usage, or that extension does not contain
the exact requested OID. It embeds no purpose registry and gives
`anyExtendedKeyUsage` no implicit wildcard meaning: callers that choose that
OID request only that exact OID. Its opaque success evidence grants no key
operation, signature, path, identity, time, trust, revocation, TLS, or network
authority. A seventh bounded gate now fixes one certificate-link direction:
it invokes one caller-supplied distinguished-name comparator exactly once with
the child issuer Name followed by the candidate issuer subject Name. Mismatch
fails closed with no Name or certificate bytes in the error; opaque success
proves only that this comparator accepted this pair. The gate embeds no name
normalization or equality rule, does not grant the candidate a CA role, and
neither constructs nor validates a path. Repository-owned signature
verification, RFC 4518 and schema-specific name value comparison,
certificate path construction and validation, trust-root integration,
revocation policy, TLS, and HTTP remain blockers, so concrete HTTPS remains
unavailable.
A reviewed trust-root data prerequisite is now shipped as
`x509-webpki-roots-snapshot`: the exact `webpki-roots` 1.0.9 Mozilla
public-Web-PKI anchor set is pinned by upstream tag and commit, crates.io
archive checksum, root count, and canonical root-set digest. Only generated
anchor bytes enter production; neither `webpki-roots` nor its
`rustls-pki-types` dependency is compiled or linked. Repository-owned code
parses the allocation-free snapshot, and independent CI verification rejects
unreviewed byte, order, count, format, or provenance drift. The static set
excludes OS, enterprise, user-installed, and local roots and carries an
explicit update cadence. It supplies data only: certificate parsing, path and
hostname validation, signature verification, constraint and algorithm policy,
revocation, TLS, HTTP, and platform trust remain separate blockers.
A bounded `x509-webpki-root-anchor-selection` composition now selects one
decoded candidate certificate against only that reviewed snapshot. It requires
exact canonical encoded Name and SubjectPublicKeyInfo contents, scans the
complete set, and fails closed when no anchor or more than one anchor matches.
Its
non-copyable evidence binds the exact candidate certificate to the selected
snapshot entry and retains that entry's optional name constraints for later
path policy. Selection does not validate those constraints, grant the
candidate a certificate role, verify any signature, check validity, construct
or validate a path, check revocation, or provide hostname validation, TLS,
HTTP, OAuth transport, or network authority.
A bounded algorithm-specific prerequisite is now shipped as
`x509-rsa-public-key`. It requires the exact RFC 3279 `rsaEncryption`
SubjectPublicKeyInfo form, decodes its nested PKCS #1 public modulus and
exponent without allocation, and enforces caller-supplied inclusive
modulus-bit and exponent-allowlist policy. It adds no modular arithmetic,
signature verification, general algorithm registry, or cryptographic
authority.
A separate bounded `rsa-pkcs1-v1-5-sha256` prerequisite now validates the
complete modulus-width encoded message produced by a future RSA public
operation. It accepts only the exact RFC 8017 `00 01 || PS || 00 ||
DigestInfo` form, at least eight `ff` padding bytes, the canonical SHA-256 DER
trailer with present NULL parameters, and the complete caller-supplied digest.
It performs no RSA arithmetic or hashing, and arbitrary caller-supplied bytes
do not become an authenticated signature by passing this formatting gate.
A sibling bounded `rsa-pkcs1-v1-5-sha384` prerequisite applies that same exact
RFC 8017 encoded-message boundary to SHA-384. It accepts only the complete
modulus-width `00 01 || PS || 00 || DigestInfo` value, at least eight `ff`
padding bytes, the canonical `id-sha384` DER trailer with present NULL
parameters, and all 48 expected digest octets. Trailer-derived boundaries
prevent separator searching or padded/truncated acceptance. Stable failures
retain no encoded-message bytes, digest bytes, mismatch position, or padding
length. The allocation-free gate performs no hashing, RSA arithmetic, key or
certificate parsing, algorithm routing, signature verdict, path validation,
trust, revocation, TLS, HTTP, OAuth transport, or external effect. A later
repository-owned X.509 composition must still bind it to an exact signed
certificate, validated issuer key, RSA public operation, SHA-384 digest, and
data-driven algorithm policy; concrete HTTPS remains unavailable.
A sibling bounded `rsa-pkcs1-v1-5-sha512` prerequisite applies that same exact
RFC 8017 encoded-message boundary to SHA-512. It accepts only the complete
modulus-width `00 01 || PS || 00 || DigestInfo` value, at least eight `ff`
padding bytes, the canonical `id-sha512` DER trailer with present NULL
parameters, and all 64 expected digest octets. Trailer-derived boundaries
prevent separator searching or padded/truncated acceptance. Stable failures
retain no encoded-message bytes, digest bytes, mismatch position, or padding
length. The allocation-free gate performs no hashing, RSA arithmetic, key or
certificate parsing, algorithm routing, signature verdict, path validation,
trust, revocation, TLS, HTTP, OAuth transport, or external effect. Any
signature-verdict composition must bind it to authenticated certificate inputs
instead of treating this formatting result as standalone proof.
A bounded `rsa-pss-mgf1-sha256` prerequisite now implements only the RFC 8017
MGF1 expansion required by the future SHA-256 EMSA-PSS verification path. It
accepts the exact 32-byte PSS hash value as its seed, validates both the
standard's `2^32 * 32` maximum and a smaller caller-selected nonzero work
ceiling before changing caller-owned output, encodes each counter as four
network-order octets, and fills the requested mask directly without allocation.
The repository-owned SHA-256 implementation is its only production dependency.
This primitive does not parse parameters or keys, perform RSA arithmetic,
validate an EMSA-PSS encoded message, choose a salt policy, verify a signature,
process a certificate, or grant path, trust, revocation, TLS, HTTP, OAuth
transport, or network authority. RSA-PSS support remains unavailable until
those separate boundaries are implemented, composed, and tested.
A bounded `rsa-public-operation` prerequisite now consumes only a previously
validated RSA public key, enforces a separate caller-owned modulus-byte ceiling
before arbitrary-precision conversion, rejects non-modulus-width or
out-of-range signature representatives, and recovers the exact modulus-width
RSAVP1 encoded message with repository-owned arithmetic. It adds no private-key
operation, hashing, padding interpretation, certificate-signature verdict,
path trust, or transport readiness; it is not by itself a verifier.
A bounded, data-driven `rsa-pss-encoded-message-layout` prerequisite now
validates the public outer layout consumed by a future RFC 8017 EMSA-PSS
verifier. Caller policy supplies the selected hash length, salt length, and an
encoded-message byte ceiling. The gate derives `emLen` from the exact public
`emBits`, requires the complete recovered message at that width, proves the
selected parameters fit, requires trailer byte `0xbc`, rejects forbidden
high bits in `maskedDB`, and returns borrowed non-copyable slices for the exact
masked data block and `H`. All size arithmetic is checked before slicing.

This layout evidence does not unmask or validate `DB`, run MGF1, hash the
message or `M'`, compare `H`, parse or operate on an RSA key, apply RSAVP1,
select an algorithm, or establish a signature verdict. Certificate processing,
path and hostname validation, trust roots, revocation, TLS, HTTP, OAuth
transport, and network effects remain unavailable.
A bounded `x509-rsa-pkcs1-v1-5-sha256-signature` composition now verifies one
exact certificate signature with repository-owned primitives. It requires the
canonical `sha256WithRSAEncryption` identifier with present NULL parameters,
an already policy-validated issuer RSA key, an explicit modulus-byte ceiling,
and a byte-aligned signature; it hashes the preserved `TBSCertificate` DER,
applies RSAVP1, and validates the complete canonical SHA-256 encoded message.
Its non-copyable borrowed result binds the exact certificate and validated
issuer key and proves only that the key verified those exact signed bytes.
Issuer selection and linkage, algorithm routing, certificate
roles, path construction and validation, trust roots, time, names, revocation,
TLS, HTTP, and network effects remain separate blockers.
A bounded sibling `x509-rsa-pkcs1-v1-5-sha512-signature` composition now binds
the SHA-512 encoded-message prerequisite to one exact decoded certificate, an
already policy-validated issuer RSA key, the bounded repository-owned RSA
public operation, and the repository-owned SHA-512 implementation. It requires
the canonical `sha512WithRSAEncryption` identifier with present NULL
parameters and a byte-aligned signature, hashes the preserved
`TBSCertificate` DER, and returns non-copyable evidence retaining the exact
certificate and issuer key only after the complete canonical encoded message
passes. The pure public-data verifier performs no credential access, signing,
storage, clock acquisition, or external effect, so it emits no runtime audit
event. Issuer selection and linkage, data-driven algorithm routing,
certificate roles, path construction and validation, trust roots, time,
names, revocation, TLS, HTTP, and network effects remain separate blockers;
concrete HTTPS remains unavailable.
A bounded `x509-rsa-sha256-issuer-link` composition now authenticates one
directed child-to-candidate-issuer link for that exact algorithm. Caller-owned
Name policy is invoked once in the fixed child-issuer to candidate-subject
direction; the exact candidate must pass issuing-CA role checks and
caller-supplied RSA key-strength policy before its key verifies the child's
preserved signed bytes. Non-copyable evidence binds the exact child, candidate
issuer, and validated key. The composition does not process other critical
extensions, validity, path length, name constraints, trust anchors, path
construction, revocation, TLS, HTTP, or network effects, so concrete HTTPS
remains unavailable.
A bounded `x509-certificate-path-length-policy` prerequisite now applies the
RFC 5280 path-length state transition to caller-ordered intermediate-CA role
evidence. The caller supplies the initial maximum, identifies whether each
intermediate certificate is self-issued, and advances from the trust-anchor
side toward the target. Non-self-issued intermediates consume one unit;
self-issued rollover certificates do not. Each certificate's decoded
`pathLenConstraint` can only tighten the remaining maximum, including exact
values too large for the local counter, which cannot loosen a tighter current
limit. Exhaustion fails closed with a payload-free category. This gate neither
decides self-issuance nor orders, links, authenticates, or trusts certificates;
the target certificate and trust anchor are outside this transition. Path
construction, complete path validation, trust-anchor constraints, name
constraints, revocation, TLS, HTTP, and network effects remain separate
blockers, so concrete HTTPS remains unavailable.
A bounded evidence-binding follow-up now invokes one caller-supplied Name
comparator exactly once with an exact certificate's issuer followed by its
subject and retains a non-copyable reference to that certificate with the
self-issued result. The path-length package can consume that evidence, derive
the issuing-CA role check from the same certificate, and then apply the state
transition, preventing accidental cross-certificate mixing. The comparator
still owns all RFC 5280 Name normalization and equality semantics; the result
does not prove self-signature, construct a path, or grant trust. Complete Name
comparison, path construction and validation, trust-anchor constraints, name
constraints, revocation, TLS, HTTP, and network effects remain blockers.
A bounded `x509-intermediate-certificate-policy` composition now consumes
certificate-wide issuing-CA role-policy evidence, certificate-bound
caller-selected algorithm-policy evidence, and caller-policy self-issued
classification. It fails closed unless all three non-copyable inputs retain
the same exact decoded certificate object and the role is exactly issuing CA,
then classifies that certificate's inclusive validity interval at one explicit
validated caller-owned observation. Its non-copyable result retains all three
inputs plus the validity result so later path traversal can consume one
non-mixable intermediate-certificate policy boundary. The self-issued result
remains only caller-owned Name classification, and the algorithm profile
remains policy data rather than a verifier. This composition does not
authenticate either adjacent link, advance path-length state, apply name
constraints, construct or validate a certification path, recognize or
constrain a trust anchor, check revocation, perform TLS or HTTP, authorize
OAuth transport, or open a network connection.
The path-length state can now consume that complete intermediate-certificate
policy evidence directly. It derives both issuing-CA role evidence and the
self-issued transition bit from the one retained certificate-bound policy,
then applies the existing decrement and `pathLenConstraint` tightening rules.
This prevents a path traversal from silently discarding the intermediate's
critical-extension recognition, algorithm policy, and explicit-time validity
checks by re-deriving only a detached role/classification pair. The aggregate
state still proves only path-length accounting: it does not retain an
authenticated adjacent link, choose or order certificates, apply name
constraints, recognize trust, check revocation, perform TLS or HTTP, authorize
OAuth transport, or open a network connection.
A bounded authenticated-intermediate binding follow-up now consumes the
strongest selected RSA SHA-256 issuer-link evidence together with complete
intermediate-certificate policy. It fails closed unless the authenticated
link's exact selected candidate issuer is the same decoded certificate object
retained by the intermediate policy, then returns one distinct non-copyable
result retaining both proofs. This prevents later traversal from combining an
authenticated child-to-issuer link with validity, self-issued, algorithm, or
critical-extension policy for a different intermediate. The result proves
only that this policy-approved intermediate authenticated one exact child; it
does not authenticate the intermediate's own issuer link, advance path-length
state, choose or order certificates, construct or validate a complete path,
recognize trust, process name constraints or revocation, perform TLS or HTTP,
authorize OAuth transport, or open a network connection.
A bounded authenticated path-length follow-up now consumes that complete
authenticated-intermediate proof together with one path-length state. It
derives the transition's issuing-CA role, caller-policy self-issued bit, and
decoded `pathLenConstraint` only from the exact intermediate policy retained
by the authenticated link, then returns a distinct non-copyable result that
retains both the authenticated intermediate and the advanced state. This
prevents callers from presenting one authenticated intermediate while
advancing path-length state with policy for another certificate, or from
silently discarding the adjacent-link proof at this composition boundary. The
result covers one caller-ordered transition only: it does not authenticate the
intermediate's own issuer, prove adjacency to another step, choose or order
certificates, construct or validate a complete path, recognize trust, process
name constraints or revocation, perform TLS or HTTP, authorize OAuth
transport, or open a network connection.
A bounded `x509-certificate-issuer-candidate-selection` path-construction
prerequisite now scans one caller-bounded pool of decoded candidate
certificates in caller order. It invokes caller-owned Name policy exactly once
per candidate with the child's issuer followed by that candidate's subject,
rejects zero or multiple matches after a complete accepted-pool scan, and
returns non-copyable evidence binding the exact child, selected candidate, and
candidate index. The selector embeds no Name normalization or equality rule and
does not grant a CA role, authenticate the selected link, recognize a trust
anchor, or validate a path. Complete RFC 4518 and schema-specific Name
comparison, algorithm routing and link authentication, full path construction
and validation, trust-anchor and name constraints, revocation, TLS, HTTP, and
network effects remain blockers.
A bounded `x509-certificate-path-construction` follow-up now repeats that unique
issuer selection into one caller-bounded target-to-terminal chain. Candidate
pool size and requested path depth are rejected before policy callbacks;
caller-owned terminal policy runs exactly once per accepted path certificate;
and exact certificate-encoding cycles or exhaustion before a terminal match
fail closed. Non-copyable success evidence retains the exact target, candidate
pool, and ordered selected indices, so a later validator can consume the one
constructed chain without caller substitution. This deterministic primitive
does not search alternate ambiguous branches, define Name equality, decide
trust, authenticate any link, apply certificate roles, algorithms, validity,
path length, name constraints, hostname or revocation policy, or provide TLS,
HTTP, OAuth transport, or network authority.
A bounded reviewed-terminal binding follow-up now consumes that constructed
path together with reviewed-root selection evidence. It requires the path to
contain at least one selected issuer, derives the exact terminal certificate
from the path's final retained candidate index, and fails closed unless the
reviewed-root evidence retains that same decoded certificate object. Distinct
non-copyable success evidence keeps both complete inputs, preventing a caller
from substituting a byte-equal re-decode or attaching reviewed-root membership
to a different path certificate. The binding does not authenticate any path
link, validate the terminal certificate's role or self-signature, apply anchor
constraints, or provide complete path validation, revocation, TLS, HTTP, OAuth
transport, or network authority.
A bounded constructed-target binding follow-up now consumes that constructed
path together with complete validated server-certificate policy. It fails
closed unless the server-policy evidence retains the same exact decoded target
certificate object as the path, then returns distinct non-copyable evidence
retaining both complete inputs. This prevents callers from attaching hostname,
validity, purpose, end-entity role, critical-extension, or algorithm policy for
another certificate, including a byte-equal re-decode, to the constructed
chain. The binding does not authenticate any path link, validate an issuer or
trust anchor, apply path-length or name constraints, check revocation, or
provide complete path validation, TLS, HTTP, OAuth transport, or network
authority.
A bounded reviewed-endpoints follow-up now consumes the reviewed-terminal-bound
constructed path together with complete validated server-certificate policy.
It fails closed unless that policy retains the same exact decoded target object
as the constructed path, then returns one distinct non-copyable result retaining
the reviewed terminal, constructed candidate pool and indices, and complete
target policy. This prevents the two endpoint proofs from requiring separate
consumption of the non-copyable path or allowing either endpoint to be silently
discarded. The composition does not authenticate path links, apply issuer,
anchor, path-length, or name-constraint policy, check revocation, or provide
complete path validation, TLS, HTTP, OAuth transport, or network authority.
A bounded adjacent-link projection now borrows one exact constructed path and
one selected-issuer depth below its retained issuer count. Depth zero derives
the target as child; every later depth derives the preceding selected candidate
as child; and the current selected candidate is always the issuer. Out-of-range
depths return no evidence. The non-copyable projection retains the construction
borrow and derives both endpoints from its private depth, so later signature
verification can consume an exact ordered pair without rerunning Name policy or
accepting caller-substituted certificates. Projection alone does not authenticate
a signature, validate either certificate, route an algorithm, advance path-length
state, apply constraints or revocation, establish trust, or provide complete path
validation, TLS, HTTP, OAuth transport, or network authority.
The RSA SHA-256 issuer-link composition can now consume non-copyable unique
candidate-selection evidence directly. It derives the exact child and selected
candidate from that evidence, then performs issuing-CA role, caller-owned RSA
key-policy, and exact child-signature checks without a second Name callback.
This closes a cross-certificate mixing gap between one-step selection and
authentication for the one implemented algorithm, but does not add algorithm
routing, construct or validate a complete path, recognize trust, process
constraints or revocation, or provide TLS, HTTP, OAuth transport, or network
authority.
A bounded constructed-path authentication follow-up now consumes one adjacent-
link projection from the exact retained path. It authenticates only the
projection's derived target-or-candidate child against its derived selected
issuer with the existing fixed RSA PKCS #1 v1.5 SHA-256 verifier, without
rerunning Name policy, then returns distinct non-copyable evidence retaining
both the construction projection and validated issuer key. This prevents path
order from being discarded or either endpoint from being substituted between
construction and authentication. It still does not route algorithms, retain
complete issuer policy, authenticate any other link, validate the whole path,
recognize or constrain trust, process name constraints or revocation, perform
TLS or HTTP, authorize OAuth transport, or open a network connection.
A bounded complete-intermediate-policy follow-up now strengthens that
construction-bound authentication without accepting either endpoint
separately. It consumes one constructed-path adjacency together with complete
intermediate policy for the exact projected non-terminal issuer, rejects the
construction's terminal adjacency and any certificate-object mismatch before
RSA key parsing or signature verification, and then retains
both the authenticated construction projection and the complete intermediate
policy in distinct non-copyable evidence. This binds caller-owned critical-
extension recognition, issuing-CA role, data-driven algorithm profile,
self-issued classification, and explicit-time validity to the same ordered
issuer that authenticated the projected child. It does not advance path-length
state, authenticate another link, route algorithms, validate the whole path,
recognize or constrain trust, process name constraints or revocation, perform
TLS or HTTP, authorize OAuth transport, or open a network connection.
A bounded construction-bound path-length follow-up now consumes that exact
non-terminal intermediate proof together with one caller-supplied path-length
state. It derives issuing-CA role, self-issued classification, and any decoded
`pathLenConstraint` only from the complete intermediate policy retained by the
ordered proof, then returns distinct non-copyable evidence retaining both the
construction-bound authenticated link and the advanced state. Terminal
certificates cannot enter this transition because the consumed proof already
rejects the construction's final adjacency. The result covers one retained
non-terminal adjacency only: it does not prove that the input state represents
earlier construction depths, authenticate another link, route algorithms,
validate the whole path, recognize or constrain trust, process name constraints
or revocation, perform TLS or HTTP, authorize OAuth transport, or open a network
connection.
A bounded construction-bound reviewed-path follow-up now consumes that
path-length result, the authenticated terminal adjacency projected from the
same construction, and reviewed-root selection for the terminal issuer. It
accepts only an exact two-link target-to-intermediate-to-terminal construction:
the first retained adjacency must be depth zero, the terminal adjacency must be
depth one, both must retain the identical constructed-path object, and the
terminal issuer must be the exact decoded certificate retained by the reviewed
root evidence. Distinct non-copyable success evidence keeps all three inputs,
including complete intermediate policy and the advanced path-length state, so
callers cannot splice authenticated links from separately constructed paths or
discard the reviewed terminal. The caller-supplied initial path-length state
still is not derived from anchor policy, and this fixed-shape composition does
not apply anchor constraints, target policy, revocation, generalized path
validation, TLS, HTTP, OAuth transport, or network authority.
A bounded construction-bound target-policy follow-up now consumes that complete
authenticated reviewed path together with complete validated server-certificate
policy. It fails closed unless the server-policy certificate is the same exact
decoded target object retained by the constructed path, then returns distinct
non-copyable evidence retaining both complete inputs. This prevents callers
from attaching hostname, validity, purpose, end-entity role, critical-extension,
and algorithm policy for one certificate to a separately constructed and
authenticated target, including a byte-equal re-decode. The result still uses
the caller-supplied initial path-length state retained by the prior proof and
does not apply anchor or intermediate name constraints, other anchor policy,
revocation, generalized path validation, TLS, HTTP, OAuth transport, or network
authority.
A bounded construction-bound anchor-policy follow-up now consumes that exact
target-policy result and applies the selected reviewed terminal's exact
supported DNS constraints to the exact target certificate retained by the
construction. Entries without out-of-band name constraints retain the complete
construction-bound proof without DNS evidence. Constrained entries reuse the
repository-owned bounded NameConstraints and Subject Alternative Name decoders,
require every target dNSName to satisfy at least one permitted DNS subtree, and
retain optional payload-free count evidence. Callers cannot substitute another
constructed path, reviewed terminal, target certificate, or SAN collection.
The result still retains caller-supplied initial path-length state and does not
apply intermediate name constraints, excluded or non-DNS anchor constraints,
other anchor policy, revocation, generalized path validation, TLS, HTTP, OAuth
transport, or network authority.
A bounded construction-bound intermediate-policy follow-up now consumes that
complete anchor-policy result and applies the exact retained intermediate's
NameConstraints extension to the exact retained target. An absent extension
retains the complete prior proof without new DNS evidence. A present extension
must be critical and must decode under caller-owned limits as the supported
permitted-and-excluded-dNSName subset; every target dNSName must satisfy the
permitted policy, excluded names take precedence, and wildcard targets that
overlap an excluded subtree fail closed. The target, intermediate, constructed
path, reviewed terminal, and SAN collection are all derived through the one
retained proof, so callers cannot splice policy evidence from another path or
certificate. This fixed two-link boundary still retains caller-supplied initial
path-length state and does not apply excluded or non-DNS anchor constraints,
other anchor policy, revocation, generalized path validation, TLS, HTTP, OAuth
transport, or network authority.
Certificate-wide role policy can now bind caller-owned critical-extension
recognition and algorithm-independent role semantics to the same exact decoded
certificate. The non-copyable result prevents checking critical extensions on
one certificate while embedding role evidence from another, and unknown
critical extensions fail before role acceptance. Binding-sensitive downstream
APIs must consume that outer result rather than detached lower-level role
evidence. Recognition still is not semantic validation of other critical
extensions, and this composition adds no algorithm routing, signature,
complete path, trust, time, identity, revocation, TLS, HTTP, OAuth transport,
or network authority.
Selected RSA SHA-256 issuer authentication can now consume that outer issuing-CA
role-policy evidence alongside the non-copyable candidate-selection evidence.
It fails before key or signature work unless both inputs retain the same exact
candidate certificate and the role-policy evidence was built for the issuing-CA
role. A distinct non-copyable result retains both proofs so later consumers can
require this stronger composition without confusing it with the earlier narrow
issuer-link result. This does not turn recognition into semantic validation of
other critical extensions or add complete path construction, constraints,
trust, time, revocation, TLS, HTTP, OAuth transport, or network authority.
A bounded algorithm-policy evidence follow-up now binds the existing exact,
caller-supplied certificate algorithm-profile result to the same decoded
certificate object that was checked. A stronger selected RSA SHA-256 issuer-link
entry point consumes that certificate-bound algorithm evidence together with
the issuing-CA role-policy and candidate-selection evidence. It rejects any
cross-certificate or non-CA combination before RSA key parsing or signature
verification, then retains all three proofs in one distinct non-copyable result.
The algorithm profile remains policy data only: it neither chooses nor enables
the hard-coded RSA SHA-256 verifier, and its encoded public-key length is not an
RSA modulus-strength check. This composition still adds no general algorithm
routing, complete path construction or validation, trust, time, identity,
constraints, revocation, TLS, HTTP, OAuth transport, or network authority.
A bounded reviewed-anchor binding follow-up now consumes that strongest RSA
SHA-256 issuer-link evidence together with reviewed-root selection evidence.
It fails closed unless both non-copyable inputs retain the same exact issuer
certificate, then retains both proofs in one distinct non-copyable result. This
proves only that one authenticated child-to-issuer link terminates at a
certificate identity occurring exactly once in the reviewed Mozilla snapshot.
It does not apply the selected anchor's name constraints, validate the target's
role, purpose, validity, or identity, construct a multi-hop path, apply path
length or other constraints, check revocation, or provide TLS, HTTP, OAuth
transport, or network authority.
A bounded `x509-server-certificate-policy` composition now consumes
certificate-wide end-entity role-policy evidence and certificate-bound,
caller-selected algorithm-policy evidence, rejecting them unless they retain
the same exact decoded target certificate and the role is exactly end entity.
Only then does it apply the existing SAN-only server-identity matcher, inclusive
validity classifier at one explicit validated observation, and one explicit
non-empty Extended Key Usage purpose policy to that same certificate. Its
non-copyable result retains the two certificate-bound inputs plus the identity,
validity, and purpose results so later path code cannot silently recombine
detached evidence from different decoded certificate objects. The caller still
owns critical-extension recognition, algorithm profiles, reference identity,
time authority, and requested purpose. This target-only policy composition does
not authenticate an issuer link, recognize or constrain a trust anchor,
construct or validate a certification path, check revocation, perform TLS or
HTTP, authorize OAuth transport, or open a network connection.
A bounded `x509-rsa-sha256-reviewed-server-link` composition now consumes that
target server-policy evidence together with one-link reviewed-anchor evidence.
It fails closed unless both non-copyable inputs retain the same exact decoded
child certificate, then retains both proofs in one distinct non-copyable
result. This binds the target's identity, validity, purpose, role, critical-
extension recognition, and algorithm policy to one authenticated RSA SHA-256
issuer link terminating at a selected reviewed Mozilla root. It still does not
apply the root's name constraints, path-length or other anchor constraints,
construct a multi-hop path, check revocation, perform TLS or HTTP, authorize
OAuth transport, or open a network connection.
A bounded fail-closed anchor-policy follow-up consumes that complete direct
reviewed-server-link evidence and inspects the selected reviewed snapshot entry.
For an unconstrained entry it retains the consumed link directly. For an entry
carrying out-of-band name constraints, it decodes those exact selected-entry
bytes, re-decodes Subject Alternative Name from the exact target certificate
retained by the server-policy evidence, and requires every target dNSName to
satisfy the supported permitted DNS subtrees. Separate caller-owned DER limits
bound both parses, and the non-copyable result retains the link plus optional
payload-free DNS-constraint evidence. Malformed or unsupported constraint
bytes, malformed target SAN, or a constraint violation fail closed. This
supports only the exact direct target-to-reviewed-root link and adds no excluded
or non-DNS constraint forms, multi-hop path construction, revocation, TLS,
HTTP, OAuth transport, or network authority.
A smaller repository-owned DER prerequisite for that future parser now opens an
exact context-specific constructed value selected by its schema tag as one
bounded child cursor. The cursor reuses the same depth, total-element, and DER
framing limits as SEQUENCE, SET, and explicit-wrapper traversal, while rejecting
the wrong class, primitive form, or tag before exposing children. This is only
the missing IMPLICIT-constructed traversal primitive required by the
`permittedSubtrees [0]` and `excludedSubtrees [1]` NameConstraints fields; it
does not recognize either field, decode GeneralSubtree, apply a constraint, or
grant certificate-path, trust, TLS, HTTP, OAuth, or network authority.
A bounded repository-owned NameConstraints structural decoder first consumed
the out-of-band SEQUENCE contents carried by reviewed trust anchors. Its first
fail-closed subset accepts one `permittedSubtrees [0]` field containing one or
more dNSName GeneralSubtree sequences with absent distance fields, retaining
the non-empty ASCII subtree strings in fixed capacity and wire order. The
decoder is data-driven and covers the reviewed `.tr` constraint without
hard-coding that suffix. That initial slice did not validate DNS syntax,
support excluded or non-DNS subtrees, evaluate a certificate name, bind the
parsed bytes back to
selected-anchor evidence, construct or validate a path, grant trust, check
revocation, perform TLS or HTTP, authorize OAuth transport, or open a network
connection.
A bounded structural follow-up now also retains exact RFC 5280 iPAddress
GeneralSubtree values from complete certificate-extension NameConstraints:
8-octet IPv4 address-plus-mask values and 32-octet IPv6 address-plus-mask
values. DNS and IP forms share one fixed 256-entry ceiling in each permitted
or excluded field and are retained in wire order within their form. The
reviewed-root snapshot-contents entry point remains restricted to its reviewed
permitted-DNS-only format, and its DNS-only evaluator fails closed if either IP
set is non-empty so existing consumers cannot silently ignore the new form.
This structural decoder does not validate CIDR mask
contiguity, evaluate a presented IP address, bind constraints to a certificate
or path, grant trust, check revocation, verify a signature, perform TLS or
HTTP, authorize OAuth transport, or cause an external effect; concrete HTTPS
remains unavailable.
A separate bounded evaluator initially validated every decoded permitted DNS
subtree before matching caller-supplied DNS names. It enforces strict ASCII labels,
case-insensitive label-boundary matching, the reviewed leading-dot semantics,
and a complete-leftmost-label-only presented wildcard, then requires every
supplied name to match at least one data-driven subtree. Both collections are
capped at 256 and evaluation allocates nothing. The result retained only the
evaluated count: it was not bound to certificate SAN evidence or the selected
anchor and did not yet handle excluded or non-DNS subtrees, construct or validate a
path, check trust or revocation, perform TLS or HTTP, authorize OAuth transport,
or open a network connection. Concrete HTTPS therefore remains unavailable.
The direct reviewed-server anchor-policy composition is the sole consumer in
this slice that binds that evaluator to certificate and anchor evidence. It
does not make the standalone decoder or evaluator authoritative, and complete
path validation, revocation, TLS, HTTP, and OAuth transport remain unavailable.
A bounded two-link evidence-binding follow-up now consumes one authenticated
child-to-intermediate path-length transition together with one authenticated
intermediate-to-reviewed-root link. It fails closed unless the complete
intermediate policy retained by the first proof and the child certificate
retained by the second proof refer to the same exact decoded certificate
object, then returns distinct non-copyable evidence retaining both inputs. This
prevents traversal from joining independently authenticated links through
byte-equal but separately decoded intermediates or silently discarding the
advanced path-length state. The result covers exactly one caller-ordered
child-to-intermediate-to-reviewed-root chain for the fixed RSA SHA-256 path;
it does not search or generalize paths, apply anchor name constraints, check
revocation, perform TLS or HTTP, authorize OAuth transport, or open a network
connection.
A bounded target-policy follow-up now consumes complete validated server
certificate policy together with that reviewed two-link path. It fails closed
unless the server-policy certificate and the authenticated path's target child
refer to the same exact decoded certificate object, then returns distinct
non-copyable evidence retaining both complete inputs. This prevents callers
from attaching hostname, validity, purpose, end-entity role, critical-extension,
and algorithm policy for one certificate to a separately authenticated target,
including a byte-equal re-decode. The result does not apply the selected
anchor's name constraints or other anchor policy, search or generalize paths,
check revocation, perform TLS or HTTP, authorize OAuth transport, or open a
network connection.
A bounded anchor-policy follow-up now consumes that complete reviewed two-link
server-path evidence and applies the selected reviewed snapshot entry's exact
supported DNS constraints to the exact target certificate already retained by
the path. Entries without out-of-band name constraints retain the complete path
without DNS evidence. Constrained entries reuse the same separately bounded,
repository-owned NameConstraints and Subject Alternative Name decoding as the
direct-link composition, require every target dNSName to satisfy at least one
permitted DNS subtree, and retain optional payload-free count evidence. Callers
cannot substitute another root, target certificate, or SAN collection. This
does not add excluded or non-DNS constraint forms, other anchor constraints,
path search or generalized construction, revocation, TLS, HTTP, OAuth transport,
or network authority.
A bounded intermediate-constraint follow-up first consumed that anchor-policy
result and inspected only the exact intermediate certificate already retained by the
authenticated two-link path. An absent NameConstraints extension preserves the
complete prior proof without new DNS evidence. A present extension must be
critical, contain one exact DER NameConstraints SEQUENCE, and decode within
caller-owned limits to the then-supported permitted-DNS-only subset; malformed,
non-critical, excluded, or non-DNS forms failed closed. The
same exact target certificate's separately bounded Subject Alternative Name is
then required, and every dNSName must satisfy at least one permitted subtree.
Success retains the complete anchor-policy proof plus optional payload-free
intermediate constraint evidence, so callers cannot substitute an intermediate,
target, root, or SAN collection. That initial form remained a fixed two-link
policy step and did not implement excluded or non-DNS constraint semantics,
apply other anchor
constraints, search or generalize paths, check revocation, perform TLS or HTTP,
authorize OAuth transport, or grant network authority.
A bounded excluded-DNS follow-up extends the repository-owned complete
NameConstraints parser and shared evaluator without widening the reviewed-root
snapshot format. A certificate extension may carry a non-empty
`permittedSubtrees [0]` field, a non-empty `excludedSubtrees [1]` field, or both
in schema order; the supported GeneralSubtree form remains dNSName with absent
minimum and maximum. The snapshot-contents decoder remains restricted to its
reviewed permitted-DNS-only input. Evaluation validates every retained
permitted and excluded subtree before examining a presented name, rejects a
name matching any excluded subtree, rejects a presented wildcard when any of
its concrete expansions falls within an excluded subtree, and, when permitted
subtrees are present, requires every represented name to stay within that set.
The exact direct and two-link anchor-policy
consumers use the shared evaluator, while the exact intermediate-policy
consumer now accepts and enforces supported critical excluded-DNS constraints
instead of rejecting their field solely by tag. Empty fields, duplicates,
reversed order, distance fields, non-DNS GeneralName choices, malformed DER,
and caller-limit exhaustion continue to fail closed with redacted categories.
This remains fixed-path DNS policy only: it does not implement other name
forms, generalized path construction or validation, revocation, TLS, HTTP,
OAuth transport, or network authority.
A bounded `x509-revocation-policy` prerequisite now evaluates one explicit
caller-supplied certificate-status observation at one explicit validated time.
The status must be `good`; `revoked` and `unknown` fail closed. A successful
observation must carry both `thisUpdate` and `nextUpdate`, those endpoints must
form an ordered inclusive interval, and the caller's observation time must lie
inside it. Missing observations, missing freshness endpoints, future
`thisUpdate` values, and stale `nextUpdate` values fail with stable payload-free
categories. The package reads no clock and accepts no tolerance or unbounded
freshness mode. Its caller-owned observation is policy input only: the package
does not parse or authenticate OCSP responses or CRLs, bind status to a
certificate or constructed path, fetch revocation data, define responder or
distribution-point authorization, implement nonce or cache policy, or grant
path, trust, TLS, HTTP, OAuth transport, or network authority. Concrete HTTPS
therefore remains unavailable until authenticated revocation evidence is
composed with the exact certificate/path boundary and the other remaining TLS
work.
A bounded `x509-ocsp-cert-id` prerequisite now decodes exactly one RFC 6960
`CertID` SEQUENCE within caller-owned DER depth, element, length, and OID-arc
limits. Success retains the generic hash `AlgorithmIdentifier`, exact borrowed
issuer-name and issuer-key hash OCTET STRING contents, and the validated
positive RFC 5280 certificate serial number. The structural layer deliberately
accepts hash lengths according to the selected algorithm's later data-driven
policy rather than embedding an algorithm registry. It does not choose a hash
algorithm, compute or compare issuer hashes, parse a larger OCSP response,
authenticate a response signature, authorize a responder, bind the identifier
to a certificate or constructed path, decide status or freshness, fetch or
cache revocation data, read a clock, or grant path, trust, TLS, HTTP, OAuth
transport, or network authority. Concrete HTTPS remains unavailable.
A bounded `x509-ocsp-cert-status` prerequisite now decodes exactly one RFC
6960 `CertStatus` CHOICE within caller-owned DER depth, element, and length
limits. It accepts the exact primitive empty `good [0]` and `unknown [2]`
forms, or the exact constructed `revoked [1]` form containing one required
OCSP GeneralizedTime and one optional explicit `CRLReason`. The reason registry
is the fixed RFC 5280 set used by RFC 6960; canonical negative, unassigned, and
out-of-range ENUMERATED values fail closed. Success retains only a clock-free
validated revocation time and the optional typed reason. This structural layer
does not parse a `SingleResponse` or larger OCSP response, authenticate a
response signature, authorize a responder, bind status to a `CertID`,
certificate, or constructed path, decide freshness, fetch or cache revocation
data, read a clock, or grant path, trust, TLS, HTTP, OAuth transport, or network
authority. Concrete HTTPS remains unavailable.
A bounded `x509-ocsp-single-response` prerequisite now composes exactly one
RFC 6960 `SingleResponse` SEQUENCE within caller-owned DER depth, element,
length, OID-arc, and extension-count limits. It requires one structural
`CertID`, one structural `CertStatus`, and one OCSP GeneralizedTime
`thisUpdate`, followed in schema order by an optional explicit `nextUpdate [0]`
and optional explicit `singleExtensions [1]`. Present extension collections
reuse the bounded generic RFC 5280 `Extensions` decoder, remain non-empty and
unique by OID, and retain no OCSP extension semantics. The structural layer
does not select or validate the CertID hash algorithm, authenticate a response
signature, authorize a responder, bind the identifier or status to a
certificate or constructed path, interpret extensions, decide freshness,
fetch or cache revocation data, read a clock, or grant path, trust, TLS, HTTP,
OAuth transport, or network authority. Concrete HTTPS remains unavailable.
A bounded `x509-ocsp-response-data` prerequisite now composes exactly one RFC
6960 `ResponseData` SEQUENCE within caller-owned DER depth, element, length,
OID-arc, extension-count, and fixed response-count limits. DER's default `v1`
version is accepted only by omission; a present explicit version is rejected
as either the encoded default or an unsupported value. The responder identifier
must be exactly one explicit `byName [1]` RFC 5280 Name or explicit `byKey [2]`
OCTET STRING, `producedAt` uses the strict OCSP GeneralizedTime profile, and
the non-empty ordered response sequence composes bounded structural
`SingleResponse` values. One optional explicit `responseExtensions [1]` field
may follow and remains a generic, non-empty, unique-OID extension collection.
Success retains the exact canonical DER `ResponseData` bytes that a later
repository-owned verifier must authenticate, without reserialization.
The structural layer does not interpret the responder key hash or extensions,
select or validate CertID hash algorithms, authenticate a response signature,
authorize a responder, bind any response to a certificate or constructed path,
decide freshness or revocation, fetch or cache data, read a clock, or grant
path, trust, TLS, HTTP, OAuth transport, or network authority. Concrete HTTPS
remains unavailable.
A bounded `x509-ocsp-basic-response` prerequisite now composes exactly one RFC
6960 `BasicOCSPResponse` SEQUENCE under a shared caller-owned DER depth,
element, length, OID-arc, extension-count, response-count, and fixed
certificate-count budget. It requires one structural `ResponseData`, one
generic signature `AlgorithmIdentifier`, and one canonical signature BIT
STRING in schema order, followed by an optional explicit `certs [0]` sequence.
Each present certificate is decoded by the complete repository-owned RFC 5280
certificate structure layer through the same enclosing DER decoder; no nested
budget is reset. Success retains the exact complete response encoding, the
exact signed `ResponseData` bytes, the generic algorithm identifier, the
borrowed signature bits, and the ordered bounded certificate values without
reserialization. The structural layer does not recognize or enable a signature
algorithm, verify the signature, authorize or select a responder certificate,
match `ResponderID`, build or validate a certificate path, apply certificate
role or critical-extension policy, decide freshness or revocation, fetch or
cache data, read a clock, or grant path, trust, TLS, HTTP, OAuth transport, or
network authority. Concrete HTTPS remains unavailable.

A bounded `x509-ocsp-response` prerequisite decodes exactly one RFC 6960
`OCSPResponse` SEQUENCE under a shared caller-owned DER depth, element,
length, and OID-arc budget. It recognizes only the assigned
`OCSPResponseStatus` values 0, 1, 2, 3, 5, and 6; rejects negative,
non-canonical, unassigned, and oversized ENUMERATED values; requires a
successful response to carry one terminal `[0] EXPLICIT ResponseBytes`; and
requires every error response to omit that field. `ResponseBytes` retains one
generic response-type OBJECT IDENTIFIER and the borrowed contents of one OCTET
STRING without interpreting either. Success retains the exact complete outer
response encoding without reserialization. An explicit structural dispatch
accepts only the exact `id-pkix-ocsp-basic` OID `1.3.6.1.5.5.7.48.1.1`, frames
the OCTET STRING contents as exactly one canonical DER document, and composes
the repository-owned `BasicOCSPResponse` decoder through the same caller-owned
decoder so no depth, element, length, OID-arc, extension-count,
response-count, or certificate-count authority is reset. An unrecognized OID
is rejected before its payload is parsed. Dispatch failures are payload-free;
their offsets are local to the embedded response body because `ResponseBytes`
exposes that body as an independently framed OCTET STRING. This structural
layer does not select or enable an algorithm, verify a signature, authorize a
responder, bind status to a certificate or constructed path, interpret
extensions, decide freshness or revocation, fetch or cache data, read a clock,
or grant path, trust, TLS, HTTP, OAuth transport, or network authority.
Concrete HTTPS remains unavailable.

A bounded `x509-ocsp-rsa-algorithm-policy` prerequisite selects exactly one
caller-supplied algorithm/key profile for one structurally decoded
`BasicOCSPResponse` and one separately validated RSA public key. Each profile
contains an exact signature-algorithm OID and parameter policy, inclusive RSA
modulus and signature bit-length ranges, and a canonical strictly increasing
list of permitted public exponents. Selection rejects unsupported and
ambiguous identifier/parameter matches before checking the selected profile's
key and signature bounds. Success returns non-copyable evidence retaining the
exact response, validated key, and selected profile index, preventing a
detached policy result from being attached to another response or key. The
policy evaluates only caller policy and public response/key metadata, so it
performs no credential access, edit, signing operation, provider action, audit
or trace emission, I/O, or external effect. It does not select or invoke a
verifier, authenticate a signature, authorize or match a responder, bind
status to a certificate or constructed path, decide freshness or revocation,
establish trust, fetch data, read a clock, or grant TLS, HTTP, OAuth transport,
or network authority. Concrete HTTPS remains unavailable.

A bounded `x509-ocsp-rsa-pkcs1-v1-5-sha256-signature` prerequisite verifies
one structurally decoded `BasicOCSPResponse` with one separately
policy-validated RSA public key. It accepts only the exact
`sha256WithRSAEncryption` algorithm identifier with canonical present NULL
parameters, requires a byte-aligned signature BIT STRING, hashes the preserved
canonical `ResponseData` DER with the repository-owned SHA-256 implementation,
applies the bounded repository-owned RSA public operation, and validates the
complete PKCS #1 v1.5 SHA-256 encoded message. Success returns non-copyable
evidence retaining the exact response and validated public key so neither can
be substituted after verification. The verifier does not select that key,
route algorithms, authorize or match a responder, bind a response or `CertID`
to a certificate or constructed path, apply certificate role or extension
policy, decide freshness or revocation, fetch or cache data, read a clock, or
grant path, trust, TLS, HTTP, OAuth transport, or network authority. Concrete
HTTPS remains unavailable.

A parallel bounded `x509-ocsp-rsa-pkcs1-v1-5-sha384-signature` prerequisite
verifies one structurally decoded `BasicOCSPResponse` with one separately
policy-validated RSA public key. It accepts only the exact
`sha384WithRSAEncryption` algorithm identifier with canonical present NULL
parameters, requires a byte-aligned signature BIT STRING, hashes the preserved
canonical `ResponseData` DER with the repository-owned SHA-384 implementation,
applies the bounded repository-owned RSA public operation, and validates the
complete PKCS #1 v1.5 SHA-384 encoded message. Success returns non-copyable
evidence retaining the exact response and validated public key. The pure
public-data verifier performs no credential access, signing, storage, clock
acquisition, or external effect, so it emits no runtime audit event. It does
not route algorithms, select or authorize the responder key, bind response
status to a certificate or path, decide freshness or revocation, establish
trust, perform TLS or HTTP, or authorize OAuth transport. Concrete HTTPS
remains unavailable.

A parallel bounded `x509-ocsp-rsa-pkcs1-v1-5-sha512-signature` prerequisite
verifies one structurally decoded `BasicOCSPResponse` with one separately
policy-validated RSA public key. It accepts only the exact
`sha512WithRSAEncryption` algorithm identifier with canonical present NULL
parameters, requires a byte-aligned signature BIT STRING, hashes the preserved
canonical `ResponseData` DER with the repository-owned SHA-512 implementation,
applies the bounded repository-owned RSA public operation, and validates the
complete PKCS #1 v1.5 SHA-512 encoded message. Success returns non-copyable
evidence retaining the exact response and validated public key so neither can
be substituted after verification. This fixed-algorithm verifier does not
route algorithms, select the supplied key, authorize or match a responder,
bind a response or `CertID` to a certificate or constructed path, apply
certificate role or extension policy, decide freshness or revocation, fetch or
cache data, read a clock, or grant path, trust, TLS, HTTP, OAuth transport, or
network authority. Concrete HTTPS remains unavailable.

A bounded `x509-ocsp-responder-id-binding` prerequisite now matches one exact
structurally decoded `BasicOCSPResponse` to one exact decoded candidate
responder certificate. For `byName`, it invokes caller-owned Name policy
exactly once with the response's responder Name followed by the candidate
certificate's subject. For `byKey`, it requires the fixed 20-byte RFC 6960
KeyHash form, computes repository-owned SHA-1 over the candidate certificate's
`subjectPublicKey` BIT STRING contents excluding the tag, length, and
unused-bits count octet, and compares the complete digest with the
repository-owned constant-time equality primitive. Success returns
non-copyable evidence retaining the exact response, certificate, and matched
identifier kind so neither input can be substituted after matching. This
binding does not verify the response signature, prove that the certificate's
key signed it, authorize the responder, apply certificate role, EKU,
critical-extension, algorithm, path, or trust policy, bind a `CertID` or status
to a certificate or constructed path, decide freshness or revocation, fetch or
cache data, read a clock, or grant TLS, HTTP, OAuth transport, or network
authority. Concrete HTTPS remains unavailable.

A bounded `x509-ocsp-responder-certificate-policy` prerequisite now consumes
certificate-wide role evidence and data-driven algorithm evidence for one
exact decoded candidate responder certificate. It rejects evidence from
distinct decoded objects, requires the retained certificate to have the
end-entity role, requires any present Key Usage to assert `digitalSignature`
or `contentCommitment`, checks its validity at one explicit caller-owned time,
and requires the fixed RFC 6960 `id-kp-OCSPSigning` Extended Key Usage purpose.
Success returns non-copyable evidence retaining both consumed policy results
and the derived validity and purpose results, so a caller cannot substitute a
different certificate after authorization. This target-certificate policy
does not bind a `ResponderID`, verify or bind a response signature, prove an
issuer relationship or delegated authorization, construct or validate a path,
establish trust, check the responder certificate's revocation, fetch or cache
revocation data, read a clock, perform TLS or HTTP, authorize OAuth transport,
or grant network authority. Concrete HTTPS remains unavailable.

A bounded `x509-ocsp-authenticated-responder` composition now consumes one
exact `ResponderID`-to-certificate binding and complete responder-certificate
policy. It rejects evidence retained from distinct decoded certificate objects
before cryptographic work, derives and validates the RSA public key from that
same policy-retained certificate under caller-owned strength and resource
limits, and uses the repository-owned RSA PKCS #1 v1.5 SHA-256 verifier on the
exact response retained by the identifier binding. Its non-copyable result
retains the identifier, certificate-policy, and response-signature evidence so
the matched response, policy-approved candidate, or verified key cannot be
silently substituted. This authenticates only one policy-approved delegated
responder candidate to one response. It does not prove that the target
certificate's issuer signed or authorized the responder certificate, construct
or validate a responder path, establish trust, check responder-certificate
revocation, bind a `CertID` or revocation decision, fetch or cache data, read a
clock, perform TLS or HTTP, authorize OAuth transport, or grant network
authority. Concrete HTTPS remains unavailable.

A bounded `x509-ocsp-issuer-bound-responder` composition now consumes that
exact authenticated-responder evidence and one exact repository-owned RSA
SHA-256 issuer link for the responder certificate. It rejects evidence whose
responder certificates are distinct decoded objects, including byte-equal
re-decodes, before returning non-copyable evidence retaining both proofs.
Together, those proofs establish that one exact candidate issuer with an
issuing-CA role signed the policy-approved OCSP-signing responder certificate,
whose key signed the exact response. This does not establish that the candidate
issuer issued any particular target certificate, bind a `CertID` or revocation
decision, construct or validate a responder path, establish trust, check the
responder certificate's revocation, fetch or cache data, read a clock, perform
TLS or HTTP, authorize OAuth transport, or grant network authority. Concrete
HTTPS remains unavailable.

A bounded `x509-ocsp-cert-id-binding` prerequisite now binds one exact
structurally decoded `SingleResponse` to one exact target certificate and one
exact candidate issuer certificate. It first requires the target certificate's
canonical serial number to equal the `CertID` serial, then invokes one
caller-owned, data-driven hash policy with the exact `CertID` algorithm and the
checked target certificate's canonical issuer Name DER followed by the
candidate issuer certificate's `subjectPublicKey` BIT STRING contents. The
caller selects and implements the hash algorithm; the binding requires one
public non-zero fixed digest length for both fields and compares each complete
digest with the repository-owned constant-time primitive. Success returns
non-copyable evidence retaining the exact response, target, and issuer so none
can be substituted after binding.
This composition does not authorize the selected hash, prove that the issuer
signed the target, verify or authorize the OCSP responder, bind the response to
a constructed path, interpret extensions or status, decide freshness or
revocation, fetch or cache data, read a clock, or grant TLS, HTTP, OAuth
transport, or network authority. Concrete HTTPS remains unavailable.

A bounded `x509-ocsp-revocation-policy` follow-up now consumes that exact
non-copyable `CertID` binding and one explicit validated caller-owned time. It
derives status, `thisUpdate`, and `nextUpdate` only from the exact retained
`SingleResponse`, rejects every currently unsupported `singleExtensions`
collection, and applies the shared repository-owned fail-closed revocation
policy. Only `good` inside the ordered half-open freshness interval
`thisUpdate <= observation_time < nextUpdate` succeeds. The distinct
non-copyable result retains the complete target, issuer, and response binding
plus the exact observation time, preventing detached status or freshness
values from being attached to another certificate. This composition does not
authenticate the OCSP signature,
authorize or bind a responder, interpret outer response extensions, prove the
candidate issuer signed the target, bind the result to a constructed path,
fetch or cache revocation data, read a clock, implement nonce policy, establish
trust, perform TLS or HTTP, authorize OAuth transport, or grant network
authority. Concrete HTTPS remains unavailable.

A bounded `x509-ocsp-authenticated-target-revocation` composition now consumes
that exact good-and-fresh target/issuer result, one issuer-bound authenticated
RSA SHA-256 responder, and one repository-owned RSA SHA-256 issuer link for the
target certificate. It requires the revocation result's exact `SingleResponse`
object to occur inside the signed `BasicOCSPResponse`, requires the target link
to retain the exact target certificate, and requires both certificate links and
the `CertID` binding to retain the same exact issuer certificate object.
It requires the signed response to contain exactly one `SingleResponse`, so an
unexamined entry encoded under another hash algorithm cannot conflict with the
selected target status, and revalidates the responder certificate at the exact
revocation observation time. Byte-equal re-decodes fail closed. Until outer
extension and nonce policy is implemented, any `responseExtensions` collection
is rejected. Non-copyable success evidence retains all three complete proofs
and the exact-time responder-validity result, establishing only that the same
candidate issuer signed the target and authorized the then-valid delegated
responder whose key signed the exact unambiguous response containing the good-
and-fresh target status. This fixed RSA SHA-256 composition does not construct
or validate a
certificate path, establish issuer trust, apply responder-path or responder-
revocation policy, interpret any OCSP extension, fetch or cache data, read a
clock, perform TLS or HTTP, authorize OAuth transport, or grant network
authority. Concrete HTTPS remains unavailable.

A bounded `x509-ocsp-reviewed-server-revocation-binding` composition now joins
that complete authenticated target-revocation evidence to the strongest
existing constructed reviewed-intermediate server-path evidence. It requires
both non-copyable inputs to retain the same exact decoded target and
intermediate issuer objects; byte-equal independent decodes fail closed.
After those identity checks, it revalidates both exact path certificates at
the OCSP proof's retained observation time and retains the two fresh validity
tokens, so earlier server-path validity evidence cannot be mixed with a later
revocation observation after either certificate expires.
Success retains the full OCSP target, issuer, delegated responder, signed
single-status response, freshness and observation-time evidence alongside the
full target-server policy, constructed two-link RSA SHA-256 path, path-length
state, exact Mozilla-reviewed root selection, selected-anchor DNS policy, and
supported intermediate DNS-constraint policy. Stable failures distinguish
target from issuer mismatch and target from issuer exact-time validity failure
without retaining response, certificate, key, signature, status, hostname, or
time payloads. This composition performs no new cryptography or external
effect. It does not generalize certificate-path
validation, establish a reviewed responder path, check responder-certificate
or intermediate revocation, interpret unsupported OCSP extensions, fetch or
cache data, read a clock, implement TLS or HTTP, authorize OAuth transport, or
grant network authority. Concrete HTTPS remains unavailable.

A bounded follow-up in
`x509-ocsp-reviewed-server-revocation-binding` now consumes that complete
reviewed server-revocation proof and reifies its delegated
responder-to-reviewed-root path as a distinct non-copyable type for later
policy composition. Earlier safe constructors already prove the exact
responder-policy, responder-link, shared-issuer, and reviewed-path identities;
this infallible projection performs no additional validation.
Success retains the signed response, good-and-fresh target status, responder
policy and signature, target and responder issuer links, constructed two-link
server path, path-length state, selected Mozilla-reviewed root, and the
exact-observation responder/intermediate validity evidence in one non-copyable
result. This establishes only the fixed RSA SHA-256 responder-to-reviewed-root
path evidence already present in those exact inputs. It does not apply anchor
or intermediate name constraints to the responder, check responder-certificate
or intermediate revocation, generalize path construction or validation,
interpret unsupported OCSP extensions, fetch or cache data, read a clock,
perform TLS or HTTP, authorize OAuth transport, or grant network authority.
Concrete HTTPS remains unavailable.

Another bounded policy slice in the same package consumes that exact reified
responder path and applies the selected Mozilla-reviewed terminal's supported
out-of-band DNS constraints to the exact authenticated responder certificate.
Both the responder and constraint bytes are derived through the retained proof;
callers cannot substitute a certificate, SAN collection, or snapshot entry.
Entries without constraints succeed without reading Subject Alternative Name.
Constrained entries decode the supported permitted-DNS subset and the exact
responder SAN under separate caller-owned ASN.1 limits, require every responder
`dNSName` to comply, and retain only a count token. Stable failures expose no
certificate, name, constraint, response, key, signature, or status payload.
This is selected-anchor DNS policy for that fixed responder path only. It does
not apply intermediate, excluded, or non-DNS constraints, check responder or
intermediate revocation, generalize path construction or validation, perform
TLS or HTTP, authorize OAuth transport, or grant network authority. Concrete
HTTPS remains unavailable.

A subsequent bounded policy slice consumes that complete responder-anchor
proof and applies the exact retained intermediate's supported critical DNS
constraints to the exact authenticated responder certificate. Both
certificates are derived through the retained proof, so callers cannot
substitute a responder, SAN collection, intermediate, or path. An absent
NameConstraints extension succeeds without reading responder SAN. A present
extension must be critical and decode under caller-owned limits as the
supported permitted-and-excluded-DNS subset; every responder `dNSName` must
comply, and success retains only count evidence. Stable failures expose no
certificate, name, constraint, response, key, signature, or status payload.
This remains fixed-path DNS policy only: it does not implement excluded anchor
constraints, non-DNS constraints, responder or intermediate revocation,
generalized path construction or validation, TLS, HTTP, OAuth transport, or
network authority. Concrete HTTPS remains unavailable.

A following bounded revocation-policy slice consumes that complete responder
DNS proof and applies RFC 6960's trust-for-certificate-lifetime branch to the
exact authenticated responder. The responder certificate is derived only
through the retained proof; callers cannot substitute a certificate or
extension value. The structurally unique `id-pkix-ocsp-nocheck` extension must
be present and its value must be exact DER `NULL`. A critical instance can
reach this semantic gate only when the prerequisite certificate-wide policy
explicitly recognized its OID; RFC 6960 recommends, but does not require, a
non-critical extension. Success retains the complete reviewed responder proof,
including issuer authorization and responder validity at the exact OCSP
observation time. Stable failures expose no certificate, extension, response,
key, signature, status, hostname, or time payload. This implements only the
no-check branch: responders without the extension still require separately
authenticated revocation evidence, and intermediate revocation, generalized
path validation, TLS, HTTP, OAuth transport, and network authority remain
unavailable. Concrete HTTPS remains unavailable.

A bounded `x509-authority-information-access` prerequisite now decodes one
RFC 5280 Authority Information Access certificate extension under the caller's
existing DER depth, element, length, and OID-arc limits. The extension must be
non-critical and contain a non-empty, fixed-capacity ordered sequence of
`AccessDescription` values. Each retained value contains its arbitrary
canonical access-method OID and one non-empty URI `GeneralName`; unsupported
location forms fail closed rather than becoming opaque fetch targets. The
package exposes the standard `id-ad-ocsp` and `id-ad-caIssuers` identifiers as
data and lets callers select any exact method without embedding provider
behavior. Success borrows only the validated method and URI values, while
stable failures retain no OID arcs, URI text, extension bytes, certificate, or
provider payload. This is structural discovery data only: it does not validate
RFC 3986 syntax, authorize URI schemes or hosts, select a responder, fetch or
cache data, bind a locator to authenticated revocation evidence, read a clock,
check revocation, perform TLS or HTTP, authorize OAuth transport, or grant
network authority. Concrete HTTPS remains unavailable.

An effect-free `x509-ocsp-aia-locator-policy` slice now consumes that decoded
collection and validates every exact `id-ad-ocsp` locator before uniquely
selecting one. The caller supplies the allowed lowercase URI schemes, each
scheme's implicit port and exact explicit-port set, plus URI, DNS-host, and
path/query length ceilings. The accepted profile is deliberately narrower than
general RFC 3986: it requires an absolute hierarchical URI with a DNS-style
host; rejects userinfo, IP literals, fragments, invalid DNS labels, malformed
or disallowed ports, legacy numeric IPv4 resolver spellings, unsafe path/query
characters, and incomplete percent
escapes; and fails closed on multiple OCSP descriptions. Other AIA methods do
not become OCSP candidates. Policy is checked before discovery data, success
borrows the exact AIA description, and stable failures expose no URI, host,
certificate, provider, or response payload. This proves syntax, caller policy,
and unique selection only: it does not authorize DNS or a destination, verify
hostname identity, resolve an address, build an HTTP request, follow redirects,
fetch or cache OCSP data, authenticate a response, read a clock, decide
revocation, validate a path, establish trust, perform TLS or HTTP, authorize
OAuth transport, open a socket, or access a credential. Concrete HTTPS remains
unavailable.

An allocation-free `tls13-record` prerequisite now owns only the outer RFC
8446 record framing needed by a future repository-owned TLS client. It decodes
and encodes strict five-byte TLS record headers plus borrowed fragments,
retains but does not interpret the deprecated plaintext legacy record version,
and provides separate plaintext and ciphertext entry points. Plaintext callers
supply an explicit non-empty content-type allowlist and a fragment ceiling no
larger than `2^14`; ciphertext requires the TLS 1.3 legacy version `0x0303`, is
fixed to outer `application_data`, rejects an empty fragment, and is bounded by
`2^14 + 256`. Control-plane plaintext records cannot be empty, while
zero-length application data remains a distinct standards-permitted case for
later sequencing policy. Exact decoders reject trailing bytes and prefix
decoders return the unconsumed stream tail. Encoders emit `0x0303` and validate
the complete record and output capacity before mutating caller-owned storage.
Stable failures retain no record contents.

This slice does not parse handshake messages or `TLSInnerPlaintext`, implement
record sequencing, negotiate versions or cipher suites, derive keys, encrypt,
decrypt, authenticate, read entropy or a clock, validate certificates, select
trust roots, check revocation, perform I/O, open a socket, authorize HTTP or
OAuth transport, or replace `tls-platform`. Concrete HTTPS remains
unavailable.

An allocation-free `tls13-extension-block` prerequisite now decodes the generic
two-byte-length-prefixed extension-vector syntax shared by TLS 1.3 handshake
messages. Caller-owned limits bound the complete payload and retained entry
count, the exact declared length must consume all remaining input, and each
entry retains one exact unclassified type plus borrowed opaque data in wire
order. Duplicate extension types, truncated headers or data, trailing bytes,
and resource-limit exhaustion fail closed with payload-free categories.
Unknown types remain data rather than becoming implicitly supported, and
diagnostics omit extension-data bytes.

This syntax layer embeds no extension registry, decodes no extension value,
and does not authorize, require, or forbid an extension in ClientHello,
ServerHello, EncryptedExtensions, or another handshake context. It does not
process a handshake, select a suite, derive keys, invoke cryptography, validate
certificates, establish trust, enforce revocation, perform I/O, authorize HTTP
or OAuth transport, or open a network connection. Context-specific extension
policy and decoding, handshake state, repository-owned record protection,
certificate-path validation, revocation, TLS transport, and HTTP composition
remain blockers. Concrete HTTPS remains unavailable.

An allocation-free `tls13-inner-plaintext` prerequisite now specifies the
authenticated cleartext syntax inside a protected TLS 1.3 record. Its decoder
first bounds the complete caller-supplied cleartext, scans backward only within
that slice for the final nonzero content-type octet, and returns borrowed
content plus the zero-padding length. Its encoder validates the entire
`content || type || zero padding` length and caller-owned output capacity before
writing. A caller-selected ceiling from 1 through `2^14 + 1` carries negotiated
record-size policy, and a non-empty, duplicate-free allowlist keeps protected
content authorization data-driven. Only `handshake`, `alert`, and
`application_data` may be allowed: protected ChangeCipherSpec is forbidden.
Empty application data remains representable while empty handshake and alert
content fail closed. Stable failures retain no cleartext bytes, content type
value, or lengths.

This slice may receive only cleartext already authenticated by a future AEAD
boundary. It does not encrypt, decrypt, authenticate, select padding, manage
nonces or record sequence numbers, process handshake or alert messages,
negotiate versions or cipher suites, derive keys, read entropy or a clock,
validate certificates, select trust roots, check revocation, perform I/O, open
a socket, authorize HTTP or OAuth transport, or replace `tls-platform`.
Concrete HTTPS remains unavailable.

An allocation-free `tls13-record-nonce` prerequisite now specifies record
sequence consumption and nonce derivation for one TLS 1.3 traffic key in one
direction. A caller-selected policy fixes the exact static-IV length from 8
through 255 bytes and a per-key record ceiling from 1 through `u64::MAX`.
Fresh state starts at sequence zero and has no reset operation. Each successful
reservation left-pads the current 64-bit network-order sequence to the selected
IV length, XORs it with the caller-borrowed static IV into caller-owned output,
and consumes that sequence before returning. Static-IV and output lengths are
validated before either output or sequence state changes; exhaustion also
leaves both untouched. The largest policy ceiling conservatively stops after
sequence `u64::MAX - 1`, before an increment could wrap. Callers must own
separate read and write state, create fresh state after each key change, and
select the lower cipher-suite-specific cryptographic usage limit.

The package's 255-byte IV ceiling is an explicit work bound, not permission for
an AEAD to use an unsupported nonce length. It stores no static IV, key, nonce,
plaintext, or ciphertext bytes, and stable failures retain no caller lengths or
bytes. It does not select or invoke an AEAD, encrypt, decrypt, authenticate,
derive traffic keys or IVs, negotiate a cipher suite, perform key updates,
process handshake or alert messages, validate certificates, select trust
roots, check revocation, perform I/O, open a socket, authorize HTTP or OAuth
transport, or replace `tls-platform`. Concrete HTTPS remains unavailable.

An allocation-free `tls13-cipher-suite-policy` prerequisite now binds the
three repository-targeted RFC 8446 TLS 1.3 cipher-suite wire identifiers to
their exact AEAD algorithm, HKDF hash, key length, IV length, authentication-tag
length, and conservative maximum records per traffic key. The standard
metadata is immutable. A caller supplies a non-empty, duplicate-free allowlist
and an explicit nonzero record ceiling for every allowed suite; each ceiling
must be no greater than the suite maximum. Selection accepts only an exact
known wire identifier already authorized by that policy and returns one
validated value carrying all parameters together, so later key derivation,
nonce state, and AEAD routing cannot independently reinterpret the negotiated
suite. AES-GCM policies conservatively stop at `2^24` records, below RFC 8446
section 5.5's `2^24.5` full-size-record bound. ChaCha20-Poly1305 policies are
bounded by the record sequence because RFC 8446 states that it wraps before the
algorithm's safety limit; the nonce prerequisite's conservative sequence
semantics still stop before increment wrap.

The registry is policy data only. It neither proves that a repository-owned
AEAD or hash implementation is suitable for production nor selects a suite on
behalf of a handshake. It stores no keys, IVs, nonces, plaintext, ciphertext,
or provider data; invokes no cryptography; and performs no negotiation, key
derivation, record protection, key update, certificate processing, trust,
revocation, I/O, HTTP, OAuth transport, or network effect. Unsupported RFC 8446
CCM suites fail closed until separately reviewed repository-owned primitives
and limits are added. Concrete HTTPS remains unavailable.

An allocation-free `tls13-certificate-verify-input` prerequisite now owns only
the exact RFC 8446 section 4.4.3 content hashed and signed for a TLS 1.3
`CertificateVerify` message. A caller selects the server or client role and
SHA-256 or SHA-384 transcript-hash metadata. The builder writes 64 space bytes,
the exact role-specific context string, one zero separator, and the caller's
exactly 32- or 48-byte transcript hash into caller-owned output. Transcript
hash length and complete output capacity are checked before output changes;
extra capacity is untouched. The returned borrowed value keeps role, hash
metadata, and the exact signed bytes together while redacting bytes from debug
output.

This framing boundary does not calculate a transcript hash, bind the hash to a
negotiated cipher suite, encode or decode a `CertificateVerify` handshake
message, select a signature scheme, parse a key, hash, sign, verify a signature,
process or validate certificates, build a path, verify a name, select trust
roots, check revocation, perform I/O, or provide TLS, HTTP, or OAuth transport.
Those bindings and effects remain future reviewed composition work. Concrete
HTTPS remains unavailable.

An allocation-free `tls13-hkdf-label` prerequisite now owns only the exact
RFC 8446 section 7.1 `HkdfLabel` information framing consumed by a future
repository-owned HKDF boundary. A validated value binds one public 16-bit
requested output length, a caller label from 1 through 249 bytes, and a context
from 0 through 255 bytes. Encoding always prepends the six-byte `"tls13 "`
namespace, emits the resulting 7-through-255-byte label and one-byte context
lengths, and writes into caller-owned storage only after the complete encoded
length and capacity have been checked. The maximum encoded value is 514 bytes.
Stable failures expose only categories, never caller bytes or lengths.

This framing slice receives no secret, performs no HKDF or hashing, selects no
cipher-suite hash, derives or stores no key or IV, and does not audit the
generic repository HKDF implementation for production TLS secret handling.
Later traffic-key composition must combine a selected cipher suite, reviewed
HKDF execution, the labels `"key"` and `"iv"`, and empty context without
letting detached caller parameters reinterpret the selected suite. Handshake
key scheduling, transcript hashing, record protection, certificate validation,
trust, revocation, I/O, HTTP, OAuth transport, and concrete HTTPS remain
unavailable.

A fixed-memory `sha384` prerequisite now supplies the missing repository-owned
FIPS 180-4 digest selected by `TLS_AES_256_GCM_SHA384`. Its one-shot and
streaming entry points share the SHA-384 initial state, 1024-bit block
compression, 128-bit length encoding, and 384-bit truncation rules. Streaming
updates reject a message whose encoded bit length would exceed the standard's
128-bit field before mutating retained state. Digest snapshots are
non-destructive, allocate no heap memory, and scrub their temporary state and
padding; the live hasher also scrubs its retained chaining state and partial
block on drop. Stable failures contain no input bytes or lengths.

This digest primitive does not implement HMAC, HKDF, `HKDF-Expand-Label`, a
TLS key schedule, transcript sequencing, cipher-suite negotiation, AEAD, record
protection, handshake processing, certificate validation, trust, revocation,
I/O, HTTP, OAuth transport, or any network effect. In particular, its presence
does not make the existing generic repository HKDF suitable for production TLS
secret handling. Reviewed fixed-output HMAC-SHA-256/HMAC-SHA-384 and HKDF
execution remain prerequisites before the label framing can derive traffic
keys or IVs. Concrete HTTPS remains unavailable.

A bounded `rsa-pss-mgf1-sha384` prerequisite now implements only the RFC 8017
MGF1 expansion required by the future SHA-384 EMSA-PSS verification path. It
accepts the exact 48-byte PSS hash value as its seed, validates both the
standard's `2^32 * 48` maximum and a smaller caller-selected nonzero work
ceiling before changing caller-owned output, encodes each counter as four
network-order octets, and fills the requested mask directly without allocation.
The repository-owned SHA-384 implementation is its only production dependency.

This primitive does not parse parameters or keys, perform RSA arithmetic,
validate an EMSA-PSS encoded message, choose a salt policy, verify a signature,
process a certificate, or grant path, trust, revocation, TLS, HTTP, OAuth
transport, or network authority. RSA-PSS support remains unavailable until
those separate boundaries are implemented, composed, and tested.

The repository-owned `sha256` prerequisite is now hardened for that future
fixed-output HMAC boundary. Its one-shot and streaming digest paths share
fixed-memory 512-bit block processing, and checked streaming updates reject a
total message whose encoded bit length would overflow FIPS 180-4's 64-bit field
before mutating retained state. The original infallible streaming update
remains an explicit compatibility wrapper around that checked operation.
Digest snapshots are non-destructive, cloning does not allocate, and retained
state plus temporary schedules and padding are scrubbed at their ownership
boundaries. This hardening supplies only SHA-256: it does not implement HMAC,
HKDF, TLS transcript or key scheduling, record protection, handshake
processing, certificate validation, trust, revocation, I/O, HTTP, OAuth
transport, or any network effect. Concrete HTTPS remains unavailable.

The repository-owned Rust `ct-compare` primitive is now hardened as a future
HMAC-verification prerequisite. Equal-length comparisons retain an observable
volatile accumulator operation for every byte, so an optimizing compiler may
not replace the full scan with a data-dependent early exit. An optimized
LLVM-IR regression gate verifies that every optimized loop cycle crosses those
operations and that loop exits remain independent of their values. Length
remains public and mismatched lengths are rejected directly. This hardening
does not migrate the existing generic `hmac` verifier
away from its external dependency, make generic HMAC fixed-memory, add
HMAC-SHA-384, review hash selection for TLS, or implement HKDF, a TLS key
schedule, record protection, handshake processing, certificate validation,
trust, revocation, I/O, HTTP, OAuth transport, or any network effect. Reviewed
fixed-output HMAC-SHA-256 and HMAC-SHA-384 remain prerequisites, and concrete
HTTPS remains unavailable.

The Rust `hmac` verifier now consumes that repository-owned comparison
primitive instead of the external `subtle` crate. Equal-length tag checks pass
through the audited compiler-preserved full scan, while public length
mismatches fail directly. The public verifier API and tag-generation outputs
are unchanged, and this slice performs no credential access, signing, audit,
or external effect. It does not make generic HMAC fixed-memory, add
HMAC-SHA-384, review hash selection for TLS, or implement HKDF, a TLS key
schedule, record protection, handshake processing, certificate validation,
trust, revocation, I/O, HTTP, OAuth transport, or any network effect. Reviewed
fixed-output HMAC-SHA-256 and HMAC-SHA-384 remain prerequisites, and concrete
HTTPS remains unavailable.

The existing Rust `hmac_sha256` entry point now supplies the bounded
fixed-output HMAC-SHA-256 prerequisite without routing secret-derived state
through the generic heap-allocating HMAC path. It uses the repository-owned
fixed-memory SHA-256 stream twice, holds one 64-byte normalized key block on
the stack, transforms that block from the inner pad to the outer pad in place,
and wipes normalized key bytes, keyed pads, the inner digest, and both retained
hash states at their ownership boundaries. Checked SHA-256 length failures are
stable categories containing no caller bytes or lengths; the returned 32-byte
tag remains caller-owned. RFC 4231 vectors and key/message boundary cases lock
the output to the generic compatibility implementation.

This primitive performs no credential access, edit, signing operation, audit,
provider action, I/O, or external effect. It does not make generic HMAC
fixed-memory, add HMAC-SHA-384, select a TLS cipher suite, implement HKDF or a
TLS key schedule, process handshake records or certificates, establish trust,
check revocation, authorize HTTP or OAuth transport, or open a network
connection. Fixed-output HMAC-SHA-384 and reviewed HKDF execution remain
prerequisites, and concrete HTTPS remains unavailable.

The Rust `hmac_sha384` entry point now supplies the corresponding bounded
fixed-output HMAC-SHA-384 prerequisite. It uses the repository-owned
fixed-memory SHA-384 stream twice, retains one 128-byte normalized key block on
the stack, transforms that block from the inner pad to the outer pad in place,
and wipes normalized key bytes, keyed pads, the inner digest, and both retained
hash states at their ownership boundaries. Checked SHA-384 length failures are
stable categories containing no caller bytes or lengths; the returned 48-byte
tag remains caller-owned. RFC 4231 vectors and key/message boundary cases lock
the result to the generic compatibility implementation.

This primitive performs no credential access, edit, signing operation, audit,
provider action, I/O, or external effect. It does not make generic HMAC
fixed-memory, select a TLS cipher suite, implement or review HKDF execution,
build a TLS key schedule, process handshake records or certificates, establish
trust, check revocation, authorize HTTP or OAuth transport, or open a network
connection. Reviewed HKDF execution remains a prerequisite, and concrete HTTPS
remains unavailable.

The Rust `hkdf_extract_fixed` entry point now supplies the bounded Extract half
of reviewed HKDF execution for SHA-256 and SHA-384. A data-driven
`FixedHkdfHash` policy admits only the repository-owned fixed-output HMAC paths.
The RFC 5869 default salt occupies a fixed stack buffer, temporary HMAC results
are wiped on drop, and the returned `HkdfPrk` owns one fixed backing array that
exposes only the selected 32-byte or 48-byte prefix and wipes the full array on
drop. Its Debug representation contains algorithm and public length metadata,
never PRK bytes. Checked hash-length failures remain payload-free.

This primitive performs no credential access, edit, signing operation, audit,
provider action, I/O, or external effect. It does not change the legacy
allocation-backed HKDF compatibility APIs, implement fixed-memory HKDF-Expand
or `HKDF-Expand-Label`, compose a TLS key schedule, process handshake records
or certificates, establish trust, check revocation, authorize HTTP or OAuth
transport, or open a network connection. Reviewed fixed-memory Expand remains
a prerequisite, and concrete HTTPS remains unavailable.

The fixed-output HMAC-SHA-256 and HMAC-SHA-384 primitives now also accept
borrowed message segments. They stream each segment into the repository hash
state in order without allocating or assembling a concatenated whole-message
buffer. A bounded partial block may be retained inside the wiped hash state.
The APIs preserve the existing fixed-memory key normalization, wipe-on-drop
key-derived state, caller-owned fixed tag, and payload-free length failure.
Segment boundaries do not change the tag, and an empty segment list is the
empty message. This is the bounded input primitive needed to represent
`T(i-1) || info || counter` without constructing an allocation-backed aggregate
buffer.

This HMAC slice performs no HKDF expansion, secret derivation, credential
access, credential edit, signing operation, provider action, trace emission,
I/O, or external effect. It does not implement `HKDF-Expand`,
`HKDF-Expand-Label`, a TLS key schedule, handshake or record protection,
certificate processing, trust or revocation policy, HTTP, an OAuth transport,
or a network connection. Reviewed fixed-memory Expand remains a prerequisite,
and concrete HTTPS remains unavailable.

The Rust `hkdf_expand_fixed` entry point now completes the bounded RFC 5869
Expand half for the repository-owned SHA-256 and SHA-384 paths. It accepts an
`HkdfPrk` directly, so the data-driven algorithm selected during Extract cannot
disagree with Expand, and it derives into caller-provided output storage. Each
HMAC receives borrowed `T(i-1)`, `info`, and counter segments without an
aggregate message allocation. One fixed 48-byte previous-block owner and every
temporary HMAC tag in its original result slot are wiped; an optimized-IR gate
checks those scrubs and rejects allocator calls in the Expand boundary. Public
output and encoded-message length checks occur before caller output changes,
and an unexpected PRF failure wipes the destination before returning.
Successful output remains caller-owned.

This fixed-memory Expand slice performs secret derivation only inside the
caller's process. It performs no credential access, credential edit, signing
operation, provider action, audit or trace emission, I/O, or external effect.
It does not implement `HKDF-Expand-Label`, select or compose a TLS cipher suite
or key schedule, process handshake or record protection, parse or validate
certificates, select trust roots, enforce revocation, authorize HTTP or OAuth
transport, or open a network connection. Reviewed `HKDF-Expand-Label` and TLS
key-schedule composition remain prerequisites, and concrete HTTPS remains
unavailable.

The Rust `tls13-hkdf-label` layer now composes that fixed-memory Expand boundary
with its existing bounded RFC 8446 section 7.1 framing. The public output
length, exact `"tls13 "` namespace, caller label, and context occupy one fixed
514-byte stack buffer before the PRK-bound SHA-256 or SHA-384 expansion derives
into caller-owned output. Algorithm selection remains data-driven through
`HkdfPrk`; framing cannot disagree with Extract. Public framing and HKDF length
failures occur before caller output changes, while the underlying unexpected
PRF failure path wipes the destination. RFC 8448 SHA-256 evidence and a
three-block independent SHA-384 vector cover the composition.

This `HKDF-Expand-Label` slice performs only in-process secret derivation. It
performs no credential access, edit, signing operation, provider action, audit
or trace emission, I/O, or external effect. It does not select or compose a TLS
cipher suite or key schedule, manage transcript sequencing, process handshake
or record protection, parse or validate certificates, select trust roots,
enforce revocation, authorize HTTP or OAuth transport, or open a network
connection. Reviewed TLS key-schedule composition remains a prerequisite, and
concrete HTTPS remains unavailable.

The same repository-owned boundary now exposes the RFC 8446 section 7.1
`Derive-Secret` operation over an already-computed transcript hash. The
wipe-on-drop `HkdfPrk` remains the single source of SHA-256 or SHA-384 policy:
both the transcript-hash input and caller-owned secret output must equal that
selected hash's exact 32-byte or 48-byte length before expansion begins. The
operation then delegates to the fixed-memory `HKDF-Expand-Label` path, so its
stack-only framing, public-failure atomicity, and unexpected-PRF output wipe are
preserved. RFC 8448 SHA-256 evidence and an independently generated SHA-384
vector cover both admitted algorithms.

This is one key-schedule primitive, not a TLS key schedule. It accepts a
transcript hash rather than owning transcript sequencing or hashing, and it
does not label or retain early, handshake, master, traffic, exporter, or
resumption secret state. It performs no credential access, edit, signing,
provider action, audit or trace emission, I/O, or external effect. Cipher-suite
selection and complete key-schedule state composition remain prerequisites,
along with handshake and record processing, certificate validation, trust-root
selection, revocation, HTTP authorization, OAuth transport, and networking.
Concrete HTTPS remains unavailable.

The fixed-memory HKDF boundary now also accepts an already-derived
pseudorandom key through one exact-length import operation. The caller selects
SHA-256 or SHA-384 through `FixedHkdfHash`; the borrowed input must be exactly
that hash's 32-byte or 48-byte output length before any secret copy occurs.
Success copies only that exact prefix into the existing fixed 48-byte
wipe-on-drop `HkdfPrk` owner, leaving the unused SHA-256 tail zeroed. The input
remains caller-owned and the caller retains responsibility for wiping it. A
stable payload-free error exposes no secret bytes or caller length.

Both fixed Extract and exact import retain their original PRK staging owner
after copying into the caller-owned result, then wipe that original 48-byte
storage before return. The release-codegen regression gate verifies those
staging scrubs, the existing HMAC-tag and previous-block scrubs, and the absence
of allocator calls across the fixed Extract, import, and Expand boundaries.

Import is not HKDF-Extract and does not establish entropy, provenance, or
protocol meaning. In particular, accepting arbitrary input keying material at
this boundary would be caller misuse. This slice exists so an already-derived
TLS secret can later re-enter the repository-owned Expand boundary without an
allocation-backed compatibility API or an unowned long-lived copy. It performs
no credential access, edit, signing, provider action, audit or trace emission,
I/O, or external effect, and it does not compose the TLS key schedule, derive
traffic keys or IVs, process handshake or records, validate certificates,
select trust, enforce revocation, authorize HTTP or OAuth transport, or open a
network connection. Concrete HTTPS remains unavailable.

A bounded `tls13-traffic-key-material` composition now derives one selected
TLS 1.3 cipher suite's record-protection key and static IV from an
already-derived traffic secret. It consumes the existing caller-policy-selected
suite value together with the fixed wipe-on-drop `HkdfPrk` owner, rejects a
SHA-256/SHA-384 mismatch before derivation, and takes the exact key and IV
lengths only from the suite's immutable descriptor. The fixed labels `"key"`
and `"iv"` and empty context are internal rather than caller-controlled. Both
outputs occupy non-cloneable caller-owned fixed wipe-on-drop storage, retain
the selected AEAD and caller-authorized per-key record ceiling, and become
accessible only through a success-only borrowed view. Secret bytes never cross
the ABI return slot by value and diagnostics remain redacted. If IV derivation
fails after the key succeeds, the key's staging owner is wiped while unwinding
the error path. On success, both staging owners remain alive after their
deliberate copies into caller-owned storage and scrub their original stack
slots before return; a release-codegen gate verifies those source scrubs and
allocator absence.

This composition does not establish the traffic secret's entropy, provenance,
or key-schedule position; derive or retain early, handshake, master, exporter,
resumption, or traffic secrets; select or negotiate a suite; invoke an AEAD;
construct per-record nonces; protect records; process a handshake; validate
certificates; establish trust; enforce revocation; perform I/O; authorize HTTP
or OAuth transport; or open a network connection. Repository-owned AEAD
review and composition, complete key-schedule and transcript state, handshake
processing, certificate-path validation, revocation policy, TLS transport, and
HTTP composition remain blockers. Concrete HTTPS remains unavailable.

An allocation-free `tls13-handshake` prerequisite now owns only the RFC 8446
four-byte handshake-message envelope needed by a future repository-owned TLS
state machine. It recognizes the ten core TLS 1.3 wire message types, while a
caller-supplied non-empty, duplicate-free allowlist decides which recognized
types the current protocol state may accept. A caller-selected body ceiling
from zero through the protocol's 24-bit maximum bounds work before any body is
borrowed. Exact decoding rejects trailing bytes; prefix decoding returns the
unconsumed stream tail for a later reassembly loop. Successful decoding borrows
both the declared body and the exact `type || uint24 length || body` bytes so a
future transcript owner can hash the authenticated wire representation without
re-encoding it. Encoding checks type policy, the complete 24-bit length, and
caller-owned output capacity before changing output. Stable failures retain no
message type, length, body, transcript, or output bytes.

Recognition is deliberately separate from state authorization. This slice
does not recognize synthetic transcript-only `message_hash` as a wire message,
reassemble a message split across records, sequence handshake phases, own or
update a transcript hash, parse any handshake body, negotiate parameters,
derive keys, invoke an AEAD, process certificates, establish trust, enforce
revocation, perform I/O, authorize HTTP or OAuth transport, or open a network
connection. The caller must first authenticate protected record contents and
must retain incomplete bytes until one complete declared message is available.
Complete transcript and handshake state, repository-owned AEAD composition,
certificate-path validation, revocation policy, TLS transport, and HTTP
composition remain blockers. Concrete HTTPS remains unavailable.

An allocation-free `tls13-key-update-message` prerequisite now owns the exact
one-octet RFC 8446 section 4.6.3 `KeyUpdate` handshake body. Exact decoding
accepts only `update_not_requested` and `update_requested`, rejects truncated,
padded, and out-of-range bodies with payload-free categories, and returns one
typed request whose response requirement cannot be reinterpreted by a caller.
Encoding writes the typed request into caller-owned output only after complete
capacity validation and leaves extra capacity untouched.

This body layer does not authorize `KeyUpdate` before peer `Finished`, sequence
post-handshake messages, retain a pending response, coalesce repeated requests,
derive or rotate read or write traffic secrets, enforce old-key protection or
record-boundary alignment, send an alert, protect a record, perform I/O,
authorize HTTP or OAuth transport, or open a network connection. Complete
handshake and post-handshake state, key-update secret derivation,
repository-owned record protection, certificate-path validation, revocation
policy, TLS transport, and HTTP composition remain blockers. Concrete HTTPS
remains unavailable.

The repository-owned ChaCha20 implementation now exposes a bounded
`chacha20_xor_fixed` prerequisite for the `TLS_CHACHA20_POLY1305_SHA256` path.
It accepts exact caller-owned input and output buffers, validates the entire
RFC 8439 32-bit block-counter range before changing output, and derives through
one reusable fixed wipe-on-drop keystream owner. Each block's original and
working 16-word key-derived states are wiped on return. Its 20 rounds and ARX
operations do not branch on secret values, output does not cross the ABI return
slot, and the boundary performs no allocation. RFC 8439 evidence, block-edge
equivalence, failure atomicity, and final-counter tests cover the admitted
surface. The caller remains responsible for preventing key, nonce, and counter
position reuse; this primitive neither reserves nor persists nonce state.

The repository-owned Poly1305 implementation now also exposes a bounded
`poly1305_mac_fixed` prerequisite. It writes the 16-byte tag into exact
caller-owned output, uses one reusable wipe-on-drop 16-byte block, and performs
no allocation. The standard five 26-bit-limb core has one fixed multiplication
and reduction schedule per admitted block; final conditional reduction uses
mask selection rather than secret-dependent control flow. Clamped key limbs,
accumulator limbs, the addition pad, multiplication products, and carry
workspace share one wipe-on-drop owner. RFC 8439 evidence and block-edge
equivalence plus an independently cross-checked multi-block vector cover the
admitted surface. The caller remains responsible for never reusing a one-time
Poly1305 key; this primitive neither derives nor reserves that key.

These are still only separate stream-cipher and authenticator primitives. They
do not provide a combined AEAD boundary, constant-time tag-verification API,
own traffic keys, construct or reserve record nonces, enforce the selected
suite's per-key record ceiling, frame or protect records, process a handshake,
validate certificates, establish trust, enforce revocation, perform I/O,
authorize HTTP or OAuth transport, or open a network connection. The
allocation-returning legacy ChaCha20, Poly1305, AEAD, and XChaCha20 APIs remain
compatibility and educational surfaces rather than production TLS composition.
Combined fixed-memory AEAD review remains a prerequisite, and concrete HTTPS
remains unavailable.

The repository-owned ChaCha20-Poly1305 implementation now also exposes a
bounded fixed-memory RFC 8439 section 2.8 AEAD composition. Encryption accepts
caller-owned plaintext, exact-length ciphertext output, a 256-bit key, a
96-bit nonce, associated data, and caller-owned 16-byte tag output. It derives
the one-time Poly1305 key from ChaCha20 block counter zero, encrypts only with
counter one and above, and authenticates the exact
`AAD || pad16(AAD) || ciphertext || pad16(ciphertext) || le64(AAD length) ||
le64(ciphertext length)` layout without constructing an allocation-backed MAC
buffer. Decryption recomputes and compares all 16 tag bytes before decrypting,
so authentication failure leaves caller-owned plaintext output unchanged.
Exact output length, 64-bit encoded lengths, and the complete ChaCha20 counter
range are validated before encryption changes either ciphertext or tag.
Stable failures retain no key, nonce, plaintext, ciphertext, tag, associated
data, or observed lengths.

The ChaCha20 block used to derive the one-time key, that copied Poly1305 key,
the Poly1305 limb state, its reusable block, and the recomputed tag all have
wipe-on-drop ownership. The fixed boundary allocates no heap memory and its tag
comparison has a fixed 16-byte schedule with no secret-dependent early exit.
The original key, nonce, inputs, and successful outputs remain caller-owned.
The existing allocation-returning AEAD and XChaCha20 APIs remain compatibility
and educational surfaces rather than production TLS entry points.

This composition performs only pure in-process cryptography and has no
credential access, edit, signing, provider action, audit or trace emission,
I/O, or external effect. It does not select a TLS cipher suite, derive or own a
traffic key, construct or reserve a record nonce, enforce a per-key record
ceiling, frame or protect a TLS record, reset record sequence state after a key
change, process a handshake, validate certificates, establish trust, enforce
revocation, perform HTTP, authorize OAuth transport, or open a network
connection. Repository-owned TLS record-protection composition and state
remain prerequisites, and concrete HTTPS remains unavailable.

An allocation-free `tls13-supported-versions` prerequisite now owns the exact
RFC 8446 `supported_versions` extension data emitted and consumed by a future
repository-owned TLS 1.3 client. A caller supplies a preference-ordered slice
of one through 127 two-byte version identifiers. Construction rejects
duplicates, requires TLS 1.3 (`0x0304`), and preserves explicitly configured
unknown values for later reserved-value or future-version policy. Encoding
emits the exact one-byte-length-prefixed ClientHello vector into caller-owned
storage only after validating complete capacity. ServerHello and
HelloRetryRequest validation accepts exactly one two-byte value, binds it to
the original client offer, rejects versions before TLS 1.3, and admits only
the exact TLS 1.3 value whose semantics this package implements. Stable
failures retain no configured identifiers, input bytes, or output lengths.

This slice receives already-delimited extension data. It does not locate or
deduplicate extensions, decide whether an extension is authorized in a
particular handshake message, construct a ClientHello, enforce the legacy
version fields or downgrade sentinel, send alerts, negotiate a cipher suite,
derive keys, protect records, process certificates, establish trust, check
revocation, perform I/O, authorize HTTP or OAuth transport, or open a network
connection. Extension-block and handshake composition remain prerequisites,
and concrete HTTPS remains unavailable.

An allocation-free `tls13-named-group-policy` prerequisite now binds each
recognized TLS named-group wire identifier to immutable key-agreement routing
metadata and exact wire/shared-secret lengths. The initial registry recognizes
X25519 (`0x001d`), with 32-byte key-exchange and shared-secret values, and
secp256r1 (`0x0017`), with a 65-byte uncompressed-point key exchange and a
32-byte shared secret. A caller constructs a non-empty, duplicate-free
allowlist from typed groups. Selection distinguishes unknown wire identifiers
from known but disallowed groups, and selected-group length validation fails
closed without retaining the identifier or observed length in stable errors.
Additional groups require separately reviewed repository-owned primitives and
standard parameter descriptors.

The registry is policy metadata only. It does not parse or encode a key-share
entry or extension, choose which groups a client offers, enforce group-specific
public-key encoding beyond the exact length, generate ephemeral secrets, invoke
or approve the existing repository X25519 implementation or a future P-256
implementation, validate a peer public key, reject an all-zero agreement
result, derive or retain handshake secrets, advance a handshake, establish
trust, enforce revocation, perform I/O, authorize HTTP or OAuth transport, or
open a network connection. Reviewed key-agreement execution, key-share and
handshake-state composition, TLS transport, and HTTP composition remain
blockers. Concrete HTTPS remains unavailable.

The current slices intentionally stop before provider HTTPS transport. The
loopback host owns only local TCP and injected browser authority; custody owns
audited secret lifecycle, the sealed-store adapter owns only encrypted storage,
and the broker owns policy and sequencing without acquiring clock or network
authority. The preceding slices remain complete boundaries, not
provider-specific midpoints.

---

## Where It Fits

```
   Agent code (e.g., gmail-agent, github-agent, calendar-agent)
        │
        │  asks broker for an access token
        ▼
   OAuthBroker  ← THIS SPEC
        │  ├── consults vault for stored tokens
        │  ├── refreshes if expired (silent)
        │  └── triggers user-facing auth if no tokens / revoked
        ▼
   OAuthClient (trait)
        │  the actual HTTP dance with the provider
        ▼
   https-transport  +  bounded-json  +  url-encode
        │
        ▼
   provider's OAuth endpoints
```

**Depends on:**
- `https-transport` — every OAuth call is HTTPS.
- `tls-platform` (transitively) — TLS for those calls.
- `bounded-json` — zero-dependency, depth-limited RFC 8259 parsing for hostile
  metadata and token endpoint responses.
- `vault-records`, `vault-key-custody` — token storage as Zeroizing
  records under `vault://oauth/...`.
- `vault-secure-channel` — when the broker talks to the vault.
- `capability-cage-rust` — `net:connect:<provider-host>:443` and
  `vault:read|write:oauth-<provider>-*` gating.
- `time` — token expiry tracking.
- `chacha20-poly1305` (transitively via vault) — encrypted at rest.
- `random` source (CSPRNG) — for PKCE verifier and state nonce.
- `sha256` — PKCE S256 transform.
- `base64url` — PKCE encoding, JWT id-token decoding.

**Used by:**
- `email-host` for Gmail XOAUTH2.
- A future `github-host`, `calendar-host`, `slack-host`, etc.
- The orchestrator's CLI (`orchestrator oauth login <provider>`)
  for first-run authorization.

---

## Design Principles

1. **Tokens live in the vault, never on disk plaintext.** Access
   tokens, refresh tokens, and id tokens are stored as `Zeroizing`
   bytes inside the vault. Agents never see the raw token unless
   they explicitly request it; the broker's `with_access_token`
   helper hands the token to a closure and zeroizes it after.

2. **Provider config is data, not code.** Adding a new provider
   means writing a `ProviderConfig` JSON file. Generic logic is
   shared.

3. **PKCE always.** Even when the provider technically allows
   confidential clients without PKCE, we always use it. PKCE
   removes the only secret-leakage path on the redirect.

4. **Auto-refresh is silent.** Agents don't manage token lifecycle.
   They ask the broker for a usable token; if the stored access
   token has under `refresh_lead_time` remaining, the broker
   refreshes before returning. Failures (revoked refresh token,
   provider down) bubble up.

5. **Multiple accounts per provider.** A user may have personal
   and work Gmail. Account identity comes from the OIDC `sub`
   claim when present; otherwise from the provider's userinfo
   endpoint or a user-supplied label.

6. **Capability-cage gated end to end.** Every HTTPS call goes
   through manifest-gated `secure_net::dial`; every vault read /
   write goes through manifest-gated vault APIs.

7. **Audit every protocol boundary and credential access.** Metadata request
   release, metadata validation, begin-auth, complete-auth, token decode,
   credential disclosure, refresh preparation, and revoke preparation are
   written to the orchestrator's audit log with provider, trace, and a closed
   outcome — never endpoint URLs, scopes, callback text, provider prose, or
   token bytes. Durable audit failure withholds the result or effect.

8. **Refresh-token rotation handled correctly.** Some providers
   issue a fresh refresh token on every refresh (rotating). The
   broker stores the new one atomically; if a rotation race
   occurs, the broker re-prompts the user.

---

## Key Concepts

### TokenSet

```rust
pub struct TokenSet {
    pub access_token:    Zeroizing<String>,
    pub refresh_token:   Option<Zeroizing<String>>,
    pub id_token:        Option<Zeroizing<String>>,
    pub token_type:      String,             // typically "Bearer"
    pub expires_at:      SystemTime,         // absolute, not "expires_in"
    pub scopes:          Vec<String>,        // actually granted, not requested
    pub refresh_token_expires_at: Option<SystemTime>,
}
```

Tokens are stored as `Zeroizing<String>` so they are wiped from
memory the moment the struct is dropped. The expiry is converted
from the provider's `expires_in` (seconds-from-now) to an absolute
`SystemTime` immediately so the broker doesn't depend on clock
drift.

### ProviderConfig

```rust
pub struct ProviderConfig {
    pub name:                  String,            // "google", "github", "slack", ...
    pub display_name:          String,            // shown in UI: "Google", "GitHub"

    // Endpoints
    pub authorization_endpoint: String,           // for browser redirect
    pub token_endpoint:        String,            // for code/refresh exchange
    pub device_endpoint:       Option<String>,    // for device flow
    pub revocation_endpoint:   Option<String>,    // for explicit revoke
    pub userinfo_endpoint:     Option<String>,    // for account identity

    // Client identity
    pub client_id:             String,
    pub client_authentication: ClientAuthentication,

    // Flow configuration
    pub redirect_uri:          String,            // e.g., http://localhost:53682/callback
    pub default_scopes:        Vec<String>,
    // PKCE is mandatory S256 and therefore is not a provider toggle.
    pub refresh_token_rotates: bool,              // some providers rotate; we track
    pub access_token_lifetime_hint: Duration,    // for proactive refresh tuning

    // Provider quirks
    pub auth_extra_params:     HashMap<String, String>,  // e.g., access_type=offline for Google
    pub token_response_format: TokenResponseFormat,      // json (default), form-encoded (rare)
}

pub enum TokenResponseFormat {
    Json,
    FormEncoded,        // legacy GitHub used this until 2014
}

pub enum ClientAuthentication {
    None, // public client; PKCE S256 remains mandatory
    ClientSecret {
        method: ClientSecretAuthenticationMethod,
        reference: ClientSecretReference, // opaque storage-agnostic key
    },
    PrivateKeyJwt {
        signer_reference: PrivateKeySignerReference, // non-exporting authority
    },
}
```

Static configs for well-known providers ship with this crate
(`code/packages/rust/oauth/src/providers/`):

```
google.json     gmail, gcal, gdrive — uses access_type=offline + prompt=consent
github.json     standard
slack.json      standard
microsoft.json  outlook, onedrive
spotify.json    standard
```

A user-defined provider is loaded at orchestrator start from
`./.orchestrator/oauth-providers/<name>.json`.

### AuthorizeBegin

When an agent (or the orchestrator on its behalf) initiates a
first-time authorization, it gets an `AuthorizeBegin` describing
how to complete:

```rust
pub enum AuthorizeBegin {
    /// Authorization Code flow: open this URL in a browser. The
    /// user logs in and is redirected back to redirect_uri with
    /// ?code=... and &state=...
    AuthCodeUrl {
        url:          String,
        state:        String,           // we generated; verify on callback
        pkce_verifier: Zeroizing<String>,  // store until callback
    },

    /// Device flow: show user_code on screen, poll device_endpoint
    /// every interval seconds until the user completes
    /// authorization at verification_uri (or it expires).
    Device {
        user_code:        String,
        verification_uri: String,
        verification_uri_complete: Option<String>,  // pre-filled URL
        device_code:      Zeroizing<String>,        // for polling
        expires_at:       SystemTime,
        interval:         Duration,
    },
}
```

The orchestrator decides which flow to use based on whether a
browser is available (auth-code) or not (device flow).

### OAuthClient Trait

```rust
pub trait OAuthClient: Send + Sync {
    fn provider(&self) -> &ProviderConfig;

    /// Construct the authorization URL or device flow start.
    fn begin_authorize(
        &self,
        scopes: &[&str],
    ) -> Result<AuthorizeBegin, OAuthError>;

    /// Complete an Authorization Code flow given the redirect URL's
    /// query string (e.g., "code=abc&state=xyz") and the previously
    /// stored PKCE verifier and state.
    fn complete_auth_code(
        &self,
        query:        &str,
        expected_state: &str,
        pkce_verifier:  &str,
    ) -> Result<TokenSet, OAuthError>;

    /// Poll a device flow. Returns Pending until the user completes
    /// or denies.
    fn poll_device(
        &self,
        device_code: &str,
    ) -> Result<DevicePollResult, OAuthError>;

    /// Exchange a refresh token for a fresh TokenSet.
    fn refresh(
        &self,
        refresh_token: &str,
    ) -> Result<TokenSet, OAuthError>;

    /// Best-effort revoke. Some providers ignore unknown tokens
    /// silently; we still call to inform them.
    fn revoke(
        &self,
        token: &str,
        kind:  TokenKind,
    ) -> Result<(), OAuthError>;
}

pub enum DevicePollResult {
    Pending     { interval: Duration },          // keep polling
    SlowDown    { new_interval: Duration },      // back off
    Authorized  { tokens: TokenSet },
    Denied,
    ExpiredToken,
}

pub enum TokenKind {
    Access,
    Refresh,
}
```

The default `HttpsOAuthClient` implements this against any
`HttpsTransport` (so tests can swap a `MockHttpsTransport` and the
client logic exercises without network).

### OAuthBroker

Sits one level above the client. Responsibilities:

```rust
pub struct OAuthBroker {
    /* opaque: clients per provider, vault handle, manifest */
}

impl OAuthBroker {
    pub fn new(
        manifest: Arc<Manifest>,
        vault:    Arc<dyn VaultClient>,
        https:    Arc<dyn HttpsTransport>,
    ) -> Self;

    /// Register a provider configuration. Idempotent.
    pub fn register_provider(&mut self, config: ProviderConfig)
        -> Result<(), OAuthError>;

    /// List accounts known for a provider.
    pub fn list_accounts(&self, provider: &str)
        -> Result<Vec<AccountSummary>, OAuthError>;

    /// Begin a first-time authorization for a new account on a
    /// provider. The orchestrator presents the AuthorizeBegin to
    /// the user.
    pub fn begin_authorize(
        &mut self,
        provider: &str,
        scopes:   &[&str],
    ) -> Result<AuthorizeBegin, OAuthError>;

    /// Complete an Authorization Code flow. The broker stores the
    /// resulting tokens in the vault and returns the account_id
    /// (derived from a fully verified OIDC subject or authenticated userinfo).
    pub fn complete_auth_code(
        &mut self,
        provider:      &str,
        query:         &str,
        expected_state: &str,
        pkce_verifier:  &str,
        label:         &str,        // user-visible label ("Personal Gmail")
    ) -> Result<AccountId, OAuthError>;

    /// Drive a device-flow poll loop until tokens or denial.
    /// Returns when finished. The orchestrator wraps this in
    /// progress callbacks for the UI.
    pub fn complete_device(
        &mut self,
        provider:    &str,
        device_code: &str,
        label:       &str,
        progress:    impl FnMut(DevicePollResult),
    ) -> Result<AccountId, OAuthError>;

    /// Get a usable access token for an account. If the stored
    /// access token has under refresh_lead_time remaining, the
    /// broker refreshes (silently) before returning. The token is
    /// passed to the closure and zeroized after.
    pub fn with_access_token<R>(
        &mut self,
        provider:   &str,
        account_id: &AccountId,
        f:          impl FnOnce(&str) -> R,
    ) -> Result<R, OAuthError>;

    /// Force a refresh now (e.g., the agent saw a 401 and wants
    /// to retry). Updates the stored tokens.
    pub fn force_refresh(
        &mut self,
        provider:   &str,
        account_id: &AccountId,
    ) -> Result<(), OAuthError>;

    /// Revoke and delete an account's tokens. Both provider-side
    /// (revocation endpoint) and vault-side cleanup.
    pub fn detach_account(
        &mut self,
        provider:   &str,
        account_id: &AccountId,
    ) -> Result<(), OAuthError>;
}

pub struct AccountSummary {
    pub account_id:     AccountId,        // verified OIDC sub or userinfo id
    pub label:          String,           // human label
    pub scopes:         Vec<String>,
    pub created_at:     SystemTime,
    pub last_refreshed: SystemTime,
    pub access_token_valid_until: SystemTime,
    pub refresh_token_present:    bool,
}
```

### Vault Storage Scheme

Tokens are stored under a deterministic namespace:

```
vault://oauth/<provider>/<account_id>/access_token       Zeroizing<String>
vault://oauth/<provider>/<account_id>/refresh_token      Zeroizing<String>
vault://oauth/<provider>/<account_id>/id_token           Zeroizing<String>
vault://oauth/<provider>/<account_id>/metadata           JSON {label, scopes, expires_at, ...}
```

The `account_id` is derived from the `sub` claim only after the OIDC ID token's
signature, issuer, audience, expiry, and transaction nonce have all been
verified; for non-OIDC providers (GitHub, Slack), it is the `id` field of an
authenticated userinfo response. If neither is
available, the broker uses a content-hash of (provider name +
canonical user identifier from userinfo).

The vault's existing per-namespace policy gates which agents can
read which OAuth tokens. A `gmail-host` agent's manifest declares:

```json
{
  "category": "vault",
  "action":   "read",
  "target":   "oauth/google/<account_id>/*"
}
```

The agent never holds the refresh token (`with_access_token` only
exposes the access token). The broker holds both, refreshes
silently, and stores rotated refresh tokens atomically.

### Refresh Lead Time

Default: 5 minutes. When the agent calls `with_access_token`, the
broker checks `expires_at - now()`. If less than 5 minutes, it
refreshes before returning the token. This means agents almost
never see expired tokens on the wire.

Configurable per-provider in `ProviderConfig` (some providers issue
short-lived 5-minute tokens; for those, lead time is shorter to
avoid refresh storms).

### Authorization Code Flow Detail

```
1. Agent (or orchestrator on first-run) calls
   broker.begin_authorize("google", &["https://mail.google.com/"]).

2. Broker:
   - Generates 32-byte URL-safe random state.
   - Generates 32-byte URL-safe random code_verifier.
   - For the explicit OpenID Connect path, requires the exact `openid` scope,
     generates an independent 32-byte URL-safe random nonce, includes it in the
     browser request, and returns a provider/client/trace-bound zeroizing nonce
     that the caller retains for the later ID-token proof.
   - Computes code_challenge = base64url(sha256(code_verifier)).
   - Constructs authorization URL with:
       client_id, redirect_uri, response_type=code,
       scope=<requested>, state=<state>,
       code_challenge=<challenge>, code_challenge_method=S256,
       + provider-specific extras (e.g., access_type=offline,
                                    prompt=consent for Google).
   - Returns AuthorizeBegin::AuthCodeUrl with the URL, state,
     pkce_verifier.

3. Orchestrator opens URL in browser; user authorizes.

4. Provider redirects to redirect_uri with ?code=...&state=....
   The orchestrator's local listener (a transient HTTP listener
   bound to redirect_uri's port) receives the request.

5. Orchestrator calls
   broker.complete_auth_code("google", query, &state, &pkce_verifier,
                              "Personal Gmail").

6. Broker:
   - Verifies state in query matches expected_state. If not, error.
   - POSTs to token_endpoint:
       grant_type=authorization_code, code=<code>,
       redirect_uri=<same as before>,
       client_id=<id>, code_verifier=<pkce verifier>
       (plus client_secret if confidential client).
   - Parses response:
       access_token, refresh_token, id_token (if openid scope),
       token_type, expires_in, scope.
   - Converts expires_in to absolute expires_at.
   - Treats `id_token` as opaque until an OpenID Connect layer has
     verified its signature plus `iss`, `aud`, `exp`, and transaction
     `nonce`. An unverified JWT payload is never an account identity.
   - A later OIDC layer may derive `account_id` from a fully verified
     `id_token.sub`; otherwise the broker uses authenticated userinfo.
   - Atomically writes: access_token, refresh_token, id_token,
     metadata to vault under vault://oauth/google/<account_id>/*.
   - Returns account_id.
```

### Device Flow Detail

```
1. Broker POSTs to device_endpoint:
       client_id=<id>, scope=<requested>.
2. Provider returns:
       device_code, user_code, verification_uri,
       verification_uri_complete (optional), expires_in, interval.
3. Broker returns AuthorizeBegin::Device with these.
4. Orchestrator displays user_code + verification_uri to user
   (e.g., "Go to https://google.com/device and enter ABCD-EFGH").
5. The orchestrator creates an opaque sequence using a value from its monotonic
   timeline, waits until the returned next-poll value, and supplies its next
   observed value to the broker. Early and locally expired steps perform no
   request; each due step performs at most one.
6. The injected transport POSTs the zeroizing form to `token_endpoint`:
       grant_type=urn:ietf:params:oauth:grant-type:device_code,
       device_code=<code>, client_id=<id>.
7. The audited response classifier returns one exact transition:
       - "error": "authorization_pending" → retain current interval
       - "error": "slow_down" → increase every later interval by 5 seconds
       - "error": "expired_token" → return ExpiredToken
       - "error": "access_denied" → return Denied
       - tokens present → return Authorized
8. Pending, SlowDown, and transient transport failure return an opaque sequence
   with an absolute next-poll value. On Authorized, a caller-selected exact
   opaque account key can route the response through the existing audited
   credential boundary without releasing token bytes; only its storage
   revision returns.
```

### Refresh Detail

```
1. Agent calls broker.with_access_token("google", &id, |tok| { ... }).
2. Broker reads vault://oauth/google/<id>/metadata.
3. If now() + refresh_lead_time < expires_at:
       Read vault://oauth/google/<id>/access_token; pass to closure.
4. Else:
       Read vault://oauth/google/<id>/refresh_token.
       POST token_endpoint with
         grant_type=refresh_token, refresh_token=<rt>,
         client_id=<id>, client_secret=<sec>.
       Parse response:
         - new access_token, expires_in, scopes
         - new refresh_token (if provider rotates)
       Atomically write new access_token + (possibly) refresh_token
       and updated metadata to the vault.
       Read the new access_token; pass to closure.
5. After closure returns, the local copy of the access token is
   zeroized (the vault still holds it under encryption at rest).
```

If refresh fails with `invalid_grant` (refresh token revoked or
expired), the broker:
- Marks the account as needing re-auth in metadata.
- Returns `OAuthError::ReauthRequired`.
- The agent can either prompt the user or fail the operation.

---

## Capability Cage Integration

Two manifest entries are required for each (agent, provider) pair:

```json
{
  "category": "net",
  "action":   "connect",
  "target":   "<token_endpoint_host>:443",
  "justification": "OAuth refresh + revoke for <provider>"
},
{
  "category": "vault",
  "action":   "read",
  "target":   "oauth/<provider>/<account_id>/access_token",
  "justification": "Read access token for <provider> requests"
}
```

The broker also requires `vault:read|write` on the metadata and
refresh-token paths, but those are namespaced to the broker
(orchestrator-owned), not to the agent. The agent only sees
access tokens.

For first-time authorization (browser flow), the orchestrator
itself needs:

```json
{
  "category": "net",
  "action":   "connect",
  "target":   "<authorization_endpoint_host>:443"
},
{
  "category": "net",
  "action":   "listen",
  "target":   "127.0.0.1:<redirect_port>"
}
```

The redirect port can be a fixed value (per provider, configured)
or chosen at runtime from a small range (53682..53692).

---

## Error Model

```rust
pub enum OAuthError {
    /// Underlying HTTPS call failed.
    Http              { source: HttpsError },
    /// Provider returned a structured error.
    ProviderError     { code: String, description: Option<String> },
    /// State mismatch in authorization callback (CSRF protection).
    StateMismatch,
    /// PKCE verifier didn't validate (provider's complaint).
    PkceFailure,
    /// Refresh token is no longer valid; user must re-authorize.
    ReauthRequired    { account_id: AccountId, reason: String },
    /// Device flow expired before user completed it.
    DeviceFlowExpired,
    /// User denied the consent screen.
    UserDenied,
    /// Stored tokens were tampered with or corrupted.
    TokenStoreCorrupted { detail: String },
    /// Vault read/write failed.
    Vault             { source: VaultError },
    /// Capability denied at the cage.
    CapabilityDenied  { detail: String },
    /// Provider not registered.
    UnknownProvider   (String),
    /// Account not found for this provider.
    UnknownAccount    (AccountId),
    /// Misc parse error in a provider response.
    Parse             { message: String },
}
```

---

## Test Strategy

### Unit Tests

1. **PKCE generation.** `code_verifier` is 32 bytes URL-safe; `code_challenge` is `base64url(sha256(verifier))`; matches RFC 7636 vectors.
2. **State generation.** Cryptographically random; verified on callback; mismatch returns `StateMismatch`.
3. **Authorization URL construction.** For Google, GitHub, Slack — the URL exactly matches a known-good fixture (parameter order normalized).
4. **Token response parsing.** Standard JSON response, GitHub's legacy form-encoded response, missing optional fields, extra unknown fields all parse correctly.
5. **OIDC validation (later slice).** Discovery/JWKS, signature, issuer,
   audience, expiry, and transaction-nonce fixtures all pass before `sub` is
   exposed. The OAuth-only slice does not decode an ID token.
6. **Refresh logic.** Token within lead time → no refresh. Token outside lead time → refresh fires. Rotated refresh token written atomically.
7. **Device-flow polling.** Sequence of `authorization_pending`, `slow_down` (with interval increase), `authorized` works correctly.
8. **Revocation.** Calls the revocation endpoint with correct parameters;
   accepts exact HTTP 200 for both revoked and unknown tokens, rejects 404 as a
   protocol violation, and preserves HTTP 503 as retryable without deleting the
   local credential.

### Integration Tests (gated)

9. **Live Google flow.** With explicit opt-in, run the auth-code flow against Google, store tokens, refresh, then revoke. Requires a real OAuth client config in the test environment.
10. **Live GitHub flow.** Same, against GitHub.
11. **Vault round-trip.** Tokens written; broker restarted; tokens read back; refresh succeeds.

### Provider Compatibility Matrix

For each shipped provider config, a fixture-based test verifies:
- Authorization URL has the right shape.
- Token response parses (using a captured real response).
- Refresh-token rotation behavior matches `refresh_token_rotates`.

### Coverage Target

`>=90%` line coverage for the broker and client logic. Provider-
specific quirks are tested via fixtures.

---

## Trade-Offs

**No unverified JWT identity.** The OAuth-only layer treats an ID token as
opaque. OpenID Connect account identity is deferred until a separate layer can
perform discovery, JWKS signature verification, issuer/audience/expiry checks,
and transaction-nonce validation. TLS authenticity alone is not substituted
for those protocol checks.

**Refresh-token rotation handled per-provider.** Some providers
rotate, some don't. Mistaking a rotating provider for a non-
rotating one means using a stale refresh token and getting locked
out. We tag rotation behavior explicitly per `ProviderConfig` and
update the stored token atomically. If a provider changes its
rotation policy, the config update needs to land before the next
refresh.

**Local HTTP listener for redirect.** The orchestrator binds a
loopback port (typically 53682) for the redirect callback. This
requires `net:listen:127.0.0.1:53682` in its manifest. On systems
where loopback listening is restricted (some corporate sandboxes),
the device flow is the alternative. We document this fallback.

**Single redirect URI per provider.** The provider's registered
redirect URI must match exactly. We do not support per-account
redirect URIs in v1; if a user wants two Google accounts on the
same machine they share the redirect URI (standard OAuth
behavior).

**Tokens are per-account, not per-agent.** Two agents that need
Gmail access for the same account share the access token (mediated
by the broker). The audit log records which agent requested
each token use. This avoids token sprawl but means a compromised
agent that holds a token (briefly, during `with_access_token`'s
closure) could exfiltrate it. Mitigation: keep the closure body
short; revoke on suspected compromise.

**No automatic provider-quirk detection.** We don't ship code that
discovers a provider's behavior at runtime. If GitHub adds device
flow tomorrow, we update `github.json` and ship a new release.

**Public-client model is the default.** `client_secret` is
optional. We assume the orchestrator runs on a user's machine and
should not embed a secret. For server-side workloads (a daemon
running on a VPS), the secret can be loaded from the vault at
provider registration.

---

## Future Extensions

- **OpenID Connect identity.** Fetch and cache discovery/JWKS data; verify the
  ID-token signature, issuer, audience, expiry, and nonce before exposing
  identity claims.
- **Token introspection (RFC 7662).** Useful for resource servers,
  not for clients; deferred.
- **Dynamic Client Registration (RFC 7591).** Self-register OAuth
  clients with providers that support it.
- **DPoP (RFC 9449).** Demonstration of Proof-of-Possession. Binds
  the access token to a key the holder owns; mitigates token
  theft.
- **Token mediation across machines.** A user with multiple devices
  could share refresh tokens via the vault sync engine; agents on
  any device get usable access tokens without re-authorizing.

These are deliberately out of scope for V1.

## Normative References

- RFC 6749 — OAuth 2.0 Authorization Framework.
- RFC 7636 — Proof Key for Code Exchange.
- RFC 8252 — OAuth 2.0 for Native Apps.
- RFC 8414 — Authorization Server Metadata.
- RFC 9207 — Authorization Server Issuer Identification.
- RFC 9700 / BCP 240 — OAuth 2.0 Security Best Current Practice.
