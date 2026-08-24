# Changelog

## Next

- Add one explicitly configured, credential-free local IPP/1.1 printer-status
  read with D23 authorization before I/O, fixed attribute selection, strict
  response correlation, bounded HTTP lifetime, and normalized state.
- Keep print submission, jobs, queue mutation, credentials, IPPS trust,
  arbitrary attributes, public endpoints, and long-lived connections out of
  scope.

## 0.1.0

- Add authorized, bounded `_ipp._tcp.local` discovery with strict IPP
  Everywhere resource-path, TXT-version, UUID, model, location,
  authentication, TLS, document-format, color, duplex, and endpoint checks.
- Normalize verified printers into D23 without opening an IPP session, reading
  status, accepting credentials, submitting jobs, mutating queues, or exposing
  control.
