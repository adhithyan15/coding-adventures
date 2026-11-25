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
   before any socket is opened. A strict allocation-free response body-framing
   policy now rejects transfer coding until a reviewed decoder exists, rejects
   duplicate or noncanonical `Content-Length`, binds explicit body limits to
   fixed-length and optionally close-delimited consumption, and handles
   bodyless, HEAD, and CONNECT framing precedence without reading a body. It
   requires request semantics from the exact emitted request and permits
   close-delimited framing only through trusted caller policy, never response
   data. It does not retrofit or authorize the existing permissive `http1` or
   `http1-client` packages; later composition must use a separately reviewed
   strict CRLF response-head decoder and preserve exact request binding before
   applying this policy. It performs no I/O, credential interpretation or
   release, provider action, audit, trace, or external effect. Response syntax
   decoding, transfer decoding, body reads, TLS, sockets, and live-provider
   transport remain unavailable.
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
A bounded `x509-authority-key-identifier` semantic layer now recognizes the
RFC 5280 Authority Key Identifier OID and decodes the exact non-critical,
keyIdentifier-only profile under the generic extension's retained ASN.1 depth
and work budgets. It rejects empty or over-policy identifiers and fails closed
on the currently unsupported authorityCertIssuer and
authorityCertSerialNumber forms. It does not compare identifiers, select or
authenticate an issuer, construct or validate a path, select trust, verify a
signature, process revocation, grant TLS or HTTP authority, or perform an
external effect.

A bounded `x509-subject-key-identifier` semantic layer now recognizes only the
RFC 5280 Subject Key Identifier OID and decodes its exact encapsulated OCTET
STRING without allocation. A caller-selected nonzero byte ceiling is checked
after canonical DER framing, empty identifiers fail closed, and stable errors
retain no identifier bytes. This primitive does not derive an identifier,
parse Authority Key Identifier, select or authenticate an issuer, construct or
validate a path, select trust, verify signatures, process revocation, grant TLS
or HTTP authority, or perform any external effect.
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
A bounded `x509-name-value-policy` layer now validates at most 64
caller-supplied attribute-type OID rules and binds each exact identifier to
either canonical DER element equality or exact Unicode-scalar equality across
the supported DirectoryString encodings. It preflights every attribute in both
bounded Names before structural comparison, so an unconfigured attribute type
fails closed. Under the DirectoryString-scalar rule, malformed values and
TeletexString without a repository-owned T.61 mapping also fail closed with a
payload-free error; the exact-DER rule intentionally treats its complete value
element as opaque. The policy removes executable value callbacks from later
issuer-link and path-construction callers while keeping the registry data
driven. It embeds no attribute OID registry and does not perform RFC 4518
StringPrep, normalization, case folding, insignificant-space processing, or
schema-specific matching, so it is not complete RFC 5280 Name equality and is
not yet suitable to authorize concrete path validation.
The pure public-certificate-data operation performs no credential access or
editing, signing, storage, clock acquisition, provider action, audit event,
trace emission, I/O, network access, or other external effect.
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
extension. A bounded policy-data layer validates at most 256 recognized OIDs
before certificate processing, rejects structurally invalid or duplicate OID
arc sequences, permits an empty fail-closed set, and retains the exact accepted
set beside non-copyable certificate-bound role evidence. OID matching is exact;
policy membership is still recognition only and does not decode or validate an
extension value, grant a certificate role, or establish trust. A second bounded
gate now composes the shipped Basic Constraints, Key Usage, and Extended Key
Usage decoders under one explicit semantic budget. It requires critical `cA = TRUE` plus
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
A bounded `x509-rsa-algorithm-policy` composition now binds that exact RSA key
validation to existing data-driven certificate algorithm-profile evidence.
The SubjectPublicKeyInfo is derived only from the exact decoded certificate
retained by the non-copyable profile result; callers separately supply the RSA
modulus range, exponent allowlist, and nested ASN.1 budget. Success retains the
selected profile, exact certificate object, and RSA key borrowed from that
certificate, preventing a detached or independently decoded key from being
substituted. The pure public-data composition performs no credential access,
signing, storage, clock acquisition, audit, provider, I/O, or external effect.
It does not choose a signature verifier, authenticate the certificate, assign
a role, construct or validate a path, establish trust, check names or
revocation, perform TLS or HTTP, authorize OAuth transport, or grant network
authority. Concrete HTTPS remains unavailable.
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
A bounded `rsa-pss-sha256` prerequisite now validates one complete recovered
RFC 8017 EMSA-PSS encoded message for the fixed SHA-256, 32-byte-salt profile.
It composes the existing public layout gate and bounded SHA-256 MGF1 primitive,
unmasks `DB` into exact caller-owned scratch space, clears only the declared
unused high bits, requires the complete zero padding and `0x01` delimiter, and
recomputes `H = SHA-256(0x00^8 || mHash || salt)` before a full 32-byte
comparison. Public size and scratch failures occur before scratch mutation;
stable failures retain no message, digest, salt, padding, or offset bytes.

A bounded `rsa-pss-sha384` prerequisite now validates one complete recovered
RFC 8017 EMSA-PSS encoded message for the fixed SHA-384, 48-byte-salt profile.
It composes the public layout gate and bounded SHA-384 MGF1 primitive, unmasks
`DB` into exact caller-owned scratch space, clears only the declared unused
high bits, requires the complete zero padding and `0x01` delimiter, and
recomputes `H = SHA-384(0x00^8 || mHash || salt)` before a full 48-byte
comparison. Public size and scratch failures occur before scratch mutation;
stable failures retain no message, digest, salt, padding, or offset bytes.

A bounded `rsa-pss-sha384-signature` composition now verifies one exact public
RSASSA-PSS signature for the fixed SHA-384, MGF1-SHA-384, 48-byte-salt
profile. It accepts only a previously validated RSA public key, an exact
modulus-width signature, an already-computed 48-byte message digest, exact
caller-owned DB scratch, and caller-owned RSA and encoded-message ceilings. It
derives `emBits = modBits - 1`, applies the repository-owned RSA public
operation, and requires the complete fixed-profile EMSA-PSS verdict. Its
borrowed non-copyable evidence binds the exact key, signature bytes, and
message digest used.

This signature evidence does not hash a message, parse RSA-PSS parameters or a
certificate, choose an algorithm or issuer, establish certificate roles or a
path, select trust roots, check time, names, or revocation, process TLS or HTTP,
authorize OAuth transport, or perform any external effect. It is not evidence
of certificate, HTTPS, or live-provider readiness.

A bounded, data-driven `tls13-rsa-pss-signature-routing` prerequisite now
routes one structurally decoded TLS 1.3 `CertificateVerify` signature to the
fixed SHA-256, SHA-384, or SHA-512 RSASSA-PSS verifier. It requires a signature
scheme already authorized by caller policy, the message body's exact matching
wire identifier, a previously validated `rsaEncryption` public key, and an
already-computed digest whose typed length matches the authorized scheme.
Mismatched wire identifiers and digest profiles fail before RSA work or DB
scratch mutation. Non-copyable success evidence retains the exact authorized
scheme, message, public key, digest, and fixed-verifier evidence used.

This routing evidence does not construct or hash the RFC 8446
`CertificateVerify` signed input, prove which transcript produced the supplied
digest, parse a certificate, bind a key to a validated server path, build or
validate a path, verify a hostname, select trust roots, check time or
revocation, sequence a handshake, process records, perform I/O, authorize HTTP
or OAuth transport, or open a network connection. Those transcript,
certificate, trust, revocation, handshake, and transport boundaries remain
separate; concrete HTTPS and live-provider readiness remain unavailable.

A bounded `tls13-rsa-pss-certificate-verify` composition now closes the
caller-digest substitution gap for one RSA-PSS `CertificateVerify` operation.
Its first stage consumes an already policy-authorized signature scheme and one
exact framed RFC 8446 `CertificateVerify` signed input, hashes those exact
bytes with the scheme-bound repository-owned SHA-256, SHA-384, or SHA-512
primitive, and retains the authorization, input, and owned digest together in
non-copyable evidence. Its second stage accepts only that evidence, the exact
structurally decoded `CertificateVerify` message, a validated `rsaEncryption`
public key, matching typed verifier limits, and caller-owned scratch, then
delegates to the existing data-driven RSA-PSS routing verifier. Success retains
both the exact hashed-input evidence and fixed-verifier routing evidence, so a
different input, scheme, message, key, or digest cannot be substituted after
verification. Diagnostics retain no signed-input, transcript-hash, signature,
digest, key, limit, or scratch bytes.

This composition does not prove which handshake transcript produced the hash
embedded in the framed signed input, bind that transcript to a selected cipher
suite or ordered handshake state, parse a certificate, bind the key to a
validated server path, construct or validate a path, verify a hostname, select
trust roots, check time or revocation, authenticate Finished, sequence a
handshake, process or protect records, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. Those transcript, certificate,
trust, revocation, handshake, and transport boundaries remain separate;
concrete HTTPS and live-provider readiness remain unavailable.

A bounded `tls13-certificate-verify-transcript-binding` composition now closes
the detached-transcript-hash and detached-transcript-algorithm gap for that
operation. It accepts one immutable selected repository transcript state, one
caller-selected CertificateVerify role, caller-owned output, and one already
policy-authorized RSA-PSS scheme. The composition snapshots the transcript
internally, maps only its suite-bound SHA-256 or SHA-384 algorithm to the exact
framing metadata, constructs the role-specific RFC 8446 signed input, and then
delegates hashing to `tls13-rsa-pss-certificate-verify`. Non-copyable evidence
retains the selected transcript state, exact snapshot, role-bound framed input,
authorized signature scheme, and owned scheme-bound digest together. The API
accepts neither detached transcript bytes nor caller-selected transcript-hash
metadata, and the retained immutable transcript borrow prevents that state from
advancing while the evidence is live.

This evidence does not prove that the caller fed the selected transcript the
right ordered handshake messages or stopped at the required handshake state,
authorize the caller-selected client or server role, bind a certificate or
validated path key, authenticate Finished, sequence a handshake, process or
protect records, perform I/O, authorize HTTP or OAuth transport, or open a
network connection. Those ordered-state, role, certificate, trust, revocation,
handshake, and transport boundaries remain separate; concrete HTTPS and
live-provider readiness remain unavailable.

A follow-up verification stage in that composition now accepts only the outer
transcript-bound hash evidence, the exact decoded `CertificateVerify` message,
one already validated RSA public key, matching typed verifier limits, and
caller-owned scratch. It delegates to the existing repository-owned RSA-PSS
certificate verifier and returns non-copyable success evidence retaining both
the outer selected-transcript proof and the complete lower signature-verdict
proof. Callers therefore cannot obtain this stronger verdict from detached
framing or hash evidence, and the returned proof keeps the selected transcript,
role-specific input, authorized scheme, message, key, and signature verdict in
one lifetime-bound graph.

The public key is still only algorithm- and strength-validated: this stage does
not bind it to an authenticated certificate or reviewed server path. It also
does not prove ordered handshake state, authorize the selected role, check
Finished, sequence a handshake, process records, perform I/O, or authorize TLS,
HTTP, OAuth transport, or live-provider access. Those state, role, certificate,
trust, revocation, handshake, and transport boundaries remain separate;
concrete HTTPS remains unavailable.

A consuming form of the same composition now advances an owned selected
transcript only after authenticating one exact decoded `CertificateVerify`
body. It accepts the exact body bytes, decodes them internally, verifies their
signature against the transcript snapshot captured before the message, frames
those same body bytes as one complete `CertificateVerify` handshake message in
caller-owned storage, and updates the owned transcript once with that complete
encoding. Success returns non-copyable evidence retaining the owned advanced
transcript, before and after snapshots, exact body, authorized scheme, role,
validated RSA key, and encoded length. Every failure consumes the transcript
and returns only a stable redacted category, so partially authenticated state
cannot be retried or reused by the caller.

This transition still accepts an algorithm- and strength-validated detached
RSA key and a caller-selected role. It does not bind the key to a reviewed
certificate path, prove the preceding server-flight order, authenticate or
append Finished, establish trust, enforce revocation, protect records, perform
I/O, authorize HTTP or OAuth transport, or open a network connection. Exact
reviewed-path and Finished continuity remain later compositions; concrete
HTTPS and live-provider readiness remain unavailable.

A follow-up consuming transition now authenticates Finished directly from that
owned post-`CertificateVerify` transcript. The Finished key's sender role must
match the client/server role already retained by the authenticated
`CertificateVerify` evidence, and the existing Finished transcript binding
requires exact selected-suite equality before HMAC work. Success returns one
non-copyable graph retaining the original exact body, role, authorized scheme,
validated RSA key, pre- and post-`CertificateVerify` snapshots, owned selected
transcript, internally captured Finished snapshot, and repository-owned
Finished verdict. The Finished snapshot is required to equal the retained
post-`CertificateVerify` snapshot. Every failure consumes the earlier evidence
and returns no transcript state for retry or reuse.

This proves continuity only between the detached-key `CertificateVerify`
transition and the immediately following role- and suite-bound Finished check.
It does not bind the RSA key to a reviewed certificate path, prove preceding
server-flight order, append the Finished handshake encoding, establish trust,
enforce revocation, protect records, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. It accesses no credential and causes
no edit, signing operation, provider or trace event, storage, clock, network,
or other external effect. Those compositions remain blockers; concrete HTTPS
and live-provider readiness remain unavailable.

A reviewed-path consuming entry point now closes the detached-certificate-key
and caller-selected-`CertificateVerify`-role gaps for exact
`CertificateVerify`-to-Finished-key transcript continuity. It accepts the
strongest accumulated reviewed RSA server-path proof, takes ownership of one
selected transcript, derives and validates the target certificate's
SubjectPublicKeyInfo only through that retained path, and fixes the
authenticated signature role to `Server`. It then uses the existing consuming
transitions to authenticate the exact decoded body against the pre-message
transcript, append that same body as one complete handshake encoding, and
verify peer verify-data with one caller-supplied Finished key over the
resulting post-message transcript. Non-copyable success evidence retains the
complete reviewed path together with the exact body, authorized scheme,
path-derived certificate key, before and after snapshots, owned transcript,
Finished snapshot, and repository-owned Finished verdict. Every failure
consumes the transcript and returns only a stable redacted category, so no
partial state is returned for substitution or retry.

The Finished key remains detached: role and selected-suite checks do not bind
it to this handshake's ECDHE result, handshake traffic secret, or key-schedule
evidence, and the result retains no such provenance. This bounded composition
also does not prove the preceding server-flight order, append the Finished
handshake encoding, verify the reviewed anchor's self-signature, select or
enforce revocation, bind an HTTPS destination, protect records, perform I/O,
authorize HTTP or OAuth transport, or open a network connection. It accesses
no credential and causes no edit, signing operation, provider or trace event,
storage, clock, network, or other external effect. Those key-schedule,
ordered-state, anchor, revocation, destination, record, and transport
compositions remain blockers; concrete HTTPS and live-provider readiness
remain unavailable.

A bounded `tls13-reviewed-server-path-certificate-verify` composition closes
the detached-key and caller-selected-role gaps for one reviewed RSA server
path. It accepts only the strongest accumulated reviewed server-path proof,
outer transcript-bound hash evidence, the exact decoded `CertificateVerify`
message, data-driven RSA key policy, matching typed verifier limits, and
caller-owned scratch. Before RSA work or scratch mutation it requires the
retained RFC 8446 input role to be `Server`. It then derives the target
certificate exclusively through the retained reviewed path, validates that
certificate's exact SubjectPublicKeyInfo under the supplied RSA key policy,
and delegates verification with that derived key. Non-copyable success
evidence retains both the complete reviewed path and the outer
selected-transcript signature verdict, so a detached certificate, key, role,
path, transcript, message, or signature cannot be substituted after success.

This composition does not prove that the selected transcript contains the
right ordered handshake messages or stopped at the required state, verify the
reviewed anchor's self-signature, select a revocation outcome, authenticate
Finished, sequence a handshake, process or protect records, perform I/O, or
authorize TLS, HTTP, OAuth transport, or live-provider access. Those remaining
anchor, revocation, state, Finished, record, and transport boundaries remain
separate; concrete HTTPS remains unavailable.

The same package now exposes a stronger exact-destination entry point. It
accepts the reviewed server path only through the existing non-copyable HTTPS
DNS destination binding, then delegates the same role-first exact-leaf
`CertificateVerify` verification. Success retains the complete certificate
SAN, TLS SNI, HTTP Host authority, reviewed path, selected transcript, decoded
message, derived key, and signature verdict in one lifetime-bound graph. This
closes substitution between a destination-proven leaf and the leaf used for
handshake authentication.

The stronger entry point still does not add anchor self-signature evidence,
select or enforce revocation, prove ordered transcript state, authenticate
Finished, protect records, perform TLS or HTTP I/O, authorize OAuth transport,
or grant network authority. Those trust, revocation, state, record, and
transport boundaries remain separate; concrete HTTPS and live-provider
readiness remain unavailable.

A still stronger entry point now accepts only the exact HTTPS DNS destination
and reviewed server path already bound to the reviewed anchor's
repository-owned self-signature proof. It performs the same role-first exact
leaf `CertificateVerify` verification and retains the destination, complete
reviewed path, reviewed root selection, anchor self-signature verdict,
selected transcript, decoded message, derived key, and server signature
verdict together. This closes substitution between the root whose
self-signature was verified and the destination-bound path used for handshake
authentication; the entry point itself neither loads trust roots nor repeats
anchor verification.

This composition still does not select or enforce revocation, prove ordered
transcript state, authenticate Finished, protect records, perform TLS or HTTP
I/O, authorize OAuth transport, or grant network authority. Those revocation,
state, record, and transport boundaries remain separate; concrete HTTPS and
live-provider readiness remain unavailable.

An orthogonal direct-revocation entry point now accepts only the exact HTTPS
DNS destination already bound to the complete direct no-check fixed-path
revocation proof. After the same server-role gate, it derives the leaf through
that destination, validates the exact SubjectPublicKeyInfo under caller policy,
verifies the selected-transcript `CertificateVerify`, and retains the complete
destination and revocation proof with the exact signature verdict. This closes
substitution between the leaf covered by complete direct revocation evidence
and the leaf key used for handshake authentication. The entry point does not
repeat revocation evaluation or acquire, refresh, or load revocation data.

The direct entry point does not admit the existing nested, double-nested,
recursive, or alternative intermediate-status-responder completion topologies.
It also does not prove ordered transcript state, authenticate Finished,
protect records, perform TLS or HTTP I/O, authorize OAuth transport, or grant
network authority. Those remaining revocation-topology, state, record, and
transport boundaries remain separate; concrete HTTPS and live-provider
readiness remain unavailable.

A second revocation entry point now accepts only the exact HTTPS DNS
destination already bound to the complete one-level nested no-check fixed-path
revocation proof. Its private sealed admission has exactly one production
implementation: the repository-owned nested destination/revocation boundary.
After the same server-role gate it derives the leaf through that retained
destination, validates the exact SubjectPublicKeyInfo under caller policy,
verifies the selected-transcript `CertificateVerify`, and retains the complete
nested proof with the exact signature verdict. This closes substitution
between the server leaf covered by the nested completion and the key used for
handshake authentication without repeating revocation evaluation.

The nested entry point does not admit the existing double-nested, recursive,
or alternative intermediate-status-responder completion topologies. It does
not acquire or refresh status, prove ordered transcript state, authenticate
Finished, protect records, perform TLS or HTTP I/O, authorize OAuth transport,
or grant network authority. Those remaining revocation-topology, state,
record, and transport boundaries remain separate; concrete HTTPS and live-
provider readiness remain unavailable.

A third revocation entry point now accepts only the exact HTTPS DNS destination
already bound to the complete double-nested no-check fixed-path revocation
proof. Its private sealed admission has exactly one production implementation:
the repository-owned double-nested destination/revocation boundary. After the
same server-role gate it derives the leaf through that retained destination,
validates the exact SubjectPublicKeyInfo under caller policy, verifies the
selected-transcript `CertificateVerify`, and retains the complete double-
nested proof with the exact signature verdict. This closes substitution
between the server leaf covered by the double-nested completion and the key
used for handshake authentication without repeating revocation evaluation.

The double-nested entry point does not admit the existing recursive or
alternative intermediate-status-responder completion topologies. It does not
acquire or refresh status, prove ordered transcript state, authenticate
Finished, protect records, perform TLS or HTTP I/O, authorize OAuth transport,
or grant network authority. Those remaining revocation-topology, state,
record, and transport boundaries remain separate; concrete HTTPS and live-
provider readiness remain unavailable.

A fourth revocation entry point now accepts only the exact HTTPS DNS
destination already bound to the complete recursive-responder no-check fixed-
path revocation proof. Its private sealed admission has exactly one production
implementation: the repository-owned recursive destination/revocation
boundary. After the same server-role gate it derives the leaf through that
retained destination, validates the exact SubjectPublicKeyInfo under caller
policy, verifies the selected-transcript `CertificateVerify`, and retains the
complete recursive proof with the exact signature verdict. This closes
substitution between the server leaf covered by the recursive completion and
the key used for handshake authentication without repeating revocation
evaluation.

The recursive entry point does not admit the existing alternative
intermediate-status-responder completion topology. It does not acquire or
refresh status, prove ordered transcript state, authenticate Finished, protect
records, perform TLS or HTTP I/O, authorize OAuth transport, or grant network
authority. Those remaining revocation-topology, state, record, and transport
boundaries remain separate; concrete HTTPS and live-provider readiness remain
unavailable.

A fifth revocation entry point now accepts only the exact HTTPS DNS destination
already bound to the complete alternative intermediate-status-responder
revocation responder no-check fixed-path proof. Its private sealed admission
has exactly one production implementation: the repository-owned alternative
destination/revocation boundary. After the same server-role gate it derives
the leaf through that retained destination, validates the exact
SubjectPublicKeyInfo under caller policy, verifies the selected-transcript
`CertificateVerify`, and retains the complete alternative proof with the exact
signature verdict. This closes substitution between the server leaf covered
by the alternative completion and the key used for handshake authentication
without repeating revocation evaluation.

This entry point completes the five existing destination-bound complete
revocation topologies; it does not generalize path construction or revocation
policy. It does not acquire or refresh status, prove ordered transcript state,
authenticate Finished, protect records, perform TLS or HTTP I/O, authorize
OAuth transport, or grant network authority. Those remaining path, state,
record, and transport boundaries remain separate; concrete HTTPS and live-
provider readiness remain unavailable.

A bounded ordered-transition follow-up accepts only one of those five sealed
destination-bound complete-revocation `CertificateVerify` verdict types and a
no-PSK server flight already positioned at `ExpectCertificateVerify`. Success
advances that exact flight to `ExpectFinished` and returns non-copyable evidence
retaining a mutable borrow of the flight plus the exact signature verdict. A
wrong position fails through the existing ordering policy without mutation,
and an accepted `CertificateRequest` requirement remains attached to the
flight.

This transition does not remove the lower-level public message-type ordering
API, link the flight's earlier accepted types to one encoded handshake stream,
or prove that the retained verification transcript was advanced by those exact
accepted messages. It does not authenticate Finished, enforce a later client-
authentication response, protect records, perform I/O, authorize OAuth
transport, or grant network authority. It accesses no credential and causes no
editing, signing, provider or trace event, storage, clock, network, or other
external effect. Ordered transcript composition, Finished authentication,
record protection, and transport therefore remain separate blockers; concrete
HTTPS and live-provider readiness remain unavailable.

A bounded follow-up consumes that exact ordered `CertificateVerify` result
together with one repository-owned `VerifiedFinished`. It requires the
Finished sender role to be server and its selected cipher suite to equal the
suite retained through the exact sealed `CertificateVerify` verdict before it
mutates the flight. Success accepts `Finished`, completes that exact flight,
preserves any accepted client-authentication requirement, and returns non-
copyable evidence retaining the complete-revocation signature verdict and
exact Finished verdict. Role or suite substitution fails before flight
mutation with a stable payload-free category.

This transition consumes an already-produced Finished verification result. It
does not prove that the verdict's retained transcript hash was advanced from
the exact retained `CertificateVerify` message or the earlier accepted message
types in one encoded handshake stream. It does not enforce a later client-
authentication response, bind record epochs, protect records, perform I/O,
authorize OAuth transport, or grant network authority. It accesses no
credential and causes no editing, signing, provider or trace event, storage,
clock, network, or other external effect. Exact ordered transcript ownership,
record protection, and transport remain separate blockers; concrete HTTPS and
live-provider readiness remain unavailable.

A bounded `rsa-pss-sha512` prerequisite now validates one complete recovered
RFC 8017 EMSA-PSS encoded message for the fixed SHA-512, 64-byte-salt profile.
It composes the public layout gate and bounded SHA-512 MGF1 primitive, unmasks
`DB` into exact caller-owned scratch space, clears only the declared unused
high bits, requires the complete zero padding and `0x01` delimiter, and
recomputes `H = SHA-512(0x00^8 || mHash || salt)` before a full 64-byte
comparison. Public size and scratch failures occur before scratch mutation;
stable failures retain no message, digest, salt, padding, or offset bytes.

A bounded `rsa-pss-sha512-signature` composition now verifies one exact public
RSASSA-PSS signature for the fixed SHA-512, MGF1-SHA-512, 64-byte-salt
profile. It accepts only a previously validated RSA public key, an exact
modulus-width signature, an already-computed 64-byte message digest, exact
caller-owned DB scratch, and caller-owned RSA and encoded-message ceilings. It
derives `emBits = modBits - 1`, applies the repository-owned RSA public
operation, and requires the complete fixed-profile EMSA-PSS verdict. Its
borrowed non-copyable evidence binds the exact key, signature bytes, and
message digest used.

This signature evidence does not hash a message, parse RSA-PSS parameters or a
certificate, choose an algorithm or issuer, establish certificate roles or a
path, select trust roots, check time, names, or revocation, process TLS or HTTP,
authorize OAuth transport, or perform any external effect. It is not evidence
of certificate, HTTPS, or live-provider readiness.

This encoded-message verdict does not parse RSA-PSS AlgorithmIdentifier
parameters or a key, perform RSAVP1, bind the recovered bytes to a signature or
certificate, choose a TLS signature scheme, process a handshake, validate a
certificate path or hostname, select trust, check revocation, perform I/O,
authorize HTTP or OAuth transport, or open a network connection. RSA-PSS
certificate and TLS verification remain unavailable until those separate
boundaries are composed and tested; concrete HTTPS remains unavailable.

A bounded `rsa-public-operation` prerequisite now consumes only a previously
validated RSA public key, enforces a separate caller-owned modulus-byte ceiling
before arbitrary-precision conversion, rejects non-modulus-width or
out-of-range signature representatives, and recovers the exact modulus-width
RSAVP1 encoded message with repository-owned arithmetic. It adds no private-key
operation, hashing, padding interpretation, certificate-signature verdict,
path trust, or transport readiness; it is not by itself a verifier.
A bounded `rsa-pss-sha256-signature` composition now verifies one exact public
RSASSA-PSS signature for the fixed SHA-256, MGF1-SHA-256, 32-byte-salt profile.
It accepts only a previously validated RSA public key, an exact modulus-width
signature, an already-computed 32-byte message digest, exact caller-owned DB
scratch, and caller-owned RSA and encoded-message ceilings. It derives
`emBits = modBits - 1`, applies the repository-owned RSA public operation, and
requires the complete fixed-profile EMSA-PSS verdict. Its borrowed non-copyable
evidence binds the exact key, signature bytes, and message digest used.

This signature evidence does not hash a message, parse RSA-PSS parameters or a
certificate, choose an algorithm or issuer, establish certificate roles or a
path, select trust roots, check time, names, or revocation, process TLS or HTTP,
authorize OAuth transport, or perform any external effect. It is not evidence
of certificate, HTTPS, or live-provider readiness.
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
A bounded, data-driven `rsa-pss-data-block` prerequisite now validates the
public unmasked data block consumed by a future RFC 8017 EMSA-PSS verifier.
Caller policy supplies the selected salt length and a data-block byte ceiling.
The gate requires the exact canonical `DB = PS || 0x01 || salt` structure,
proves every `PS` octet is zero under that ceiling, and returns borrowed
non-copyable evidence retaining the complete block and exact selected salt.

This data-block evidence does not run MGF1, clear unused high bits, hash the
message or `M'`, compare `H`, parse or operate on an RSA key, apply RSAVP1,
select an algorithm, or establish a signature verdict. Certificate processing,
path and hostname validation, trust roots, revocation, TLS, HTTP, OAuth
transport, and network effects remain unavailable.

A bounded, data-driven `rsa-pss-algorithm-policy` prerequisite now validates
already-decoded public RSA-PSS parameters. Caller policy supplies an exact
allowlist of message-hash, MGF1-hash, and salt-length profiles plus a salt-work
ceiling. The gate requires RFC 8017 trailer field 1, applies the ceiling, and
returns non-copyable evidence only for an exact allowlist match. SHA-256,
SHA-384, and SHA-512 routing remains explicit; the policy does not silently
couple the message and MGF1 hash choices.

This profile evidence does not parse DER or X.509 AlgorithmIdentifier values,
select an implementation, hash, run MGF1, validate PSS encoding, parse or
operate on an RSA key, apply RSAVP1, or establish a signature verdict.
Certificate processing, path and hostname validation, trust roots, revocation,
TLS, HTTP, OAuth transport, and network effects remain unavailable.
A bounded, data-driven `rsa-pss-hash-match` prerequisite now validates the
public hash-field comparison consumed by a future RFC 8017 EMSA-PSS verifier.
Caller policy supplies the selected digest length and comparison ceiling. The
gate requires complete recomputed `H'` and encoded `H` values at that exact
length, consumes every selected byte without reporting a mismatch position,
and returns borrowed non-copyable evidence only when the values match.

This hash-match evidence does not construct or hash `M'`, run MGF1, validate a
PSS data block, parse or operate on an RSA key, apply RSAVP1, select an
algorithm, or establish a signature verdict. Certificate processing, path and
hostname validation, trust roots, revocation, TLS, HTTP, OAuth transport, and
network effects remain unavailable. It makes no constant-time guarantee.


A bounded, data-driven `rsa-pss-data-block-unmask` prerequisite now performs
the public byte transformation consumed by a future RFC 8017 EMSA-PSS
verifier. Only after complete `maskedDB`, MGF1-mask, output-length,
unused-high-bit, and caller-owned byte-ceiling checks pass does it write the
complete XOR into caller-owned memory and clear the high bits required by
`emBits`. It returns borrowed non-copyable evidence for that exact output.

This unmask evidence does not run MGF1, validate `PS || 0x01 || salt`, hash a
message or `M'`, compare `H`, parse or operate on an RSA key, apply RSAVP1,
select an algorithm, or establish a signature verdict. Certificate processing,
path and hostname validation, trust roots, revocation, TLS, HTTP, OAuth
transport, and network effects remain unavailable. It makes no constant-time
guarantee.

A bounded `x509-rsassa-pss-parameters` prerequisite now decodes the public
RFC 4055 / RFC 8017 parameter structure carried by an X.509 signature
AlgorithmIdentifier. It requires `id-RSASSA-PSS` with present parameters,
handles the four explicitly tagged fields in canonical order, applies omitted
defaults, accepts the specified SHA identifier parameter forms, and recognizes
the explicit SHA-1, MGF1-SHA-1, and trailer-field-1 defaults required for RFC
4055 signature-validation compatibility. It rejects an explicit default salt
length under DER omission rules and preserves independent message hash, MGF1
hash, salt length, and trailer field values for later policy.

This decoded structure grants no algorithm approval, implementation routing,
key authority, signature verdict, certificate role, path or hostname
validation, trust root, revocation, TLS, HTTP, OAuth transport, or network
effect. Concrete HTTPS remains unavailable.


A bounded, data-driven `rsa-pss-message-prime` prerequisite now constructs the
public hash input consumed by a future RFC 8017 EMSA-PSS verifier. Caller policy
supplies the selected `mHash` length, salt length, and an `M'` byte ceiling.
Only after exact input and output lengths pass does the encoder write the
canonical `M' = zeroes(8) || mHash || salt` bytes into caller-owned memory and
return borrowed non-copyable evidence for the exact output and copied fields.

