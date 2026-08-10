# Changelog

## 0.1.0

- Added provider-neutral `storage-core` implementations of the VLT-PM05
  bootstrap and local owner-state store contracts.
- Preserved immutable bootstrap generations behind an atomic latest pointer.
- Added exact, bounded, read-back-verified local-state compare-and-exchange.
- Serialized application-store writes within the supported backend instance so
  restart-local revision tokens cannot admit an exact stale value.
- Added in-memory race coverage and filesystem restart coverage.
