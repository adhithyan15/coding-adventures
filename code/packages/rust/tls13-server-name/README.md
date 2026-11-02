# tls13-server-name

`tls13-server-name` is a small repository-owned boundary for the TLS
`server_name` extension used by a future TLS 1.3 client.

It validates one borrowed ASCII DNS hostname, rejects trailing root dots,
wildcards, malformed labels, and IP literals (including legacy or ambiguous
numeric IPv4 resolver spellings), then encodes the exact RFC 6066
ClientHello `ServerNameList` containing one `host_name`. It can decode that
same strict one-name profile and validates the empty acknowledgement carried by
TLS 1.3 EncryptedExtensions. Output capacity is checked before any mutation,
errors retain no hostname or wire bytes, and the crate has no dependencies.

The boundary receives already-delimited extension data. It does not locate the
extension, decide whether it is present in an authorized handshake message,
compare SNI with a certificate identity, parse or validate certificates,
establish trust, check revocation, perform DNS or network I/O, construct a
ClientHello, or provide TLS, HTTPS, HTTP, or OAuth transport readiness.

## Development

```bash
bash BUILD
```
