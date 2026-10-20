# x509-webpki-root-anchor-selection

An allocation-free repository-owned selector for the exact reviewed Mozilla
public-Web-PKI root snapshot.

The crate accepts one already decoded candidate certificate and scans the
complete pinned snapshot for exact canonical Name and SubjectPublicKeyInfo
sequence-content matches. It rejects absent and ambiguous matches and returns
non-copyable evidence binding the candidate to the selected entry, including
the entry's optional name constraints.

This is reviewed-set membership evidence only. It does not interpret name
constraints, validate certificate roles or critical extensions, verify a
signature, check time or revocation, construct or validate a path, match a
hostname, or provide TLS, HTTP, OAuth transport, or network authority. The
snapshot excludes OS, enterprise, user-installed, and local roots.

## Validation

```bash
bash BUILD
cargo clippy -p x509-webpki-root-anchor-selection --all-targets -- -D warnings
cargo doc -p x509-webpki-root-anchor-selection --no-deps
```
