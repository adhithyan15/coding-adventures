# x509-time

`x509-time` is the allocation-free RFC 5280 time-profile layer above the
repository-owned `der-asn1` and `der-tlv` primitives. It accepts an already
framed ASN.1 element, enforces the exact certificate UTCTime or GeneralizedTime
wire shape, resolves the fixed UTCTime century rule, and returns validated
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

## Security boundary

The package accepts only primitive universal tags 23 and 24, the fixed RFC
5280 Zulu forms with mandatory seconds, the fixed 1950/2050 century split, and
valid Gregorian fields. It allocates nothing from wire data and uses no locale,
timezone database, platform calendar, epoch conversion, or clock.

Successful decoding does not establish that a certificate is currently valid.
This crate does not parse certificate schemas, evaluate validity intervals,
validate paths, verify signatures, select trust roots, match server identities,
perform TLS, or open a network connection.
