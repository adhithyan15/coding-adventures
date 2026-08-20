# Changelog

## 0.1.0

- Add authorized, bounded `_scan._sub._ipp._tcp.local` discovery with strict
  IPP Scan Service resource-path, TXT-version, UUID, model, location,
  authentication, TLS, document-format, document-feeder, transparency-adaptor,
  push-destination-scheme, and endpoint checks.
- Normalize verified scanners into D23 without opening an IPP session, reading
  status, accepting credentials, submitting scans, retrieving documents,
  accessing destinations, or exposing control.
