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

## Kernel proofs of the gates (pointer)

Owner direction, 2026-11-24: the gates are the **first proof target** of the
trusted kernel, ahead of ct-compare. Kernel-checked proofs of the eleven gates
are [VER32](../VER32-proof-interchange-format.md) milestones **G-M1** (model
level: truth tables, NAND-built equivalence and rejection of every non-bit over
every integer, produced as Proof Interchange Format by an untrusted case-split
producer and checked by `lean-kernel`), **G-M2** (the Rust lane's gates lowered
to straight-line WIR, the theorems restated over the code) and **G-M3** (half
adder, full adder and an n-bit ripple-carry adder correct for every width by
induction). They sit beside this program's lane work: the lanes other than Rust
reach `tested` by the queue below, and VER32's model-level theorems are the
statement all lanes' integer ranges are instances of. The backlog rows are in
`VER-BACKLOG.md`, section "Proof interchange format (VER32) and first proof
target (logic gates)".

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

Priority legend (re-mapped 2026-11-24, LGE-003): **P1** truthfulness and security
defects and blockers; **P2** hardening, runners, analyzers, publishing;
**P3** hygiene. `-` marks done rows. Rows stay in rough execution order.
State vocabulary: `done, #N` (merged), `open, #N`, `local, ready (branch ...)`
(implemented and security-reviewed locally, queued for push because CI is
overwhelmed), `local, in progress`, `blocked on ...`, `queued`, `unverified`.

