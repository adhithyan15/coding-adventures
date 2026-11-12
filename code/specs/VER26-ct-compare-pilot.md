# VER26 — The `ct-compare` verification pilot (Rust)

**Status:** Pilot in progress. VER09, VER11 and VER10's first bit-vector slice,
the taxonomy extension and quirks log are on main. TF01 is tracked in #18807.
The Rust CT01 L0 adapter implements step 4; it is hand-written until Warrant's
backend replaces it. Each step lands as its own PR, spec first, on its own
branch. [VER-BACKLOG.md](VER-BACKLOG.md) tracks dependencies and merge evidence.

**Layer:** Repository-wide verification program.

**Builds on:** VER00 (the verified shipping program), CT01 (constant-time
comparison and selection), TF00 (portable test framework), D21 (capability
cage).

---

## 1. Why a pilot

VER00 describes a long ladder of evidence:

- unit tests, property tests and fuzzing;
- proofs about bounds, constant time and capabilities;
- an assurance report that a release gate reads.

Every tool on that ladder is to be built in-house, in every language. Written
down, the design looks coherent. Built, it will not be: the design has rough
edges that only appear once code meets it.

This pilot finds those edges early and cheaply. It takes **one small package
in one language** all the way up the ladder, building each tool only as far as
that package needs. Every surprise is written down as it is found (§6). The
tools are generalised to the rest of the repository only after the pilot
shows they work.

## 2. Scope

**The package.** The pilot uses `code/packages/rust/ct-compare`, the Rust lane
of CT01.

- **It is tiny.** About 370 lines of source.
- **It has no dependencies and no capabilities.**
- **It is relied on.** It sits under `hmac`, `shamir`, the Vault crates and
  several X.509/OCSP checks.
- **Its main promise needs proof, not tests.** In CT01's terms, that promise is
  that, for valid equal-length inputs, control flow and memory indexing do not
  depend on the compared contents or the selection bit. No test can establish
  that. CT01 makes no wall-clock, cache or speculation promise, and does not
  promise that every compiler preserves the source's shape, and neither does
  this pilot. Its functional
  promise, `eq(a, b) == (a == b)`, is easy to state, so the pilot spends its
  effort on the tools rather than on the specification.

**The language.** The pilot targets Rust alone.

- Rust is the reference lane for every tool in VER00.
- The verification toolchain is written in Rust as well.
- The Rust `ct-compare` is the most complete port. It has a volatile optimiser
  barrier and tests that inspect the optimised LLVM IR.

**Out of scope for the pilot:**

- the other 17 languages' `ct-compare` lanes (§8.3);
- migrating any package's existing tests to the new infrastructure. That
  happens only once the infrastructure is proven, and it is a separate
  programme;
- the publishing and approval flow (VER00 §9.2.1, §9.4, §9.5), which VER00
  §12.1 blocks until its open findings are resolved;
- the languages deferred in §8.

## 3. Success criteria: the evidence the pilot must produce

The pilot succeeds when `rust/ct-compare` has the evidence below. Each item
must carry an honest label (VER00 §7.6.5); no label may be rounded up.

| Rung | Evidence | Expected label for `ct-compare` |
|---|---|---|
| E1 unit | Every CT01 corpus case (`code/specs/fixtures/ct-compare-v1/`) runs through the bootstrap harness (§4, L0), driven by tests generated from a Warrant file | pass |
| E3 property | `eq` ⇔ `==`; `eq_fixed` agrees with `eq`; `select_bytes` picks correctly and returns a fresh copy; `eq_u64` single-bit sweeps; `cover` targets met; deliberately broken implementations caught and shrunk to a minimal case | pass |
| E4 fuzz | No panics; agreement with `==`; length-mismatch errors as CT01 specifies | pass, with the coverage label the Rust fuzz lane achieves (coverage-guided, or "black-box, weaker") |
| E5 proof | `eq_u64` and small `eq_fixed` widths proved equal to their specification; step bounds `steps ≤ c·n + d` for all lengths; a **WIR-level constant-time proof over the observable trace** for all lengths; `eq` and `select_bytes` functional correctness `bounded(k)` until loop invariants are supported | `proved` / `bounded(k)` as stated, each with its assumption list (below) |
| E0 capability | `effects(ct-compare) = ∅`, with an effect certificate checked by the small checker; the deny-by-default name allowlist passes | `proved` |
| Weakness rows | CWE-208 (timing) is a computed exemption. Its floor needs a proof down to the gate-level CPU, which this pilot does not reach (§8.1). The exemption also stays **unapproved**, because approval is part of the publishing flow that VER00 §12.1 blocks, so this report feeds no release. | exemption (unapproved), backed by the WIR-level constant-time proof and equal instruction counts |
| Microbenchmark | Equal retired-instruction counts within each input class, at n = 16, 256 and 4096: `eq` all-equal vs differ-at-first-byte vs differ-at-last-byte; `select_bytes` with `choice` true vs false; `eq_u64` equal vs unequal. Where hardware counters are unavailable, the row is reported as `not-live`, never `measured` or `pass`. | measured, or `not-live` |
| Report | A report-only assurance report for `ct-compare`, fail-closed, with rungs that are not yet live reported as `not-live` | report only |