This `M'` evidence does not hash the message or `M'`, compare `H`, run MGF1,
validate a PSS data block, parse or operate on an RSA key, apply RSAVP1, select
an algorithm, or establish a signature verdict. Certificate processing, path
and hostname validation, trust roots, revocation, TLS, HTTP, OAuth transport,
and network effects remain unavailable.
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
A bounded sibling `x509-rsa-pkcs1-v1-5-sha384-signature` composition now binds
the SHA-384 encoded-message prerequisite to one exact decoded certificate, an
already policy-validated issuer RSA key, the bounded repository-owned RSA
public operation, and the repository-owned SHA-384 implementation. It requires
the canonical `sha384WithRSAEncryption` identifier with present NULL
parameters and a byte-aligned signature, hashes the preserved
`TBSCertificate` DER, and returns non-copyable evidence retaining the exact
certificate and issuer key only after the complete canonical encoded message
passes. The pure public-data verifier performs no credential access, signing,
storage, clock acquisition, or external effect, so it emits no runtime audit
event. Issuer selection and linkage, data-driven algorithm routing,
certificate roles, path construction and validation, trust roots, time,
names, revocation, TLS, HTTP, and network effects remain separate blockers;
concrete HTTPS remains unavailable.
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
A bounded sibling `x509-rsa-sha384-issuer-link` composition now authenticates
one directed child-to-candidate-issuer link for the exact RSA PKCS #1 v1.5
SHA-384 algorithm. It invokes caller-owned Name policy exactly once in the
child-issuer to candidate-subject direction, requires the exact candidate to
pass issuing-CA role checks and caller-supplied RSA key-strength policy, and
then verifies the child's preserved signed bytes with the merged repository-
owned SHA-384 certificate verifier. Non-copyable evidence binds the exact
child, candidate issuer, and validated key. This fixed-algorithm slice performs
no credential access, signing, storage, audit or trace emission, I/O, or
external effect. It does not route algorithms, process all critical
extensions, validate time or path length, apply name constraints, select or
validate trust, check revocation, perform TLS or HTTP, or authorize OAuth
transport. Concrete HTTPS remains unavailable.
A stronger bounded entry point now requires the exact SHA-384 candidate issuer
to be retained by both certificate-wide critical-extension/issuing-CA role
evidence and certificate-bound, data-driven RSA algorithm/key-policy evidence
before any Name comparison or signature work begins. Object identity, rather
than encoded-byte equality, binds both non-copyable inputs to that exact
candidate. The existing fixed RSA PKCS #1 v1.5 SHA-384 verifier then
authenticates only the supplied child-to-issuer pair with the already validated
key, and success retains the role, algorithm, key, and signature evidence
together. The algorithm profile remains caller policy data and does not route
to or enable the fixed verifier. This pure public-data composition performs no
credential access, signing, storage, clock acquisition, audit event, trace
emission, or external effect. It does not validate issuer time, path length,
name constraints, a complete path, trust, revocation, TLS, HTTP, OAuth
transport, or network authority; concrete HTTPS remains unavailable.
A bounded complete-intermediate-policy follow-up now consumes that exact
policy-bound RSA SHA-384 issuer link together with complete validated
intermediate policy. It fails closed unless the candidate issuer that verified
the child signature and the certificate retained by the complete intermediate
policy are the same exact decoded object; byte-equal independent decodes are
deliberately insufficient. Success returns distinct non-copyable evidence
retaining both complete inputs, including the intermediate's certificate-wide
critical-extension and issuing-CA role policy, data-driven algorithm policy,
self-issued classification, and explicit-time validity result. This pure
public-data binding performs no new parsing or cryptography, does not advance
path-length state or evaluate decoded NameConstraints, authenticate the
intermediate's issuer, construct or validate a complete path, grant trust,
check revocation, perform TLS or HTTP, authorize OAuth transport, or cause an
external effect. Concrete HTTPS and live-provider readiness remain
unavailable.
A bounded `x509-rsa-sha384-reviewed-anchor-link` composition now consumes one
exact policy-bound RSA SHA-384 issuer-link proof and one exact reviewed
Mozilla-root selection. It accepts them only when both non-copyable values
retain the identical decoded issuer certificate object; byte-equal independent
decodes fail closed. Success retains the complete certificate-wide critical-
extension/issuing-CA role evidence, data-driven RSA algorithm/key evidence,
fixed SHA-384 signature proof, and reviewed subject-plus-SPKI membership.
This is an exact evidence join only. It does not apply anchor name constraints,
construct or validate a path, check target identity, validity, purpose, path
length, or revocation, perform TLS or HTTP, emit audit or trace events, access
credentials, or authorize OAuth transport or network effects. Concrete HTTPS
and live-provider readiness remain unavailable.
A bounded two-link follow-up now consumes one policy-bound RSA SHA-384
target-to-intermediate link together with one policy-bound SHA-384
intermediate-to-reviewed-root link. It fails closed unless the candidate
issuer retained by the first proof and the child retained by the reviewed
parent proof are the same exact decoded intermediate certificate object;
byte-equal independent decodes are deliberately insufficient. Success returns
distinct non-copyable evidence retaining both complete authenticated links and
the exact reviewed-root selection. This is one caller-ordered fixed-algorithm
evidence join only. It does not derive or advance path-length state, construct
or search a path, apply anchor or intermediate name constraints, validate the
target's role, purpose, time, or hostname, check revocation, perform TLS or
HTTP, authorize OAuth transport, or cause an external effect. Concrete HTTPS
and live-provider readiness remain unavailable.
A bounded `x509-rsa-sha384-reviewed-server-link` composition now consumes one
exact target-server policy result together with one authenticated RSA SHA-384
issuer link terminating at a reviewed root. It fails closed unless both
non-copyable inputs retain the same exact decoded target certificate object,
then retains the complete target policy and reviewed-link evidence together.
Encoded-byte equality is deliberately insufficient, preventing independently
decoded certificates from contributing mixed evidence. This pure public-data
binding performs no credential access, signing, storage, clock acquisition,
audit event, trace emission, or external effect. It performs no new parsing or
signature work, applies no reviewed-anchor constraints, constructs no
multi-hop path, and does not check revocation, perform TLS or HTTP, authorize
OAuth transport, or open a network connection; concrete HTTPS remains
unavailable.
A bounded follow-up now applies the exact selected reviewed anchor's optional
DNS NameConstraints bytes to every dNSName in that retained SHA-384 target's
Subject Alternative Name. Separate caller-owned ASN.1 budgets bound the
reviewed-root constraint decoder and target SAN decoder. An unconstrained
selected anchor succeeds without manufacturing DNS evidence; malformed
constraints, a missing or malformed target SAN, unsupported constraint forms,
and names outside the permitted subtrees fail with payload-free categories.
The non-copyable result retains the complete SHA-384 reviewed server-link proof
and optional constrained-name evidence together, preventing another target or
snapshot entry from being substituted after evaluation. This remains one
direct fixed-algorithm link and one supported DNS-policy subset: it does not
construct or validate a multi-hop path, apply intermediate or IP constraints,
check revocation or hostnames, perform TLS or HTTP, authorize OAuth transport,
or open a network connection; concrete HTTPS remains unavailable.
A bounded sibling `x509-rsa-sha512-issuer-link` composition now authenticates
one directed child-to-candidate-issuer link for the exact RSA PKCS #1 v1.5
SHA-512 certificate algorithm. Caller-owned Name policy is invoked exactly once
in the fixed child-issuer to candidate-subject direction; that same candidate
must pass the algorithm-independent issuing-CA role gate and caller-supplied
RSA key-strength policy before its key verifies the child's preserved signed
bytes with the repository-owned SHA-512 verifier. Non-copyable evidence binds
the exact child, candidate issuer, and validated issuer key. This pure
public-data composition performs no credential access, signing, storage,
clock acquisition, or external effect, so it emits no runtime audit event. It
does not route algorithms, recognize every critical extension, check validity,
path length or name constraints, construct or validate a complete path, select
trust, check revocation, perform TLS or HTTP, or authorize OAuth transport;
concrete HTTPS remains unavailable.
A construction-bound sibling entry point now consumes one exact adjacent-link
projection from an existing bounded certificate-path construction. It derives
both endpoints only from that projection's retained depth and construction,
does not rerun caller Name policy, and applies the same fixed repository-owned
RSA PKCS #1 v1.5 SHA-512 role, key, and signature checks to that exact pair.
The non-copyable result retains the complete projection beside the validated
issuer key, preventing this authenticated link from being detached from its
selected construction order. This remains one public-data adjacency only: it
does not retain complete issuer policy, authenticate another depth, validate a
path, select trust, check constraints, hostname or revocation, perform TLS or
HTTP, authorize OAuth transport, emit audit or trace events, or cause an
external effect. Concrete HTTPS and live-provider readiness remain
unavailable.
A stronger bounded entry point now requires the exact candidate issuer to be
retained by both certificate-wide critical-extension/issuing-CA role evidence
and certificate-bound, data-driven RSA algorithm/key-policy evidence before
any Name comparison or signature work begins. Object identity, rather than
encoded-byte equality, binds both non-copyable inputs to that exact candidate.
The existing fixed RSA PKCS #1 v1.5 SHA-512 verifier then authenticates only
the supplied child-to-issuer pair with the already validated key, and success
retains the role, algorithm, key, and signature evidence together. The
algorithm profile remains caller policy data and does not route to or enable
the fixed verifier. This pure public-data composition performs no credential
access, signing, storage, clock acquisition, audit event, trace emission, or
external effect. It does not validate issuer time, path length, name
constraints, a complete path, trust, revocation, TLS, HTTP, OAuth transport,
or network authority; concrete HTTPS remains unavailable.
A bounded complete-intermediate-policy follow-up now consumes that exact
policy-bound RSA SHA-512 issuer link together with complete validated
intermediate policy. It fails closed unless the candidate issuer that verified
the child signature and the certificate retained by the complete intermediate
policy are the same exact decoded object; byte-equal independent decodes are
deliberately insufficient. Success returns distinct non-copyable evidence
retaining both complete inputs, including the intermediate's certificate-wide
critical-extension and issuing-CA role policy, data-driven algorithm policy,
self-issued classification, and explicit-time validity result. This pure
public-data binding performs no new parsing or cryptography, does not advance
path-length state or evaluate decoded NameConstraints, authenticate the
intermediate's issuer, construct or validate a complete path, grant trust,
check revocation, perform TLS or HTTP, authorize OAuth transport, or cause an
external effect. Concrete HTTPS and live-provider readiness remain
unavailable.
A bounded path-length follow-up now consumes that exact authenticated complete-
intermediate-policy proof together with one caller-owned path-length state. It
derives issuing-CA role, self-issued classification, and any decoded
`pathLenConstraint` only from the retained intermediate policy, then returns
distinct non-copyable evidence retaining the authenticated link, complete
intermediate policy, and advanced state together. Exhausted non-self-issued
capacity fails with the existing payload-free repository-owned category. This
is one caller-supplied state transition only: it does not prove prior traversal
depth or path order, authenticate the intermediate's issuer, evaluate
NameConstraints, construct or validate a complete path, grant trust, check
revocation, perform TLS or HTTP, authorize OAuth transport, or cause an
external effect. Concrete HTTPS and live-provider readiness remain
unavailable.
A construction-bound SHA-512 follow-up now consumes one exact adjacent-link
projection together with complete intermediate policy for that projection's
exact non-terminal issuer. It rejects a terminal issuer and policy retained by
any other decoded certificate object, including a byte-equal re-decode, before
the existing RSA key and signature work. Non-copyable success retains the
constructed adjacency and complete policy together. A bounded path-length
transition then derives role, self-issued classification, and any decoded
`pathLenConstraint` only from that retained policy and retains the ordered
proof with the advanced state. The caller still supplies the initial state;
this public-data composition does not authenticate another adjacency, prove
prior traversal, validate a complete path, establish reviewed trust, evaluate
name constraints or hostname, check revocation, perform TLS or HTTP, authorize
OAuth transport, emit audit or trace events, or cause a provider or network
effect. Concrete HTTPS and live-provider readiness remain unavailable.
A bounded `x509-rsa-sha512-reviewed-anchor-link` follow-up now consumes that
strongest RSA SHA-512 issuer-policy link together with reviewed-root selection
evidence. It fails closed unless both non-copyable inputs retain the same exact
decoded issuer certificate object, then retains both proofs in one distinct
non-copyable result. This proves only that the authenticated fixed-algorithm
link terminates at a certificate identity occurring exactly once in the
reviewed Mozilla snapshot. The pure public-data binding performs no credential
access, signing, storage, clock acquisition, audit event, trace emission, or
external effect. It does not apply anchor constraints, validate target role,
purpose, validity, or identity, construct or validate a multi-hop path, check
revocation, perform TLS or HTTP, authorize OAuth transport, or open a network
connection; concrete HTTPS remains unavailable.
A bounded two-link follow-up now consumes one complete-intermediate-policy-
bound RSA SHA-512 target-to-intermediate link together with one policy-bound
SHA-512 intermediate-to-reviewed-root link. It fails closed unless the
candidate issuer retained by the first proof and the child retained by the
reviewed parent proof are the same exact decoded intermediate certificate
object; byte-equal independent decodes are deliberately insufficient. Success
returns distinct non-copyable evidence retaining complete intermediate policy,
both authenticated links, and the exact reviewed-root selection. This is one
caller-ordered fixed-algorithm evidence join only. It does not derive or
advance path-length state, construct or search a path, apply anchor or
intermediate name constraints, validate the target's role, purpose, or
hostname, check revocation, perform TLS or HTTP, authorize OAuth transport, or
cause an external effect. Concrete HTTPS and live-provider readiness remain
unavailable.
A stronger additive two-link follow-up now consumes the existing authenticated
SHA-512 intermediate path-length proof together with the same policy-bound
intermediate-to-reviewed-root link. It fails closed unless the complete
intermediate policy retained by the advanced proof and the child authenticated
by the reviewed parent proof refer to the same exact decoded certificate
object. Success retains the authenticated child link, complete intermediate
policy, advanced path-length state, reviewed parent link, and exact root
selection in one distinct non-copyable result. The initial state remains
caller-owned and is not derived from anchor policy. This pure public-data
composition performs no credential access, signing, storage, clock
acquisition, audit event, trace emission, or external effect. It does not
construct or search a path, apply anchor or intermediate constraints, validate
target role, purpose, identity, or hostname, check revocation, perform TLS or
HTTP, authorize OAuth transport, or grant network authority. Concrete HTTPS
and live-provider readiness remain unavailable.
A bounded `x509-rsa-sha512-reviewed-server-link` composition now consumes one
exact target-server policy result together with one authenticated RSA SHA-512
issuer link terminating at a reviewed root. It fails closed unless both
non-copyable inputs retain the same exact decoded target certificate object,
then retains the complete target policy and reviewed-link evidence together.
Encoded-byte equality is deliberately insufficient, preventing independently
decoded certificates from contributing mixed evidence. This pure public-data
binding performs no credential access, signing, storage, clock acquisition,
audit event, trace emission, or external effect. It performs no new parsing or
signature work, applies no reviewed-anchor constraints, constructs no
multi-hop path, and does not check revocation, perform TLS or HTTP, authorize
OAuth transport, or open a network connection; concrete HTTPS remains
unavailable.
A bounded follow-up now applies the exact selected reviewed anchor's optional
DNS NameConstraints bytes to every dNSName in that retained SHA-512 target's
Subject Alternative Name. Separate caller-owned ASN.1 budgets bound the
reviewed-root constraint decoder and target SAN decoder. An unconstrained
selected anchor succeeds without manufacturing DNS evidence; malformed
constraints, a missing or malformed target SAN, unsupported constraint forms,
and names outside the permitted subtrees fail with payload-free categories.
The non-copyable result retains the complete SHA-512 reviewed server-link proof
and optional constrained-name evidence together, preventing another target or
snapshot entry from being substituted after evaluation. This remains one
direct fixed-algorithm link and one supported DNS-policy subset: it does not
construct or validate a multi-hop path, apply intermediate or IP constraints,
check revocation or hostnames, perform TLS or HTTP, authorize OAuth transport,
or open a network connection; concrete HTTPS and live-provider readiness
remain unavailable.
A bounded two-link `x509-rsa-sha512-reviewed-server-link` follow-up now
consumes one exact target-server policy result together with one authenticated
RSA SHA-512 target-to-intermediate-to-reviewed-root proof. It fails closed
unless the server-policy certificate and the target authenticated by the
proof's first link are the same exact decoded object; independently decoded
byte-equal certificates are deliberately insufficient. Success returns
distinct non-copyable evidence retaining complete target policy, complete
intermediate policy, both authenticated links, and exact reviewed-root
selection. This is evidence composition only. It does not advance path-length
state, construct or search a path, apply anchor or intermediate constraints,
check hostnames or revocation, perform TLS or HTTP, authorize OAuth transport,
or cause an external effect. Concrete HTTPS and live-provider readiness remain
unavailable.
A stronger bounded follow-up now consumes the exact target-server policy
together with the reviewed SHA-512 two-link proof that already retains its
advanced path-length state. It fails closed unless the server-policy
certificate and the first link's authenticated target are the same exact
decoded object; byte-equal independently decoded targets are insufficient.
Success retains complete target policy, complete intermediate policy, both
authenticated links, advanced path-length state, and exact reviewed-root
selection in one distinct non-copyable result. The initial path-length state
remains caller-owned and no additional transition is performed. This pure
public-data binding performs no credential access, signing, storage, clock
acquisition, audit event, trace emission, or external effect. It does not
construct or search a path, apply anchor or intermediate constraints, verify a
hostname, check revocation, perform TLS or HTTP, authorize OAuth transport, or
grant network authority. Concrete HTTPS and live-provider readiness remain
unavailable.
A bounded selected-anchor policy follow-up now consumes that complete
path-length-preserving SHA-512 server proof. It derives both the selected
reviewed snapshot entry's optional out-of-band NameConstraints bytes and the
exact target certificate's Subject Alternative Name only through the consumed
proof. Separate caller-owned ASN.1 budgets bound both decoders. An
unconstrained selected anchor succeeds without manufacturing DNS evidence;
supported permitted DNS subtrees must admit every retained target dNSName, and
malformed constraints, a missing or malformed target SAN, unsupported forms,
or names outside the permitted set fail closed with payload-free categories.
The non-copyable result retains complete target policy, intermediate policy,
both authenticated links, advanced path-length state, exact reviewed-root
selection, and optional constrained-name evidence together. This pure
public-data policy performs no credential access, signing, storage, clock
acquisition, audit event, trace emission, or external effect. It does not
apply intermediate or non-DNS constraints, construct or search a path, verify
a hostname, check revocation, perform TLS or HTTP, authorize OAuth transport,
or grant network authority. Concrete HTTPS and live-provider readiness remain
unavailable.
A bounded intermediate-policy follow-up now consumes that complete SHA-512
selected-anchor proof and applies the exact retained intermediate's
NameConstraints extension to the exact retained target Subject Alternative
Name. An absent extension preserves the complete prior proof without new DNS
evidence. A present extension must be critical and must decode under a separate
caller-owned ASN.1 budget as the supported permitted-and-excluded-dNSName
subset; every target dNSName must satisfy the permitted policy, exclusions take
precedence, and malformed or unsupported forms fail closed with payload-free
categories. Both certificates and the SAN are derived only through the
consumed proof, preventing detached certificate or policy substitution. The
non-copyable result retains target and intermediate policy, both authenticated
links, advanced path-length state, exact reviewed-root selection, anchor DNS
policy, and optional intermediate constrained-name evidence together. This pure
public-data policy performs no credential access, signing, storage, clock
acquisition, audit event, trace emission, or external effect. It does not apply
non-DNS constraints, construct or search a path, verify a hostname, check
revocation, perform TLS or HTTP, authorize OAuth transport, or grant network
authority. Concrete HTTPS and live-provider readiness remain unavailable.
A bounded `x509-rsa-signature-routing` prerequisite now separates
caller-owned certificate algorithm selection from fixed repository-owned RSA
verification. It consumes exact child-certificate algorithm-profile evidence
and exact issuer-certificate RSA key-policy evidence, selects one route by the
child profile index, and invokes only the routed PKCS #1 v1.5 SHA-256,
SHA-384, or SHA-512 verifier. Missing and duplicate routes fail before RSA
work. A route naming a verifier inconsistent with the child certificate fails
that fixed verifier's exact AlgorithmIdentifier checks. Success retains both
non-copyable policy inputs and the exact fixed-verifier evidence, preventing a
detached child certificate or issuer key from entering after policy.
The routed success enum now projects the exact retained child algorithm policy
and issuer RSA algorithm/key policy through one verifier-independent API. Each
projection internally matches on the fixed-verifier variant and borrows the
same non-copyable evidence already retained by that variant; it does not clone,
re-decode, rerun policy, choose a new algorithm, or expose secret material.
Downstream path and reviewed-anchor composition can therefore verify exact
child and issuer object identity without reopening SHA-256, SHA-384, and
SHA-512 variants or accepting detached certificates and keys.

This is public-data signature composition only. It does not decide issuer
Name linkage or role, recognize certificate extensions, construct or validate
a path, establish reviewed trust, check target identity, time, purpose, path
length, name constraints, or revocation, perform TLS or HTTP, access
credentials, sign data, emit an external audit/provider effect, authorize
OAuth transport, or grant network authority. Concrete HTTPS remains
unavailable.
A bounded `x509-rsa-constructed-path-signature-routing` composition now binds
that data-driven RSA verifier to one exact adjacent pair projected from a
constructed certificate path. It consumes certificate-bound child algorithm
policy, certificate-wide issuing-CA role policy, and certificate-bound issuer
RSA algorithm/key policy only when all three retain the exact decoded objects
projected by the path. Cross-certificate evidence and non-CA issuers fail
before RSA work. Caller-owned profile routes then invoke exactly one reviewed
RSA PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512 verifier, and non-copyable
success retains the complete path projection, issuer role policy, algorithm
policies, validated issuer key, and fixed-verifier result together. The
construction's caller-owned Name comparison is not upgraded into a complete
RFC 5280 Name rule, and this one-link composition does not validate another
adjacency, path length, constraints, a terminal trust anchor, target policy,
revocation, TLS, HTTP, OAuth transport, or network authority. Concrete HTTPS
remains unavailable.
A bounded follow-up now consumes that routed construction-bound signature
proof together with complete intermediate policy for its exact non-terminal
issuer. It rejects the construction's terminal root and policy retained by any
other decoded certificate object, including a byte-equal independent decode.
Success retains the routed SHA-256, SHA-384, or SHA-512 authentication,
construction order, certificate-wide critical-extension and issuing-CA role
policy, data-driven algorithm policy, self-issued classification, and explicit-
time validity result together. A second bounded follow-up advances caller-
owned path-length state using only that retained complete policy and retains
the authenticated adjacency, policy, and advanced state in one non-copyable
result. This pure public-certificate composition performs no credential
access, signing, storage, clock acquisition, audit event, trace emission, I/O,
or external effect. It does not authenticate another adjacency, prove prior
traversal, construct or validate a complete path, apply name constraints,
establish reviewed trust, verify a hostname, check revocation, perform TLS or
HTTP, authorize OAuth transport, or grant network authority. Concrete HTTPS
and live-provider readiness remain unavailable.
A separate bounded terminal follow-up now consumes one data-driven routed
constructed-path signature proof together with reviewed Mozilla root-selection
evidence. It succeeds only when the authenticated adjacency ends at the
construction's final selected issuer and both proofs retain the same exact
decoded terminal certificate object; a byte-equal independent decode fails
closed. The non-copyable result retains construction order, certificate-wide
critical-extension and issuing-CA role policy, certificate-bound RSA
algorithm/key policy, the selected repository-owned SHA-256, SHA-384, or
SHA-512 verifier result, and reviewed-root selection together. This is an
authenticated trust-root handoff, not complete path validity: reviewed
snapshot membership does not apply anchor or intermediate name constraints,
prove prior traversal or path-length state, validate target policy, identity,
or time, check revocation, perform TLS or HTTP, authorize OAuth transport, or
grant network authority. The pure public-certificate composition performs no
credential access, signing, storage, clock acquisition, audit event, trace
emission, I/O, or external effect. Concrete HTTPS and live-provider readiness
remain unavailable.
A bounded two-link follow-up now consumes the routed intermediate path-length
transition and routed reviewed-terminal handoff for one exact
target -> intermediate -> root construction. It fails closed unless both
inputs retain the same constructed-path object at depths zero and one, the
construction has exactly two issuers, and the terminal child is the exact
intermediate retained by complete policy. Success retains both data-driven RSA
signature results, complete intermediate critical-extension, role, algorithm,
self-issued, and explicit-time policy, advanced caller-owned path-length state,
and reviewed Mozilla root selection together. Proofs projected from a distinct
construction fail even when they select the same decoded certificates. This
fixed shape does not support additional intermediates, derive the initial
path-length cap from anchor policy, apply anchor or intermediate name
constraints, validate target identity or purpose, check revocation, perform TLS
or HTTP, authorize OAuth transport, or grant network authority. The pure
public-certificate composition performs no credential access, signing,
storage, clock acquisition, audit event, trace emission, I/O, or external
effect. Concrete HTTPS and live-provider readiness remain unavailable.
A bounded target-policy follow-up now consumes that data-driven routed
two-link result together with complete server-certificate policy. It succeeds
only when the policy retains the exact decoded target object in the retained
construction; policy for a byte-equal independent decode fails closed. The
non-copyable result keeps complete target critical-extension, algorithm,
end-entity role, SAN-only identity, explicit-time validity, and purpose policy
together with both routed signature results, complete intermediate policy,
advanced caller-owned path-length state, and reviewed Mozilla root selection.
This pure public-certificate composition performs no credential access,
signing, storage, clock acquisition, audit event, trace emission, I/O, or
external effect. It does not derive the initial path-length cap from anchor
policy, apply anchor or intermediate constraints, check revocation, generalize
path validation, perform TLS or HTTP, authorize OAuth transport, or grant
network authority. Concrete HTTPS and live-provider readiness remain
unavailable.
A bounded selected-anchor DNS-policy follow-up now consumes that exact routed
server-path result. It derives the target certificate only from retained
complete server policy and derives optional constraint bytes only from the
exact reviewed terminal selected by the same authenticated construction.
Separate caller-owned limits bound the repository-owned reviewed-constraint
and Subject Alternative Name decoders. An unconstrained snapshot entry retains
the complete proof without DNS evidence; a constrained entry requires every
target dNSName to satisfy the supported permitted DNS subtrees and retains only
a payload-free count. This pure public-certificate composition performs no
credential access, signing, storage, clock acquisition, audit event, trace
emission, I/O, or external effect. It does not apply intermediate, excluded,
or non-DNS anchor constraints, derive the initial path-length cap, check
revocation, generalize path validation, perform TLS or HTTP, authorize OAuth
transport, or grant network authority. Concrete HTTPS and live-provider
readiness remain unavailable.
An additive bounded intermediate name-constraints follow-up now consumes that
exact anchor-policy proof. It derives the intermediate certificate, target
certificate, and already-validated target SAN extension index only through the
retained routed construction. An absent NameConstraints extension succeeds
without manufacturing evidence; a present extension must be critical and must
decode under a separate caller-owned limit before every retained target
dNSName and IPv4 or IPv6 iPAddress is evaluated together against the supported
permitted-and-excluded DNS and IP subsets. Success keeps the complete target
policy, both routed signatures, complete intermediate policy, advanced
construction-derived path-length state, reviewed root selection, anchor-policy
evidence, and optional payload-free DNS/IP counts together. The earlier DNS
count remains available as compatibility evidence only after the combined
evaluation succeeds. This pure public-certificate composition performs no
credential access, signing, storage, clock acquisition, audit event, trace
emission, I/O, or external effect. It does not support additional
intermediates, apply other GeneralName forms or excluded or non-DNS anchor
constraints, verify a hostname, check revocation, generalize path validation,
perform TLS or HTTP, authorize OAuth transport, or grant network authority.
Concrete HTTPS and live-provider readiness remain unavailable.
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
A bounded follow-up now consumes that exact intermediate-certificate policy
and structurally decodes the optional NameConstraints extension retained by
the same decoded certificate. Absence is recorded explicitly; a present value
must be critical and must decode as one exact supported DNS/IP NameConstraints
SEQUENCE under a separate caller-owned ASN.1 budget. Non-copyable success
retains the complete intermediate policy and borrowed decoded constraints for
later path processing. This gate does not evaluate those constraints against
any certificate, traverse or authenticate a path, apply reviewed-anchor
constraints, select trust, check revocation, or provide TLS, HTTP, OAuth
transport, or network authority.
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
A bounded policy-backed follow-up now lets the same selector consume one
validated `X509NameValuePolicy` directly. Candidate limits fail before policy
evaluation; the first unconfigured or malformed Name value aborts with a
payload-free category; and successful comparisons still scan the complete pool
to reject ambiguity. Distinct non-copyable success evidence retains the exact
validated policy as well as the child, candidate, and index, preventing generic
callback selection evidence from substituting at a later policy-requiring
boundary. This removes executable attribute-value callbacks from the concrete
selector boundary, but the available exact-DER and exact-Unicode-scalar rules
do not implement RFC 4518 normalization, case or whitespace folding,
schema-specific matching, or TeletexString mapping. It therefore does not
establish complete RFC 5280 Name equality, authenticate a selected link,
construct or validate a path, recognize trust, check revocation, perform TLS or
HTTP, authorize OAuth transport, or open a network connection.
A bounded policy-backed path-construction follow-up now repeats that direct
selector across one caller-bounded target-to-terminal chain. All candidate and
path limits fail before terminal or Name-policy evaluation; every non-terminal
step uses the same exact validated `X509NameValuePolicy`; and the first
selection or Name-comparison failure aborts with a stable payload-free nested
category. Distinct non-copyable success evidence retains both the constructed
target, candidate pool, and ordered indices and the exact Name policy that
governed them, so generic callback construction evidence cannot substitute at
a later policy-requiring boundary. The available exact-DER and
exact-Unicode-scalar rules still do not implement RFC 4518 normalization, case
or whitespace folding, schema-specific matching, or TeletexString mapping.
This follow-up therefore does not establish complete RFC 5280 Name equality,
authenticate any selected link, validate the constructed path, recognize
trust, check revocation, perform TLS or HTTP, authorize OAuth transport, or
open a network connection.
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
A distinct policy-backed reviewed-terminal binding follow-up now consumes the
complete `ConstructedX509CertificatePathWithNamePolicy` evidence together with
reviewed-root selection evidence. It applies the same nontrivial-path and exact
decoded-terminal object checks without extracting or recreating generic path
evidence, then retains the complete policy-backed construction and reviewed
anchor in one non-copyable result. Generic callback construction evidence
cannot produce this result, and the exact validated `X509NameValuePolicy` that
governed every issuer-selection step remains available from the retained
construction. This is provenance-preserving evidence composition only: the
available Name rules remain incomplete for RFC 5280 Name equality, and binding
does not authenticate any link, validate certificate roles or a complete path,
apply anchor constraints, check revocation, perform TLS or HTTP, authorize OAuth
transport, or open a network connection.
A distinct policy-backed role-policy follow-up consumes that complete
Name-policy-backed reviewed-terminal evidence together with certificate-wide
role-policy evidence. It fails closed unless the role evidence retains the
identical decoded reviewed-root object and authorizes that object as an issuing
CA, then retains the complete Name-policy-backed construction, reviewed-root
selection, critical-extension recognition, and role proof in one non-copyable
result. Generic reviewed-terminal evidence cannot produce this stronger result,
and neither a byte-equal independent decode nor end-entity role evidence can be
substituted. This provenance-preserving composition still does not define
complete RFC 5280 Name equality, authenticate a path link, validate the anchor
self-signature or algorithm, apply anchor constraints, check time or
revocation, perform TLS or HTTP, authorize OAuth transport, or grant network
authority. Concrete HTTPS remains unavailable.
A distinct policy-backed path-length initialization follow-up consumes that
complete Name-policy-backed reviewed-terminal issuing-CA proof and derives RFC
5280's initial capacity only from its retained construction. The final selected
issuer is already bound to the reviewed trust anchor, so subtracting that
anchor from the selected-issuer count yields the exact prospective intermediate
count. The non-copyable result retains the complete Name-policy-backed
construction, reviewed-root selection, certificate-wide anchor role policy,
and derived state together. Generic reviewed-terminal evidence cannot produce
this stronger result, and callers cannot detach the exact Name-value policy,
inflate the capacity, or attach it to another construction. This pure public-
certificate-data composition performs no parsing, signature verification,
credential access or editing, signing, storage, clock acquisition, provider
action, audit event, trace emission, I/O, network access, or other external
effect. It does not define complete RFC 5280 Name equality, authenticate an
adjacency, apply an intermediate transition, generalize full path validation,
check revocation, perform TLS or HTTP, authorize OAuth transport, or grant
network authority. Concrete HTTPS and live-provider readiness remain
unavailable.
A distinct policy-backed ordered-transition follow-up now borrows that stronger
initializer and consumes one already routed terminal proof only when both
retain the same exact constructed-path object. The terminal proof authenticates
the anchor-nearest intermediate as the reviewed root's exact child, and
complete intermediate policy must retain that same decoded child before a
private working state is recreated from the construction-derived capacity and
advanced. The non-copyable result retains the Name-policy-backed initializer,
its exact validated `X509NameValuePolicy`, reviewed-root selection,
certificate-wide anchor role policy, terminal repository-owned RSA signature
evidence, complete intermediate policy, and advanced path-length state
together. Generic initialization evidence cannot produce this stronger result;
proofs from another construction and policy for another certificate fail
closed. This pure public-certificate-data composition performs no new parsing
or signature verification, credential access or editing, signing, storage,
clock acquisition, provider action, audit event, trace emission, I/O, network
access, or other external effect. It proves only the first ordered
anchor-to-target intermediate transition. The available Name-value rules still
do not define complete RFC 5280 Name equality, and this boundary does not
process remaining intermediates, validate the whole path, apply name
constraints, check revocation, perform TLS or HTTP, authorize OAuth transport,
or grant network authority. Concrete HTTPS and live-provider readiness remain
unavailable.
A bounded second policy-backed ordered-transition follow-up consumes that
stronger first-step result together with one already routed proof for the
immediately target-nearer intermediate. It fails closed unless the new proof
retains the same exact constructed-path object, occupies the depth immediately
preceding the reviewed terminal, uses the already processed anchor-nearest
intermediate as its exact issuer object, authenticates another selected
candidate child, and complete intermediate policy retains that exact child.
Only then is the retained path-length state recreated privately and advanced.
The non-copyable result retains the complete first-step proof, including its
exact validated `X509NameValuePolicy`, reviewed-root selection, anchor role
policy, terminal repository-owned RSA signature, and first intermediate
policy, together with the second routed RSA signature, second intermediate
policy, and twice-advanced state. Generic first-step evidence, detached
constructions, skipped depths, disconnected issuers, target children, and
cross-certificate policy cannot produce this stronger result. This pure
public-certificate-data composition performs no new parsing or signature
verification, credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network access,
or other external effect. It proves exactly two ordered anchor-to-target
intermediate transitions. The available Name-value rules still do not define
complete RFC 5280 Name equality, and this boundary does not generalize
arbitrary traversal, validate the whole path, apply name constraints, check
revocation, perform TLS or HTTP, authorize OAuth transport, or grant network
authority. Concrete HTTPS and live-provider readiness remain unavailable.
A bounded role-policy follow-up now consumes that exact reviewed-terminal-bound
path together with certificate-wide role-policy evidence. It fails closed
unless the role evidence retains the identical decoded terminal certificate
object and authorizes that object as an issuing CA, then retains the complete
critical-extension recognition and role proof with the constructed path and
reviewed-root selection. Byte-equal independent decodes and end-entity role
evidence fail before retention. This does not authenticate any path link,
validate the anchor self-signature or algorithm, apply anchor constraints,
check time or revocation, perform TLS or HTTP, authorize OAuth transport, or
grant network authority. Concrete HTTPS remains unavailable.
A bounded path-length initialization follow-up now consumes that exact
reviewed-terminal issuing-CA proof and derives RFC 5280's initial capacity only
from the retained construction. Because the construction already proves that
its final selected issuer is the reviewed trust anchor, subtracting that anchor
from the selected-issuer count yields the exact prospective intermediate count.
The non-copyable result retains the complete constructed path, reviewed-root
selection, anchor role policy, and derived state together; callers cannot
inflate the capacity or attach it to another construction. This pure public-
certificate-data composition performs no parsing, signature verification,
credential access or editing, signing, storage, clock acquisition, provider
action, audit event, trace emission, I/O, network access, or other external
effect. It does not authenticate an adjacency, apply an intermediate
transition, generalize full path validation, check revocation, perform TLS or
HTTP, authorize OAuth transport, or grant network authority. Concrete HTTPS
and live-provider readiness remain unavailable.
A bounded ordered-transition follow-up now borrows that retained initializer
and consumes one routed terminal proof from the same exact constructed-path
object. That terminal proof authenticates the anchor-nearest intermediate as
its exact child using the reviewed root's key. Complete intermediate policy
must retain that same child before a fresh private working state is derived
from the construction-derived capacity and advanced. The non-copyable result
keeps the initializer reference, terminal signature and reviewed-root handoff,
complete child policy, and advanced state together. Proofs from another
construction and policy for another certificate fail closed, so a caller
cannot skip the root-authenticated first step or inject detached initial state.
This is only the first ordered intermediate transition. A bounded second-step
follow-up now consumes that result together with the immediately target-nearer
routed adjacency from the same exact constructed-path object. The new
adjacency's issuer must be the already processed anchor-nearest intermediate,
its issuer depth must immediately precede the retained terminal depth, and its
child must be another candidate certificate rather than the target. Complete
intermediate policy must retain that exact authenticated child before the
retained path-length state is advanced. The non-copyable result keeps the
entire first step, second routed signature, complete second-child policy, and
twice-advanced state together. A proof from another construction, a skipped or
replayed depth, an issuer not retained by the first step, or policy for another
certificate fails closed. This establishes exactly two ordered intermediate
transitions; it does not iterate over an arbitrary path, prove that all
remaining depths were processed, or establish complete path validation. The
pure public-certificate-data
composition performs no parsing, new signature verification, credential access
or editing, signing, storage, clock acquisition, provider action, audit event,
trace emission, I/O, network access, or other external effect. It does not
check names, time, revocation, TLS, HTTP, OAuth transport, or network authority.
Concrete HTTPS and live-provider readiness remain unavailable.
A bounded arbitrary-depth follow-up now borrows the same retained initializer,
reviewed terminal, an anchor-to-target slice of already routed follow-on
signatures, and an equally ordered slice of complete intermediate policies.
The caller supplies a nonzero intermediate ceiling. Before transition, the
composition derives the exact intermediate count from the retained
construction, rejects counts above that ceiling, and requires exactly one
policy per intermediate plus one follow-on signature for every intermediate
after the root-authenticated first child. It then requires every follow-on
proof to retain the same exact construction, consume the immediately
target-nearer issuer depth, name the previously processed intermediate as its
exact issuer object, authenticate another candidate rather than the target,
and pair that child with policy for the same exact object before advancing the
private working state. Success retains the initializer, reviewed terminal,
both complete borrowed evidence slices, the processed count, and the final
path-length state. This proves only that every intermediate path-length
transition represented by the selected construction was processed in
anchor-to-target order; it deliberately stops before the target signature and
does not turn routed link proofs into complete path validation. The pure
public-certificate-data composition performs no parsing, new signature
verification, credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network access,
or other external effect. It does not check target authentication, complete
Name semantics, time, hostname, revocation, TLS, HTTP, OAuth transport, or
network authority. Concrete HTTPS and live-provider readiness remain
unavailable.
A stronger bounded arbitrary-depth entry point now accepts only the retained
Name-policy-backed initializer together with the same reviewed terminal,
ordered routed-signature slice, ordered complete-intermediate-policy slice,
and nonzero caller ceiling. It applies the identical construction-derived
count, ceiling, exact-object, immediate-depth, issuer-continuity,
authenticated-child, complete-policy, and path-length checks as the generic
accumulator. Success retains the exact Name-value policy, reviewed trust-root
selection, certificate-wide anchor-role policy, construction-derived initial
capacity, both complete borrowed evidence slices, processed count, and final
path-length state in one non-copyable result. A generic initializer or evidence
from another construction cannot produce that result. This preserves stronger
policy provenance through arbitrary-depth intermediate traversal only; it does
not close the target signature, define complete RFC 5280 Name equality, apply
all certificate or Name constraints, validate a complete path, check time,
hostname, or revocation, perform TLS or HTTP, authorize OAuth transport, or
grant network authority. The pure public-certificate-data composition performs
no parsing, new signature verification, credential access or editing, signing,
storage, clock acquisition, provider action, audit event, trace emission, I/O,
network access, or other external effect. Concrete HTTPS and live-provider
readiness remain unavailable.
A bounded target-signature closure now consumes that arbitrary-depth
intermediate accumulator together with one already routed depth-zero signature
proof and complete server-certificate policy. It fails closed unless the
signature retains the same exact constructed-path object, projects issuer
depth zero, authenticates the construction's exact target child, and names the
target-nearest intermediate retained by the accumulator's final ordered policy
as its exact issuer object. The complete server policy must retain that same
exact target object. Success keeps the entire intermediate accumulator, the
target signature, and complete target policy in one non-copyable result, so
every selected child-to-issuer adjacency from target through reviewed anchor
has repository-owned routed RSA verification evidence without allowing a
detached target or issuer proof to be spliced in. This is signature-evidence
closure only: it does not validate the anchor self-signature, generalize all
certificate or Name policy across an arbitrary path, apply every intermediate
NameConstraints value, verify the requested hostname, check revocation, or
compose TLS, HTTP, OAuth transport, or network authority. The pure public-
certificate-data composition performs no parsing, new signature verification,
credential access or editing, signing, storage, clock acquisition, provider
action, audit event, trace emission, I/O, network access, or other external
effect. Concrete HTTPS and live-provider readiness remain unavailable.
A stronger target-signature closure follow-up consumes only the Name-policy-
backed arbitrary-depth accumulator together with the same already routed depth-
zero signature proof and complete server-certificate policy. It applies the
identical exact-construction, depth-zero, exact-target-child, target-nearest-
issuer, and exact-server-policy checks as the generic closure. Success retains
the exact Name-value policy, reviewed trust-root selection, certificate-wide
anchor-role policy, construction-derived path-length state, complete ordered
intermediate evidence, target signature, and complete target policy in one
non-copyable result. A generic accumulator cannot produce this stronger
closure, and a target proof or policy from another construction fails closed
even when its certificate bytes compare equal. This preserves stronger policy
provenance across target-signature closure only; it does not define complete
RFC 5280 Name equality, apply all certificate or Name constraints, validate a
complete path, check time, hostname, or revocation, perform TLS or HTTP,
authorize OAuth transport, or grant network authority. The pure public-
certificate-data composition performs no parsing, new signature verification,
credential access or editing, signing, storage, clock acquisition, provider
action, audit event, trace emission, I/O, network access, or other external
effect. Concrete HTTPS and live-provider readiness remain unavailable.
A bounded reviewed-anchor DNS-policy follow-up now consumes that exact target-
signature closure together with separate caller-owned limits for the reviewed
constraint and target Subject Alternative Name decoders. It derives the exact
target certificate and already-validated SAN extension index only from the
closure's complete server policy, and derives optional constraint bytes only
from the reviewed anchor retained through the accumulator's terminal proof.
An unconstrained snapshot entry succeeds without new DNS evidence. A
constrained entry must decode as the repository's supported permitted-DNS
subset and every retained target dNSName must satisfy that subset before
success retains the complete target-signature closure plus only a payload-free
constrained-name count. Callers cannot substitute another target, anchor, SAN
index, or constraint value. This applies only the reviewed snapshot's current
supported anchor DNS policy. It does not apply every intermediate
NameConstraints value, excluded or non-DNS anchor constraints, generalize full
Name processing or path validation, check revocation, compose TLS or HTTP,
authorize OAuth transport, or grant network authority. The pure public-
certificate-data composition performs no credential access or editing,
signing, storage, clock acquisition, provider action, audit event, trace
emission, I/O, network access, or other external effect. Concrete HTTPS and
live-provider readiness remain unavailable.
A stronger reviewed-anchor DNS-policy follow-up consumes only the Name-policy-
backed target-signature closure with the same separate caller-owned decoder
limits. It derives the exact target, already-validated SAN extension index,
reviewed anchor, and optional reviewed-anchor constraint bytes only through
that stronger retained evidence, then applies the identical supported
permitted-DNS evaluator as the generic follow-up. Success retains the exact
Name-value policy, reviewed trust-root selection, certificate-wide anchor-role
policy, construction-derived path-length state, complete ordered intermediate
and target-signature evidence, complete target policy, and any payload-free
constrained-name count in one non-copyable result. A generic target-signature
closure cannot produce this stronger policy result. This preserves stronger
policy provenance through the supported reviewed-anchor DNS subset only; it
does not define complete RFC 5280 Name equality, apply excluded or non-DNS
anchor constraints or every intermediate constraint, validate a complete
path, check time, hostname, or revocation, perform TLS or HTTP, authorize OAuth
transport, or grant network authority. The pure public-certificate-data
composition performs no credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network access,
or other external effect. Concrete HTTPS and live-provider readiness remain
unavailable.
A bounded ordered-intermediate name-policy follow-up now consumes that exact
reviewed-anchor policy result together with separate caller-owned limits for
each intermediate NameConstraints decoder and each target Subject Alternative
Name decoder. It derives the exact target certificate and already-validated
SAN extension index only from the retained target-signature closure, then
visits every complete intermediate policy retained by the accumulator in its
already-proved anchor-to-target order. An intermediate without NameConstraints
succeeds without new evidence. A present extension must be critical, must
decode as the repository's supported permitted-and-excluded DNS and IP
subsets, and every retained target dNSName and iPAddress must satisfy both
subsets before traversal may continue. The accumulator's earlier nonzero
intermediate ceiling bounds the number of visits; the new caller-owned limits
bound each repository-owned decode. Success retains the complete reviewed-
anchor policy result, the number of constrained intermediates, and only
payload-free target DNS/IP counts. Callers cannot substitute another target,
SAN index, intermediate, or traversal order. The generic ordered target-policy
result cannot construct this stronger result. The strong and generic entry
points must share the same private ordered evaluator so supported constraint
semantics, decoder-limit propagation, stable failure kinds, and payload
redaction cannot drift between them.

