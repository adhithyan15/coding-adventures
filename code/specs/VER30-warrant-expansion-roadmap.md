# VER30 — Warrant Expansion Roadmap: from six adopters to the whole repository

**Status:** Roadmap, committed as a specification by owner decision (section 8,
decision 3). Nothing here is implemented by this document and no code changes
with it. It orders existing and new work; each item keeps its own backlog row,
its own spec where it needs one, and its own PR.

**Parent:** [VER27](VER27-layered-validation-roadmap.md) (the vision and the
layer ladder), [VER29](VER29-layer-contract-linking.md) (how one layer assumes
the one below).

**Builds on:** [VER04j](VER04j-warrant-v1-extensions.md) (the v1 features that
exist today), [VER04k](VER04k-warrant-v1-wave4.md) (WX-D, WX-F, WX-G, still
mostly unimplemented), [VER04i](VER04i-polyglot-warrant-evidence.md) (lanes),
[VER13](VER13-assumption-catalogue.md), [LG01](LG01-logic-gates-contract.md),
[AR01](AR01-arithmetic-contract.md), [LZ01](LZ01-lzw-contract.md),
[LEB01](LEB01-wasm-leb128.md), [CT01](CT01-constant-time-compare.md).

**Backlog:** rows LP-*, WX-*, WD-004 / WD-1xx, LV-*, CAP-RT-*, LP-010b, LP-012,
AR-001 and ARITH-001 of [VER-BACKLOG](VER-BACKLOG.md), which now point here for
their order, and the new rows WX-T0/T1, WX-S0/S1, WX-B2, WX-I128, VER30-A1 and
VER30-INV that section 4 introduces.

**Data:** [`data/warrant-inventory/summary-2026-11-14.md`](data/warrant-inventory/summary-2026-11-14.md)
and [`inventory-2026-11-14.csv`](data/warrant-inventory/inventory-2026-11-14.csv)
(1,459 rows, one per Rust package). Every number in this document is taken
from that summary. When this text and the summary disagree, the summary wins and
this text has a bug.

**Direction (owner, 2026-11-14):** three decisions, recorded in section 8. In
short: the feature freeze is lifted for text and stateful objects (spec both
now); the polyglot track starts now, in parallel with Rust adoption; and this
roadmap is a committed spec.

---

## 1. Goal

One `.warrant` file per package concept is the single source of truth. It is
compiled into evidence for every language port of that concept, with equal
rigour: the Python, TypeScript, Go (and so on) port of `hkdf` is held to the
same cases, the same bounds and the same sabotage checks as the Rust one,
because all of them are generated from the same document.

Verification is layered (VER27 P9, VER29). Each layer is verified alone, and
the layer above takes that result as an assumption instead of re-proving it.
Gates, then adders, then an ALU, then a CPU: the CPU's evidence cites the
published claims of the ALU and does not re-derive Boolean algebra.

Today this holds for six packages. The goal of this roadmap is to make it hold
for every package where it can hold, in an order that buys the most safety per
unit of effort, without turning the shared tooling into a bottleneck.

## 2. Where we are

### 2.1 The six adopters

| Package | Role | Notes |
|---|---|---|
| `ct-compare` | the pilot (VER26, CT01) | first constant-time proof target |
| `gf256` | field arithmetic (LP-001) | all 2^24 triples exhaustive |
| `wasm-leb128` | first v1 adopter (LP-008, LEB01) | canonical encoders, malformed input |
| `logic-gates` | layer 1 of the 4004 chain (LP-009, LG01) | 88 claims; publishes contracts |
| `lzw` | first decompressor (LP-010, LZ01) | output bound, hostile input |
| `arithmetic` | layer 2 of the 4004 chain (LP-011, AR01) | adders and ALU |

### 2.2 The inventory

The inventory is a read-only survey of every package under
`code/packages/rust/` at `origin/main` on 2026-11-14: 1,459 crates and
2,693,888 non-blank, non-comment `src/` lines. It tiers each crate by the
cheapest Warrant feature set that covers its principal API.

