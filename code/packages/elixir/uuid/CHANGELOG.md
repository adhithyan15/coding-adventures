# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Full UUID v1/v3/v4/v5/v7 implementation from scratch (RFC 4122 + RFC 9562)
- `Ca.Uuid` module working with 16-byte binary UUIDs natively
- `Ca.Uuid.parse/1`: accepts standard, URN, braced, compact string formats
- `Ca.Uuid.to_string/1`: formats 16-byte binary as 8-4-4-4-12 hex
- `Ca.Uuid.version/1`, `Ca.Uuid.variant/1`: bit-pattern extraction via Elixir
  binary pattern matching
- `Ca.Uuid.is_nil_uuid/1`, `Ca.Uuid.is_max_uuid/1`
- Namespace accessors: `namespace_dns/0`, `namespace_url/0`, `namespace_oid/0`,
  `namespace_x500/0` (RFC 4122 Appendix C; decoded at compile time)
- `nil_uuid/0` and `max_uuid/0` sentinels
- `Ca.Uuid.v4/0`: `:crypto.strong_rand_bytes/1` for CSPRNG
- `Ca.Uuid.v5/2`: SHA-1 name-based via Ca.Sha1;
  RFC vector: v5(namespace_dns(), "python.org") = "886313e1-3b8a-5372-9b90-0c9aee199e5d"
- `Ca.Uuid.v3/2`: MD5 name-based via Ca.Md5;
  RFC vector: v3(namespace_dns(), "python.org") = "6fa459ea-ee8a-3ca4-894e-db77e160355e"
- `Ca.Uuid.v1/0`: 60-bit Gregorian timestamp via `System.os_time(:nanosecond)` +
  random clock sequence + random node (multicast bit set)
- `Ca.Uuid.v7/0`: 48-bit Unix millisecond timestamp (sortable) + 74 random bits
- App named `:ca_uuid`, module `Ca.Uuid` (ca_ prefix for consistency)
- 47 tests, 94% line coverage
- Knuth-style literate programming comments throughout
