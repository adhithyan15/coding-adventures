# x509-ocsp-authenticated-target-revocation

`x509-ocsp-authenticated-target-revocation` closes one evidence-mixing gap
between the repository-owned delegated-responder, certificate-identifier,
revocation, and issuer-link primitives. It consumes three complete proofs and
requires them to retain the same exact decoded objects:

- the issuer-authorized delegated responder signed the `BasicOCSPResponse`;
- one `SingleResponse` inside that exact signed response names an exact target
  and issuer and is `good` inside its strict freshness interval; and
- that same issuer signed that same target certificate.

The signed response must contain exactly one `SingleResponse`; this strict
cardinality rule prevents an unexamined entry encoded under another hash
algorithm from conflicting with the selected target status. The responder
certificate is revalidated at the revocation proof's exact observation time.
Byte-equal re-decodes do not satisfy the identity checks. Outer
`responseExtensions` also fail closed because this package does not implement
extension or nonce policy. Errors expose only stable categories and retain no
response, certificate, key, status, time, or signature bytes.

This fixed RSA PKCS #1 v1.5 SHA-256 composition does not construct or validate
a certificate path, establish trust in the candidate issuer, validate a
responder path, check responder-certificate revocation, interpret OCSP
extensions, fetch or cache data, read a clock, perform TLS or HTTP, authorize
OAuth transport, or open a network connection.

## Test-vector signing trace

OpenSSL 3.6.3 generated the public fixtures once on 2026-10-25. The fixture
operation trace is:

1. a temporary 2048-bit issuer RSA key signed its self-issued CA certificate
   with SHA-256 and serial `01`;
2. a temporary 2048-bit responder RSA key signed its `CN=ocsp.example` CSR;
3. the issuer key signed the OCSP-signing responder certificate with SHA-256
   and serial `02`;
4. a temporary 2048-bit target RSA key signed its `CN=target.example` CSR;
5. the issuer key signed the server-auth target certificate with SHA-256 and
   serial `03`;
6. the responder key signed the exact checked-in extension-free `ResponseData`
   DER with RSA PKCS #1 v1.5 SHA-256; and
7. the responder key separately signed the checked-in negative-test
   `ResponseData` containing an unrecognized outer response extension;
8. the responder key separately signed the checked-in negative-test
   `ResponseData` containing both `good` and `revoked` entries for the same
   `CertID`; and
9. the responder key separately signed the checked-in negative-test
   `ResponseData` whose freshness interval occurs after the responder
   certificate expires.

OpenSSL verified both certificate chains for their recorded purposes and
verified all four response signatures before capture. No private key or CSR is
checked in. After the public DER, signatures, hashes, and operation trace were
captured, the exact temporary generation directory was moved to Trash.

- issuer certificate DER SHA-256:
  `e753e3a04ff526e93dba801c21d224f2fdae2966b5883f0b67aad79b063c7341`
- responder certificate DER SHA-256:
  `623765352cc5bd58e04a113a31a16af13f468d5871eee7eb96a71adaab58aa77`
- target certificate DER SHA-256:
  `fef3439bddea088a83e89e6c28c8a44c103a003e972bb331c951b6d01e43e6f4`
- signed `ResponseData` DER SHA-256:
  `ab87cbe247226d66a02f9983ffa1949e7439cac16416862ca383ed8a191c7411`
- response signature SHA-256:
  `5499394a8e612423cd9b94b98b3f053f7488d3ff860b41bc25d7b73d4417717d`
- response-extension negative-test DER SHA-256:
  `2f3f969bfb17561460c6f595367300729ee54ad93508a6f8b8d9bf98c5fbf45a`
- response-extension negative-test signature SHA-256:
  `e4c137e5bc1d8e14cc681c7e220573c3c2e4665a5c4639f6ec3f0e0afc0717fa`
- conflicting-response negative-test DER SHA-256:
  `73ccb85d8dc14d705aa5941d68f6921d8aa1561117d2567290932c696b987187`
- conflicting-response negative-test signature SHA-256:
  `91afb47c4402eda1242793733fceb132c5d6507c81aa780c7f915ce3b4aa3243`
- expired-responder-time negative-test DER SHA-256:
  `1fc2b7bb4699113706f2cd3df4450255ee92b41c2ad5f3f2d7de41a7ec80e6ef`
- expired-responder-time negative-test signature SHA-256:
  `432e046c88355db731a6b05ed18d861eaa5ac2b0c5db97a323caff56896eb5d1`

## Validation

```bash
bash BUILD
cargo clippy -p x509-ocsp-authenticated-target-revocation --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ocsp-authenticated-target-revocation --no-deps
```
