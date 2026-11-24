# Polyglot testing tooling: L0 harness, PBT and fuzz in every lane

**Authority:** current matrix, plan and status for bringing the testing
engines to every language lane: the three core engines (the L0 harness ([TF01](../TF01-bootstrap-harness.md)
`test-bootstrap`), the property engine ([VER05](../VER05-property-engine.md))
and the coverage-guided fuzz engine ([VER06](../VER06-coverage-guided-fuzz-engine.md)))
plus the "others" added 2026-11-24: mutation testing, microbenchmarks and a
uniform coverage gate. The main [VER-BACKLOG](../VER-BACKLOG.md) keeps a one-line pointer; this shard
owns the detail.

Owner direction, 2026-11-23: "bring Lean like testing along with fuzzing, unit
testing, property testing and others to every language as every language
deserves such a tooling".

Owner request, 2026-11-24 (PT-001): "Add property based testing library and
others for all the languages that do not have them to the backlog". The
per-lane rows below are the answer: one row per lane per engine, so a lane's
absence from a tool is a visible, schedulable line and never a footnote.

## What "done" means for one lane and one engine

A lane's engine counts as **delivered** only when all of these hold:

1. A per-language child spec (VER05x for PBT, VER06x for fuzz, TF01x for L0)
   is merged before or with the code, following VER05a/VER05b and VER06b/VER06c.
2. Zero third-party runtime dependencies. Test-only tools the lane's BUILD
   already uses (pytest, ruff, `go vet`) are allowed.
3. The lane replays the **whole** shared corpus, `code/specs/fixtures/pbt-v1`
   or `code/specs/fixtures/fuzz-v1` (25 cases each), from the committed fixture
   file, not from a copy, and asserts every case: exact outputs, exact shrink
   minimals **and step counts**, exact dedup hashes.
4. **Negative controls.** A sabotage mode or falsification tests prove that the
   corpus run can fail: corrupting each case category's expected value makes
   the run fail, and the run also fails if it evaluated fewer cases than the
   fixture holds. A corpus check that cannot fail is the LH-003 false pass this
   repo has been burned by before.
5. PBT: the generator algebra of VER05 section 3, integrated shrinking of
   section 4, deterministic replay with sub-seeds (section 2.3), the closed
   failure vocabulary, and the verdict-honesty rules of section 7.1 (zero cases
   run is never a pass; the reported witness is re-verified; CI messages carry
   no concrete value).
6. Fuzz: the mutator algebra of VER06 section 3, coverage-bitmap admission
   (4.2), the power schedule (4.3), signature dedup (5.2), minimization (5.3),
   opaque TAP reporting (6), and the campaign contracts of section 9.1 (honest
   `is_clean`, timeouts and OOMs counted, determinism). The coverage source
   is the lane's dependency-free mechanism from VER06 section 7, or the run is
   labelled `black-box-uninstrumented`.
7. Coverage of the engine's own tests well above 80% (target 95%), literate
   source, BUILD and BUILD_windows, README, CHANGELOG.d shard.

"Has an engine" without item 3 and 4 is recorded below as **partial**.

## Matrix (re-surveyed 2026-11-24 on `origin/main` `7da67070ca0`)

Re-survey method: `ls code/packages/<lane>` for `test-bootstrap`, `pbt`,
`fuzz`, `*bench*`, `*mutation*`, `warrant-corpus`; merged and open PRs through
`gh pr list --search "pbt OR fuzz OR test-bootstrap"`. Changes since
2026-11-23: none to the cells below (Go PBT #19891 and Go fuzz #19898 were
already recorded as delivered); no open PR for Java or any other new lane was
found, so PT-JAVA-PBT is still unstarted. Anything not directly verified is
marked "unverified" in the tables. Package counts are refreshed.

Packages per lane is the number of directories under `code/packages/<lane>/`,
used as a proxy for how much of the repo each engine would serve.

