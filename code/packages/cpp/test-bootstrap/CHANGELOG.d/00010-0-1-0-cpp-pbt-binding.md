## 0.1.0 - pbt-v1 conformance through a C++ binding

First release of the C++ test-bootstrap package (spec:
`code/specs/VER05j-cpp-pbt-binding.md`). A header-mostly C++17 binding over the
C engine of `c/test-bootstrap` (VER05i); there is deliberately no second engine.

### Added
- `include/test_bootstrap.hpp` (namespace `ca::pbt`): `Value` / `ValueView`
  (RAII over `pbt_val`, `std::vector` / `std::string` conversions, decimal text for
  wide integers), `Rng`, `Gen` (RAII over `pbt_gen` plus closure anchors) with
  `map` / `filter` / `bind` and `gen::one_of` / `frequency` / `list` / `tuple` /
  `constant` / primitives / `Gen::custom`, `Options`, `Verdict` (no seed and no
  witness when redacted), `check_property` / `for_all` (lambdas, including the
  nested `Scope` form and a custom shrinker override), `shrink_int` /
  `shrink_list_ints` / `shrink_bytes` / `simulate_replay`, and small pure helpers.
- The exception boundary: every C callback is a `noexcept` trampoline
  `try { user lambda } catch (...) { status }`, so no exception crosses a C
  frame. `std::bad_alloc` maps to `PBT_E_NOMEM` where the C convention has an OOM
  path and to the panic / error return where it does not.
- `tests/test_corpus.cpp` + `tests/corpus_runner.hpp`: the whole `pbt-v1` corpus
  (25/25, exact outputs, minimals and step counts) replayed through the C++ API,
  never through `pbt_corpus_evaluate`; 43 per-op corruption controls and 252
  wrong-type-field controls (each must fail and name exactly its own case),
  dropped / truncated / empty / duplicate / unknown-op / extra-field / wrong-version
  controls, hostile sizes, and C++-layer sabotage seams (`seed_offset`,
  `shrink_step_delta`, `swap_bounds`) that must make the corpus fail.
- `tests/test_exceptions.cpp`: a std exception, a non-std type and `std::bad_alloc`
  thrown in generation (`map`, `bind`, sampler), the property, a filter predicate and
  a custom shrinker each yield a verdict with the documented failure id and leave
  `pbt_hooks.live_allocs` at its baseline; the C OOM seam swept through the binding.
- `tests/test_api.cpp`: API unit tests.
- `tools/run.sh`: every available C++ compiler with its C partner (strict ISO
  C++17 / C17, `-pedantic-errors -Wall -Wextra -Werror`), then an ASan + UBSan pass.
  `tools/run.ps1` for MSVC (authored without a Windows host; unverified).
