# VER05j — C++ Property-Based Testing Binding

**Parent:** [VER05](VER05-property-engine.md), [VER05i](VER05i-c-pbt-engine.md) (the engine this binds), [VER05f](VER05f-kotlin-pbt-binding.md) (the binding pattern this mirrors), [TF00](TF00-portable-test-framework.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-CPP-PBT.

This specification defines the C++ face of the Level 2 Property-Based Testing
(PBT) engine. The package is `code/packages/cpp/test-bootstrap`: a header-mostly,
idiomatic C++17 **binding** over the C engine of VER05i
(`code/packages/c/test-bootstrap`). It adds RAII owners, `std::` conversions,
lambda-friendly builders and an exception boundary, and it replays the whole
shared `pbt-v1` corpus (25 cases) through that C++ surface.

The letter `j` follows VER05i (C), the highest letter in use on `origin/main`
and on every open branch at the time of writing.

---

## 1. Decision: bind, do not re-implement

C++ can link C directly. A second engine written in C++ would share the
platform's integer semantics and would have to re-derive the PRNG, the shrink
order, every step count, the budget tree and the caps. It would be a
*duplicate*, not an independent witness: any divergence between the two would
be a bug in one of them, never evidence about the language. The repo has already
paid for this once: the TypeScript lane carries two engines (recorded as
WD-901) that must be kept in lock-step by hand. VER05f made the same decision
for Kotlin over Java.

So C++ gets **one engine, shared with C**, and a binding that is as thin as
idiomatic C++ allows:

```text
 C++ consumers                                      conformance (test tree)
 ─────────────                                      ───────────────────────
 for_all(gen, [](ValueView v){...}, seed)           CppCorpusRunner::evaluate(json)
 check_property(seed, gen, prop, options)                   │ decodes with the C strict
        │                                                   │ JSON reader, builds Values
        ▼                                                   │ with the C++ constructors,
   Gen   (RAII over pbt_gen + closure anchors;              │ then calls ONLY the C++ API
          map/filter/bind/one_of/frequency/list/tuple)◄─────┘
   Value / ValueView / Verdict / Options / Rng / Scope
        │  trampolines: try { user lambda } catch (...) { status }
        ▼
   C engine (libpbt: pbt.h) — owns the PRNG stream, shrink order, step counts,
                              failure ids, verdict honesty, budgets and caps
```

What the binding **owns**: C++ types, argument order and defaults, ownership,
lifetime of user closures, the conversion of C++ exceptions to C status codes,
and a `Verdict` class.

What the binding **must not own**: any draw, any shrink move, any step count,
any failure-id decision, any budget or cap. Those are inherited from VER05i
unchanged (§9). A conversion that is a bijection (a `std::vector<std::uint8_t>`
to a `PBT_V_BYTES` value) may wrap an engine call; it may not alter what the
engine does.

The C engine is consumed as source: `tools/run.sh` compiles the C sources of
`code/packages/c/test-bootstrap/src` with the C compiler in strict ISO C17 and
links them with the C++ objects. No C source is copied or modified.

## 2. Ownership model (RAII)

| C++ type | Owns | Release |
|---|---|---|
| `Value` | one `pbt_val` | `pbt_val_free` in the destructor; move steals and leaves the source nil; copy is `pbt_val_clone` |
| `ValueView` | nothing (a `const pbt_val *`) | none; valid for the call that handed it out |
| `Gen` | one reference on a `pbt_gen` plus *anchors* (below) | `pbt_gen_release`; copy is `pbt_gen_retain` |
| `Verdict` | nothing from C: it copies the verdict out and calls `pbt_verdict_free` immediately | n/a |

**Anchors.** The C combinators (`pbt_gen_map`, `pbt_gen_filter`,
`pbt_gen_flat_map`) take a `void *self` context but never own it. The binding
allocates the context as a `std::shared_ptr` holding the user's
`std::function`, passes the raw pointer to C, and stores the `shared_ptr` in the
`Gen` it returns (the *anchor list*). Every combinator that wraps child
generators concatenates the children's anchor lists into its own. A `Gen`
therefore keeps every closure alive for as long as any copy of it exists, and a
closure outlives the C generator that points at it because the C generator is
released first (members are destroyed in reverse order: the `pbt_gen` reference
is dropped before the anchors).

**`bind` (flat map).** The C engine releases the generator a flat-map callback
returns right after sampling it. The produced `Gen` is a temporary of the
trampoline, so its anchors would die too early. The trampoline therefore wraps
the produced `Gen` in a tiny forwarding `pbt_gen_custom` (sample delegates to
`pbt_gen_sample`, no shrinker) whose `destroy` deletes the wrapper and with it
the produced `Gen`. Divergence: a bound generator is sampled one level deeper
than in C (`PBT_MAX_SAMPLE_DEPTH` counts the forwarder).

## 3. C++ API surface

Namespace `ca::pbt`, header `include/test_bootstrap.hpp` (one header, all
`inline`). The C headers are included inside `extern "C"`.

| C++ | Delegates to | Notes |
|---|---|---|
| `class Value` | `pbt_val_*` | factories `nil/boolean/integer(int64)/integer_u64/integer_dec(string)/bytes(vector)/string/list(vector<Value>)/tuple(...)`; accessors `kind/as_bool/as_i64/as_u64/to_u64_wrapping/decimal/as_bytes/as_string/size/at(i)/items/canonical`; `operator==` is `pbt_val_equal`; integers wider than 64 bits stay decimal strings (`pbt_big` caps apply) |
| `class ValueView` | the same accessors, non-owning | what lambdas receive |
| `class Rng` | `pbt_rng_init/next/bounded` | `Rng(std::uint64_t seed)` |
| `class Gen` | `pbt_gen` | `sample(Rng&)`, `shrink(Value&, falsifies, max_steps)`; `Gen::custom(sampler, shrinker?)` |
| `gen::boolean u8 u64 i128 integer(min,max) integer(min_dec,max_dec) bytes(min,max) constant(Value)` | `pbt_gen_*` | `u8()` yields int values 0..255 |
| `Gen::map(f)`, `filter(pred, max_retries = 100)`, `bind(f)` | `pbt_gen_map/filter/flat_map` | `map` and `bind` drop shrinking (no inverse), as in C |
| `gen::one_of(vector<Gen>)`, `frequency(vector<pair<uint64_t, Gen>>)`, `list(elem, min, max)`, `tuple(vector<Gen>)` | `pbt_gen_one_of/frequency/list/tuple` | tuple arity at most 8 |
| `struct Options { num_tests = 100; max_shrink_steps = 1000; CiMode ci = Unset; }`, `enum class CiMode` | `pbt_options` | the default still **redacts** (fail closed) |
| `class Verdict` | `pbt_verdict` | `passed()`, `failure_id()`, `message()`, `cases_run()`, `iteration()`, `shrink_steps()`, `status()`, `digest()`, `seed()` (a `std::optional`), `counterexample()` / `minimal()` (`std::optional<Value>`), `assert_passed()`; a redacted verdict has **no seed and no witness** |
| `check_property(seed, gen, prop, options = {}, shrinker = nullptr)`, `for_all(gen, prop, seed, options = {})` | `pbt_check` | `prop` returns `bool` (true holds) or `void` (holds unless it throws); it may also take a `Scope` second parameter for nested checks |
| `check_property(scope, seed, gen, prop, ...)` | `pbt_check` with a parent budget | the nested form |
| `shrink_int`, `shrink_list_ints`, `shrink_bytes`, `simulate_replay` | `pbt_shrink_*`, `pbt_simulate_replay` | return `Shrunk{minimal, steps}` / `Replay{initial_fail, minimal}`; `NotFalsifying` (an `Error`) when the initial value satisfies the property |
| `is_failure_id`, `tfv_valid`, `ci_from_value`, `sub_seed`, `fnv1a64` | `pbt_is_failure_id`, `pbt_tfv_validate`, `pbt_ci_from_value`, `pbt_sub_seed`, `pbt_fnv1a64` | |
| `class Error : std::runtime_error` | status codes | `status()` is the `PBT_E_*` code; `StatusError`, `NotFalsifying` derive from it |

Seeds are `std::uint64_t`. Constructors that the C engine refuses (`NULL`)
throw `Error`; entry points that report a status (`check_property`) never throw
for engine reasons: invalid options yield a failing `property-exhausted`
verdict with `status() == PBT_E_ARG`, never a pass.

## 4. The exception boundary

Letting a C++ exception propagate through C frames is undefined behaviour: the
C frames were compiled without unwind tables on some targets, never run their
cleanup, and leak the engine's allocations. **No exception thrown by a user
lambda ever crosses a C frame.** Every C callback the binding registers is a
trampoline of the form `try { user lambda } catch (...) { return status; }`
and the status mapping is part of the contract:

| callback | the lambda | C return convention | thrown `std::bad_alloc` | any other exception |
|---|---|---|---|---|
| property (`pbt_prop_fn`) | `bool(ValueView[, Scope])` | 1 holds, 0 fails, negative panics | `-1` (no OOM path in the property convention) | `-1`: `property-panic` |
| filter (`pbt_filter_fn`) | `bool(ValueView)` | 1 keep, 0 reject, negative error | `-1` (error) | `-1` (the engine reports `PBT_E_GEN`) |
| map (`pbt_map_fn`) | `Value(ValueView)` | 0 ok, positive status passes through | `PBT_E_NOMEM` | `PBT_E_GEN` |
| flat map (`pbt_flat_map_fn`) | `Gen(ValueView)` | 0 ok, positive status passes through | `PBT_E_NOMEM` | `PBT_E_GEN` |
| sampler (`pbt_sample_fn`) | `Value(Run &)` | 0 ok, `PBT_E_*` | `PBT_E_NOMEM` | `PBT_E_GEN`; a `StatusError` carries its own code (this is how `Run::sample` reports an inner failure) |
| shrinker (`pbt_shrink_fn`) | `int(Value &, const Falsifies &, int max_steps)` | 0 ok | `PBT_E_NOMEM` | `PBT_E_GEN` |

The engine already gives a failing sampler `property-exhausted`, a negative
property `property-panic`, and a failing shrinker "report the original
candidate". So a throwing generator, property, filter predicate or custom
shrinker yields a **verdict**, never a crash and never a pass. A throwing
*shrinker* keeps the original witness (the engine re-verifies and discards the
shrinker's value); the shrinker's `Value &` is a view of the engine's slot, and
a guard moves it back on unwind so the slot is always valid and freeable.

The binding is single-threaded by contract (VER05i §1); `catch (...)` would
also swallow a forced thread unwind, and cancelling a property is unsupported.

Further contracts (security review, LOW items): builders allocate the anchor
list *before* creating the C generator, so a `std::bad_alloc` cannot leak it;
`Gen::sample` runs in a fresh root budget capped at 2^22 filter retries, so
stacked rare filters end as `property-exhausted` instead of hanging; a shrinker's
reported step count is validated (`0 <= n <= max_steps - steps so far`, else
`PBT_E_ARG`) before it is added; `sub_seed` throws `Error(PBT_E_ARG)` for an id
with an embedded NUL; `ValueView`s, `Scope` and `Falsifies` are call-scoped (a
temporary `Value` neither converts to a view nor supports `at()`); `Gen` is not
thread-safe.

## 5. Corpus replay goes through C++

`CppCorpusRunner` (test tree, `tests/corpus_runner.hpp`) shares only the
*decoding* primitives with C: `pbt_json_parse` for the strict JSON text. It
never calls `pbt_corpus_evaluate`. Every other step builds `Value`s with the
C++ factories and calls the C++ API:

| op | C++ call |
|---|---|
| `gen_primitive` / `gen_bounded` / `gen_bytes` | `gen::boolean/u8/u64/i128`, `gen::integer`, `gen::bytes` + `Gen::sample(Rng &)` |
| `shrink_int` / `shrink_refinement` | `shrink_int` |
| `shrink_list` | `shrink_list_ints` |
| `shrink_bytes` | `shrink_bytes` |
| `replay_consistency` | `simulate_replay` |
| `filter_exhausted` | `check_property` over `gen::u8().filter(never, max_retries)`; verdict must be failing, id `property-exhausted`, predicate called exactly `max_retries` times |
| `failure_id_valid` | `is_failure_id` |
| `corpus_reject` | `tfv_valid` must be false |

It asserts exact outputs, exact minimals and exact step counts, requires all 11
ops and all four failure ids, requires an unregistered-id rejection, and
requires exactly 25 cases. Every reported problem starts with the case id. The
named corpus properties (`int_le_100`, `list_sum_le_20`, ...) are C++ lambdas
in the runner; they are the test's own definition of the property, and the
C package's table is never consulted.

## 6. Negative controls and the C++-layer sabotage seam

Per VER05i §8, re-run against the C++ runner:

* corrupting **every** case in every way its op allows fails the run, a
  problem names exactly that case id, and no other case id is named;
* dropping any case, truncating the document, a duplicate id, an unknown op,
  extra and missing fields and a wrong schema version are problems.

Additionally a **C++-layer seam**, `ca::pbt::detail::seams` (an `inline`
struct that exists only when a translation unit defines `CA_PBT_TEST_SEAMS`
before including the header; without it `seeded`/`stepped` are the identity and
there is no global), lets a test make the binding itself wrong. Only
`test_corpus.cpp` defines it:

* `seed_offset` — every seed handed to the engine becomes `seed + offset`;
* `shrink_step_delta` — every step count the binding reports is `steps + delta`;
* `swap_bounds` — `gen::integer(min, max)` swaps its bounds before delegating.

A test sets each seam, runs the corpus through C++ and requires problems,
proving that the corpus run really exercises the C++ layer; a reset restores
production behaviour and the corpus is green again. (The C engine's own
`pbt_hooks` seams stay the C package's business.)

## 7. Exception-safety and OOM tests

* A lambda that throws (a `std::runtime_error`, a non-`std` type, and a
  `std::bad_alloc`) inside generation (`map`, `bind`, a custom sampler), the
  property, a filter predicate and a custom shrinker produces a verdict with the
  id of §4, and the process continues.
* After each scenario the C engine's live-allocation counter
  (`pbt_hooks.live_allocs`) is back at its baseline: RAII released everything,
  including on unwind. Under ASan this also proves no use-after-free.
* The C OOM seam (`pbt_hooks.fail_after`) is swept through the *binding*
  (build a generator tree, run a check, build and clone values): for N =
  1, 2, ... every run either throws `Error` or returns a verdict, never claims
  success once an allocation failed in a strict scenario, and leaks nothing.

## 8. Build and Inputs

* `BUILD` is `# build-tool: deps=c/iso-harness c/test-bootstrap` plus
  `sh tools/run.sh`; `BUILD_windows` runs `tools\run.ps1`. The `c/test-bootstrap`
  dep makes a change to the C engine rebuild this package.
* `tools/run.sh` takes every available C++ compiler (`g++`, `clang++`) with its
  C partner (`gcc`, `clang`): the C engine is compiled with the harness's strict
  C flags (`-std=c17 -pedantic-errors -Wall -Wextra -Werror`), the tests with the
  strict C++ flags (`-std=c++17 -pedantic-errors -Wall -Wextra -Werror`), and
  they are linked by the C++ driver. Then every test is built a second time with
  `-fsanitize=address,undefined -fno-sanitize-recover=all` when the compiler can
  link the runtime. On Linux CI `gcc` and `clang` are both required.
* The corpus is read from `code/specs/fixtures/pbt-v1/cases.json` (never a
  copy). The Go build tool's `repository_inputs.go` has a `cpp/test-bootstrap`
  consumer row so a corpus edit selects this package; a missing consumer fails
  closed.
* `required_capabilities.json`: empty. The library performs no I/O; only the
  tests read the committed fixture.
* Zero third-party dependencies: the C++ standard library and the C engine.

## 9. Inherited from VER05i (unchanged, not re-specified)

PRNG stream and bounded-draw rules; draw order of every generator; shrink
phases, order and step counting; failure ids and the closed vocabulary;
verdict honesty (zero cases is `property-exhausted`, a generator error is
`property-exhausted`, a negative property is `property-panic`, the reported
witness is re-verified); CI redaction that fails closed and the unkeyed
`repro-fnv` caveat; `property-timeout` reserved and never produced; the shrink
budget tree; every cap (`PBT_MAX_*`, `PBT_BIG_MAX_BITS`, CWE-400 bounds).

## 10. Divergences

* No independent engine (§1). Deliberate; this is why VER05j is a binding.
* Properties are `bool`-returning lambdas (true holds), the Java/Go convention,
  not C's `1/0/negative`; a throw is the C `negative`.
* `Verdict` copies out of the C verdict; the seed and witnesses are
  `std::optional`. A witness is present exactly when the verdict is a
  non-redacted `property-counterexample` (so `has_seed` is not "not redacted").
* A bound generator is sampled one forwarding level deeper (§2).
* `std::bad_alloc` has no OOM path in the property and filter conventions, so it
  is a panic / generator error there (§4).
* The C++ binding never catches an engine abort or crash; as in VER05i §12 #15
  a crash inside C is not recoverable. Only C++ exceptions are mapped.
* `repro-fnv` digests of tuple witnesses are the C engine's; cross-language
  digest equality is not claimed.