| Tier | Meaning | Crates | LOC | Port-sum |
|---|---|---:|---:|---:|
| T0 | adoptable now | 119 | 77,643 | 428 |
| T1 | generator fix | 21 | 13,898 | 97 |
| T2 | needs WX-F / WX-G / WX-D2 | 35 | 9,502 | 80 |
| T3 | needs unspecced feature | 1056 | 2,306,846 | 2873 |
| T4 | not a Warrant fit | 228 | 285,999 | 91 |

"Port-sum" is the number of same-name packages in the other 21 language
directories (3,569 in total; an exact name match after normalising `-`, `_` and
case, so `-native` and renamed ports are undercounted).

**The classification rule** (so it can be argued with). Take the crate's
non-trivial public entries: crate-level `pub fn`, `pub mod` functions and
`impl` methods of public types. Constructors, `fmt`/`clone`-style trait glue,
no-argument `&self` getters and text-rendering conveniences such as `*_hex` are
excluded from the vote. Each entry gets the set of features its signature
needs. The crate's *required set* is the union over its cheapest 60% of
entries, forced to include every entry of a public object type with two or more
`&mut self` methods and of any streaming type. The primary tier is the most
expensive tier in the required set; everything else becomes secondary blockers.
T4 is decided first: bin-only, cdylib/staticlib, FFI export names, re-export
glue, effect-dominated signatures (`Path`, `File`, `Duration`, raw pointers), or
a name pattern (`-cli`, `-repl`, `-daemon`, `-integration`, `-uart` and so on)
combined with a mostly-impure surface.

### 2.3 How far to trust it

The classifier is a regex and brace-matching reader, not rustc. Three sampled
rounds plus a read of the top-25 list put the error at 8.3% on the final
classifier (4 of 48), 10.9% on the previous round, and 5-10% on the top-25
list. Per tier over rounds 1 and 2 (n=103): T0 16 of 20 correct (80%), T1 9 of
9, T2 15 of 16, T3 33 of 36 (92%), T4 20 of 22 (91%). The inventory states the
residual expectation as about 8% overall, with about 20% in T0, concentrated
at the T3/T4 boundary and in T0.

Consequence, binding on Track B: **the T0 list is a list of candidates. Every
T0 crate is confirmed by reading its public signatures before adoption begins**,
and a crate that turns out not to be T0 is reclassified in the next inventory
run, not forced through. Misses found so far were always one tier away, except
`csprng` (T0 to T4: OS entropy) and `dynval-runtime` (T0 to T3: raw-pointer
heap).

## 3. Demand

### 3.1 What each feature unblocks

Cumulative, in the proposed order (cheap generator gaps first, then the
already-specced WX items, then the unspecced ones). A crate counts as unblocked
when its whole required set is covered.

| Step | Feature added | New crates | New ports | Cumulative adoptable crates (incl. T0) | Cumulative ports |
|---:|---|---:|---:|---:|---:|
| 0 | (nothing: T0 today) | 119 | 428 | 119 | 428 |
| 1 | Bytes<N> | 17 | 74 | 136 | 502 |
| 2 | fixed-array | 4 | 23 | 140 | 525 |
| 3 | int128 | 0 | 0 | 140 | 525 |
| 4 | WX-G | 25 | 4 | 165 | 529 |
| 5 | WX-F | 10 | 76 | 175 | 605 |
| 6 | WX-D2 | 0 | 0 | 175 | 605 |
| 7 | text | 133 | 421 | 308 | 1026 |
| 8 | stateful-objects | 268 | 810 | 576 | 1836 |
| 9 | floats | 107 | 427 | 683 | 2263 |
| 10 | collections | 124 | 243 | 807 | 2506 |
| 11 | opaque-types | 108 | 129 | 915 | 2635 |
| 12 | generics | 315 | 843 | 1230 | 3478 |
| 13 | async | 1 | 0 | 1231 | 3478 |

