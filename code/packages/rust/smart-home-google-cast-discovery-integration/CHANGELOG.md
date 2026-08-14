# Changelog

## 0.1.0

- Add authorized, bounded `_googlecast._tcp.local` discovery with strict CastV2
  receiver identity, protocol-version, capability, status, and endpoint checks.
- Normalize verified Cast receivers into D23 without opening a TLS channel or
  exposing media commands.
