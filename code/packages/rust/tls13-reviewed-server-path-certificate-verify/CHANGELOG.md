# Changelog

All notable changes to this package will be documented in this file.

## [Unreleased]

### Added

- Exact decoded-leaf object composition between an authenticated retry
  handshake `Certificate` proof and the sealed HTTPS DNS destination,
  reviewed-path, path-derived RSA key, and server `CertificateVerify` verdict,
  including byte-equal independently decoded substitution rejection.
- Consuming reviewed-server-path `CertificateVerify`-to-Finished-key
  transcript continuity with a fixed server signature role, exact path-derived
  RSA key, retained proof graph, explicit detached Finished-key provenance,
  and no returned state on failure.
- Same-suite, server-role `VerifiedFinished` completion of the retained ordered
  no-PSK server flight, preserving both cryptographic verdicts, the optional
  client-authentication requirement, and stable pre-mutation failures.
- Ordered no-PSK server-flight transition from `ExpectCertificateVerify` to
  `ExpectFinished` that admits only the five sealed destination-bound complete-
  revocation signature verdicts and retains the exact flight and proof.
- Exact HTTPS DNS destination, alternative intermediate-status-responder
  no-check complete fixed-path revocation, and TLS 1.3 server
  `CertificateVerify` composition with sealed exact-leaf admission, retained
  evidence, and role-first public-fixture tests.
- Exact HTTPS DNS destination, recursive-responder no-check complete fixed-path
  revocation, and TLS 1.3 server `CertificateVerify` composition with sealed
  exact-leaf admission, retained evidence, and role-first public-fixture tests.
- Exact HTTPS DNS destination, double-nested no-check complete fixed-path
  revocation, and TLS 1.3 server `CertificateVerify` composition with sealed
  exact-leaf admission, retained evidence, and role-first public-fixture tests.
- Exact HTTPS DNS destination, nested no-check complete fixed-path revocation,
  and TLS 1.3 server `CertificateVerify` composition with sealed exact-leaf
  admission, non-copyable retained evidence, and public-fixture role coverage.
- Exact HTTPS DNS destination, direct no-check complete fixed-path revocation,
  and TLS 1.3 server `CertificateVerify` composition with role-first leaf-key
  verification, non-copyable retained evidence, public-fixture success, and
  buffer-non-mutation coverage.
- Exact HTTPS DNS destination, reviewed path, reviewed anchor self-signature,
  and TLS 1.3 server `CertificateVerify` composition, with public-fixture
  success and role-first buffer-non-mutation coverage.
- Exact HTTPS DNS destination, reviewed path, and TLS 1.3 server
  `CertificateVerify` composition that retains the complete SAN, SNI, Host,
  path, transcript, message, key, and signature evidence graph.
- End-to-end destination-bound success and role-first buffer-non-mutation
  coverage over the retained public handshake fixtures.

## [0.1.0] - 2026-11-18

### Added

- Server-role-only transcript evidence acceptance before RSA work.
- Reviewed-path leaf SubjectPublicKeyInfo derivation and data-driven RSA key
  policy.
- Non-copyable success evidence retaining the complete reviewed server path and
  exact selected-transcript RSA-PSS signature verdict.
- Stable redacted failure categories and explicit non-readiness boundaries.
- End-to-end public-path fixtures covering role-first rejection, caller-buffer
  non-mutation, exact reviewed-leaf key derivation, signature verification,
  and retained proof identity.
