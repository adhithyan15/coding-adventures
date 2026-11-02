# Changelog

## 0.2.0 — 2026-10-30

- Add non-copyable retention for the exact decoded Subject Alternative Name
  collection selected by successful server-identity policy.
- Re-decode only the retained extension index under a fresh explicit ASN.1
  budget and keep failures payload-free for later NameConstraints composition.

## 0.1.0 — 2026-10-21

- Add non-copyable exact-certificate server-policy evidence.
- Bind certificate-wide end-entity role and data-driven algorithm policy before
  deriving SAN identity, explicit-time validity, and exact EKU purpose results.
- Reject byte-equal evidence from independently decoded certificate objects.
- Keep failures redacted and grant no path, trust, revocation, TLS, HTTP, OAuth
  transport, or network authority.
