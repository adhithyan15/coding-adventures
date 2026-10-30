# tls13-certificate-message

Bounded, allocation-free decoding of one already-delimited RFC 8446 TLS 1.3
`Certificate` handshake body.

The decoder borrows the request context, complete X.509 certificate bytes, and
per-entry extension blocks. Callers set ceilings for total bytes, context
bytes, certificate-list bytes, entry count, certificate bytes, and extension
bytes. Empty client certificate lists and non-empty post-handshake request
contexts remain structural values for later handshake-state policy.

## Boundary

This package owns framing only. It does not interpret extension blocks, parse
X.509, decide client/server or handshake state, enforce extension placement,
authenticate a certificate, construct or validate a path, establish trust,
check names or revocation, send an alert, protect records, perform I/O, provide
TLS or HTTP, authorize OAuth transport, or open a network connection. It has no
external effect.

## Development

```bash
bash BUILD
```
