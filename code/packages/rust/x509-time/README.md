# x509-time

`x509-time` is the allocation-free RFC 5280 time-profile layer above the
repository-owned `der-asn1` and `der-tlv` primitives. It accepts an already
framed ASN.1 element, enforces exact UTCTime or GeneralizedTime wire shapes,
resolves the certificate-validity UTCTime century rule, and returns validated
Gregorian calendar fields.

```rust
use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_time::decode_x509_time;

let encoded = [
    0x17, 13, b'4', b'9', b'1', b'2', b'3', b'1', b'2', b'3', b'5', b'9',
    b'5', b'9', b'Z',
];
let mut decoder = Asn1Decoder::new(Asn1Limits::default());
let element = decoder.decode_exact(&encoded)?;
let time = decode_x509_time(element)?;

assert_eq!(time.year(), 2049);
# Ok::<(), Box<dyn std::error::Error>>(())
```

RFC 6960 OCSP timestamps always use the RFC 5280 GeneralizedTime wire form,
including for present-day years that a certificate `Validity` field must encode
as UTCTime. That separate syntax boundary is explicit:

```rust
use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_time::decode_x509_generalized_time;

let encoded = b"\x18\x0f20261024115654Z";
let mut decoder = Asn1Decoder::new(Asn1Limits::default());
let time = decode_x509_generalized_time(decoder.decode_exact(encoded)?)?;

assert_eq!(time.year(), 2026);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Security boundary

The package accepts only primitive universal tags 23 and 24, the fixed RFC
5280 Zulu forms with mandatory seconds, the fixed 1950/2050 century split, and
valid Gregorian fields. The dedicated GeneralizedTime entry point accepts only
primitive universal tag 24 and deliberately omits the certificate-specific
2050 cutoff so RFC 6960 timestamps can reuse the same validated value. It
allocates nothing from wire data and uses no locale, timezone database,
platform calendar, epoch conversion, or clock.

Successful decoding does not establish that a certificate is currently valid
or that an OCSP timestamp is authentic or fresh. This crate does not parse
certificate or OCSP schemas, evaluate validity or freshness intervals, validate
paths, verify signatures, select trust roots, match server identities, perform
TLS, or open a network connection.
