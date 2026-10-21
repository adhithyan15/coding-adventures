# x509-certificate-issuer-candidate-selection

A bounded, allocation-free repository-owned primitive for selecting exactly one
candidate issuer for one decoded X.509 certificate.

The caller supplies an explicit candidate-count ceiling and a distinguished-name
comparison policy. The complete accepted pool is scanned in caller order, and
the comparator is invoked exactly once per candidate with the child's issuer
Name followed by that candidate's subject Name. Zero matches and multiple
matches fail closed. Success returns non-copyable borrowed evidence binding the
exact child, selected candidate, and candidate index.

This is one path-construction input only. The crate embeds no Name normalization
or equality rule and does not establish that the selected certificate is a CA,
verify a signature, recognize a trust anchor, validate a path or constraint,
check time or revocation, perform TLS or HTTP, or open a network connection.

## Validation

```bash
bash BUILD
cargo clippy -p x509-certificate-issuer-candidate-selection --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-certificate-issuer-candidate-selection --no-deps
```
