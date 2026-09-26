# der-asn1

Bounded typed ASN.1 DER value decoding with canonical BOOLEAN, INTEGER, BIT
STRING, OCTET STRING, IA5String, NULL, OBJECT IDENTIFIER, implicit primitive,
SEQUENCE, SET, and explicit-wrapper handling.

The package snapshots bounded input before exposing immutable typed values,
shares depth and total-element budgets across nested cursors, and reports only
stable payload-blind errors with local byte offsets. Its package-native suite
executes all 122 language-neutral DER ASN.1 v1 cases and all 46 referenced DER
TLV framing cases.

## Dependencies

- der-tlv

## Example

```ruby
decoder = CodingAdventures::DerAsn1::Decoder.new
element = decoder.decode_exact("\x02\x01\x2a".b)
integer = CodingAdventures::DerAsn1.decode_integer(element)
integer.to_u64 # => 42
```

This layer performs no filesystem, network, clock, entropy, credential, or
cryptographic work. X.509 schemas, trust, signatures, TLS, and PEM remain out
of scope.

Validated wrappers use a closure-held construction capability and reject raw,
subclassed, or reflectively invoked construction through the package API. As
with any pure Ruby library, a caller that already controls unrestricted
`ObjectSpace`, class reopening, or process-wide reflection is inside the
runtime trust boundary rather than an untrusted package consumer.

## Development

```bash
# Run tests
bash BUILD
```
