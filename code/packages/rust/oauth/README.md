# `coding_adventures_oauth`

Provider-neutral OAuth 2.0 client primitives implemented in this repository.
The crate owns the security-sensitive, pure portion of installed-app OAuth:

- strict provider/end-point configuration;
- bounded RFC 8414 metadata discovery with exact issuer trust binding,
  confidential authentication-capability retention, public-client `none`
  enforcement, and confidential configuration derivation limited to exact
  advertised `client_secret_basic`, `client_secret_post`, or `private_key_jwt`;
  both derivation paths require explicit RFC 9207 or registry-owned
  distinct-redirect mix-up defense, including fail-closed JWT
  client-authentication signing algorithms and validated optional RFC 8628
  device authorization endpoints;
- metadata-bound RFC 8628 public-client initiation, bounded response decoding,
  zeroizing device and user codes, audited one-use device-code poll contexts,
  and exact `authorization_pending`, `slow_down`, authorization, denial, and
  expiry classification with caller-owned timing;
- 256-bit caller-injected state and PKCE entropy;
- mandatory PKCE `S256` authorization requests;
- deterministic RFC 3986 form encoding;
- exact redirect, state, and optional authorization-server issuer validation;
- closed provider-denial and callback errors;
- one-use callback completion into an opaque token-exchange request; and
- bounded JSON and form token/error decoding, explicit refresh-token rotation,
  public-client refresh grants, and RFC 7009 revocation preparation plus
  request-bound response classification. Only exact HTTP 200 confirms
  revocation; HTTP 503 remains a closed retryable outcome, while response
  handling acquires no retry timing, transport, or credential-removal authority.

Caller-owned trace correlation and privacy-safe audit descriptors cover every
implemented boundary. `Audited::publish_then_release` is the only result
release path, so audit failure closes before a browser URL, secret-bearing
request, metadata URL, validated provider record, provider error, parsed
response, or credential material is exposed.
Raw response bytes, attacker descriptions, URLs, scopes, and credentials never
enter audit records or diagnostics. Parsed JSON trees and request/response
buffers containing credentials are wipe-on-drop or explicitly scrubbed; secret
form bodies are zeroizing from their first byte and create no encoded temporary
strings.

Authorization-server JSON is parsed by the sibling zero-dependency
`coding_adventures_bounded_json` package. The OAuth normal dependency tree is
therefore entirely repository-owned; it does not reach `serde_json`, the
third-party `regex` crate, or another registry library.

The crate performs no network, browser, listener, sleep, clock, storage, or
credential I/O. The sibling `coding_adventures_oauth_installed_app_host`
package owns the separately audited literal-loopback and injected-browser boundary, while
`coding_adventures_oauth_credential_custody` owns audited secret lifecycle over
an injected atomic store. Later broker and transport packages will inject the
remaining authorities. Provider differences are data in `ProviderConfig`; the
core contains no Google, Microsoft, GitHub, Dropbox, or other provider branch.

This implements the pure authorization, device-flow, token-lifecycle, and RFC
8414 metadata trust slices of `code/specs/oauth.md`. Device-flow clock/sleep and
broker orchestration, DPoP, and production HTTPS transport remain separately
testable backlog items.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_oauth --all-targets -- -D warnings
cargo doc -p coding_adventures_oauth --no-deps
```
