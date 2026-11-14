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

`bind_https_fixed_request_body` then consumes that exact field-set proof and
one exact `http1-fixed-request-body` proof that already binds a body to its
canonical `Content-Length` field. The length field counts inside the retained
total-field ceiling. Every additional field name is checked case-insensitively;
another `Content-Length` or any `Transfer-Encoding` fails closed. The result
defines logical field order as Host, Content-Length, then the retained
additional fields in caller order, and retains both complete input proofs.

`bind_https_public_request_line` first requires data-driven provider policy to
classify one exact validated `http1-request-line` method and query-free target
as public request metadata. The resulting non-copyable proof is then consumed
by `bind_https_fixed_request_head` with the complete fixed-body destination
proof. Additional fields fail closed because their potentially credential-
bearing bytes require a later provider/trace-audited release composition. The
allocation-free encoder delegates each segment to the repository-owned request-
line and field encoders, writing only the public request line, Host, Content-
Length, and terminating CRLF into caller-owned storage after a complete
capacity preflight. The exact body remains retained but is not written.

This does not authorize transport, release an additional field, write the body,
or preserve the destination through audited transport. Secret-bearing targets
and fields require separate provider/trace-audited proofs. Those later policies
and effects must consume this exact proof.

This package does not parse URLs or provider documents, choose ports, validate
critical extensions, certificate role, purpose, algorithms, validity,
signatures, paths, roots, constraints, or revocation, compose a complete HTTP
request including its body, perform TLS or I/O, access or release credentials,
emit audits or traces, authorize effects, or grant HTTPS, OAuth, or live-
provider readiness.

## Validation

```bash
bash BUILD
cargo clippy -p https-destination-name-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p https-destination-name-binding --no-deps
```
