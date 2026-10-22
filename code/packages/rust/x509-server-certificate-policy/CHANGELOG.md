# Changelog

## 0.1.0 — 2026-10-21

- Add non-copyable exact-certificate server-policy evidence.
- Bind certificate-wide end-entity role and data-driven algorithm policy before
  deriving SAN identity, explicit-time validity, and exact EKU purpose results.
- Reject byte-equal evidence from independently decoded certificate objects.
- Keep failures redacted and grant no path, trust, revocation, TLS, HTTP, OAuth
  transport, or network authority.
