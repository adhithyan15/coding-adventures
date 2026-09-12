# Changelog

## Unreleased

- Added exact-schema static confidential-provider decoding that retains one
  closed implemented authentication method while rejecting public `none`,
  absent, case-variant, and user-defined methods. JWT profiles require bounded,
  unique, non-`none` algorithm data while secret profiles reject algorithms;
  no credential/key access or concrete signer is added.
- Added audited composition from one caller-timed device poll into exact
  opaque-key credential creation, returning only the custody revision after an
  authorized response while continuation paths avoid clock and storage access.
- Added an opaque caller-timed RFC 8628 polling sequence that rejects early and
  expired polls before request preparation or transport, performs at most one
  audited effect per step, preserves provider/client/endpoint/trace binding,
  applies cumulative `slow_down`, and reschedules transient transport failures
  without acquiring clock or sleep authority.
- Added exact registry-bound RFC 8628 device authorization initiation through
  an injected transport, with provider/trace audit gates around orchestration,
  transport effect and result, protocol preparation, and response release.
- Added an injected static provider-data source boundary with durable
  provider/trace audit before each read, exact requested-provider binding before
  registry mutation, closed source errors, zeroizing profile bytes, and a
  separately audited registration result.
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
