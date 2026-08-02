# Changelog

## 0.3.0

- Preserved canonical host and pinned socket review across snapshot/stream
  registration and the trusted camera-media executor boundary.
- Made explicit reviewed TLS names authoritative, redacted TLS/device debug
  details, normalized bracketed IPv6 identities, bounded custom SOAP transports,
  rejected credential-bearing SOAP query strings, and poisoned conflicting
  duplicate WS-Discovery endpoint references fail-closed.
- Strengthened the language-neutral origin contract with operation-specific
  schemas, every-XAddr checks, approved-versus-observed resolver evidence,
  exact production-shared size boundaries, semantic URI/IP validation, and
  always-on CI schema validation.

## 0.2.0

- Correlated WS-Discovery responses with the emitted probe ID and responder,
  retained discoveries as candidates, and bounded discovery values.
- Added a language-neutral hostile-origin fixture suite and a host-owned origin
  policy with HTTPS-by-default review, loopback-only plaintext tests, exact
  origin checks, pinned addresses, DNS-rebinding defense, and redacted errors.
- Pinned TCP/TLS connections while retaining canonical Host/SNI names and
  rejected redirects before any credential-bearing follow-up.
- Validated media, snapshot, and stream URLs before storing them and proved a
  cross-origin media service receives no fresh UsernameToken digest.
- Bounded credentials, profiles, device-controlled strings, URLs, and generated
  requests.

## 0.1.1

- Adapted camera registration and the real loopback integration path to the
  host-owned camera-media service and its narrow endpoint registry; the loopback
  transport exception is now an explicit fixture policy rather than a default.

## 0.1.0

- Added bounded ONVIF WS-Discovery scanning and ProbeMatch normalization.
- Added WS-Security SOAP over real LAN HTTP/TLS transports.
- Added camera/device/media profile collection and D23 runtime projection.
- Added privacy-preserving camera media lease handoff and a one-shot CLI.
- Added real loopback UDP and TCP protocol tests with credential redaction.
