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
| python | 523 | yes, `python/test-bootstrap` (PRNG, TFV, evidence; no TF01 child spec) | yes, `test-bootstrap` `pbt` (VER05c) | pass (`tests/test_pbt_corpus.py`, #19881) | yes, `test-bootstrap` `fuzz` (VER06d), `sys.monitoring` / `sys.settrace` coverage | pass (`tests/test_fuzz_corpus.py`) |
| typescript | 501 | yes, `typescript/test-bootstrap` (TF01a, TF01b) | yes, `typescript/pbt` (VER05a) and a copy in `test-bootstrap` | pass | yes, `typescript/fuzz` (VER06b) and a copy in `test-bootstrap` | pass |
| ruby | 308 | no | no | - | no | - |
| go | 307 | partial: `go/test-bootstrap` holds the shared `prng` (no TF01 runner yet; `go/warrant-corpus` is a Warrant corpus runner) | yes, `test-bootstrap/pbt` (VER05d) | pass (`pbt/corpus_test.go`, #19891) | yes, `test-bootstrap/fuzz` (VER06e), caller-reported edges (`Handle.Edge`) | pass (`fuzz/corpus_test.go`, #19898) |
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
| PT-000 | Done, PR #19878. |
| PT-PY-PBT | Done, PR #19881 (`58baec506ac`). 25/25 with exact step counts; 26 per-op corruptions plus engine sabotage. Found and fixed: `SplitMix64.bounded` used `(-n) % n`, which is 0 in Python, so it never rejected a draw. |
| PT-PY-FUZZ | Done, PR #19884. 25/25. Coverage comes from `sys.monitoring` (3.12+) or `sys.settrace`, with edges keyed by `basename:qualname:from:to`. Minimization runs under a campaign-wide budget. |
| PT-GO-PBT | PR #19891. New `go/test-bootstrap` module with `prng`, `pbt`, a validator-only `tfv` and the `tools/covergate` 95% gate. 25/25 with exact step counts. Integers are `*big.Int`. |
| PT-GO-FUZZ | PR #19898, stacked on #19891. 25/25, plus 820 differential vectors from the reference, which close PT-F1 for this lane. Go's native coverage cannot report per-edge data in process, so edges are caller-reported and a run with none is labelled `black-box-uninstrumented`. Strict-JSON plumbing is shared in `internal/corpusjson`. |
| PT-JAVA-PBT | Next. |

## Follow-ups found while delivering

| ID | Work | Source |
|---|---|---|
| PT-F1 | Strengthen `fuzz-v1`. The `mutate_splice` case uses equal splits (2, 2), so an engine that swaps target and donor splits passes. The single `mutate_sequence` case does not pin every constant (delta -35 to -36, or 8-bit interest 255 to 254, still passes). Add cases, then rerun every lane. | PT-PY-FUZZ |
| PT-F2 | Rust, TypeScript and Dart engines are not registered in the build tool as consumers of `pbt-v1`/`fuzz-v1`, so a corpus edit does not reselect them. Python is registered (and Go will be). | PT-PY-PBT |
| PT-F3 | `sub_seed` differs across lanes: VER05 section 2.3, Rust, Python and Go use `SplitMix64(seed ^ fnv1a64(id)).next_u64()`; TypeScript and Dart use the xor alone. No corpus case pins it. Add one and fix the two lanes. | PT-PY-PBT |
| PT-F4 | `bounded(1)` consumes a stream word in Rust and the reference but not in Dart or Python's base `SplitMix64` (the Python engines align through their own helper). Pin it in a corpus case. | PT-PY-FUZZ |
| PT-F5 | Python corpus negative controls: the single-case assertion in `test_corrupted_expectation_is_reported` (pbt and fuzz) passes vacuously, because a lone case always reports "never exercised" for the other ops. The full-corpus swap assertion beside it is real. Make the single-case check require the problem to name the corrupted case id, as the Go lane does. | PT-GO-FUZZ |
| PT-F6 | Go has no in-process per-edge coverage. Consider a source-instrumenting pass (stdlib `go/ast` rewrite that inserts `Edge` calls) so Go campaigns stop depending on hand-placed edges. | PT-GO-FUZZ |
