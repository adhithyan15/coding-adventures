# Changelog

## 0.4.0

- Add host-owned connected-client pseudonym-key rotation for an explicit UniFi
  site using one bounded authenticated client response and two atomically
  consumed one-shot 32-byte key leases.
- Require exact correspondence with the installed pseudonymous client set,
  preserve each client's existing five-minute presence expiry, and dispose
  native identifiers and both keys before runtime migration.
- Persist all client device/entity replacements through expected-revision
  runtime-store CAS and reject stale automation identity references before live
  state changes.

## 0.3.0

- Add official per-device latest-statistics inspection for explicitly selected,
  already-installed UniFi devices.
- Require D23 read authorization and an exact two-minute local operational-
  telemetry grant before transport I/O.
- Cap each poll at 64 unique targets, enforce a one-minute integration-level
  poll interval before I/O, and expire normalized metrics after two minutes.
- Validate CPU, memory, load, uptime, uplink-rate, frequency, and radio-retry
  bounds while excluding native heartbeat timestamps from runtime state.

## 0.2.0

- Add bounded official connected-client inspection behind D23 read,
  device-identifier, and five-minute presence authorization.
- Derive host-scoped client pseudonyms from a separate zeroizing 32-byte key and
  exclude native IDs, names, MACs, IPs, and connection timestamps from runtime.
- Install only expiring pseudonymous presence and access shape, with exact
  loopback coverage and consent denial before transport.

## 0.1.0

- Add D23-authorized local UniFi Network application, site, and adopted-device
  health inspection through the official integration API.
- Keep API keys transport-private in zeroizing memory and materialize
  X-API-Key only while encoding a bounded HTTPS request.
- Add exact loopback coverage for application info and paginated site/device
  reads.
