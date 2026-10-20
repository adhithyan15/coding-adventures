# x509-directory-string

`x509-directory-string` is the allocation-free RFC 5280 `DirectoryString`
value layer above the repository-owned `der-asn1` and `der-tlv` primitives. It
recognizes exactly UTF8String, PrintableString, TeletexString,
UniversalString, and BMPString; requires a primitive, non-empty value; and
validates each non-Teletex representation before exposing it.

UTF8String and PrintableString borrow validated text. UniversalString and
BMPString validate fixed-width Unicode scalar values and expose them through a
borrowed iterator. TeletexString is retained only as non-empty opaque bytes:
the package does not invent a T.61 character mapping.

Decoded values use an opaque public struct with a private representation, so
safe callers cannot construct malformed values that bypass these checks.

The scalar comparator can compare the four Unicode-capable representations
without allocation and refuses either Teletex operand. Its result is exact
Unicode-scalar equality only. It deliberately performs no RFC 4518 StringPrep,
case folding, normalization, insignificant-space processing, attribute-OID
schema selection, or complete RFC 5280 distinguished-name comparison.

## Security boundary

Errors contain only a stable category and local byte offset. The package
retains no input in errors, allocates nothing from wire data, reads no ambient
state, and uses no external dependency.

Successful decoding or scalar comparison does not select an issuer, validate
a certificate path, verify a signature, authorize a key or certificate role,
load roots, check revocation or hostnames, perform TLS, or open a connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-directory-string --all-targets -- -D warnings
cargo doc -p x509-directory-string --no-deps
```