### 3.2 What blocks each feature

"Primary" means the feature is in the crate's required set. Excludes the 228 T4
crates. Rows overlap because a crate can need several features.

| Feature | Tier | Crates needing it (primary) | Port-sum of those | As secondary only | Crates unblocked *alone* | Port-sum alone |
|---|---|---:|---:|---:|---:|---:|
| `Bytes<N>`: `[u8; N]` params/results (generator gap) | T1 | 128 | 249 | 11 | 17 | 74 |
| fixed arrays of non-byte elements (generator gap) | T1 | 116 | 97 | 8 | 0 | 0 |
| `u128`/`i128` (no 128-bit type in v1; WX-B stops at 64) | T1 | 72 | 44 | 6 | 0 | 0 |
| WX-G `&mut [u8]` / `&mut Vec<u8>` out-buffers, in-place | T2 | 144 | 236 | 28 | 19 | 4 |
| WX-F update/finalize and bit-coder streams | T2 | 33 | 162 | 0 | 3 | 2 |
| WX-D2 list folds for the reference model | T2 | 2 | 25 | 0 | 0 | 0 |
| text: `&str`/`String`/`char` in the core API | T3 | 839 | 2161 | 39 | 119 | 358 |
| stateful objects: `&mut self` method protocols | T3 | 535 | 1633 | 27 | 16 | 78 |
| floats (`f32`/`f64`) | T3 | 349 | 932 | 44 | 26 | 208 |
| maps, sets, `BinaryHeap` | T3 | 261 | 558 | 42 | 4 | 9 |
| generics, traits, closures, `dyn` | T3 | 316 | 843 | 46 | 36 | 77 |
| opaque foreign types (cross-crate handles) | T3 | 201 | 340 | 35 | 35 | 0 |
| async | T3 | 1 | 0 | 0 | 0 | 0 |

### 3.3 The plain reading

The work that is already specced, plus the generator gaps (Bytes<N>, fixed
arrays, int128, WX-G, WX-F, WX-D2), reaches only **175 of 1,459 crates** (605
ports) in total, including today's T0. The repository is dominated by
text-processing and stateful-object crates. Text as a core input or output
(119 crates alone) and stateful object protocols (268 more once text is in)
together add about **400 crates** (133 + 268 = 401, and 421 + 810 = 1,231 more
ports). That is why decision 1 of section 8 lifts the freeze for exactly those
two features and for no others.

Floats, collections, opaque types and generics each add 100 to 300 crates, but
mostly simulators, layout engines, compilers and smart-home glue rather than
security code. They stay frozen until the demand is re-assessed (section 6).

Two cautions on the numbers. 985 of the 1,459 crates have no ports at all
(mostly TLS, X.509 and HTTP framing, smart home, board-vm, chief-of-staff);
their value is Rust-only evidence plus the runtime jail, and a security-weighted
order puts WX-G (19 crates alone, among them the `tls13-*`, `rsa-pss-*` and
`http1-*` crates) ahead of where the port-weighted table above puts it. And
WX-D1/D2 are not unlocks: only 2 crates have a signature that demands a fold.
Their value is quality, a differential model for the 48 T0-to-T2 hash, codec,
KDF and checksum crates (395 ports), so they are scheduled as a multiplier in
track C.

## 4. Tracks

Five tracks. A and D unblock the others; B, C and E consume what they produce.
Within a track, items are in order. "Entry" is what must be true before an item
starts; "exit" is what makes it done.

### 4.1 Track A: scale enablers

Everything in this track removes a cost that otherwise grows with every
adoption. They are small relative to the features, and most of them were found
as defects of the first six adoptions.

