# test-bootstrap (C++)

Two bindings over the C engines of [`c/test-bootstrap`](../../c/test-bootstrap/): a property-based
testing binding (`ca::pbt`, below) and a coverage-guided fuzz binding (`ca::fuzz`, see
[Fuzzing](#fuzzing-ca-fuzz)).

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
ISO C++17 (`-pedantic-errors -Wall -Wextra -Werror`), runs the test programs
(`test_api`, `test_exceptions`, `test_alloc_throw`, `test_corpus` and the six `test_fuzz_*`), then repeats everything under
AddressSanitizer + UndefinedBehaviorSanitizer. `BUILD_windows` runs
`tools\run.ps1` (MSVC; written without a Windows host, not yet verified).

The corpus test shares only strict JSON decoding with C and never calls
`pbt_corpus_evaluate`, so a broken C++ layer fails it. It also proves it can fail:
per-op corruptions that must name exactly their case, structural corruptions, and
C++-layer sabotage seams (`ca::pbt::detail::seams`: shifted seed, dropped step,
swapped bounds).

## Fuzzing (`ca::fuzz`)

A header-mostly **C++17 binding** over the C coverage-guided fuzz engine (spec
`code/specs/VER06k-cpp-fuzz-binding.md`, engine VER06j), in `include/fuzz_bootstrap.hpp`. Policy
WD-901: C++ binds over C, there is no second engine. Mutators, signature hash, minimiser, power
schedule, budgets and the TAP grammar are the C engine's, unchanged.

```cpp
#include "fuzz_bootstrap.hpp"
namespace fz = ca::fuzz;

int main() {
    fz::Options opts;
    opts.max_iterations = 2000;
    fz::Report r = fz::run({fz::Bytes{'A', 'B', 'C'}}, opts, [](fz::ByteView d, fz::Handle& h) {
        h.edge(1);                                    // caller-reported coverage
        if (d.size() > 0 && d[0] == 0xFF) throw fz::Crash("ff", {"parse"});
    });
    for (const std::string& line : r.tap_failures("my_target")) { /* not ok 1 - my_target [fuzz-crash:...] */ }
}
```

- **Types**: `ByteView` (a C++17 stand-in for `std::span`; accepts one under C++20), `Bytes`,
  `BigInt` (the corpus carries integers wider than 64 bits), `Endian`; every mutator has an
  `int64_t` and a `BigInt` form.
- **Exception boundary**: nothing a target throws crosses a C frame. Each callback is a `noexcept`
  trampoline classifying by **type, never `what()`**: `ca::fuzz::Crash(kind, frames)` is a crash with
  that kind and frames, `std::bad_alloc` is an **OOM** (counted, never a signature), any other
  `std::exception` is a crash of kind `typeid(e).name()`, anything else `non-std-exception`.
- **RAII**: `Report` is move-only and owns the C report; `Handle` is move-only and **inert** when moved
  from, after its execution returned, after its ring slot was reissued (epoch check) and off the
  creating thread. Single-threaded by contract, like the C engine.
- **Privacy**: no seed is stored and no `operator<<` exists; private bytes are reached only through
  `CrashRecord::minimal_input()` and `Report::corpus()`.
- **Conformance**: the whole fuzz-v1 corpus (25 cases) and the 820 Go reference vectors are replayed
  **through the C++ API** (`test_fuzz_corpus`, `test_fuzz_vectors`; never `fz_corpus_evaluate`), with
  per-case and per-row corruption controls that must name exactly their case, and C++-layer sabotage
  seams (`ca::fuzz::detail::seams`, only under `CA_FUZZ_TEST_SEAMS`). `test_fuzz_pinned` pins
  signatures, energy ties, TAP tokens and the minimiser trace; `test_fuzz_exceptions`,
  `test_fuzz_handle` and `test_fuzz_alloc` cover the boundary, the handle and allocation failure.

## Where it sits

L2 of the VER testing pyramid (runtime generative testing), above the L0 harness.
Dependencies: the C++ standard library and `c/test-bootstrap`. No capabilities
(`required_capabilities.json` is empty).
