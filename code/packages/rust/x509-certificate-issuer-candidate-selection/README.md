# x509-certificate-issuer-candidate-selection

A bounded, allocation-free repository-owned primitive for selecting exactly one
candidate issuer for one decoded X.509 certificate.

The caller supplies an explicit candidate-count ceiling and either a generic
distinguished-name comparison callback or a validated `X509NameValuePolicy`.
The complete accepted pool is scanned in caller order, comparing the child's
issuer Name with each candidate's subject Name. Zero matches and multiple
matches fail closed. The policy-backed entry point also stops at the first
payload-free Name comparison failure. It returns a distinct non-copyable token
binding the exact child, selected candidate, candidate index, and validated
Name policy, so evidence minted by the generic callback cannot substitute for
policy-backed evidence.

The repository-owned policy rules are data-driven, but they do not implement
complete RFC 5280 Name equality: they provide no RFC 4518 normalization, case
or whitespace folding, schema-specific matching, or TeletexString mapping.
This is one path-construction input only. The crate does not establish that the
selected certificate is a CA, verify a signature, recognize a trust anchor,
validate a path or constraint, check time or revocation, perform TLS or HTTP,
or open a network connection.

## Validation

```bash
bash BUILD
cargo clippy -p x509-certificate-issuer-candidate-selection --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-certificate-issuer-candidate-selection --no-deps
```
