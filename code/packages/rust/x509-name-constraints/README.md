# x509-name-constraints

`x509-name-constraints` is a bounded, allocation-free structural decoder for
RFC 5280 NameConstraints. It accepts either the optional SEQUENCE contents
retained by the reviewed Web PKI root snapshot or one exact complete DER
SEQUENCE carried by a certificate extension. The public
`NAME_CONSTRAINTS_OID` identifies `id-ce-nameConstraints` (`2.5.29.30`).

The complete certificate-extension subset accepts non-empty
`permittedSubtrees [0]`, `excludedSubtrees [1]`, or both in schema order. Each
field contains dNSName GeneralSubtree sequences with absent distance fields.
The reviewed-root snapshot contents decoder remains deliberately restricted to
one exact permitted field. The crate retains arbitrary non-empty ASCII subtree
strings in wire order, including leading-dot forms such as `.tr`, without
hard-coding a reviewed root or DNS suffix. A separate bounded evaluator
validates every retained constraint and caller-supplied DNS name, rejects any
name within an excluded subtree, and rejects a presented wildcard if any name
it can represent falls within an excluded subtree. It then requires each
remaining name, including every expansion of a wildcard, to stay within a
permitted subtree when a permitted set is present.

## Security boundary

The package uses repository-owned `der-asn1` and `der-tlv` only. It shares the
caller's depth and total-element limits, stores borrowed names in a fixed-size
array, validates exact tags and canonical DER framing, and keeps input bytes
out of diagnostics. Unknown, duplicate, reversed, or empty top-level fields,
unsupported GeneralName choices, minimum/maximum distances, and trailing
fields fail closed. Each constraint class and the presented-name collection
are capped at 256. Evaluation is allocation-free, validates all constraints
before matching, gives exclusion precedence, distinguishes wildcard overlap
from wildcard containment, and uses strict ASCII label, leading-dot, and
complete-leftmost-label wildcard rules with case-insensitive label-boundary
comparison.

Successful decoding or evaluation proves only that supplied strings satisfy
the decoded supported DNS subtree sets. It does not extract names from a
certificate, bind the result to exact certificate or selected-anchor evidence,
evaluate non-DNS subtrees, construct or validate a path, grant trust, check
revocation, verify a signature, perform TLS or HTTP, authorize OAuth transport,
or open a network connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-name-constraints --all-targets -- -D warnings
cargo doc -p x509-name-constraints --no-deps
```