This composes supported intermediate constraints only against the exact target
identities. It does not apply constraints to subordinate CA subject or SAN
names, support every GeneralName form, generalize full Name processing or path
validation, validate the anchor self-signature, check revocation, compose TLS
or HTTP, authorize OAuth transport, or grant network authority. The pure
public-certificate-data composition performs no credential access or editing,
signing, storage, clock acquisition, provider action, audit event, trace
emission, I/O, network access, or other external effect. Concrete HTTPS and
live-provider readiness remain unavailable.
A bounded subordinate-intermediate name-policy follow-up consumes that exact
ordered target-policy result and derives every complete intermediate policy,
certificate, and self-issued classification only from its retained
accumulator. For each intermediate that carries critical supported DNS/IP
NameConstraints, it visits every target-nearer intermediate in the already-
proved anchor-to-target order. The constraint is applied to each such
intermediate unless that subordinate certificate is self-issued, matching the
RFC 5280 exception for non-final self-issued certificates. An absent
subordinate Subject Alternative Name contributes an empty supported name set;
a present extension must decode under caller-owned limits, and every retained
dNSName and iPAddress must satisfy the constraining ancestor before traversal
continues. The earlier path and intermediate ceilings bound certificates and
constraint applications, while separate caller-owned limits bound every
repository-owned decode. Success retains the complete target-policy result
plus payload-free counts of constraint applications and repeated DNS/IP name
evaluations. Callers cannot substitute or reorder constraints, subordinate
certificates, SAN values, or self-issued classifications.

The previously consumed target-policy result still proves every intermediate
constraint against the exact final target. This additive slice does not apply
the reviewed anchor's out-of-band constraints to subordinate CAs, constrain
subject distinguished names, support directoryName or other GeneralName forms,
or establish complete Name or path validation. It performs only pure public-
certificate-data processing with no credential access or editing, signing,
storage, clock acquisition, provider action, audit event, trace emission, I/O,
network access, or other external effect. It does not validate the anchor
self-signature, check revocation, compose TLS or HTTP, authorize OAuth
transport, or grant network authority. Concrete HTTPS and live-provider
readiness remain unavailable.

A Name-policy-backed subordinate-intermediate follow-up consumes the exact
strong ordered target-policy result and derives every complete intermediate
policy, certificate, anchor-to-target order, and self-issued classification
only from its retained accumulator. It applies the same supported DNS/IP
constraints to the same target-nearer, non-self-issued subordinate
intermediates under the same independent caller-owned decoder limits as the
generic follow-up. Success retains the complete strong target policy plus only
payload-free counts of constraint applications and repeated DNS/IP name
evaluations. The generic subordinate-intermediate result cannot construct or
substitute for this stronger result. The strong and generic entry points must
share the exact private ordered subordinate evaluator so traversal, the
self-issued exception, decoder-limit propagation, stable failure kinds, and
payload redaction cannot drift between them.

This preserves the exact Name-value policy, reviewed trust-root selection,
certificate-wide anchor-role policy, construction-derived path-length state,
complete ordered intermediate and target-signature evidence, complete target
policy, and both target and subordinate supported Name-constraint evaluations
in one non-copyable result. It still does not apply reviewed-anchor constraints
to subordinate CAs, constrain subject distinguished names, support every
GeneralName form, establish complete Name or path validation, validate the
anchor self-signature, check revocation, compose TLS or HTTP, authorize OAuth
transport, or grant network authority. The pure public-certificate-data
composition performs no credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network access,
or other external effect. Concrete HTTPS and live-provider readiness remain
unavailable.

A bounded reviewed-anchor subordinate-name-policy follow-up consumes that
exact subordinate-intermediate policy result and derives the reviewed
anchor's optional constraint bytes, every complete intermediate policy,
certificate, anchor-to-target order, and self-issued classification only from
retained evidence. An unconstrained reviewed snapshot entry succeeds without
reading any subordinate Subject Alternative Name. When the reviewed entry
carries its supported permitted-DNS constraint contents, the composition
decodes those bytes once under caller-owned limits and visits every retained
intermediate in the already-proved order. Each non-self-issued subordinate
certificate's optional Subject Alternative Name must decode under separate
caller-owned limits, and every retained dNSName must satisfy the reviewed
anchor constraint. An absent subordinate Subject Alternative Name contributes
an empty supported name set. Non-final self-issued intermediate certificates
are skipped under the RFC 5280 exception. Success retains the complete
subordinate-intermediate policy plus payload-free counts of reviewed-anchor
constraint applications and repeated DNS-name evaluations. Callers cannot
substitute the reviewed anchor, constraint bytes, subordinate certificates,
SAN values, order, or self-issued classifications.

The consumed target-anchor policy already proves the same reviewed constraint
against the exact final target, and the consumed subordinate-intermediate
policy already applies supported intermediate constraints across the path.
This additive slice does not constrain subordinate subject distinguished
names, support directoryName or other GeneralName forms, add excluded or
non-DNS reviewed-anchor constraints, or establish complete Name or path
validation. It performs only pure public-certificate-data processing with no
credential access or editing, signing, storage, clock acquisition, provider
action, audit event, trace emission, I/O, network access, or other external
effect. It does not validate the anchor self-signature, check revocation,
compose TLS or HTTP, authorize OAuth transport, or grant network authority.
Concrete HTTPS and live-provider readiness remain unavailable.

A Name-policy-backed reviewed-anchor subordinate-name-policy follow-up consumes
the exact strong subordinate-intermediate result and derives the reviewed
anchor's optional constraint bytes, every complete intermediate certificate,
anchor-to-target order, and self-issued classification only through that
retained evidence. It applies the same supported permitted-DNS contents under
the same independent caller-owned decoder limits as the generic reviewed-anchor
follow-up. Success retains the complete strong subordinate-intermediate policy
plus only payload-free counts of reviewed-anchor constraint applications and
repeated DNS-name evaluations. The generic reviewed-anchor subordinate result
cannot construct or substitute for this stronger result. The strong and generic
entry points must share the exact private reviewed-anchor subordinate evaluator
so unconstrained short-circuiting, traversal, the self-issued exception,
decoder-limit propagation, stable failure kinds, and payload redaction cannot
drift between them.

This preserves the exact Name-value policy, reviewed trust-root selection,
certificate-wide anchor-role policy, construction-derived path-length state,
complete ordered intermediate and target-signature evidence, complete target
policy, and all supported target and subordinate Name-constraint evaluations in
one non-copyable result. It still does not constrain subordinate subject
distinguished names, support every GeneralName form, add excluded or non-DNS
reviewed-anchor constraints, establish complete Name or path validation,
validate the anchor self-signature, check revocation, compose TLS or HTTP,
authorize OAuth transport, or grant network authority. The pure public-
certificate-data composition performs no credential access or editing,
signing, storage, clock acquisition, provider action, audit event, trace
emission, I/O, network access, or other external effect. Concrete HTTPS and
live-provider readiness remain unavailable.

A bounded revocation-binding follow-up consumes that exact strongest supported
Name-policy-backed path result together with one fully routed authenticated
OCSP target-revocation proof. The target certificate is derived only from the
retained complete server policy, and the target-nearest issuer is derived only
from the retained depth-zero routed signature. Binding requires the OCSP proof
to retain those same exact decoded certificate objects, then validates both
path certificates at the OCSP proof's retained observation time. Failure order
is stable: target identity, issuer identity, target validity, then issuer
validity. Success retains both complete non-copyable inputs plus the two exact-
time validity tokens; byte-equal independent decodes, detached issuers, and
cross-time evidence cannot substitute.

This is pure public-certificate and signed-response evidence composition. It
performs no credential access or editing, signing, storage, clock acquisition,
provider action, audit event, trace emission, I/O, network access, or other
external effect. It does not fetch revocation data, reify or authorize the
delegated responder path, apply responder DNS or no-check policy, establish
status for every non-root certificate, complete Name or path validation,
validate the anchor self-signature, compose TLS or HTTP, authorize OAuth
transport, or grant network authority. Concrete HTTPS and live-provider
readiness remain unavailable.

A bounded responder-path projection follow-up consumes only that exact strong
Name-policy path/fully routed OCSP binding and nominally reifies the delegated
responder's already-authenticated responder-to-issuer-to-reviewed-root suffix.
It accepts no caller-supplied certificate, path, algorithm, key, signature,
time, or policy input, performs no additional validation, and retains the
complete non-copyable strong binding intact. The generic older routed-path
binding cannot substitute for this stronger projection.

This pure public-data composition performs no parsing, cryptography, credential
access or editing, signing, storage, clock acquisition, provider action, audit
event, trace emission, I/O, network access, or other external effect. It does
not apply responder DNS or no-check policy, establish separately authenticated
responder or intermediate status, complete revocation or Name/path validation,
validate the anchor self-signature, compose TLS or HTTP, authorize OAuth
transport, or grant network authority. Concrete HTTPS and live-provider
readiness remain unavailable.

A bounded strong-path responder anchor-DNS follow-up consumes only that exact
responder-path projection plus separate caller-owned limits for the reviewed
constraint and responder Subject Alternative Name decoders. It derives the
fully routed authenticated responder certificate only through the retained
OCSP proof and derives the exact selected Mozilla-reviewed root's optional
out-of-band constraint bytes only through the retained strongest Name-policy
path. An unconstrained snapshot entry succeeds without reading the responder
SAN. A constrained entry must decode as the repository's supported permitted-
DNS subset, and every exact responder `dNSName` must satisfy that subset before
success retains the complete non-copyable responder path plus only payload-free
count evidence. Callers cannot substitute another responder, root, SAN,
constraint value, path, algorithm, key, signature, or observation time. The
entry point shares the existing private responder-anchor evaluator and stable
payload-free failure categories so decoding, limit propagation, evaluation,
and redaction cannot drift from the older fixed-path and routed-path policies.

This pure public-certificate-data policy performs no credential access or
editing, signing, storage, clock acquisition, provider action, audit event,
trace emission, I/O, network access, or other external effect. It does not
apply excluded or non-DNS reviewed-root constraints or the retained
intermediate's constraints, establish no-check or separately authenticated
responder revocation, authenticate intermediate status, complete revocation or
Name/path validation, validate the anchor self-signature, compose TLS or HTTP,
authorize OAuth transport, or grant network authority. Concrete HTTPS and
live-provider readiness remain unavailable.

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
A distinct policy-backed reviewed-endpoints follow-up now consumes the complete
Name-policy-backed reviewed-terminal evidence together with validated server-
certificate policy. It applies the same exact decoded target-object check, then
retains the policy-backed construction, its exact validated
`X509NameValuePolicy`, reviewed-root selection, and complete target policy in
one non-copyable result. Generic callback construction or generic reviewed-path
evidence cannot produce this stronger result, and callers cannot discard the
Name policy while keeping both endpoint proofs. This provenance-preserving
composition does not make the available Name rules complete RFC 5280 Name
equality, authenticate path links, apply issuer or anchor constraints, check
revocation, perform TLS or HTTP, authorize OAuth transport, or grant network
authority. Concrete HTTPS and live-provider readiness remain unavailable.
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
discard the reviewed terminal. The fixed-shape binder no longer accepts caller-
supplied initial path-length state: after the exact two-link shape and retained
endpoint identities are checked, it derives the RFC 5280 initial capacity of
one intermediate from that construction and advances it using only the
complete retained intermediate policy. The lower-level caller-owned transition
remains available for future generalized traversal, but it cannot directly
create this fixed reviewed-path proof. This composition does not apply anchor
constraints, target policy, revocation, generalized path validation, TLS,
HTTP, OAuth transport, or network authority.
A bounded construction-bound target-policy follow-up now consumes that complete
authenticated reviewed path together with complete validated server-certificate
policy. It fails closed unless the server-policy certificate is the same exact
decoded target object retained by the constructed path, then returns distinct
non-copyable evidence retaining both complete inputs. This prevents callers
from attaching hostname, validity, purpose, end-entity role, critical-extension,
and algorithm policy for one certificate to a separately constructed and
authenticated target, including a byte-equal re-decode. The result retains the
construction-derived fixed-shape path-length state and does not apply anchor or
intermediate name constraints, other anchor policy, revocation, generalized
path validation, TLS, HTTP, OAuth transport, or network authority.
A bounded construction-bound anchor-policy follow-up now consumes that exact
target-policy result and applies the selected reviewed terminal's exact
supported DNS constraints to the exact target certificate retained by the
construction. Entries without out-of-band name constraints retain the complete
construction-bound proof without DNS evidence. Constrained entries reuse the
repository-owned bounded NameConstraints and Subject Alternative Name decoders,
require every target dNSName to satisfy at least one permitted DNS subtree, and
retain optional payload-free count evidence. Callers cannot substitute another
constructed path, reviewed terminal, target certificate, or SAN collection.
The result retains the construction-derived fixed-shape path-length state and
does not apply intermediate name constraints, excluded or non-DNS anchor
constraints, other anchor policy, revocation, generalized path validation,
TLS, HTTP, OAuth transport, or network authority.
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
certificate. This fixed two-link boundary retains construction-derived path-
length state and does not apply excluded or non-DNS anchor constraints, other
anchor policy, revocation, generalized path validation, TLS, HTTP, OAuth
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
A bounded target-SAN follow-up now consumes that complete non-copyable server
policy and reopens only the exact Subject Alternative Name extension index
already retained by its successful identity result. Under a new explicit
caller-owned ASN.1 budget it retains the decoded DNS/IP collection beside the
complete server policy, so later NameConstraints composition cannot substitute
another certificate or SAN extension. Missing indexed evidence and structural
decode failures fail closed with payload-free categories. This is retained
public name evidence only: it does not evaluate name constraints, authenticate
an issuer, construct or validate a path, establish trust, check revocation,
perform TLS or HTTP, authorize OAuth transport, or cause an external effect;
concrete HTTPS remains unavailable.
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
A bounded IP constraint evaluator now composes the shared repository-owned IP
name-constraint policy for every retained IPv4 or IPv6 address-and-mask subtree
before examining any presented address. Masks must contain one contiguous
most-significant-bit prefix, masked-out address bits do not affect subnet
matching, and matching never crosses address families. Excluded subtrees take precedence; when a
permitted IP set is present, every presented address must fall within at least
one permitted prefix. The IP-only entry point fails closed while DNS subtrees
remain unevaluated, matching the existing DNS-only refusal of retained IP
subtrees. A combined bounded entry point evaluates both supported name forms
and releases only payload-free DNS and IP counts after both policies pass, so
mixed critical constraints cannot silently lose one form. Inputs are typed,
caller-supplied SAN values; errors retain only a stable category and collection
index. This does not bind either result to a certificate, intermediate,
selected anchor, or constructed path, process any other GeneralName form,
generalize path validation, check revocation, perform TLS or HTTP, authorize
OAuth transport, access a credential, or cause an external effect. Concrete
HTTPS remains unavailable.
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
A bounded component follow-up now accepts explicit borrowed permitted-DNS and
excluded-DNS subtree slices separately from presented DNS names. It retains the
same allocation-free syntax validation, exclusion precedence, wildcard rules,
and 256-entry ceilings while allowing later path policy to compose DNS and IP
evaluation over one decoded mixed-form NameConstraints value. The existing
whole-value DNS entry point still fails closed whenever any retained IP
subtree remains unevaluated. The component entry point does not itself prove
that IP constraints were evaluated, bind names or constraints to exact
certificate/path evidence, establish trust, check revocation, perform TLS or
HTTP, authorize OAuth transport, or cause an external effect; concrete HTTPS
remains unavailable.
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
A bounded `x509-ip-name-constraint-policy` prerequisite now owns the RFC 5280
IPv4 and IPv6 subnet-matching rule independently of DER parsing. Caller policy
supplies ordered permitted and excluded `iPAddress` constraint values in the
standard address-plus-CIDR-mask form and presented Subject Alternative Name IP
address octets. The allocation-free evaluator requires exact 8- or 32-octet
constraints, exact 4- or 16-octet presented addresses, contiguous CIDR masks,
fixed collection ceilings, matching address families, exclusion precedence,
and membership in a permitted subnet whenever a permitted set exists. Stable
failures retain only a category and collection index. This policy does not
decode NameConstraints or Subject Alternative Name, extract certificate names,
bind evidence to a certificate or path, process DNS or other GeneralName forms,
construct or validate a path, verify signatures, grant trust, check revocation,
perform TLS or HTTP, authorize OAuth transport, or cause an external effect;
concrete HTTPS remains unavailable.
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
A bounded `x509-ocsp-response-nonce-policy` prerequisite now applies the strict
RFC 8954 response-echo branch to one already decoded `ResponseData` value. The
caller supplies the exact nonce sent in its corresponding request; this policy
requires the modern 32-octet client nonce, requires one non-critical
`id-pkix-ocsp-nonce` response extension and no other outer response extension,
decodes the extension's embedded `Nonce` OCTET STRING without resetting the
caller's DER depth or element budget, and compares all 32 response octets with
the repository-owned full-scan equality primitive. Missing, malformed,
critical, unexpected, or mismatched response extensions fail closed with
stable payload-free categories. Non-copyable success evidence retains the
exact `ResponseData` object and requested nonce, preventing a detached echo
verdict from being attached to another response or request. The required-echo
branch deliberately rejects RFC 8954's permitted responder
nonce omission; a deployment choosing a nonce-less freshness strategy needs a
separate policy and cannot reinterpret this rejection as a revocation verdict.
This effect-free policy does not generate randomness, construct or send an
OCSP request, authenticate the response signature, authorize a responder, bind
status to a certificate or path, decide freshness or revocation, fetch or cache
data, read a clock, establish trust, perform TLS or HTTP, authorize OAuth
transport, or grant network authority. It is not yet consumed by the
authenticated target-revocation composition, so concrete HTTPS remains
unavailable.
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

A bounded `x509-ocsp-rsa-signature-routing` prerequisite now consumes the
non-copyable response/key evidence from `x509-ocsp-rsa-algorithm-policy` and
routes its selected profile index through an explicit caller-owned table to
one reviewed fixed repository-owned verifier. The table vocabulary covers the
RSA PKCS #1 v1.5 SHA-256, SHA-384, and SHA-512 OCSP verifiers. Missing and
duplicate routes fail before cryptography, while a route that names a verifier
inconsistent with the selected profile fails that verifier's exact algorithm
identifier and parameter checks. Success retains both the consumed algorithm
policy evidence and the fixed-verifier result, binding the same exact response
and validated key across policy selection, routing, and cryptographic
verification. Routes and inputs are public policy and response/key data, so
this layer performs no credential access, edit, signing operation, provider
action, audit or trace emission, I/O, or external effect. It does not authorize
or match a responder, bind `CertID` or status to a certificate or constructed
path, apply certificate role or extension policy, decide freshness or
revocation, establish trust, fetch data, read a clock, or grant TLS, HTTP,
OAuth transport, or network authority. Concrete HTTPS and live-provider
readiness remain unavailable.

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
limits. A new data-driven entry point then validates the exact retained response
and derived key against caller-owned RSA algorithm profiles and routes the
selected profile through the repository-owned SHA-256, SHA-384, or SHA-512
PKCS #1 v1.5 verifier. The original fixed SHA-256 entry point remains as a
compatibility surface for current callers. Both non-copyable results retain the
identifier, certificate-policy, and response-signature evidence so the matched
response, policy-approved candidate, selected profile, route, or verified key
cannot be silently substituted. This public-data composition performs no
credential access, edit, signing operation, provider action, audit or trace
emission, I/O, or external effect. It authenticates only one policy-approved
delegated responder candidate to one response. It does not prove that the
target certificate's issuer signed or authorized the responder certificate,
construct or validate a responder path, establish trust, check responder-
certificate revocation, bind a `CertID` or revocation decision, fetch or cache
data, read a clock, perform TLS or HTTP, authorize OAuth transport, or grant
network authority. Concrete HTTPS and live-provider readiness remain
unavailable.

A bounded `x509-ocsp-issuer-bound-responder` composition now consumes either
the fixed RSA SHA-256 or the data-driven authenticated-responder evidence and
one exact repository-owned RSA SHA-256 issuer link for the responder
certificate. The additive data-driven result preserves the caller-selected
SHA-256, SHA-384, or SHA-512 response-signature route without claiming that the
responder-certificate issuer-link algorithm is routed. Both entry points reject
evidence whose responder certificates are distinct decoded objects, including
byte-equal re-decodes, before returning non-copyable evidence retaining both
proofs. Together, those proofs establish that one exact candidate issuer with
an issuing-CA role signed the policy-approved OCSP-signing responder certificate,
whose key signed the exact response. This pure public-data composition performs
no credential access, edit, signing operation, provider action, audit or trace
emission, I/O, or external effect. It does not establish that the candidate
issuer issued any particular target certificate, bind a `CertID` or revocation
decision, construct or validate a responder path, establish trust, check the
responder certificate's revocation, fetch or cache data, read a clock, perform
TLS or HTTP, authorize OAuth transport, or grant network authority. Concrete
HTTPS and live-provider readiness remain unavailable.

An additive data-driven entry point in the same composition now authenticates
the responder-certificate issuer link without fixing that certificate
signature to SHA-256. It consumes one routed authenticated-responder proof,
independent certificate-bound algorithm policy for that exact responder
certificate, certificate-wide issuing-CA role policy and certificate-bound RSA
algorithm/key policy for one exact candidate issuer, caller-owned Name policy,
caller-owned profile routes, and repository-owned RSA operation limits. Exact
responder and issuer object-identity checks plus the issuing-CA role check run
before Name policy or cryptography; byte-equal independent decodes fail closed.
The Name callback runs exactly once, after which the selected child profile is
routed through the existing repository-owned RSA PKCS #1 v1.5 SHA-256,
SHA-384, or SHA-512 verifier. Non-copyable success evidence retains the routed
response authentication, responder algorithm policy, issuer role policy, and
selected certificate-signature proof so none can be substituted after
verification. Existing fixed-link entry points remain unchanged. This
public-data composition performs no credential access, edit, signing
operation, provider action, storage access, audit or trace emission, I/O, or
external effect. It does not establish that the candidate issuer issued a
particular target certificate, bind a `CertID` or revocation decision,
construct or validate a responder path, establish trust, check responder
revocation, fetch or cache data, read a clock, perform TLS or HTTP, authorize
OAuth transport, or grant network authority. Concrete HTTPS and live-provider
readiness remain unavailable.

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

An allocation-free `x509-ocsp-cert-id-encoding` prerequisite now validates one
exact canonical hash `AlgorithmIdentifier` and one exact canonical positive
certificate serial number under caller-owned ASN.1 limits, retains those exact
inputs with two caller-owned issuer-hash byte strings, and encodes the RFC 6960
`CertID` structure into caller-owned output only after complete nested-length
and capacity preflight. It neither selects nor authorizes the algorithm,
computes either hash, requires a particular digest length, binds the values to
certificates, proves an issuer relationship, or creates a complete OCSP
request. It performs no certificate-path, trust, revocation, clock, nonce,
DNS, TLS, HTTP, I/O, audit, trace, credential, OAuth-transport, or network
operation. Concrete HTTPS and live-provider readiness remain unavailable.

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
that exact good-and-fresh target/issuer result, either a fixed RSA SHA-256 or a
data-driven issuer-bound authenticated responder, and one repository-owned RSA
SHA-256 issuer link for the target certificate. The additive data-driven result
preserves the caller-selected SHA-256, SHA-384, or SHA-512 response-signature
route without claiming that either certificate issuer-link algorithm is routed.
An additional routed-issuer result consumes the exact data-driven responder-
certificate issuer proof plus exact target-certificate and candidate-issuer
algorithm policy. It requires the responder proof, revocation binding, target
policy, and issuer policy to retain the same exact certificate objects before
invoking caller-owned target-to-issuer Name policy once, then routes the target
certificate signature through the repository-owned RSA PKCS #1 v1.5 SHA-256,
SHA-384, or SHA-512 verifier. The distinct non-copyable result therefore retains
data-driven response, responder-certificate, and target-certificate signature
evidence without widening either existing API.
All entry points require the revocation result's exact `SingleResponse` object
to occur inside the signed `BasicOCSPResponse`, require the target link to retain
the exact target certificate, and require both certificate links and the
`CertID` binding to retain the same exact issuer certificate object.
It requires the signed response to contain exactly one `SingleResponse`, so an
unexamined entry encoded under another hash algorithm cannot conflict with the
selected target status, and revalidates the responder certificate at the exact
revocation observation time. Byte-equal re-decodes fail closed. Until outer
extension and nonce policy is implemented, any `responseExtensions` collection
is rejected. Non-copyable success evidence retains all three complete proofs
and the exact-time responder-validity result, establishing only that the same
candidate issuer signed the target and authorized the then-valid delegated
responder whose key signed the exact unambiguous response containing the good-
and-fresh target status. This public-data composition performs no credential
access, edit, signing operation, provider action, audit or trace emission, I/O,
or external effect. It does not construct or validate a certificate path,
establish issuer trust, apply responder-path or responder-
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

