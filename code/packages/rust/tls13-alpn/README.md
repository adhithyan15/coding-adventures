# tls13-alpn

`tls13-alpn` is an allocation-free repository-owned implementation of the
RFC 7301 Application-Layer Protocol Negotiation extension-data framing used by
a future TLS 1.3 client.

It provides:

- caller-ordered client offers containing 1 through 64 opaque protocol names;
- exact two-byte list and one-byte name length framing;
- duplicate rejection and complete failure-atomic encoding;
- bounded exact decoding with a caller-selected protocol-count ceiling; and
- server selection validation that accepts exactly one originally offered
  protocol.

Protocol identifiers are opaque bytes. This package does not register or
enable HTTP/1.1, HTTP/2, or another application protocol. It does not locate
the extension in a handshake, authorize its message context, require ALPN,
sequence a handshake, validate certificates, protect records, perform I/O, or
provide TLS, HTTPS, HTTP, OAuth transport, or live-provider readiness.

## Development

```bash
bash BUILD
cargo clippy -p tls13-alpn --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p tls13-alpn --no-deps
```
