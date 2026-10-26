# der-asn1

`der-asn1` is the repository-owned, allocation-free typed value layer above
`der-tlv`. It validates the small canonical ASN.1 DER vocabulary needed by a
future X.509 parser while keeping all decoded bytes borrowed from the caller.

```rust
use der_asn1::{decode_integer, Asn1Decoder, Asn1Limits};

let encoded = [0x30, 0x03, 0x02, 0x01, 0x2a];
let mut decoder = Asn1Decoder::new(Asn1Limits::default());
let root = decoder.decode_exact(&encoded)?;
let mut sequence = decoder.sequence(root)?;
let integer = sequence.read(&mut decoder)?.expect("one child");

assert_eq!(decode_integer(integer)?.to_u64()?, 42);
assert!(sequence.read(&mut decoder)?.is_none());
# Ok::<(), der_asn1::Asn1Error>(())
```

## Security boundary

The decoder applies one shared constructed-depth and total-element budget to a
schema walk. Typed helpers require exact class, constructed bit, and tag, then
reject alternate BER value encodings for BOOLEAN, INTEGER, BIT STRING, NULL,
ENUMERATED, and OBJECT IDENTIFIER. INTEGER and ENUMERATED retain their distinct
universal tags and result types while sharing canonical two's-complement
content validation. OCTET STRING contents remain borrowed and IA5String
contents must be ASCII. Schema-selected implicit NULL, BIT STRING, OCTET STRING,
IA5String, and OBJECT IDENTIFIER values require an exact primitive
context-specific tag while reusing the universal value checks. The implicit
NULL helper also requires empty contents; the caller remains responsible for
selecting the schema tag and interpreting the surrounding choice. SEQUENCE, SET,
schema-selected context-specific IMPLICIT constructed values, and explicit
context wrappers can be opened without recursion or allocation. Every
constructed cursor shares the same depth and total-element budget. Errors
contain only a category and local byte offset. A schema can frame an exact DER
document embedded inside an existing element while deriving its root depth
from that container and retaining the decoder's total-element count. A checked
element can also yield a compact opaque container context for schemas that
must retain this authority without retaining the complete framing element;
context creation fails closed if the element depth cannot fit its fixed-width
representation. The schema remains responsible for binding the embedded input
to the container.

This crate does not parse X.509 fields, time or string semantics, PEM, paths,
signatures, trust roots, TLS, or network traffic. Successful decoding proves
only canonical framing and the typed value checks invoked by the caller.
In particular, ENUMERATED decoding does not define a value registry, and
implicit NULL decoding does not establish an OCSP status. Neither grants CRL,
OCSP, revocation, authentication, or freshness semantics.

The package-native conformance test executes the 122-case language-neutral
DER ASN.1 v1 corpus. Its framing tranche resolves and executes the referenced
DER TLV cases instead of treating the `upstream` marker as automatic success.
