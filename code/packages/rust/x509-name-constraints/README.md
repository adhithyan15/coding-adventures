# x509-name-constraints

`x509-name-constraints` is the first bounded, allocation-free structural
decoder for the optional NameConstraints bytes retained by the reviewed Web
PKI root snapshot. Those bytes are the contents of the RFC 5280
`NameConstraints` SEQUENCE.

The current subset accepts one exact `permittedSubtrees [0]` field containing
one or more dNSName GeneralSubtree sequences. It retains arbitrary non-empty
ASCII subtree strings in wire order, including leading-dot forms such as
`.tr`, without hard-coding a reviewed root or DNS suffix. A separate bounded
evaluator validates the retained constraints and caller-supplied DNS names,
then requires every supplied name to match at least one permitted subtree.

## Security boundary

The package uses repository-owned `der-asn1` and `der-tlv` only. It shares the
caller's depth and total-element limits, stores borrowed names in a fixed-size
array, validates exact tags and canonical DER framing, and keeps input bytes
out of diagnostics. Unsupported top-level fields, GeneralName choices,
minimum/maximum distances, and trailing fields fail closed. Evaluation is
allocation-free, caps presented names at 256, validates all constraints before
matching, and uses strict ASCII label, leading-dot, and complete-leftmost-label
wildcard rules with case-insensitive label-boundary comparison.

Successful evaluation proves only that supplied strings satisfy the decoded
permitted DNS subtree set. It does not extract names from a certificate, bind
the result to exact certificate or selected-anchor evidence, evaluate excluded
or non-DNS subtrees, construct or validate a path, grant trust, check
revocation, verify a signature, perform TLS or HTTP, authorize OAuth transport,
or open a network connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-name-constraints --all-targets -- -D warnings
cargo doc -p x509-name-constraints --no-deps
```
