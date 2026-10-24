# x509-certificate-path-construction

A bounded, allocation-free repository-owned primitive for constructing one
deterministic chain of decoded X.509 certificates.

The caller supplies:

- a target certificate;
- a bounded candidate pool;
- an active path-certificate ceiling no larger than the crate's fixed capacity;
- distinguished-name policy for unique issuer selection; and
- a terminal-certificate predicate.

The constructor checks every bound before invoking either callback. It invokes
terminal policy exactly once per accepted path certificate and, for each
non-terminal step, reuses `x509-certificate-issuer-candidate-selection` to scan
the complete candidate pool. Zero or multiple issuer-name matches fail closed.
An exact complete-certificate DER repeat is a cycle even when it came from a
separate decode. A non-terminal certificate at the active depth limit also
fails before another selection.

Success returns non-copyable borrowed evidence retaining the exact target,
candidate slice, and ordered selected indices. A later validator can therefore
consume the constructed chain without accepting a caller-substituted pool.

This crate does not search ambiguous alternate branches, implement complete
RFC 5280 distinguished-name equality, decide that a terminal is trusted,
authenticate a link, or apply role, algorithm, validity, path-length,
name-constraint, hostname, or revocation policy. It performs no TLS, HTTP,
OAuth transport, filesystem, clock, credential, signing, or network effect.

## Validation

```bash
bash BUILD
cargo clippy -p x509-certificate-path-construction --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-certificate-path-construction --no-deps
```
