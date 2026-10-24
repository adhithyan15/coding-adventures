# x509-ocsp-cert-status

`x509-ocsp-cert-status` is the allocation-free structural RFC 6960
`CertStatus` layer above the repository-owned DER and X.509 time primitives.
It decodes exact `good`, `revoked`, and `unknown` choices. A revoked value
retains one validated, clock-free OCSP GeneralizedTime and an optional typed
RFC 5280 CRL reason.

## Security boundary

The package shares the caller's DER depth, element, and length limits, retains
no response bytes in errors, allocates nothing from wire data, and denies
unsafe code. It has no ambient authority or external dependency.

Successful decoding proves only that one value has the RFC 6960 `CertStatus`
shape. It does not parse a `SingleResponse` or larger OCSP response,
authenticate a response signature, authorize a responder, bind status to a
`CertID`, certificate, or path, decide freshness, fetch or cache data, read a
clock, establish trust, perform TLS or HTTP, authorize OAuth transport, or
access a network.

## Example

```rust
use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_ocsp_cert_status::{decode_x509_ocsp_cert_status, X509OcspCertStatus};

let encoded = [0x80, 0x00]; // good [0] IMPLICIT NULL
let mut decoder = Asn1Decoder::new(Asn1Limits::default());
let element = decoder.decode_exact(&encoded)?;
assert_eq!(
    decode_x509_ocsp_cert_status(&mut decoder, element)?,
    X509OcspCertStatus::Good
);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Verification

```bash
bash BUILD
cargo clippy -p x509-ocsp-cert-status --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ocsp-cert-status --no-deps
```
