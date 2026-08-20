# Changelog

## 0.1.0

- Add authorized, bounded `_ipp._tcp.local` discovery with strict IPP
  Everywhere resource-path, TXT-version, UUID, model, location,
  authentication, TLS, document-format, color, duplex, and endpoint checks.
- Normalize verified printers into D23 without opening an IPP session, reading
  status, accepting credentials, submitting jobs, mutating queues, or exposing
  control.