| Lane | Packages | L0 harness (TF01) | PBT (VER05) | pbt-v1 | Fuzz (VER06) | fuzz-v1 |
|---|---:|---|---|---|---|---|
| rust | 1552 | yes, `rust/test-bootstrap` (VER-P003) | yes, in `test-bootstrap` (VER-P010) | pass (`tests/pbt_corpus.rs`) | yes, in `test-bootstrap` (VER-P014) | pass (`tests/fuzz_corpus.rs`) |
| python | 523 | yes, `python/test-bootstrap` (PRNG, TFV, evidence; no TF01 child spec) | yes, `test-bootstrap` `pbt` (VER05c) | pass (`tests/test_pbt_corpus.py`, #19881) | yes, `test-bootstrap` `fuzz` (VER06d), `sys.monitoring` / `sys.settrace` coverage | pass (`tests/test_fuzz_corpus.py`) |
| typescript | 501 | yes, `typescript/test-bootstrap` (TF01a, TF01b) | yes, `typescript/pbt` (VER05a) and a copy in `test-bootstrap` | pass | yes, `typescript/fuzz` (VER06b) and a copy in `test-bootstrap` | pass |
| ruby | 308 | no | no | - | no | - |
| go | 308 | partial: `go/test-bootstrap` holds the shared `prng` (no TF01 runner yet; `go/warrant-corpus` is a Warrant corpus runner) | yes, `test-bootstrap/pbt` (VER05d) | pass (`pbt/corpus_test.go`, #19891) | yes, `test-bootstrap/fuzz` (VER06e), caller-reported edges (`Handle.Edge`) | pass (`fuzz/corpus_test.go`, #19898) |
| elixir | 286 | no | no | - | no | - |
| lua | 261 | no | no | - | no | - |
| perl | 260 | no | no | - | no | - |
| haskell | 213 | no | no | - | no | - |
| csharp | 205 | no | no | - | no | - |
| fsharp | 204 | no | no | - | no | - |
| swift | 179 | no | no | - | no | - |
| c | 168 | no | no | - | no | - |
| cpp | 151 | no | no | - | no | - |
| java | 146 | no | no | - | no | - |
| kotlin | 145 | no | no | - | no | - |
| dart | 152 | no | yes, `dart/pbt` (VER05b) | pass | yes, `dart/fuzz` (VER06c) | pass |
| ocaml | 8 | no | no | - | no | - |

Not lanes for this program: `wasm` (Rust compiled to WebAssembly; served by
the Rust engines), `mosaic`, `starlark`, `twig` (in-repo languages whose test
tooling belongs to their own compilers, VER06 section 7 "self-compiled").

Both corpora validate against their independent references today:
`python3 code/scripts/pbt_conformance.py` and `fuzz_conformance.py` report
25 cases, 0 problems each.

## Placement decision

One engine per lane, living where the lane's PRNG lives, so a lane never
carries two diverging engines (WD-901 records that TypeScript already does,
and that the copies have diverged):

- **Python**: upgrade `python/test-bootstrap`'s `pbt` and `fuzz` modules in
  place (the Rust model). The existing public names consumers import
  (`for_all`, `gen_int`, `gen_list`, `fuzz_bytes`, `SplitMix64`) keep working;
  the corpus-conforming functions are added beside them and `for_all` is
  rebuilt on the conforming shrinker.
- **Go and every lane without a harness**: create `<lane>/test-bootstrap` with
  the shared SplitMix64/FNV-1a first, and put the engines in it as
  sub-packages or modules (Go: `test-bootstrap/prng`, `test-bootstrap/pbt`,
  `test-bootstrap/fuzz`). The TF01 runner itself (TFV, TAP, CLI) follows in
  its own PR; the engines' own tests use the lane's standard test runner
  until then.
- Dart keeps its standalone `dart/pbt` and `dart/fuzz`; its TF01 harness, when
  written, should import their PRNG rather than add a third copy.

## Ordered plan (recommendation: reach first, then adoption value)

Order within a lane is PBT, then fuzz, then the L0 runner where it is missing
(the engines are what the corpora check and what Warrant's native layer needs;
WD-2xx). One language and one engine per PR.

| Order | ID | Lane / engine | Spec | Notes |
|---|---|---|---|---|
| 1 | PT-PY-PBT | python PBT, corpus-conforming | VER05c | 523 packages; engine half-exists; Warrant oracle and WD-101 lane are Python |
| 2 | PT-PY-FUZZ | python fuzz with `sys.monitoring` / `sys.settrace` coverage | VER06d | |
| 3 | PT-GO-PBT | go `test-bootstrap` (PRNG) + PBT | VER05d | 307 packages; WD-103 Warrant lane exists |
| 4 | PT-GO-FUZZ | go fuzz; coverage source settled in VER06e (`runtime/coverage` counters under `-cover`, or caller-reported edges, else `black-box-uninstrumented`) | VER06e | |
Priority key: **P1** = reach lanes (more than 140 packages and a mainstream
runtime), the next to be scheduled; **P2** = high adoption value but after the
P1 lanes; **P3** = small lanes and the "others" tracks that wait on a parent
spec or a delivered engine. The Order column is the scheduling order; the
priority is shown beside it. One row is one PR (one language, one engine).

### Core engines: PBT, fuzz, L0, one row per lane per engine

Spec letters below are **provisional** (the next free ones on 2026-11-24, in
lane order) and are claimed only when each spec is written; re-check
`ls code/specs/VER05*.md VER06*.md TF01*.md` first. PBT and fuzz rows share a
lane letter so the pair reads together (VER05e java, VER06f java).

| Order | ID | Lane / engine | Spec | Notes |
|---|---|---|---|---|
| 5 (P1) | PT-JAVA-PBT | java `test-bootstrap` module + PBT | VER05e | 146 packages; JVM reach. Integers are `long` with `BigInteger` where VER05 needs 64-bit unsigned. |
| 5 (P1) | PT-JAVA-FUZZ | java fuzz | VER06f | no stdlib per-edge API: decide in the spec between caller-reported edges and a `java.lang.instrument` agent (stdlib), else `black-box-uninstrumented` |
| 6 (P1) | PT-KT-PBT | kotlin PBT | VER05g | 145 packages. **Decision recorded in the child spec first:** native port, or a thin idiomatic binding over the Java engine; either way the whole `pbt-v1` corpus is replayed from Kotlin |
| 6 (P1) | PT-KT-FUZZ | kotlin fuzz | VER06g | same decision as PT-KT-PBT |
| 7 (P1) | PT-CS-PBT | csharp PBT | VER05h | 205 packages; `ulong` arithmetic is native |
| 7 (P1) | PT-CS-FUZZ | csharp fuzz | VER06h | caller-reported edges unless the spec finds a stdlib source |
| 8 (P1) | PT-RB-PBT | ruby PBT | VER05i | 308 packages; the second largest lane without any engine. Integers are native bignums |
| 8 (P1) | PT-RB-FUZZ | ruby fuzz, edges from stdlib `Coverage` (branch coverage) | VER06i | |
| 9 (P2) | PT-C-PBT | c PBT | VER05j | 168 packages; 64-bit arithmetic native; header-only or small static library, no allocator surprises |
| 9 (P2) | PT-C-FUZZ | c fuzz; coverage via caller edges (compiler sanitizer-coverage callbacks are a candidate to settle in the spec) | VER06j | |
| 10 (P2) | PT-CPP-PBT | cpp PBT | VER05k | 152 packages; may wrap the C engine behind a typed generator API, but the corpus must still be replayed from C++ |
| 10 (P2) | PT-CPP-FUZZ | cpp fuzz | VER06k | may wrap the C engine |
| 11 (P2) | PT-SWIFT-PBT | swift PBT | VER05l | 179 packages; `UInt64` native; SwiftPM test target |
| 11 (P2) | PT-SWIFT-FUZZ | swift fuzz | VER06l | caller-reported edges unless the spec finds a stdlib source (unverified) |
| 12 (P2) | PT-FS-PBT | fsharp PBT | VER05m | 204 packages; may bind the C# engine (step 7) but must replay the corpus through the F# surface |
| 12 (P2) | PT-FS-FUZZ | fsharp fuzz | VER06m | may bind the C# engine |
| 13 (P2) | PT-EX-PBT | elixir PBT | VER05n | 286 packages; BEAM integers are arbitrary precision, so SplitMix64 needs explicit masking |
| 13 (P2) | PT-EX-FUZZ | elixir fuzz, edges from BEAM `:cover` | VER06n | |
| 14 (P3) | PT-HS-PBT | haskell PBT | VER05o | 213 packages; `Word64`; the generator algebra maps onto an integrated-shrinking tree (Hedgehog-style) naturally |
| 14 (P3) | PT-HS-FUZZ | haskell fuzz; coverage via caller edges or `hpc` ticks (decide in the spec) | VER06o | |
| 15 (P3) | PT-LUA-PBT | lua PBT | VER05p | 261 packages; 53-bit numbers: SplitMix64 needs a two-word representation like Dart's BigInt note (Lua 5.3+ has 64-bit integers, but the lane's supported runtimes must be verified first) |
| 15 (P3) | PT-LUA-FUZZ | lua fuzz; edges via `debug.sethook` (line events) | VER06p | |
| 16 (P3) | PT-PL-PBT | perl PBT | VER05q | 260 packages; 64-bit integer support must be checked per build (`use integer` / `ivsize`) |
| 16 (P3) | PT-PL-FUZZ | perl fuzz; caller-reported edges (Devel::Cover is third-party, so not a runtime source) | VER06q | |
| 17 (P3) | PT-OCAML-PBT | ocaml PBT | VER05r | only 8 packages: lowest reach, schedule last; native `Int64` |
| 17 (P3) | PT-OCAML-FUZZ | ocaml fuzz; caller-reported edges (bisect_ppx is a test-time gate tool, not a runtime source) | VER06r | |
| 18 (P1) | PT-PY-L0 | python: write the missing TF01 child spec and bring `python/test-bootstrap` to it (PRNG, TFV, evidence exist today) | TF01c | spec-only gap; the engines are done |
| 18 (P1) | PT-GO-L0 | go: TF01 runner (TFV, TAP v14, CLI, closed vocabulary) on top of the existing `prng` and validator-only `tfv` | TF01d | |
| 18 (P2) | PT-DART-L0 | dart: TF01 runner; import `dart/pbt`'s PRNG rather than add a third copy | TF01e | |
| 19 (P2) | PT-JAVA-L0, PT-KT-L0, PT-CS-L0, PT-RB-L0 | TF01 runner for each lane, one PR per lane, after that lane's engines | TF01f..i | |
| 19 (P2) | PT-C-L0, PT-CPP-L0, PT-SWIFT-L0, PT-FS-L0, PT-EX-L0 | same | TF01j..n | |
| 19 (P3) | PT-HS-L0, PT-LUA-L0, PT-PL-L0, PT-OCAML-L0 | same | TF01o..r | |
| 20 | PT-DEDUP-TS | WD-901: collapse the TypeScript copies onto one engine | - | existing item, linked here |

Row counts for the core engines (unique lanes without the engine on
2026-11-24): PBT 13, fuzz 13, L0 16 (the 13 above plus Python, Go and Dart,
whose engines exist but whose TF01 runner or child spec does not). The 13 are
ruby, elixir, lua, perl, haskell, csharp, fsharp, swift, c, cpp, java, kotlin
and ocaml. Rust, Python, TypeScript and Go already have PBT and fuzz; Dart has
PBT and fuzz; Rust and TypeScript have a full L0 harness; Python has one
without a child spec; Go has the shared PRNG only.

Spec letters are claimed only when the spec is written; the letters above are
provisional (next free on 2026-11-24) and must be re-checked first.

## Status

| ID | State |
|---|---|
| PT-000 | Done, PR #19878. |
| PT-PY-PBT | Done, PR #19881 (`58baec506ac`). 25/25 with exact step counts; 26 per-op corruptions plus engine sabotage. Found and fixed: `SplitMix64.bounded` used `(-n) % n`, which is 0 in Python, so it never rejected a draw. |
| PT-PY-FUZZ | Done, PR #19884. 25/25. Coverage comes from `sys.monitoring` (3.12+) or `sys.settrace`, with edges keyed by `basename:qualname:from:to`. Minimization runs under a campaign-wide budget. |
| PT-GO-PBT | PR #19891. New `go/test-bootstrap` module with `prng`, `pbt`, a validator-only `tfv` and the `tools/covergate` 95% gate. 25/25 with exact step counts. Integers are `*big.Int`. |
| PT-GO-FUZZ | PR #19898, stacked on #19891. 25/25, plus 820 differential vectors from the reference, which close PT-F1 for this lane. Go's native coverage cannot report per-edge data in process, so edges are caller-reported and a run with none is labelled `black-box-uninstrumented`. Strict-JSON plumbing is shared in `internal/corpusjson`. |
| PT-JAVA-PBT | Next. Re-surveyed 2026-11-24: no branch or PR found (unverified for unpushed local work). |
| PT-001 | This backlog expansion (docs only): per-lane rows for PBT, fuzz and L0, new tracks for mutation, benchmarks and coverage gates. |

## The "others": mutation, microbenchmarks, coverage gate (PT-001, 2026-11-24)

These are new engine tracks. Each follows the shard's definition of done
(above), adapted: a child spec merged first; zero third-party runtime
dependencies; the lane replays the **whole** shared corpus from the committed
fixture and asserts every case; negative controls so the corpus run can fail
(and fails on a short run); the engine's own tests cover at least 95% of its
own code; literate source; BUILD, BUILD_windows, README and a CHANGELOG.d
shard. "What done means" per track is below. State of each track as of
`7da67070ca0`:

### Track MUT: mutation testing

**The parent spec does not exist yet.** `PT-MUT-SPEC` writes it before any lane
row starts (docs only, P1 because every MUT lane row blocks on it, and it
contends with nothing). It must settle:

- Mutation operators for each lane's AST, derived from the repo's own grammars
  under `code/grammars/` and the shared grammar-driven parser where one exists
  (c, csharp, fsharp, haskell, java, perl, python, ruby, rust are present;
  lanes without a grammar start from the token stream, or are marked
  `text-level-only`). Check reuse of the VER08c grammar-driven analyzer plan
  (LGE-S4; its spec file does not exist yet) before inventing a second front
  end.
- Equivalent-mutant handling: a mutant is never silently dropped; each is
  `killed`, `survived`, `equivalent-by-rule` (a named rule), `timeout` or
  `uncompilable`, and each class is counted.
- A shared `mutation-v1` corpus under `code/specs/fixtures/mutation-v1`
  (source, operator, expected mutant text, expected verdict) with an
  independent stdlib reference validator, as for `pbt-v1`/`fuzz-v1`.
- An honest mutation score: survivors, timeouts and equivalents are reported;
  a score computed over zero mutants is never a pass.

**Done means** (per lane): PT-MUT-SPEC and a per-lane child spec merged; the
lane generates every `mutation-v1` mutant exactly and replays the corpus with
negative controls; a run on one real package of the lane reports a score with
the counts above; at least 95% of the engine's own tests; the six hygiene items.

Prerequisite note: a lane's MUT row can start before its PBT/fuzz rows, but its
usefulness depends on the lane having a real unit-test suite and the
coverage gate (track COV).

| Order | ID | Lane | Spec | Notes |
|---|---|---|---|---|
| M0 (P1) | PT-MUT-SPEC | all | new parent spec (number claimed when written) | does not exist yet; docs only |
| M1 (P2) | PT-PY-MUT | python | child of PT-MUT-SPEC | grammar and `ast` both available; pilot lane (523 packages) |
| M1 (P2) | PT-TS-MUT | typescript | child | |
| M1 (P2) | PT-RUST-MUT | rust | child | VER25a Rust-subset grammar exists but is a subset; the full-language question is settled in the spec |
| M2 (P2) | PT-GO-MUT | go | child | stdlib `go/ast` can drive operators |
| M2 (P3) | PT-JAVA-MUT, PT-KT-MUT, PT-CS-MUT, PT-RB-MUT | java, kotlin, csharp, ruby | child | |
| M3 (P3) | PT-C-MUT, PT-CPP-MUT, PT-SWIFT-MUT, PT-FS-MUT, PT-EX-MUT | c, cpp, swift, fsharp, elixir | child | |
| M3 (P3) | PT-HS-MUT, PT-LUA-MUT, PT-PL-MUT, PT-OCAML-MUT, PT-DART-MUT | haskell, lua, perl, ocaml, dart | child | |

18 lane rows plus PT-MUT-SPEC. No lane has a mutation engine today (verified
by directory survey: no `*mutation*` package in any lane; unverified for
unpushed work). The Rust mentions of "mutation testing" in VER-BACKLOG
(KRN-004, KRN-011) are kernel-corpus goals, not this track.

### Track BENCH: microbenchmarks (VER07 children)

[VER07](../VER07-microbenchmarks.md) specifies the harness; VER-P016 delivered
it in `rust/test-bootstrap`, VER07a (VER-P017) the ct-compare measurements.
Survey: `rust/sir-bench` exists (a Rust benchmark program); no `*bench*`
harness package was found under any other lane (unverified for unpushed work).
Rust is therefore done; every other lane lacks the harness.

**Done means** (per lane): a VER07x child spec fixing the lane's clock, its
deterministic cost counter (steps and allocations where the runtime allows;
labelled otherwise), warmup and calibration, robust statistics (median, MAD,
bootstrap CI) conforming to a shared `bench-v1` statistics corpus (fixed
sample arrays to exact estimator outputs; the corpus does not exist yet, so
the first lane row also writes it), budget checking, and TAP/JSON reporting;
negative controls (a sabotaged estimator or a short run fails); the six
hygiene items. Timing numbers are never asserted by the corpus, only the
estimators.

| Order | ID | Lane | Spec | Notes |
|---|---|---|---|---|
| B1 (P3) | PT-PY-BENCH | python | VER07b (provisional) | first lane also writes `bench-v1` |
| B1 (P3) | PT-TS-BENCH, PT-GO-BENCH | typescript, go | VER07c, VER07d | Go has `testing.B` as a clock/alloc source |
| B2 (P3) | PT-JAVA-BENCH, PT-KT-BENCH, PT-CS-BENCH, PT-FS-BENCH | java, kotlin, csharp, fsharp | VER07e..h | JIT warmup matters; F# can bind C# |
| B2 (P3) | PT-RB-BENCH, PT-EX-BENCH, PT-LUA-BENCH, PT-PL-BENCH | ruby, elixir, lua, perl | VER07i..l | |
| B3 (P3) | PT-C-BENCH, PT-CPP-BENCH, PT-SWIFT-BENCH, PT-HS-BENCH, PT-OCAML-BENCH, PT-DART-BENCH | c, cpp, swift, haskell, ocaml, dart | VER07m..r | |

17 lane rows (every lane except Rust). Benchmarks are P3 across the board: they
do not gate correctness and depend on the lane's L0 runner for reporting.

### Track COV: uniform line and branch coverage gate

Every lane already measures coverage somehow; the gap is uniformity of
**threshold** (CLAUDE.md requires well above 80%, target 95% libraries), of
**branch** (not only line) measurement, and of **failing the build** rather
than printing a report. Spot-check of BUILD, test config and a few packages per
lane on `7da67070ca0` (a sample, not an audit; PT-COV-POLICY performs the
full audit):

| Lane | Tool | Gate found | Threshold found | Verdict |
|---|---|---|---|---|
| python | coverage.py via pytest-cov | `fail_under` in `pyproject.toml` | 80 on 322 packages, 85 on 16, 90 on 29, 95 on 53 | enforced, line; threshold not uniform; branch not checked |
| typescript | vitest `--coverage` (v8) | `thresholds` in `vitest.config.ts` | 80 lines in some, 95 in others | enforced, line; not uniform |
| go | `go test -cover` | `-cover` prints; 13 BUILDs use `covergate`/`coverprofile` | none on most | **report only** on most packages |
| ruby | SimpleCov | `minimum_coverage` in test helpers (156 of 308 packages reference SimpleCov) | 60, 70, 80, 95 seen | partly enforced, line; others unverified |
| elixir | `mix test --cover` | `test_coverage: [summary: [threshold: 80]]` seen | 80 seen | partly enforced; many packages have no `--cover` |
| lua | luacov | only `lua/der_asn1` has a `check_coverage.lua`; others run `busted` with no coverage | none | **no gate** |
| perl | Devel::Cover | `prove -l -v t/` only | none | **no gate** |
| haskell | `hpc` (`cabal test --enable-coverage`) | `cabal test all` only | none | **no gate** |
| csharp | coverlet | `/p:Threshold=80 /p:ThresholdType=line` | 80 | enforced, line only |
| fsharp | coverlet | same properties | 80, 95 seen | enforced, line only |
| swift | `swift test --enable-code-coverage` | measured, no threshold seen | none | **report only** |
| c | gcov / llvm-cov | `sh tools/run.sh`, no coverage step seen | none | **no gate** |
| cpp | gcov / llvm-cov | same | none | **no gate** |
| java | JaCoCo | `jacocoTestCoverageVerification`, minimum 0.95 (in a sampled package) | 95 | enforced; other packages unverified |
| kotlin | JaCoCo | same | 95 | enforced in sample; others unverified |
| dart | `dart run coverage:test_with_coverage` | `--fail-under=95 --branch-coverage` | 95 | enforced, branch, in sample; not every package |
| ocaml | `bisect_ppx` | `bisect-ppx-report summary --expect <file>` | no percent flag seen | measured; failing threshold unverified |
| rust | `cargo tarpaulin` / `llvm-cov` | 27 BUILDs mention a coverage tool | varies | unverified per package |

**Done means** (per lane): a `PT-COV-POLICY` note (first row) fixes one rule
across lanes: line and branch (where the tool can measure branches; say so where
it cannot), 95% for libraries and 80% for programs, and a build that fails when
under; the lane's BUILD pattern is adopted by a pilot package, with a negative
control (a deliberately uncovered branch makes the BUILD fail); then a sweep of
the lane's packages in batches. A package that cannot meet it records the
exception in its CHANGELOG.d shard, not silently. This is configuration work, not
a new engine, so there is no shared corpus and no third-party-runtime-dependency
question (the coverage tools are test-time, as the existing BUILDs already use).

| Order | ID | Lane | Notes |
|---|---|---|---|
| C0 (P1) | PT-COV-POLICY | all | write the policy and run the full per-lane audit; replaces the sample above with counts |
| C1 (P1) | PT-LUA-COV, PT-PL-COV, PT-HS-COV, PT-C-COV, PT-CPP-COV | lua, perl, haskell, c, cpp | no gate today; new gate pattern plus pilot; cheapest high-value rows |
| C1 (P1) | PT-GO-COV, PT-SWIFT-COV | go, swift | report-only today: add a failing threshold |
| C2 (P2) | PT-PY-COV, PT-TS-COV, PT-RB-COV, PT-EX-COV, PT-CS-COV, PT-FS-COV | python, typescript, ruby, elixir, csharp, fsharp | enforced but not uniform: raise to the policy threshold and add branch where the tool supports it (coverlet: `ThresholdType=branch`) |
| C2 (P2) | PT-JAVA-COV, PT-KT-COV, PT-DART-COV, PT-OCAML-COV, PT-RUST-COV | java, kotlin, dart, ocaml, rust | extend the sampled gate to every package; ocaml needs a failing threshold |

18 lane rows plus PT-COV-POLICY.

### Differential and metamorphic testing: no new rows

Differential and metamorphic testing against the shared corpus already has an
owner: the Warrant `warrant-corpus` runners of
[VER04i](../VER04i-polyglot-warrant-evidence.md) (runners exist for python,
typescript, go and rust's `warrant-corpus-gen`; the other lanes' runners are
tracked in [portable-warrant](portable-warrant.md) and the logic-gates rollout in
[logic-gates-everywhere](logic-gates-everywhere.md), LGE-L-<lane>). Adding rows
here would duplicate that backlog. A lane gains this testing by getting its
VER04i runner, not by a row in this shard.

### Out of scope here

- Lean-like proofs and the proof kernel: VER11a (`VER11a-lean4-proof-kernel.md`),
  the Lean machine vision and the KRN-* rows in VER-BACKLOG. This shard
  delivers the testing half of "Lean like testing", not the proving half.
- Capability analyzers: the VER08 family and the grammar-driven VER08c plan
  (logic-gates-everywhere LGE-S4 and LGE-A1..A3).

## Recommended next three items

1. **PT-JAVA-PBT** (order 5, P1): next in the existing reach order; the Java lane
   is the largest lane with a mainstream runtime and no engine, and it unblocks
   Kotlin's native-or-binding decision.
2. **PT-JAVA-FUZZ** (order 5, P1): the pair completes one lane (PBT then fuzz,
   one PR each).
3. **PT-KT-PBT** (order 6, P1): the Kotlin decision (native port or binding over
   the Java engine) is recorded in its child spec first.

Two docs-only items can run alongside these without contending on any lane:
**PT-MUT-SPEC** (nothing in track MUT can start without it) and
**PT-COV-POLICY** (replaces the spot-check above with a full audit, and the
no-gate lanes lua, perl, haskell, c and cpp are the cheapest wins in the whole
backlog).

## Follow-ups found while delivering

| ID | Work | Source |
|---|---|---|
| PT-F1 | Strengthen `fuzz-v1`. The `mutate_splice` case uses equal splits (2, 2), so an engine that swaps target and donor splits passes. The single `mutate_sequence` case does not pin every constant (delta -35 to -36, or 8-bit interest 255 to 254, still passes). Add cases, then rerun every lane. | PT-PY-FUZZ |
| PT-F2 | Rust, TypeScript and Dart engines are not registered in the build tool as consumers of `pbt-v1`/`fuzz-v1`, so a corpus edit does not reselect them. Python is registered (and Go will be). | PT-PY-PBT |
| PT-F3 | `sub_seed` differs across lanes: VER05 section 2.3, Rust, Python and Go use `SplitMix64(seed ^ fnv1a64(id)).next_u64()`; TypeScript and Dart use the xor alone. No corpus case pins it. Add one and fix the two lanes. | PT-PY-PBT |
| PT-F4 | `bounded(1)` consumes a stream word in Rust and the reference but not in Dart or Python's base `SplitMix64` (the Python engines align through their own helper). Pin it in a corpus case. | PT-PY-FUZZ |
| PT-F5 | Python corpus negative controls: the single-case assertion in `test_corrupted_expectation_is_reported` (pbt and fuzz) passes vacuously, because a lone case always reports "never exercised" for the other ops. The full-corpus swap assertion beside it is real. Make the single-case check require the problem to name the corrupted case id, as the Go lane does. | PT-GO-FUZZ |
| PT-F6 | Go has no in-process per-edge coverage. Consider a source-instrumenting pass (stdlib `go/ast` rewrite that inserts `Edge` calls) so Go campaigns stop depending on hand-placed edges. | PT-GO-FUZZ |
