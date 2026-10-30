# http1-response-head

Strict, bounded, allocation-free HTTP/1 response-head decoding.

The package accepts HTTP/1.0 and HTTP/1.1 response heads with exact CRLF line
endings. Caller policy bounds the complete head, each line, and the retained
header count. Successful decoding borrows the exact head, reason phrase,
ordered field names and values, and unconsumed body tail without allocation.

The parser deliberately stops before duplicate-field reconciliation, response
body framing, transfer decoding, redirect policy, request-method binding, TLS,
socket I/O, or OAuth transport. Although the caller-borrowed response can
contain sensitive credential material, this package does not interpret or
release credentials and performs no provider action, audit, trace emission, or
external effect. It does not make concrete HTTPS or live-provider use ready.

This package does not retrofit or authorize the existing general-purpose
`http1` or `http1-client` packages. A later reviewed composition must choose
this strict syntax boundary and add the remaining response and transport
policies explicitly.

## Development

```bash
cargo test -p http1-response-head --all-targets
cargo clippy -p http1-response-head --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p http1-response-head --no-deps
```
