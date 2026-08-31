# `coding_adventures_oauth`

Provider-neutral OAuth 2.0 client primitives implemented in this repository.
The first slice owns the security-sensitive, pure portion of an installed-app
Authorization Code flow:

- strict provider/end-point configuration;
- 256-bit caller-injected state and PKCE entropy;
- mandatory PKCE `S256` authorization requests;
- deterministic RFC 3986 form encoding;
- exact redirect, state, and optional authorization-server issuer validation;
- closed provider-denial and callback errors;
- one-use callback completion into an opaque token-exchange request; and
- caller-owned trace correlation plus privacy-safe audit descriptors for begin
  and completion attempts. `Audited::publish_then_release` is the only result
  release path, so audit publication failure fails closed before a browser URL,
  callback error, authorization code, or exchange request becomes observable.

The crate performs no network, browser, listener, clock, storage, or credential
I/O. Those authorities are deliberately injected by later broker and host
packages. Provider differences are data in `ProviderConfig`; the core contains
no Google, Microsoft, GitHub, Dropbox, or other provider branch.

This is the first implementation slice of `code/specs/oauth.md`. Refresh,
device authorization, token parsing/custody, loopback hosting, discovery,
DPoP, and a production HTTPS transport remain separately testable backlog
items rather than hidden fallbacks.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_oauth --all-targets -- -D warnings
cargo doc -p coding_adventures_oauth --no-deps
```
