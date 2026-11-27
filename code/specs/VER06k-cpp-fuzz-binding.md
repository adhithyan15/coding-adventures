# VER06k — C++ Coverage-Guided Fuzz Binding

**Parent:** [VER06](VER06-coverage-guided-fuzz-engine.md), [VER06j](VER06j-c-fuzz-engine.md) (the engine this binds), [VER05j](VER05j-cpp-pbt-binding.md) (the sibling PBT binding in the same package), [VER06g](VER06g-kotlin-fuzz-binding.md) and VER06m (F#; the binding pattern this follows), [TF00](TF00-portable-test-framework.md). Plan: [polyglot-testing-tooling](VER-BACKLOG.d/polyglot-testing-tooling.md), item PT-CPP-FUZZ.

This specification defines the C++ face of the Level 2 coverage-guided fuzz
engine. It lives in the existing package `code/packages/cpp/test-bootstrap`, next
to the PBT binding of VER05j: a header-mostly, idiomatic C++17 **binding** over the
C engine of VER06j (`code/packages/c/test-bootstrap`, `include/fuzz.h`). It replays
the whole shared `fuzz-v1` corpus (25 cases) and the 820 Go reference vectors
through that C++ surface.

The letter `k` is the one VER06m reserved for C++ (VER06j is C, VER06l is Swift).

---

## 1. Decision: bind, do not re-implement

Policy WD-901: Kotlin binds over Java, F# over C#, **C++ over C**. C++ can link C
directly. A second engine in C++ would have to re-derive the mutator algebra, the
draw order of `mutate_sequence`, the power schedule, the signature hash, the
minimiser, the budgets and the TAP grammar. It would be a *duplicate*, not an
independent witness: any divergence would be a bug in one of them, never evidence
about the language.

```text
 C++ consumers                                      conformance (test tree)
 ─────────────                                      ───────────────────────
 ca::fuzz::run(seeds, options, target)              FuzzCorpusRunner::evaluate(json)
 ca::fuzz::bit_flip(view, off, count) ...           vector replay (Go reference table)
        │                                                   │ decode with the C strict JSON
        ▼                                                   │ reader, then call ONLY the C++
   Handle / Options / Report / CrashRecord / BigInt  ◄──────┘ API below
   trampolines: noexcept, try { user code } catch (...) { classify }
        │  adapts types, owns lifetimes, translates exceptions
        ▼
   C engine (fuzz.h): mutators, sig_hash, dedup, minimiser, power schedule,
                      handle ring, budgets, TAP grammar, failure ids, every cap
```

The binding **owns**: C++ types and defaults, `ByteView` / `std::string_view`
inputs, `std::vector` outputs, RAII for everything the C engine hands out, the
handle wrapper, and the translation of C++ exceptions into the engine's crash
classification. It **must not own** any mutation, admission, energy, signature,
dedup decision, minimisation step, budget, cap, failure id or TAP character.

The C engine is consumed as source, as in VER05j §8: `tools/run.sh` compiles
`c/test-bootstrap/src/fuzz_*.c` with the C compiler in strict ISO C17 and links
them with the C++ objects. No C source is copied or modified.

## 2. API surface

Namespace `ca::fuzz`, header `include/fuzz_bootstrap.hpp` (one header, all
`inline`; the C headers are included inside `extern "C"`).

### 2.1 Input and output types

| C++ | Meaning |
|---|---|
| `class ByteView` | a non-owning `(const uint8_t *, size_t)` window; implicit from `const Bytes &`, `std::string_view` (its bytes), pointer + size, and `std::span<const uint8_t>` when the library has one |
| `using Bytes = std::vector<uint8_t>` | every owned byte result |
| `class BigInt` | one `pbt_big` (up to 1,000 bits / 300 digits): `BigInt(int64_t)` explicit, `from_u64`, `parse(string_view)`, `fits_i64`, `to_i64`, `mod_u64`. The corpus carries integers wider than 64 bits |
| `enum class Endian { Little, Big }` | `fz_endian` |

**Divergence: `std::span`.** `std::span` is C++20 and the iso-harness pins
`-std=c++17 -pedantic-errors` (lesson: *`std::span` isn't C++17*). `ByteView` is the
C++17 stand-in with the same shape; under C++20 it also accepts a
`std::span<const uint8_t>`. Text inputs (targets, kinds, frames, ids, signatures,
decimal integers) are `std::string_view` and may hold embedded NULs.

### 2.2 Mutators and pure helpers

Free functions returning `Bytes`, each with an `int64_t` form and a `BigInt` form:
`bit_flip`, `byte_flip`, `byte_arithmetic`, `boundary_insert`, `block_delete`,
`block_insert`, `block_duplicate`, `splice`, `mutate_sequence`.
`mutate_sequence_budget(...)` returns `SequenceResult { status, bytes, work_done }`
because the engine writes `work_done` on every return, `FZ_E_WORK` included.
Also: `interest_values`, `interest8`, `deltas8`, `calculate_energy`, `edge_id`,
`corpus_admit -> Novelty`, `crash_signature`, `is_duplicate_crash`, `is_failure_id`,
`format_tap_failure`, `tfv_valid`.

### 2.3 Errors: exceptions versus codes

* A call whose C counterpart returns a failing `FZ_E_*` status throws
  `ca::fuzz::Error` (`status()` is the engine's code). The one exception is
  `mutate_sequence_budget` (above).
* Findings are values, never exceptions: a campaign that finds crashes returns a
  `Report`. A refused TAP request throws `Error(FZ_E_ARG)`.
* Nothing the **user's code** throws escapes a campaign (§3).

### 2.4 Campaign

| C++ | Delegates to | Notes |
|---|---|---|
| `struct Options` | `fz_options` | defaults come from `fz_options_default()`. No `user` field: closures capture state; the binding owns `fz_options.user` |
| `using Target = std::function<void(ByteView, Handle &)>` | `fz_target_fn` | normal return = pass; a throw = §3 |
| `Report run(seeds, Options, Target)` | `fz_run` | the engine validates (`Error(FZ_E_ARG)`) |
| `Bytes minimize(ByteView, sig, MinimizeTest, budget)` | `fz_minimize_crashing_input` | `MinimizeTest = std::function<Outcome(ByteView)>` |
| `struct Outcome` | `fz_outcome` | `pass()`, `crash(kind, frames)`, `timeout()`, `oom()` |

### 2.5 Views and privacy

`Report` and `CrashRecord` expose counts, tier, cleanliness, signatures, kinds,
frames, `summary()` and `tap_failures(target, first_test)` (the engine's text). The
campaign seed is not stored anywhere, and no `operator<<` exists. Private bytes are
reachable only through explicit named calls returning copies:
`CrashRecord::minimal_input()` and `Report::corpus()`.

## 3. Exception boundary and classification

A C++ exception that unwinds through a C frame is undefined behaviour. **No
exception thrown by user code ever crosses a C frame.** Every C callback the binding
registers is a `noexcept` trampoline `try { user code } catch (...) { classify }`.
**Classification is by the exception's TYPE, never by `what()`**: a message can carry
input bytes or secrets and is the part that varies between occurrences of one bug.

| thrown by the target | outcome | crash `kind` | frames |
|---|---|---|---|
| `ca::fuzz::Crash(kind, frames)` | `FZ_CRASH` | the declared kind | the declared frames |
| `std::bad_alloc` (and subclasses) | `FZ_OOM` | none: counted, never a signature | none |
| any other `std::exception` | `FZ_CRASH` | `typeid(e).name()`, normalised by the engine | none |
| anything else | `FZ_CRASH` | `non-std-exception` | none |

* `bad_alloc` is an OOM, as in the other lanes. The trampoline reaches the engine's
  OOM state by calling `fz_reserve(h, SIZE_MAX)` (always refused, sticky), so the C
  executor classifies the execution itself. A handle already exhausted by a timeout
  stays a timeout (first budget wins).
* The engine's frame rules apply on top (sanitised charset, caps).
* `typeid().name()` is implementation-defined; cross-compiler signature equality is
  not claimed. C++ has no portable stack trace, so a typeid crash has no frames and two
  throw sites of one type deduplicate; a target wanting site-level signatures throws
  `Crash` with its own frames.
* An exception thrown by a `MinimizeTest` is classified the same way (`bad_alloc`
  becomes the never-reproducing `oom` outcome).
* A C status becomes `Error` only after the C call returned, outside every C frame.
* `catch (...)` also swallows a forced thread unwind; cancelling a target thread is
  unsupported.

## 4. Lifetime and ownership across the C boundary

| C++ | Owns | Rule |
|---|---|---|
| `OwnedBytes` (internal) | one `fz_bytes` | freed in the destructor, so a `bad_alloc` while copying leaks nothing |
| `Report` | one `fz_report` | `fz_report_free` in the destructor; **non-copyable, move-only**; moved-from is empty |
| `Handle` | nothing | a window onto one execution |
| campaign context (internal) | the `Target` during `run` | passed as `fz_options.user`, read back by `fz_handle_user` |

**`Handle`.** The C engine issues one handle per execution from a static ring and
retires it when the target returns. The wrapper adds, without re-implementing engine
logic:

1. **Move-only.** A moved-from `Handle` is **inert** (`alive()` false; `edge` no-op;
   `tick`/`reserve` report exhaustion).
2. **Cannot outlive the execution.** It records the epoch (`fz_handle_epoch`, never
   reused) and re-checks liveness and epoch on every call. After the target returns
   the C handle is retired, so a retained wrapper is inert; after the ring slot is
   reissued the epoch differs, so C's documented 4,096-slot aliasing window cannot
   attribute a late call to a later execution through C++. There is no
   use-after-free: the slots are static and the wrapper frees nothing.
3. **Thread affine.** Calls from a thread other than the creating one are inert and
   never touch the unsynchronised C state.

`tick()` / `reserve()` return **true when the budget is exhausted** (as C returns
nonzero); exhaustion is sticky and shared (VER06j §5.2).

A `ByteView` given to a target points into a heap copy the engine made (VER06j §5.4):
it is call-scoped; keeping it is use-after-free, as in C. Copy to `Bytes` to keep.

## 5. Thread-safety

Single-threaded by contract, like the C engine: the handle ring, coverage map,
`pbt_hooks`, `fz_hooks` and the allocation counter are unsynchronised globals. The
binding does not claim concurrent calls are safe and adds no lock. It adds: thread
affine inert `Handle` (§4), so a helper thread cannot race the engine; threads must be
joined before the target returns; campaigns may be nested on one thread.

## 6. Corpus replay goes through C++

`tests/fuzz_corpus_runner.hpp` is a second harness for
`code/specs/fixtures/fuzz-v1/cases.json`. It shares only decoding with C
(`pbt_json_parse`) and **never calls `fz_corpus_evaluate`** or any `fz_*` function.
Every op is answered by the C++ API:

| corpus op | C++ call |
|---|---|
| `mutate_*` | the §2.2 mutators over `BigInt` (`mutate_sequence` via `mutate_sequence_budget` with the shared 2^24-byte aggregate work cap) |
| `corpus_admit` | `corpus_admit` |
| `crash_dedup` | `is_duplicate_crash`, and `crash_signature(a) == crash_signature(b)` must agree with it (8 lowercase hex each) |
| `failure_id_valid` | `is_failure_id` |
| `private_reporting` | `format_tap_failure` |
| `corpus_reject` | `tfv_valid` must be false |

It asserts exact outputs and the engine invariants the C runner checks, requires all
14 ops, an accepted and a rejected id, every registered id accepted and exactly 25
cases. Every problem starts with the case id; sizes are capped at 10,000; problems are
clipped to 400 bytes.

The **820 Go reference vectors** (`code/packages/go/test-bootstrap/fuzz/reference_vectors_test.go`,
read from its committed path) are replayed through the `int64` and the `BigInt`
overloads, each problem naming `vector N (op)`. Additionally pinned through the C++
API (values from the C lane's tests): signatures, energy rows with ties, every
interest and delta constant, splice blind spots, TAP tokens (`a#b` to `a_b`, the
128-byte cut, empty), and the minimiser's call sequence.

## 7. Negative controls and the C++-layer sabotage seam

* corrupting every case in every way its op allows, and every field of every case with
  the wrong type, fails the run and names exactly that case id;
* dropped/truncated corpora, duplicate ids, unknown ops, extra/missing fields and a
  wrong schema version are problems;
* corrupting any vector row fails and names that row; a short table fails.

`ca::fuzz::detail::seams` exists only when `CA_FUZZ_TEST_SEAMS` is defined (without it
every hook is the identity and there is no global) and breaks the *wrapper*:
`offset_delta`, `swap_endian`, `seed_offset`, `invert_dedup`, `leak_tap_target`,
`admit_everything`, `accept_any_failure_id`. A test sets each, replays the corpus and
requires problems, then resets and requires a clean 25/25 run.

## 8. Build and inputs

* `BUILD` / `BUILD_windows` keep their shape: `tools/run.sh` / `tools\run.ps1` build the
  C engine (now including `fuzz_*.c`) per compiler with its **C** flags and the tests
  with its **C++** flags, then repeat under ASan + UBSan (POSIX). On Windows the C++
  flags never reach a `.c` file (`Iso-StrictFlags -Lang c` vs `-Lang cpp`).
* `fuzz_sancov.c` (not ISO C) and `fuzz_corpus.c` (the C lane's runner) are not linked.
* The build tool's `repository_inputs.go` gains `cpp/test-bootstrap` rows for the
  fuzz-v1 fixture and the Go vector table (fail closed).
* `required_capabilities.json` stays empty.

## 9. Inherited from VER06j (not re-specified)

Mutator algebra and saturation; bitmap, admission, energy; sticky shared budgets;
handle retirement and ring; crash normalisation, signature, dedup; minimisation
budgets; retained-bytes and crash caps; failure-id vocabulary; TAP sanitising; honest
`is_clean`; the open items of VER06j §10.

## 10. Divergences

| # | Item |
|---|---|
| 1 | No independent engine (§1). Deliberate |
| 2 | `ByteView` instead of `std::span` under the C++17 pin |
| 3 | `bad_alloc` is an OOM outcome, other exceptions crash by type (§3) |
| 4 | `Handle` adds epoch and thread checks over C's retired-handle rule |
| 5 | `fz_run_outcomes` and `fuzz_hooks.h` are not bound |
| 6 | A C++ crash has no frames unless the target declares them |
