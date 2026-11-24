# Logic gates everywhere: verified, publishable basic gates in every lane

**Authority:** current plan, definition of done, queue and status for bringing
the eleven basic logic gates to every language lane at the Rust standard of
evidence, with a per-language weakness (CWE) threat list, and publishing them.
The main [VER-BACKLOG](../VER-BACKLOG.md) keeps a one-line pointer; this shard
owns the detail.

Owner direction, 2026-11-24: the basic gates must work in all languages, "be
extensively tested and validated because I want other things to build on top
of it taking its proofs as assumptions", show that the classes of CVEs and
other common security problems do not occur, and be published. Publish while
truthfully stating what is verified; upgrade later.

## Scope

The eleven basic gates, in every lane:

| Group | Operations |
|---|---|
| Primitive | NOT, AND, OR, XOR, NAND, NOR, XNOR |
| NAND-built (functional completeness) | nand_not, nand_and, nand_or, nand_xor |

Lanes (18): rust, python, typescript, go, ruby, c, cpp, java, kotlin, csharp,
fsharp, elixir, lua, perl, haskell, swift, dart, ocaml. Not lanes: wasm,
mosaic, starlark, twig (same rule as
[polyglot-testing-tooling](polyglot-testing-tooling.md)).

Out of scope for this program: n-input gates, sequential logic and
combinational blocks (LG01 keeps covering them in Rust).

## Owner decisions (2026-11-24)

1. Logic gates opens each new VER04i lane, in place of ct-compare
   (VER04i section 13.2 to be amended).
2. Lanes without a capability analyzer get one built on the repository's own
   grammars and parsers (`code/grammars/<lang>`, `rust/grammar-tools`). This
   track runs in parallel and does not block publishing; reports are upgraded
   when each analyzer lands.
3. `weakness-registry-v2` adds CWE-20 (improper input validation), CWE-843
   (type confusion) and CWE-704 (incorrect type conversion). Logic gates
   adopts v2; other packages stay on v1 until migrated.
4. Every lane gets a **per-language threat list**: the registry classes plus
   the language's own pitfalls, each with the check that closes it.
5. Releases state truthfully what is verified; open rows are labelled, never
   hidden, and do not block a `tested`-tier release.

## Audit at the start (`origin/main` `955444d0190`)

| Lane | Package | 11 ops | Full truth tables | Rejection tests | Notes |
|---|---|---|---|---|---|
| rust | `rust/logic-gates` | yes | exhaustive, independent oracle (LG01, Warrant) | every op, debug and release | report verdict `report-only`: CWE-400 computed exemption (no live step proof) |
| go | `go/logic-gates` | yes | yes | all 11 | `assertPanics` ignores its message argument |
| ruby | `ruby/logic_gates` | yes | yes | all 11, one case each | |
| swift | `swift/logic_gates` | yes | yes | all 11 | bare `XCTAssertThrowsError` except NOT |
| python | `python/logic-gates` | yes | yes | 4 of 11, first argument only | |
| typescript | `typescript/logic-gates` | yes | yes | 4 of 11, first argument only | NAND/NOR/XNOR rely on transistors' validation |
| elixir | `elixir/logic_gates` | yes | yes | all 11 for range, types only for NOT/AND/nand_not | |
| java | `java/logic-gates` | yes | yes | 7 of 11 | no coverage gate |
| kotlin | `kotlin/logic-gates` | yes | yes | 7 of 11 | no coverage gate |
| dart | `dart/logic-gates` | yes | yes | second argument only, no nand_* | no coverage gate |
| lua | `lua/logic_gates` | yes | yes | 4 of 11 | accepts `1.0` |
| csharp | `csharp/logic-gates` | yes | yes | 4 of 11 | |
| fsharp | `fsharp/logic-gates` | yes | yes | 4 of 11 | |
| ocaml | `ocaml/logic-gates` | yes | yes, but expected values use the same operators | 2 of 11 | |
| perl | `perl/logic-gates` | yes | yes | 4 of 11 | **bug: numeric `!=` validation, `AND("1abc", 1)` returns 1** |
| haskell | `haskell/logic-gates` | **6 of 11** | **AND and OR 2 rows each only** | none | |
| c | none | | | | |
| cpp | none | | | | |

