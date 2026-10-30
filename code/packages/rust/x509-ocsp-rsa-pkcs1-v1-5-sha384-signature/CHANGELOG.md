# Changelog

All notable changes to `x509-ocsp-rsa-pkcs1-v1-5-sha384-signature` will be
documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-10-29

### Added

- Added exact `sha384WithRSAEncryption` and present-NULL parameter checks for a
  structurally decoded `BasicOCSPResponse`.
- Added composition of repository-owned SHA-384, bounded RSAVP1, and canonical
  PKCS #1 v1.5 SHA-384 validation over preserved `ResponseData` DER.
- Added non-copyable exact response/key evidence, payload-free failures, and an
  independently generated interoperability fixture with public trace digests.

### Security

- The API accepts only separately policy-validated public RSA keys and exposes
  no private-key, signing, decryption, trust, TLS, or network authority.
- Algorithm routing, responder authorization, `ResponderID`, certificate/path
  binding, freshness, and revocation decisions remain separate boundaries.
