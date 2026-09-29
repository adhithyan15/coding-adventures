### Fixed -- list story fixtures reach the generated app (#15428)

The generated `mosaicStringList` and `mosaicStringListList` readers ignored
fallbacks entirely and returned an empty list when the host had not sent the
slot. So even a supported fixture could never show on Flutter.

- **Readers:** both now take an optional positional fallback.
- **Call sites:** `host_value_for_slot` passes one only when a text-list
  fixture applies, so output for fixture-less builds is byte-identical.
- **Literals:** `dart_literal_for_fixture` renders the fixture as a `const`
  typed list.

Checked with `dart analyze` on an emitted SegmentedControl story: the call,
its `const <List<String>>` fallback and a `$` escaped as `\$` all resolve.

