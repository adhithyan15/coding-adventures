# OAuth

**Status:** Phase 2 provider-neutral broker boundary implemented —
installed-app Authorization Code + PKCE, RFC 8628 device-flow initiation and
caller-driven polling classification and sequencing,
token/error codecs, refresh rotation, revocation request preparation, RFC 8414
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
  expiry, and transaction nonce before exposing `sub`.

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
   semantics, and RFC 7009 revocation preparation. Token or raw provider
   response bytes never enter errors or audit data, and both decoded material
   and secret-bearing requests are audit-gated before release.
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
   separate adapter.
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
   add no sleep or concrete network authority.
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

Before the concrete HTTPS adapter, replace `tls-platform`'s `rustls` and
`webpki-roots` dependency chain with repository-owned TLS, certificate parsing,
path validation, hostname verification, and a reviewed trust-root source. This
is prerequisite work, not permission to weaken HTTPS or certificate checks.

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
8. **Revocation.** Calls revocation endpoint with correct params; tolerates 404 (already revoked).

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
