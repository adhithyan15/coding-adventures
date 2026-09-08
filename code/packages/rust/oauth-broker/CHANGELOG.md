# Changelog

## Unreleased

- Added a provider-neutral OAuth broker with an arbitrary-size data-driven
  provider registry, audited credential creation and refresh orchestration,
  proactive expiry policy, and injected clock and bounded token-transport
  boundaries.
- Added no external dependency; the broker composes only repository-owned OAuth,
  credential-custody, and zeroization primitives.
