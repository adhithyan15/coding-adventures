# Changelog

## 0.1.0

- Add authorized, bounded read-only CoAP telemetry for explicit local unicast
  endpoints and profile-defined resources.
- Decode strict text and JSON scalar representations into normalized D23
  sensors while excluding writes, Observe, multicast, blockwise transfer, and
  unauthenticated public-network endpoints.
