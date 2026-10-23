# x509-certificate-path-length-policy

A bounded repository-owned RFC 5280 path-length state transition for
caller-ordered intermediate CA certificates.

The caller initializes an explicit maximum and advances from the trust-anchor
side toward the target. Each step requires existing issuing-CA role evidence.
A non-self-issued intermediate consumes one unit; a self-issued intermediate
does not. The intermediate's decoded `pathLenConstraint`, when present, can
only tighten the remaining maximum.

The strongest entry point accepts complete non-copyable
`ValidatedX509IntermediateCertificatePolicy` evidence. It derives both the
issuing-CA role and self-issued transition bit from that exact-certificate
boundary, preserving the prerequisite critical-extension recognition,
algorithm policy, and explicit-time validity checks in the traversal contract.
An earlier evidence-bound entry point can still derive the role check from an
exact certificate's self-issued classification, and the lower-level transition
remains available when a larger caller already owns all required bindings.

The caller remains responsible for deciding whether a certificate is
self-issued through its Name policy and for supplying the correct path order.
The target certificate and trust anchor are not passed through this transition.
This crate does not construct, order, link, authenticate, or trust a path;
retain authenticated adjacent-link evidence; process trust-anchor or name
constraints; check revocation; or provide TLS, HTTP, OAuth transport, or
network authority.

## Validation

```bash
bash BUILD
cargo clippy -p x509-certificate-path-length-policy --all-targets -- -D warnings
cargo doc -p x509-certificate-path-length-policy --no-deps
```
