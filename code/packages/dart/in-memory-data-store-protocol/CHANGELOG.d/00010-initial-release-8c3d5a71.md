## 0.1.0

- Initial Dart lane of the in-memory-data-store-protocol identity (closes
  15/15): `CommandFrame` with uppercased names and copied byte args,
  RESP conversions both directions with null-on-reject semantics, scalar
  byte rendering, protocol errors, and text parse/format helpers.
- 12 tests: frames, conversions, rejections, round-trips, and a seeded
  256-frame sweep; 95% branch+function coverage gate.
