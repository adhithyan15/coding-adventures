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
| - | LGE-P1 | Perl: canonical type-and-value bit validation, arity, no invocant guessing, bounded widths, analog input validation in transistors; 2,654 + 607 tests | merged #19927 |
| - | LGE-H1 | Haskell: xnor and the four NAND-built gates, full tests | merged #19924 |
| 1 | LGE-C1 | New `c/logic-gates` (status code, no pointers), 1,351,741 checks under strict warnings and ASan/UBSan | PR #19926 (waiting on macOS runners) |
| 1 | LGE-CPP1 | New `cpp/logic-gates` (`constexpr`, `std::invalid_argument`, deleted non-int overloads), 96,454 checks | merged #19928 |
| - | LGE-S1a | LG02 cross-lane contract (separate corpus-only `logic-gates-basic.warrant`, drift guard, v2 corpus, per-lane error identities) and the VER04i 13.2 amendment (was LGE-S2) | merged #19931 |
| 3 | LGE-R-python | Exhaustive rejection and type-confusion matrix; `type(x) is int` | PR #19933 |
| 3 | LGE-R-typescript | Every gate validates its own inputs (NAND/NOR/XNOR relied on transistors); reject `-0`; matrix | merged #19932 |
| 3 | LGE-R-go | Typed panic value (today a plain string at `gates.go:92`, and a different one at `:154`, so a recover adapter cannot tell them apart); `assertPanics` checks its message; matrix | merged #19935 (plus bounded widths and validated state) |
| 3 | LGE-ID-perl | Stable error identity: `CodingAdventures::LogicGates::Error` with a code (bindings may not match message text) | in review (Error class; concatenation overload being added) |
| 3 | LGE-ID-lua | Stable error identity (error table with `code`); reject `1.0`; matrix | PR #19936 |
| 3 | LGE-ID-haskell | `LogicGateError` ADT instead of `Left String` | PR #19942 (plus total muxN, select/width caps, no silent register repair) |
| 3 | LGE-R-java | `InvalidBitException`, exhaustive tests, JaCoCo 0.8.12 gate | PR #19937 |
| 3 | LGE-R-kotlin | `InvalidBitException`, exhaustive tests, JaCoCo gate | PR #19938 |
| 3 | LGE-R-csharp | `InvalidBitException`, exhaustive tests, 95% gate | PR #19939 |
| 3 | LGE-R-fsharp | `InvalidBitException`, exhaustive tests, 95% gate | PR #19941 |
| 3 | LGE-R-swift | Exhaustive truth tables and bit rejection with exact `invalidBit(name:got:)` payloads, no crash on oversized selects (caps at 16 select bits and 2^16 lines) | merged #19955 |
| 3 | LGE-R-<lane> | Lane-local exhaustive suites for ruby, swift, elixir, dart, ocaml (OCaml: literal expected values, not `land`/`lor`) | queued |
| 2 | LGE-HS-DEPS | Haskell dependents (security, pre-existing): `alu [] []` returns Right with bottom flags; `rippleCarryAdder [] [] 5` returns an unvalidated carry; `isNegative` checks only the head; `newBRAM` width 0 throws DivideByZero out of the Either; RAM ticks never validate data/clock/write-enable (stores 5); `configureLUT` does not validate its truth table; RAM sizes uncapped | queued, after #19942 |
| 2 | LGE-DOS-ALL | Cap decoder/demux/mux select bits at 16 and sequential widths in every lane with these blocks (Rust SEC-008, Go and Haskell done in-PR; Python confirmed unbounded) | queued |
| 3 | LGE-PY-SEQ-COMB | Python: counter returns unvalidated state bits; encoders accept any sized container | queued |
| 3 | LGE-LUA-SEQ | Lua `sequential.lua` still has free-text validation accepting `1.0` | queued |
| 3 | LGE-ID-perl-transistors | Perl transistors needs a typed error identity too | queued |
| 4 | LGE-LUA-STALE | Lua BUILD install guards skip reinstall when an old rock exists, so a cached tree could test dependents against the old validator | queued |
| 4 | LGE-HS-COUNTER | Haskell `counter 1 0 [0,1]` returns `[0,1]`, not `[1,0]` (two-phase flip-flop emulation); investigate | queued |
| 3 | LGE-HS-HARDEN | Haskell `logic-gates` hardening found by the lane review, in three sub-items. (a) code: bounded forced snapshots of every caller list with caps (`decoder (cycle [0])` never returns; encoder, priorityEncoder, mux4, mux8, muxN inputs and the exported `validateBits` are uncapped), shiftRegister and counter reject an empty state instead of defaulting to four zeros, un-export or newtype the validators' name argument, document the caller-bottom contract (`andGate 1 (error "boom")` throws out of the Either) with an `evaluate`/`try` test, LG02 wide rows and a rejection table for every block, hpc coverage, `required_capabilities.json`. (b) build flags: `ghc-options: -Wall -Wcompat -Wincomplete-patterns -Wincomplete-uni-patterns -Werror` (check the test suite under CI GHC 9.4.8), `{-# LANGUAGE Safe #-}` on every module, BUILD fails when `cabal` is missing instead of printing "skipping" and exiting 0, a GHC version matrix. (c) supply chain: `cabal.project.freeze` or index-state, tighter hspec bound | queued |
| 4 | LGE-S1b | `logic-gates-basic.warrant` and `code/scripts/logic_gates_basic_drift.py` with its tests and build-input wiring | in review (drift-guard fail-closed fixes) |
| 4 | LGE-S1c | Generated `logic-gates-basic-warrant-v2` fixture, build-tool row (optional `bindings`), `--check` and `verify-v2` BUILD lines | queued |
| 5 | LGE-S3 | VER15 amendment, `weakness-registry-v2.json` (CWE-20, CWE-843, CWE-704), per-lane matrix section: spec [VER15a](../VER15a-weakness-registry-v2-and-lane-matrices.md), registry, lane threat-list schema and the worked `c` file written (not yet merged); section 9 lists what the validator (new item, queued) must check | spec written, awaiting review |
| 3 | LGE-C2 | `c/logic-gates`: `warn_unused_result` / `[[nodiscard]]` on every `lg_*` function (the `-1` status is truthy, CWE-252); found by the VER15a review | queued |
| 3 | LGE-C3 | `c/logic-gates` `tools/run.sh`: a skipped sanitizer run (lines 58-60) fails CI; MSan/TSan where supported; found by the VER15a review | queued |
| 7 | LGE-CPP-EXT | C++ memory-safety extension `cpp_address_surface_allowlist` (VER15a section 5.5; allow-list is exactly `detail::reject`) and its analyzer rule `analyzer-rule:cpp.no-address-surface` (references, `this`, containers, `string_view`/`span`, iterators, lambdas by reference, `new`, RAII) | queued |
| 3 | LGE-CPP-HARDEN | `cpp/logic-gates` hardening found by the lane review: deleted `std::initializer_list` overloads (braced arguments such as `and_gate({true}, 1)` skip the deleted templates), compile-must-fail tests, a hostile-macro include test (`#define check(x)`), an `-O2` run, a skipped sanitizer run fails CI, `[[nodiscard]]`, versioned inline namespace | local, in progress |
| 3 | LGE-CPP-MATRIX | `cpp/logic-gates` extended type-confusion matrix (LG02 section 9): unscoped and scoped enum, `wchar_t`, `char16_t`, `char32_t`, `char8_t`, `unsigned short`, `unsigned long long`, `long double`, `nullptr_t`, string literal, class with `operator int`, braced-list and function-pointer rows | queued |
| 5 | LGE-S3v | Validator `code/scripts/validate_weakness_lanes.py` with falsification tests (VER15a section 9) | queued |
| 5 | LGE-S3b | Per-language threat lists `code/specs/data/weakness-registry-v2/lanes/<lang>.json` (draft agent plus independent adversarial reviewer per lane) | queued |
| 3 | LGE-TS-HARDEN | TypeScript `logic-gates` hardening found by the lane review: `validateBit` parameter name, mux/dmux arity checks, `InternalGateError`, `tsc` in BUILD, n-ary input cap | local, ready |
| 3 | LGE-LUA-HARDEN | Lua `logic-gates` hardening found by the lane review (all reproduced on Lua 5.5.0): `Register`/`Counter` trust `rawlen` on tables with holes (`{1, nil, 1}` silently becomes one bit); `ANDn`/`ORn` uncapped and arity below 2 is a position-prefixed string; composite gates call `gates.NAND` etc. through the mutable module table (patching one entry changes `NAND_AND`) and `error`, `tostring`, `table.pack`, `ipairs` are looked up as globals at call time (`_G.error = function() end` makes `AND("x", 1)` return instead of raise: capture at load, test replacing globals); read-only module tables; public `raise_invalid_*` and mutable `err.value` reaching `__tostring`; `ANDn` "stack overflow" at ~900k args; error tables forgeable and mutable; CMOS result and internal-failure branch not coded or re-validated; `SRLatch` convergence untested; LG02 lists the Lua class as `INVALID_INPUT`; rockspec source and dependency unpinned; LG02 section 9 matrix rows for ANDn/ORn/sequential; luacheck-style BUILD step | queued |
| 3 | LGE-PY-HARDEN | Python `logic-gates` hardening found by the lane review: bounded error text, metaclass-safe type names, exact-list mux4/mux8, LG02 section 9 rows | local, ready |
| 3 | LGE-EX-HARDEN | Elixir `logic_gates` hardening found by the lane review (LGE-S3b): `mux_n` with a non-power-of-two input count raises `MatchError`, `and_n`/`or_n`/`demux` raise `FunctionClauseError` for a non-list and `demux` accepts `n_outputs` below 1 (descending `0..(n-1)` range, `[1, 1]` for 0), a bad sequential state raises `KeyError`/`BadMapError`/`CaseClauseError` and `counter` reset skips state validation, `mux4` runs the caller's `Inspect` via `inspect(sel)` (`combinational.ex:136`), n-ary input cap and linear-time `shift_*` append, `--warnings-as-errors` in BUILD, 95% coverage gate and Hex-publishable `mix.exs`, rewrite `do_mux_n`/`ceil_log2` iteratively, matrix over `and_n`/`or_n` and the combinational and sequential blocks, LG02 section 7 row 12 and section 9 row updated to `InvalidBitError`, `shift_register` guarded (`is_list(state)`, element-shape and keyword-opts validation; today a caller `Enumerable` runs, a `Stream.cycle` state never returns, `%{}` returns `{[], []}`), `demux` rejects `n_outputs` outside a small positive range before `ceil_log2` (quadratic on bignums), improper-list errors made uniform | queued |
| 3 | LGE-PL-HARDEN | Perl `logic-gates` hardening found by the lane review: ShiftRegister/Register validate every state element before reading it (today `[0]`, `["x"]`, `[[]]` die with a plain Perl string at `LogicGates.pm:916` and `[undef]` is autovivified in the caller's state) and return canonical bits (`serial_out` returns the caller's dualvar); cap n-ary and register lengths (ANDn over 300000 ones takes about 6 s); `Error->new` blesses into `__PACKAGE__`, not the caller's invocant; wantarray guard for list-returning blocks; rewrite the three regex sites without patterns; add `required_capabilities.json`; LG02 section 9 rows (wide ints, Unicode digits, bad second argument with first 0, tied scalars); pinned CPAN install | queued |
| 3 | LGE-JAVA-NARY | Java N-ary gates: snapshot the input array, cap its length, report `inputs[i]` | local, ready |
| 3 | LGE-KT-NARY | Kotlin N-ary gates: snapshot the input, cap its length, report `inputs[i]` | local, ready |
| 3 | LGE-CS-NARY | C# N-ary gates: snapshot the input, cap its length, report `inputs[i]` | local, ready |
| 3 | LGE-CS-HARDEN | `csharp/logic-gates` hardening found by the lane review: tests under Debug and Release (coverage gate on Release), TreatWarningsAsErrors with WarningsAsErrors=nullable and AnalysisLevel 9.0-recommended, ImplicitUsings disabled, CheckForOverflowUnderflow enabled, null-safe `InvalidBitException` parameter name, invariant-culture messages (Message overridden), null-array tests for all three N-ary gates; deliberately no lock file, source mapping or target-framework change | local, finished (branch 795d7e93e4a) |
| 3 | LGE-DOTNET-SDK | Repo-wide .NET: bump the shared CI SDK pin (.github/workflows/ci.yml:4989-4992, 9.0.319) to the .NET 10 LTS SDK, retarget every C#/F# package to net10.0 (.NET 9 STS and .NET 8 LTS support ended 2026-11-10), and adopt NuGet lock files with `--locked-mode` plus package source mapping in every package | queued |
| 3 | LGE-FS-NARY | F# N-ary gates: snapshot the input, cap its length, report `inputs[i]` | local, ready |
| 3 | LGE-FS-HARDEN | F# `logic-gates` hardening found by the lane review: TreatWarningsAsErrors (FS0025, nullness), CheckForOverflowUnderflow, `LogicGates.fsi`, explicit FSharp.Core pin, Debug and Release tests (lock files, `global.json` and the supported TFM moved to LGE-DOTNET-SDK) | queued (starts after #19941 merges) |
| 3 | LGE-SWIFT-CAPS | Swift `logic-gates`: `maxInputs` cap on andN/orN and a `maxWidth` cap on shiftRegister, checked before any loop or allocation; `inputs[i]` payload naming (local branch `claude/swift-logic-gates-caps`, commits cf97c6031c9 and 60bc70d7e5a) | local, ready |
| 3 | LGE-SWIFT-HARDEN | Swift `logic-gates` hardening found by the lane review: iterative `muxN` (no recursion), the LG02 section 9 wide-integer rows 2^31, 2^32, 2^32+1 and 2^53+1 for all eleven gates, exact payload assertions for the block functions, a coverage threshold gate in BUILD, a release-configuration test run (`-c release -Xswiftc -enable-testing`) | queued |
| 5 | LGE-S3-FIX | VER15a fix batch from the review of LGE-S3 (shared-object 1321 subset, fail-closed eval subset, tighter ids) | local, ready |
| 7 | LGE-A-PYTHON | Python analyzer rules (`no_dynamic_eval_reachable`, `no_computed_key_writes_to_shared_objects`, banned constructs) with seeded negative-control corpora, and wiring the merged `ca-capability-analyzer` scanner into the report step | queued |
| 7 | LGE-A-TYPESCRIPT | TypeScript analyzer rules `ts.no-int32-bitwise-operators`, `ts.call-graph-acyclic`, `ts.import-allowlist`, `ts.no-prototype-keys`, plus the eval and computed-key rules, each with a seeded negative-control corpus | queued |
| 7 | LGE-A-GO | Go analyzer rule additions on the merged `ca-capability-analyzer`: nil-deref reachability, `unsafe.Slice`/`String`/`Add`/`StringData`/`SliceData`, `reflect` beyond Call/CallSlice/MethodByName (`Value.Set`, `NewAt`, `MakeFunc`), `//go:linkname` and `//go:noescape`, `.s`/`.c` files, import allowlist, no-mutable-package-state; seeded negative-control corpora; wired into BUILD and the report; run over BOTH `go/logic-gates` and `go/transistors` (it runs in-process); replace the file-name exemption for `gen_capabilities.go` with a "Code generated" header plus content-hash check | queued |
| 8 | LGE-PUB-GO | Go publish gate: subdirectory tags `code/packages/go/<dir>/vX.Y.Z`, publish `transistors` first, no `replace` directive, no `v0.0.0` requires, `go.sum` committed, consistent `go`/`toolchain` lines, `retract` as the only remedy for a bad first tag | queued |
| 3 | LGE-GO-HARDEN | `go/logic-gates` hardening found by the lane review: `ANDn`/`ORn`/`XORn` snapshot the input and cap its length (MaxInputs), LG02 wide-integer rows (2^32, 2^32+1, 2^53+1, MaxInt64), `go vet`/`-race`/gofmt in BUILD, a concurrency test, `Counter` output/state aliasing | local, in progress |
| 3 | LGE-GO-ALLOC | Shared code-generator `Operation` wrapper allocations (measured locally: `AND(1,1)` = 66, `NAND_XOR` = 169, `ANDn(1000)` = 65,938 allocations per call) | queued |
| 4 | LGE-GO-MUXN-ITER | Iterative `muxNInner` in `go/logic-gates` (today recursive, depth at most 16 behind the MaxWidth cap) | queued |
| 8 | LGE-PUB-NPM | npm publish gate rejects `file:` dependencies, verifies `@coding-adventures` scope ownership, and installs with `npm ci --ignore-scripts` | queued |
| 8 | LGE-PUB-SWIFT | SwiftPM publish gate rejects `.package(path:)` dependencies and a Package.swift that is not at the tagged repository root, and requires pinned remote dependencies; swift/logic_gates today has a path dependency on `../transistors` and lives in a monorepo subdirectory | queued |
| 8 | LGE-OCAML-REP-VERSION | OCAML07 representative chain hardcodes 0.1.0 in five places; parameterize before the first OCaml release | queued |
| 6 | LGE-CV2 | Dependencies owned by the portable-warrant queue and idle since 2026-11-20: S6b/S6c (Python v2 runner), S7 (TypeScript), Go v2 port, S9 lane validator and per-corpus registry; coordinate before taking any | queued |
| 6 | LGE-L-<lane> | Lane runner (v2) and logic-gates binding with BUILD negative controls, VER04i order, at most two lanes in flight | blocked on LGE-CV2 |
| 7 | LGE-S4 | VER08c grammar-driven capability analyzer spec | queued |
| 7 | LGE-A1 | Analyzer core plus rule tables for c, csharp, fsharp, haskell, java, perl, ruby; Haskell also needs a "no class constraints on exported functions" rule (CWE-1025 and CWE-843: a polymorphic signature would let a caller-defined instance reach a gate) | queued |
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
| 2026-11-24 | merges | #19924 Haskell gates, #19927 Perl validation, #19930 status, #19931 LG02, #19932 TypeScript, #19935 Go |
| 2026-11-24 | discoveries | review rounds found pre-existing DoS (unbounded 2^n decoders, uncapped widths) in Go, Python and Haskell, crashes escaping Either in Haskell, and unvalidated state in Go/Python/Haskell sequential code; logged above at priority 2-3 |
| 2026-11-24 | LG02 errata | drift guard: checks 10-12 gated until LGE-S1c; three BUILD variants parsed; recorded in LG02 by the LGE-S1b PR |