Infrastructure at the start: warrant-corpus runners exist for python,
typescript and go only; capability analyzers for rust, go, python, typescript
and ocaml only; the weakness matrix is Rust-only (VER04i section 8.3);
`publish.yml` publishes to PyPI and RubyGems only and the VER31 gate is not
wired.

## Definition of done, per lane

1. The eleven operations exist, take integers, and reject every non-bit in
   the lane's idiom (exception, `Result`/`Either`, status code in C).
2. **Contract evidence (VER04i layer A).** The lane's `warrant-corpus` runner
   replays the shared basic-gates corpus: examples, every valid input and
   every non-bit `U8`, oracle outside the lane, every negative control caught,
   manifest written by a real run, `ca-assurance-report --check-lane` passes.
3. **Type-confusion suite.** A fixed adversarial matrix on all eleven
   operations and both arguments (numeric strings, `"1abc"`, empty string,
   booleans, nil/undef/None, `1.0`, NaN, 2^64, -1, lists, whatever the lane's
   threat list adds). Fuzz target added when the lane's VER06 engine lands.
4. **Weakness matrix v2** for the lane, every row `excluded`, `discharged` or
   a computed exemption, from the lane's own evidence (analyzer, corpus,
   type-confusion suite; sanitizers for C and C++).
5. **Release-ready** (VER31): BUILD and BUILD_windows, README with a "What is
   verified" section generated from the report, CHANGELOG.d, capabilities
   `[]`, coverage 95%+, registry metadata, a publish job, the VER31 gate
   passing at tier `tested` (the most a lane package reaches, VER31 6.2).

## Queue

Phases run in this order; Phase 3 (analyzers) runs in parallel with the
rest. Re-prioritized 2026-11-24 after the design spike: no lane can replay a
`warrant-corpus-v2` corpus yet (the Python, TypeScript and Go runners read v1
only; `ca-assurance-report` lane evidence is hard-wired to ct-compare v1), so
each lane's interim evidence is a **lane-local exhaustive** suite and a
release may claim only that until the lane's v2 runner lands.

Priority rule before each pick: (1) unblock in-flight PRs; (2) security bugs
in a gate validator; (3) lane-local exhaustive suites and error identities
(they are what a release can truthfully claim); (4) the LG02 corpus chain;
(5) registry v2 and threat lists; (6) CV2 runner dependencies; (7) analyzers;
(8) publishing.

