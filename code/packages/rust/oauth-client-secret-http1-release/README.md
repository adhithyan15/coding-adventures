# oauth-client-secret-http1-release

This crate releases one exact, complete HTTP/1 request from the repository's
existing OAuth client-secret custody material and HTTPS destination evidence.
It binds the request's own provider and trace to:

- one validated request line and data-driven method policy;
- one existing certificate/SNI/canonical-Host destination proof;
- the exact custody endpoint, form media type, canonical Content-Length,
  optional Basic Authorization field, and fixed body; and
- one caller-selected complete-request byte ceiling.

The wire order is fixed: request line, Host, Content-Length, Content-Type,
optional Authorization, empty line, and body. The endpoint must be the exact
lowercase `https://` origin from the canonical Host field followed by the exact
origin-form target.

An attempted provider/trace event publishes before this call reads any
credential-bearing body or Authorization bytes. The entire request is then
prepared in repository-owned wipe-on-drop memory. A terminal success event
publishes only after preparation, and success moves that prepared allocation
into the returned non-copyable proof without another credential read or copy.
Failure returns no request; any prepared storage wipes on drop. Debug and
stable errors redact all identity and request payloads.

Borrowing the prepared wire bytes requires a new provider/trace audit event on
every call. Audit failure withholds the slice. The owner still independently
holds the earlier custody-built request, so this guarantee covers accesses
introduced by this package rather than revoking that prior authority.

This is not an HTTP client. It performs no certificate role, purpose,
algorithm, validity, signature, path, trust-root, constraint, or revocation
validation beyond the evidence already retained by its input. It performs no
DNS, socket, TLS, HTTP I/O, response handling, credential acquisition or edit,
signing, or transport effect and does not grant HTTPS, OAuth, or live-provider
readiness.

## Validation

```bash
bash BUILD
cargo clippy -p oauth-client-secret-http1-release --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p oauth-client-secret-http1-release --no-deps
```
