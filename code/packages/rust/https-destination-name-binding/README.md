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

This package does not parse URLs, bind request targets or provider
configuration, choose ports, validate critical extensions, certificate role,
purpose, algorithms, validity, signatures, paths, roots, constraints, or
revocation, build a TLS or HTTP message, perform I/O, access credentials, emit
audits or traces, authorize effects, or grant HTTPS, OAuth, or live-provider
readiness.

## Validation

```bash
bash BUILD
cargo clippy -p https-destination-name-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p https-destination-name-binding --no-deps
```
