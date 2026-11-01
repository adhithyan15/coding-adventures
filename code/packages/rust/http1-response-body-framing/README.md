# http1-response-body-framing

Strict, allocation-free HTTP/1 response body-framing policy.

The package classifies response headers from a strict CRLF response-head
decoder into no-body, fixed-length, or explicitly allowed close-delimited
framing. It rejects every
`Transfer-Encoding` until a separately reviewed transfer-coding decoder is
available, rejects duplicate or noncanonical `Content-Length`, applies an
explicit caller body limit, and handles HEAD, informational, 204, 304, and
successful CONNECT framing precedence without allocating. Higher-level
response semantics, including the prohibition on content in 205 responses,
remain separate.

Request semantics must be derived from the exact emitted request; response
bytes must never select HEAD or CONNECT handling. Close-delimited acceptance
is trusted caller policy and cannot be enabled by a response field.

This package does not retrofit or authorize the existing permissive `http1` or
`http1-client` packages. Later composition must use a separately reviewed
strict response-head decoder, preserve exact emitted-request binding, and then
apply this framing policy before any body consumption.

Caller-borrowed header values can contain sensitive material. Stable failures
retain no names, values, status, lengths, or offsets, and the package performs
no I/O, credential interpretation or release, provider action, audit, trace
emission, or external effect. It does not parse response syntax or body bytes,
decode transfer codings, read until connection close, perform TLS, or make
concrete HTTPS or live-provider use ready.

## Development

```bash
cargo test -p http1-response-body-framing --all-targets
cargo clippy -p http1-response-body-framing --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p http1-response-body-framing --no-deps
```
