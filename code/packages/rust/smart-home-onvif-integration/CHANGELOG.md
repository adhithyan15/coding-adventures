# Changelog

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
