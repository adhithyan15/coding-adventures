# x509-name-comparison

`x509-name-comparison` is the allocation-free structural comparison layer for
already decoded RFC 5280 distinguished names. It compares RDNs in sequence
order, treats the attributes within each RDN as a set, requires exact canonical
attribute-type OIDs, and delegates attribute-value equivalence to caller
policy.

```rust
use x509_name_comparison::compare_x509_names_by;

let equal = compare_x509_names_by(&left, &right, |left, right| {
    left.value().encoded() == right.value().encoded()
});
```

That example intentionally chooses strict DER value equality. The package does
not represent that choice as complete RFC 5280 semantic name equality. A later
policy can instead decode and normalize permitted DirectoryString encodings or
apply schema-specific rules.

## Security boundary

The package allocates nothing, uses the `x509-name` bounds of at most 32 RDNs
and 64 total attributes, considers at most 4,096 candidate attribute pairs,
and retains no input. The caller's value callback must be a stable equivalence
relation and may run more than once when an RDN repeats an attribute type.

It does not decode DirectoryString values, normalize Unicode, select issuers,
construct or validate certificate paths, verify signatures, load trust roots,
process revocation, perform TLS, or open a network connection.

## Verification

```bash
bash BUILD
cargo clippy -p x509-name-comparison --all-targets -- -D warnings
cargo doc -p x509-name-comparison --no-deps
```
