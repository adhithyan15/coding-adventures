# Changelog

## 0.1.0

- Add a non-copyable proof that one exact decoded certificate's Subject
  Alternative Name matches the same DNS name used by TLS SNI and HTTP/1 Host.
- Retain the exact certificate, SNI, Host authority, and certificate-identity
  evidence without granting TLS, HTTP, OAuth, I/O, or external-effect authority.
- Keep stable failures and Debug output free of hostnames and certificate bytes.
