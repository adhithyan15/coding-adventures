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

A consuming reviewed-path entry point additionally takes ownership of one
selected transcript, fixes the signature role to server, derives the RSA key
only from the reviewed target certificate, authenticates and appends the exact
`CertificateVerify` body, and verifies peer verify-data with one caller-
supplied Finished key over that exact post-message transcript. Its non-copyable
result retains the complete reviewed path and the full body, certificate key,
transcript-snapshot, and Finished-verdict evidence graph. Failure consumes the
transcript and returns no partial state. The caller-supplied Finished key is
checked for server role and exact selected suite, but this boundary neither
establishes nor retains its ECDHE, handshake-traffic-secret, or key-schedule
provenance.

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

The package also seals the complete HTTPS destination, reviewed-path, and
`CertificateVerify` result behind a marker trait and can join it to an
authenticated retry-handshake `Certificate` leaf. The non-copyable result
retains both complete evidence graphs only when they point to the exact same
repository-decoded X.509 leaf object. Independently decoded certificates are
rejected even when their DER bytes are equal, preventing a byte-equal leaf
substitution between the authenticated record stream and the reviewed server
authentication graph.

The package can separately join that sealed reviewed server-authentication
graph to the authenticated ordered retry `CertificateVerify` callback. Both
inputs must retain the exact same repository-owned signature-verdict object;
message, signature, key, or transcript value equality cannot substitute a
detached callback verdict. The non-copyable result retains both complete
graphs and therefore proves that the callback used the destination/path-
derived RSA-key verdict.

The earlier authenticated retry `Certificate` leaf join can now initialize a
sealed non-copyable continuity receipt that survives its synchronous callback
without retaining callback-scoped message bytes. The receipt keeps the exact
reviewed-authentication reference plus opaque identities for the single-use
server record-state issuer, decoded leaf, and signature verdict. A later
ordered receipt retains the issuer borrow until completion or drop, preventing
a later owner from matching through a recycled allocation or stack address. The later
ordered retry `CertificateVerify` callback completes it only when its existing
reviewed-authentication join retains that same authentication object, issuer,
leaf, and verdict. Different ordered owners, independently decoded byte-equal
leaves, detached authentication wrappers, and value-equal detached verdicts
fail closed. The complete upstream callback proofs remain authoritative at
their transitions; the receipt records their continuity rather than extending
their borrowed message lifetimes.

That completed `Certificate`-to-`CertificateVerify` receipt can now consume the
exact repository-authenticated ordered retry server `Finished` proof. The join
requires the same lifetime-bearing single-use record-state issuer on both
inputs, then returns non-copyable evidence owning the entire reviewed
destination, path, leaf-key, signature-verdict, retry Finished-key,
pre-`Finished` transcript-snapshot, and peer-verify-data graph. A proof from a
different ordered owner fails without returning either consumed input.

For the stronger transcript handoff, use
`complete_ordered_retry_reviewed_committed_server_finished_continuity`
with the reviewed certificate receipt and the result of
`bind_committed_finished` from the successful authenticated record.
This consumes both inputs and retains the entire proof through the
record-boundary commitment and the post-Finished transcript. It rejects a
different record owner even for an independently authenticated, byte-equal
flight. A callback-only Finished verdict is not sufficient for this entry
point. The join itself does not derive secrets or install an application epoch.

To derive the initial application traffic secrets, consume that completed
proof with `derive_application_traffic_secrets` and caller-owned
`ApplicationTrafficSecretStorage`. The full selected suite policy must
match before storage is touched. The derivation uses the retained retry
Master Secret and committed post-Finished snapshot; no caller-provided
digest or schedule is accepted. Success keeps the reviewed proof beside the
two initial secrets in wipe-on-drop storage. This does not derive or install
record keys or authorize application data.

This package does not itself verify an anchor self-signature or evaluate
revocation; the specialized entry points accept only complete proofs produced
by existing repository-owned boundaries. The revocation composition currently
covers the direct, one-level nested, double-nested, recursive-responder, and
alternative intermediate-status-responder no-check completions. It does not
acquire or refresh revocation data, prove ordered handshake state before the
retained `Certificate` leaf, install later
record epochs, perform I/O, or make TLS, HTTPS, HTTP, OAuth transport, or live-
provider access available.

The end-to-end tests retain a public `www.apple.com` TLS 1.3 handshake capture
from 2026-11-18 plus its 2026-11-11 leaf certificate. They reconstruct the
complete reviewed path to the pinned public-Web-PKI root, prove that a client
role fails before output or scratch mutation, and verify the captured server
signature with the exact reviewed leaf key. They also exercise the stronger
destination-bound entry point and prove that it retains the same destination,
path, transcript, message, and signature graph. Finally, they verify the
reviewed root self-signature and exercise the strongest entry point over the
same exact graph. These checked-in public fixtures are test data only; the
package performs no network access. The same fixtures also prove exact path-
derived key retention, post-`CertificateVerify` Finished snapshot equality,
and failure without returned state for invalid Finished verify-data. Focused
revocation entry-point tests use the same public transcript, leaf, message, and
signature to prove exact direct,
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
