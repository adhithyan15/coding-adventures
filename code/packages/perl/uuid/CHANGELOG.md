# Changelog — CodingAdventures::UUID (Perl)

All notable changes to this package are documented here.

## [0.01] — 2026-03-29

### Added

- `nil_uuid()` — returns the all-zeros UUID string
- `validate($uuid_str)` — validates canonical UUID format (8-4-4-4-12 hex)
- `parse($uuid_str)` — parses UUID into `{version, variant, bytes}` hashref
- `generate_v4()` — random UUID with 122 bits of randomness
- `generate_v1()` — time-based UUID with random node (RFC 4122 §4.5)
- `generate_v3($namespace, $name)` — MD5 name-based UUID; passes RFC 4122 test vectors
- `generate_v5($namespace, $name)` — SHA-1 name-based UUID; passes RFC 4122 test vectors
- `generate_v7()` — Unix epoch millisecond time-sortable UUID
- Namespace constants: `$NAMESPACE_DNS`, `$NAMESPACE_URL`, `$NAMESPACE_OID`, `$NAMESPACE_X500`
- Comprehensive Test2::V0 test suite including RFC 4122 Appendix B test vectors
