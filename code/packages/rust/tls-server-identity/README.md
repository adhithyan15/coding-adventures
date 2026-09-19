# tls-server-identity

`tls-server-identity` is the small, pure matching boundary that answers one
question: does an already-decoded certificate `subjectAltName` identify the
server the caller intended to reach?

It implements the direct DNS-ID and IP-ID rules from RFC 9525 without external
dependencies. DNS names compare as case-insensitive ASCII labels. A wildcard
is accepted only as the complete left-most label and matches exactly one
reference label. Parsed IPv4 and IPv6 addresses compare exactly and never
cross-match textual DNS entries.

```rust
use std::net::IpAddr;
use tls_server_identity::{
    verify_server_identity, MatchKind, PresentedIdentities, ReferenceIdentity,
};

let reference = ReferenceIdentity::parse("api.example.test")?;
let dns_names = ["*.example.test"];
let ip_addresses: [IpAddr; 0] = [];
let verified = verify_server_identity(
    &reference,
    PresentedIdentities::new(&dns_names, &ip_addresses),
)?;
assert_eq!(verified.kind(), MatchKind::WildcardDns);
# Ok::<(), tls_server_identity::IdentityError>(())
```

## Security boundary

This crate intentionally cannot receive a certificate Common Name, raw DER,
or a socket. It does not parse certificates, validate a certificate chain,
choose trust roots, check revocation, perform IDNA mapping, or implement TLS.
A successful match proves only that one typed SAN matches one validated
reference identity. A later TLS stack must separately prove the certificate
path and supply SAN values from a strict repository-owned X.509 parser.

Inputs are bounded to 253-byte DNS names, 63-byte labels, and 256 total
presented identities. Invalid reference identities fail immediately. Invalid
presented DNS names are ignored so that another valid SAN can match; excessive
collections fail closed before matching. Errors contain only bounded counts
and categories, never raw presented names or certificate material.

Unicode U-labels are rejected. Callers must supply already-converted ASCII
A-labels if internationalized names are needed. The crate performs no ambient
I/O and requests no capabilities.
