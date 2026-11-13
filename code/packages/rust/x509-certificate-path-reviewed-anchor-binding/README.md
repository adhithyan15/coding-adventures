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

`bind_name_policy_constructed_x509_certificate_path_to_reviewed_anchor`
provides a distinct boundary for policy-backed construction. It consumes the
complete `ConstructedX509CertificatePathWithNamePolicy`, applies the same
nontrivial-path and exact-object terminal checks, and retains the exact
Name-value policy with the reviewed anchor. Generic callback construction
cannot produce this stronger result, and callers cannot detach the policy while
keeping its reviewed-terminal binding.

A distinct policy-backed role follow-up consumes that stronger reviewed path
together with certificate-wide role-policy evidence for the exact terminal
object. It rejects byte-equal re-decodes and end-entity role evidence, then
retains the complete Name-policy-backed construction, reviewed-root selection,
and issuing-CA policy together. Generic reviewed-terminal evidence cannot
produce this result.

`initialize_name_policy_reviewed_constructed_x509_certificate_path_length`
consumes that complete policy-backed role proof and derives the initial
path-length capacity from its retained construction. Its non-copyable result
keeps the exact Name policy, reviewed-root selection, certificate-wide anchor
role policy, and derived state together. Generic reviewed-terminal evidence
cannot produce this stronger initializer, and callers cannot detach the policy,
inflate the capacity, or attach it to another construction. This remains only
initialization; it does not authenticate a link, apply a transition, define
complete Name equality, or validate a complete path.

A stronger follow-up consumes that reviewed-terminal-bound path together with
certificate-wide role-policy evidence for the exact terminal object. It
rejects byte-equal re-decodes and requires the retained role to be issuing CA,
then keeps the complete critical-extension and role-policy evidence with the
reviewed path.

`initialize_reviewed_constructed_x509_certificate_path_length` consumes that
stronger reviewed-terminal issuing-CA proof and derives RFC 5280's initial
path-length capacity from the retained construction. The final selected issuer
is the reviewed trust anchor, so the initial capacity is exactly the remaining
selected-issuer count. The non-copyable result keeps the complete construction,
reviewed-root selection, anchor role policy, and derived state together; a
caller cannot inflate the capacity or attach it to another path. This is only
initialization. It does not authenticate a link, apply an intermediate
transition, validate a complete path, or provide TLS, HTTP, OAuth, or network
authority.

This crate does not authenticate path links or validate self-signature,
algorithms, validity, path-length transitions, name constraints, hostname,
revocation, or any other complete path policy. Issuing-CA role binding and
construction-derived initialization do not make a path suitable for TLS, HTTP,
OAuth transport, or live-provider use.
The policy-backed bindings also do not make the available exact-DER and exact
Unicode-scalar Name rules complete RFC 5280 Name equality.
The crate performs no filesystem, clock, credential, signing, or network
effect.

## Validation

```bash
bash BUILD
cargo clippy -p x509-certificate-path-reviewed-anchor-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-certificate-path-reviewed-anchor-binding --no-deps
```
