# tls13-reviewed-server-path-certificate-verify

`tls13-reviewed-server-path-certificate-verify` binds one TLS 1.3 server
`CertificateVerify` verdict to the exact target key of the strongest
accumulated reviewed RSA server path. Its stronger entry point accepts that
path only through the existing exact HTTPS DNS destination binding. Its
anchor-aware entry point additionally requires the destination-bound path to
retain the reviewed anchor's repository-owned self-signature proof. Orthogonal
direct, nested, double-nested, recursive-responder, and alternative
intermediate-status-responder revocation entry points accept the leaf only
through their exact existing destination-bound complete no-check fixed-path
revocation proofs.

The composition accepts only complete reviewed-path evidence, outer
selected-transcript hash evidence, the exact decoded message, data-driven RSA
key policy, typed verifier limits, and caller-owned scratch. It rejects a
client-role signed input before RSA work or scratch mutation, derives the leaf
certificate exclusively through the reviewed path, validates that leaf's exact
SubjectPublicKeyInfo, and delegates signature verification to repository-owned
RSA-PSS code. The path-only non-copyable result retains both complete proofs.
The destination entry point additionally retains the complete certificate SAN,
TLS SNI, HTTP Host authority, and reviewed-path binding, so the authenticated
signature cannot be detached from the exact destination identity. The
anchor-aware entry point retains that same graph plus the existing reviewed
root self-signature evidence, so the handshake signature cannot be detached
from the exact root whose self-signature was verified. The direct, nested,
double-nested, recursive-responder, and alternative intermediate-status-
responder revocation entry points each derive the leaf only through the
destination already bound to their complete proof, then retain that exact
proof and the signature verdict together.

This package does not itself verify an anchor self-signature or evaluate
revocation; the specialized entry points accept only complete proofs produced
by existing repository-owned boundaries. The revocation composition currently
covers the direct, one-level nested, double-nested, recursive-responder, and
alternative intermediate-status-responder no-check completions. It does not
acquire or refresh revocation data, prove ordered handshake state,
authenticate Finished, process records, perform I/O, or make TLS, HTTPS, HTTP,
OAuth transport, or live-provider access available.

The end-to-end tests retain a public `www.apple.com` TLS 1.3 handshake capture
from 2026-11-18 plus its 2026-11-11 leaf certificate. They reconstruct the
complete reviewed path to the pinned public-Web-PKI root, prove that a client
role fails before output or scratch mutation, and verify the captured server
signature with the exact reviewed leaf key. They also exercise the stronger
destination-bound entry point and prove that it retains the same destination,
path, transcript, message, and signature graph. Finally, they verify the
reviewed root self-signature and exercise the strongest entry point over the
same exact graph. These checked-in public fixtures are test data only; the
package performs no network access. Focused revocation entry-point tests use
the same public transcript, leaf, message, and signature to prove exact direct,
nested, double-nested, recursive-responder, and alternative intermediate-
status-responder evidence retention, leaf-key derivation, and role-first
scratch non-mutation; the upstream sealed destination-to-revocation boundary
enforces the exact-leaf object binding admitted by the production entry point.

An ordered no-PSK server-flight transition accepts only those five sealed
destination-bound complete-revocation `CertificateVerify` result types. The
flight must already be positioned at `ExpectCertificateVerify`; success
advances it to `ExpectFinished` and returns non-copyable evidence retaining a
mutable borrow of that exact flight plus the exact verification proof. A wrong
state fails through the ordering policy without mutation, and the optional
client-authentication requirement remains attached to the flight.

This bounded transition does not remove the lower-level message-type API or
prove that its earlier accepted types and the verified message share one
encoded handshake stream. It does not update a transcript, authenticate
Finished, enforce the later client-authentication response, protect records,
perform I/O, or make TLS, HTTPS, HTTP, OAuth transport, or live-provider access
available. It accesses no credential and causes no signing, provider, trace,
storage, clock, network, or other external effect.

A second bounded transition consumes that ordered result together with one
repository-owned `VerifiedFinished`. It rejects client-role evidence and a
cipher-suite substitution before flight mutation, then accepts `Finished` in
the retained ordering policy and returns non-copyable evidence retaining the
exact complete-revocation `CertificateVerify` verdict, exact `Finished`
verdict, and completed flight. The optional client-authentication requirement
remains attached to the flight.

This completion does not prove that the `Finished` verdict's retained
transcript hash follows the exact retained `CertificateVerify` message or any
earlier accepted type in one encoded handshake stream. It does not enforce a
later client-authentication response, bind record epochs, protect records,
perform I/O, or make TLS, HTTPS, HTTP, OAuth transport, or live-provider access
available. It accesses no credential and causes no signing, provider, trace,
storage, clock, network, or other external effect.

## Development

```bash
bash BUILD
```
