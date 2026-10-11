---
category: CI & GitHub Actions
---

# A new native build-tool fixture consumer changes both selection count and toolchain assertions

Adding Elixir to the exact discovery-fixture selection map made the focused Go
test fail: its build-plan assertion still expected eleven consumers, and its
toolchain list omitted Elixir. When registering a new direct native fixture
consumer, update the detector map, synthetic discovered roots, source-reference
scan suffixes, selected-package cardinality, and required-toolchain checks
together. Run the full focused selection test after the native consumer test;
passing only the native test does not prove CI will select it on fixture edits.
