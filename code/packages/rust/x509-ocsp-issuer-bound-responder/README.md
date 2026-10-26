# x509-ocsp-issuer-bound-responder

`x509-ocsp-issuer-bound-responder` binds one exact authenticated OCSP responder
candidate to one exact candidate issuer. It consumes the complete responder
authentication proof and an RSA SHA-256 issuer link, rejects evidence whose
responder certificates are distinct decoded objects, and retains both proofs.

Together, the consumed evidence proves that the candidate issuer has an
issuing-CA role, signed the policy-approved OCSP-signing responder certificate,
and that the responder certificate's key signed the exact response. Errors
expose only stable categories and retain no response, certificate, key, or
signature bytes.

This package does not establish that the candidate issuer issued any particular
target certificate, bind an OCSP `CertID` or revocation decision, construct or
validate a responder path, establish trust, check responder-certificate
revocation, fetch or cache data, read a clock, perform TLS or HTTP, authorize
OAuth transport, or open a network connection.

## Test-vector provenance

The public issuer and responder certificate fixtures were generated once with
OpenSSL 3.6.3 on 2026-10-25. A temporary 2048-bit issuer key signed the responder
certificate; that test-only key was not checked in and its temporary directory
was moved to Trash after the public DER and audit hashes were captured. The
responder public key and response signature reuse the independently recorded
public interoperability vector from `x509-ocsp-authenticated-responder`.

- issuer certificate DER SHA-256:
  `3fad12f7deaeb31ad73d9342f823d2c34e7b46c9636c8e25b4745d6777072fa5`
- responder certificate DER SHA-256:
  `38a954a1080409d5fd31d50581af31ec5df84f75814468108dd27a172f90f66e`
- responder SPKI DER SHA-256:
  `425c2ae3ec7b195f1befc2c088cedc338d745a82d3bf9560b4f56abdf09d7d36`

## Validation

```bash
bash BUILD
cargo clippy -p x509-ocsp-issuer-bound-responder --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p x509-ocsp-issuer-bound-responder --no-deps
```
