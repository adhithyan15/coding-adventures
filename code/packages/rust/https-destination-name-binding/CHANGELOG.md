# Changelog

## 0.1.0

- Add a non-copyable proof that one exact decoded certificate's Subject
  Alternative Name matches the same DNS name used by TLS SNI and HTTP/1 Host.
- Retain the exact certificate, SNI, Host authority, and certificate-identity
  evidence without granting TLS, HTTP, OAuth, I/O, or external-effect authority.
- Keep stable failures and Debug output free of hostnames and certificate bytes.
- Bind the complete destination proof to one strict HTTP/1 Host field only when
  its value is the retained authority's canonical lowercase-host and
  shortest-decimal-port encoding.
- Retain both non-copyable inputs with payload-free failures and redacted Debug
  output without claiming complete request-head uniqueness or transport
  authority.
- Bind that canonical Host proof to an exact caller-borrowed slice of
  additional validated fields under a caller-selected total-field ceiling
  hard-capped at 64.
- Reject every additional case-insensitive Host spelling while retaining the
  complete field slice, caller order, and limit policy with payload-free
  failures and redacted Debug output.
