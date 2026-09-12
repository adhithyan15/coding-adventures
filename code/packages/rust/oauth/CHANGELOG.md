# Changelog

## Unreleased

- Exposed the validated optional revocation endpoint from `ProviderConfig` so
  broker composition can require an exact registered endpoint before any
  client-secret access or transport effect.
- Added bounded, request-bound RFC 7009 response classification: only exact
  HTTP 200 confirms revocation and its bounded body is ignored; HTTP 400/401
  errors are parsed into closed codes; HTTP 503 remains explicitly retryable;
  and provider/trace audit is durable before success or error release.
- Exposed the already-validated token endpoint as read-only provider policy so
  higher-level authentication adapters can retain its exact audience binding.
- Exposed an opaque exact-binding check between metadata-derived RFC 8628
  device profiles and registered public-client provider configurations.
- Added confidential `ProviderConfig` derivation for the stack's closed,
  implemented authentication-method set. The exact method must be advertised
  by RFC 8414 metadata, PKCE `S256` and mix-up defense remain mandatory, and
  public derivation still requires explicit `none`.
- Exposed the validated provider redirect URI and whether it relies on
  registry-wide distinct-redirect ownership so composition roots can enforce
  the mix-up-defense invariant without duplicating protocol state.
- Added audited RFC 8628 device-token response classification with one-use
  request/response session ownership, exact HTTP 400 polling-error handling,
  persistent five-second `slow_down` increases, normal token-response reuse,
  and caller-owned retry timing without sleep, clock, storage, or network
  authority.
- Added metadata-bound RFC 8628 public-client device authorization initiation:
  exact device endpoint, grant, and `none` authentication capability checks;
  audited zeroizing request and response ownership; strict bounded verification
  URI, lifetime, and interval validation; and audited device-code poll request
  preparation without acquiring sleep, clock, or network authority.
- Generalized RFC 8414 decoding to retain exact confidential-client token
  authentication capabilities even when `none` is absent, while both public
  `ProviderConfig` derivation paths still fail closed unless `none` was
  explicitly advertised.
- Retained RFC 8414 `token_endpoint_auth_signing_alg_values_supported` as
  immutable provider data, require it when JWT client authentication is
  advertised, and reject the unsecured `none` algorithm without inventing
  defaults.
- Retained and exposed each prepared request's exact non-secret client ID and
  trace for the audit-gated client-secret authentication adapter; revocation
  requests now retain the same caller trace as exchange and refresh requests.
- Replaced the generic JSON parser dependency with the repository-owned,
  zero-dependency `bounded-json` parser, retaining duplicate-field rejection,
  recursive scrubbing, and the existing 64-level OAuth nesting limit while
  removing all external normal dependencies from the OAuth tree.
- Exposed privacy-safe provider/trace bindings on decoded token responses and
  the decoder's response-size bound so provider-neutral brokers can validate
  orchestration boundaries before credential storage.
- Added an ownership-transferring `TokenCredentials::into_parts` handoff so an
  opaque custodian can ingest decoded credentials without plaintext clones.
- Exposed read-only provider, trace, and redirect-URI ceremony bindings on
  `AuthorizationRequest` so an authorized host can reject cross-ceremony or
  cross-provider browser release without exposing transaction secrets.
- Added provider-neutral RFC 8414 request preparation and bounded metadata
  decoding with exact issuer comparison, strict HTTPS endpoints, explicit
  Authorization Code, public-client `none`, PKCE `S256`, and RFC 9207 response
  issuer or registry-owned distinct-redirect negotiation,
  immutable capability retention, provider-config derivation, recursive JSON
  scrubbing, and mandatory audit publication before either request or response
  release.
- Added provider-neutral bounded JSON/form token response decoding, closed
  token-endpoint errors, explicit refresh-token rotation decisions, audited
  public-client refresh and RFC 7009 revocation request preparation, recursive
  response-tree scrubbing, and a separate audit gate before parsed credential
  material can leave the codec. Secret form encoders now write directly into a
  zeroizing destination without ordinary heap-string intermediates.
- Added the first provider-neutral OAuth 2.0 installed-app primitive: strict
  configuration, caller-injected 256-bit state and PKCE entropy, mandatory
  `S256`, authorization URL construction, exact callback/state/issuer
  validation, opaque token-exchange preparation, closed errors, redacted
  diagnostics, caller-owned trace correlation, and first-class privacy-safe
  audit descriptors whose durable publication is required before result
  release.
