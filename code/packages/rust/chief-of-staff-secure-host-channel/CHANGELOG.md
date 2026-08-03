# Changelog

## 0.1.0

- Add fresh per-spawn X3DH bootstrap offers and authenticated child hellos.
- Add strict bounded `D18O`, `D18H`, and `D18F` wire decoders.
- Bind host, UUID-v7 session, direction, and exact sequence into frame AAD.
- Close ratchet state after authentication failure or sequence exhaustion.