| Order | Item | Backlog id | Entry | Exit |
|---:|---|---|---|---|
| A1 | Data-driven assurance-report profiles | VER30-A1 (new) | none | a new package adopts a report profile by adding a data file beside its `.warrant`; no edit to `ca-assurance-report` code |
| A2 | Domain-kind counting | LP-012 | none (design exists) | every claim declares `full`, `sampled` or `bounded-set`; the manifest counts per kind; the report calls only `full` exhaustive |
| A3 | CI re-selection | LV-002 | none | a change to an adopter's `.warrant`, a pinned fixture, `warrant-grammar`, `warrant-ir` or `ca-assurance-report` re-selects every adopter |
| A4 | Runtime evidence in the report; strict CI wiring | CAP-RT-002, CAP-RT-004 | CAP-RT-001 (done) | the report has a runtime rung that `--check` requires on Linux; `RCG_STRICT=1` set on Linux runners |
| A5 | CWE-409 profile pin | LP-010b | none | `--check` cannot verify an `Excluded` CWE-409 row for a decompressor whose BUILD omitted `--profile` |
| A6 | `Bytes<N>` and fixed-array generator support | WX-B2 (new) | none | `[u8; N]` and `[T; N]` parameters and results generate evidence; the 17 crates the inventory finds blocked by `Bytes<N>` alone become T0 |
| A7 | 128-bit integers | WX-I128 (new, low) | after A6 | `u128`/`i128` in the type system, evaluator and generator; 72 crates have it as a primary blocker, none unblocked by it alone |
| A8 | Single binding table | WD-006 | none | the op-to-export map lives in one place, not in both `gen_evidence.rs` and `warrant_proofs.rs` |
| A9 | Contract linking | LV-005 (VER29 slices) | A1, A3 | VER29 section 13 slices land in order; the 4004 chain's upper layers cite published claims |

**Why A1 comes first.** Today an adoption edits `ca-assurance-report`, a crate
every adopter depends on. That causes two costs. Concurrent adoptions conflict
in the shared crate (section 5.3), and any change to it rebuilds 554 packages.
Moving the per-package profile into a data file removes both: the shared code
changes once, and each adoption touches only its own package. Until A1 lands,
report-profile edits are serialised (section 5.3).

**Notes on the others.**

- A2 (LP-012) is a correctness item, not a scaling one: the headline case count
  of arithmetic (2,655,437) is 859,848 exhaustive, 1,279,161 sampled and 516,428
  bounded-set. Wave 1 of track B must not report numbers that carry this flaw
  into 20 more crates.
- A6 is cheap and large: `Bytes<N>` is a primary blocker for 128 crates, and the
  crypto crates (x25519, ed25519, chacha20-poly1305, hmac, hash-functions)
  are among the 17 it unblocks alone. Fixed arrays of non-byte elements
  (116 crates primary) unblock only 4 crates on their own, and are in this
  item because the generator change is the same shape.
- A7 is low priority: it unblocks no crate alone.
- A8 (WD-006) is a prerequisite for adding bindings at the rate track B needs;
  duplicated tables drift.
- A9 follows VER29. Its implementation slices and their order are defined
  there; this roadmap only places them after A1 and A3, because they edit the
  same shared crates.
- AR-001 (`--check` does not re-derive the profile or the reason text) is a
  correctness defect in the same crate as A1 and should land in the same PR
  series.

### 4.2 Track B: adoption waves over T0, security-weighted

