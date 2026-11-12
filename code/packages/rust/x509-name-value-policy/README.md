# x509-name-value-policy

`x509-name-value-policy` binds a bounded caller-supplied set of attribute-type
OIDs to repository-owned value comparison rules for already decoded RFC 5280
Names. It removes executable value callbacks from later certificate-link code
while keeping the attribute registry data driven.

The current rules are exact canonical DER element equality and exact Unicode-
scalar equality across supported DirectoryString encodings. Every attribute in
both Names is preflighted before structural comparison. An unconfigured type
fails closed. Under the DirectoryString-scalar rule, a malformed value or
TeletexString without a repository-owned T.61 mapping also fails closed with a
payload-free error. The exact-DER rule intentionally treats its complete
canonical value element as opaque and applies no DirectoryString schema.

## Security boundary

The package accepts at most 64 validated, unique OID rules, allocates nothing
from certificate input, and composes the fixed 32-RDN/64-attribute bounds from
`x509-name` with the at-most-4,096 candidate comparisons from
`x509-name-comparison`.

It does not implement RFC 4518 StringPrep, normalization, case folding,
insignificant-space handling, T.61 mapping, a built-in attribute registry, or
complete RFC 5280 Name equality. It does not select or authenticate issuers,
validate paths or directoryName constraints, verify signatures, check
revocation, perform TLS or HTTP, authorize OAuth transport, access credentials,
or produce external effects.

## Verification

```bash
bash BUILD
cargo clippy -p x509-name-value-policy --all-targets -- -D warnings
cargo doc -p x509-name-value-policy --no-deps
```
