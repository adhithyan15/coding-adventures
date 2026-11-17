# oauth-client-secret-reviewed-server-binding

This crate joins one complete audited OAuth client-secret HTTP/1 request to one
complete HTTPS destination, reviewed RSA server path, and reviewed-anchor
self-signature proof. It accepts only the repository-owned sealed evidence
types and requires both retained destinations to have:

- the same exact decoded certificate object;
- ASCII-case-insensitively equal SNI DNS names; and
- ASCII-case-insensitively equal Host DNS names with the same optional port.

The separate SNI and Host comparisons prevent substituting another valid SAN
name from the same certificate. A byte-equal independent certificate decode
also fails closed. Success moves both complete non-copyable inputs into one
redacted result.

The crate also consumes that complete request/signed-server binding together
with the exact destination-bound direct no-check complete fixed-path revocation
proof. The signed server and revocation destinations must satisfy the same
leaf certificate, target-nearest issuer, reviewed anchor, SNI, Host, and port
checks. Both accumulated paths must contain exactly one processed intermediate
because this direct completion covers the target and that one intermediate;
zero- and multi-intermediate paths fail closed. The admitted direct completion
must itself retain the exact signed reviewed path-and-anchor-self-signature
proof shape, so an alternate intermediate or reviewed-root path fails closed.
Success retains both inputs, and the direct revocation proof augments rather
than substitutes for the signed path evidence.

The crate separately admits the destination-bound nested no-check complete
fixed-path revocation branch. That branch retains the same exact signed
reviewed path-and-anchor-self-signature proof beneath its responder-revocation
and responder-no-check layers. Composition therefore repeats the exact leaf,
target-nearest issuer, reviewed anchor, SNI, Host, and port checks and requires
exactly one processed server-path intermediate on both complete inputs. The
additional responder layers do not extend the target-plus-intermediate server
chain covered by the nested completion. Success retains the nested proof as
additive evidence; a byte-equal independent leaf decode, alternate issuer or
root, destination substitution, or zero- or multi-intermediate path fails
closed.

The crate also admits the destination-bound double-nested no-check complete
fixed-path revocation branch under a separate sealed marker. The proof retains
the same exact signed reviewed path-and-anchor-self-signature evidence beneath
both responder-revocation layers. Composition traverses those layers back to
the original server proof, repeats the exact leaf, target-nearest issuer,
reviewed anchor, SNI, Host, and port checks, and requires exactly one processed
intermediate on both retained server paths. The additional responder paths do
not extend the target-plus-intermediate server chain. Success retains the
double-nested proof as additive evidence; a byte-equal independent leaf decode,
alternate issuer or root, destination substitution, or zero- or multi-
intermediate path fails closed.

The recursive-responder no-check complete fixed-path branch is admitted under
another sealed marker implemented only for the exact repository-owned
destination-bound completion. Composition traverses its recursive no-check and
authenticated responder-revocation layers back to the original signed
reviewed-server proof. It repeats the exact leaf, target-nearest issuer,
reviewed anchor, SNI, Host, optional-port, and one-processed-intermediate
checks. Recursive responder paths do not extend the retained server chain.
Success preserves the complete recursive proof as additive evidence; identity,
path, destination, and unsupported-depth substitutions fail closed.

The composition never asks the request for its credential-bearing wire bytes,
so it introduces no credential access or audit event. Every later byte borrow
still passes through the request release's provider-and-trace audit gate.

This is not a TLS or HTTP client. It performs no certificate parsing, new
validation, cryptography, revocation check or acquisition, credential edit,
signing, I/O, or external effect. It covers only the direct no-check complete
fixed-path, nested no-check complete fixed-path, double-nested no-check
complete fixed-path, and recursive-responder no-check complete fixed-path
revocation branches, does not complete RFC 5280 Name or path validation, and
does not grant HTTPS, OAuth, network, or live-provider readiness. The
alternative revocation branch remains outside this boundary.

## Validation

```bash
bash BUILD
cargo clippy -p oauth-client-secret-reviewed-server-binding --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p oauth-client-secret-reviewed-server-binding --no-deps
```
