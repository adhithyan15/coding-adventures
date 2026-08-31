# `coding_adventures_oauth_installed_app_host`

Audit-first, provider-neutral host authority for OAuth native-app loopback
redirects. It binds only the literal IPv4 or IPv6 loopback interface, injects
the external-user-agent opener, accepts one bounded HTTP/1.1 callback, and
returns the callback URI in wipe-on-drop storage for the pure
`coding_adventures_oauth` core to validate.

Every effect is bracketed by privacy-safe durable audit events using the exact
provider and caller trace. An audit failure before an attempt prevents the
socket or browser effect; an audit failure after an effect drops the listener
and withholds the result. Request targets, authorization URLs, callback bytes,
codes, state, issuers, headers, and OS diagnostics never enter audit records or
errors.

The listener is consumed by the first accepted connection, including malformed
or hostile local traffic. Strict request-line, header-count, line-size,
aggregate-size, Host, no-body, path, peer, and timeout bounds fail closed. PKCE,
state, redirect, and issuer validation remain in the OAuth core.

The crate contains no provider branches and no OS-specific browser command.
Desktop and CLI composition roots implement the tiny `ExternalUserAgent` trait
with their authorized platform launcher.

## Verification

```bash
bash BUILD
cargo clippy -p coding_adventures_oauth_installed_app_host --all-targets -- -D warnings
cargo doc -p coding_adventures_oauth_installed_app_host --no-deps
```
