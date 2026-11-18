# tls13-reviewed-server-path-certificate-verify

`tls13-reviewed-server-path-certificate-verify` binds one TLS 1.3 server
`CertificateVerify` verdict to the exact target key of the strongest
accumulated reviewed RSA server path. Its stronger entry point accepts that
path only through the existing exact HTTPS DNS destination binding.

The composition accepts only complete reviewed-path evidence, outer
selected-transcript hash evidence, the exact decoded message, data-driven RSA
key policy, typed verifier limits, and caller-owned scratch. It rejects a
client-role signed input before RSA work or scratch mutation, derives the leaf
certificate exclusively through the reviewed path, validates that leaf's exact
SubjectPublicKeyInfo, and delegates signature verification to repository-owned
RSA-PSS code. The path-only non-copyable result retains both complete proofs.
The destination entry point additionally retains the complete certificate SAN,
TLS SNI, HTTP Host authority, and reviewed-path binding, so the authenticated
signature cannot be detached from the exact destination identity.

This package does not prove ordered handshake state, verify the reviewed
anchor's self-signature, choose a revocation result, authenticate Finished,
process records, perform I/O, or make TLS, HTTPS, HTTP, OAuth transport, or
live-provider access available.

The end-to-end tests retain a public `www.apple.com` TLS 1.3 handshake capture
from 2026-11-18 plus its 2026-11-11 leaf certificate. They reconstruct the
complete reviewed path to the pinned public-Web-PKI root, prove that a client
role fails before output or scratch mutation, and verify the captured server
signature with the exact reviewed leaf key. They also exercise the stronger
destination-bound entry point and prove that it retains the same destination,
path, transcript, message, and signature graph. These checked-in public
fixtures are test data only; the package performs no network access.

## Development

```bash
bash BUILD
```
