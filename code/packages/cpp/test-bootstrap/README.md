# test-bootstrap (C++)

An idiomatic, header-mostly **C++17 binding** over the C property-based testing
engine ([`c/test-bootstrap`](../../c/test-bootstrap/), spec VER05i). Spec:
`code/specs/VER05j-cpp-pbt-binding.md`. It replays the whole shared `pbt-v1`
corpus (`code/specs/fixtures/pbt-v1`, 25 cases) through the C++ surface.

It is a *binding*, not an engine: PRNG stream, shrink order, step counts, failure
ids, verdict honesty, budgets and caps all come from C, unchanged. A second engine
would be a duplicate witness (the TypeScript WD-901 problem), not an independent one.

## What the binding adds

- **RAII owners**: `Value` (one `pbt_val`; copy clones, move steals), `ValueView`
  (a non-owning window, what lambdas receive), `Gen` (a `pbt_gen` reference plus
  the user closures the C combinators point at), `Verdict`.
- **`std::` conversions**: `std::uint64_t` seeds, `std::vector<std::uint8_t>`
  bytes, `std::string`, `std::vector<Value>` lists, decimal text for wide ints.
- **Lambda builders**: `Gen::map` / `filter` / `bind`, `gen::one_of` / `frequency`
  / `list` / `tuple`, `Gen::custom`, `check_property` / `for_all`.
- **An exception boundary**: a C++ exception thrown by your lambda never crosses a
  C frame. Each C callback is a `noexcept` trampoline that catches everything and
  returns the engine's status, so a throwing generator, property, filter predicate
  or shrinker yields a **verdict** (`property-exhausted`, `property-panic`, the
  original witness), never a crash. `std::bad_alloc` maps to `PBT_E_NOMEM` where C
  has an out-of-memory path (map, bind, sampler, shrinker) and to the panic /
  error return where it does not (property, filter).

## Example

```cpp
#include "test_bootstrap.hpp"
namespace pbt = ca::pbt;

int main() {
    pbt::Gen xs = pbt::gen::list(pbt::gen::integer(0, 10), 0, 6);
    pbt::Options opts;
    opts.ci = pbt::CiMode::Off;  // the default REDACTS: no seed, no witness
    pbt::Verdict v = pbt::for_all(xs, [](pbt::ValueView list) {
        std::int64_t sum = 0;
        for (std::size_t i = 0; i < list.size(); ++i) sum += list.at(i).as_i64();
        return sum <= 20;  // true means the property holds
    }, /*seed=*/7, opts);
    if (!v.passed()) {
        // v.failure_id() == "property-counterexample"; v.minimal() is the shrunk witness
        return 1;
    }
}
```

## Build and test

`BUILD` runs `tools/run.sh`: for every available C++ compiler (`g++`, `clang++`) it
compiles the C engine with the C partner in strict ISO C17 and the tests in strict
ISO C++17 (`-pedantic-errors -Wall -Wextra -Werror`), runs three test programs
(`test_api`, `test_exceptions`, `test_corpus`), then repeats everything under
AddressSanitizer + UndefinedBehaviorSanitizer. `BUILD_windows` runs
`tools\run.ps1` (MSVC; written without a Windows host, not yet verified).

The corpus test shares only strict JSON decoding with C and never calls
`pbt_corpus_evaluate`, so a broken C++ layer fails it. It also proves it can fail:
per-op corruptions that must name exactly their case, structural corruptions, and
C++-layer sabotage seams (`ca::pbt::detail::seams`: shifted seed, dropped step,
swapped bounds).

## Where it sits

L2 of the VER testing pyramid (runtime generative testing), above the L0 harness.
Dependencies: the C++ standard library and `c/test-bootstrap`. No capabilities
(`required_capabilities.json` is empty).
