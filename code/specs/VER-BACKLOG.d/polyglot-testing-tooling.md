# Polyglot testing tooling: L0 harness, PBT and fuzz in every lane

**Authority:** current matrix, plan and status for bringing the three testing
engines to every language lane: the L0 harness ([TF01](../TF01-bootstrap-harness.md)
`test-bootstrap`), the property engine ([VER05](../VER05-property-engine.md))
and the coverage-guided fuzz engine ([VER06](../VER06-coverage-guided-fuzz-engine.md)).
The main [VER-BACKLOG](../VER-BACKLOG.md) keeps a one-line pointer; this shard
owns the detail.

Owner direction, 2026-11-23: "bring Lean like testing along with fuzzing, unit
testing, property testing and others to every language as every language
deserves such a tooling".

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

## Matrix (surveyed 2026-11-23 on `origin/main` `7696fbf867d`)

Packages per lane is the number of directories under `code/packages/<lane>/`,
used as a proxy for how much of the repo each engine would serve.

| Lane | Packages | L0 harness (TF01) | PBT (VER05) | pbt-v1 | Fuzz (VER06) | fuzz-v1 |
|---|---:|---|---|---|---|---|
| rust | 1550 | yes, `rust/test-bootstrap` (VER-P003) | yes, in `test-bootstrap` (VER-P010) | pass (`tests/pbt_corpus.rs`) | yes, in `test-bootstrap` (VER-P014) | pass (`tests/fuzz_corpus.rs`) |
| python | 523 | yes, `python/test-bootstrap` (PRNG, TFV, evidence; no TF01 child spec) | **partial**: `test_bootstrap.pbt` (`for_all`, `Gen`, shrinkers) | **not run**; shrinker API cannot express `shrink_int(initial, target, min, max)` or report step counts | **partial**: `test_bootstrap.fuzz` is blind mutation (no coverage bitmap, power schedule, VER06 dedup hash, arithmetic or boundary mutators) | **not run** |
| typescript | 501 | yes, `typescript/test-bootstrap` (TF01a, TF01b) | yes, `typescript/pbt` (VER05a) and a copy in `test-bootstrap` | pass | yes, `typescript/fuzz` (VER06b) and a copy in `test-bootstrap` | pass |
| ruby | 308 | no | no | - | no | - |
| go | 307 | no (`go/warrant-corpus` is a Warrant corpus runner, not TF01) | no | - | no | - |
| elixir | 286 | no | no | - | no | - |
| lua | 261 | no | no | - | no | - |
| perl | 260 | no | no | - | no | - |
| haskell | 213 | no | no | - | no | - |
| csharp | 205 | no | no | - | no | - |
| fsharp | 204 | no | no | - | no | - |
| swift | 179 | no | no | - | no | - |
| c | 167 | no | no | - | no | - |
| cpp | 151 | no | no | - | no | - |
| java | 146 | no | no | - | no | - |
| kotlin | 145 | no | no | - | no | - |
| dart | 135 | no | yes, `dart/pbt` (VER05b) | pass | yes, `dart/fuzz` (VER06c) | pass |
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
| 5 | PT-JAVA-PBT / FUZZ | java `test-bootstrap` + engines | VER05e / VER06f | JVM reach; Kotlin can call the Java engine but gets idiomatic bindings at step 6 |
| 6 | PT-KT-PBT / FUZZ | kotlin | VER05f / VER06g | decide then: native port or thin binding over the Java engine |
| 7 | PT-CS-PBT / FUZZ | csharp | VER05g / VER06h | F# can bind the C# engine (step 11) |
| 8 | PT-RB-PBT / FUZZ | ruby (`Coverage` module for edges) | VER05h / VER06i | 308 packages |
| 9 | PT-C-PBT / FUZZ | c | next free | 64-bit arithmetic native; coverage via caller edges |
| 10 | PT-CPP-PBT / FUZZ | cpp | next free | may wrap the C engine |
| 11 | PT-SWIFT-PBT / FUZZ | swift | next free | |
| 12 | PT-FS, PT-EX, PT-HS, PT-LUA, PT-PL, PT-OCAML | fsharp, elixir (BEAM `:cover`), haskell, lua (53-bit numbers: SplitMix64 needs a two-word representation like Dart's BigInt note), perl, ocaml | next free | |
| 13 | PT-L0-<lane> | TF01 runner (TFV, TAP v14, CLI, closed vocabulary) for each lane above that lacks one, plus dart | TF01x | |
| 14 | PT-DEDUP-TS | WD-901: collapse the TypeScript copies onto one engine | - | existing item, linked here |

Spec letters are claimed only when the spec is written; the letters above are
the next free ones on 2026-11-23 and must be re-checked first.

## Status

| ID | State |
|---|---|
| PT-000 | Matrix and plan (this file). |
| PT-PY-PBT | In progress. |
