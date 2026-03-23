# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Full UUID v1/v3/v4/v5/v7 implementation from scratch (RFC 4122 + RFC 9562)
- `UUID` class: stores 16 bytes in network byte order; derives `bytes`, `int`,
  `version`, `variant`, `is_nil`, `is_max` from the raw bytes
- `parse()`: accepts standard (8-4-4-4-12), compact (no hyphens), braced
  `{...}`, and `urn:uuid:` string formats; case-insensitive
- `is_valid()`: non-raising string validation
- Namespace constants: `NAMESPACE_DNS`, `NAMESPACE_URL`, `NAMESPACE_OID`,
  `NAMESPACE_X500` (RFC 4122 Appendix C)
- `NIL` and `MAX` sentinels
- `v4()`: 122 bits of `os.urandom` (CSPRNG), version=4, variant=10xx
- `v5(namespace, name)`: SHA-1(namespace_bytes || name_utf8)[:16]; uses ca_sha1
  package. RFC vector: v5(NAMESPACE_DNS, "python.org") = 886313e1-3b8a-5372-...
- `v3(namespace, name)`: MD5(namespace_bytes || name_utf8); uses ca_md5
  package. RFC vector: v3(NAMESPACE_DNS, "python.org") = 6fa459ea-ee8a-3ca4-...
- `v1()`: 60-bit 100-ns Gregorian timestamp + 14-bit random clock sequence +
  48-bit random node (multicast bit set to signal random generation)
- `v7()`: 48-bit Unix millisecond timestamp in bytes 0-5 (sortable) +
  12 random bits (rand_a) + 62 random bits (rand_b)
- Package named `ca_uuid` (not `uuid`) to avoid Python stdlib name collision
- Fixed module-load ordering: `_parse_str` defined before the `NAMESPACE_*`
  constants that call `UUID("...")` at import time
- 91 tests, 95% line coverage
- Knuth-style literate programming: every function explains the algorithm,
  bit layouts, the Gregorian epoch derivation, and the RFC 4122 variant bits