**What the constant-time proof does and does not cover.** CT01 promises more
than equal step counts: control flow and memory indexing must not depend on
secrets. A branch on `choice` whose two arms take the same number of steps
would pass a proof about step counts alone, and would pass the
equal-instruction-count benchmark too. Equal counts cannot detect balanced
branches. So the proof is **self-composition over the whole observable trace**:
for two runs whose inputs differ only in secret values, it must show that the
following are identical:
- every branch decision;
- every memory index and address;
- the step count.

Only a proof over that trace may be labelled `proved` or back the CWE-208
exemption. It is a proof about WIR lowered from the Rust source, and its
assumption list must name, at least:

- **compiler preservation**: rustc and LLVM keep the source's secret-independent
  control flow and indexing. CT01 explicitly disclaims this. Here it is backed
  only by `tests/optimized_ir.rs` and the measured instruction counts, so it
  is evidence, not proof;
- **machine timing**: the latency of each instruction on the target does not
  depend on its data.

**The secret labels are mandatory.** `ct-compare.warrant` must mark the compared
contents and `choice` as `secret`, and lengths and widths as public, exactly as
CT01 assigns them. Byte parameters are not secret under VER00's default rules,
so without these labels the proof would have no secret inputs and would pass
vacuously. The vacuity check of VER00 §7.6.3 therefore **fails the pilot** if
the constant-time claim has no secret input.

## 4. How the pieces are layered

The pilot builds its tools in layers. Each layer is tested by the one below it,
and the bottom layer is small enough to verify by reading. This is the same way
the repository bootstraps its compilers.

```
L0  bootstrap harness (Rust, about 60 lines, no dependencies)
      named checks → TAP v14 → exit code
      verified by reading it, plus a data-only self-corpus with mandatory
      negative cases and a sabotage run that must fail
L1  Warrant v0 (Rust)
      warrant.tokens + warrant.grammar → generated lexer and parser
      (grammar-tools, lexer and parser crates) → hand-written type checker and
      bindings → a Rust backend that emits example, claim, input and bound
      blocks as plain tests over L0
L2  runtime support
      random generator, tape, generators and shrinking, fuzz mutator,
      assertion and deep-equality semantics, each pinned by a corpus.
      Hand-written in Rust for this pilot; generated per language later, so
      a seed replays identically in every language by construction
L3  evidence tools
      property engine, fuzz engine, microbenchmark harness, capability
      analyzer, SAT solver with DRAT certificates, Warrant IR with a Rust
      front end, proofs, assurance report
L4  self-hosting (after the pilot)
      Warrant specifies grammar-tools, the lexer, the parser and its own type
      checker; stage 2 must equal stage 3; diverse double-compiling against
      the Rust bootstrap guards against a self-perpetuating bug
```

Two transport rules hold at every layer:

- **Corpora are data, not code, and they include cases that must fail.** A
  checker that never reports failure passes any corpus that only expects
  passes, so cases expected to fail are mandatory.
- **Harnesses read TFV, never JSON.** `cases.json` stays the source of truth. A
  generator emits **TFV**: one case per line, using prefix tokens with arity
  counts, hex-encoded strings, `f64` as 16 hex bits so `-0` and NaN payloads are
  exact, and arbitrary-precision decimal integers. CI checks that the TFV is up
  to date. JSON parsers are third-party or missing in many languages' standard
  libraries.

## 5. Steps

Each step is one PR. Child specs are written first.

