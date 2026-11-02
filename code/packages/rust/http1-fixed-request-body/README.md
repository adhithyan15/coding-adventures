# http1-fixed-request-body

Bounded repository-owned binding between one borrowed HTTP request body and its
canonical `Content-Length` field value.

The crate retains one caller-borrowed, potentially sensitive body under an
explicit byte ceiling. It writes the exact body length in shortest decimal form
into caller-owned storage without allocation. Empty bodies encode as `0`; an
undersized output remains unchanged. Stable errors retain no body, length,
limit, or output bytes, and Debug redacts the body.

This package does not create a header field, reconcile duplicate or conflicting
lengths, admit transfer coding, encode forms, interpret or release credentials,
authorize methods or destinations, compose a request, perform TLS or HTTP I/O,
emit provider/trace audit events, or make OAuth, HTTPS, or live-provider use
ready.

## Validation

```bash
bash BUILD
cargo clippy -p http1-fixed-request-body --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p http1-fixed-request-body --no-deps
```
