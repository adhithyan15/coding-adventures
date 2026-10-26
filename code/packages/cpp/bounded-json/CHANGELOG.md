# Changelog

## 0.1.0 — NN50 (unreleased)

- Add an independent native ISO C++17 JSON parser with RAII document ownership,
  immutable typed views, ordered duplicate/NUL keys and lexical int64 metadata.
- Bound bytes, depth, nodes and actual parser-owned allocation requests; inject
  allocation faults through an upstream memory resource and release partial
  trees iteratively.
- Validate strict JSON/UTF-8, escapes/surrogates, finite locale-independent
  numbers, malformed input, bounds, ownership and every observed allocation
  failure position. Add a second translation-unit public-header consumer.
- Reuse shared ISO compiler fronts and add CMake header installation metadata.
- Initial GCC/Clang local suites pass; GCC library line coverage is 98.71%.
  Clang ASan and actual installed CMake consumer compile/link/run also pass.
  Native Windows compiler validation is blocked by a missing SDK header;
  UBSan and exact-head CI remain pending. This package does not implement
  checkpoint schemas or training.
