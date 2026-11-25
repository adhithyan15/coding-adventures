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
| 1 | LGE-CPP1 | New `cpp/logic-gates` (`constexpr`, `std::invalid_argument`, deleted non-int overloads), 96,454 checks | PR #19928 (waiting on macOS runners) |
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
| 3 | LGE-R-<lane> | Lane-local exhaustive suites for ruby, swift, elixir, dart, ocaml (OCaml: literal expected values, not `land`/`lor`) | queued |
| 2 | LGE-HS-DEPS | Haskell dependents (security, pre-existing): `alu [] []` returns Right with bottom flags; `rippleCarryAdder [] [] 5` returns an unvalidated carry; `isNegative` checks only the head; `newBRAM` width 0 throws DivideByZero out of the Either; RAM ticks never validate data/clock/write-enable (stores 5); `configureLUT` does not validate its truth table; RAM sizes uncapped | queued, after #19942 |
| 2 | LGE-DOS-ALL | Cap decoder/demux/mux select bits at 16 and sequential widths in every lane with these blocks (Rust SEC-008, Go and Haskell done in-PR; Python confirmed unbounded) | queued |
| 3 | LGE-PY-SEQ-COMB | Python: counter returns unvalidated state bits; encoders accept any sized container | queued |
| 3 | LGE-LUA-SEQ | Lua `sequential.lua` still has free-text validation accepting `1.0` | queued |
| 3 | LGE-ID-perl-transistors | Perl transistors needs a typed error identity too | queued |
| 4 | LGE-LUA-STALE | Lua BUILD install guards skip reinstall when an old rock exists, so a cached tree could test dependents against the old validator | queued |
| 4 | LGE-HS-COUNTER | Haskell `counter 1 0 [0,1]` returns `[0,1]`, not `[1,0]` (two-phase flip-flop emulation); investigate | queued |
| 4 | LGE-S1b | `logic-gates-basic.warrant` and `code/scripts/logic_gates_basic_drift.py` with its tests and build-input wiring | in review (drift-guard fail-closed fixes) |
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
| 8 | LGE-S5 | [VER31a](../VER31a-registry-publishing.md): per-registry publish jobs behind the VER31 gate, tag grammar, owner actions, findings B1-B9 | spec in review |
| 8 | LGE-OWNER-ACCOUNTS | Owner creates accounts, namespaces and trusted publishers (never the coordinator): crates.io account and name claim; npm `coding-adventures` org; one GitHub environment per registry (`release-crates`, `release-npm`, `release-pypi`, `release-rubygems`, `release-maven`, `release-nuget`, `release-hex`, `release-luarocks`, `release-cpan`, `release-hackage`, `release-pubdev`, `release-opam`, `release-source`); PyPI trusted publishers pinned to `release-pypi`; RubyGems and NuGet trusted publishing (required); owner-manual uploads for crates.io, Hex, LuaRocks, CPAN, pub.dev; dedicated machine GitHub account and opam fork; Go signed-tag ruleset; Maven Central portal account, namespace verification and GPG key; NuGet account and `CodingAdventures.` prefix; hex.pm account and API key; luarocks.org account and API key; PAUSE (CPAN) account; Hackage account and token; pub.dev account and automated-publishing setup; GitHub account able to PR `ocaml/opam-repository`; Swift mirror repo if chosen; C/C++ signing; GitHub `release` environment, tag-protection ruleset, machine account, `.github/release-signers` and CODEOWNERS (VER31a section 1.2). Names are the separate row LGE-OWNER-NAMES, ordered first. Also decisions O1-O6 (VER31a section 6) | owner action |
| 8 | LGE-OWNER-NAMES | **Owner action, before every LGE-PUB row; agents never create accounts, tokens or reservations.** Claim or reserve the whole dependency closure's names (logic-gates and transistors, B1) on every registry (VER31a section 1.3): the npm `@coding-adventures` org/scope; the NuGet `CodingAdventures.` prefix; Maven namespace verification before any build; a pub.dev verified publisher; the PAUSE `CodingAdventures::` namespace; the Hex, LuaRocks, Hackage and opam names; typosquat variants (underscore, hyphen, prefix); record each in `code/assumptions/first-party-names.json` with its expected owner | owner action |
| 8 | LGE-PUB-validator | `code/scripts/tests/` validator over `publish.yml`: triggers are exactly `push: tags` (allow-list) plus `workflow_dispatch`; every publish and `attest` job `if:` has `event_name == 'push'` and `needs.gate.result == 'success'` (plus verify-tag, attest); no `always()`, `!cancelled()` or `failure()`; every publish job needs `gate`; no publish verb in `gate` (VER31a 4.1 rules 0 and 2) | queued, with LGE-PUB-gate |
| 8 | LGE-PUB-go-helper | `code/scripts/go_release_tag.sh` with tests: refuses unless the gate run `headSha` equals the commit and succeeded, the SHA is on `origin/main`, `go.mod` has no `replace` or `v0.0.0` sibling, the tree is clean; then `git tag -s`. Plus the post-tag observer that fails loudly on a Go tag with no matching green gate run (VER31a 3.4) | queued |
| 8 | LGE-PUB-names-map | `code/assumptions/first-party-names.json` (lane, registry, name, expected owner) under CODEOWNERS, and the gate's owner and digest check for first-party dependencies (VER31a 2.3 item 2) | queued, with LGE-PUB-gate |
| 8 | LGE-OWNER-INTERIM | Owner actions now, ahead of the gate: scope and rotate `RUBYGEMS_API_KEY`, move it into the `rubygems` environment, create the tag ruleset on `refs/tags/python/**` and `refs/tags/ruby/**`, enable hardware-key 2FA on the owner account (VER31a 5.5) | owner action |
| 8 | LGE-PUB-gate | Wire the VER31 gate (LV-007a) and the multi-lane `parse-tag` allow-list, Go tag shape and `registry` output (VER31a sections 2-3) before any registry row below | blocked on LV-007a |
| 8 | LGE-PUB-crates | crates.io: `rust/logic-gates`; needs a version on the `transistors` path dep (B1), a `license` field, name decision O6; owner claims the name by a hand-published first version | queued; owner: crates.io account |
| 8 | LGE-PUB-npm | npm: `@coding-adventures/logic-gates` with provenance; owner: `coding-adventures` org, first version by hand, trusted publisher; `file:../transistors` dep (B1) | queued |
| 8 | LGE-PUB-pypi | PyPI: `coding-adventures-logic-gates`, existing job moved under the gate and the `release` environment; owner: trusted publisher pinning `release` | queued |
| 8 | LGE-PUB-rubygems | RubyGems: `coding_adventures_logic_gates`; trusted publishing in place of `RUBYGEMS_API_KEY`; transistors gem dep (B1); owner: trusted publisher or key moved into `release` | queued |
| 8 | LGE-PUB-go | Go proxy: tag `code/packages/go/logic-gates/vX.Y.Z`, remove `replace` and `v0.0.0` on transistors (B1, B7); owner signs the tag; no account | queued |
| 8 | LGE-PUB-maven | Maven Central for java and kotlin: distinct artifact ids (B2), namespace decision (B3, O2), signed bundle to the Central portal; owner: portal account, namespace verification, GPG key and token secrets | queued |
| 8 | LGE-PUB-nuget | NuGet for csharp and fsharp: `.nupkg` push, trusted publishing or key; owner: nuget.org account, ID prefix reservation | queued |
| 8 | LGE-PUB-hex | Hex: `coding_adventures_logic_gates`; add package metadata to `mix.exs`; path dep on transistors (B1); owner: hex.pm account and API key | queued |
| 8 | LGE-PUB-luarocks | LuaRocks: `coding-adventures-logic-gates`; transistors rock dep (B1); owner: luarocks.org account and API key | queued |
| 8 | LGE-PUB-cpan | CPAN: `CodingAdventures-LogicGates` via PAUSE; transistors dist dep (B1); owner: PAUSE account | queued |
| 8 | LGE-PUB-hackage | Hackage: rename `logic-gates` (B4, O6), `cabal sdist` and upload; owner: Hackage account and token | queued |
| 8 | LGE-PUB-pubdev | pub.dev: remove `publish_to: none` (B5), add LICENSE, automated publishing; owner: pub.dev account and package setup | queued |
| 8 | LGE-PUB-opam | opam: `prepare-opam` opens a PR to `ocaml/opam-repository`, never a direct upload; owner: GitHub fork credential, maintainer review | queued |
| 8 | LGE-PUB-swift | Swift: blocked by the root `Package.swift` requirement (B6); owner decides mirror repo, shim or defer (O3) | blocked on owner decision |
| 8 | LGE-PUB-source | C and C++: GitHub Release source tarball, SHA256SUMS, signature, attestation; C++ waits for PR #19928 (B8); owner: signing method (O4) | queued |

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
| 2026-11-24 | LGE-S5 | VER31a drafted; nine findings (B1-B9): unpublished transistors dependency in 8 lanes, Java/Kotlin Maven coordinate clash, Go subdirectory tag, SwiftPM root manifest, generic crate/Hackage names |
| 2026-11-24 | LG02 errata | drift guard: checks 10-12 gated until LGE-S1c; three BUILD variants parsed; recorded in LG02 by the LGE-S1b PR |