| Pri | ID | Work | State |
|---|---|---|---|
| - | LGE-000 | This backlog shard and the VER-BACKLOG pointer | done, #19922 |
| - | LGE-P1 | Perl: canonical type-and-value bit validation, arity, no invocant guessing, bounded widths, analog input validation in transistors; 2,654 + 607 tests | done, #19927 |
| - | LGE-H1 | Haskell: xnor and the four NAND-built gates, full tests | done, #19924 |
| - | LGE-C1 | New `c/logic-gates` (status code, no pointers), 1,351,741 checks under strict warnings and ASan/UBSan | done, #19926 |
| - | LGE-CPP1 | New `cpp/logic-gates` (`constexpr`, `std::invalid_argument`, deleted non-int overloads), 96,454 checks | done, #19928 |
| - | LGE-S1a | LG02 cross-lane contract (separate corpus-only `logic-gates-basic.warrant`, drift guard, v2 corpus, per-lane error identities) and the VER04i 13.2 amendment (was LGE-S2) | done, #19931 |
| - | LGE-R-python | Exhaustive rejection and type-confusion matrix; `type(x) is int` | done, #19933 |
| - | LGE-R-typescript | Every gate validates its own inputs (NAND/NOR/XNOR relied on transistors); reject `-0`; matrix | done, #19932 |
| - | LGE-R-go | Typed panic value (today a plain string at `gates.go:92`, and a different one at `:154`, so a recover adapter cannot tell them apart); `assertPanics` checks its message; matrix | done, #19935 |
| - | LGE-ID-perl | Stable error identity: `CodingAdventures::LogicGates::Error` with a code (bindings may not match message text) | done, #19944 |
| - | LGE-ID-lua | Stable error identity (error table with `code`); reject `1.0`; matrix | done, #19936 |
| - | LGE-ID-haskell | `LogicGateError` ADT instead of `Left String` | done, #19942 |
| - | LGE-R-java | `InvalidBitException`, exhaustive tests, JaCoCo 0.8.12 gate | done, #19937 |
| - | LGE-R-kotlin | `InvalidBitException`, exhaustive tests, JaCoCo gate | done, #19938 |
| - | LGE-R-csharp | `InvalidBitException`, exhaustive tests, 95% gate | done, #19939 |
| - | LGE-R-fsharp | `InvalidBitException`, exhaustive tests, 95% gate | done, #19941 |
| - | LGE-R-swift | Exhaustive truth tables and bit rejection with exact `invalidBit(name:got:)` payloads, no crash on oversized selects (caps at 16 select bits and 2^16 lines) | done, #19955 |
| P2 | LGE-R-<lane> | Lane-local exhaustive suites for ruby, swift, elixir, dart, ocaml (OCaml: literal expected values, not `land`/`lor`) | done for ruby #19950, elixir #19952, dart #19953; ocaml in review, #19949 (see LGE-R-ocaml) |
| - | LGE-HS-DEPS | Haskell dependents (security, pre-existing): `alu [] []` returns Right with bottom flags; `rippleCarryAdder [] [] 5` returns an unvalidated carry; `isNegative` checks only the head; `newBRAM` width 0 throws DivideByZero out of the Either; RAM ticks never validate data/clock/write-enable (stores 5); `configureLUT` does not validate its truth table; RAM sizes uncapped | done, #19948 |
| P1 | LGE-DOS-ALL | Cap decoder/demux/mux select bits at 16 and sequential widths in every lane with these blocks (Rust SEC-008, Go and Haskell done in-PR; Python confirmed unbounded) | partly done: python #19946, typescript #19958, lua #19957, go #19935, haskell #19942, swift #19955; local, ready for ruby (claude/ruby-logic-gates-bounds), elixir (claude/elixir-logic-gates-bounds), dart (claude/dart-logic-gates-bounds); ocaml blocked on #19949; java/kotlin/csharp/fsharp have no combinational or sequential modules (audited, nothing to fix) |
| - | LGE-PY-SEQ-COMB | Python: counter returns unvalidated state bits; encoders accept any sized container | done, #19946 |
| - | LGE-LUA-SEQ | Lua `sequential.lua` still has free-text validation accepting `1.0` | done, #19957 |
| P2 | LGE-ID-perl-transistors | Perl transistors needs a typed error identity too | local, ready (branch claude/perl-transistors-error-class) |
| - | LGE-LUA-STALE | Lua BUILD install guards skip reinstall when an old rock exists, so a cached tree could test dependents against the old validator | done, #19957 (fresh installs) |
| P3 | LGE-HS-COUNTER | Haskell `counter 1 0 [0,1]` returns `[0,1]`, not `[1,0]` (two-phase flip-flop emulation); investigate | queued, unverified |
| P2 | LGE-HS-HARDEN | Haskell `logic-gates` hardening found by the lane review, in three sub-items. (a) code: bounded forced snapshots of every caller list with caps (`decoder (cycle [0])` never returns; encoder, priorityEncoder, mux4, mux8, muxN inputs and the exported `validateBits` are uncapped), shiftRegister and counter reject an empty state instead of defaulting to four zeros, un-export or newtype the validators' name argument, document the caller-bottom contract (`andGate 1 (error "boom")` throws out of the Either) with an `evaluate`/`try` test, LG02 wide rows and a rejection table for every block, hpc coverage, `required_capabilities.json`. (b) build flags: `ghc-options: -Wall -Wcompat -Wincomplete-patterns -Wincomplete-uni-patterns -Werror` (check the test suite under CI GHC 9.4.8), `{-# LANGUAGE Safe #-}` on every module, BUILD fails when `cabal` is missing instead of printing "skipping" and exiting 0, a GHC version matrix. (c) supply chain: `cabal.project.freeze` or index-state, tighter hspec bound | local, ready (branch claude/haskell-logic-gates-harden); security-reviewed |
| - | LGE-S1b | `logic-gates-basic.warrant` and `code/scripts/logic_gates_basic_drift.py` with its tests and build-input wiring | done, #19945 |
| - | LGE-S1c | Generated `logic-gates-basic-warrant-v2` fixture, build-tool row (optional `bindings`), `--check` and `verify-v2` BUILD lines | done, #19994 |
| P2 | LGE-S1d | warrant-corpus-gen: give rejection claims a real witness instead of the constant-Err stand-in so shift-mask (p mod 8, identity on {0,1}) is killable by the corpus; remove KNOWN_STAND_IN_BLIND; MUST land before any lane binds the shift-mask fault | queued |
| - | LGE-S3 | VER15 amendment, `weakness-registry-v2.json` (CWE-20, CWE-843, CWE-704), per-lane matrix section: spec [VER15a](../VER15a-weakness-registry-v2-and-lane-matrices.md), registry, lane threat-list schema and the worked `c` file written (not yet merged); section 9 lists what the validator (new item, queued) must check | done, #19954 |
| P1 | LGE-C2 | `c/logic-gates`: `warn_unused_result` / `[[nodiscard]]` on every `lg_*` function (the `-1` status is truthy, CWE-252); found by the VER15a review | local, ready (branch claude/c-logic-gates-nodiscard); security-reviewed |
| P1 | LGE-C3 | `c/logic-gates` `tools/run.sh`: a skipped sanitizer run (lines 58-60) fails CI; MSan/TSan where supported; found by the VER15a review | local, ready (branch claude/c-logic-gates-nodiscard) (with LGE-C2); security-reviewed |
| P2 | LGE-CPP-EXT | C++ memory-safety extension `cpp_address_surface_allowlist` (VER15a section 5.5; allow-list is exactly `detail::reject`) and its analyzer rule `analyzer-rule:cpp.no-address-surface` (references, `this`, containers, `string_view`/`span`, iterators, lambdas by reference, `new`, RAII) | queued (spec amendment is in the LGE-S3b fold; analyzer rule not started) |
| P2 | LGE-CPP-HARDEN | `cpp/logic-gates` hardening found by the lane review: deleted `std::initializer_list` overloads (braced arguments such as `and_gate({true}, 1)` skip the deleted templates), compile-must-fail tests, a hostile-macro include test (`#define check(x)`), an `-O2` run, a skipped sanitizer run fails CI, `[[nodiscard]]`, versioned inline namespace | local, ready (branch claude/cpp-logic-gates-harden); security-reviewed; Windows run.ps1 not run locally (see LGE-CPP-WIN) |
| P2 | LGE-CPP-MATRIX | `cpp/logic-gates` extended type-confusion matrix (LG02 section 9): unscoped and scoped enum, `wchar_t`, `char16_t`, `char32_t`, `char8_t`, `unsigned short`, `unsigned long long`, `long double`, `nullptr_t`, string literal, class with `operator int`, braced-list and function-pointer rows | queued, unverified (the hardening branch carries a 512-snippet reject matrix; reconcile with LG02 section 9 rows before closing) |
| P2 | LGE-S3v | Validator `code/scripts/validate_weakness_lanes.py` with falsification tests (VER15a section 9) | queued |
| P2 | LGE-S3b | Per-language threat lists `code/specs/data/weakness-registry-v2/lanes/<lang>.json` (draft agent plus independent adversarial reviewer per lane) | local, ready (branch claude/lge-s3b-all) (all 18 lanes, every row open, no live evidence); needs /security-review at push |
| P1 | LGE-R-ocaml | OCaml exact row for the lane-local exhaustive suite (the `LGE-R-<lane>` template row has no `(lanes: ...)` marker, so each lane needs an exact row, VER15a rule 13): literal truth tables, exhaustive rejection with exact `Invalid_bit` name and value, NAND-built gates validate their own arguments (fixes `nand_or 1 2` reporting parameter `a`; also `shift_register` reporting a bad `slave_q` in caller state as `data`) | open, #19949 (OCaml CI archive-mirror fix merged into the PR; Windows solve was failing on an upstream checksum change) |
| P2 | LGE-OCAML-HARDEN | OCaml `logic-gates` hardening found by the lane review: replace `List.nth`/`List.hd`/`List.tl`/`assert false` with total code, rewrite the non-tail `increment_lsb` and every `let rec` as folds/loops, `List.equal Int.equal` instead of polymorphic `<>` (`ml:370`), explicit `-w +a -warn-error +a` in the dune files plus a no `-unsafe`/`-noassert` check, `--require-checksums` and an opam lock file in BUILD, an allocation microbenchmark, a no-exception sweep test, bounded traversal of caller lists (`List.compare_length_with l (cap+1)` before any full walk, so a cyclic list `let rec l = 1 :: l` returns an error instead of hanging) and caps on `mux_n`/`encoder`/n-ary input list lengths, `log2_exact` made total, and a README line that the gates are not constant time (the width caps themselves are LGE-DOS-ALL) | blocked on #19949 (same package); then queued |
| P2 | LGE-A-OCAML | OCaml analyzer rules beyond the merged `ca-capability-analyzer` (Obj, `Marshal.from_*`, `external`, `Dynlink.loadfile*` and the capability table are already enforced in CI): stdlib-only import allowlist, Toploop/compiler-libs eval ban, unsafe-accessor ban, partial-function ban, no polymorphic compare, no `let rec`, step-bound-linear-in-input, every-gate-validates-every-parameter-first, no-hash-or-sort-keyed-by-caller-data, no-secret-inputs-declared, no concurrency or mutable state, no numeric conversions, no formatting/serialization, value-range rule for `+`/`-`/`lsl`; plus reading the analyzer result at report time | queued |
| P2 | LGE-TS-HARDEN | TypeScript `logic-gates` hardening found by the lane review: `validateBit` parameter name, mux/dmux arity checks, `InternalGateError`, `tsc` in BUILD, n-ary input cap | local, ready (branch claude/typescript-logic-gates-harden); security-reviewed |
| P2 | LGE-LUA-HARDEN | Lua `logic-gates` hardening found by the lane review (all reproduced on Lua 5.5.0): `Register`/`Counter` trust `rawlen` on tables with holes (`{1, nil, 1}` silently becomes one bit); `ANDn`/`ORn` uncapped and arity below 2 is a position-prefixed string; composite gates call `gates.NAND` etc. through the mutable module table (patching one entry changes `NAND_AND`) and `error`, `tostring`, `table.pack`, `ipairs` are looked up as globals at call time (`_G.error = function() end` makes `AND("x", 1)` return instead of raise: capture at load, test replacing globals); read-only module tables; public `raise_invalid_*` and mutable `err.value` reaching `__tostring`; `ANDn` "stack overflow" at ~900k args; error tables forgeable and mutable; CMOS result and internal-failure branch not coded or re-validated; `SRLatch` convergence untested; LG02 lists the Lua class as `INVALID_INPUT`; rockspec source and dependency unpinned; LG02 section 9 matrix rows for ANDn/ORn/sequential; luacheck-style BUILD step | local, ready (branch claude/lua-logic-gates-harden); security-reviewed |
| P2 | LGE-DART-HARDEN | Dart `logic-gates` hardening found by the lane review: `muxN` returns an unvalidated second read of a caller `List` element (a `ListBase` subclass makes it return 7); `xorN` of a one-element hostile list returns the unvalidated second read of `inputs.first` (gates.dart:114-118; 7 on VM and dart2js); `andN`/`orN` iterate the list twice and report a hostile list as `a`; `xorN` and `_requireAtLeastTwo` re-read `inputs.length` every iteration (a growing-length list never terminates); caller-implemented `FlipFlopState`/`CounterState` are validated by one getter read and used by another (sequential.dart:72, :196-197); document and test that `InvalidBitError` is an `Error`, not an `Exception`; List<dynamic>/List<Object?> rows (TypeError); dart2wasm in the platform matrix; read each index once, cap the input length, build the `inputs[i]` name only on failure; `decoder` of 64 zero bits returns an empty list on the VM (32 on dart2js) and 63 throws `RangeError`; exported `validateBit` echoes a caller-chosen `name`; `analysis_options.yaml` (strict-casts, strict-inference, strict-raw-types, avoid_dynamic_calls), `dart test -p node` web run, LG02 section 9 matrix on all 13 callables plus the N-ary gates and mux, `dart pub get --enforce-lockfile`, `required_capabilities.json` | local, ready (branch claude/dart-logic-gates-harden); security-reviewed |
| P2 | LGE-PY-HARDEN | Python `logic-gates` hardening found by the lane review: bounded error text, metaclass-safe type names, exact-list mux4/mux8, LG02 section 9 rows | local, ready (branch claude/python-logic-gates-harden); security-reviewed |
| P2 | LGE-EX-HARDEN | Elixir `logic_gates` hardening found by the lane review (LGE-S3b): `mux_n` with a non-power-of-two input count raises `MatchError`, `and_n`/`or_n`/`demux` raise `FunctionClauseError` for a non-list and `demux` accepts `n_outputs` below 1 (descending `0..(n-1)` range, `[1, 1]` for 0), a bad sequential state raises `KeyError`/`BadMapError`/`CaseClauseError` and `counter` reset skips state validation, `mux4` runs the caller's `Inspect` via `inspect(sel)` (`combinational.ex:136`), n-ary input cap and linear-time `shift_*` append, `--warnings-as-errors` in BUILD, 95% coverage gate and Hex-publishable `mix.exs`, rewrite `do_mux_n`/`ceil_log2` iteratively, matrix over `and_n`/`or_n` and the combinational and sequential blocks, LG02 section 7 row 12 and section 9 row updated to `InvalidBitError`, `shift_register` guarded (`is_list(state)`, element-shape and keyword-opts validation; today a caller `Enumerable` runs, a `Stream.cycle` state never returns, `%{}` returns `{[], []}`), `demux` rejects `n_outputs` outside a small positive range before `ceil_log2` (quadratic on bignums), improper-list errors made uniform | local, ready (branch claude/elixir-logic-gates-harden); security-reviewed |
| P2 | LGE-PL-HARDEN | Perl `logic-gates` hardening found by the lane review: ShiftRegister/Register validate every state element before reading it (today `[0]`, `["x"]`, `[[]]` die with a plain Perl string at `LogicGates.pm:916` and `[undef]` is autovivified in the caller's state) and return canonical bits (`serial_out` returns the caller's dualvar); cap n-ary and register lengths (ANDn over 300000 ones takes about 6 s); `Error->new` blesses into `__PACKAGE__`, not the caller's invocant; wantarray guard for list-returning blocks; rewrite the three regex sites without patterns; add `required_capabilities.json`; LG02 section 9 rows (wide ints, Unicode digits, bad second argument with first 0, tied scalars); pinned CPAN install | local, ready (branch claude/perl-logic-gates-harden); security-reviewed (Counter cap 64, parity with Rust) |
| P1 | LGE-RUST-HARDEN | Rust `logic-gates` hardening found by the lane review (rust threat list): `#![forbid(unsafe_code)]`; analyzer run over `transistors` and a real report closure (`ca-assurance-report/src/lib.rs:117`); validate `counter`'s `reset` and every sequential bit argument, with release-profile rejection tests; build `validate_bits` names only on failure; replace `(n as f64).log2()` with `trailing_zeros()` (LG-001) and deny the clippy cast lints; make `mux_n` iterative and correct the merged v1 `assurance_report.json` through its canonical generator (CWE-674 is over-excluded: `mux_n_inner` is recursive; memory-class reasons must say the evidence is reachability of `unsafe` in the package's own sources, closure being the package alone); add `--verify-cert` to the BUILD (the report's analyzer status comes from the checked-in effect certificate, never re-verified there); validate `counter`'s `reset` (done below too); cap `CounterState::new` width; build `validate_bits` names only on failure; typed panic payload; `rust-version` and `--locked`; cargo-audit/deny; a release-profile report manifest. Also tracks the new Rust analyzer rules the rust rows name (call-graph acyclic, no unchecked arithmetic, index guards, library dependency allowlist) until they get their own rows **Scope note:** the merged v1 Rust assurance report is wrong in two ways (CWE-674 is marked excluded although `mux_n_inner` recurses at `combinational.rs:250-263`; memory-class reasons say "reachability" while the closure is the package alone) and must be regenerated through the canonical tooling with those rows opened; counter reset is never validated (`sequential.rs:422`). | local, in progress (iterative mux_n, regenerated v1 report, --verify-cert, counter reset/cap); fixes the v1 report CWE-674 over-exclusion |
| P1 | LGE-JAVA-NARY | Java N-ary gates: snapshot the input array, cap its length, report `inputs[i]` | local, ready (branch claude/java-logic-gates-nary); security-reviewed |
| P1 | LGE-KT-NARY | Kotlin N-ary gates: snapshot the input, cap its length, report `inputs[i]` | local, ready (branch claude/kotlin-logic-gates-nary); security-reviewed |
| P1 | LGE-CS-NARY | C# N-ary gates: snapshot the input, cap its length, report `inputs[i]` | local, ready (branch claude/csharp-logic-gates-harden) (with LGE-CS-HARDEN; supersedes claude/csharp-logic-gates-nary) |
| P2 | LGE-CS-HARDEN | `csharp/logic-gates` hardening found by the lane review: tests under Debug and Release (coverage gate on Release), TreatWarningsAsErrors with WarningsAsErrors=nullable and AnalysisLevel 9.0-recommended, ImplicitUsings disabled, CheckForOverflowUnderflow enabled, null-safe `InvalidBitException` parameter name, invariant-culture messages (Message overridden), null-array tests for all three N-ary gates; deliberately no lock file, source mapping or target-framework change | local, ready (branch claude/csharp-logic-gates-harden); security-reviewed |
| P2 | LGE-DOTNET-SDK | Repo-wide .NET: bump the shared CI SDK pin (.github/workflows/ci.yml:5010-5013, 9.0.319) to the .NET 10 LTS SDK, retarget every C#/F# package to net10.0 (.NET 9 STS and .NET 8 LTS support ended 2026-11-10), and adopt NuGet lock files with `--locked-mode` plus package source mapping in every package | queued (own PR, touches every .NET lane) |
| P2 | LGE-FS-NARY | F# N-ary gates: snapshot the input, cap its length, report `inputs[i]` | local, ready (branch claude/fsharp-logic-gates-harden) (with LGE-FS-HARDEN; supersedes claude/fsharp-logic-gates-nary) |
| P2 | LGE-FS-HARDEN | F# `logic-gates` hardening found by the lane review: TreatWarningsAsErrors (FS0025, nullness), CheckForOverflowUnderflow, `LogicGates.fsi`, explicit FSharp.Core pin, Debug and Release tests (lock files, `global.json` and the supported TFM moved to LGE-DOTNET-SDK) | local, ready (branch claude/fsharp-logic-gates-harden); security-reviewed (#19941 merged) |
| P2 | LGE-SWIFT-CAPS | Swift `logic-gates`: `maxInputs` cap on andN/orN and a `maxWidth` cap on shiftRegister, checked before any loop or allocation; `inputs[i]` payload naming (local branch `claude/swift-logic-gates-caps`, commits cf97c6031c9 and 60bc70d7e5a) | local, ready (branch claude/swift-logic-gates-harden) (with LGE-SWIFT-HARDEN; supersedes claude/swift-logic-gates-caps); covers the andN/orN/shiftRegister cap parity item |
| P2 | LGE-SWIFT-HARDEN | Swift `logic-gates` hardening found by the lane review: iterative `muxN` (no recursion), the LG02 section 9 wide-integer rows 2^31, 2^32, 2^32+1 and 2^53+1 for all eleven gates, exact payload assertions for the block functions, a coverage threshold gate in BUILD, a release-configuration test run (`-c release -Xswiftc -enable-testing`) | local, ready (branch claude/swift-logic-gates-harden); security-reviewed |
| P1 | LGE-S3-FIX | VER15a fix batch from the review of LGE-S3 (shared-object 1321 subset, fail-closed eval subset, tighter ids) | local, ready (inside claude/lge-s3b-all, which supersedes claude/lge-s3-fix-batch) |
| P2 | LGE-A-PYTHON | Python analyzer rules (`no_dynamic_eval_reachable`, `no_computed_key_writes_to_shared_objects`, banned constructs) with seeded negative-control corpora, and wiring the merged `ca-capability-analyzer` scanner into the report step | queued |
| P2 | LGE-A-TYPESCRIPT | TypeScript analyzer rules `ts.no-int32-bitwise-operators`, `ts.call-graph-acyclic`, `ts.import-allowlist`, `ts.no-prototype-keys`, plus the eval and computed-key rules, each with a seeded negative-control corpus | queued |
| P2 | LGE-A-GO | Go analyzer rule additions on the merged `ca-capability-analyzer`: nil-deref reachability, `unsafe.Slice`/`String`/`Add`/`StringData`/`SliceData`, `reflect` beyond Call/CallSlice/MethodByName (`Value.Set`, `NewAt`, `MakeFunc`), `//go:linkname` and `//go:noescape`, `.s`/`.c` files, import allowlist, no-mutable-package-state; seeded negative-control corpora; wired into BUILD and the report; run over BOTH `go/logic-gates` and `go/transistors` (it runs in-process); replace the file-name exemption for `gen_capabilities.go` with a "Code generated" header plus content-hash check | queued |
| P1 | LGE-PUB-GO | Go publish gate: subdirectory tags `code/packages/go/<dir>/vX.Y.Z`, publish `transistors` first, no `replace` directive, no `v0.0.0` requires, `go.sum` committed, consistent `go`/`toolchain` lines, `retract` as the only remedy for a bad first tag **Blocker (VER31a):** `go.mod` `replace .../transistors => ../transistors` is ignored by consumers and `require ... v0.0.0` cannot resolve; no `go.sum`; `go 1.26` with no toolchain line. Publish `transistors` first under a real tag (`code/packages/go/transistors/vX.Y.Z`) and require that version before tagging logic-gates. Publish row (VER31a): Go proxy: tag `code/packages/go/logic-gates/vX.Y.Z`, remove `replace` and `v0.0.0` on transistors (B1, B7); owner signs the tag; no account | queued; BLOCKED for any release by the Go publishing blocker below (publish row: queued) |
| P2 | LGE-GO-HARDEN | `go/logic-gates` hardening found by the lane review: `ANDn`/`ORn`/`XORn` snapshot the input and cap its length (MaxInputs), LG02 wide-integer rows (2^32, 2^32+1, 2^53+1, MaxInt64), `go vet`/`-race`/gofmt in BUILD, a concurrency test, `Counter` output/state aliasing | local, ready (branch claude/go-logic-gates-harden); security-reviewed (drop -race in BUILD_windows if Windows CI lacks cgo) |
| P2 | LGE-GO-ALLOC | Shared code-generator `Operation` wrapper allocations (measured locally: `AND(1,1)` = 66, `NAND_XOR` = 169, `ANDn(1000)` = 65,938 allocations per call) | queued |
| P3 | LGE-GO-MUXN-ITER | Iterative `muxNInner` in `go/logic-gates` (today recursive, depth at most 16 behind the MaxWidth cap) | queued |
| P2 | LGE-A-DART | Dart analyzer rules (`dart.no-ffi-mirrors-isolates-native`, `dart.import-allowlist-no-ambient-capabilities`, `dart.no-mirrors-spawnuri-function-apply-nosuchmethod`, `dart.list-parameters-read-once-per-index`, `dart.integer-value-range-and-shift-bounds`, `dart.call-graph-acyclic-loops-bounded-by-input` and the rest of the lane list), the dynamic-code rule with a seeded negative-control corpus; needs the dart `.tokens`/`.grammar` of LGE-A2 first | queued |
| P2 | LGE-PUB-NPM | npm publish gate rejects `file:` dependencies, verifies `@coding-adventures` scope ownership, and installs with `npm ci --ignore-scripts`. Publish row (VER31a): npm: `@coding-adventures/logic-gates` with provenance; owner: `coding-adventures` org, first version by hand, trusted publisher; `file:../transistors` dep (B1) | queued (publish row: queued) |
| P2 | LGE-PUB-SWIFT | SwiftPM publish gate rejects `.package(path:)` dependencies and a Package.swift that is not at the tagged repository root, and requires pinned remote dependencies; swift/logic_gates today has a path dependency on `../transistors` and lives in a monorepo subdirectory. Publish row (VER31a): Swift: blocked by the root `Package.swift` requirement (B6); owner decides mirror repo, shim or defer (O3) | blocked on owner decision (O3); queued; blocked on owner decision |
| P2 | LGE-OCAML-REP-VERSION | OCAML07 representative chain hardcodes 0.1.0 in five places; parameterize before the first OCaml release Five places carry the version: opam file, `PACKAGES` in `ocaml_representative_ci.py`, `manifest.json`, the workflow `0.1.0` literals, the spec. | queued (#19949 kept 0.1.0 as a workaround) |
| P2 | LGE-CV2 | Dependencies owned by the portable-warrant queue and idle since 2026-11-20: S6b/S6c (Python v2 runner), S7 (TypeScript), Go v2 port, S9 lane validator and per-corpus registry; coordinate before taking any | queued, owned by the portable-warrant queue |
| P2 | LGE-L-<lane> | Lane runner (v2) and logic-gates binding with BUILD negative controls, VER04i order, at most two lanes in flight | blocked on LGE-CV2 and LGE-LG02-ERRATA |
| P2 | LGE-S4 | [VER08c](../VER08c-grammar-driven-capability-analyzer.md): grammar-driven capability analyzer spec (rule-table format, two modes, output contract, C rules, per-lane rollout) | open, PR pending (branch claude/lge-s4-grammar-analyzer-spec) |
| P2 | LGE-A1 | Analyzer core plus rule tables for c, csharp, fsharp, haskell, java, perl, ruby; Haskell also needs a "no class constraints on exported functions" rule (CWE-1025 and CWE-843: a polymorphic signature would let a caller-defined instance reach a gate) | queued; split into the LGE-A0, LGE-A1-core, LGE-A1-<lane> and LGE-A1-c-* rows below (VER08c) |
| P2 | LGE-A2 | `.tokens` then `.grammar` for cpp, kotlin, elixir, lua, swift, dart | queued; split into the LGE-A2-tokens/rules/grammar-<lane> rows below (VER08c) |
| P2 | LGE-A0 | VER08c schemas: `rule-table.schema.json`, report schema `ca-grammar-analyzer-report-v1`; extend the `banned_construct_exceptions` language enum only if a package needs an exception; loader behaviours: `max_evidence` is forced to `claimed` for tokens-mode rules, version-gap lanes and `token-whitelist` rules (an author-supplied `live` is overridden or refused), with a unit test that a tokens-mode rule declaring `max_evidence: live` is refused or forced to `claimed` | queued (blocked on LGE-S4) |
| P2 | LGE-A1-core | `rust/ca-grammar-analyzer` crate: rule loader, token and node matchers, text and JSON report, bounds, negative-control runner, CLI, own manifest | queued (blocked on LGE-A0) |
| P2 | LGE-A1-<lane> | Rule table, controls and BUILD line, one lane per PR: java first (grammar already matches Java 21), then fsharp, haskell, csharp (C# 13 vs a C# 12 grammar), perl (no 5.26 grammar), ruby (3.4 vs 3.3 grammar) | queued (blocked on LGE-A1-core) |
| P2 | LGE-A1-c-g / -c-r / -c-sem | C: header grammar extension decided by measuring the real package; the active rules (`c.no-shift-operators`, `grammar-is-allowlist`, include allowlist); then SIR27-based `c.integer-value-range` and `c.no-ub-reachable` | queued (blocked on the C package PR #19926) |
| P2 | LGE-A2-tokens-<lane> | `.tokens` file, generated lexer crate and lexical corpus for cpp, kotlin, elixir, lua, swift, dart (six PRs) | queued |
| P2 | LGE-A2-rules-<lane> | Tokens-only rule table and controls per lane | queued (blocked on LGE-A1-core and the lane's tokens PR) |
| P2 | LGE-A2-grammar-<lane> | `.grammar` and parser crate; lane moves to AST mode | queued |
| P2 | LGE-A3-gate | Independent protected CI gate running the analyzer over every package of the 13 lanes | queued (blocked on LGE-A1-core) |
| P2 | LGE-A0-codeowners | CODEOWNERS on the VER08c rule tables, analyzer crate and gate workflow | queued |
| P2 | LGE-A3-base-gate | Gate runs the base-branch analyzer and tables on the PR tree and emits an attested report | queued |
| P2 | LGE-A1-closed-world | Closed-world identifiers or prelude-export allowlists for every lane except c and cpp | queued |
| P2 | LGE-A1-buildfiles | Per-lane build-file schemas or scanning; repo BUILD classified; dependency pin check | queued |
| P2 | LGE-A1-directives | Per-lane pragma, attribute and annotation allowlists with controls | queued |
| P2 | LGE-W1-attest | LGE-W1 verifies report hashes against the gate attestation | queued |
| P2 | LGE-W1 | Per-lane v2 weakness matrices in `ca-assurance-report` | queued |
| P2 | LGE-S5 | [VER31a](../VER31a-registry-publishing.md): per-registry publish jobs behind the VER31 gate, tag grammar, owner actions, findings B1-B9 | open, PR pending (branch claude/lge-s5-publishing-spec) |
| P2 | LGE-OWNER-NAMES | **Owner action, before every LGE-PUB row; agents never create accounts, tokens or reservations.** Claim or reserve the whole dependency closure's names (logic-gates and transistors, B1) on every registry (VER31a section 1.3): the npm `@coding-adventures` org/scope; the NuGet `CodingAdventures.` prefix; Maven namespace verification before any build; a pub.dev verified publisher; the PAUSE `CodingAdventures::` namespace; the Hex, LuaRocks, Hackage and opam names; typosquat variants (underscore, hyphen, prefix); record each in `code/assumptions/first-party-names.json` with its expected owner | owner action |
| P2 | LGE-OWNER-ACCOUNTS | Owner creates accounts, namespaces and trusted publishers (never the coordinator): crates.io account and name claim; npm `coding-adventures` org; one GitHub environment per registry (`release-crates`, `release-npm`, `release-pypi`, `release-rubygems`, `release-maven`, `release-nuget`, `release-hex`, `release-luarocks`, `release-cpan`, `release-hackage`, `release-pubdev`, `release-opam`, `release-source`); PyPI trusted publishers pinned to `release-pypi`; RubyGems and NuGet trusted publishing (required); owner-manual uploads for crates.io, Hex, LuaRocks, CPAN, pub.dev; dedicated machine GitHub account and opam fork; Go signed-tag ruleset; Maven Central portal account, namespace verification and GPG key; NuGet account and `CodingAdventures.` prefix; hex.pm account and API key; luarocks.org account and API key; PAUSE (CPAN) account; Hackage account and token; pub.dev account and automated-publishing setup; GitHub account able to PR `ocaml/opam-repository`; Swift mirror repo if chosen; C/C++ signing; the per-registry `release-*` environments, tag-protection ruleset, machine account, `.github/release-signers` and CODEOWNERS (VER31a section 1.2). Names are the separate row LGE-OWNER-NAMES, ordered first. Also decisions O1-O6 (VER31a section 6) | owner action |
| P1 | LGE-OWNER-INTERIM | Owner actions now, ahead of the gate: scope and rotate `RUBYGEMS_API_KEY`, move it into the `rubygems` environment, create the tag ruleset on `refs/tags/python/**` and `refs/tags/ruby/**`, enable hardware-key 2FA on the owner account (VER31a 5.5) | owner action |
| P2 | LGE-PUB-validator | `code/scripts/tests/` validator over `publish.yml`: triggers are exactly `push: tags` (allow-list) plus `workflow_dispatch`; every publish and `attest` job `if:` has `event_name == 'push'` and `needs.gate.result == 'success'` (plus verify-tag, attest); no `always()`, `!cancelled()` or `failure()`; every publish job needs `gate`; no publish verb in `gate` (VER31a 4.1 rules 0 and 2) | queued, with LGE-PUB-gate |
| P2 | LGE-PUB-go-helper | `code/scripts/go_release_tag.sh` with the normative step order and falsification tests of VER31a 3.4 (fetch, resolve one SHA, tag absent locally and remotely, ancestry, `go.mod` read from the object, HEAD equals SHA, gate run bound to SHA/package/lane and current `publish.yml` blob, `git tag -s`, push, re-check remote target). Plus the detective observer as a scheduled workflow on `main` (not in the tag's workflow file) | queued |
| P2 | LGE-PUB-names-map | `code/assumptions/first-party-names.json` (lane, registry, name, expected owner, verification method) and `first-party-digests.json` (the committed trust anchor) under CODEOWNERS, and the gate's fail-closed owner and digest check with its falsification tests (VER31a 2.3 item 2) | queued, with LGE-PUB-gate |
| P2 | LGE-PUB-gate | Wire the VER31 gate (LV-007a) and the multi-lane `parse-tag` allow-list, Go tag shape and `registry` output (VER31a sections 2-3) before any registry row below | blocked on LV-007a |
| P2 | LGE-PUB-crates | crates.io: `rust/logic-gates`; needs a version on the `transistors` path dep (B1), a `license` field, name decision O6; owner claims the name by a hand-published first version | queued; owner: crates.io account |
| P2 | LGE-PUB-pypi | PyPI: `coding-adventures-logic-gates`, existing job moved under the gate and the `release-pypi` environment; owner: trusted publisher pinning `release-pypi` | queued |
| P2 | LGE-PUB-rubygems | RubyGems: `coding_adventures_logic_gates`; trusted publishing in place of `RUBYGEMS_API_KEY`; transistors gem dep (B1); owner: trusted publisher pinned to `release-rubygems` (key only as a time-boxed fallback) | queued |
| P2 | LGE-PUB-maven | Maven Central for java and kotlin: distinct artifact ids (B2), namespace decision (B3, O2), signed bundle to the Central portal; owner: portal account, namespace verification, GPG key and token secrets | queued; the Kotlin runtime-dependency POM requirement is the row LGE-PUB-KOTLIN |
| P2 | LGE-PUB-nuget | NuGet for csharp and fsharp: `.nupkg` push, trusted publishing or key; owner: nuget.org account, ID prefix reservation | queued |
| P2 | LGE-PUB-hex | Hex: `coding_adventures_logic_gates`; add package metadata to `mix.exs`; path dep on transistors (B1); owner: hex.pm account and API key | queued |
| P2 | LGE-PUB-luarocks | LuaRocks: `coding-adventures-logic-gates`; transistors rock dep (B1); owner: luarocks.org account and API key | queued |
| P2 | LGE-PUB-cpan | CPAN: `CodingAdventures-LogicGates` via PAUSE; transistors dist dep (B1); owner: PAUSE account | queued |
| P2 | LGE-PUB-hackage | Hackage: rename `logic-gates` (B4, O6), `cabal sdist` and upload; owner: Hackage account and token | queued |
| P2 | LGE-PUB-pubdev | pub.dev: remove `publish_to: none` (B5), add LICENSE, automated publishing; owner: pub.dev account and package setup | queued |
| P2 | LGE-PUB-opam | opam: `prepare-opam` opens a PR to `ocaml/opam-repository`, never a direct upload; owner: GitHub fork credential, maintainer review | queued |
| P2 | LGE-PUB-source | C and C++: GitHub Release source tarball, SHA256SUMS, signature, attestation; C++ waits for PR #19928 (B8); owner: signing method (O4) | queued |
| P2 | LGE-PUB-<registry> | Publish jobs and gate, one registry per PR; first release per lane needs the owner's go-ahead | queued; first release per lane needs the owner go-ahead |
| P1 | LGE-PARSER-DEPTH | Repo-wide security: `rust/parser` `grammar_parser.rs` `parse()` ignored the sticky `depth_exceeded` flag when another PEG alternative succeeded, and `GrammarParser::new` had unbounded depth; depth exhaustion now fails the whole parse (structural flag); dependents checked (ruby, python, js-iir, warrant-grammar, c/csharp/java/perl/haskell/fsharp parsers) | local, ready (branch claude/parser-depth-exceeded-fail-closed); security-reviewed |
| P1 | LGE-VOS-DEPTH | Outside LGE scope (DoS): `vos-manifest` `document.rs` and `contract.rs` call `GrammarParser::new` without `with_max_depth` on possibly untrusted text (stack overflow); add depth caps (owner of vos-manifest) | queued |
| P2 | LGE-RB-HARDEN | Ruby `logic_gates` hardening found by the lane review: dispatch-free validation (exact-class container copies, no caller `equal?`/`==`), fixed-text errors, caps, frozen constants, LG02 section 9 rows, standardrb in BUILD | local, ready (branch claude/ruby-logic-gates-harden); security-reviewed (5 rounds) |
| P2 | LGE-RB-SEQ | Ruby sequential and combinational blocks raised `TypeError` from their own `validate_bit`; unify onto `InvalidBitError`, drop the dead `Array.new` statement | local, ready (branch claude/ruby-logic-gates-bounds) (with LGE-DOS-ruby); security-reviewed |
| P3 | LGE-LUA-FP-BUILD | Lua `fp_arithmetic` BUILD installs no dependencies although its rockspec depends on logic-gates (pre-existing build completeness) | queued |
| P3 | LGE-GO-SIZEERR | Go package panics with plain `fmt.Sprintf` strings for every shape, size and cap violation (`combinational.go`, `sequential.go`, `gates.go` ANDn/ORn/XORn caps): introduce one typed `*SizeError` plus `IsSizeError`, keeping `*InvalidBitError` for non-bits. Cross-lane: Java/Kotlin `InvalidShapeException`, C#/F# `ArgumentException`; LG02 should name a shape-error identity per lane | queued |
| P2 | LGE-HS-CABAL-PROJECT | Shared `code/packages/haskell` `cabal.project` fails to resolve (a `containers` conflict, pre-existing); workers use scratch projects. Check whether CI builds per package, then fix or document; GHC 9.4.8 (CI) is untested for `-Werror`, watch the first CI run | queued |
| P1 | LGE-LG02-ERRATA | LG02 section 7 binding identities and new 7.1 shape identities rewritten for all 18 lanes from the merged packages (ruby `InvalidBitError`, kotlin family, haskell `LogicGateError`, lua/perl `INVALID_BIT`, elixir rows, section 9/10/11 fixes). The runner matches exact class, so this must land before any LGE-L-<lane>; re-check 7.1 after the hardening branches merge | local, ready (branch claude/lg02-binding-errata); stacked on #19945, push after it merges |
| P2 | LGE-CPP-WIN | Windows C++: `run.ps1` (MSVC) has never been run; the Windows lane has no macro-hygiene, negative-compile or noexcept tests; watch the first Windows CI run of the hardening branch | queued |
| P2 | LGE-PUB-KOTLIN | Kotlin publishing: `kotlin/logic-gates` has runtime dependencies (kotlin-stdlib 2.1.20 and annotations 13.0 via the `kotlin("jvm")` plugin), so the Maven Central POM must declare them | queued |
| P1 | LGE-PUB-OWNER | VER31a decision to surface: crates.io, Hex, LuaRocks, CPAN and pub.dev uploads stay owner-manual by default (their publish tools rebuild or evaluate code while holding the credential); also the npm `file:` / scope checks in LGE-PUB-NPM | queued, needs owner decision; decided in [VER31a](../VER31a-registry-publishing.md) sections 4.2 and 4.3 (owner-manual cells, owner-held tokens); the per-registry environment and name rows are LGE-OWNER-NAMES and LGE-OWNER-ACCOUNTS |
| P3 | PT-001 | Pointer only: per-lane PBT, fuzz, L0, mutation, benchmark and coverage-gate rows live in [polyglot-testing-tooling](polyglot-testing-tooling.md); separate branch, own PR (owner request 2026-11-24) | local, ready (branch claude/pt-backlog-per-lane) (docs only) |

## Status 2026-11-24 (LGE-003)

Merged PRs listed here were checked with `gh pr view`. Only #19945 (LG02
drift guard, LGE-S1b) and #19949 (OCaml, LGE-R-ocaml) are open.

- **Threat lists:** all 18 lane weakness threat lists are written and folded
  into one branch (`claude/lge-s3b-all`, LGE-S3b with the LGE-S3-FIX batch).
  Every row is open and no item is `live`: VER15a section 5.2 forbids live
  evidence before LGE-W1 and LGE-S3v. Each list passed an independent
  adversarial review with fixes.
- **Hardening done locally (security-reviewed, not pushed):** java, kotlin,
  csharp, fsharp, typescript, python, go, ruby, cpp, swift, elixir, lua, perl,
  haskell, dart. Rust hardening is in progress and fixes the v1 report's
  CWE-674 over-exclusion (the merged report excludes recursion although
  `mux_n_inner` recurses, and its memory-class reasons say "reachability"
  while the closure is the package alone). OCaml hardening is blocked on
  #19949 (same package; CI archive-mirror fix folded into it).
- **CI hold policy:** CI is overwhelmed, so new PRs are held. Finished work
  sits in a ready-to-push queue (branch, review state, notes), pushed as CI
  capacity returns. `claude/lge-s3b-all` supersedes the earlier S3 fix batch
  and every per-lane threat-list branch; the csharp, fsharp and swift
  hardening branches supersede their nary/caps predecessors.
- **Cross-cutting findings:**
  1. Dynamic languages ran **caller code during validation** (overloads,
     getters, metamethods, `Inspect`/`toString`/`__repr__`, tied variables,
     `equal?`/`==` receivers); fixed by snapshotting with bound or raw
     primitives first (python, ruby, lua, perl, elixir, dart, typescript).
  2. **Double-read TOCTOU** in N-ary gates and mux (validate, then re-read
     the caller container): java, kotlin, csharp, dart, python.
  3. **Uncapped sizes** (decoders, widths, N-ary inputs, shift registers)
     allocate or loop without bound in most lanes; caps of 16 select bits,
     65536 inputs and widths, 64 counter bits.
  4. **Error-identity drift in LG02**: section 7 named identities the merged
     packages no longer use, and shape errors have no cross-lane identity
     (LGE-LG02-ERRATA, LGE-GO-SIZEERR).
  5. **v1 Rust report over-exclusion** (above).
  6. **Spec additions** in the fold: C++ memory extension
     (`cpp_address_surface_allowlist`, VER15a 5.5), fail-closed
     `no_dynamic_eval_reachable` subset with negative-control corpus, and the
     shared-object subset for CWE-1321.
  7. Publishing blockers: Go (`replace`/`v0.0.0`, see LGE-PUB-GO), Kotlin
     runtime dependencies (LGE-PUB-KOTLIN), registry uploads that are
     owner-manual by default (LGE-PUB-OWNER), npm `file:` dependencies.
- **Merged since the previous status (LGE-002):** #19926 C, #19928 C++,
  #19936 and #19957 Lua, #19937 Java, #19938 Kotlin, #19939 C#, #19941 F#,
  #19942 Haskell, #19944 Perl error class, #19946 and #19933 Python,
  #19948 Haskell dependents, #19950 Ruby, #19952 Elixir, #19953 Dart,
  #19954 VER15a registry v2, #19955 Swift, #19958 TypeScript bounds,
  #19943 the previous status.

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
| 2026-11-24 | LGE-003 | status re-mapped to merged/local/blocked truth; new rows; see "Status 2026-11-24 (LGE-003)" |
| 2026-11-24 | LGE-S4 | VER08c written (spec only). Findings: `ca-assurance-report` consumes no analyzer output today (it reads the effect certificate and runs its own scan), so LGE-W1 must define the consumer; the C grammar is an integer-core subset and the legacy C parser deletes `#` lines; csharp (13 vs 12), ruby (3.4 vs 3.3) and perl (5.26) have grammar version gaps |
