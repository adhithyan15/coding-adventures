# Changelog

## 0.3.0

- Add host-owned Enphase inverter pseudonym-key rotation over one bounded
  authenticated production response and two atomically consumed one-shot key
  leases.
- Derive exact old/new inverter correspondence while raw serials remain in the
  zeroizing response tree, then dispose both keys before runtime migration.
- Persist complete gateway, meter, and inverter identity replacement through
  the runtime store's expected-revision CAS path, rejecting stale automation
  references before live state changes.

## 0.2.0

- Add authenticated per-microinverter production inspection over Enphase's
  documented `/api/v1/production/inverters` endpoint.
- Require a host-owned grant with declared purpose, consent receipt, and
  ephemeral device-identifier retention before credentials or transport I/O.
- Derive stable host-scoped inverter pseudonyms with a Vault-leased 32-byte key
  while zeroizing raw serial response values and excluding serials from runtime
  identity, metadata, state, and debug output.
- Bound inverter counts, reject duplicate or malformed native identifiers, and
  prove exact three-request behavior over real loopback TCP.

## 0.1.0

- Add D23-authorized local IQ Gateway meter inspection over HTTPS.
- Keep bearer tokens zeroizing and private to the transport boundary.
- Normalize bounded meter status and aggregate readings into confirmed sensor state.
- Cover the exact production HTTP flow with a loopback protocol test.
