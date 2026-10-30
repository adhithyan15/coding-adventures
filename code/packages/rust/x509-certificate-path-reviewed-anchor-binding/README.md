# x509-certificate-path-reviewed-anchor-binding

A repository-owned evidence-composition primitive that binds the terminal
candidate of one constructed X.509 certificate path to reviewed Web PKI root
selection evidence.

The binder requires a nontrivial constructed path: construction must have
selected at least one issuer after the target. It derives the terminal from the
last selected candidate and requires that candidate to be the same decoded
certificate object retained by `SelectedReviewedWebPkiRootAnchor`. A byte-equal
certificate from a separate decode does not match. Success returns non-copyable
evidence retaining both complete inputs.

A stronger follow-up consumes that reviewed-terminal-bound path together with
certificate-wide role-policy evidence for the exact terminal object. It
rejects byte-equal re-decodes and requires the retained role to be issuing CA,
then keeps the complete critical-extension and role-policy evidence with the
reviewed path.

This crate does not authenticate path links or validate self-signature,
algorithms, validity, path length, name constraints, hostname, revocation, or
any other complete path policy. Issuing-CA role binding does not make a path
suitable for TLS, HTTP, OAuth transport, or live-provider use.
The crate performs no filesystem, clock, credential, signing, or network
effect.

## Validation

```bash
bash BUILD
cargo clippy -p x509-certificate-path-reviewed-anchor-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-certificate-path-reviewed-anchor-binding --no-deps
```