A bounded data-driven follow-up in the same package now joins that exact
authenticated OCSP target-revocation evidence to the routed constructed
reviewed-intermediate server-path evidence. The routed input retains
certificate-bound child algorithm policy, certificate-wide issuer role policy,
certificate-bound issuer RSA algorithm/key policy, and the selected
repository-owned RSA PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512 verifier for each
of the two exact constructed adjacencies. Binding still requires both inputs to
retain the same exact decoded target and intermediate objects; independently
decoded byte-equal certificates fail closed. It then revalidates those exact
path certificates at the OCSP proof's retained observation time and keeps both
fresh validity tokens with both complete non-copyable proofs. Stable failures
preserve target-before-issuer identity and validity ordering without exposing
certificate, response, key, signature, status, hostname, algorithm, or time
payloads. This pure public-data composition performs no new cryptography,
credential access, signing, storage, clock acquisition, audit event, trace
emission, I/O, or external effect. It does not yet reify the routed responder
path, apply routed responder name or no-check policy, generalize certificate-
path validation, check responder or intermediate revocation, interpret
unsupported OCSP extensions, fetch or cache data, implement TLS or HTTP,
authorize OAuth transport, or grant network authority. Concrete HTTPS and
live-provider readiness remain unavailable.

An additive routed-response follow-up in the same package now joins the
data-driven authenticated OCSP target-revocation evidence to that same routed
constructed reviewed-intermediate server-path evidence. The OCSP proof retains
the caller-selected RSA PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512 response-
signature verifier, while its responder-certificate and target-certificate
issuer links remain deliberately fixed to repository-owned RSA SHA-256. The
server-path proof independently retains the selected SHA-256, SHA-384, or
SHA-512 verifier for each exact target-to-intermediate and intermediate-to-root
adjacency. Binding requires the two complete non-copyable inputs to retain the
same exact decoded target and intermediate objects, rejects byte-equal
independent decodes, revalidates both path certificates at the OCSP proof's
retained observation time, and keeps both fresh validity tokens with both
complete proofs. Existing fixed-response and fixed-path entry points remain
unchanged. This pure public-data composition performs no new cryptography,
credential access, signing, storage, clock acquisition, audit event, trace
emission, I/O, or external effect. It does not route either certificate link
inside the OCSP proof, reify a routed responder path, apply responder name or
no-check policy, generalize certificate-path validation, check responder or
intermediate revocation, interpret unsupported OCSP extensions, fetch or cache
data, implement TLS or HTTP, authorize OAuth transport, or grant network
authority. Concrete HTTPS and live-provider readiness remain unavailable.

An additive routed-issuer follow-up in the same package now joins the fully
routed OCSP target-revocation evidence to that same routed constructed
reviewed-intermediate server-path evidence. The OCSP proof retains the
caller-selected repository-owned RSA PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512
verifier for the response signature, responder-certificate issuer signature,
and target-certificate issuer signature. The server-path proof independently
retains the selected verifier for each exact target-to-intermediate and
intermediate-to-root adjacency. Binding requires both complete non-copyable
inputs to retain the same exact decoded target and intermediate objects,
rejects byte-equal independent decodes, revalidates both path certificates at
the OCSP proof's retained observation time, and keeps both fresh validity
tokens with both complete proofs. Existing fixed-response, routed-response,
and routed-path entry points remain unchanged. This pure public-data
composition performs no new cryptography, credential access, signing, storage,
clock acquisition, provider action, audit event, trace emission, I/O, or
external effect. It does not reify a routed responder path, apply responder
name or no-check policy, generalize certificate-path validation, check
responder or intermediate revocation, interpret unsupported OCSP extensions,
fetch or cache data, implement TLS or HTTP, authorize OAuth transport, or grant
network authority. Concrete HTTPS and live-provider readiness remain
unavailable.

An additive routed-responder projection in the same package now consumes that
complete fully routed reviewed-server revocation proof and reifies its already-
proven delegated responder-to-reviewed-root path as the existing distinct non-
copyable responder-path type. The consumed proof already retains the exact
responder certificate, its routed issuer-signature proof, the shared exact
intermediate, that intermediate's routed reviewed-root adjacency, responder and
intermediate validity at the OCSP observation time, and the complete routed
target-revocation and reviewed-server-path evidence. The infallible nominal
projection retains that proof intact and accepts no caller-supplied certificate,
issuer, path, algorithm, key, signature, time, or policy input. It performs no
new parsing, cryptography, credential access, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, or external
effect. It does not apply selected-anchor or intermediate DNS constraints to
the routed responder, validate responder no-check policy, check responder or
intermediate revocation, generalize certificate-path validation, implement TLS
or HTTP, authorize OAuth transport, or grant network authority. Concrete HTTPS
and live-provider readiness remain unavailable.

A bounded routed-policy follow-up in the same package consumes that exact
reified fully routed responder path and applies the selected Mozilla-reviewed
terminal's supported out-of-band DNS constraints to the exact authenticated
responder certificate. The responder and selected root constraints are both
derived through the retained routed proof; callers cannot substitute a
certificate, SAN collection, snapshot entry, path, algorithm, key, signature,
or time. Entries without constraints succeed without reading Subject
Alternative Name or manufacturing DNS evidence. Constrained entries decode
the supported permitted-DNS subset and exact responder SAN under separate
caller-owned ASN.1 limits, require every responder `dNSName` to comply, and
retain only count evidence. Stable failures expose no certificate, name,
constraint, response, key, signature, or status payload. This pure
public-certificate-data policy performs no credential access or editing,
signing, storage, clock acquisition, provider action, audit event, trace
emission, I/O, network access, or other external effect. It does not apply the
retained intermediate's DNS constraints, validate responder no-check policy,
check responder or intermediate revocation, generalize path construction or
validation, implement TLS or HTTP, authorize OAuth transport, or grant network
authority. Concrete HTTPS and live-provider readiness remain unavailable.

A second bounded routed-policy follow-up,
`validate_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_intermediate_dns_policy`,
consumes that complete selected-anchor responder-policy proof and applies the
exact retained intermediate's supported critical DNS constraints to the exact
authenticated responder certificate. Both certificates are derived exclusively
through the consumed fully routed strongest-Name-policy proof;
callers cannot substitute a responder, SAN collection, intermediate, path,
algorithm, key, signature, or time. An absent intermediate NameConstraints
extension succeeds without reading Subject Alternative Name. A present
extension must be critical and decode as the supported permitted-and-excluded-
DNS subset under separate caller-owned ASN.1 limits, and every responder
`dNSName` must comply. Success retains only count evidence; stable failures
expose no certificate, name, constraint, response, key, signature, status, or
time payload. This pure public-certificate-data policy performs no credential
access or editing, signing, storage, clock acquisition, provider action, audit
event, trace emission, I/O, network access, or other external effect. It does
not validate routed responder no-check policy, check responder or intermediate
revocation, generalize path construction or validation, implement TLS or HTTP,
authorize OAuth transport, or grant network authority. Concrete HTTPS and
live-provider readiness remain unavailable.

A third bounded routed-policy follow-up consumes that complete routed
intermediate-DNS proof and derives the exact authenticated responder
certificate exclusively through the retained fully routed proof.
`validate_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_no_check_policy`
accepts only that complete proof and requires the structurally unique
`id-pkix-ocsp-nocheck` extension already admitted by
certificate-wide role policy and validates that the extension value is exact
DER `NULL`. RFC 6960 recommends, but does not require, non-critical marking; a
critical instance can reach this semantic gate only when the prerequisite role
policy explicitly recognized the OID. Missing or malformed values fail with
the existing stable payload-free categories, and success retains the complete
prior proof under a distinct non-copyable type. Callers cannot substitute a
responder certificate, extension, path, algorithm, key, signature, or time.
This pure public-certificate-data policy performs no credential access or
editing, signing, storage, clock acquisition, provider action, audit event,
trace emission, I/O, network access, or other external effect. It does not
check responder or intermediate revocation, generalize path construction or
validation, implement TLS or HTTP, authorize OAuth transport, or grant network
authority. Concrete HTTPS and live-provider readiness remain unavailable.

An alternative bounded routed revocation follow-up consumes that same complete
intermediate-DNS proof plus a separate fully authenticated, fully routed OCSP
target-revocation proof.
`bind_x509_ocsp_routed_issuer_accumulated_name_policy_reviewed_responder_revocation`
accepts only those complete proofs. The second proof must name the exact routed
responder certificate as its target, the exact retained intermediate as its
issuer, and the original server-status OCSP observation time as its own
explicit observation time. Every certificate comparison uses decoded-object
identity,
so byte-equal independent decodes fail closed, and stable target, issuer, and
observation-time mismatch categories retain no certificate, response, key,
signature, status, hostname, or time payload. Success retains both complete
non-copyable inputs under a distinct type; callers cannot substitute raw
status, freshness endpoints, certificates, paths, algorithms, keys,
signatures, or time. This pure public-certificate-data composition performs no
credential access or editing, signing, storage, clock acquisition, provider
action, audit event, trace emission, I/O, network access, or other external
effect. It proves only authenticated revocation status for the exact routed
responder at the original observation time. Revocation policy for the second
proof's own responder and for the retained intermediate, generalized path
construction or validation, TLS, HTTP, OAuth transport, and network authority
remain unavailable. Concrete HTTPS and live-provider readiness remain
unavailable.

The strongest Name-policy-backed responder-revocation result is now also
admitted directly to the existing generic retained-intermediate revocation
binding boundary. A private repository-owned evidence adapter derives the
target-nearest intermediate, selected Mozilla-reviewed root, and original
server-status observation time only through the retained strong proof;
callers cannot substitute a certificate, root, status proof, or time. The
existing stable target-issuer-time validator can therefore bind separately
authenticated status for that exact intermediate without a second public
entry point or an external effect. This closes only the strongest proof's
retained-intermediate binding edge. Policy for either added OCSP proof's own
responder, complete revocation or Name/path validation, TLS, HTTP, OAuth
transport, network authority, and live-provider readiness remain unavailable.

A bounded follow-up projection consumes that complete separately authenticated
original-responder branch and reifies the added revocation proof's own exact
delegated responder-to-intermediate-to-reviewed-root path under a distinct
non-copyable type. The consumed branch already retains the added proof's exact
authenticated responder, its routed repository-owned RSA response and
certificate-signature verification, observation-time validity, exact
intermediate issuer, and the selected Mozilla-reviewed root. The infallible
projection retains the entire branch and accepts no substitute certificate,
path, root, algorithm, key, signature, status, freshness endpoint, or time.
This pure public-certificate-data composition performs no credential access or
editing, signing, storage, clock acquisition, provider action, audit event,
trace emission, I/O, network access, or other external effect. It does not
apply reviewed-root or intermediate name policy to that added responder,
establish its RFC 6960 no-check or separately authenticated revocation policy,
complete revocation policy, generalize path construction or validation,
implement TLS or HTTP, authorize OAuth transport, or grant network authority.
Concrete HTTPS and live-provider readiness remain unavailable.

A bounded policy follow-up consumes that exact added responder path and
applies the selected Mozilla-reviewed root's supported out-of-band DNS
constraints to the separately authenticated proof's exact fully routed
responder certificate. The responder and reviewed-root constraint bytes are
derived only through the consumed proof; callers cannot substitute a
certificate, SAN collection, snapshot entry, root, path, algorithm, key,
signature, status, freshness endpoint, or time. Entries without out-of-band
constraints succeed without reading Subject Alternative Name. Constrained
entries decode the supported permitted-DNS subset and exact responder SAN
under separate caller-owned ASN.1 limits, require every responder `dNSName` to
comply, and retain only count evidence. Stable failures expose no certificate,
name, constraint, response, key, signature, status, root, or time payload.
This pure public-certificate-data policy performs no credential access or
editing, signing, storage, clock acquisition, provider action, audit event,
trace emission, I/O, network access, or other external effect. It does not
apply intermediate, excluded, or non-DNS name constraints, establish no-check
or separately authenticated revocation policy for the added responder,
implement complete revocation policy, generalize path construction or
validation, or provide TLS, HTTP, OAuth transport, or network authority.
Concrete HTTPS and live-provider readiness remain unavailable.

The same generic projection and selected-root DNS-policy boundary admit the
strongest Name-policy-backed arbitrary-depth original proof above. A private
evidence adapter derives the selected reviewed root's exact optional
constraint bytes exclusively through that retained proof, without a second
public entry point or any detached certificate, root, constraint, path,
signature, status, or time input. This composition stops at selected-root DNS
policy for the added responder; intermediate DNS, no-check, further revocation,
complete Name/path validation, TLS, HTTP, OAuth transport, and network
authority remain unavailable.

A further bounded policy follow-up consumes that complete reviewed-root DNS
proof and applies the separately authenticated proof's exact retained
intermediate NameConstraints to its exact responder certificate. Both
certificates are derived only through the consumed repository-owned evidence;
callers cannot substitute a certificate, SAN collection, intermediate, path,
root, algorithm, key, signature, status, freshness endpoint, or time. An
intermediate without NameConstraints succeeds without reading Subject
Alternative Name. A present extension must be critical; its supported
permitted and excluded DNS subtrees and the responder SAN are decoded under
separate caller-owned ASN.1 limits, every responder `dNSName` must comply, and
success retains only count evidence. Stable failures expose no certificate,
name, constraint, response, key, signature, status, root, or time payload. This
pure public-certificate-data policy performs no credential access or editing,
signing, storage, clock acquisition, provider action, audit event, trace
emission, I/O, network access, or other external effect. It does not establish
no-check or separately authenticated revocation policy for the added
responder, implement complete revocation policy, generalize path construction
or validation, or provide TLS, HTTP, OAuth transport, or network authority.
Concrete HTTPS and live-provider readiness remain unavailable.

A final bounded branch policy consumes that complete root-and-intermediate DNS
proof and requires RFC 6960 no-check semantics on the separately authenticated
proof's exact responder. The responder is derived only through the consumed
repository-owned evidence; callers cannot substitute a certificate, extension
set, path, root, algorithm, key, signature, status, freshness endpoint, or
time. One structurally unique `id-pkix-ocsp-nocheck` extension must be present
with exact DER `NULL` contents, and success retains the complete non-copyable
proof. Stable failures expose no certificate, extension, response, key,
signature, status, root, or time payload. This pure public-certificate-data
policy performs no credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network access,
or other external effect. It resolves only RFC 6960's trust-for-certificate-
lifetime branch for this responder and does not establish separately
authenticated revocation for it, implement complete revocation policy,
generalize path construction or validation, or provide TLS, HTTP, OAuth
transport, or network authority. Concrete HTTPS and live-provider readiness
remain unavailable.

A bounded composition follow-up consumes that responder-of-responder no-check
proof plus a separate fully authenticated, fully routed OCSP target-revocation
proof for the retained intermediate. It derives the exact retained
intermediate, selected Mozilla-reviewed root, and original server-status
observation time exclusively through the consumed no-check proof, then
requires the second proof to name those exact certificate objects and time.
Stable target, issuer, and observation-time mismatch categories retain no
certificate, response, key, signature, status, hostname, root, or time
payload. Success retains both complete non-copyable inputs under a distinct
type; callers cannot duplicate or substitute the consumed responder branch,
raw status, certificates, paths, roots, algorithms, keys, signatures, or time.
This pure public-certificate-data composition performs no credential access or
editing, signing, storage, clock acquisition, provider action, audit event,
trace emission, I/O, network access, or other external effect. It resolves the
original responder through separately authenticated status, resolves that
status proof's own responder through RFC 6960 no-check policy, and
authenticates status for the exact retained intermediate at the same
observation time. It does not establish revocation policy for the intermediate-
status proof's own responder, implement complete revocation policy, generalize
path construction or validation, or provide TLS, HTTP, OAuth transport, or
network authority. Concrete HTTPS and live-provider readiness remain
unavailable.

A subsequent infallible projection consumes that complete composition and
reifies the intermediate-status proof's exact authenticated responder-to-
reviewed-root path. The preceding proof already established certificate-wide
critical-extension and end-entity role policy, caller-selected algorithm/key
policy, OCSP-signing purpose, validity at the exact observation time,
repository-owned routed RSA verification for both the response and responder
certificate, and exact issuer identity with the selected Mozilla-reviewed
root. The nominal path wrapper retains the complete non-copyable composition
under a source-specific accessor, accepts no substitute certificate, path,
root, algorithm, key, signature, status, or time, and performs no additional
parsing or validation. This pure public-certificate-data projection performs
no credential access or editing, signing, storage, clock acquisition, provider
action, audit event, trace emission, I/O, network access, or other external
effect. It does not apply the selected root's supported out-of-band DNS
constraints to that responder, establish no-check or separately authenticated
revocation policy for it, implement complete revocation policy, generalize path
construction or validation, or provide TLS, HTTP, OAuth transport, or network
authority. Concrete HTTPS and live-provider readiness remain unavailable.

A bounded policy follow-up consumes that exact nested-source responder path
and applies the selected Mozilla-reviewed root's supported out-of-band DNS
constraints to the intermediate-status proof's exact fully routed
authenticated responder certificate. Both policy inputs are derived only
through the complete responder-of-responder no-check plus intermediate-status
composition; callers cannot substitute a certificate, SAN collection,
snapshot entry, root, path, algorithm, key, signature, status, or time. Entries
without constraints succeed without reading Subject Alternative Name.
Constrained entries decode the supported permitted-DNS subset and exact
responder SAN under separate caller-owned ASN.1 limits, require every responder
`dNSName` to comply, and retain only count evidence. Stable failures expose no
certificate, name, constraint, response, key, signature, status, root, or time
payload. The complete source remains available to the existing added-responder
no-check and separately authenticated revocation policies. This pure public-
certificate-data policy performs no credential access or editing, signing,
storage, clock acquisition, provider action, audit event, trace emission, I/O,
network access, or other external effect. It does not apply excluded or non-
DNS root constraints, implement complete revocation policy, generalize path
construction or validation, or provide TLS, HTTP, OAuth transport, or network
authority. Concrete HTTPS and live-provider readiness remain unavailable.

An independent bounded routed intermediate-revocation follow-up consumes the
complete fully routed reviewed server-revocation proof plus a separate fully
authenticated, fully routed OCSP target-revocation proof. The second proof
must name the exact retained intermediate certificate as its target, the exact
selected Mozilla-reviewed root as its issuer, and the original server-status
OCSP observation time as its own explicit observation time. Every certificate
comparison uses decoded-object identity, so byte-equal independent decodes
fail closed, and stable target, issuer, and observation-time mismatch
categories retain no certificate, response, key, signature, status, hostname,
root, or time payload. Success retains both complete non-copyable inputs under
a distinct type; callers cannot substitute raw status, freshness endpoints,
certificates, paths, roots, algorithms, keys, signatures, or time. This pure
public-certificate-data composition performs no credential access or editing,
signing, storage, clock acquisition, provider action, audit event, trace
emission, I/O, network access, or other external effect. This evidence remains
independent of responder policy: complete revocation composition still needs
either the exact responder no-check branch or separately authenticated exact
responder revocation. Revocation policy for the second proof's own responder,
generalized path construction or validation, TLS, HTTP, OAuth transport, and
network authority remain unavailable. Concrete HTTPS and live-provider
readiness remain unavailable.

A bounded no-check composition follow-up consumes an admitted fully routed
responder no-check proof plus a separate fully authenticated, fully routed
OCSP target-revocation proof for the retained intermediate. Private
repository-owned evidence now admits both the fixed-depth and strongest Name-
policy-backed server proofs while preventing external substitute
implementations. It derives the exact retained intermediate, selected
Mozilla-reviewed root, and original server-status OCSP observation time
exclusively through the consumed no-check proof, then requires the second proof
to name those exact certificate objects and time. Stable target, issuer, and
observation-time mismatch categories retain no certificate, response, key,
signature, status, hostname, root, or time payload. Success retains the
complete non-copyable no-check proof and intermediate-status proof under a
distinct type; callers cannot substitute raw status, certificates, paths,
roots, algorithms, keys, signatures, or time.
This pure public-certificate-data composition performs no credential access or
editing, signing, storage, clock acquisition, provider action, audit event,
trace emission, I/O, network access, or other external effect. It resolves the
original routed responder through RFC 6960 no-check policy and authenticates
status for the exact retained intermediate at the same observation time, but
does not establish revocation policy for the added proof's own responder.
Generalized path construction or validation, complete revocation policy, TLS,
HTTP, OAuth transport, and network authority remain unavailable. Concrete
HTTPS and live-provider readiness remain unavailable.

An alternative bounded authenticated-responder composition now consumes the
complete fully routed original responder-revocation proof plus a separate
fully authenticated, fully routed OCSP target-revocation proof for the retained
intermediate. Private repository-owned evidence interfaces derive the exact
retained intermediate, selected Mozilla-reviewed root, and original server-
status observation time from the first branch; downstream callers can use the
concrete proof types but cannot implement substitute evidence. The second
proof must name those exact certificate objects and time. Stable target,
issuer, and observation-time mismatch categories retain no certificate,
response, key, signature, status, hostname, root, or time payload. Success
retains both complete non-copyable branches under a distinct type. This pure
public-certificate-data composition performs no credential access or editing,
signing, storage, clock acquisition, provider action, audit event, trace
emission, I/O, network access, or other external effect. Revocation policy for
both added proofs' own responders, generalized path construction or validation,
complete revocation policy, TLS, HTTP, OAuth transport, and network authority
remain unavailable. Concrete HTTPS and live-provider readiness remain
unavailable.

A subsequent bounded projection consumes either the complete no-check plus
intermediate-status composition or the complete separately authenticated
original-responder plus intermediate-status composition and reifies the added
status proof's exact delegated responder-to-reviewed-root path as a distinct
non-copyable type. Private repository-owned evidence prevents an external
source implementation, and defaulted source parameters preserve the existing
no-check projection's public type shape.
The consumed inputs already prove that the added proof's exact responder
certificate passed certificate-wide critical-extension and end-entity role
policy, caller-selected algorithm/key policy, OCSP-signing purpose, and
validity at the exact observation time; that its response signature and
responder certificate signature selected repository-owned routed RSA
verifiers; and that its exact issuer is the same selected Mozilla-reviewed
root retained by the original proof. The infallible projection retains the
complete prior composition without accepting a substitute certificate, path,
root, algorithm, key, signature, status, or time and performs no additional
parsing or validation. This pure public-certificate-data composition performs
no credential access or editing, signing, storage, clock acquisition,
provider action, audit event, trace emission, I/O, network access, or other
external effect. It does not yet apply the selected root's supported
out-of-band DNS constraints to that responder, establish no-check or
separately authenticated revocation policy for it, generalize path
construction or validation, implement complete revocation policy, TLS, HTTP,
OAuth transport, or network authority. Concrete HTTPS and live-provider
readiness remain unavailable.

A bounded policy follow-up consumes each exact added-status responder path
and applies the selected Mozilla-reviewed root's supported out-of-band DNS
constraints to the exact fully routed authenticated responder certificate.
The responder and reviewed-root constraint bytes are derived only through the
consumed proof; the preceding identity binding already proves that this same
selected root issued the responder. Callers cannot substitute a certificate,
SAN collection, snapshot entry, root, path, algorithm, key, signature, status,
or time. Entries without out-of-band constraints succeed without reading
Subject Alternative Name. Constrained entries decode the supported permitted-
DNS subset and the exact responder SAN under separate caller-owned ASN.1
limits, require every responder `dNSName` to comply, and retain only count
evidence. Stable failures expose no certificate, name, constraint, response,
key, signature, status, root, or time payload. This pure public-certificate-
data policy performs no credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network access,
or other external effect. It does not apply excluded or non-DNS root
constraints, establish no-check or separately authenticated revocation policy
for the added responder, implement complete revocation policy, generalize path
construction or validation, or provide TLS, HTTP, OAuth transport, or network
authority. Concrete HTTPS and live-provider readiness remain unavailable.

A following bounded revocation-policy slice consumes that complete added-
responder reviewed-root DNS proof and applies RFC 6960's trust-for-certificate-
lifetime branch to the exact authenticated responder. The responder
certificate is derived only through the retained proof; callers cannot
substitute a certificate or extension value. The structurally unique
`id-pkix-ocsp-nocheck` extension must be present and its value must be exact DER
`NULL`. A critical instance can reach this semantic gate only when the earlier
certificate-wide role policy explicitly recognized its OID; RFC 6960
recommends, but does not require, non-critical marking. Success retains the
complete reviewed-root DNS proof under a distinct non-copyable type. Missing
or malformed values reuse stable failures that expose no certificate,
extension, response, key, signature, status, hostname, root, or time payload.
This pure public-certificate-data policy performs no credential access or
editing, signing, storage, clock acquisition, provider action, audit event,
trace emission, I/O, network access, or other external effect. It resolves
only the no-check branch for the added responder; separately authenticated
revocation for responders without the extension, complete revocation policy,
generalized path construction or validation, TLS, HTTP, OAuth transport, and
network authority remain unavailable. Concrete HTTPS and live-provider
readiness remain unavailable.

The direct branch can then nominally complete revocation policy for its one
exact retained fixed path. The consumed type proves that authenticated good-
and-fresh OCSP status covers the server target and retained intermediate,
while exact RFC 6960 no-check policy covers the original responder and added
intermediate-status responder. The same generic path admits the strongest
Name-policy-backed original proof without a duplicate detached-input API. The
selected Mozilla-reviewed root remains the trust anchor rather than a
revocation target. The infallible non-copyable result retains all prerequisite
evidence and accepts no substitute certificate, path, root, algorithm, key,
signature, status, or time. This pure public-certificate-data composition
performs no parsing, cryptography, credential access or editing, signing,
storage, clock acquisition, provider action, audit event, trace emission, I/O,
network access, or other external effect. It proves complete revocation policy
only for this one direct retained fixed path; it does not generalize revocation
or path construction and validation, fetch, cache, acquire, or refresh
revocation data, or provide TLS, HTTP, OAuth transport, or network authority.
Concrete HTTPS and live-provider readiness remain unavailable.

A branch-specific bounded completion slice then consumes that exact nested
no-check composition after the server target, original responder, responder-
status proof's own responder, intermediate, and intermediate-status proof's
responder have each been covered by authenticated good-and-fresh OCSP status
or exact RFC 6960 no-check policy. The selected Mozilla-reviewed root remains
the trust anchor rather than a revocation target. The infallible non-copyable
result retains all prerequisite evidence and accepts no substitute
certificate, path, root, algorithm, key, signature, status, or time. This pure
public-certificate-data composition performs no parsing, cryptography,
credential access or editing, signing, storage, clock acquisition, provider
action, audit event, trace emission, I/O, network access, or other external
effect. It proves complete revocation policy only for this one retained fixed
path; it does not generalize path construction or validation, fetch, cache,
acquire, or refresh revocation data, or provide TLS, HTTP, OAuth transport, or
network authority. Concrete HTTPS and live-provider readiness remain
unavailable.

An alternative bounded added-responder revocation follow-up consumes that
same complete reviewed-root DNS proof plus a separate fully authenticated,
fully routed OCSP target-revocation proof. Private repository-owned evidence
interfaces expose the exact added responder, its selected reviewed-root
issuer, and the explicit observation time from the retained intermediate-
status proof, plus the corresponding target, issuer, and time from the second
proof. External callers can use the concrete repository-owned authenticated
proof types but cannot implement substitute evidence. The second proof must
name the same exact decoded responder and reviewed root objects and the same
observation time. Stable target, issuer, then time mismatch categories retain
no certificate, response, key, signature, status, hostname, root, or time
payload. Success retains both complete non-copyable inputs under a distinct
type. This pure public-certificate-data composition performs no credential
access or editing, signing, storage, clock acquisition, provider action, audit
event, trace emission, I/O, network access, or other external effect. It does
not establish revocation policy for the second proof's own responder, compose
complete revocation policy, generalize path construction or validation, or
provide TLS, HTTP, OAuth transport, or network authority. Concrete HTTPS and
live-provider readiness remain unavailable.

That alternative branch now has a bounded infallible projection for the added
revocation proof's own exact delegated responder-to-reviewed-root path. The
non-copyable result retains the complete generic source, selected-root DNS
proof, target binding, and fully routed authenticated revocation proof. It
accepts no substitute certificate, path, root, algorithm, key, signature,
status, or time and performs no parsing, cryptography, credential access or
editing, signing, storage, clock acquisition, provider action, audit event,
trace emission, I/O, network access, or other external effect. Reviewed-root
and intermediate name policy for that delegated responder, no-check or
separately authenticated revocation policy for it, complete revocation policy,
generalized path construction or validation, TLS, HTTP, OAuth transport, and
network authority remain unavailable. Concrete HTTPS and live-provider
readiness remain unavailable.

A following bounded policy slice consumes that exact delegated-responder path
and applies the selected Mozilla-reviewed root's supported out-of-band DNS
constraints to the added revocation proof's authenticated responder. Both the
responder certificate and reviewed-root constraint bytes are derived only
through the consumed proof; callers cannot substitute a certificate, SAN
collection, snapshot entry, root, path, algorithm, key, signature, status, or
time. Entries without constraints succeed without reading Subject Alternative
Name. Constrained entries decode the supported permitted-DNS subset and the
exact responder SAN under separate caller-owned ASN.1 limits, require every
responder `dNSName` to comply, and retain only count evidence. Stable failures
expose no certificate, name, constraint, response, key, signature, status,
root, or time payload. This pure public-certificate-data policy performs no
credential access or editing, signing, storage, clock acquisition, provider
action, audit event, trace emission, I/O, network access, or other external
effect. It does not apply excluded or non-DNS root constraints or the retained
intermediate's constraints, establish no-check or separately authenticated
revocation policy for that responder, compose complete revocation policy,
generalize path construction or validation, or provide TLS, HTTP, OAuth
transport, or network authority. Concrete HTTPS and live-provider readiness
remain unavailable.

A following bounded no-check slice consumes that complete reviewed-root DNS
proof and derives the added revocation proof's exact authenticated delegated
responder only through the retained path. It requires one structurally unique
`id-pkix-ocsp-nocheck` extension whose contents are exact DER `NULL`. RFC 6960
recommends but does not require a non-critical extension, so a critical
instance can reach this semantic gate only after certificate-wide role policy
recognizes its OID. Success retains the complete generic non-copyable proof.
This pure public-certificate-data policy performs no credential access or
editing, signing, storage, clock acquisition, provider action, audit event,
trace emission, I/O, network access, or other external effect. It resolves
only the no-check branch for that exact responder and does not establish
separately authenticated recursive responder revocation, complete revocation
policy, generalized path construction or validation, TLS, HTTP, OAuth
transport, or network authority. Concrete HTTPS and live-provider readiness
remain unavailable.

A parallel bounded recursive-revocation slice consumes that same complete
reviewed-root DNS proof plus another fully authenticated, fully routed OCSP
target-revocation proof. Repository-owned private evidence derives the added
proof's own exact authenticated delegated responder, the exact selected
Mozilla-reviewed root already bound as issuer, and the original explicit
observation time only through the retained proof; external callers cannot
implement substitute evidence. The recursive proof must target that same
decoded responder object, name that same decoded reviewed-root object, and use
that same observation time. Stable target, issuer, then time failures expose
no certificate, response, key, signature, status, hostname, root, or time
payload. Success retains the complete generic source and both non-copyable
authenticated proof layers. This pure public-certificate-data composition
performs no credential access or editing, signing, storage, clock acquisition,
provider action, audit event, trace emission, I/O, network access, or other
external effect. It does not establish revocation policy for the recursive
proof's own responder, complete revocation policy, generalized path
construction or validation, TLS, HTTP, OAuth transport, or network authority.
Concrete HTTPS and live-provider readiness remain unavailable.

A bounded infallible projection now consumes that exact recursive proof and
reifies the recursive proof's own delegated responder-to-reviewed-root path as
a distinct non-copyable type. The retained binding already proves the exact
recursive target, reviewed-root issuer, observation time, repository-owned RSA
signature routes, certificate role policy, and responder validity. The
projection retains the complete generic source and both authenticated proof
layers without accepting a substitute certificate, path, root, algorithm,
key, signature, status, or time. This pure public-certificate-data composition
performs no parsing, cryptography, credential access or editing, signing,
storage, clock acquisition, provider action, audit event, trace emission, I/O,
network access, or other external effect. It does not apply reviewed-root name
policy to that recursive responder, establish its RFC 6960 no-check or
separately authenticated revocation policy, complete revocation policy,
generalize path construction or validation, implement TLS or HTTP, authorize
OAuth transport, or grant network authority. Concrete HTTPS and live-provider
readiness remain unavailable.

A following bounded name-policy slice consumes that exact recursive responder
path and derives the recursive proof's authenticated delegated responder plus
the selected Mozilla-reviewed root's optional out-of-band constraint bytes
only through the retained generic source. An unconstrained root succeeds
without reading responder Subject Alternative Name. A constrained root is
decoded under a caller-owned ASN.1 limit, the responder SAN is decoded under a
separate caller-owned limit, every responder `dNSName` must satisfy the
supported permitted-DNS subset, and success retains only count evidence beside
the complete proof. Stable failures expose no certificate, name, constraint,
response, key, signature, status, root, or time payload. This pure public-
certificate-data policy performs no credential access or editing, signing,
storage, clock acquisition, provider action, audit event, trace emission, I/O,
network access, or other external effect. It does not apply excluded or
non-DNS root constraints or intermediate constraints, establish RFC 6960 no-
check or further separately authenticated revocation policy for that
responder, complete revocation policy, generalize path construction or
validation, implement TLS or HTTP, authorize OAuth transport, or grant network
authority. Concrete HTTPS and live-provider readiness remain unavailable.

A following bounded no-check slice consumes that complete reviewed-root DNS
proof and derives the recursive proof's exact authenticated delegated responder
only through retained repository-owned evidence. It requires the structurally
unique `id-pkix-ocsp-nocheck` extension already admitted by certificate-wide
role policy and validates that its value is exact DER `NULL`. RFC 6960
recommends, but does not require, non-critical marking; a critical instance can
reach this semantic gate only after the prerequisite role policy recognized
its OID. Missing or malformed values fail with existing stable payload-free
categories, and success retains the complete prior proof under a distinct
non-copyable type. Callers cannot substitute a responder certificate,
extension, path, algorithm, key, signature, status, root, or time. This pure
public-certificate-data policy performs no credential access or editing,
signing, storage, clock acquisition, provider action, audit event, trace
emission, I/O, network access, or other external effect. It does not establish
separately authenticated further responder revocation, complete revocation
policy, generalized path construction or validation, TLS, HTTP, OAuth
transport, or network authority. Concrete HTTPS and live-provider readiness
remain unavailable.

