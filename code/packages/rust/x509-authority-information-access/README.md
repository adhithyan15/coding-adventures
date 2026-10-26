# x509-authority-information-access

`x509-authority-information-access` is the allocation-free RFC 5280 Authority
Information Access (AIA) structure layer above the repository-owned
`x509-extension` and `der-asn1` primitives. It recognizes extension OID
`1.3.6.1.5.5.7.1.1`, requires the RFC 5280 non-critical form, and decodes a
non-empty ordered sequence of access-method OIDs paired with URI
`GeneralName` locations.

The package retains arbitrary canonical method OIDs, so later policy remains
data-driven. It publishes the standard `id-ad-ocsp` and `id-ad-caIssuers`
identifiers and provides exact-method iteration without assigning either one
authority. Unsupported non-URI `GeneralName` forms fail closed in this bounded
profile.

## Example

```rust
use der_asn1::{Asn1Decoder, Asn1Limits};
use x509_authority_information_access::{
    decode_authority_information_access, ID_AD_OCSP_OID,
};
use x509_extension::decode_x509_extension;

# fn inspect(encoded_extension: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
let mut decoder = Asn1Decoder::new(Asn1Limits::default());
let root = decoder.decode_exact(encoded_extension)?;
let extension = decode_x509_extension(&mut decoder, root)?;
let access = decode_authority_information_access(&mut decoder, extension)?;

for uri in access.locations_for(ID_AD_OCSP_OID) {
    // A later policy must validate and authorize this locator before use.
    println!("candidate OCSP locator: {uri}");
}
# Ok(())
# }
```

## Security boundary

The decoder shares the caller's DER depth, total-element, value-length, and
OID-arc budgets; retains at most 32 borrowed descriptions; and exposes only
stable payload-free errors. Public values can only be constructed by checked
decoding.

Success proves only the supported AIA structure. It does not validate RFC 3986
URI syntax, authorize a scheme, host, port, or path, choose a responder, fetch
or cache data, authenticate revocation evidence, read a clock, validate a
certificate path, establish trust, perform TLS or HTTP, authorize OAuth
transport, open a network connection, or access a credential.

## Verification

```bash
bash BUILD
cargo clippy -p x509-authority-information-access --all-targets -- -D warnings
cargo doc -p x509-authority-information-access --no-deps
```
