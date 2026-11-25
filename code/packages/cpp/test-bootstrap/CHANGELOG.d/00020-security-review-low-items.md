## 0.1.1 - security review LOW items

### Fixed
- Generator builders (`map`, `filter`, `bind`, `custom`, `one_of`, `frequency`, `list`,
  `tuple`) allocate the anchor vector before creating the C generator, so a
  `std::bad_alloc` no longer leaks it. New `test_alloc_throw` replaces global `operator new`
  and sweeps the failing allocation through every builder.
- The test seams (`detail::seams`) compile in only under `CA_PBT_TEST_SEAMS` (defined by
  `test_corpus.cpp` alone); production has no mutable global.
- `Gen::sample` runs in a fresh root budget (2^22 retries): stacked rare filters under direct
  sampling are bounded and end as `property-exhausted`.
- `sub_seed` rejects an id with an embedded NUL (`Error(PBT_E_ARG)`).
- A shrinker's step count is validated (negative, or beyond the remaining `max_steps`, is
  `PBT_E_ARG`), which also makes the running total overflow-safe.
- A temporary `Value` no longer converts to `ValueView` and has no `at()`.

### Documentation
- `ValueView`/`Scope`/`Falsifies` are call-scoped; `Gen` is not thread-safe; `tools/run.sh` fails
  fast on a repository path containing whitespace.