A source-specific bounded completion slice then consumes that exact default-
source recursive proof after the server target, original responder,
intermediate, added intermediate-status responder, that responder's revocation
proof signer, and the recursive revocation proof's own signer have each been
covered by authenticated good-and-fresh OCSP status or exact RFC 6960 no-check
policy. The input type excludes generic sources whose earlier responder
branches remain unresolved, and the selected Mozilla-reviewed root remains the
trust anchor rather than a revocation target. The infallible non-copyable
result retains every prerequisite without accepting a substitute certificate,
path, root, algorithm, key, signature, status, or time. This pure public-
certificate-data composition performs no parsing, cryptography, credential
access or editing, signing, storage, clock acquisition, provider action, audit
event, trace emission, I/O, network access, or other external effect. It proves
complete revocation policy only for this one retained fixed branch; it does not
generalize path construction or validation, fetch, cache, acquire, or refresh
revocation data, or provide TLS, HTTP, OAuth transport, or network authority.
Concrete HTTPS and live-provider readiness remain unavailable.

A branch-specific bounded completion slice then consumes that exact default-
source alternative proof after the server target, original responder,
intermediate, added intermediate-status responder, and the added responder-
revocation proof's own responder have each been covered by authenticated good-
and-fresh OCSP status or exact RFC 6960 no-check policy. The input type excludes
the generic source whose separately authenticated responder branch remains
unresolved. The selected Mozilla-reviewed root remains the trust anchor rather
than a revocation target. The infallible non-copyable result retains every
prerequisite and accepts no substitute certificate, path, root, algorithm,
key, signature, status, or time. This pure public-certificate-data composition
performs no parsing, cryptography, credential access or editing, signing,
storage, clock acquisition, provider action, audit event, trace emission, I/O,
network access, or other external effect. It proves complete revocation policy
only for this one retained fixed branch; it does not generalize path
construction or validation, fetch, cache, acquire, or refresh revocation data,
or provide TLS, HTTP, OAuth transport, or network authority. Concrete HTTPS
and live-provider readiness remain unavailable.

A second source-specific bounded completion slice consumes the corresponding
double-nested proof after separately authenticated revocation covers the
original responder and the added intermediate-status responder, while each
revocation proof's own exact delegated responder has passed RFC 6960 no-check
policy. Authenticated status also covers the server target and retained
intermediate, and the selected Mozilla-reviewed root remains the trust anchor
rather than a revocation target. The input type fixes the source to the
complete original-responder-revocation plus responder-no-check composition;
the unresolved generic source cannot reach this constructor. The infallible
non-copyable result retains every prerequisite without accepting a substitute
certificate, path, root, algorithm, key, signature, status, or time. This pure
public-certificate-data composition performs no parsing, cryptography,
credential access or editing, signing, storage, clock acquisition, provider
action, audit event, trace emission, I/O, network access, or other external
effect. It proves complete revocation policy only for this one retained fixed
branch; separately authenticated recursive responder revocation, generalized
path construction or validation, revocation acquisition or refresh, TLS,
HTTP, OAuth transport, and network authority remain unavailable. Concrete
HTTPS and live-provider readiness remain unavailable.

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

A bounded certificate-binding entry point in the same package now consumes
certificate-wide critical-extension and role-policy evidence, derives the
Authority Information Access extension only from that evidence's exact decoded
certificate, decodes it under caller-owned ASN.1 limits, and applies the same
data-driven unique OCSP-locator policy. Success retains the complete role
policy together with URI, host, port, path, and query slices borrowed from that
exact certificate's encoded bytes; callers cannot attach a locator decoded
from another certificate, including a byte-equal re-decode. Stable failures
distinguish an absent extension collection, absent AIA, structural AIA failure,
and locator-policy failure without exposing certificate or URI payloads. This
composition remains discovery evidence only: it does not authorize a
destination, resolve DNS, fetch or cache OCSP data, authenticate a response,
read a clock, decide revocation, validate a path, establish trust, perform TLS
or HTTP, authorize OAuth transport, open a socket, or access a credential.
Concrete HTTPS remains unavailable.

An allocation-free `x509-ocsp-request` prerequisite now validates one exact
caller-supplied RFC 6960 `CertID` with the repository-owned structural decoder
and wraps those same canonical DER bytes into the minimal unsigned request
shape containing exactly one `Request`. The encoder omits the default version,
requestor name, per-request extensions, request extensions, and optional
signature; it preflights every nested DER length and caller-owned output
capacity before writing, and success retains the exact validated identifier
with the encoded request. Stable failures expose no identifier, hash, serial,
or output bytes. This framing slice does not choose or compute a hash, bind the
identifier to target or issuer certificates, add a nonce, authorize a
responder or destination, perform DNS, TLS, HTTP, caching, or I/O, emit audit
or trace events, access a credential, decide revocation, authorize OAuth
transport, or open a network connection. Concrete HTTPS remains unavailable.

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

An allocation-free `tls13-certificate-verify-message` prerequisite now owns
only the RFC 8446 section 4.4.3 `CertificateVerify` message body. Its exact
decoder borrows one uninterpreted two-byte signature-scheme identifier and the
following two-byte-length-prefixed signature, rejecting a truncated header,
short or long payload, and trailing bytes. Its encoder validates the complete
body and caller-owned capacity before changing output and leaves extra capacity
untouched. The structural RFC vector range of zero through 65,535 signature
bytes is preserved; an empty vector is syntax only and must fail later
cryptographic verification. Stable diagnostics retain no algorithm, signature,
or caller-length data.

This body codec does not frame the outer handshake header, authorize a
signature scheme, construct the signed input, hash, sign, verify a signature,
parse a key or certificate, validate a path or name, select trust roots, check
revocation, perform I/O, or provide TLS, HTTP, or OAuth transport. Those policy,
cryptographic, certificate, and composition boundaries remain separate.
Concrete HTTPS remains unavailable.

An allocation-free `tls13-compatibility-change-cipher-spec` prerequisite now
owns only the exact RFC 8446 middlebox-compatibility ChangeCipherSpec body.
Exact decoding accepts the single canonical octet `0x01` and rejects empty,
trailing, or differently valued bodies with payload-free categories. Encoding
writes that same canonical octet into caller-owned output only after complete
capacity validation and leaves extra capacity untouched. The returned typed
value is unforgeable outside the package and retains no caller bytes.

This body codec receives content already delimited from an unprotected
ChangeCipherSpec record. It does not authorize such a record in any handshake
state, decide whether to send or silently discard it, include or exclude bytes
from a transcript, accept protected ChangeCipherSpec content, sequence a
ClientHello, ServerHello, HelloRetryRequest, or Finished message, send an
alert, derive keys, protect records, validate certificates, establish trust,
enforce revocation, perform I/O, authorize HTTP or OAuth transport, or open a
network connection. A future state machine must restrict compatibility records
to the RFC 8446 window and keep them outside the handshake transcript.
Concrete HTTPS remains unavailable.

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

An allocation-free `tls13-extension-context-policy` prerequisite now composes
structurally decoded extension blocks with the TLS 1.3 client-side message
contexts in RFC 8446 section 4.2. Its caller-selected rule slice maps exact
extension types to non-empty context sets, rejects duplicate rules, and is
bounded to 64 entries. A reviewed standard rule table records all 22 extension
types and message placements in RFC 8446 Table 1 as data rather than control
flow. Outbound ClientHello validation rejects unrecognized or misplaced
extensions and requires `pre_shared_key`, when present, to be last. Inbound
ServerHello, EncryptedExtensions, HelloRetryRequest, Certificate,
CertificateRequest, and NewSessionTicket validation rejects unrecognized or
misplaced extensions. Server response extensions in ServerHello,
EncryptedExtensions, HelloRetryRequest, and the server Certificate must have
been offered by the validated ClientHello, except that HelloRetryRequest may
carry an unsolicited `cookie`; CertificateRequest remains a server request and
NewSessionTicket remains unsolicited by definition. Stable failures retain no
extension type, data, count, or message bytes.

This slice consumes only already decoded `tls13-extension-block` values. It
does not decode extension data, require mandatory extensions, validate the
semantics or relationships of selected values, generalize to a server role or
client-certificate response, emit alerts, sequence handshake messages,
calculate a transcript, invoke cryptography, validate certificates or names,
establish trust, enforce revocation, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. The standard table recognizes syntax
and placement only; it does not enable every listed extension in a future
client profile. Extension-content policy, ClientHello composition, handshake
state, certificate-path validation, revocation, TLS transport, and HTTP
composition remain blockers. Concrete HTTPS remains unavailable.

An allocation-free `tls13-signature-scheme-list` prerequisite now decodes the
exact RFC 8446 section 4.2.3 `SignatureSchemeList` carried as extension data by
`signature_algorithms` and `signature_algorithms_cert`. The decoder requires a
non-empty, even-length, two-byte-length-prefixed vector, requires the declared
length to consume the complete input, applies a caller-selected scheme-count
ceiling before exposing entries, retains the exact borrowed extension data,
and yields every two-byte scheme identifier in preference order. Unknown and
private-use identifiers remain data for a later allowlist instead of becoming
implicitly supported, and diagnostics omit all identifiers and input bytes.

This framing layer does not locate either extension in a handshake message,
decide where an extension is legal or required, reject duplicate preferences,
select or authorize a scheme, bind a selection to an offer, parse a public
key, invoke hashing, MGF1, RSA, ECDSA, or EdDSA, verify a signature, process a
certificate or transcript, establish trust, enforce revocation, perform I/O,
authorize HTTP or OAuth transport, or open a network connection. Separate
context policy, data-driven signature-scheme policy, repository-owned
verification, handshake state, certificate validation, TLS transport, and
HTTP composition remain blockers. Concrete HTTPS remains unavailable.

An allocation-free `tls13-certificate-request-message` prerequisite now
decodes one exact TLS 1.3 `CertificateRequest` body. A caller-owned limit
bounds the one-byte-length-prefixed `certificate_request_context`, and the
existing generic extension-block limits bound the complete extension payload
and retained entry count. The message-specific decoder rejects an empty
extension vector while leaving individual extension requirements to later
policy. Success borrows the exact body, request context, and ordered
duplicate-free opaque extensions without allocation. Empty and non-empty
contexts remain structural data, unknown extensions remain unclassified, and
stable failures retain no context or extension bytes.

This syntax layer does not authorize `CertificateRequest` in a main or
post-handshake state, enforce an empty main-handshake context, establish
connection-wide context uniqueness or unpredictability, require or decode the
`signature_algorithms` extension, authorize client authentication, access a
client key or credential, produce a client certificate or signature, update a
transcript, send an alert, derive keys, protect records, validate certificates,
perform I/O, authorize HTTP or OAuth transport, or open a network connection.
Those semantic, credential, audit, trace, and external-effect decisions remain
future reviewed composition work. Concrete HTTPS remains unavailable.

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

An allocation-free `tls13-alert-message` prerequisite now owns the exact
two-octet RFC 8446 section 6 alert message and its TLS 1.3 disposition. The
decoder retains the legacy level byte without trusting it, recognizes every
standard alert description, preserves unknown description values, and
classifies only `close_notify` and `user_canceled` as closure dispositions;
every defined error alert and every unknown description is an error regardless
of the received legacy level. Exact decoding rejects both truncated and padded
messages. The encoder admits only standard descriptions, requires `fatal` for
every section 6.2 error alert, validates complete output capacity before
writing, and leaves extra caller-owned capacity untouched. Stable failures
retain no caller bytes or alert values.

This message layer receives already authenticated alert content. It does not
sequence alerts, remember sent or received closure, suppress later data, wipe
connection secrets, log or audit an alert, choose an alert for another
failure, protect a record, perform I/O, close a transport, authorize HTTP or
OAuth traffic, or open a network connection. Alert state and connection
shutdown policy, complete handshake processing, repository-owned record
protection, certificate-path validation, revocation policy, TLS transport, and
HTTP composition remain blockers. Concrete HTTPS remains unavailable.

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

An allocation-free `tls13-signature-scheme-policy` prerequisite now binds the
three RFC 8446 RSASSA-PSS RSAE wire identifiers to their exact message hash,
MGF1 hash, digest-sized salt length, and required `rsaEncryption` public-key
form. A caller supplies an ordered non-empty, duplicate-free allowlist;
selection accepts only an exact known wire identifier already authorized by
that policy. EdDSA is deliberately absent while the repository implementation
retains secret-dependent scalar branches and key-derived temporaries. ECDSA
and RSASSA-PSS-key-OID schemes remain absent until their separate
repository-owned key and verification boundaries exist.

This registry is routing metadata only. Possessing a selected value does not
prove that its hash, MGF1, RSA-PSS encoded-message validation, RSA operation,
key parser, or certificate/handshake composition exists or is production-ready.
The package does not encode or parse a `signature_algorithms` extension,
advertise a scheme, validate a public key, verify a signature, process a
certificate or transcript, establish trust, enforce revocation, perform I/O,
authorize HTTP or OAuth transport, or open a network connection. Concrete
HTTPS remains unavailable.

An allocation-free `tls13-cipher-suite-offer` prerequisite now binds a
caller's preference-ordered ClientHello cipher-suite identifiers to that same
repository-owned policy. Offers contain one through 64 exact two-byte values,
reject duplicates, and must include at least one suite authorized by the
supplied policy. Explicitly configured unknown identifiers remain ordered data
for a separate reserved-value or GREASE policy. Failure-atomic encoding emits
the exact two-byte-length-prefixed ClientHello vector into caller-owned
storage. Server selection first must match an identifier in the original
offer and then must pass the retained policy; success returns the policy's
existing descriptor with the suite's algorithms, lengths, and per-key record
limit still bound together. Stable failures and diagnostics retain no offered
or selected identifiers.

This slice does not decide which suites or unknown identifiers to configure,
construct a ClientHello, generate entropy, parse a ServerHello, select a suite
for the peer, invoke an AEAD or hash, derive or store keys, protect records,
advance a handshake, validate certificates, establish trust, enforce
revocation, perform I/O, authorize HTTP or OAuth transport, or open a network
connection. ClientHello construction, ServerHello and handshake-state
composition, reviewed cryptographic execution, certificate validation, TLS
transport, and HTTP composition remain blockers. Concrete HTTPS remains
unavailable.

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
A bounded `rsa-pss-mgf1-sha512` prerequisite now owns the RFC 8017 MGF1 byte
expansion needed by a future SHA-512 RSA-PSS verification path. It accepts an
exact 64-byte public hash value, hashes `seed || I2OSP(counter, 4)` with the
repository-owned SHA-512 implementation from counter zero, copies a partial
final digest block when required, and checks both the RFC maximum and a
caller-selected nonzero work ceiling before changing caller-owned output.
Stable failures expose categories only. The fixed 68-byte hash input bounds
the current SHA-512 implementation's internal allocation and length domain.

This public-data primitive does not parse RSA-PSS parameters or keys, choose a
salt policy, validate an encoded message, perform RSA arithmetic, verify a
signature, process certificates, validate a path or hostname, select trust
roots, check revocation, run TLS, or open a socket. It grants no cryptographic
authority, and concrete HTTPS remains unavailable.

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

The repository-owned AES implementation now also exposes an AES-128-only
fixed-memory block-encryption prerequisite for the
`TLS_AES_128_GCM_SHA256` path. It accepts exact caller-owned 16-byte key,
plaintext, and output buffers, expands exactly eleven round keys into fixed
wipe-on-drop storage, and writes the ciphertext only after all ten rounds
complete. SubBytes computes the FIPS 197 multiplicative inverse and affine
transform with fixed-work GF(2^8) arithmetic rather than indexing a
secret-dependent S-box table. The round count, key-schedule branches, and
memory addresses depend only on AES-128's public fixed dimensions. The copied
state, expanded key, key-schedule scratch, row scratch, and column scratch are
wiped on return, while the borrowed inputs remain caller-owned. The boundary
does not allocate and does not move ciphertext through an ABI return slot.
It performs no credential access, edit, signing, provider action, audit or
trace emission, I/O, or external effect.

This is one block-cipher primitive, not AES-GCM or TLS record protection. It
does not support fixed-memory AES-192 or AES-256, authenticate ciphertext,
compose GHASH, construct or reserve record nonces, enforce a selected suite's
per-key record ceiling, own traffic keys, frame or protect records, process a
handshake, validate certificates, establish trust, enforce revocation, perform
I/O, authorize HTTP or OAuth transport, or open a network connection. The
allocation-returning and table-indexing legacy AES APIs remain compatibility
and educational surfaces rather than production TLS composition. The reviewed
AES-GCM composition below remains distinct from TLS record protection, and
concrete HTTPS remains unavailable.

A bounded fixed-memory AES-128-GCM composition is now shipped as the next
independently reviewable record-protection prerequisite. It accepts only an
exact 128-bit key, the RFC 5116 96-bit nonce profile, caller-borrowed associated
data and plaintext or ciphertext, an exact 128-bit tag, and caller-owned output
whose length must equal the input length. It builds
`J0 = nonce || 0x00000001`, starts payload counter mode at `inc32(J0)`, rejects
a request that could exhaust the 32-bit counter or GCM's encoded length domains
before changing caller output, and composes only the repository-owned
fixed-memory AES-128 block and GHASH boundaries. Encryption authenticates the
exact produced ciphertext. Decryption recomputes and compares the complete tag
with a compiler-checked fixed schedule before releasing any plaintext, so
authentication failure leaves caller output unchanged. The hash subkey,
counters, keystream block, GHASH result, encrypted `J0`, and recomputed tag
remain in fixed wipe-on-drop storage; no secret-derived byte crosses the ABI
return slot by value. Release-codegen checks reject allocator reachability,
early-exit tag comparison, missing scrubs, or plaintext writes before
authentication.

This pure in-process cryptographic boundary performs no credential access,
edit, signing operation, provider action, audit or trace emission, I/O, or
external effect. It does not select a TLS cipher suite, derive or own a
traffic key, reserve a record nonce, enforce a negotiated per-key record
ceiling, frame or protect a TLS record, reset record state after a key update,
process a handshake, validate certificates, establish trust, enforce
revocation, authorize HTTP or OAuth transport, or open a network connection. A
later TLS record composition must bind it to the selected
`TLS_AES_128_GCM_SHA256` route and the existing record-protection state.
Complete handshake state, the AES record route, stream I/O, certificate
validation, revocation, and HTTP composition remain blockers; concrete HTTPS
remains unavailable.

A bounded `tls13-finished` prerequisite now composes the repository-owned
fixed-memory HKDF and HMAC paths for RFC 8446 Finished verify-data. It derives
the fixed "finished" label with empty context from one already-derived
BaseKey, rejecting a SHA-256/SHA-384 mismatch against the exact
policy-authorized cipher suite before derivation. The resulting key occupies
caller-owned non-cloneable fixed wipe-on-drop storage and is exposed only by a
success-only borrowed view, so secret bytes do not cross the ABI return slot
by value. It has no byte accessor and retains only public suite metadata in
Debug output. An explicit client/server sender role remains bound to that key
storage and to successful verification evidence. Generation accepts an exact
Hash.length transcript hash and writes verify-data into exact caller-owned
storage only after all public checks and HMAC computation succeed.

Peer verification separately checks the public transcript-hash and
verify-data lengths, computes the expected value in wipe-on-drop storage, and
uses the repository-owned compiler-preserved full scan. Success returns
non-cloneable evidence retaining the selected suite, sender role, and exact
public transcript hash; mismatch is a stable redacted category. RFC 8448 SHA-256 key
evidence plus independently generated SHA-256 and SHA-384 verify-data vectors
cover the admitted algorithms.

The caller remains responsible for constructing the transcript hash, choosing
the correct client/server BaseKey, sequencing the Finished transition, and
consuming verification evidence in a complete handshake state machine. This
slice does not compose the early/handshake/master key schedule, parse or
sequence handshake messages, authenticate certificates, establish trust,
enforce revocation, invoke an AEAD, protect records, perform I/O, authorize
HTTP or OAuth transport, or open a network connection. Concrete HTTPS remains
unavailable.

An allocation-free `tls13-finished-transcript-binding` prerequisite now closes
the detached transcript-hash input at that Finished verifier. It accepts one
immutable repository-owned selected transcript, one already-derived Finished
key, and exact borrowed peer verify-data. Exact selected-suite equality is
checked before snapshotting or HMAC work; then the transcript is snapshot only
inside the composition and that snapshot alone is supplied to repository-owned
Finished verification. Success returns non-copyable evidence retaining the
exact transcript, suite-bound snapshot, and `VerifiedFinished` verdict
together. The retained immutable transcript borrow prevents that state from
advancing while the evidence is live. Stable failures retain no transcript,
digest, verify-data, suite, role, length, or offset bytes. SHA-256 and SHA-384
success paths and a same-hash different-suite substitution are covered.

This binding does not prove which ordered handshake encodings entered the
transcript, choose the correct client/server BaseKey or sender role, or connect
the earlier `CertificateVerify` transcript to the later Finished transcript.
It does not sequence the handshake, authenticate a certificate, establish
trust, enforce revocation, bind a record epoch, protect records, perform I/O,
authorize HTTP or OAuth transport, or open a network connection. It accesses
no credential and causes no edit, signing operation, provider or trace event,
storage, clock, network, or other external effect. Ordered transcript
continuity, record protection, and transport remain separate blockers;
concrete HTTPS and live-provider readiness remain unavailable.

The same binding also provides a consuming Finished-verification form for a
later handshake state machine. It takes ownership of the exact selected
transcript before checking the suite and verify-data and returns non-copyable
owned evidence retaining that transcript, its internally captured snapshot,
and the repository-owned Finished verdict together. Failure returns only the
same stable redacted error and does not return the transcript, so a failed
Finished check cannot leave caller-accessible authenticated-handshake state
available for retry or reuse. The borrowed form remains available when a
caller needs an immutable proof without transferring transcript ownership.

Ownership alone does not establish ordered transcript continuity. This form
still does not authenticate or append `CertificateVerify`, prove which exact
handshake encodings entered the transcript, sequence the server flight, bind a
certificate path, establish trust, enforce revocation, protect records,
perform I/O, authorize HTTP or OAuth transport, or open a network connection.
Those compositions remain blockers; concrete HTTPS and live-provider
readiness remain unavailable.

The repository-owned `tls13-key-schedule` boundary now composes the no-PSK
RFC 8446 section 7.1 base-secret transitions for a full ephemeral key exchange.
One caller-policy-selected cipher suite remains the sole source of SHA-256 or
SHA-384 policy. The composition derives the Early Secret from the required
Hash-length zero input, derives the Handshake Secret from a non-empty bounded
caller-owned shared secret, and derives the Master Secret from the required
Hash-length zero input. Both "derived" transitions use the canonical empty
transcript hash for the suite-selected algorithm. All intermediate derived
secrets occupy fixed wipe-on-drop storage, and the three resulting secrets are
copied into non-cloneable caller-owned wipe-on-drop storage only after the full
schedule succeeds. A success-only borrowed view binds the secrets to the
selected suite; stable failures retain no secret bytes or caller lengths.

This bounded slice deliberately excludes PSKs, 0-RTT, binders, traffic-secret
labels, exporter and resumption secrets, transcript sequencing, Finished,
record protection, handshake processing, certificate validation, trust-root
selection, revocation, I/O, HTTP, and OAuth. The caller remains responsible for
supplying a group-valid shared secret from a separately reviewed key-exchange
boundary, binding the peer through the later authenticated handshake, and
wiping that borrowed input. It performs no credential access,
edit, signing, provider action, audit or trace emission, or external effect.
Complete key-schedule, AEAD, handshake, certificate, transport, and HTTP
composition remain blockers; concrete HTTPS remains unavailable.

A bounded `tls13-traffic-secret-update` prerequisite now owns the exact RFC
8446 section 7.2 derivation from one application traffic secret to its
successor. It accepts the existing fixed-memory `HkdfPrk`, takes the selected
SHA-256 or SHA-384 output length only from that owner, and invokes
`HKDF-Expand-Label` with the internal label `"traffic upd"` and empty context.
The caller supplies the destination buffer; its length must equal Hash.length
before any output changes. Successful secret bytes remain caller-owned, while
the underlying fixed-memory HKDF boundary preserves its output wipe on an
unexpected PRF failure. The current secret is only borrowed and is never
modified.

This derivation does not decide whether a `KeyUpdate` message is authorized,
sequence post-handshake messages, retain current or next traffic secrets,
import the result into a new PRK owner, derive a traffic key or IV, reset record
sequence state, enforce old-key record protection, send a reciprocal update,
protect a record, emit an audit event, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. Atomic traffic-secret installation,
post-handshake state, repository-owned record protection, certificate-path
validation, revocation policy, TLS transport, and HTTP composition remain
blockers. Concrete HTTPS remains unavailable.

An allocation-free `tls13-transcript` prerequisite now owns the data-driven
SHA-256/SHA-384 transcript-hash state needed around TLS 1.3 cipher-suite
selection. Before selection it hashes each exact byte segment from a
caller-accepted handshake message encoding through both repository-owned hash
implementations. A
caller-selected nonzero cumulative wire-byte ceiling, itself no larger than
SHA-256's FIPS 180-4 message bound, limits work before either state changes.
Selection consumes one policy-authorized cipher suite and retains only the
suite-bound hash state; later snapshots carry that exact hash identifier and
length together with the public digest bytes. Snapshots are non-destructive,
and a failed cumulative-limit check leaves the transcript unchanged.

The selected transcript also supports RFC 8446's one-shot HelloRetryRequest
rewrite. It snapshots the first ClientHello transcript, builds the synthetic
`message_hash(254) || uint24(Hash.length) || Hash(ClientHello1)` input in fixed
storage, replaces the selected hash state only after the rewrite succeeds, and
rejects a repeated rewrite. The caller remains responsible for invoking that
operation at the correct state transition and then hashing the exact
caller-accepted HelloRetryRequest encoding. This slice does not parse or
authenticate handshake framing, prove segment boundaries, sequence handshake
messages, select a suite,
derive or retain TLS secrets, invoke an AEAD, process certificates, establish
trust, enforce revocation, perform I/O, authorize HTTP or OAuth transport, or
open a network connection. Complete handshake and key-schedule state,
repository-owned AEAD composition, certificate-path validation, revocation
policy, TLS transport, and HTTP composition remain blockers. Concrete HTTPS
remains unavailable.

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

An allocation-free `tls13-handshake-reassembly` prerequisite now retains one
TLS 1.3 handshake message across already authenticated record-content
fragments in caller-owned storage. Each push consumes only the input prefix
needed by that message and reports the exact consumed count, so coalesced
messages and a partial next message remain with the caller for ordered replay.
The reassembler validates the four-byte header as soon as it is complete,
applies the caller's existing handshake-type and body-length policy, rejects a
declared message that cannot fit the supplied storage, and borrows the exact
completed encoding through `tls13-handshake`. Incomplete pushes retain their
bounded prefix; completed-message and failure diagnostics retain no copied
wire bytes, declared lengths, or body contents.

Supplying only plaintext content from a record whose authenticity and inner
handshake content type were already accepted is solely a caller precondition.
The raw-byte API cannot distinguish authenticated from unauthenticated input.
This slice does not decode or authenticate records, classify an inner content
type, retain a coalesced tail, sequence handshake phases, update a transcript,
parse a handshake body, derive keys, invoke an AEAD, validate certificates,
establish trust, check revocation, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. Full handshake state, record
authentication and composition, certificate-path and identity validation,
revocation policy, TLS transport, and HTTP composition remain blockers.
Concrete HTTPS remains unavailable.

An allocation-free `tls13-encrypted-extensions-message` prerequisite now owns
the exact RFC 8446 section 4.3.1 `EncryptedExtensions` handshake body. It
delegates the complete two-byte-length-prefixed extension vector to the
repository-owned `tls13-extension-block` decoder under the caller's explicit
payload-byte and extension-count limits, permits the protocol-defined empty
vector, and retains both the exact borrowed body and its structurally decoded
extension block in one non-copyable value. Malformed lengths, truncated data,
duplicate types, and resource-limit failures remain the existing payload-free
extension-block categories.

This message-body layer embeds no extension registry and does not decide which
extensions are legal in `EncryptedExtensions`, bind responses to a
`ClientHello` offer, decode extension data, sequence handshake messages, emit
alerts, update a transcript, invoke cryptography, validate certificates,
establish trust, check revocation, protect records, perform I/O, authorize HTTP
or OAuth transport, or open a network connection. The reviewed extension-
context policy, handshake state, certificate validation, TLS-record, and HTTP
composition remain separate prerequisites, and concrete HTTPS remains
unavailable.

An allocation-free `tls13-client-hello-message` prerequisite now decodes one
exact RFC 9846 section 4.2.2 TLS 1.3 `ClientHello` handshake body. It enforces
the `0x0303` legacy version, retains the exact 32-byte random and bounded
zero-through-32-byte legacy session ID, requires a non-empty even-length
cipher-suite vector under a caller-selected suite-count ceiling, requires the
single null legacy compression method, and delegates the complete mandatory
extension vector to `tls13-extension-block` under caller byte and entry
limits. The extension-vector payload accepts RFC 9846's corrected seven-byte
minimum, including an otherwise valid one-version `supported_versions` offer
without an unrelated padding extension. Success retains the exact borrowed
body and all structural fields in a non-copyable value; unknown cipher-suite
and extension identifiers remain data for later policy, and diagnostics omit
random, session, suite, and extension bytes.

This message-body layer does not classify a first or retry ClientHello,
generate entropy, authorize a session ID or cipher suite, require or interpret
`supported_versions` or any other extension, apply extension-context policy,
bind later server selections to this offer, process a transcript, derive keys,
invoke cryptography, validate certificates, establish trust, enforce
revocation, perform I/O, authorize HTTP or OAuth transport, or open a network
connection. Separate offer and extension policy, retry/state sequencing,
transcript and key-schedule composition, certificate validation, TLS record
protection, and HTTP composition remain blockers. Concrete HTTPS remains
unavailable.

An allocation-free `tls13-no-psk-server-flight` policy now sequences the
already decoded server authentication flight after `ServerHello` for the
repository's no-PSK TLS 1.3 client path. It requires
`EncryptedExtensions`, accepts one optional `CertificateRequest`, then requires
`Certificate`, `CertificateVerify`, and `Finished` exactly once and in order.
Unexpected messages fail without advancing state. Completion retains whether
client authentication was requested and makes that requirement available to
later client-flight policy. A future composition must require that policy to
consume the retained value rather than allowing callers to ignore it.

This ordering layer does not accept PSK or early-data handshakes, sequence the
initial hellos or the client flight, decode or semantically validate a body,
update a transcript, authenticate `CertificateVerify` or `Finished`, validate
a certificate or path, check identity or revocation, access a credential or
signing key, produce a client certificate or signature, emit an alert, derive
keys, protect records, perform I/O, authorize HTTP or OAuth transport, or open
a network connection. Full handshake state and all of those compositions
remain blockers. Concrete HTTPS remains unavailable.

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

A bounded `tls13-record-protection-state` composition now consumes one
success-only traffic-key-material view and creates the matching one-direction
record nonce sequence directly from the same selected cipher suite. Static IV,
nonce length, AEAD routing, authentication-tag length, and caller-authorized
record ceiling cannot be supplied or reinterpreted independently. Fresh state
starts at sequence zero, has no reset operation, and consumes each sequence
before returning one non-cloneable borrowed reservation containing the exact
matching key, state-owned nonce, public routing metadata, and sequence number.
The reservation borrows the complete state, so another nonce cannot be reserved
while that key/nonce pair is in use; exhaustion fails closed without advancing
state. Secret key and static-IV bytes remain in the caller-owned wipe-on-drop
storage retained by the consumed material view.

This composition does not derive a traffic secret; establish its key-schedule
position; select or negotiate a suite; invoke or review an AEAD; frame, encrypt,
decrypt, or authenticate records; perform key updates; process a handshake;
validate certificates; establish trust; enforce revocation; perform I/O;
authorize HTTP or OAuth transport; or open a network connection. Repository-
owned AEAD review and composition, complete key-schedule and transcript state,
handshake processing, certificate-path validation, revocation policy, TLS
transport, and HTTP composition remain blockers. Concrete HTTPS remains
unavailable.

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

The repository-owned AES modes package now exposes fixed-memory
`ghash_multiply_fixed` and `ghash_fixed` prerequisites for the
`TLS_AES_128_GCM_SHA256` and `TLS_AES_256_GCM_SHA384` paths. The multiplication
boundary always performs 128 rounds, replaces secret-derived choices with
masks, writes its result into caller-owned fixed storage, and allocates no
memory. The multiplication product and working multiplier are wipe-on-drop;
the complete GHASH boundary also reuses and wipes one padded block and its
accumulator. AAD and ciphertext bit lengths are validated before caller output
changes. NIST SP 800-38D and block-edge equivalence evidence cover the admitted
surface.

This is only the universal-hash component of GCM. The repository's legacy AES
and GCM APIs remain educational compatibility surfaces and are not approved as
production TLS composition. This slice does not harden AES, derive or own an
AES key, construct or reserve a nonce, combine counter-mode encryption with
GHASH into a reviewed fixed-memory AEAD, compare a tag, enforce the selected
suite's per-key record ceiling, frame or protect records, process a handshake,
validate certificates, establish trust, enforce revocation, perform I/O,
authorize HTTP or OAuth transport, or open a network connection. A hardened
repository-owned AES primitive and reviewed combined AES-GCM boundary remain
prerequisites, and concrete HTTPS remains unavailable.

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

A bounded `tls13-chacha20-poly1305-record` composition now protects records
only for the reviewed `TLS_CHACHA20_POLY1305_SHA256` route. It consumes the
selected-suite-bound `tls13-record-protection-state`, rejects every other AEAD
before reserving a sequence number, and binds the reserved 256-bit key and
96-bit nonce directly to the repository-owned fixed-memory ChaCha20-Poly1305
boundary. Sealing encodes
one policy-authorized `TLSInnerPlaintext`, authenticates the exact
five-byte `TLSCiphertext` header as associated data, appends the exact 16-byte
tag, and wipes the exact caller-owned scratch prefix written after successful
inner encoding on every subsequent return. Route and inner-encoding failures
leave caller scratch unchanged. Opening validates one exact outer record before
consuming a sequence, authenticates its exact retained header before releasing
any cleartext, decodes only a policy-authorized inner content type, and returns
a non-cloneable borrowed view whose drop path wipes the successfully opened
caller-owned storage. Public framing, policy, and capacity failures occur
before sequence consumption; an authentication failure consumes its reserved
sequence and leaves caller output unchanged.

