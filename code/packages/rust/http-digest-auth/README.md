# http-digest-auth

Bounded RFC 7616 HTTP Digest authentication primitives for production HTTP
transports.

The package parses one `WWW-Authenticate: Digest ...` challenge and builds a
zeroizing `Authorization` value for `qop=auth` or the legacy no-`qop` form. It
supports MD5, MD5-sess, SHA-256, and SHA-256-sess. Callers own network I/O,
credential leasing, nonce-count state, retry limits, and CSPRNG-backed client
nonce generation.

The parser rejects oversized challenges, duplicate or malformed directives,
unsupported quality-of-protection modes, unsupported algorithms, header
injection, and `userhash=true`. Passwords and derived response material are
held only in zeroizing buffers and the returned authorization value does not
implement `Debug` or `Display`.

Protocol reference: [RFC 7616](https://www.rfc-editor.org/rfc/rfc7616)
