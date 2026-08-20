# Changelog

## 0.1.0

- Add authorized, bounded `_hap._tcp.local` discovery with strict HAP IP device
  identity, configuration, pairing-feature, protocol, state, status, category,
  optional setup-hash, and endpoint checks.
- Normalize verified HomeKit accessories into D23 without opening a HAP session,
  accepting a setup code, pairing, subscribing, reading, or exposing control.