| Pri | ID | Work | State |
|---|---|---|---|
| - | LGE-000 | This backlog shard and the VER-BACKLOG pointer | merged #19922 |
| 1 | LGE-P1 | Perl: canonical type-and-value bit validation, arity, no invocant guessing, bounded widths, analog input validation in transistors; 2,654 + 607 tests | PR #19927 |
| 1 | LGE-H1 | Haskell: xnor and the four NAND-built gates, full tests | PR #19924 |
| 1 | LGE-C1 | New `c/logic-gates` (status code, no pointers), 1,351,741 checks under strict warnings and ASan/UBSan | PR #19926 |
| 1 | LGE-CPP1 | New `cpp/logic-gates` (`constexpr`, `std::invalid_argument`, deleted non-int overloads), 96,454 checks | PR #19928 |
| 1 | LGE-S1a | LG02 cross-lane contract (separate corpus-only `logic-gates-basic.warrant`, drift guard, v2 corpus, per-lane error identities) and the VER04i 13.2 amendment (was LGE-S2) | security-review findings being addressed |
| 3 | LGE-R-python | Exhaustive rejection and type-confusion matrix; `type(x) is int` | in progress |
| 3 | LGE-R-typescript | Every gate validates its own inputs (NAND/NOR/XNOR relied on transistors); reject `-0`; matrix | in progress |
| 3 | LGE-R-go | Typed panic value (today a plain string at `gates.go:92`, and a different one at `:154`, so a recover adapter cannot tell them apart); `assertPanics` checks its message; matrix | queued |
| 3 | LGE-ID-perl | Stable error identity: `CodingAdventures::LogicGates::Error` with a code (bindings may not match message text) | queued |
| 3 | LGE-ID-lua | Stable error identity (error table with `code`); reject `1.0`; matrix | queued |
| 3 | LGE-ID-haskell | `LogicGateError` ADT instead of `Left String` | queued |
| 3 | LGE-R-<lane> | Lane-local exhaustive suites for ruby, swift, elixir, java, kotlin, csharp, fsharp, dart, ocaml (OCaml: literal expected values, not `land`/`lor`) | queued |
| 4 | LGE-S1b | `logic-gates-basic.warrant` and `code/scripts/logic_gates_basic_drift.py` with its tests and build-input wiring | queued |
| 4 | LGE-S1c | Generated `logic-gates-basic-warrant-v2` fixture, build-tool row (optional `bindings`), `--check` and `verify-v2` BUILD lines | queued |
| 5 | LGE-S3 | VER15 amendment, `weakness-registry-v2.json` (CWE-20, CWE-843, CWE-704), per-lane matrix section: spec [VER15a](../VER15a-weakness-registry-v2-and-lane-matrices.md), registry, lane threat-list schema and the worked `c` file written (not yet merged); section 9 lists what the validator (new item, queued) must check | spec written, awaiting review |
| 3 | LGE-C2 | `c/logic-gates`: `warn_unused_result` / `[[nodiscard]]` on every `lg_*` function (the `-1` status is truthy, CWE-252); found by the VER15a review | queued |
| 3 | LGE-C3 | `c/logic-gates` `tools/run.sh`: a skipped sanitizer run (lines 58-60) fails CI; MSan/TSan where supported; found by the VER15a review | queued |
| 7 | LGE-CPP-EXT | C++ memory-safety safe-subset extension, distinct from the C one (references, `this`, containers, `string_view`/`span`, iterators, lambdas by reference, `new`, RAII) and its analyzer rule (VER15a section 5.5) | queued |
| 5 | LGE-S3v | Validator `code/scripts/validate_weakness_lanes.py` with falsification tests (VER15a section 9) | queued |
| 5 | LGE-S3b | Per-language threat lists `code/specs/data/weakness-registry-v2/lanes/<lang>.json` (draft agent plus independent adversarial reviewer per lane) | queued |
| 6 | LGE-CV2 | Dependencies owned by the portable-warrant queue and idle since 2026-11-20: S6b/S6c (Python v2 runner), S7 (TypeScript), Go v2 port, S9 lane validator and per-corpus registry; coordinate before taking any | queued |
| 6 | LGE-L-<lane> | Lane runner (v2) and logic-gates binding with BUILD negative controls, VER04i order, at most two lanes in flight | blocked on LGE-CV2 |
| 7 | LGE-S4 | VER08c grammar-driven capability analyzer spec | queued |
| 7 | LGE-A1 | Analyzer core plus rule tables for c, csharp, fsharp, haskell, java, perl, ruby | queued |
| 7 | LGE-A2 | `.tokens` then `.grammar` for cpp, kotlin, elixir, lua, swift, dart | queued |
| 7 | LGE-W1 | Per-lane v2 weakness matrices in `ca-assurance-report` | queued |
| 8 | LGE-S5 | VER31 amendment: publish job per registry and the gate | queued |
| 8 | LGE-PUB-<registry> | Publish jobs and gate, one registry per PR; first release per lane needs the owner's go-ahead | queued |

## Honest limits

- "Every CVE class" means every class in registry v2 plus the lane's threat
  list, each closed by evidence; it is not a claim about every possible bug.
- Lanes other than Rust reach `tested` (exhaustive over a finite domain), not
  `proved`, until a lane front end exists (VER04i section 10).
- Registry accounts, trusted publishers and secrets are owner actions.

## Status log

| Date | Item | Result |
|---|---|---|
| 2026-11-24 | audit | table above; Perl bug reproduced locally |
| 2026-11-24 | LGE-000 | merged #19922 |
| 2026-11-24 | design spike | v2 corpus required; lane replay blocked on CV2 runners; separate corpus-only basic document chosen |
| 2026-11-24 | Perl review | four security-review rounds: dualvars computed on the numeric part, invocant guessing that dropped inputs (`ANDn('CodingAdventures::LogicGates',1,1)` returned 1), unbounded widths, the same guessing in `nmos`/`pmos`/`npn`/`pnp`; all fixed in #19927 |
| 2026-11-24 | discoveries | Go panics with plain strings; Perl, Lua and Haskell report free-text errors; TypeScript accepts `-0` and NAND/NOR/XNOR skip their own validation; logged above |
