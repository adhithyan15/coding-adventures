# Changelog

## Unreleased

- Added bounded, exact-schema static public-provider decoding with caller-owned
  client IDs and redirect URIs, explicit mix-up defense and response format,
  closed errors for malformed or unknown provider data, and registry-enforced
  uniqueness when a provider relies on distinct-redirect mix-up defense.
- Added a provider-neutral OAuth broker with an arbitrary-size data-driven
  provider registry, audited credential creation and refresh orchestration,
  proactive expiry policy, and injected clock and bounded token-transport
  boundaries.
- Added no external dependency; the broker composes only repository-owned
  bounded JSON, OAuth, credential-custody, and zeroization primitives.
