# http1-request-host-authority

Bounded repository-owned syntax for one HTTP/1 request `Host` field value.

The crate validates a caller-borrowed ASCII DNS name and optional non-zero port
under explicit byte ceilings. DNS labels use the LDH profile and reject empty,
oversized, or hyphen-delimited labels. IPv4/IPv6 literals and legacy numeric
address spellings are rejected.
Encoding lowercases the name and appends
the supplied port in shortest decimal form into caller-owned storage without
allocation. The output is unchanged when it is too small. Stable errors retain
no hostname, port, length, or output bytes, and Debug redacts the hostname.

This package does not parse URLs, recognize scheme-default ports, encode the
surrounding header field, compare the authority with request, TLS, certificate,
or provider identities, perform DNS or I/O, release credentials, emit audits,
or make HTTP, OAuth, HTTPS, or live-provider use ready.

## Validation

```bash
bash BUILD
cargo clippy -p http1-request-host-authority --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p http1-request-host-authority --no-deps
```
