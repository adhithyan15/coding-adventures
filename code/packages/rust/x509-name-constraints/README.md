# x509-name-constraints

`x509-name-constraints` is the first bounded, allocation-free structural
decoder for the optional NameConstraints bytes retained by the reviewed Web
PKI root snapshot. Those bytes are the contents of the RFC 5280
`NameConstraints` SEQUENCE.

The current subset accepts one exact `permittedSubtrees [0]` field containing
one or more dNSName GeneralSubtree sequences. It retains arbitrary non-empty
ASCII subtree strings in wire order, including leading-dot forms such as
`.tr`, without hard-coding a reviewed root or DNS suffix.

## Security boundary

The package uses repository-owned `der-asn1` and `der-tlv` only. It shares the
caller's depth and total-element limits, stores borrowed names in a fixed-size
array, validates exact tags and canonical DER framing, and keeps input bytes
out of diagnostics. Unsupported top-level fields, GeneralName choices,
minimum/maximum distances, and trailing fields fail closed.

Successful decoding does not validate DNS syntax, match a certificate name,
evaluate permitted or excluded subtrees, bind bytes to reviewed-anchor
evidence, construct or validate a path, grant trust, check revocation, verify a
signature, perform TLS or HTTP, authorize OAuth transport, or open a network
connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-name-constraints --all-targets -- -D warnings
cargo doc -p x509-name-constraints --no-deps
```
