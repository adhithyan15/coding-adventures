# Changelog

## 0.1.0

- Add a production ONVIF snapshot host that requires exact D23 Human Approval
  before resolving durable sealed-Vault credentials or reaching media I/O.
- Keep HTTP credentials process-local and remove them after every delivery
  attempt while retaining only opaque endpoint and Vault references.
- Require a bounded versioned credential envelope so future schema changes fail
  closed instead of being interpreted ambiguously.
