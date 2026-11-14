# http1-fixed-request-body

Bounded repository-owned binding between one borrowed HTTP request body and its
canonical `Content-Length` field value.

The crate retains one caller-borrowed, potentially sensitive body under an
explicit byte ceiling. It writes the exact body length in shortest decimal form
into caller-owned storage without allocation. Empty bodies encode as `0`; an
undersized output remains unchanged. Stable errors retain no body, length,
limit, or output bytes, and Debug redacts the body.

The crate can also consume that body evidence together with one exact validated
request-header field. The resulting non-copyable evidence proves that the
field name is `Content-Length` under HTTP ASCII case-insensitive comparison and
that its value is the canonical shortest-decimal encoding of the retained
body's exact byte length. It retains both original evidence objects without
normalizing or replacing their borrowed bytes.

This package does not create a header field, authorize a surrounding field set,
reject an additional `Content-Length` or `Transfer-Encoding` field, encode
forms, interpret or release credentials, authorize methods or destinations,
compose a request, perform TLS or HTTP I/O, emit provider/trace audit events, or
make OAuth, HTTPS, or live-provider use ready.

## Validation

```bash
bash BUILD
cargo clippy -p http1-fixed-request-body --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p http1-fixed-request-body --no-deps
```
