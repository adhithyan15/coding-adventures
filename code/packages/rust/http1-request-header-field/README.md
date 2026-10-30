# http1-request-header-field

Bounded repository-owned syntax for one HTTP/1 request-header field line.

The crate validates a token-only caller-borrowed field name and a strict ASCII
field value under explicit byte ceilings, then writes the exact field line into
caller-owned storage without allocation. Values may be empty or contain
internal space and horizontal tab, but outer optional whitespace, controls,
DEL, and non-ASCII bytes fail closed. The output is unchanged when it is too
small. Stable failures retain no field bytes, and Debug redacts both inputs.

This package does not authorize or interpret fields, release credentials,
detect duplicates, enforce Host or body-framing policy, compose a request head,
emit audit or trace events, perform TLS or HTTP I/O, open sockets, or make OAuth
HTTPS/live-provider use ready.

## Validation

```bash
bash BUILD
cargo clippy -p http1-request-header-field --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p http1-request-header-field --no-deps
```