| # | Step | Child spec |
|---|---|---|
| 1 | This spec | VER26 |
| 2 | The quirks log, `quirks.d/` (§6) | VER26 |
| 3 | TF00 revision: bootstrap layer, TFV, run protocol (random generator, shuffle, TAP profile) | TF00, TF01, TF02 |
| 3b | `code/packages/rust/test-bootstrap`: the L0 harness, its self-corpus `code/specs/fixtures/test-bootstrap-v1/`, a stdlib-only validator, the sabotage run, and registration of the fixture as a build-tool repository input | TF01, TF02 |
| 4 | `ct-compare` runs every CT01 corpus case through L0; settle the length-mismatch error; add the fresh-copy and `expected_work` checks | CT01 |
| 5–8 | Warrant v0: spec, grammar and generated lexer/parser, type checker and bindings, Rust backend, `ct-compare.warrant` | VER04 |
| 9–12 | Property engine: spec, `pbt-v1` corpus, Rust engine, `claim` blocks compiled to properties | VER05 |
| 13–15 | Fuzz engine: spec, `fuzz-v1` corpus, Rust engine and `ct-compare` targets | VER06 |
| 16–17 | Microbenchmark harness and the equal-instruction-count check | VER07 |
| 18–20 | SAT: CDCL core, DRAT checker, Tseitin/AIG/bit-blaster. An independent track | VER09–VER11 |
| 21–23 | Warrant IR core, a Rust-subset grammar, and a Rust → WIR front end | VER25 |
| 24–25 | Symbolic execution; bounds and constant-time proofs | VER19, VER20 |
| 26 | Capability taxonomy extension (`rand`, `thread`, `ui`). An independent track | D21 |
| 27 | Rust capability analyzer, generated capability module, name allowlist, effect certificate | VER08, VER23, VER24 |
| 28 | Weakness registry v1 and a report-only assurance report | VER15, VER01 |

The critical path is 1 → 3 → 5–8 → 9–12 → 21–25. The SAT track (18–20) and the
taxonomy extension (26) can run in parallel from the start. Microbenchmarks
(16–17) follow step 3, and fuzzing (13–15) follows step 11.

## 6. The quirks log

Every rough edge the pilot meets is recorded the moment it is found, one file
per quirk, in `quirks.d/` at the repository root. It works like `lessons.d/`,
for the same reason: a shared file would make concurrent PRs conflict. A
lesson records a mistake *we* made. A quirk records a property of *the world*
that the tools must cope with: a language, a toolchain, an existing package or
a CI runner.

Each quirk file has front matter:

| Field | Meaning |
|---|---|
| `package` | The package it was found in, or `*` |
| `lane` | The language or toolchain, or `*` |
| `rung` | The rung of the ladder it affects (`E0`–`E6`, `bench`, `report`, `infra`) |
| `status` | `open`, `mitigated` (a workaround exists), or `accepted` (a documented limit) |

**A quirk must never contain an untriaged counterexample, failing seed or
crashing input** (VER00 §7.3). It may name the class of failure and point to
the private triage record.

## 7. Divergences already found across the existing ports

Comparing the 12 existing `ct-compare` lanes against CT01 found divergences,
which fall into two kinds.

**Security-relevant findings are fixed as soon as they are found.** No
`ct-compare` lane has been published to a package registry or is used in
production. VER00 §7.3 lets a never-shipped package take a fix and its
regression tests in the same change, so each such finding is fixed straight
away, outside this pilot, in its own pull request, with regression tests. A
concrete input that defeats a comparison is an exploit, so until that fix
lands it is not written in any public spec, quirk or issue. Once the package
has shipped, the private-advisory path of VER00 §7.3 applies instead.

**Conformance gaps that carry no exploit** are recorded here and as quirks, for
whoever owns those lanes (§8.3). The pilot touches only the Rust lane, so it
does not fix them.

- **Originally no lane ran the CT01 corpus.** Step 4 adds the Rust L0 adapter
  with checked TFV generation; the other lanes remain with package-parity.
- **`eq_fixed` is not one operation across lanes.** Some lanes enforce the
  width in the type system, one takes an explicit length with a caller
  precondition, and the rest delegate to `eq`. CT01 needs to say which of
  these are acceptable.
- **Errors on a select length mismatch differ in every lane**, and none maps to
  CT01's semantic error `CT_SELECT_LENGTH_MISMATCH`.
- **The `eq_u64` reduction technique differs between lanes.** Each lane's form
  must be reviewed against CT01's "fixed operation sequence" rule.
- **Only Rust checks its generated code for an optimiser-introduced early
  exit** (`tests/optimized_ir.rs`).
- **C, C++, Java and Kotlin have no `required_capabilities.json`.**
- **OCaml has no `ct-compare` lane, and CT01's list of missing ports omits
  it.**

## 8. Deferred languages, and why

### 8.1 Nib, Oct and Twig: deferred, with a better pilot planned

