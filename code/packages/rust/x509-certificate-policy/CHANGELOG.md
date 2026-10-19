# Changelog

## 0.1.0

- Add a certificate-wide, fail-closed gate for caller-recognized critical
  extensions.
- Add bounded algorithm-independent issuing-CA and end-entity role checks that
  compose Basic Constraints, Key Usage, and Extended Key Usage under one
  explicit semantic decoder budget.
- Add registry-free selection of one exact caller-supplied certificate
  algorithm profile with parameter and inclusive encoded BIT STRING length
  policy.
- Add bounded complete-certificate Subject Alternative Name decoding and typed
  DNS-ID/IP-ID server-identity matching without Common Name fallback.
- Add an ambient-clock-free complete-certificate validity gate for one explicit
  caller-owned observation with inclusive RFC 5280 endpoints.
- Add fail-closed authorization of one exact caller-supplied Extended Key Usage
  purpose over already validated end-entity role evidence, with no embedded
  registry or implicit `anyExtendedKeyUsage` wildcard.
