# http1-request-line

Bounded repository-owned syntax for one HTTP/1.0 or HTTP/1.1 request line.

The crate validates a caller-borrowed HTTP token method and an RFC 9112
origin-form target under explicit byte ceilings. Path and query bytes use the
RFC 3986 character classes, and every percent escape must contain exactly two
hexadecimal digits. The crate then writes the exact request line into caller-
owned storage without allocation. The output is not modified when it is too
small. Stable failures retain no method, target, or output bytes, and Debug
redacts both borrowed inputs.

This package does not authorize methods or destinations, parse URI semantics,
encode headers or bodies, select framing, access or release credentials, emit
audit or trace events, perform TLS or HTTP I/O, open sockets, or make OAuth
HTTPS/live-provider use ready.

## Validation

```bash
bash BUILD
cargo clippy -p http1-request-line --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p http1-request-line --no-deps
```
