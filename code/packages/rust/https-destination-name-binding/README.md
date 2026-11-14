# https-destination-name-binding

Repository-owned proof that a future HTTPS client uses one DNS identity across
three separate protocol boundaries:

- the exact decoded X.509 certificate's Subject Alternative Name;
- the TLS 1.3 ClientHello `server_name`; and
- the HTTP/1 request `Host` authority.

`bind_https_dns_destination` accepts values that the owning packages have
already syntax-checked. It requires SNI and Host to match case-insensitively,
then reruns complete-certificate SAN matching against the SNI name. The
non-copyable result retains the exact certificate object, SNI, Host authority,
and SAN evidence so later composition cannot accidentally substitute any one
of them. An explicit Host port is retained but does not participate in DNS
identity comparison.

The certificate reference parser creates at most one temporary normalized DNS
name bounded by the 253-byte SNI profile. Stable errors and Debug output retain
no hostname or certificate bytes.

`bind_https_host_header_field` then consumes that complete destination proof
together with one `http1-request-header-field` value. It accepts the field name
`Host` under HTTP's case-insensitive name semantics and requires the value to
be the retained authority's canonical encoding: lowercase DNS bytes plus an
optional colon and shortest-decimal port. The non-copyable result retains both
inputs, so a caller cannot replace the Host field with one for another
destination after binding.

`bind_https_request_header_fields` moves that proof together with the exact
caller-borrowed slice of additional validated fields into a second
non-copyable result. A caller-selected total-field ceiling includes Host and is
hard-capped at 64. Every additional field name is checked case-insensitively,
and any second Host field fails closed. The result retains the exact field
slice, caller order, and limit policy for a later encoder.

This does not authorize additional fields, reject duplicate names other than
Host, reconcile `Content-Length`, prohibit `Transfer-Encoding`, or compose a
request head. A later composition must consume this exact proof, bind the
request line and body framing, and preserve the destination through audited
transport.

This package does not parse URLs, bind request targets or provider
configuration, choose ports, validate critical extensions, certificate role,
purpose, algorithms, validity, signatures, paths, roots, constraints, or
revocation, compose a complete HTTP request, perform TLS or I/O, access or
release credentials, emit audits or traces, authorize effects, or grant HTTPS,
OAuth, or live-provider readiness.

## Validation

```bash
bash BUILD
cargo clippy -p https-destination-name-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p https-destination-name-binding --no-deps
```