This narrow composition does not enable either AES-GCM suite, derive traffic
secrets, establish handshake epochs, process a transcript, authenticate
`CertificateVerify` or `Finished`, validate certificates, construct or validate
a path, check a hostname or revocation, perform key updates, read or write a
stream, open a socket, authorize an HTTP or OAuth request, or provide a TLS
connector. The caller must retain distinct read/write state and select the
correct traffic epoch. Complete handshake state, the remaining record routes,
stream I/O, certificate validation, revocation, and HTTP composition remain
blockers; concrete HTTPS remains unavailable.

A bounded `tls13-aes128-gcm-record` composition protects records only for the
reviewed `TLS_AES_128_GCM_SHA256` route. It consumes selected-suite-bound
`tls13-record-protection-state`, rejects AES-256-GCM and ChaCha20-Poly1305
before reserving a sequence number, and binds the reserved 128-bit key and
96-bit nonce directly to the repository-owned fixed-memory AES-128-GCM
boundary. Sealing encodes one policy-authorized `TLSInnerPlaintext`,
authenticates the exact five-byte `TLSCiphertext` header as associated data,
appends the exact 16-byte tag, and wipes the exact caller-owned scratch prefix
written after successful inner encoding on every subsequent return. Route and
inner-encoding failures leave caller scratch unchanged. Opening validates one
exact outer record before consuming a sequence, authenticates its exact
retained header before releasing any cleartext, decodes only a
policy-authorized inner content type, and returns a non-cloneable borrowed view
whose drop path wipes the successfully opened caller-owned storage. Public
framing, policy, and capacity failures occur before sequence consumption; an
authentication failure consumes its reserved sequence and leaves caller output
unchanged.

This narrow composition does not enable AES-256-GCM or ChaCha20-Poly1305,
derive traffic secrets, establish handshake epochs, process a transcript,
authenticate `CertificateVerify` or `Finished`, validate certificates,
construct or validate a path, check a hostname or revocation, perform key
updates, read or write a stream, open a socket, authorize an HTTP or OAuth
request, or provide a TLS connector. The caller must retain distinct
read/write state and select the correct traffic epoch. Complete handshake
state, the remaining record route, stream I/O, certificate validation,
revocation, and HTTP composition remain blockers; concrete HTTPS remains
unavailable.

A bounded `tls13-record-protection` dispatcher now routes one record operation
only from the exact cipher suite retained by `tls13-record-protection-state`.
It admits the repository-owned `TLS_AES_128_GCM_SHA256` and
`TLS_CHACHA20_POLY1305_SHA256` record compositions and accepts no detached
algorithm choice, key, nonce, tag length, or suite identifier. The recognized
but unimplemented `TLS_AES_256_GCM_SHA384` route fails before inner-plaintext
encoding, outer-record decoding, caller-output mutation, or record-sequence
consumption. Route-specific stable errors remain distinguishable without
retaining peer, content, key, nonce, tag, or output bytes. Successful opening
returns one non-cloneable algorithm-erased view whose methods expose only the
authenticated content type, content, and padding length; dropping the view
still wipes the exact successfully opened caller-owned prefix through the
selected reviewed route.

This dispatcher adds no AES-256-GCM primitive, detached or caller-selected
algorithm fallback, traffic-secret derivation, epoch transition, handshake
processing, certificate or path validation, hostname or revocation policy,
stream I/O, TLS connector, HTTP authorization, OAuth transport, credential
access or editing, signing, storage, provider action, audit event, trace
emission, network access, or other external effect. Complete handshake state,
certificate validation, revocation, repository-owned TLS transport, HTTP
composition, and audited networking remain blockers; concrete HTTPS and
live-provider readiness remain unavailable.

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

An allocation-free `tls13-key-share-entry` prerequisite owns only the RFC 8446
`KeyShareEntry` structure shared by ClientHello and ServerHello key-share
extension data. A caller supplies one exact two-byte named-group identifier,
one non-empty borrowed key-exchange byte string, and an explicit reviewed
maximum key-exchange length from 1 through 65,535 bytes. Construction and
decoding reject empty or over-policy key material. Unknown and reserved group
identifiers remain exact data for a later data-driven group policy. Encoding
writes the two-byte group, two-byte key-exchange length, and exact caller bytes
only after validating complete output capacity. Prefix decoding returns one
complete borrowed entry plus the unconsumed input, while rejecting truncated
headers or bodies and over-policy declared lengths before borrowing payload
bytes. Stable failures and diagnostics retain no group identifiers or key
bytes.

This structural slice does not construct the ClientHello key-share vector,
interpret HelloRetryRequest, authorize a named group, enforce a group-specific
public-key length or encoding, generate ephemeral secrets, invoke X25519 or
another key-agreement primitive, validate a peer public key, derive or retain a
shared secret, advance a handshake, establish trust, enforce revocation,
perform I/O, authorize HTTP or OAuth transport, or open a network connection.
Named-group policy, reviewed repository-owned key agreement, handshake-state
composition, TLS transport, and HTTP composition remain blockers. Concrete
HTTPS remains unavailable.

An allocation-free `tls13-certificate-message` prerequisite now decodes one
already-delimited RFC 8446 Certificate handshake body under caller-owned total,
request-context, certificate-list, entry-count, certificate, and per-entry
extension ceilings. It preserves borrowed request-context bytes, complete
certificate bytes, and opaque extension blocks, and exposes the validated
entries without allocation. Structurally valid empty client certificate lists
and non-empty post-handshake request contexts remain available for later exact
handshake-state policy rather than being assigned one role by the framing
layer. Stable failures retain no handshake bytes or length values.

This slice does not interpret entry extensions, parse X.509, decide handshake
role or state, authenticate a certificate, construct or validate a path,
establish trust, check names or revocation, send alerts, derive keys, protect
records, perform I/O, provide TLS or HTTP, authorize OAuth transport, or open a
network connection. Concrete HTTPS remains unavailable.

An allocation-free `tls13-server-hello-message` prerequisite now decodes one
already-delimited RFC 8446 `ServerHello` body. Caller-owned limits bound the
complete body, extension-vector payload, and retained extension count. The
decoder enforces the fixed TLS 1.3 legacy version `0x0303`, the zero legacy
compression method, the zero-to-32-byte session-id echo, exact body
consumption, and the structure's minimum extension-vector payload. It retains
the exact random and unclassified cipher-suite identifier, classifies the
random as an ordinary `ServerHello` or the exact RFC 8446
`HelloRetryRequest` sentinel, then composes the repository-owned generic
extension-block decoder so malformed lengths, trailing bytes, duplicate
extension types, and resource exhaustion fail closed. One-bit and every other
non-exact random remain ordinary `ServerHello` values. Stable failures contain
only a category and body-local offset, and diagnostics omit random,
session-id, and extension bytes.

This exact public-data classification does not itself apply the transcript's
`message_hash` rewrite, sequence an ordinary or retry handshake, interpret or
authorize extension contents, require or validate
`supported_versions`, bind the session-id echo or cipher-suite selection to an
original `ClientHello`, enforce the downgrade sentinel, send alerts, advance a
handshake, derive keys, protect records, parse or validate certificates,
establish trust, enforce revocation, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. ClientHello construction,
context-specific extension and selection policy, handshake state,
repository-owned signature verification, certificate name/path validation,
trust-root and revocation integration, TLS transport, and HTTP composition
remain blockers. Concrete HTTPS remains unavailable.

An allocation-free `tls13-server-hello-selection` prerequisite binds one
structurally decoded ordinary `ServerHello` or `HelloRetryRequest` to the
exact decoded `ClientHello` and one supplied policy-bearing cipher-suite
offer. Before accepting any server selection, the binder requires the offer's
entry count, wire identifiers, and preference order to equal the cipher-suite
vector retained by that exact `ClientHello`; detached, truncated, reordered,
or otherwise substituted offer wire evidence fails closed. It then requires
the server's legacy session-id echo to equal the exact client session ID and
asks the retained offer and its data-driven cipher-suite policy to validate
the server's selected wire identifier. Success moves both non-copyable decoded
messages and the offer into one evidence value, retains the public ServerHello
kind, and returns the repository-owned selected-suite descriptor that binds
the record algorithm, HKDF hash, key lengths, and per-key record ceiling.
Stable failures and diagnostics retain no session ID, offer, selection,
random, handshake, or policy payloads.

This selection binding does not parse either handshake body, interpret or
authorize extensions, require or validate `supported_versions`, compare a
post-retry `ServerHello` with the earlier `HelloRetryRequest`, apply the
transcript `message_hash` rewrite, sequence a handshake, send alerts, derive
keys, protect records, parse or validate certificates, establish trust,
enforce revocation, perform I/O, authorize HTTP or OAuth transport, or open a
network connection. Extension policy, retry-state and transcript composition,
key schedule, certificate validation, TLS transport, and HTTP composition
remain blockers. Concrete HTTPS and live-provider readiness remain
unavailable.

An allocation-free `tls13-server-hello-version-selection` prerequisite now
extends that retained selection evidence through extension-context and
`supported_versions` policy. The binder first validates the exact decoded
ClientHello extension block and the exact decoded ordinary ServerHello or
HelloRetryRequest extension block against one caller-supplied, data-driven
context policy. It then requires exactly one `supported_versions` extension in
both retained messages. Before trusting a supplied `ClientVersionOffer`, it
requires the offer's complete encoded ClientHello extension data, including
the vector length, identifier order, and every identifier, to equal the
extension data retained by that exact decoded ClientHello. Finally, it asks
the retained offer to validate the server's exact extension data as a version
the client offered and as exactly TLS 1.3. Success moves the non-copyable prior
selection binding into a new evidence value and retains the policy, exact
version offer, and selected TLS 1.3 descriptor. Stable failures and diagnostics
retain no extension data, version identifiers, message bytes, or policy
payloads.

This version binding does not compare a HelloRetryRequest selection with a
later ServerHello, require the retry to change the ClientHello, compare retry
cipher suites, apply the transcript `message_hash` rewrite, interpret
`key_share`, sequence a handshake, send alerts, derive keys, protect records,
parse or validate certificates, establish trust, enforce revocation, perform
I/O, authorize HTTP or OAuth transport, or open a network connection. Retry
state, transcript composition, key schedule, certificate validation, TLS
transport, and HTTP composition remain blockers. Concrete HTTPS and
live-provider readiness remain unavailable.

An allocation-free `tls13-server-hello-retry-selection` prerequisite now
binds one retained HelloRetryRequest version-selection proof to one retained
ordinary ServerHello version-selection proof. The binder requires the first
message to be classified as HelloRetryRequest and the second as ServerHello,
then compares the exact retained ClientHello random, legacy session ID,
cipher-suite offer count and order, and supported-version offer count and
order across the two proofs. It also requires the complete extension-context
policy, policy-authorized cipher-suite descriptor, and selected TLS version to
remain equal. Success moves both non-copyable proofs into one evidence value,
so later handshake composition cannot replace either message, either
ClientHello, either data-driven offer, or the extension rule table after
continuity has been checked. Stable failures and diagnostics retain no random,
session ID, offer, selection, extension, handshake, or policy payloads.

This retry continuity binding does not compare the complete ClientHello pair
or validate extension-specific permitted changes; require the retry to change
the ClientHello; reject a second retry in connection state; interpret
`cookie`, `key_share`, or `pre_shared_key`; apply the transcript
`message_hash` rewrite; sequence a handshake; send alerts; derive keys;
protect records; parse or validate certificates; establish trust; enforce
revocation; perform I/O; authorize HTTP or OAuth transport; or open a network
connection. ClientHello retry-transition policy, connection state, transcript
composition, key schedule, certificate validation, TLS transport, and HTTP
composition remain blockers. Concrete HTTPS and live-provider readiness
remain unavailable.

An allocation-free `tls13-cookie` prerequisite now owns the RFC 9846 section
4.3.2 `cookie` extension-data structure needed by a later ClientHello retry
transition policy. Construction and decoding require the exact non-empty
two-byte-length-prefixed value, enforce one caller-selected maximum before
exposing borrowed cookie bytes, and reject trailing or truncated data.
Encoding validates the complete wire length and caller-owned output capacity
before changing output. A separate echo binder consumes one decoded
HelloRetryRequest cookie and one decoded second-ClientHello cookie, requires
their complete encoded extension data to be byte-for-byte equal, and retains
both non-copyable values as evidence. Stable failures and diagnostics retain
no cookie bytes, encoded extension data, lengths, mismatch positions, or
output contents.

This cookie boundary treats the cookie as opaque peer-supplied data. It does
not locate or authorize extension type 44 in either message, generate a
cookie, authenticate or decrypt server state carried by a cookie, compare any
other ClientHello field, require a retry to make progress, reject a second
retry, apply the transcript `message_hash` rewrite, sequence a handshake, send
alerts, derive keys, protect records, parse or validate certificates,
establish trust, enforce revocation, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. Complete ClientHello retry-transition
policy, connection state, transcript composition, key schedule, certificate
validation, TLS transport, and HTTP composition remain blockers. Concrete
HTTPS and live-provider readiness remain unavailable.

An allocation-free `tls13-alpn` prerequisite now owns the RFC 7301
`application_layer_protocol_negotiation` extension data needed to bind a
future repository-owned TLS client to one application protocol. A client
offer contains one through 64 caller-ordered, opaque, non-empty protocol names,
each from one through 255 bytes, inside the exact non-empty 16-bit-length
vector. Construction rejects duplicate names and validates the complete
encoded size before changing caller-owned output. Decoding checks the caller's
explicit one-through-64 protocol-count ceiling before borrowing each exact
name. A server selection must contain exactly one protocol name and must equal
one member of the original validated client offer; the returned borrowed value
therefore cannot authorize an unsolicited protocol. Stable failures retain no
protocol bytes, names, counts, or encoded lengths.

This slice treats protocol identifiers as opaque policy data. It does not
register or enable HTTP/1.1, HTTP/2, or any other application protocol; locate
or authorize the ALPN extension in a handshake message; enforce that a server
sent ALPN; sequence a handshake; validate certificates; establish trust; check
revocation; protect records; perform I/O; authorize HTTP or OAuth transport; or
open a network connection. Extension-context, handshake-state, certificate,
TLS-record, and HTTP composition remain prerequisites, and concrete HTTPS
remains unavailable.

An allocation-free `tls13-supported-groups` prerequisite now decodes the exact
RFC 8446 section 4.2.7 `NamedGroupList` extension data shared by client and
server `supported_groups`. The decoder requires a non-empty, even-length,
two-byte-length-prefixed vector, requires the declared length to consume the
complete input, applies a caller-selected group-count ceiling before exposing
entries, retains the exact borrowed extension data, and yields every two-byte
group identifier in preference order. Unknown and private-use identifiers
remain data for a later allowlist instead of becoming implicitly supported,
and diagnostics omit all identifiers and input bytes.

This framing layer does not locate the extension in a handshake message,
decide where it is legal or required, reject duplicate preferences, select or
authorize a group, bind a key share to an offer, parse or generate key-share
bytes, invoke ECDHE or finite-field arithmetic, process a handshake or
transcript, validate certificates, establish trust, enforce revocation,
perform I/O, authorize HTTP or OAuth transport, or open a network connection.
Separate extension-context policy, data-driven named-group policy, key-share
composition, handshake state, certificate validation, TLS transport, and HTTP
composition remain blockers. Concrete HTTPS remains unavailable.

An allocation-free `tls13-server-name` prerequisite now owns the exact RFC
6066 ClientHello `server_name` extension data used by a future repository-owned
TLS 1.3 client. It validates one borrowed ASCII DNS hostname under explicit
253-byte name and 63-byte label ceilings, rejects trailing root dots,
wildcards, malformed labels, and IPv4 or IPv6 literals (including legacy or
ambiguous numeric IPv4 resolver spellings), and encodes the exact
two-byte-length-prefixed `ServerNameList` containing one `host_name`. Exact
decoding admits only that current single-name-type profile; unknown name types,
length mismatches, extra entries, and malformed names fail closed. The
corresponding TLS 1.3 EncryptedExtensions acknowledgement is accepted only with
empty extension data. Encoding checks complete caller-owned output capacity
before mutation, and stable failures retain no hostname, input bytes, or
lengths.

This slice receives already-delimited extension data. It does not locate or
authorize the extension in a handshake message, require the server response,
bind SNI to certificate identity, construct a ClientHello, process a handshake,
parse or validate certificates, establish trust, check revocation, perform DNS
or network I/O, authorize HTTP or OAuth transport, or replace `tls-platform`.
Extension-context policy, ClientHello composition, handshake state,
certificate-path validation, revocation policy, TLS transport, and HTTP
composition remain blockers. Concrete HTTPS remains unavailable.

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

An allocation-free `http1-request-host-authority` prerequisite now validates
one caller-borrowed ASCII DNS name and optional non-zero port under explicit
caller limits, then encodes the exact HTTP/1 `Host` field value into
caller-owned storage. DNS labels use the LDH profile, cannot be empty or exceed
63 bytes, and cannot begin or end with a hyphen. A terminal root dot, URI
delimiters, whitespace, controls, IPv4/IPv6 literals including legacy numeric
spellings, Unicode, and other non-DNS authority forms fail closed. Encoding
lowercases the DNS name and appends the
caller-supplied port in shortest decimal form only after proving the complete
output fits, so an undersized output remains unchanged. Stable failures retain
no hostname, port, length, or output bytes, and Debug redacts the borrowed
hostname.

This syntax boundary does not parse a URL, decide whether a configured port is
the scheme default, create the surrounding `Host` field line, compare the
authority with a request target, TLS SNI, certificate identity, or provider
configuration, authorize a destination, perform DNS or I/O, release a
credential, emit provider or trace audit events, or provide HTTP, OAuth, or
live-provider transport. Those semantic bindings and external effects remain
separate. Concrete HTTPS remains unavailable.

A bounded `https-destination-name-binding` prerequisite now joins the three
repository-owned DNS identity boundaries that a future HTTPS client must keep
consistent. It consumes one exact decoded certificate, one already validated
TLS `server_name`, and one already validated HTTP/1 `Host` authority. It first
requires the SNI and Host DNS names to be equal under ASCII case-insensitive
comparison, then derives the certificate reference from that same SNI value
and reruns complete-certificate Subject Alternative Name matching. On success,
one non-copyable result retains the exact certificate object, SNI value, Host
authority including any explicit port, and certificate-identity evidence. The
port is deliberately retained as transport metadata but is not part of DNS
identity comparison. Stable failures retain no hostname, certificate bytes,
port, or observed input length, and Debug output redacts all three identities.

This proof does not parse a URL, bind a request target or provider
configuration, decide scheme-default ports, validate critical extensions,
certificate role, purpose, algorithm, validity, signatures, paths, trust
anchors, constraints, or revocation, construct a ClientHello or HTTP request,
perform TLS or HTTP, access a credential, emit an audit event or trace, perform
DNS or network I/O, authorize an external effect, or grant HTTPS, OAuth, or
live-provider readiness. Those policy, protocol, transport, and effect
compositions remain separate. Concrete HTTPS remains unavailable.

The same `https-destination-name-binding` boundary now also consumes one
strictly validated HTTP/1 request-header field together with that complete
destination proof. It accepts only the `Host` field name under HTTP's
case-insensitive field-name semantics and requires the field value to equal
the retained Host authority's canonical encoding: the DNS name is lowercase,
and any explicit port uses shortest decimal form. Success moves both
non-copyable inputs into one result, so a field for another destination cannot
be substituted after binding. Stable failures and Debug output retain no
certificate, hostname, authority, field-name, field-value, port, or observed
length payloads.

The same boundary can now move that canonical Host proof into one bounded
request-field-set proof together with the exact caller-borrowed slice of
additional validated fields. An explicit caller-selected total-field ceiling,
including Host, is hard-capped at 64 before the slice is scanned. Every
additional field name is compared under HTTP's ASCII case-insensitive
semantics, so any second Host spelling fails closed. Success retains the exact
Host proof, additional-field slice, caller order, and count policy in one
non-copyable result. Stable failures and Debug output retain no field name,
field value, hostname, certificate, observed count, or limit violation
payloads.

The same boundary can now consume that exact request-field-set proof together
with one exact fixed-body proof already bound to its canonical Content-Length
field. The resulting non-copyable proof defines logical field order as Host,
Content-Length, then the retained additional fields in caller order. The
length field counts inside the same retained total-field ceiling, hard-capped
at 64. Every additional name is checked under HTTP's ASCII case-insensitive
semantics, so any second Content-Length or any Transfer-Encoding field fails
closed. Success retains both complete input proofs; stable failures and Debug
output retain no field, body, hostname, certificate, observed count, or limit
violation payloads.

The strict request-head boundary first requires data-driven provider policy to
classify one exact validated request-line method and query-free origin-form
target as public request metadata. This produces a non-copyable proof rather
than accepting an unclassified target. The complete head composition consumes
that proof with the exact fixed-body destination proof and rejects every
additional field; potentially credential-bearing targets and fields instead
require a later provider/trace-audited release proof. A caller-selected
complete-head byte ceiling is bounded by the fixed package maximum before any
output is written. Success retains both complete inputs and their exact encoded
head length. Its allocation-free encoder delegates to the repository-owned
request-line and field encoders and writes, in order, the public request line,
canonical Host field, canonical Content-Length field, and terminating empty
line. Insufficient output fails before mutation. The body remains retained but
is not written. Stable failures and Debug output retain no method, target,
field, body, hostname, certificate, observed length, limit, or output payloads.

This stricter composition does not authorize transport, release an additional
field, write the body, perform credential access or release, emit provider or
trace audit events, perform TLS or HTTP I/O, open a socket, or grant network,
OAuth, or live-provider authority. A later audited transport must consume this
exact proof without substituting any retained request or destination input.
Concrete HTTPS remains unavailable.

An additive audited request-head release now admits a non-empty ordered slice
of public additional fields only through exact data-driven provider policy.
The policy retains one validated provider, public method and query-free path,
validated DNS Host authority, and an ordered duplicate-free allowlist of exact
public HTTP field names and values. This low-level boundary admits only the
closed non-credential metadata names `Accept` and `Content-Type`;
credential-bearing, custom, Host, Content-Length, and Transfer-Encoding fields
remain unavailable. The release consumes the exact public request-line and
fixed-body destination proofs, requires the retained method, path, Host
authority, additional-field count, order, case-insensitive names, and exact
values to match that provider policy, and preflights the complete bounded head
and caller-owned output before any effect or mutation. A privacy-safe
event containing only provider, trace, stable action, and closed outcome must
be durably published before the repository-owned request-line and field
encoders copy any bytes. Audit failure leaves output unchanged and releases no
proof; successful encoding returns a non-copyable redacted wrapper retaining
the exact audited provider, trace, and emitted head. Each wrapper requires one
fresh audited encoding call, so one event cannot authorize another output
release or be relabelled under another provider or trace.

This audited release copies only exact public metadata classified by provider
policy. It does not admit or access credential-bearing fields, obtain
credentials from storage, interpret or edit a request body, sign, write the
retained body, wipe caller-owned storage, authorize transport, perform TLS or
HTTP I/O, open a socket, or grant network, OAuth, or live-provider authority.
A later custody-bound credential-field, body, and transport composition must
retain this exact provider, trace, request, destination, and release binding
without substituting any input, and must apply its own audit gates before every
credential access and external effect. Concrete HTTPS remains unavailable.

A bounded `oauth-client-secret-http1-binding` prerequisite now binds
one custody-built `ClientSecretAuthenticatedRequest` to strict HTTP/1 evidence
constructed only inside the audited boundary. Before inspecting either
credential-bearing body or Authorization bytes, the binder durably publishes
an attempted event containing only the request's validated provider, trace,
stable action, and closed outcome. It then uses the repository-owned fixed-body
and field primitives to retain the request's original form-body allocation,
encode and bind its canonical Content-Length in bounded caller storage, and,
when custody produced Basic authentication, validate an `Authorization` field
that retains the original Authorization allocation. Body authentication
produces no Authorization field. The binder accepts no caller-supplied
credential bytes or prebuilt evidence, but it does not revoke the owner's
independent access to the already custody-released request; its proof attests
only to reads and construction performed by this call. A closed rejected or
succeeded event must be durable before the call returns an error or releases
the non-copyable proof. Validation or audit failure leaves bounded caller
storage unchanged and releases no proof. After terminal success, deterministic
output binding rereads no credential-bearing input. Success retains the exact
request reference, fixed-body/Content-Length proof, Authorization-field
presence, provider, and trace so a later composition cannot substitute
credential material or relabel the release.

Stable failures and Debug output retain no endpoint, body, Authorization,
field-name, field-value, credential, observed-length, or identity payloads.
This identity boundary does not obtain credentials from storage, edit or sign
them, encode or copy a head or body, authorize a method or endpoint, bind an
HTTPS destination, perform TLS or HTTP I/O, open a socket, or authorize a
transport effect. The audited complete-request release, destination binding,
repository-owned TLS/HTTP composition, response policy, and every actual
external effect remain separate blockers. Concrete HTTPS and live-provider
readiness remain unavailable.

A bounded `oauth-client-secret-http1-release` prerequisite composes that
custody-bound material with one exact validated HTTP/1 request line and one
existing `https-destination-name-binding` certificate/SNI/Host proof. Its
data-driven policy retains the expected provider, method, exact form media
type, and complete-request byte ceiling. The request endpoint must equal the
exact lowercase `https://` origin retained by the canonical Host field plus
the exact origin-form request target; detached endpoint, destination, method,
provider, trace, media-type, Content-Length, body, and Authorization claims
are unavailable. The only admitted wire order is request line, Host,
Content-Length, Content-Type, optional Authorization for Basic authentication,
the terminating empty line, and the exact custody-built form body. Post
authentication therefore has no Authorization field, while Basic
authentication has exactly the one field already retained by the binding.

Each release publishes a privacy-safe attempted event for the request's own
provider and trace before inspecting credential-bearing body or Authorization
bytes. Policy, endpoint, framing, field-placement, length, allocation, and
encoding rejection publishes a closed failed outcome. The complete request is
prepared in repository-owned wipe-on-drop storage before a terminal succeeded
event; after that event the implementation only moves the already prepared
storage into a non-copyable returned proof, so it performs no later credential
read or copy. Audit failure returns no request and wipe-on-drop storage covers
every prepared failure path. Debug and errors reveal no provider, trace,
endpoint, request, header, body, or credential bytes.

The returned proof exposes complete wire bytes only through a further
provider/trace audit-gated borrow. Audit failure withholds the slice, so every
credential-byte access introduced by this package remains attributable even
though the owner still retains independent access to the earlier custody-built
request.

This release is still pure in-memory evidence. The retained certificate proof
does not add certificate role, purpose, algorithm, validity, signature, path,
constraint, trust-root, or revocation validation. The package performs no DNS,
socket, TLS, HTTP I/O, response handling, credential acquisition or edit, signing,
or external transport effect and grants no HTTPS, OAuth, or live-provider
readiness. Repository-owned TLS/HTTP composition, audited network effects, and
response policy remain separate blockers.

A bounded `https-reviewed-server-path-binding` prerequisite now consumes that
complete HTTPS destination-name proof together with the strongest accumulated
reviewed RSA server-path policy. It derives the path target only through the
retained complete server policy and requires both inputs to retain the same
exact decoded certificate object. A byte-equal independently decoded
certificate therefore fails closed. Success returns one non-copyable boundary
that retains the destination's certificate, TLS SNI, HTTP Host authority and
SAN evidence alongside the reviewed root selection, data-driven RSA signature
routing, certificate-wide target and issuer policy, path-length state, and the
currently supported target and subordinate DNS/IP NameConstraints evidence.
Stable failures and Debug output retain no certificate, hostname, authority,
path, key, signature, algorithm, constraint, or trust-root payloads.

This composition performs no certificate parsing or new validation, does not
complete RFC 5280 Name or path validation, validate an anchor self-signature,
check revocation, build or run TLS or HTTP, access credentials, emit provider
or trace audit events, perform I/O, authorize an external effect, or grant
HTTPS, OAuth, network, or live-provider readiness. Complete path/Name policy,
revocation, repository-owned TLS/HTTP composition, and audited transport remain
separate blockers. Concrete HTTPS remains unavailable.

A bounded `x509-rsa-reviewed-anchor-self-signature-binding` prerequisite now
joins the existing data-driven repository-owned RSA signature router to the
reviewed Mozilla root snapshot without adding another algorithm switch. It
consumes one routed RSA PKCS #1 v1.5 SHA-256, SHA-384, or SHA-512 policy proof
and one reviewed-root selection. The routed proof's child certificate, its
issuer RSA algorithm-and-key policy certificate, and the selected reviewed
root must all be the same exact decoded object. Byte-equal independently
decoded certificates fail closed. Success returns non-copyable evidence that
retains the selected fixed verifier, complete certificate algorithm policy,
complete RSA algorithm-and-key policy, exact repository-owned signature proof,
and exact reviewed-snapshot membership. Stable failures and Debug output
retain no certificate, key, signature, algorithm, route, or root payloads.

This is a public-data evidence boundary only. It does not select certificate
algorithm profiles or signature routes, parse certificates or keys, decide
trust-anchor semantics, establish CA role, validate time or constraints,
construct or validate a certification path, bind the proof into the current
reviewed server-path result, check revocation, build or run TLS or HTTP, access
credentials, emit provider or trace audit events, perform I/O, authorize an
external effect, or grant HTTPS, OAuth, network, or live-provider readiness.
Complete path/Name policy, composition of this self-signature evidence into
the reviewed server path, revocation, repository-owned TLS/HTTP composition,
and audited transport remain separate blockers. Concrete HTTPS remains
unavailable.

A bounded `x509-rsa-reviewed-server-path-anchor-self-signature-binding`
prerequisite now consumes the strongest accumulated Name-policy-backed
reviewed RSA server-path result together with complete reviewed-anchor RSA
self-signature evidence. It derives the path terminal only through the
retained constructed-path evidence and requires the terminal's reviewed-root
selection and the self-signature proof's reviewed-root selection to retain the
same exact decoded certificate object. A byte-equal independent decode fails
closed. Success returns one non-copyable result retaining the complete ordered
path-signature, role, path-length, target and subordinate NameConstraints
policy together with the data-driven SHA-256, SHA-384, or SHA-512
repository-owned anchor self-signature proof. Stable failures and Debug output
retain no certificate, path, key, signature, algorithm, constraint, or root
payloads.

This is an evidence-integrity composition only. It performs no parsing,
signature verification, credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network access,
or other external effect. It does not add trust-anchor semantics, complete RFC
5280 Name or path validation, revocation, TLS, HTTP, OAuth transport, or
network authority. Composition with the HTTPS destination proof, complete
revocation policy, repository-owned TLS/HTTP, and audited transport remain
separate blockers. Concrete HTTPS and live-provider readiness remain
unavailable.

The existing bounded `https-reviewed-server-path-binding` prerequisite now
also joins the complete HTTPS destination-name proof directly to that complete
reviewed server-path-and-anchor-self-signature evidence. It derives the path
target only through the retained accumulated path, requires the destination
certificate and path target to be the same exact decoded certificate object,
and retains both complete non-copyable inputs after success. A byte-equal
independently decoded leaf fails closed. The original path-only binder remains
available as the deliberately weaker evidence boundary; this stronger entry
point cannot be constructed from a detached leaf, path, anchor selection, or
self-signature claim. Stable failures and Debug output retain no certificate,
hostname, authority, path, key, signature, algorithm, constraint, or root
payloads.

This stronger composition performs no parsing, signature verification,
credential access or editing, signing, storage, clock acquisition, provider
action, audit event, trace emission, I/O, network access, or other external
effect. It does not add trust-anchor semantics, complete RFC 5280 Name or path
validation, revocation, TLS, HTTP, OAuth transport, or network authority.
Complete revocation policy, repository-owned TLS/HTTP, and audited transport
remain separate blockers. Concrete HTTPS and live-provider readiness remain
unavailable.

A bounded `oauth-client-secret-reviewed-server-binding` prerequisite now joins
one complete audited client-secret HTTP/1 release to that stronger signed
reviewed HTTPS server-path boundary. It derives the request destination only
through the retained canonical Host binding and the reviewed destination only
through the retained path-and-anchor-self-signature proof. The two destinations
must retain the same exact decoded certificate object, ASCII-case-insensitively
equal SNI DNS names, and ASCII-case-insensitively equal Host DNS names with the
same optional port. This closes independent destination substitution even when
one certificate admits more than one SAN name. A byte-equal independently
decoded certificate, different admitted SAN name, or different explicit port
fails closed. Success moves both complete non-copyable inputs into one result;
stable failures and Debug output retain no provider, trace, request, credential,
certificate, hostname, authority, path, key, signature, algorithm, constraint,
or trust-root payloads.

This identity-only composition does not borrow or copy the released request
bytes, so it introduces no credential access and emits no provider or trace
audit event. The release's existing audit gate remains mandatory for every
later request-byte borrow. The composition performs no parsing, new
validation, cryptography, credential editing, signing, storage, clock
acquisition, I/O, network access, or other external effect. It does not add
trust-anchor semantics, complete RFC 5280 Name or path validation, revocation,
TLS, HTTP, OAuth transport, or network authority. Complete revocation policy,
repository-owned TLS/HTTP composition, audited network effects, and response
policy remain separate blockers. Concrete HTTPS and live-provider readiness
remain unavailable.

The same bounded OAuth boundary now also consumes that complete audited
request-to-signed-reviewed-server binding together with the exact HTTPS
destination-bound direct no-check complete fixed-path revocation proof. The
admitted direct completion must itself retain the exact signed reviewed-server
path-and-anchor-self-signature proof shape. The composition derives both paths
only through retained repository-owned evidence and requires the same exact
decoded leaf, target-nearest issuer, and reviewed-anchor objects, ASCII-case-
insensitively equal SNI DNS names, and ASCII-case-insensitively equal Host DNS
names with the same optional port. Both retained accumulated paths must contain
exactly one processed intermediate because this direct completion establishes
status only for the target and that one intermediate; longer paths fail closed
rather than inheriting an incomplete revocation claim. It then moves both
complete non-copyable inputs into one result. Requiring the already-composed
request/signed-server proof preserves the stronger path-and-anchor-self-
signature boundary instead of admitting a revocation proof as a substitute
for it. A byte-equal independent certificate decode, zero- or multi-
intermediate path, alternate intermediate or reviewed-root path, different
admitted SAN name, or different explicit port fails closed.
Stable failures and Debug output retain no provider, trace, request,
credential, certificate, hostname, authority, path, key, signature, algorithm,
constraint, revocation, status, or trust-root payloads.

