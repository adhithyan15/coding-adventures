# Changelog

## 0.1.0

- Add exact target-and-issuer binding between authenticated OCSP revocation and
  a constructed, Mozilla-reviewed, anchor- and intermediate-name-constrained
  RSA SHA-256 server path.
- Retain both complete non-copyable inputs and expose only stable payload-free
  target/issuer mismatch categories.
- Revalidate and retain target and intermediate validity at the exact OCSP
  observation time, rejecting cross-time evidence mixing with stable redacted
  target/issuer validity categories.
- Add a static public Apple/DigiCert stapled-OCSP integration fixture with
  recorded acquisition and verification provenance.