Nib, Oct and Twig are languages invented in this repository. They are the most
interesting targets of all, because the whole stack beneath them is ours:
compiler, VM and, for Nib and Oct, the gate-level CPU. That is where VER00
§7.5.6's owned-layer proofs, in which a platform assumption becomes a proved
claim of the layer below, can be exercised for real. `ct-compare` is the
wrong package to try it with, because each language lacks something the
package needs:

| Language | Gap for `ct-compare` | Source |
|---|---|---|
| **Nib** (Intel 4004) | No arrays in v1, so byte strings cannot be passed or iterated | NIB00, "Arrays — reserved for v2" |
| **Oct** (Intel 8008) | No arrays in the language specification; it works on `u8` values and static variables | OCT00 |
| **Twig** (Lisp on the LANG VM) | No bitwise operators, so the XOR accumulator and the select mask cannot be written without arithmetic substitutes that would themselves branch | TW00, TW03 |

Porting `ct-compare` to them would mean either a reduced, scalar-only API, or
extending the languages first. Neither tests the verification tools any
better than the Rust pilot does.

**The next pilot after this one** proves a Nib program all the way down to
gates: the **Busicom 141-PF calculator** written in Nib, compiled by our Nib
compiler to the 4004, and checked against the repository's gate-level 4004.
It exercises the whole owned-layer chain: Nib source, our compiler, our 4004
netlist, gates. The first route to meeting the CWE-208 floor for constant time, which needs a
proof down to the gate-level CPU, is RV32I on `riscv-gatelevel` (owner,
2026-11-12; [VER13](VER13-assumption-catalogue.md) §4.4). The Busicom/4004
track is a second, independent stack beside it
([VER27](VER27-layered-validation-roadmap.md) §4.15).

### 8.2 Starlark and Mosaic: excluded

These are a build-rule language and a UI component language, not
general-purpose programming languages. They are out of scope for the
verification program's language lanes.

### 8.3 The other 17 languages: later, and owned elsewhere

The other `ct-compare` lanes are C, C++, C#, Dart, Elixir, F#, Go, Haskell,
Java, Kotlin, Lua, OCaml, Perl, Python, Ruby, Swift and TypeScript. They join
once the tools have been proven on Rust. Each will get an L0 harness, a Warrant
backend and a generated copy of the L2 runtime support.

The repository's automated **package-parity loop** already owns the work of
porting `ct-compare` to Dart, Haskell, Lua, Perl and Swift, and of making the
existing lanes adopt CT01. Its items are in
`.claude/package-parity-loop-state.json`, for example
`ct-compare-existing-lane-conformance-adoption` and
`ct-compare-<lang>-lane-parity`. The pilot must not duplicate that work. The
conformance gaps in §7 are handed to it. The security-relevant findings are
not; they go through private advisories.

## 9. What the pilot cannot exercise

`ct-compare` is a pure function, so several parts of VER00 never come into
play. A second pilot, with a package that declares capabilities such as a
small filesystem package, is needed before they can be considered proven:

- the simulated host, fault plans and chaos injection (VER03, VER17);
- information-flow claims (VER24), since `ct-compare` has no sources or sinks;
- the locked box (VER22), beyond recording a runtime baseline;
- capability diffs and generated modules with real functions in them (VER23).

The pilot also does not reach the CWE-208 floor. That needs a proof down to
gates: RV32I on `riscv-gatelevel` first, with the Busicom pilot a second,
independent stack (§8.1).

## 10. Decisions the owner still has to make

None of these blocks the start of the pilot.

- **A private repository for long fuzz campaigns.** A failure in public CI is
  a disclosure if anyone can reproduce it. A seed derived from public data,
  such as the property id and engine version, can be recomputed by anyone, so
  even a red check that prints only an opaque id reproduces the failure. The
  rules are therefore:
  - public runs print only an opaque failure id, and the reproducer goes to
    the private store (VER00 §7.3);
  - **anything new or changed runs privately first** against every
    already-shipped release, before public CI enables it. That covers a new
    claim, property, fuzz target, generator or engine version. A failure on
    unchanged shipped code goes to the private store and the advisory path,
    never to a public check;
  - alternatively, public seeds are derived with a private key, so they
    cannot be recomputed. An opaque id alone is not sufficient;
  - randomised exploration runs only in the private campaigns.
- **How the Rust name allowlist handles macro expansion.** Checking after
  expansion needs the compiler's HIR, which is available only on nightly
  Rust. The options are to restrict macros in package crates, or to run the
  check in a separate nightly job.
- **Instruction counters on CI runners.** The equal-instruction-count check
  uses `perf_event_open`. If the runners forbid it, the row is reported as
  `not-live` there, never as passing. It blocks only where counters are
  available.