This direct-branch composition does not borrow or copy the released request
bytes, so it introduces no credential access and emits no provider or trace
audit event. The release's existing audit gate remains mandatory for every
later request-byte borrow. It performs no parsing, new validation,
cryptography, credential editing, signing, storage, clock acquisition, status
acquisition or refresh, I/O, network access, or other external effect. It does
not generalize revocation or path construction, cover the other complete
fixed-path revocation branches, add trust-anchor semantics, complete RFC 5280
Name or path validation, build or run TLS or HTTP, authorize OAuth transport,
or grant network authority. The other complete revocation branches,
repository-owned TLS/HTTP composition, audited network effects, and response
policy remain separate blockers. Concrete HTTPS and live-provider readiness
remain unavailable.

The bounded OAuth boundary now also admits the exact HTTPS destination-bound
nested no-check complete fixed-path revocation proof as a separate nominal
branch. The nested completion must retain the same exact signed reviewed-
server path-and-anchor-self-signature proof shape used by the already-bound
request/server evidence. The composition derives both path identities only
through retained repository-owned evidence and requires the same exact decoded
leaf, target-nearest issuer, and reviewed-anchor objects, ASCII-case-
insensitively equal SNI DNS names, and ASCII-case-insensitively equal Host DNS
names with the same optional port. Both accumulated server paths must contain
exactly one processed intermediate. The nested completion adds its retained
responder-revocation and responder no-check policy layers, but its server-chain
coverage is still the target plus that one intermediate; zero- or multi-
intermediate server paths therefore fail closed. Success moves the complete
request/signed-server binding and complete nested-revocation proof into one
non-copyable result. A byte-equal independent certificate decode, alternate
intermediate or reviewed-root path, different admitted SAN name, or different
explicit port also fails closed. Stable failures and Debug output retain no
provider, trace, request, credential, certificate, hostname, authority, path,
key, signature, algorithm, constraint, revocation, status, or trust-root
payloads.

This nested-branch composition does not borrow or copy released request bytes,
so it introduces no credential access and emits no provider or trace audit
event. The release's existing audit gate remains mandatory for every later
request-byte borrow. It performs no parsing, new validation, cryptography,
credential editing, signing, storage, clock acquisition, status acquisition or
refresh, I/O, network access, or other external effect. It does not generalize
revocation or path construction, cover the double-nested, recursive, or
alternative intermediate-status-responder complete branches, add trust-anchor
semantics, complete RFC 5280 Name or path validation, build or run TLS or HTTP,
authorize OAuth transport, or grant network authority. Those remaining
revocation branches, repository-owned TLS/HTTP composition, audited network
effects, and response policy remain separate blockers. Concrete HTTPS and
live-provider readiness remain unavailable.

The bounded OAuth boundary now also admits the exact HTTPS destination-bound
double-nested no-check complete fixed-path revocation proof under its own
sealed nominal branch. The double-nested completion must retain the same exact
signed reviewed-server path-and-anchor-self-signature proof shape already held
by the request/server binding. The composition reaches that original server
proof only through both retained responder-revocation layers, derives both
server-path identities from repository-owned evidence, and requires the same
exact decoded leaf, target-nearest issuer, and reviewed-anchor objects,
ASCII-case-insensitively equal SNI DNS names, and ASCII-case-insensitively equal
Host DNS names with the same optional port. Both retained server paths must
contain exactly one processed intermediate: the additional responder paths do
not extend the target-plus-intermediate server chain whose status the
double-nested completion establishes. Zero- or multi-intermediate server paths
therefore fail closed. Success moves the complete request/signed-server binding
and complete double-nested-revocation proof into one non-copyable result. A
byte-equal independent leaf decode, alternate intermediate or reviewed root,
different admitted SAN name, or different explicit port also fails closed.
Stable failures and Debug output retain no provider, trace, request,
credential, certificate, hostname, authority, path, key, signature, algorithm,
constraint, revocation, status, or trust-root payloads.

This double-nested-branch composition does not borrow or copy released request
bytes, so it introduces no credential access and emits no provider or trace
audit event. The release's existing audit gate remains mandatory for every
later request-byte borrow. It performs no parsing, new validation,
cryptography, credential editing, signing, storage, clock acquisition, status
acquisition or refresh, I/O, network access, or other external effect. It does
not generalize revocation or path construction, cover the recursive or
alternative intermediate-status-responder complete branches, add trust-anchor
semantics, complete RFC 5280 Name or path validation, build or run TLS or HTTP,
authorize OAuth transport, or grant network authority. Those remaining
revocation branches, repository-owned TLS/HTTP composition, audited network
effects, and response policy remain separate blockers. Concrete HTTPS and
live-provider readiness remain unavailable.

The bounded OAuth boundary now also admits the exact HTTPS destination-bound
recursive-responder no-check complete fixed-path revocation proof under its
own sealed nominal branch. The recursive completion must retain the same exact
signed reviewed-server path-and-anchor-self-signature proof shape already held
by the request/server binding. The composition traverses the retained
recursive-responder no-check and authenticated responder-revocation layers
back to that original server proof, derives both server-path identities only
through repository-owned evidence, and requires the same exact decoded leaf,
target-nearest issuer, and reviewed-anchor objects, ASCII-case-insensitively
equal SNI DNS names, and ASCII-case-insensitively equal Host DNS names with the
same optional port. Both retained server paths must contain exactly one
processed intermediate. The recursive responder paths do not extend the
target-plus-intermediate server chain whose status the branch-specific
completion establishes, so zero- or multi-intermediate server paths fail
closed. Success moves the complete request/signed-server binding and complete
recursive-revocation proof into one non-copyable result. A byte-equal
independent leaf decode, alternate intermediate or reviewed root, different
admitted SAN name, or different explicit port also fails closed. Stable
failures and Debug output retain no provider, trace, request, credential,
certificate, hostname, authority, path, key, signature, algorithm, constraint,
revocation, status, or trust-root payloads.

This recursive-branch composition does not borrow or copy released request
bytes, so it introduces no credential access and emits no provider or trace
audit event. The release's existing audit gate remains mandatory for every
later request-byte borrow. It performs no parsing, new validation,
cryptography, credential editing, signing, storage, clock acquisition, status
acquisition or refresh, I/O, network access, or other external effect. It does
not generalize revocation or path construction, cover the alternative
intermediate-status-responder complete branch, add trust-anchor semantics,
complete RFC 5280 Name or path validation, build or run TLS or HTTP, authorize
OAuth transport, or grant network authority. That remaining revocation branch,
repository-owned TLS/HTTP composition, audited network effects, and response
policy remain separate blockers. Concrete HTTPS and live-provider readiness
remain unavailable.

The bounded OAuth boundary now also admits the exact HTTPS destination-bound
alternative intermediate-status-responder revocation no-check complete fixed-
path proof under its own sealed nominal branch. The alternative completion
must retain the same exact signed reviewed-server path-and-anchor-self-
signature proof shape already held by the request/server binding. The
composition traverses the retained intermediate-status-responder revocation,
responder no-check, and authenticated responder-revocation layers back to that
original server proof, derives both server-path identities only through
repository-owned evidence, and requires the same exact decoded leaf, target-
nearest issuer, and reviewed-anchor objects, ASCII-case-insensitively equal SNI
DNS names, and ASCII-case-insensitively equal Host DNS names with the same
optional port. Both retained server paths must contain exactly one processed
intermediate. The alternative responder paths do not extend the target-plus-
intermediate server chain whose status the branch-specific completion
establishes, so zero- or multi-intermediate server paths fail closed. Success
moves the complete request/signed-server binding and complete alternative-
revocation proof into one non-copyable result. A byte-equal independent leaf
decode, alternate intermediate or reviewed root, different admitted SAN name,
or different explicit port also fails closed. Stable failures and Debug output
retain no provider, trace, request, credential, certificate, hostname,
authority, path, key, signature, algorithm, constraint, revocation, status, or
trust-root payloads.

This alternative-branch composition does not borrow or copy released request
bytes, so it introduces no credential access and emits no provider or trace
audit event. The release's existing audit gate remains mandatory for every
later request-byte borrow. It performs no parsing, new validation,
cryptography, credential editing, signing, storage, clock acquisition, status
acquisition or refresh, I/O, network access, or other external effect. It does
not generalize revocation or path construction, add trust-anchor semantics,
complete RFC 5280 Name or path validation, build or run TLS or HTTP, authorize
OAuth transport, or grant network authority. Repository-owned TLS/HTTP
composition, audited network effects, and response policy remain separate
blockers. Concrete HTTPS and live-provider readiness remain unavailable.

The existing bounded `x509-ocsp-reviewed-server-revocation-binding`
prerequisite now also provides an ordered entry point that consumes fully
routed authenticated OCSP target status together with the complete reviewed
server-path-and-anchor-self-signature proof. It derives the target and
target-nearest issuer only through the signed wrapper's retained accumulated
Name-policy path, requires the OCSP and path proofs to retain the same exact
decoded target and issuer objects, validates both path certificates at the
OCSP proof's retained observation time, and retains both complete non-copyable
inputs with the resulting exact-time validity evidence. Byte-equal
independently decoded target or issuer objects fail closed. The earlier
path-only entry point remains available and unchanged.

This ordered composition performs no parsing, new signature verification,
credential access or editing, signing, storage, clock acquisition, provider
action, audit event, trace emission, I/O, network access, or other external
effect. It does not fetch or refresh status, add trust-anchor semantics,
authorize the delegated responder path, establish status for every non-root
certificate, complete Name, path, or revocation validation, build or run TLS
or HTTP, authorize OAuth transport, or grant network authority. Responder and
intermediate revocation policy, repository-owned TLS/HTTP, and audited
transport remain separate blockers. Concrete HTTPS and live-provider
readiness remain unavailable.

The same repository-owned revocation boundary now exposes a sealed target-
evidence projection for the fully routed reviewed-server proof and carries it
through the exact direct no-check fixed-path revocation completion. The
projection derives the leaf only from the retained authenticated target-status
proof whose safe constructor already required exact identity with the retained
reviewed path. External callers cannot implement substitute target evidence.
The direct completion returns that same exact decoded certificate object
through its complete retained proof, without accepting a detached certificate,
callback, path, status result, or trust assertion. This prepares a later HTTPS
destination-to-complete-revocation binding without moving or duplicating the
retained direct-revocation proof.

The nested no-check fixed-path completion carries the same sealed target-
evidence projection through its deeper retained proof graph. It derives the
server leaf only from the original reviewed-server proof inside the complete
nested composition and returns that same exact decoded certificate object.
No detached certificate, callback-shaped claim, path, status result, or trust
assertion can be substituted. This is target projection only; binding an HTTPS
destination to this nested completion remains a separate slice.

This projection performs no parsing, new validation, cryptography, credential
access or editing, signing, storage, clock acquisition, provider action, audit
event, trace emission, I/O, network access, or other external effect. It does
not bind an HTTPS destination, generalize revocation or path construction,
build or run TLS or HTTP, authorize OAuth transport, or grant network
authority. The remaining complete fixed-path revocation branches and the final
HTTPS composition remain separate slices. Concrete HTTPS and live-provider
readiness remain unavailable.

The existing bounded `https-reviewed-server-path-binding` prerequisite now
also joins the complete HTTPS destination proof directly to the exact direct
no-check complete fixed-path revocation proof. It derives the server leaf only
through that completion's sealed repository-owned target projection, requires
the destination and completed revocation policy to retain the same exact
decoded certificate object, and moves both complete non-copyable inputs into
the result. A byte-equal independent decode fails closed, and callers cannot
supply a detached certificate, callback-shaped target claim, path, status
result, or trust assertion. Stable failures and Debug output retain no
certificate, hostname, authority, path, key, signature, algorithm, status,
constraint, or root payloads. This direct-revocation branch is orthogonal to
the separate path-to-anchor-self-signature composition: it does not require,
retain, or imply that signed-anchor evidence.

This orthogonal direct-branch composition performs no parsing, new validation,
cryptography, credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network access,
or other external effect. It does not generalize revocation or path
construction, cover the other complete fixed-path revocation branches, build
or run TLS or HTTP, authorize OAuth transport, or grant network authority.
Those other branches, repository-owned TLS/HTTP composition, and audited
transport remain separate blockers. Concrete HTTPS and live-provider
readiness remain unavailable.

The same `https-reviewed-server-path-binding` boundary now also joins the
complete HTTPS destination proof to the exact nested no-check complete fixed-
path revocation proof. It derives the server leaf only through that
completion's deeper sealed repository-owned target projection, requires both
non-copyable inputs to retain the same exact decoded certificate object, and
moves both complete inputs into the result. A byte-equal independent decode
fails closed, and callers cannot supply a detached certificate, callback-
shaped target claim, path, status result, or trust assertion. Stable failures
and Debug output retain no certificate, hostname, authority, path, key,
signature, algorithm, status, constraint, or root payloads. The nested branch
does not require, retain, or imply the separate anchor-self-signature
composition.

This nested-branch composition performs no parsing, new validation,
cryptography, credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network
access, or other external effect. It does not generalize revocation or path
construction, cover the remaining complete fixed-path revocation branches,
build or run TLS or HTTP, authorize OAuth transport, or grant network
authority. Those remaining branches, repository-owned TLS/HTTP composition,
and audited transport remain separate blockers. Concrete HTTPS and live-
provider readiness remain unavailable.

The existing `x509-ocsp-reviewed-server-revocation-binding` prerequisite now
also carries its sealed exact-server-target projection through the complete
double-nested no-check fixed-path revocation proof. The projection traverses
only the completion's retained repository-owned evidence graph back to the
original reviewed-server proof and returns that proof's exact decoded target
certificate object. A byte-equal independent decode remains a different
object, and callers cannot supply a detached certificate, callback-shaped
claim, path, status result, or trust assertion.

This double-nested projection performs no parsing, new validation,
cryptography, credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network
access, or other external effect. It does not bind an HTTPS destination,
generalize revocation or path construction, build or run TLS or HTTP,
authorize OAuth transport, or grant network authority. The composition below,
the branch-specific compositions below, repository-owned TLS/HTTP composition,
and audited transport remain separate from this projection. Concrete HTTPS and
live-provider readiness remain unavailable.

The `https-reviewed-server-path-binding` boundary now also joins the complete
HTTPS destination proof to that exact double-nested no-check complete fixed-
path revocation proof. The binder derives the server leaf only through the
completion's sealed target projection, requires both non-copyable inputs to
retain the same exact decoded certificate object, and moves both complete
inputs into the result. A byte-equal independent decode fails closed, and the
surface accepts no detached certificate, callback-shaped target claim, path,
status result, or trust assertion. Stable failures and Debug output retain no
certificate, hostname, authority, path, key, signature, algorithm, status,
constraint, or root payloads. This branch neither requires nor implies the
separate anchor-self-signature composition.

This double-nested composition performs no parsing, new validation,
cryptography, credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network
access, or other external effect. It does not generalize revocation or path
construction, cover the other complete fixed-path revocation branches, build
or run TLS or HTTP, authorize OAuth transport, or grant
network authority. The branch-specific compositions below, repository-owned
TLS/HTTP composition, and audited transport remain separate boundaries.
Concrete HTTPS and live-provider readiness remain unavailable.

The existing `x509-ocsp-reviewed-server-revocation-binding` prerequisite now
also carries its sealed exact-server-target projection through the complete
recursive responder no-check fixed-path revocation proof. The projection
traverses only that completion's retained repository-owned evidence graph,
including its recursive responder-revocation layer, back to the original
reviewed-server proof and returns the exact decoded target certificate retained
there. A byte-equal independent decode remains a different object, and callers
cannot supply a detached certificate, callback-shaped claim, path, status
result, or trust assertion.

This recursive-branch projection performs no parsing, new validation,
cryptography, credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network
access, or other external effect. It does not bind an HTTPS destination,
generalize revocation or path construction, build or run TLS or HTTP,
authorize OAuth transport, or grant network authority. The composition below,
repository-owned TLS/HTTP composition, and audited transport remain separate
from this projection. Concrete HTTPS and live-provider readiness remain
unavailable.

The `https-reviewed-server-path-binding` boundary now also joins the complete
HTTPS destination proof to that exact recursive responder no-check complete
fixed-path revocation proof. The binder derives the server leaf only through
the completion's sealed target projection, requires both non-copyable inputs
to retain the same exact decoded certificate object, and moves both complete
inputs into the result. A byte-equal independent decode fails closed, and the
surface accepts no detached certificate, callback-shaped target claim, path,
status result, or trust assertion. Stable failures and Debug output retain no
certificate, hostname, authority, path, key, signature, algorithm, status,
constraint, or root payloads. This branch neither requires nor implies the
separate anchor-self-signature composition.

This recursive-branch composition performs no parsing, new validation,
cryptography, credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network
access, or other external effect. It does not generalize revocation or path
construction, build or run TLS or HTTP, authorize OAuth transport, or grant
network authority. Repository-owned TLS/HTTP composition and audited transport
remain separate blockers. Concrete HTTPS and live-provider readiness remain
unavailable.

The existing `x509-ocsp-reviewed-server-revocation-binding` prerequisite now
also carries its sealed exact-server-target projection through the complete
alternative intermediate-status-responder revocation no-check fixed-path
proof. The projection traverses only that completion's retained repository-
owned evidence graph back to the original reviewed-server proof and returns
the exact decoded target certificate retained there. A byte-equal independent
decode remains a different object, and callers cannot supply a detached
certificate, callback-shaped claim, path, status result, or trust assertion.

This alternative-branch projection performs no parsing, new validation,
cryptography, credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network
access, or other external effect. It does not bind an HTTPS destination,
generalize revocation or path construction, build or run TLS or HTTP,
authorize OAuth transport, or grant network authority. Its HTTPS destination
composition below, repository-owned TLS/HTTP composition, and audited
transport remain separate from this projection. Concrete HTTPS and live-
provider readiness remain unavailable.

The `https-reviewed-server-path-binding` boundary now also joins the complete
HTTPS destination proof to that exact alternative intermediate-status-
responder revocation no-check complete fixed-path proof. The binder derives
the server leaf only through the completion's sealed target projection,
requires both non-copyable inputs to retain the same exact decoded certificate
object, and moves both complete inputs into the result. A byte-equal
independent decode fails closed, and the surface accepts no detached
certificate, callback-shaped target claim, path, status result, or trust
assertion. Stable failures and Debug output retain no certificate, hostname,
authority, path, key, signature, algorithm, status, constraint, or root
payloads. This branch neither requires nor implies the separate anchor-self-
signature composition.

This alternative-branch composition performs no parsing, new validation,
cryptography, credential access or editing, signing, storage, clock
acquisition, provider action, audit event, trace emission, I/O, network
access, or other external effect. It does not generalize revocation or path
construction, build or run TLS or HTTP, authorize OAuth transport, or grant
network authority. Repository-owned TLS/HTTP composition and audited transport
remain separate blockers. Concrete HTTPS and live-provider readiness remain
unavailable.

An allocation-free `form-urlencoded` prerequisite now validates an ordered,
caller-borrowed sequence of UTF-8 name/value pairs under explicit field,
component, and encoded-output ceilings. It serializes the exact
`application/x-www-form-urlencoded` byte profile into caller-owned storage:
ASCII alphanumerics plus `*`, `-`, `.`, and `_` pass through, space becomes
`+`, and every other UTF-8 byte becomes uppercase percent encoding. Empty names
and values, duplicate names, and caller order are preserved because their
meaning belongs to the consuming protocol. The complete encoded length is
proven before any write, so a small output remains unchanged. Stable failures
retain no field bytes, counts, lengths, limits, or output bytes, and Debug
redacts every name and value.

This serializer does not validate OAuth parameter names, uniqueness, grant
semantics, or provider policy; interpret or release credentials; allocate or
wipe its caller-owned output; emit provider or trace audit events; create HTTP
headers or fixed-body evidence; perform TLS or I/O; or authorize HTTP, OAuth,
or live-provider transport. Secret-bearing callers must obtain inputs through
the audited custody path and wipe the output after its separately audited use.
Concrete HTTPS remains unavailable.

An allocation-free `http1-request-header-field` prerequisite now validates and
encodes one deliberately narrow HTTP/1 request field line into caller-owned
storage. Caller limits bound the token-only field name, ASCII field value, and
complete line. Values may be empty or contain internal space and horizontal
tab, but leading or trailing optional whitespace, controls, DEL, and non-ASCII
bytes fail closed. The encoder emits the exact validated name, colon, one
space, value, and CRLF only after proving the complete output fits, so a small
output remains unchanged. Success retains borrowed name and potentially
sensitive value bytes plus the exact encoded length without allocation. Stable
failures retain no field bytes or lengths, and Debug redacts both inputs.

This syntax boundary does not authorize a field name or value, interpret or
release a credential, detect duplicate or conflicting fields, enforce Host or
body-framing policy, compose a request head, perform TLS, read or write a
socket, emit provider or trace audit events, or authorize HTTP or OAuth
transport. Those semantic and external-effect policies remain separate.
Concrete HTTPS remains unavailable.

An allocation-free `tls13-key-share` prerequisite now binds exact TLS 1.3
`KeyShareEntry` values to the existing caller-selected named-group policy.
Client construction validates each public key-exchange value against the
selected group's immutable wire length, rejects duplicate groups, and encodes
the complete two-byte-length-prefixed ClientHello extension data only after
proving caller-owned output capacity. Construction and decoding also admit the
exact empty ClientHello vector required for an explicit HelloRetryRequest group
selection round trip; every nonempty vector remains subject to the same policy
and fixed registry-sized entry ceiling. Successful decoding retains the exact
complete caller-borrowed ClientHello extension data, including its vector-
length prefix, together with the borrowed public key-exchange entries. The
complete input remains available through a read-only accessor so later
message-context composition can compare it directly without re-encoding or
allocation. ServerHello decoding accepts exactly one entry and fails closed
unless its selected group appeared in the exact retained client offer. Stable
failures retain no group identifiers, public-key bytes, lengths, or output
contents.

The empty vector is syntax evidence only: this public framing and selection
evidence does not implement or authorize HelloRetryRequest, bind
`supported_groups`, require a retry to make progress, generate entropy or an
ephemeral private key, invoke X25519 or P-256, validate a peer point beyond the
selected group's exact wire length, reject a low-order key, derive or retain a
shared secret, update a transcript, process a handshake, derive traffic
secrets, protect records, validate certificates, establish trust, enforce
revocation, perform I/O, authorize HTTP or OAuth transport, or open a network
connection. Reviewed retry-transition policy, key-agreement execution, and
handshake-state composition remain prerequisites, and concrete HTTPS remains
unavailable.

An allocation-free `tls13-key-share-retry` prerequisite binds the public
group transition requested by an RFC 9846 `HelloRetryRequest`. It accepts
exactly two bytes of `key_share` extension data as the requested
`selected_group`, authorizes that identifier through the caller's existing
data-driven named-group policy, requires the group to occur in the exact
retained first-ClientHello `supported_groups` list, and rejects a group that
already occurred in the exact retained first-ClientHello key-share offer. A
second binding step then requires the decoded second-ClientHello key-share
vector to contain exactly one entry and requires that entry to use the retry-
selected group. Success consumes and retains the supported-group list, both
decoded ClientHello key-share vectors, the exact retry extension data, and the
policy-authorized group descriptor as non-copyable transition evidence.
An additional final-share binding step borrows that completed transition and
decodes the exact ServerHello key-share extension data directly against the
transition's retained second-ClientHello offer under the caller-supplied
named-group policy boundary. Success returns non-copyable continuity evidence
retaining both the borrowed retry transition and the decoded ServerHello
share, so a
caller cannot substitute an unrelated offer between retry validation and
final server selection. The ServerHello decoder's exact single-entry framing,
key-exchange-length policy, and offered-group requirement remain authoritative.
Stable failures and diagnostics retain no group identifiers, extension data,
public-key bytes, list positions, counts, or lengths.

This group-transition evidence does not prove key material was freshly
generated or prevent reuse across connections; generate or own entropy or an
ephemeral private key; invoke X25519 or P-256; validate a peer point beyond
the selected group's exact wire length; locate or authorize extension type 51
in a handshake message; compare any other ClientHello field; apply a retry
cookie, PSK binder update, early-data removal, or transcript `message_hash`
rewrite; reject a second retry; sequence a handshake; send alerts; derive
traffic secrets; protect records; validate certificates; establish trust;
enforce revocation; perform I/O; authorize HTTP or OAuth transport; or open a
network connection. Complete ClientHello retry-transition policy, connection
state, transcript composition, key agreement, and the remaining TLS/HTTP stack
remain separate prerequisites. Concrete HTTPS and live-provider readiness
remain unavailable.

An allocation-free composition boundary in
`tls13-server-hello-retry-selection` now attaches that retry key-share
transition to the exact decoded messages already retained by one
`BoundServerHelloRetrySelection`. After the retained extension-context policy
has authorized both ClientHello/ServerHello pairs, the composer locates the
first ClientHello `supported_groups` and `key_share`, the HelloRetryRequest
`key_share`, and the second ClientHello `key_share` directly in those exact
extension blocks. It decodes them under caller-selected resource ceilings and
one retained data-driven named-group policy, then runs the repository-owned
retry-transition binders. Success retains the complete retry-selection proof,
the exact policy, and the non-copyable transition evidence. A final step
locates the ordinary ServerHello `key_share` from the same retained proof and
returns message-bound final-share evidence that cannot outlive that composed
transition. Missing, malformed, disallowed, or inconsistent public inputs fail
with stable diagnostics that retain no extension data, group identifiers,
public-key bytes, counts, positions, or lengths.

This composition proves only the origin and continuity of public key-share
evidence across the four retained messages. It does not compare every other
permitted ClientHello2 change; apply a retry cookie, PSK binder update, or
early-data removal; require fresh private key material; invoke key agreement;
apply the transcript `message_hash` rewrite; reject a second retry in
connection state; sequence a handshake; derive traffic secrets; protect
records; validate certificates; establish trust; enforce revocation; perform
I/O; authorize HTTP or OAuth transport; or open a network connection. Complete
retry-transition policy, connection state, transcript composition, key
agreement, and the remaining TLS/HTTP stack remain separate prerequisites.
Concrete HTTPS and live-provider readiness remain unavailable.

A bounded `tls13-x25519-key-agreement` prerequisite now routes one key-
agreement operation only from an exact `tls13-named-group-policy` selection.
The selected group must bind the X25519 algorithm, standard 32-byte peer key-
exchange length, and standard 32-byte shared-secret length before the caller-
borrowed private scalar is read or the repository-owned X25519 primitive is
invoked. Every other recognized group fails before the private scalar is read,
before peer bytes are interpreted, and before a shared-secret owner exists.
The X25519 route passes the exact peer key-exchange bytes to the repository-
owned primitive, preserves its low-order all-zero rejection, validates the
derived length through the same selected-group policy, and returns one non-
cloneable shared-secret value borrowing caller-owned fixed storage. Dropping
that value wipes the borrowed storage through the repository-owned zeroization
primitive.
Stable failures and Debug output retain no private-scalar, peer-key, shared-
secret, group, algorithm, or length payloads.

This pure execution boundary does not generate entropy or an ephemeral private
key, own or wipe the caller's private scalar, construct or decode a key-share
extension, prove that a private scalar corresponds to an offered ClientHello
share, bind a ServerHello share to that offer, advance a transcript or
handshake, derive traffic secrets, protect records, validate certificates,
establish trust, enforce revocation, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. The existing X25519 primitive's
optimized-code behavior and internal temporary handling remain properties of
that package rather than new claims of this composition. Exact key-share and
handshake-state binding, the remaining TLS transport, and HTTP composition
remain prerequisites; concrete HTTPS and live-provider readiness remain
unavailable.

An allocation-free `tls13-x25519-key-share-binding` prerequisite now composes
the exact retained ClientHello offer and decoded ServerHello key share with the
selected-group-bound X25519 execution boundary. The decoded server share must
select X25519 before the caller-borrowed private scalar is read. The package
derives that scalar's public key through the repository-owned X25519 primitive
and compares it with the exact retained X25519 ClientHello public share through
the repository-owned compiler-preserved full-scan fixed-length comparison. A
mismatch fails before shared-secret derivation and leaves caller-owned output
unchanged. A match passes the exact decoded ServerHello public share and exact
selected-group evidence to the existing X25519 key-agreement boundary, which
continues to reject low-order peer keys and wipe the borrowed shared-secret
storage on drop. Stable failures and Debug output retain no private scalar,
public key, shared secret, group, algorithm, length, or output payloads.

This pure binding boundary does not generate entropy or an ephemeral private
key, own or wipe the caller's private scalar, prove that the retained offer was
transmitted in a ClientHello, bind the offer or ServerHello to a transcript,
process a handshake, derive traffic secrets, protect records, validate
certificates, establish trust, enforce revocation, perform I/O, authorize HTTP
or OAuth transport, or open a network connection. The fixed-length comparison
inherits its repository package's compiler-observability guarantee and does
not add hardware side-channel claims. The X25519 primitive's optimized-code
behavior and internal temporary handling remain properties of that package.
Handshake-state and transcript binding, the remaining TLS transport, and HTTP
composition remain prerequisites; concrete HTTPS and live-provider readiness
remain unavailable.

The `tls13-x25519-key-share-binding` composition additionally accepts the
message-bound final-share evidence returned by
`tls13-server-hello-retry-selection`. That entry point obtains the decoded
ServerHello share only through the retained CH1, HelloRetryRequest, CH2, and
final-ServerHello proof, then applies the existing scalar-to-offer comparison
and repository-owned X25519 execution without accepting detached key-share or
policy input. Success returns a non-copyable wrapper borrowing the complete
message proof and owning the existing wipe-on-drop shared-secret view, so
later composition cannot separate the derived secret from the exact public
handshake evidence that authorized it. Failures preserve the existing stable,
payload-free categories and leave caller-owned output unchanged.

This message-bound execution still does not generate entropy or an ephemeral
private key, own or wipe the caller's private scalar, compare every permitted
ClientHello2 change, apply a retry cookie or PSK binder update, bind the
messages or secret to a transcript, apply the transcript `message_hash`
rewrite, reject a second retry in connection state, process a handshake,
derive traffic secrets, protect records, validate certificates, establish
trust, enforce revocation, perform I/O, authorize HTTP or OAuth transport, or
open a network connection. Handshake-state and transcript composition, the
remaining TLS transport, and HTTP composition remain prerequisites; concrete
HTTPS and live-provider readiness remain unavailable.

An allocation-free `tls13-retry-transcript-binding` prerequisite now constructs
one selected TLS 1.3 retry transcript only from the exact CH1,
HelloRetryRequest, CH2, and final-ServerHello proof retained by
`tls13-server-hello-retry-selection`. The composer frames each retained body
with its canonical handshake type and three-byte length, hashes CH1 through
both supported transcript algorithms, selects the policy-authorized cipher
suite carried by the retained HelloRetryRequest proof, applies the existing
one-shot synthetic `message_hash` rewrite, and then hashes the exact retained
HelloRetryRequest, CH2, and final ServerHello encodings in order. Success
returns a non-copyable wrapper borrowing that complete message proof and owning
the selected transcript state, so later composition cannot substitute another
message sequence or detached cipher-suite policy. Transcript-limit or hashing
failures retain no message bytes, digest, length, suite, or policy payloads.

This pure composition does not validate every permitted ClientHello2 change,
apply a retry cookie or PSK binder update, reject a second retry in connection
state, own handshake sequencing, combine the transcript with a key-agreement
secret, derive handshake or traffic secrets, authenticate Finished, protect
records, validate certificates, establish trust, enforce revocation, perform
I/O, authorize HTTP or OAuth transport, or open a network connection. The
remaining TLS state, key-schedule, record, certificate, revocation, and HTTP
composition remain prerequisites; concrete HTTPS and live-provider readiness
remain unavailable.

An allocation-free `tls13-retry-key-schedule-binding` prerequisite now joins
the message-bound retry transcript and message-bound X25519 shared secret only
when both borrow the same exact retained CH1, HelloRetryRequest, CH2, and final
ServerHello selection proof. Before touching caller-owned base-secret storage,
the composer also requires that storage's data-driven selected cipher suite
equal the suite retained by the exact transcript. Only then does it pass the
repository-owned 32-byte X25519 secret to the existing no-PSK base schedule.
Success returns a non-copyable wrapper retaining both exact message-bound
inputs and the Early, Handshake, and Master Secret view into caller-owned
wipe-on-drop storage. Proof or
suite substitution fails with stable payload-free categories before schedule
derivation or storage mutation; downstream schedule failures retain the
existing redacted categories and clearing behavior.

This pure binding does not derive client or server handshake traffic secrets,
consume the retained transcript hash in Derive-Secret, sequence any later
handshake message, authenticate CertificateVerify or Finished, protect
records, validate certificates, establish trust, enforce revocation, perform
I/O, authorize HTTP or OAuth transport, or open a network connection. Traffic-
secret derivation, authenticated handshake progression, record protection,
certificate/revocation policy, and HTTP composition remain prerequisites;
concrete HTTPS and live-provider readiness remain unavailable.

An allocation-free `tls13-handshake-traffic-secrets` prerequisite now derives
the client and server handshake traffic secrets from the existing
suite-selected `BaseSecrets` view and one repository-owned `TranscriptHash`.
Caller-owned fixed storage must retain the exact same data-driven selected
cipher suite as the base schedule, and the transcript snapshot's algorithm
must equal that suite's HKDF hash. Only then does the primitive import the
existing Handshake Secret into a fixed wipe-on-drop PRK, apply the fixed
`"c hs traffic"` and `"s hs traffic"` labels, stage both Hash-length results in
wipe-on-drop owners, and copy them into caller-owned wipe-on-drop storage only
after both derivations succeed. Success returns a non-copyable borrowed view;
stable failures retain no secret or transcript bytes.

This primitive accepts an already-computed transcript snapshot and does not
prove which messages produced it, own transcript sequencing, or attach its
outputs to the exact retry proof. It also does not derive traffic keys or IVs,
authenticate CertificateVerify or Finished, protect records, validate
certificates, establish trust, enforce revocation, perform I/O, authorize HTTP
or OAuth transport, or open a network connection.