Order is by the inventory's value function: `max(ports, 1) x (1 + direct
dependents) x weight`, with weight 3 for crypto, constant-time and KDF crates,
2 for a codec or a parser of untrusted input, 1 otherwise. Already adopted
crates are excluded. `ports = 0` means no other language has the package, so
those Rust-only parsers rank by dependents and weight alone.

**Wave 1** (inventory section 4, after confirming each crate per section 2.3):

| # | Crate | Value | Note |
|---:|---|---:|---|
| 1 | `hkdf` | 507 | `expand_fixed` slice-writer variant needs WX-G |
| 2 | `der-tlv` | 364 | `DerReader` object is secondary |
| 3 | `argon2id` | 273 | heavy per-case runtime; bound `memory_cost` in the domains |
| 5 | `lzss` | 136 | same API shape as lzw (LP-003) |
| 6 | `deflate` | 128 | has the `max_output` variant (`inflate_counted`) |
| 8 | `argon2d` | 78 | |
| 9 | `pbkdf2` | 78 | |
| 10 | `reed-solomon` | 64 | |
| 11 | `zstd` | 64 | has `decompress_with_limit` |
| 14 | `argon2i` | 39 | |
| 16 | `lz77` | 32 | same template as lzw |
| 17 | `scrypt` | 30 | |

Crates 4 (`font-parser`), 7 (`aes-modes`), 12 (`qr-code`), 18 and 19
(`aztec-code`, `data-matrix`) carry secondary blockers in the inventory (Bytes<N>,
WX-G, text, floats, collections) and wait for the feature that clears them.
`huffman-compression` (#15) and `brotli` (#23) are also T0 and join wave 1 as
capacity allows.

**Wave 2: the X.509 and TLS parsers**, Rust-only, ranked by dependents:
`x509-rsa-public-key` (#13; arguable, since its input is another crate's SPKI
handle and it may be T3), `x509-name` (#20; needs WX-G and has a stateful
secondary), `x509-time` (#21; not hand-read), `tls13-cipher-suite-policy` (#22).
These have no cross-language multiplier, so their value is untrusted-input
rigour and dependents. The layered `*-link` and `*-binding` validators are T3
(opaque handles) and are out of scope until VER29 contract linking can express
a handle as a cited claim.

**Wave 3: `Bytes<N>` crypto**, after A6: `x25519`, `ed25519`,
`chacha20-poly1305`, `hmac`, `aes`, `des`, and the `sha*` family. The SHA family
also wants WX-F (track C) for a streaming driver; the one-shot slice can be
adopted without it.

**Per-adoption checklist.** Every adoption, in this order, and not merged
without all of it:

1. **Spec first.** A contract document (like LZ01 or AR01) is committed before
   any `.warrant` file: operations, domains, claims, what is out of scope.
2. **The `.warrant` file**, written to the spec and checked by the generator.
3. **Bindings**, one per operation, through the single binding table (A8).
4. **A security review** of the new files and of any shared code touched,
   before the push.
5. **The strict runtime jail** (CAP-RT-003): the capability-runtime gate runs
   with `RCG_STRICT=1` on Linux.
6. **Honest case counts**: exhaustive, sampled and bounded-set reported
   separately (A2); no number in the PR text that the manifest does not carry.
7. **A would-build check**: the build tool's plan for the PR's diff is printed
   and read before pushing (section 5.4); a change that selects the whole
   repository is split or deferred.
8. **Lessons**: anything that went wrong or surprised is recorded with
   `code/scripts/lessons.py new` before the PR is opened.

Plus, as for every package, the README and CHANGELOG (or `CHANGELOG.d`) updates
that CLAUDE.md requires.

### 4.3 Track C: language features

Order is by what each unblocks for the security-weighted wave and by what is
already specced.

| Order | Item | Backlog id | Why here |
|---:|---|---|---|
| C1 | WX-G: `inout` and `ensures` (VER04k section 4) | WX-G | `zeroize` alone has 89 dependents; 25 more crates unlock; it is the 4004 sequential layer (section 4.5) |
| C2 | WX-F: streaming drivers (VER04k section 3) | WX-F | `sha*`, `md5`, `blake2b`, `range-coder`; 10 crates, 76 ports |
| C3 | WX-D1, then WX-D2: sequence constructors, then folds | WX-D1, WX-D2 | not an unlock but a model-quality multiplier: a differential loop model for 48 hash, codec, KDF and checksum crates (395 ports) |
| C4 | Text: specification | WX-T0 (new) | in parallel with C1 to C3; 119 crates alone, 133 more after Track A |
| C5 | Stateful objects: specification | WX-S0 (new) | in parallel; 268 crates once text exists |
| C6 | Text: implementation | WX-T1 (new) | after the WX-T0 spec is approved |
| C7 | Stateful objects: implementation | WX-S1 (new) | after the WX-S0 spec is approved, and after C2 (it shares the driver machinery) |

**Frozen, by decision 1:** floats, collections (maps, sets, `BinaryHeap`),
opaque types and generics. No spec, no implementation, until the inventory is
re-run and the demand is re-assessed (section 6).

**What the two new specs must say.** The designs come from a separate design
memo, not from this roadmap, so this section states requirements only.

*Text (WX-T0).*
- A text type in the language, with a defined encoding (UTF-8 is what Rust
  `&str` guarantees; the ports differ, and the spec must say what a port with
  another native representation is held to).
- Total behaviour on every input a generator can produce, including invalid
  UTF-8 where the Rust API takes bytes, and a stated rule for it.
- Bounded domains: text values enumerated by length and alphabet, with a stated
  upper bound, the same way byte domains are.
- Per-language generation: the corpus encoding (VER04i) carries text and its
  digests identically in every lane.
- No normalisation, locale or case-folding semantics unless a named adopter
  needs one; those are separate, later decisions.
- Interaction with WX-G (out-buffers that hold text) and with the existing
  `*_hex`-style conveniences the classifier excludes.

*Stateful objects (WX-S0).*
- A model of an object protocol: a constructor, a sequence of method calls, an
  observer of state. It extends the WX-F driver rather than inventing a second
  mechanism, and it must say how it relates to WX-G `inout`.
- Bounded sequences of calls, enumerated by length, with a stated bound.
- A reference model in the language for the state, so that evidence is
  differential and not just "did not crash".
- Fault behaviour: what a failed call does to the object state, stated per
  operation.
- The 4004 and RV32I CPUs (section 4.5) are the first customers and are the
  acceptance test for the spec: a design that cannot express `step` on a
  register file is the wrong design.

### 4.4 Track D: polyglot

The goal of section 1 depends on this track. It starts now (decision 2), in
parallel with Rust adoption, because it does not depend on the T0 list.

| Order | Item | Backlog id | Notes |
|---:|---|---|---|
| D1 | Lane artifact loader, registry join, digest checks | WD-004b1 | WD-004a is merged (PR #19236) |
| D2 | Lane-aware reports, CLI and `--check`, honest absent rungs | WD-004b2 | the cross-language report; keep Rust-only proof and effect attestations isolated from the other lanes |
| D3 | Per-language runners: Python, TypeScript, Go first | WD-1xx | one PR per lane: corpus runner, sabotage, manifest, report, for ct-compare first; then the other lanes (ruby, c, cpp, java, kotlin, csharp, fsharp, elixir) |
| D4 | Per-language runtime jails | CAP-RT-1xx | arm the filter after interpreter or VM startup (Python, Node, Go, JVM, .NET and so on) |
| D5 | Fan-out | per adoption | the 428 ports of the T0 crates, as each Rust adoption lands |

D5 is the payoff. Every Rust adoption in track B creates a corpus; D1 to D4 make
that corpus runnable in the lanes. A lane without a runner reports an honest
absent rung (D2) and is not counted as covered (section 6).

Python, TypeScript and Go are first because they have the largest port counts
among the languages with a working engine and a tractable runtime jail. If a
lane's jail (D4) cannot be built, the lane runs with the jail rung reported
absent; it is not blocked.

### 4.5 Track E: layer chains

Two chains show what stacking looks like (inventory section 6).

**The Intel 4004 chain.** Layers 1 and 3 (`logic-gates`, `arithmetic`) are T0
and adopted. `transistors`, below the gates, is T3 on floats; it is not needed
for the gate truth tables. The stall points, in order:

1. **Sequential logic.** `d_flip_flop(data, clock, &mut FlipFlopState)`,
   `register(.., &mut [FlipFlopState])` and `shift_register` mutate a record in
   place. This is exactly WX-G (an `inout` record, and an `inout` list of
   records, with an `ensures` post-state); no object protocol is needed.
   `sr_latch` and `d_latch` pass state as arguments and results, and are T0
   today. **WX-G (C1) is the 4004 sequential layer.**
2. **The CPU.** `intel4004-gatelevel` (2,039 LOC, 65 entries) and
   `intel4004-simulator` are `&mut self` CPU objects (`reset`, `load`, `step`,
   `run`, `snapshot`): stateful objects, with traces as a text secondary. They
   wait for WX-S1 (C7). A WX-F-style driver with a record observer is the
   cheapest route, since it is the same machinery as the hash drivers.
3. **The pure helpers above arithmetic** can be adopted earlier as slices:
   `intel4004-encoder` (T0; its `[u8; 2]` results are a Bytes<N> secondary, so
   after A6), and the `encode_*`/`decode_*` free functions of the simulator (47
   of its 52 entries are such T0 helpers, but they are not the CPU).
4. The assembler, compiler and backend crates around it are T3 text or
   collections, and follow WX-T1.

**The RV32I chain.** The same two T0 base layers, then everything above is
object-shaped. The pilot is the `riscv-simulator` **encoder slice**: 49 of its
82 entries are `encode_*` instruction encoders returning `u32` (`encode_addi`,
`encode_add`, `encode_mul` and so on), comparable to `intel4004-encoder` and
adoptable today as a layer-0 slice. The decoder and executor mutate a
`PipelineToken` and register file in place (WX-G for the single step; an object
protocol for programs; the required set is WX-F, WX-G, collections, fixed-array,
generics, stateful-objects, text). `riscv-encoder` is re-export glue (T4).
`riscv-gatelevel` has a 525k-flip-flop topology that makes exhaustive evidence
infeasible; differential evidence against the functional oracle is the only mode
there.

Both chains share three layers of blockers: WX-G, then stateful object
protocols, then text and collections for the toolchain around them.

## 5. Capacity and sequencing constraints

These bind the schedule regardless of how the tracks are ordered.

### 5.1 The macOS CI queue

macOS runners are the scarce resource; a PR that selects many packages waits in
the queue behind every other PR that does. The plan: keep Warrant PRs narrow
(section 5.4), and stagger adoption PRs rather than opening a wave at once.

### 5.2 Disk

A worktree and its build output are 1 to 20 GB, and unreaped worktrees have
filled the disk before. Every agent works in a sparse worktree under
`.claude/worktrees/` and removes it when its PR merges (CLAUDE.md, Workflow).
A new worktree is not started while disk headroom is short; the headroom
figure is an open question (section 7).

### 5.3 Shared-crate serialisation

`ca-assurance-report`, `warrant-grammar`, `warrant-ir` and the generator are
depended on by every adopter. Two concurrent PRs that edit one of them conflict,
and a merged edit rebuilds 554 packages. Until A1 lands, **edits to report profiles are
serialised**: one open PR at a time may touch `ca-assurance-report`. After A1,
only edits to the shared code itself are serialised; an adoption no longer
touches it.

### 5.4 The would-build discipline

Before pushing any Warrant PR, run the build tool in plan mode against the
branch diff (`--diff-base origin/main`) and read what it selects. Pushing is
allowed when the selection matches what the change should affect. A selection
that includes the whole of the Warrant dependents means a shared crate was
touched, and the change is split so that shared edits land alone (5.3).

### 5.5 Concurrency

At most about 3 concurrent building agents. More than that queues on the macOS
runners (5.1), fills the disk (5.2) and multiplies shared-crate conflicts (5.3)
faster than it multiplies merged work. Spec-only work (the WX-T0 and WX-S0
specs, this document) does not build and does not count against the limit.

## 6. Metrics

Tracked per inventory snapshot and reported in the PR that lands each item:

| Metric | Meaning |
|---|---|
| Crates adopted, per tier | adopted count out of the tier's crates (T0 119, T1 21, T2 35, T3 1056) |
| Ports covered | ports (of the port-sum, 3,569 total) whose lane runs the corpus with a live runner, not merely a port that exists |
| Exhaustive versus sampled | cases per kind (`full`, `sampled`, `bounded-set`) once LP-012 lands; before it, the headline figure is flagged unaudited |
| Jail coverage | adopters whose runtime-jail rung is live in strict mode, per lane |
| Classifier error | the spot-check rate (8.3% at the 2026-11-14 snapshot), re-measured on each run |

**Re-running the inventory.** The inventory is regenerated from `origin/main` by
the scanner whose stages are extract, classify, analyze and summarise. Its
scripts are not committed; backlog row VER30-INV lands them as
`code/scripts/warrant_inventory.py`, tested and ruff-clean. After that, a re-run
is one command, writing a dated `inventory-YYYY-MM-DD.csv` and
`summary-YYYY-MM-DD.md` beside the 2026-11-14 pair. The re-run schedule: at the
end of each adoption wave, and whenever a track-C item lands (which changes the
unblock table), and before the frozen features are reconsidered. The
hand-overrides recorded in the CSV `notes` column must be carried forward by
the script, not recomputed.

## 7. Risks and open questions

1. **T0 accuracy.** About 80% on T0. Mitigation: per-crate confirmation
   (section 2.3). Open: should the first re-run fix the known error families
   before wave 2?
2. **The 554-package rebuild.** Until A1, every report-profile change rebuilds
   554 packages and serialises the work. Risk: A1 itself is such a change and
   must land alone.
3. **Port undercounting.** Ports are same-name directory counts; renamed ports
   (`sha256` vs `sha-256`, `*-native`) are missed, so value for those crates is
   understated.
4. **Text design.** The unblock count for text (133 crates) assumes the design
   covers the common API shapes; a design that covers only byte-oriented text
   unblocks fewer. The design memo must be checked against the 119 crates the
   inventory says it unblocks alone.
5. **Stateful objects depend on WX-F.** If the WX-F driver does not generalise
   to observers over a record, WX-S0 may need its own mechanism; the 4004 CPU is
   the test.
6. **Polyglot semantics.** Languages disagree on integer width, text encoding
   and wrap behaviour. The corpus is generated from one document, but a lane
   whose native types cannot hold the domain (JavaScript numbers above 2^53) must
   report that honestly. Open: the lane-specific escape hatch per type.
7. **Jail coverage.** A lane without a feasible runtime jail (D4) caps its
   assurance. Open: is "jail absent, reported" acceptable for a lane to count as
   covered, or only for the weaker metric?
8. **Review capacity.** One security review per adoption (checklist item 4) is
   the real limit on wave speed; the limit of 3 concurrent agents (5.5) assumes
   reviews keep pace.
9. **The frozen features.** Floats, collections, opaque types and generics
   together add 654 crates (107 + 124 + 108 + 315) but little security-weighted
   value. The re-assessment criterion is not defined yet: proposed, the first
   re-run after wave 2 of track B.
10. **Disk headroom.** No threshold is set for starting a new worktree (5.2).
11. **Contract linking order.** A9 depends on A1 and A3, which are not done;
    if the 4004 upper layers become urgent, VER29 slices could land first, at
    the cost of a heavier shared-crate queue.

## 8. Decisions log

Owner decisions, 2026-11-14.

1. **Lift the feature freeze for text and stateful objects.** Spec both now;
   implementation follows each spec's approval. Floats, collections, opaque
   types and generics stay frozen until demand is re-assessed.
2. **Start the polyglot track now,** in parallel with Rust adoption: the
   cross-language report (WD-004b) and the per-language runners (WD-1xx), with
   Python, TypeScript and Go first.
3. **This roadmap is committed as a specification** (VER30).