An allocation-free `tls13-retry-handshake-traffic-secrets-binding`
prerequisite now accepts only the exact-message `BoundRetryBaseSecrets` proof
and caller-owned handshake-traffic-secret storage. Before touching that
storage, it requires the storage's data-driven selected cipher suite to equal
the suite retained by the bound retry transcript. It then snapshots that exact
CH1, HelloRetryRequest, CH2, and final ServerHello transcript and delegates the
bound base secrets plus snapshot to the fixed-memory handshake traffic-secret
primitive. Success returns a non-copyable wrapper that retains the complete
message-bound base schedule beside the borrowed client and server secret view;
no detached message, suite, transcript hash, or base secret is accepted.
Suite substitution fails before output mutation, and downstream derivation
failures retain the existing stable redacted categories and clearing behavior.

This exact-message binding does not advance the transcript beyond ServerHello,
derive traffic keys or IVs, authenticate CertificateVerify or Finished,
protect records, validate certificates, establish trust, enforce revocation,
perform I/O, authorize HTTP or OAuth transport, or open a network connection.
Authenticated handshake progression, record protection, certificate and
revocation policy, and HTTP composition remain prerequisites; concrete HTTPS
and live-provider readiness remain unavailable.

The same exact-message binding now also derives the server Finished key only
from its retained server handshake traffic secret. Caller-owned Finished-key
storage must select the exact same data-driven cipher suite and the `Server`
sender role before any output mutation. The composer then imports only the
retained server secret under the suite-selected SHA-256 or SHA-384 policy and
delegates the fixed `"finished"` label derivation to the repository-owned
Finished boundary. Non-copyable success evidence retains the complete CH1,
HelloRetryRequest, CH2, final ServerHello, base-schedule, and handshake-
traffic-secret proof beside the borrowed key view into caller-owned wipe-on-
drop storage. No detached traffic secret, suite, role, label, or key is
accepted; stable failures retain no secret bytes.

This provenance binding does not verify peer Finished data, advance the
transcript beyond ServerHello, authenticate CertificateVerify, prove later
handshake-message order, protect records, validate certificates, establish
trust, enforce revocation, perform I/O, authorize HTTP or OAuth transport, or
open a network connection. Authenticated handshake progression, certificate
and revocation policy, record protection, and HTTP composition remain
prerequisites; concrete HTTPS and live-provider readiness remain unavailable.

The exact retry server Finished-key proof can now also reconstruct a fresh
selected transcript state only from its retained CH1, HelloRetryRequest, CH2,
and final ServerHello message graph and the original transcript policy retained
by that graph. No caller-supplied transcript, policy, message, suite, hash, or
snapshot is accepted. The reconstruction must reproduce both the retained
selected suite and the exact ServerHello transcript snapshot before success.
Non-copyable success evidence keeps the complete retry-derived server
Finished-key provenance beside the reconstructed state, providing a bounded
starting point for later authenticated server-flight transcript composition
without detaching the key schedule from the retry messages that selected it.

This reconstruction does not advance beyond ServerHello, authenticate or
sequence EncryptedExtensions, Certificate, CertificateVerify, or Finished,
consume authenticated records, validate certificates, establish trust,
enforce revocation, protect records, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. Authenticated ordered transcript
advancement and all remaining TLS, certificate, revocation, HTTP, and audited
transport composition remain prerequisites; concrete HTTPS and live-provider
readiness remain unavailable.

An allocation-free `tls13-retry-handshake-traffic-key-material-binding`
prerequisite now accepts only the exact-message retry handshake-traffic-secret
proof above, one explicit client-or-server sender role, and caller-owned
traffic-key-material storage. Before touching that storage, it requires the
storage's data-driven selected cipher suite to equal the suite retained by the
bound traffic secrets. It selects only the retained sender secret, imports
that exact hash-length value into a fixed wipe-on-drop PRK using the retained
suite's hash, and delegates to the existing fixed-memory traffic-key and
static-IV derivation. Success returns a non-copyable wrapper retaining the
complete CH1, HelloRetryRequest, CH2, final ServerHello, base-schedule, and
handshake-traffic-secret proof beside the sender role and borrowed key-material
view. No detached traffic secret, suite, hash, key or IV length, label, or
context is accepted. Suite substitution fails before output mutation;
downstream failures retain stable redacted categories and the existing
clearing and staging behavior.

This exact-message binding derives one direction's handshake key and static IV
per call. It does not advance the transcript, authenticate CertificateVerify
or Finished, construct or reserve a record nonce, create record-protection
state, invoke an AEAD, frame or protect records, process alerts, validate
certificates, establish trust, verify a hostname, enforce revocation, perform
I/O, authorize HTTP or OAuth transport, or open a network connection.
Authenticated handshake progression, record-state composition, certificate and
revocation policy, and HTTP composition remain prerequisites; concrete HTTPS
and live-provider readiness remain unavailable.

The same allocation-free boundary now also consumes the complete exact-message
retry handshake traffic-secret proof into a non-cloneable per-sender
record-state issuer. One successful client issuance and one successful server
issuance are permitted. A repeated issuance for the same sender fails before
the caller-owned output storage is touched, so the same retained key and static
IV cannot acquire a second sequence-zero state through this boundary. Failed
derivation or state construction does not consume the sender's issuance, so a
caller may retry with valid storage.

Each successful issuance accepts only an explicit sender role and
caller-owned traffic-key-material storage for the retained selected suite. It
derives that retained sender's key and static IV, moves the resulting view into
the existing repository-owned record-state primitive, starts the monotonic
record sequence at zero, and marks the sender issued before returning. The
non-copyable result borrows the single-use issuer, retains the explicit sender,
and permits only borrowed access to the state. It accepts no detached key,
static IV, nonce length, record ceiling, AEAD identifier, sequence number, or
cipher suite, and it cannot detach the state from the exact CH1,
HelloRetryRequest, CH2, final ServerHello, base-schedule, traffic-secret, and
sender evidence.

This exact-message state composition does not advance the handshake
transcript, authenticate CertificateVerify or Finished, invoke an AEAD, frame
or protect a record, process alerts, validate certificates, establish trust,
verify a hostname, enforce revocation, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. Authenticated handshake progression,
record protection, certificate and revocation policy, and HTTP composition
remain prerequisites; concrete HTTPS and live-provider readiness remain
unavailable.

The exact retry record-state wrapper can now be borrowed by an allocation-free
handshake-record binding that delegates only to the existing repository-owned,
data-driven record-protection dispatcher. Sealing accepts one borrowed
handshake message, explicit zero-padding length, the existing ciphertext and
inner-plaintext policies, caller scratch, and caller output. It fixes the
protected content type to `Handshake`, consumes the next nonce only under the
existing dispatcher rules, and returns non-copyable evidence retaining the
exact state wrapper, plaintext request, and sealed output prefix. Callers
cannot substitute a detached key, IV, nonce, sequence, AEAD route, content
type, or output prefix.

Opening retains the exact ciphertext input and state wrapper beside the
existing authentication-before-release output owner. An authenticated inner
content type other than `Handshake` is rejected before any content view is
returned; dropping that rejected opened owner clears the authenticated output
prefix. Success returns non-copyable evidence exposing only the authenticated
handshake content, padding length, sender role, selected suite, exact input,
and retained retry proof. AEAD or policy failure preserves the existing
redacted categories and nonce-consumption rules.

This binding does not parse or sequence the authenticated handshake message,
advance its transcript, authenticate CertificateVerify or Finished, process
alerts, select epochs, perform key updates, validate certificates, establish
trust, verify a hostname, enforce revocation, perform I/O, authorize HTTP or
OAuth transport, or open a network connection. Authenticated handshake
progression, certificate and revocation policy, and HTTP/TLS transport
composition remain prerequisites; concrete HTTPS and live-provider readiness
remain unavailable.

The exact retry record boundary can now be consumed into an allocation-free
authenticated handshake reassembler that owns both the single-use sender state
and one existing bounded handshake reassembler over caller-owned storage. Each
record is opened internally through the repository-owned dispatcher, must
authenticate as inner content type `Handshake`, and is copied into the bounded
reassembler only after authentication. The owner therefore preserves one
monotonic sender sequence and cannot accept detached plaintext fragments or a
second record state.

One call drains the complete authenticated record content. It invokes a
caller-supplied, non-retaining handler once for each complete policy-authorized
handshake message, continues across coalesced messages, and retains an
incomplete final message for the next authenticated record. Success returns
non-copyable evidence retaining the exact ciphertext input and reassembly
owner beside the completed-message count, authenticated content length,
partial-message status, sender role, selected suite, and post-open record
count. Record authentication, unexpected-content-type rejection, output
clearing, reassembly bounds, initialized-storage clearing, and stable redacted
failures remain those of the composed repository-owned primitives.
Any record authentication, content-type, reassembly, or handler-unwind failure
clears the buffered fragment and permanently poisons that owner. Later calls
fail before record processing, so a rejected sequence position cannot splice
non-contiguous authenticated records into one handshake message.

This composition does not interpret a handshake body, enforce the no-PSK
server-flight order, advance a transcript, bind handler behavior to later
state, authenticate CertificateVerify or Finished, process alerts, select
epochs, perform key updates, validate certificates, establish trust, verify a
hostname, enforce revocation, perform I/O, authorize HTTP or OAuth transport,
or open a network connection. Authenticated ordered handshake progression,
certificate and revocation policy, and HTTP/TLS transport composition remain
prerequisites; concrete HTTPS and live-provider readiness remain unavailable.

The authenticated retry reassembly boundary can now be consumed into a
server-only ordered-flight owner that composes the exact monotonic server
record state with the existing `tls13-no-psk-server-flight` transition policy.
Construction rejects client-sender state and any reassembler that already
consumed or failed a record, then starts only at `ExpectEncryptedExtensions`;
callers cannot inject a detached ordering state, carry a pre-composition
fragment, or resume at a later position. For each complete policy-authorized message,
the owner advances the public no-PSK server-flight type sequence before it
lends the message to the caller's non-retaining handler. The handler receives
an ordered wrapper retaining the exact authenticated message beside the
post-accept state and client-authentication requirement, so it cannot mistake
pre-transition bytes for ordered progression.

Success returns non-copyable evidence retaining the exact ciphertext input,
authenticated reassembly result, post-record server-flight state, and
client-authentication requirement. An unexpected message type permanently
poisons and clears the composed reassembly owner before returning a stable,
payload-free ordering error; later records fail before authentication or
handler invocation. Record, content-type, reassembly, and handler-unwind
failures retain the same terminal behavior. A handler may already have acted
on earlier valid messages in the same authenticated record before a later
message fails ordering, so any such caller-owned effect remains caller-audited
and is not transactional.

This bounded composition enforces only the public no-PSK server-flight message
type order over authenticated retry-handshake records. It does not interpret
message bodies, advance a transcript, validate EncryptedExtensions, process a
CertificateRequest, validate a certificate chain or hostname, authenticate
CertificateVerify or Finished, establish trust, enforce revocation, select
epochs, perform key updates, emit an alert, perform I/O, authorize HTTP or
OAuth transport, or open a network connection. Body-specific processing,
transcript and signature authentication, certificate and revocation policy,
and HTTP/TLS transport composition remain prerequisites; concrete HTTPS and
live-provider readiness remain unavailable.

The fresh retry server transcript-continuation proof can now be consumed
together with one fresh authenticated ordered server-flight owner only when
both retain the same exact handshake-traffic-secret object. Construction also
requires the ordered owner to remain at `ExpectEncryptedExtensions` with zero
records consumed, no partial message, and no failure, and requires the
continuation transcript, ordered record state, and retained server Finished
key to select the same suite. The continuation snapshot must still equal the
exact ServerHello snapshot retained by that shared retry proof. Success moves
the complete ordered record owner and reconstructed transcript into one
non-copyable wrapper while retaining the exact retry-derived server Finished-
key proof; callers cannot substitute a detached transcript, Finished key,
traffic-secret proof, record state, or resumed flight position.

This provenance join does not open a record, advance the transcript, expose a
mutable transcript, validate or authenticate a server-flight message, verify
CertificateVerify or Finished, validate certificates, establish trust,
enforce revocation, protect application data, perform I/O, authorize HTTP or
OAuth transport, or open a network connection. Authenticated message capture,
ordered transcript advancement, certificate and revocation policy, and all
remaining TLS/HTTP/audited transport composition remain prerequisites;
concrete HTTPS and live-provider readiness remain unavailable.

The joined retry owner can now authenticate one ciphertext record, enforce the
no-PSK server-flight transition for every completed message, and lend each
exact header-plus-body encoding with the transcript snapshot from immediately
before that message to one synchronous non-retaining handler. The handler must
explicitly accept the message before the joined owner appends that exact
encoding to its private transcript. This ordering lets later repository-owned
CertificateVerify and Finished processing use the required pre-message
snapshot without exposing mutable transcript state. Success returns
non-copyable record evidence retaining the joined owner and exact ciphertext
beside the post-record flight state, authenticated content length, completed
message count, partial-message status, client-authentication requirement, and
post-record transcript snapshot.

Record, content-type, reassembly, ordering, handler-rejection, transcript, or
handler-unwind failure clears buffered fragments and permanently closes the
joined record owner before a later record can be processed. A handler may have
accepted earlier messages in the same record before a later message fails, and
transcript-capacity failure occurs only after the handler accepts the affected
message, so caller-owned effects remain caller-audited and non-transactional.
The handler acceptance bit is not body validation, signature verification, or
Finished authentication. This boundary does not parse EncryptedExtensions,
CertificateRequest, Certificate, CertificateVerify, or Finished bodies; build
or validate a certificate path or hostname; enforce revocation; select later
epochs; protect application data; perform I/O; authorize HTTP or OAuth
transport; or open a network connection. Those repository-owned validation
and transport compositions remain prerequisites, and concrete HTTPS and
live-provider readiness remain unavailable.

Each pending ordered retry message now also retains sealed access to the exact
retry base-schedule evidence that supplied its traffic keys. For an
`EncryptedExtensions` message at the post-accept
`ExpectCertificateRequestOrCertificate` position, a repository-owned helper
decodes the exact authenticated body under caller-selected structural limits,
then applies the exact extension-context policy and final ClientHello extension
offer retained by that same retry evidence. Success returns non-copyable proof
retaining the pending message and structurally decoded, placement-authorized,
offer-bound `EncryptedExtensions`; a different type or flight position,
malformed body, unknown or forbidden extension placement, or unsolicited
response fails with a stable payload-free category. This proof is suitable as
the handler's reason to accept and hash that exact message without a detached
ClientHello, policy, message body, or transcript snapshot.

This validation proves only extension-vector structure, standard-table
placement, and type-level offer binding. It does not semantically decode or
negotiate any extension payload, select application protocols, parse later
handshake bodies, authenticate CertificateVerify or Finished, validate
certificates or hostnames, enforce revocation, select later epochs, perform
I/O, authorize HTTP or OAuth transport, or open a network connection. Those
repository-owned semantic, authentication, trust, revocation, and transport
compositions remain prerequisites; concrete HTTPS and live-provider readiness
remain unavailable.

An authenticated ordered retry `CertificateRequest` at the post-accept
`ExpectCertificate` position can now be decoded under explicit caller-selected
context, extension, and signature-scheme count limits. The helper requires the
ordered flight to have recorded a client-authentication request, requires the
request context to be empty for this main-handshake boundary, applies the exact
retained extension-context policy in the `CertificateRequest` context, requires
the `signature_algorithms` extension, and structurally decodes its nonempty
ordered scheme list. When `signature_algorithms_cert` is present, its list is
decoded under the same count limit. Success returns a non-copyable proof that
retains the exact pending message, decoded request, required handshake-signature
list, and optional certificate-signature list; no detached context, extension
block, algorithm list, policy, or transcript snapshot is accepted.

This proof establishes only authenticated message provenance, main-handshake
request-context emptiness, extension placement, required-list presence, and
signature-list framing. It does not select a signature algorithm, decide
whether client authentication is supported, access or choose credentials,
sign, emit a client Certificate or CertificateVerify, authenticate the server,
validate certificates or hostnames, enforce revocation, select later epochs,
perform I/O, authorize HTTP or OAuth transport, or open a network connection.
Those data-driven credential, signing, server-authentication, trust, revocation,
and transport compositions remain prerequisites, and concrete HTTPS and
live-provider readiness remain unavailable.

An authenticated ordered retry server `Certificate` at the post-accept
`ExpectCertificateVerify` position can now be decoded under explicit
caller-selected message and per-entry extension limits. The main-handshake
helper requires an empty request context and at least one certificate entry,
then structurally decodes every entry's exact length-prefixed extension vector
and applies the exact retained extension-context policy and final ClientHello
offer in the server `Certificate` context. The certificate framing primitive
therefore retains both each extension payload and its exact two-byte-length-
prefixed vector so this composition accepts no reconstructed or detached
extension block. Success returns a non-copyable proof retaining the pending
message and exact decoded nonempty server certificate chain after all entry
extensions pass placement, uniqueness, and offer binding.

This proof establishes only authenticated message provenance, server-certificate
framing, main-handshake empty context, nonempty chain presence, and per-entry
extension structure and type-level policy. It does not parse X.509 certificate
bytes, semantically decode certificate-entry extension payloads, construct or
validate a certificate path, authenticate CertificateVerify or Finished,
establish trust, verify a hostname, enforce revocation, select later epochs,
perform I/O, authorize HTTP or OAuth transport, or open a network connection.
Those repository-owned certificate, signature, trust, revocation, and transport
compositions remain prerequisites, and concrete HTTPS and live-provider
readiness remain unavailable.

The exact first entry of that authenticated ordered retry server `Certificate`
proof can now be bound to one caller-supplied certificate decoded by the
repository-owned X.509 primitive. The binding requires byte-for-byte equality
between the entry's retained `certificate_data` vector and the decoded
certificate's complete retained DER document. Success returns a non-copyable
proof that retains both the complete ordered server-certificate proof and the
exact decoded X.509 certificate; no detached, re-encoded, or merely equivalent
certificate can cross this boundary.

This proof establishes exact authenticated leaf-byte identity and
repository-owned structural decoding evidence only. It does not yet bind that
leaf to a reviewed path or requested destination, extract or authorize its
public key, authenticate CertificateVerify or Finished, establish trust,
verify a hostname, load a trust root, enforce revocation, select later epochs,
perform I/O, authorize HTTP or OAuth transport, or open a network connection.
Those repository-owned certificate-to-path, key, signature, hostname, trust,
revocation, and transport compositions remain prerequisites, and concrete
HTTPS and live-provider readiness remain unavailable.

A stronger vertical composition can retain that authenticated retry leaf proof
beside the complete HTTPS DNS destination, reviewed server path, and
repository-owned `CertificateVerify` verdict only when both evidence graphs
retain the same decoded leaf object. Byte equality alone is insufficient at
this boundary: exact object identity prevents an independently decoded leaf
from being substituted between the ordered record stream and the reviewed
destination/path/signature graph. The admitted authentication evidence type is
sealed by the repository, and the non-copyable result retains both complete
inputs.

This composition joins the authenticated Certificate bytes to the reviewed
destination, path-derived RSA key, and server signature verdict, but it does
not yet prove that the later ordered retry `CertificateVerify` callback used
that same verdict, authenticate Finished, bind record epochs, enforce
revocation freshness, perform I/O, authorize HTTP or OAuth transport, or open a
network connection. Those ordered-transition, Finished, revocation, record,
and transport compositions remain prerequisites, and concrete HTTPS and
live-provider readiness remain unavailable.

An authenticated ordered retry server `CertificateVerify` at the post-accept
`ExpectFinished` position can now be decoded from the exact pending message and
bound to the exact pre-message transcript snapshot. The helper decodes the
final ClientHello's retained `signature_algorithms` extension under an explicit
caller-owned scheme-count limit, requires the peer's exact wire identifier to
occur in that offer, and then authorizes it through the caller's data-driven
repository-owned signature-scheme policy. Success returns a non-copyable proof
retaining the pending message, exact decoded `CertificateVerify`, exact offered
scheme list, and policy-authorized RSA-PSS scheme. A detached transcript hash,
ClientHello offer, message body, wire identifier, or algorithm policy cannot be
substituted through this boundary.

This proof establishes authenticated message provenance, exact pre-message
transcript selection, `CertificateVerify` framing, final-offer membership, and
data-driven algorithm authorization only. In particular, the framing grammar
permits an empty signature vector, which later repository-owned cryptographic
verification must reject. This slice does not derive or bind the reviewed leaf
key, verify the signature, prove that the preceding certificate message and a
reviewed path retain the same leaf, authenticate Finished, establish trust,
verify a hostname, enforce revocation, select later epochs, perform I/O,
authorize HTTP or OAuth transport, or open a network connection. Those
repository-owned certificate, signature, trust, revocation, and transport
compositions remain prerequisites, and concrete HTTPS and live-provider
readiness remain unavailable.

The authenticated ordered retry `CertificateVerify` proof can additionally be
bound to one exact repository-owned RSA-PSS verification verdict. The
composition requires the verifier's role to be `server`, its selected-
transcript snapshot to equal the pending message's exact pre-message snapshot,
its policy-authorized scheme to equal the scheme retained from the final
ClientHello offer, and its complete decoded `CertificateVerify` message to
equal the authenticated ordered message. Success returns non-copyable evidence
retaining both complete proofs; callers cannot substitute a detached role,
snapshot, algorithm identifier, signature, key verdict, or message body.

This proof establishes repository-owned RSA-PSS authentication of the exact
ordered retry server message and transcript snapshot only. The verifier's RSA
key may still be caller supplied: this slice does not prove that it came from
the preceding authenticated `Certificate` message, a reviewed server path, the
requested DNS identity, or a reviewed trust root. It also does not authenticate
Finished, enforce revocation, select later epochs, perform I/O, authorize HTTP
or OAuth transport, or open a network connection. Those repository-owned
certificate-to-key, path, hostname, trust, revocation, and transport
compositions remain prerequisites, and concrete HTTPS and live-provider
readiness remain unavailable.

A bounded cross-boundary composition can now bind that exact authenticated
ordered retry `CertificateVerify` signature proof to sealed HTTPS destination,
reviewed server-path, path-derived RSA-key, and server-signature evidence. Both
graphs must retain the exact same repository-owned `CertificateVerify` verdict
object; message, signature, key, or transcript value equality is deliberately
insufficient. Success returns non-copyable evidence retaining both complete
inputs, so a caller-supplied key verdict cannot be substituted at the ordered
callback after the reviewed destination and path authenticated a different
verdict.

This composition proves that the ordered retry `CertificateVerify` callback
used the reviewed destination/path verdict, but it does not yet retain the
earlier authenticated retry `Certificate` leaf join in the same long-lived
ordered state. It also does not authenticate Finished, enforce revocation
freshness, select later record epochs, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. Those ordered-certificate, Finished,
revocation, record, and transport compositions remain prerequisites, and
concrete HTTPS and live-provider readiness remain unavailable.

A bounded ordered-continuity follow-up now carries the authenticated retry
server `Certificate` leaf join across callback lifetimes as a sealed
non-copyable receipt. The receipt is initialized only from the complete exact-
object leaf-to-reviewed-authentication proof and retains the reviewed
destination/path/signature evidence together with the exact single-use server
record-state issuer identity, decoded leaf identity, and `CertificateVerify`
verdict identity established by that proof. The authenticated ordered retry
issuer identity carries the original issuer borrow, so the receipt cannot
outlive that issuer and a later issuer cannot substitute an address reused
after deallocation. The authenticated ordered retry
`CertificateVerify` callback can complete the receipt only when it retains the
same reviewed-authentication object, the same single-use server record-state
issuer, and the same repository-owned signature-verdict object. The completed
receipt therefore records one ordered server-flight continuity chain from the
authenticated `Certificate` leaf through the destination/path-derived RSA-key
verdict used by the later `CertificateVerify` callback. A different ordered
record owner, byte-equal independent leaf decode, detached authentication
wrapper, or value-equal detached signature verdict fails closed.

This sealed receipt does not retain callback-scoped message bytes after their
synchronous authorization and does not replace their complete upstream proof
graphs; those graphs remain authoritative at each transition. It does not
decode another message, perform new signature or certificate validation,
authenticate or append Finished, establish Finished-key provenance, refresh
revocation state, select later record epochs, protect application data,
perform I/O, authorize HTTP or OAuth transport, or open a network connection.
Those Finished, revocation, record, and transport compositions remain
prerequisites, and concrete HTTPS and live-provider readiness remain
unavailable.

The repository-owned TLS 1.3 Finished primitive now retains the exact
successfully authenticated peer `verify_data` bytes in its non-copyable
`VerifiedFinished` evidence beside the selected suite, endpoint role, and
exact transcript hash. Retention is fixed-memory and length-selected only by
the reviewed cipher suite; the evidence exposes the exact public bytes for a
later ordered-message composition while Debug reports only their public
length. A failed length check or constant-time verification still returns no
evidence and retains no peer input.

This evidence does not itself decode or sequence a Finished handshake
message, prove that its bytes came from an authenticated ordered record,
establish Finished-key provenance, advance the transcript, select later
record epochs, protect application data, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. Those ordered-Finished, key-
provenance, record, and transport compositions remain prerequisites, and
concrete HTTPS and live-provider readiness remain unavailable.

An authenticated ordered retry server `Finished` at the post-accept
`Complete` position can now be verified against the exact transcript snapshot
captured immediately before that message. The verifier accepts the server
Finished key only through the complete retry-derived Finished-key proof,
requires that proof to retain the same exact retry base-schedule evidence as
the pending ordered message, and requires the selected suite and server sender
role to agree before repository-owned HMAC verification. Verification consumes
the exact authenticated message body, and non-copyable success evidence
retains the single-use ordered record-owner identity, retry-derived Finished-
key proof, pre-message transcript snapshot, optional client-authentication
requirement, and `VerifiedFinished` verdict whose retained peer bytes equal the
authenticated body. A detached retry proof, different record owner, wrong
ordered position, client key, different suite, malformed length, or invalid
verify-data fails with a stable payload-free category.

This exact ordered-Finished proof does not yet join the preceding long-lived
ordered `Certificate`-to-`CertificateVerify` continuity receipt, append the
Finished encoding to the transcript, derive application traffic secrets,
select later record epochs, enforce the requested client-authentication
response, protect application data, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. Those cross-callback continuity,
post-Finished key-schedule, record, and transport compositions remain
prerequisites, and concrete HTTPS and live-provider readiness remain
unavailable.

A bounded cross-callback composition can now consume the completed ordered
retry `Certificate`-to-`CertificateVerify` continuity receipt together with
the exact ordered retry server `Finished` proof. It requires both evidence
graphs to retain the same lifetime-bearing single-use server record-state
issuer identity before returning non-copyable completion evidence that owns
both complete inputs. Because the earlier receipt already binds the
authenticated Certificate leaf to the reviewed destination/path-derived RSA
signature verdict and the Finished proof already binds the exact pre-message
snapshot and authenticated body to the retry-derived server Finished key, the
result records one monotonic ordered server-flight continuity chain from
Certificate through CertificateVerify and Finished. A different ordered owner
fails closed even when its messages, retry secrets, certificates, signature
verdicts, transcript snapshot, or Finished bytes are value-equal.

This completed continuity evidence does not append Finished to a retained
transcript itself, derive application traffic secrets, select or install later
record epochs, enforce a requested client-authentication response, refresh
revocation state, protect application data, perform I/O, authorize HTTP or
OAuth transport, or open a network connection. Those post-Finished key-
schedule, client-authentication, revocation, record, and transport compositions
remain prerequisites, and concrete HTTPS and live-provider readiness remain
unavailable.

An allocation-free `tls13-application-traffic-secrets` prerequisite now
derives the client and server TLS 1.3 application traffic secrets from the
existing suite-selected Master Secret and one repository-owned transcript-hash
snapshot. Caller-owned fixed storage must retain the exact same data-driven
selected cipher suite as the base schedule, and the snapshot algorithm must
equal that suite's HKDF hash. Only then does the primitive import the existing
Master Secret into a fixed wipe-on-drop PRK, apply the protocol-fixed
`"c ap traffic"` and `"s ap traffic"` labels, stage both Hash-length results in
wipe-on-drop owners, and publish them to caller-owned wipe-on-drop storage only
after both derivations succeed. Success returns a non-copyable borrowed view;
stable failures retain no secret or transcript bytes.

This primitive accepts an already-computed snapshot and does not prove that it
is the exact transcript through the authenticated server Finished, append that
Finished encoding, bind either output to the completed ordered retry server
flight, or install later record epochs. It also does not derive traffic keys or
IVs, enforce a requested client-authentication response, refresh revocation
state, protect application data, perform I/O, authorize HTTP or OAuth
transport, or open a network connection. Exact post-Finished transcript and
ordered-flight binding, epoch installation, revocation, record, and transport
compositions remain prerequisites, and concrete HTTPS and live-provider
readiness remain unavailable.

The ordered retry record binder seals the post-server-Finished transcript
only when the exact authenticated Finished proof is joined to the record
that successfully appended that message's complete encoding. The record must
have completed Finished in this operation, end with no pending handshake
fragment, and retain the same single-use server record owner and exact
Finished-key proof. Completion of the public message order alone does not
establish this evidence. This preserves the Finished record boundary required
by RFC 8446 §5.1 and provides the transcript position needed by §7.1.

The join consumes both the successful record result and the verdict. The
non-copyable result retains the verified Finished proof and the exact
post-message snapshot. It accepts no caller-supplied digest or message bytes.
It does not join the reviewed Certificate-to-CertificateVerify chain, derive
application traffic secrets, install record keys, or authorize application
data, network I/O, HTTP, or OAuth. Concrete HTTPS and live-provider readiness
remain unavailable.

The reviewed server-authentication binder can consume that committed
Finished evidence with the completed ordered Certificate-to-CertificateVerify
receipt. Both inputs must retain the same lifetime-bearing single-use server
record issuer. The result owns both proof graphs, preserving the reviewed
authentication and exact transcript through the committed Finished record
boundary; neither a detached digest nor a callback-only Finished verdict can
construct it. A different issuer fails without returning either input.
This join does not derive application traffic secrets, install keys, refresh
revocation, satisfy client authentication, or authorize HTTP, OAuth, or I/O.

The reviewed committed Finished proof can be consumed to derive the initial
client and server application traffic secrets. The binder extracts the
Master Secret only from the retry schedule retained by the Finished-key proof
and supplies only the committed post-Finished snapshot to the existing
application traffic-secret primitive. It accepts no detached schedule,
digest, or suite. The caller's fixed storage must match the full selected
suite policy, including its record ceiling, before the primitive is invoked.
The non-copyable result owns the entire reviewed proof and borrows the two
derived secrets in caller-owned wipe-on-drop storage. These secrets are not
record keys and the result does not install epochs, send application data,
refresh revocation, satisfy client authentication, or enable HTTPS or OAuth.

An allocation-free `http1-fixed-request-body` prerequisite now binds one
caller-borrowed, potentially sensitive request body to its exact byte length
under an explicit caller ceiling. It encodes that length in canonical shortest
decimal form into caller-owned storage, including `0` for an empty body, only
after proving the complete output fits. The non-copyable evidence prevents a
different body from being substituted without revalidation, and a small output
remains unchanged. Stable failures retain no body, length, limit, or output
bytes, and Debug redacts the body.

The package can additionally consume that exact body evidence with one exact
validated request-header field. The resulting non-copyable evidence proves the
field name is `Content-Length` under HTTP ASCII case-insensitive comparison and
its value is the canonical shortest-decimal form of the retained body's exact
byte length. Success retains both original evidence objects and their borrowed
bytes without normalization or substitution. Non-`Content-Length` names and
noncanonical or mismatched values fail with stable payload-free errors; Debug
redacts both retained body and field bytes.

This package boundary does not create a `Content-Length` field line, authorize
the surrounding field set, encode form data, interpret or release credentials,
authorize a method or destination, compose a request, perform TLS or I/O, emit
provider or trace audit events, or grant HTTP, OAuth, or live-provider
transport. The reviewed `https-destination-name-binding` composition now
consumes this exact proof to establish one length field and no transfer coding
across its exact retained field set. That composition still does not authorize
or encode the fields, audit credential release or external effects, or write
the retained body.

An allocation-free `http1-request-line` prerequisite now validates and encodes
one deliberately narrow HTTP/1.0 or HTTP/1.1 request line into caller-owned
storage. Caller limits bound the method, origin-form target, and complete line;
methods must be non-empty HTTP tokens, targets must begin with `/`, contain only
the RFC 3986 path/query character classes, contain only complete percent
triplets, and exclude fragments. The encoder writes the exact validated method,
one space, target, one space, version, and CRLF only after proving the complete
output fits. Success retains borrowed method and target bytes plus the exact
encoded length without allocation. Stable failures retain no method, target,
length, or output bytes, and Debug redacts both borrowed inputs.

This syntax boundary does not authorize a method or target, validate URI or
percent-encoding semantics, encode headers or a body, select body framing,
interpret or release a credential, perform TLS, read or write a socket, emit
provider or trace audit events, or authorize HTTP or OAuth transport. The
reviewed `https-destination-name-binding` request-head composition now consumes
this exact syntax proof only after data-driven provider policy has classified
its exact method and query-free target as public. It combines that proof with
the exact fixed-body destination proof and encodes only the request line, Host,
Content-Length, and terminating empty line into caller-owned storage.
Potentially credential-bearing targets and additional fields remain behind a
later provider/trace-audited release boundary. The composition still supplies
no transport or external-effect authority. Concrete HTTPS remains unavailable.

An allocation-free `http1-response-head` prerequisite now decodes one strict
HTTP/1.0 or HTTP/1.1 response head from a caller-borrowed byte stream. Explicit
caller limits bound the complete head, every line, and the retained header
count before values are exposed. The parser requires CRLF line endings, an
exact three-digit status code from 100 through 599 and status-line separator,
token-only field names, no whitespace before a colon, no obsolete line
folding, and field values containing only the HTTP field-value octet profile.
Success retains the
exact encoded head, a borrowed reason phrase, ordered borrowed header fields,
and the unconsumed body bytes without allocation or normalization. Stable
errors retain no response bytes, header names or values, reason text, status,
lengths, or offsets; Debug output redacts header values and reason text.

This syntax boundary does not classify response-body framing, reconcile
duplicate fields, decode transfer codings, parse a body, authorize redirects,
bind a request method, perform TLS, read or write a socket, interpret or release
a credential, emit provider or trace audit events, or authorize OAuth transport.
Callers must apply those separate policies before any external effect or
credential release.
It does not retrofit or authorize the existing general-purpose `http1` or
`http1-client` packages; a later reviewed composition must select this strict
syntax boundary and add the remaining response and transport policies.
Concrete HTTPS remains unavailable.

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
