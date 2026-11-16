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
VER30-INV that section 4 introduces, plus (2026-11-16) WX-O0/O1, WX-M0/M1,
WX-P1, WX-FL0/FL1, VER30-INV2, the corpus v2 slices CV2-S1 to CV2-S13, WD-100,
WD-HARN1 and BT-TIMEOUT, and (2026-11-16, later) WX-R0/R1 (recursive types,
VER04s), TRIG-CONV, WX-O4a/WX-O4b (verify the two RSA lower crates) and WD-100b
(two cross-lane path rules).

**Data:** [`data/warrant-inventory/summary-2026-11-14.md`](data/warrant-inventory/summary-2026-11-14.md)
and [`inventory-2026-11-14.csv`](data/warrant-inventory/inventory-2026-11-14.csv)
(1,459 rows, one per Rust package), and the re-run with a `recursive-types` tag
(VER30-INV2, done 2026-11-16):
[`summary-2026-11-16.md`](data/warrant-inventory/summary-2026-11-16.md),
[`inventory-2026-11-16.csv`](data/warrant-inventory/inventory-2026-11-16.csv)
(1,499 rows) and [`method-delta-2026-11-16.md`](data/warrant-inventory/method-delta-2026-11-16.md)
(what changed in the method; the two snapshots are not directly comparable).
Every number in this document is taken from one of the two summaries, and
sections 3.1 to 3.3 are the 2026-11-14 snapshot unless they say otherwise;
section 3.4 is the 2026-11-16 one and is the decision-relevant table. When this text and the summary disagree, the summary wins and
this text has a bug.

**Direction (owner, 2026-11-14):** three decisions, recorded in section 8. In
short: the feature freeze is lifted for text and stateful objects (spec both
now); the polyglot track starts now, in parallel with Rust adoption; and this
roadmap is a committed spec.

**Direction (owner, 2026-11-16):** ten further decisions, recorded in section 8
(decisions 4 to 13). In short: the priority is two goals pursued together,
Warrant evidence in every language the repository uses and filling the language
gaps that make code harder to validate; the freeze is lifted for floats,
collections, opaque types and generics (specs VER04n, VER04o, VER04p, written
by other branches); the corpus moves to a `warrant-corpus-v2` profile; and the
shared-crate queue is fixed (section 5.3).

**Direction (owner, later on 2026-11-16):** seven further decisions, recorded in
section 8 (decisions 14 to 20). In short: the inventory re-run is done and
recursive types are lifted (spec VER04s, after VER04o); the `trig` ports converge
on one reference algorithm first and the floats pilot waits; the RSA pilot's
lower crates are verified first; the reviewers' remaining recommendations are
accepted; every stream tag and diagnostic code now lives in one registry (VER04i
section 2.6.1); and two cross-lane path rules are pinned for the conformance kit.

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

*(2026-11-14 snapshot. The decision-relevant re-run with the `recursive-types` tag is section 3.4.)*

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
security code. They were frozen on 2026-11-14 until the demand was
re-assessed; decision 5 of section 8 (2026-11-16) lifts that freeze. The
654 crates in step 9 to 12 of the table are an **upper bound**: counted alone
the four features unblock only about 26 (floats), 4 (collections), 35 (opaque
types) and 36 (generics) crates, and recursive types (`JsonValue`, `CborValue`,
every tree crate) block most of the rest. Decision 6 re-runs the inventory
with a `recursive-types` tag (VER30-INV2) before the order of the four is
treated as final.

Two cautions on the numbers. 985 of the 1,459 crates have no ports at all
(mostly TLS, X.509 and HTTP framing, smart home, board-vm, chief-of-staff);
their value is Rust-only evidence plus the runtime jail, and a security-weighted
order puts WX-G (19 crates alone, among them the `tls13-*`, `rsa-pss-*` and
`http1-*` crates) ahead of where the port-weighted table above puts it. And
WX-D1/D2 are not unlocks: only 2 crates have a signature that demands a fold.
Their value is quality, a differential model for the 48 T0-to-T2 hash, codec,
KDF and checksum crates (395 ports), so they are scheduled as a multiplier in
track C.

### 3.4 The 2026-11-16 re-run with `recursive-types` (VER30-INV2, done)

Source: [`summary-2026-11-16.md`](data/warrant-inventory/summary-2026-11-16.md),
`origin/main` at `a522491e`, 1,499 crates. The method changed in two ways that
make it not directly comparable with section 3.1 (type names now resolve through
a crate's own dependencies; the repository grew by 40 crates), and one feature
was added, `recursive-types`: a public entry whose signature names a type that
can reach a cycle in the crate's type-reference graph (the full rule and its
validation are in
[`method-delta-2026-11-16.md`](data/warrant-inventory/method-delta-2026-11-16.md)).
Tiers on this snapshot: T0 123, T1 21, T2 34, T3 1,090, T4 231. The baseline is
the lifted set of decisions 1 and 2 plus the generator gaps (`Bytes<N>`, fixed
arrays, int128, WX-G, WX-F, WX-D2, text, stateful objects): **492 crates**.

What the five features unblock **on top of that baseline** (new crates and ports;
T4 excluded; `async` stays unlifted):

| Lifted on top of the baseline | New crates | New ports |
|---|---:|---:|
| floats alone | 94 | 387 |
| collections alone | 38 | 125 |
| opaque types alone | 55 | 25 |
| generics alone | 115 | 236 |
| recursive types alone | **74** | 332 |
| the four (floats, collections, opaque, generics), no recursive types | **432** | 1,134 |
| the four **and** recursive types | **775** | **2,028** |

Reading:

* The four features alone add **432** crates beyond the baseline. Adding
  recursive types lifts that to **775** crates (2,028 ports): recursive types are
  worth another 343 crates on top of the four, and 74 on their own.
* Of the 701 crates (1,696 ports) that sit behind the four features once the
  baseline is lifted and recursion is ignored (the "about 654" population of
  3.3), **269 (562 ports), or 38%, also need recursive types** (296, or 42%, if
  recursion hidden in private fields is counted). The four alone unblock 432 of
  the 701.
* The **baseline itself shrinks, from 566 to 492 crates**, because 74 crates that
  looked adoptable after text and stateful objects (parsers on the shared
  `parser::ASTNode` and `grammar-tools::GrammarElement`, document-AST consumers,
  `json-parser`) carry the tag.
* **No real cryptography becomes newly reachable.** The 26 weight-2 and 3 weight-3
  crates newly reachable were read by hand: none is real cryptography (the
  weights are substring matches that mis-fire on names like `csharp-parser` and
  `barcode-*`); about 14 are genuine untrusted-input parsers or codecs
  (`json-value`, `json-serializer`, `bounded-json`, `json-parser`,
  `canonical-cbor`, `resp-protocol`, `wasm-types`, `wasm-validator`,
  `wasm-execution`, `wasm-runtime`, `wasm-wast-parser`, `html-tree-builder`, and
  two state-machine markup serialisers). The gain is **parsers and codecs**.
* The highest-value crates newly reachable are `parser` (137 dependents),
  `grammar-tools` (115 dependents), `json-value`, `json-serializer` and
  `wasm-types`; `wasm-types`, `json-parser` and `wasm-validator` are blocked by
  nothing but recursion.

**Correction to the per-feature labels.** The figures "about 26, 4, 35 and 36"
for the four features counted alone belong to **floats 26, collections 4, opaque
types 35 and generics 36** (the 2026-11-14 table, section 3.2 above, is correct
and was checked against the summaries). The brief that introduced the numbers
quoted the labels permuted (26 opaque, 4 collections, 35 generics, 36 floats);
that permutation is wrong wherever it was repeated. On the 2026-11-16 snapshot
the strict-alone counts are floats 31, collections 4, opaque types 34, generics
33 (37 if recursive types are also lifted); that reading is now nearly vacuous
because text and stateful objects are lifted, and the table above is the one to
use.

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
| A2 | Domain-kind counting | LP-012 | none (design exists) | every claim declares `full`, `sample` or `bounded-set`; the manifest counts per kind; the report calls only `full` exhaustive |
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
  of arithmetic (2,655,437) is 859,848 exhaustive, 1,279,161 `sample` and 516,428
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
6. **Honest case counts**: exhaustive, `sample` and bounded-set reported
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
| C8 | Opaque types: specification, VER04n with VER29 and VER04m amendments | WX-O0 (new) | first of the four new specs; foreign handles only through VER29 published lower contracts; it is the mechanism the `*-link` and `*-binding` validators wait for |
| C9 | Floats: specification, VER04p | WX-FL0 (new) | written in parallel with VER04n; integer-only softfloat model, never host libm |
| C10 | Collections plus the first generics slice: specification, VER04o | WX-M0 (new) | after VER04n (shares the key order and the customer set) |
| C11 | Implementation, in this order: opaque types, collections, generics, floats | WX-O1, WX-M1, WX-P1, WX-FL1 (new) | after WX-G, WX-D2, WX-F, LP-012, WX-S1 and WX-T1 (C1 to C3, A2, C6, C7); pilots `rsa-public-operation`, `csv-parser`, `hash-set`, `trig` |
| C12 | Re-run the inventory with a `recursive-types` tag | VER30-INV2 | **done 2026-11-16** (section 3.4); the order inside the four stands, recursive types follow them (C14, C15) |
| C13 | Model arithmetic semantics | WD-013 (existing), CV2-S1 | resolve to the VER04 section 3 wrapping semantics before gf256 v2 (risk 12) |
| C14 | Recursive types: specification, VER04s (recursive algebraic types) | WX-R0 (new) | **lifted by decision 15.** Written **after** VER04o (collections plus first generics, in progress on another branch), because it shares the variant, `List` and generic machinery; spec first, adversarial review |
| C15 | Recursive types: implementation | WX-R1 (new) | after WX-R0 and **after the four features** (WX-O1, WX-M1, WX-P1, WX-FL1); last item in the `warrant-grammar` queue of 5.3 |
| C16 | `trig` convergence: one reference algorithm, then every port | TRIG-CONV (new) | decision 16: separate work, specification first, **each port updated in its own PR**; the floats pilot (VER04p FL1-S6) waits for it |
| C17 | Verify the two RSA lower crates | WX-O4a, WX-O4b (new) | decision 17: `x509-subject-public-key-info` and `der-asn1` each get their own Warrant evidence **before** pilot stage P-b of VER04n |

**Unfrozen, by decision 5 (2026-11-16):** floats, collections (maps, sets,
`BinaryHeap`), opaque types and generics. Decision 1 (2026-11-14) froze them;
decision 5 lifts the freeze, spec first, each spec reviewed adversarially
before any implementation. The designs come from the frozen-features design
memo; this roadmap records only the order and the adopted defaults:

- Specs: VER04n (opaque types, with VER29 and VER04m amendments) and VER04p
  (floats) in parallel; VER04o (collections plus the first generics slice)
  after VER04n. Other branches write them; this roadmap does not.
- Implementation order: opaque types, collections, generics, floats, after
  WX-G, WX-D2, WX-F, LP-012, WX-S1 and WX-T1. Pilots, in that order:
  `rsa-public-operation`, `csv-parser`, `hash-set`, `trig`.
- Foreign opaque handles only through VER29 published lower contracts; no
  handle without a published lower contract.
- Generic claims are never `exhaustive`, except `all-instances(n/n)` over a
  closed universe; coverage is counted per lane and per instance.
- Floats: an integer-only softfloat model in every lane, never the host libm;
  a lane that fails the environment probe reports `not-live`; float values
  travel as bit patterns; float `==` is a type error (use `bits_eq`, `same` or
  `ieee_eq`).
- Collections: one canonical map order; a host whose iteration order is
  unspecified cannot bind to a `List` result.
- Recursive types are a fifth blocker. VER30-INV2 re-measured (section 3.4):
  the four alone add 432 crates, with recursive types 775, and decision 15 lifts
  the freeze on them too: spec VER04s after VER04o, implementation after the four.
- Floats pilot: `trig` waits for the convergence of its ports (decision 16,
  TRIG-CONV). `within_ulps` is strict at infinity, Go float evidence is
  amd64-only, and `f32_all` runs on compiled lanes only (decision 18).
- Opaque types pilot: stage P-b of `rsa-public-operation` waits until
  `x509-subject-public-key-info` and `der-asn1` have their own Warrant evidence
  (decision 17); no `eq=observational` (explicit `key_eq`), sealing is not an
  adoption gate, and no big-integer model (decision 18).

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
| D1 | Lane artifact loader, registry join, digest checks | WD-004b1 | WD-004a is merged (PR #19236); loader delivered on `claude/wd-004b1-lane-loader` (VER04i 8.2) |
| D2 | Lane-aware reports, CLI and `--check`, honest absent rungs | WD-004b2 | the cross-language report; keep Rust-only proof and effect attestations isolated from the other lanes |
| D3 | Per-language runners: Python, TypeScript, Go first | WD-1xx | one PR per lane: corpus runner, sabotage, manifest, report, for ct-compare first; then the other lanes (ruby, c, cpp, java, kotlin, csharp, fsharp, elixir) |
| D4 | Per-language runtime jails | CAP-RT-1xx | arm the filter after interpreter or VM startup (Python, Node, Go, JVM, .NET and so on) |
| D5 | Fan-out | per adoption | the 428 ports of the T0 crates, as each Rust adoption lands |
| D6 | Lane conformance kit | WD-100, WD-100b | `warrant-corpus-conformance-v1` (VER04i section 9.1), delivered in PR #19489; every new lane must pass it; the Go lane adopts it in a follow-up. WD-100b (decision 20) pins two more rules, Windows reserved device names and ASCII-only `win32` case folding, applied to Python, TypeScript, Go and the Rust loader (PR #19492) |
| D7 | Corpus v2 (`warrant-corpus-v2`) | CV2-S1 to CV2-S13 | in-lane enumeration plus a model IR the lane re-evaluates; gf256 is the first adopter after ct-compare (slice S8); slices and rules below |
| D8 | Lane order after Go | WD-1xx | ruby, java, csharp, kotlin, fsharp, c, cpp, elixir, lua, perl, haskell, swift, dart, ocaml, two at a time, each on corpus v2 once CV2-S6 and CV2-S7 land, each passing WD-100 |
| D9 | Harness supervisor hardening | WD-HARN1 | wait timeout, never block exit, poison recovery (from the 2026-11-14 Windows `rust/lzw` hang); second in the `warrant-grammar` queue (section 5.3) |

**Priority (decision 4, 2026-11-16).** Two goals are pursued together: (a)
Warrant evidence in every language used in the repository (about 18 lane
languages, every one already with a ct-compare port), and (b) filling the
Warrant language gaps that make code harder to validate and verify (tracks C
and A). Track D carries (a); track C carries (b). Neither waits for the other.

**Corpus v2 slices** (from the corpus design memo; `warrant-corpus-v2` is
specified by an amendment to VER04i, written on another branch):

| Slice | Kind | Content | Touches `warrant-grammar` |
|---|---|---|---|
| CV2-S1 | spec | amend VER04i with the v2 profile, the binding grammar, the arithmetic erratum, the manifest path decision | no |
| CV2-S2 | generator | export the resolved-domain enumerator and the post-slot resolver as public API; no behaviour change | yes |
| CV2-S3 | generator | new leaf crate `rust/warrant-corpus-gen`; ct-compare digests must equal v1 | no |
| CV2-S4 | oracle | `warrant_oracle` package: a FULL independent `.warrant` source reader, gating each adopter | no |
| CV2-S5 | fixture | `warrant-corpus-conformance-v1` at `ir.core` (shares WD-100) | no |
| CV2-S6 | lane | Python runner reads v2; `python/ct-compare` moves to v2 | no |
| CV2-S7 | lane | TypeScript runner, the same | no |
| CV2-S8 | adopter | gf256 (v2), then `python/gf256`, `typescript/gf256`, `go/gf256` | no |
| CV2-S9 | report | generic v2 lane validator in `ca-assurance-report`, joined to WD-004b1 | no (`ca-assurance-report` once) |
| CV2-S10 | level | `ir.fixed` and tuples; then wasm-leb128, lzw, x25519 | no |
| CV2-S11 | level | `ir.data`; then logic-gates, then arithmetic (the first VER29 lane link) | no |
| CV2-S12 | feature | `inout` and `ensures`; then zeroize | no |
| CV2-S13 | cleanup | delete v1 `corpus.rs`, `gen_corpus`, the old oracle and the v1 fixture | yes |

**Owner choices on corpus v2 (decision 7).**
- PR CI runs a corpus-chosen `slim` chunk subset; the `full` depth runs nightly
  on compiled lanes; the depth is shown in the labels.
- The oracle is a FULL independent `.warrant` source reader and gates each
  adopter (CV2-S4).
- The Go lane finishes on v1 and is ported to v2 alongside the Python and
  TypeScript lanes (CV2-S6, CV2-S7).
- The corpus compiler is a new leaf crate after one export PR (CV2-S2,
  CV2-S3); the remaining `warrant-grammar` touches are S2 and S13 only.

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

**The `warrant-grammar` queue (decision 12, 2026-11-16).** Edits to
`warrant-grammar` are serialised in this order, one open PR at a time:

1. WX-G (PR #19369);
2. harness supervisor hardening, WD-HARN1 (wait timeout, never block exit,
   poison recovery; from the 2026-11-14 Windows `rust/lzw` hang);
3. corpus v2 S2 export, CV2-S2;
4. WX-F;
5. WX-T1;
6. WX-S1;
7. WX-I128;
8. the four new features (WX-O1, WX-M1, WX-P1, WX-FL1);
9. recursive types (WX-R1), after the four (decision 15).

CV2-S13 (the v1 deletion) follows the last adopter conversion and is queued
behind item 9 unless the owner reorders it.

**Build-tool timeout.** PR #19483 adds a per-command timeout to the build tool
so a hung command can no longer consume a CI job silently (BT-TIMEOUT). It is
independent of the queue above and is the CI-side half of the WD-HARN1 fix.

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
| Exhaustive versus sampled | cases per kind (`full`, `sample`, `bounded-set`) once LP-012 lands; before it, the headline figure is flagged unaudited |
| Jail coverage | adopters whose runtime-jail rung is live in strict mode, per lane |
| Classifier error | the spot-check rate (8.3% at the 2026-11-14 snapshot), re-measured on each run |

**Re-running the inventory.** The inventory is regenerated from `origin/main` by
the scanner whose stages are extract, classify, analyze and summarise. Its
scripts are not committed; backlog row VER30-INV lands them as
`code/scripts/warrant_inventory.py`, tested and ruff-clean. After that, a re-run
is one command, writing a dated `inventory-YYYY-MM-DD.csv` and
`summary-YYYY-MM-DD.md` beside the 2026-11-14 pair. The re-run schedule: at the
end of each adoption wave, and whenever a track-C item lands (which changes the
unblock table), and before the order of the four unfrozen features is treated as final (VER30-INV2 adds the `recursive-types` tag). The
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
9. **The frozen features. Resolved (2026-11-16, decision 5).** The freeze on
   floats, collections, opaque types and generics is lifted, spec first.
   The 654 crates (107 + 124 + 108 + 315) were an upper bound: alone the four
   unblock about 26 (floats), 4 (collections), 35 (opaque types) and 36
   (generics). VER30-INV2 is done (section 3.4, decision 14): the four add 432
   crates beyond the baseline, 38% of the 701 behind them also need recursive
   types, and recursive types are now lifted too (decision 15). What remains open
   is only the order inside the four.
10. **Disk headroom.** No threshold is set for starting a new worktree (5.2).
11. **Contract linking order.** A9 depends on A1 and A3, which are not done;
    if the 4004 upper layers become urgent, VER29 slices could land first, at
    the cost of a heavier shared-crate queue.
12. **Model arithmetic has three semantics today.** The spec (VER04 section 3
    and VER04i section 2.7) wraps fixed-width arithmetic, both lane runners
    refuse an overflow, and the independent oracle uses unbounded Python
    integers; the v0 harness additionally evaluates `U8` as `Nat` (WD-013).
    ct-compare has no model arithmetic, so nothing has collided; gf256 does.
    Mitigation: resolve to the VER04 section 3 wrapping semantics (an overflow
    on `Nat` is a generation error) in CV2-S1, with differential tests in the
    oracle and both runners, before gf256 v2 (CV2-S8). Link: WD-013.

## 8. Decisions log

Owner decisions, 2026-11-14.

1. **Lift the feature freeze for text and stateful objects.** Spec both now;
   implementation follows each spec's approval. Floats, collections, opaque
   types and generics stay frozen until demand is re-assessed
   (superseded by decision 5, 2026-11-16).
2. **Start the polyglot track now,** in parallel with Rust adoption: the
   cross-language report (WD-004b) and the per-language runners (WD-1xx), with
   Python, TypeScript and Go first.
3. **This roadmap is committed as a specification** (VER30).

Owner decisions, 2026-11-16.

4. **Priority: two goals together.** (a) Warrant evidence in every language
   used in the repository (about 18 lane languages; every one already has a
   ct-compare port); (b) filling the Warrant language gaps that make code
   harder to validate and verify.
5. **Lift the freeze on floats, collections, opaque types and generics,** spec
   first, each spec reviewed adversarially. Spec documents and order, from the
   frozen-features memo: VER04n opaque types (with VER29 and VER04m
   amendments); VER04p floats, written in parallel with VER04n; VER04o
   collections plus the first generics slice, after VER04n. Implementation
   order: opaque types, collections, generics, floats, after WX-G, WX-D2,
   WX-F, LP-012, WX-S1 and WX-T1. Pilots: `rsa-public-operation`,
   `csv-parser`, `hash-set`, `trig`.
6. **Recursive types.** The memo found they block most of the 654 counted
   crates; the four features alone unblock about 26, 4, 35 and 36. Re-run the
   inventory with a `recursive-types` tag before deciding (VER30-INV2). 654 is
   an upper bound.
7. **Corpus v2 (`warrant-corpus-v2`).** In-lane enumeration plus a model IR,
   adopted per the corpus memo, with slices S1 to S13 (CV2-S1 to CV2-S13) and
   gf256 as the first adopter after ct-compare. PR CI runs a corpus-chosen
   `slim` chunk subset and the `full` depth runs nightly on compiled lanes,
   with depth shown in the labels; the oracle is a FULL independent `.warrant`
   source reader that gates each adopter; the Go lane finishes on v1 and is
   ported to v2 alongside Python and TypeScript. The memo's flagged problem,
   that model arithmetic has three semantics today (the spec wraps, the
   runners refuse, the oracle is unbounded), must be resolved to the VER04
   section 3 wrapping semantics before gf256 v2 (risk 12, WD-013).
8. **Floats.** An integer-only softfloat model in every lane, never the host
   libm. Lanes failing the environment probe are `not-live`; float values
   travel as bit patterns.
9. **Other defaults adopted from the memos.** Foreign opaque handles only
   through VER29 published lower contracts; generic claims are never
   `exhaustive` except `all-instances(n/n)` over a closed universe, with
   coverage counted per lane and instance; float `==` is a type error (use
   `bits_eq`, `same` or `ieee_eq`); one canonical map order, and an
   unspecified-order host cannot bind to a `List` result; the corpus compiler
   is a new leaf crate after one export PR.
10. **Lane conformance kit.** WD-100 (`warrant-corpus-conformance-v1`, VER04i
    section 9.1) is delivered on branch `claude/warrant-lane-conformance`.
    Every new lane must pass it; the Go lane adopts it in a follow-up.
11. **Lane order after Go:** ruby, java, csharp, kotlin, fsharp, c, cpp,
    elixir, lua, perl, haskell, swift, dart, ocaml, two at a time, each on
    corpus v2 once CV2-S6 and CV2-S7 land.
12. **Shared-crate queue.** `warrant-grammar` is serialised in the order given
    in section 5.3: WX-G (PR #19369), harness supervisor hardening (WD-HARN1),
    corpus v2 S2 export, WX-F, WX-T1, WX-S1, WX-I128, the four new features.
13. **Build-tool timeout.** PR #19483 adds a per-command timeout so a hung
    command can no longer consume a CI job silently (BT-TIMEOUT).

Owner decisions, later on 2026-11-16.

14. **VER30-INV2 is done.** The inventory was re-run with a `recursive-types`
    tag (section 3.4; data in `summary-2026-11-16.md`, `inventory-2026-11-16.csv`
    and `method-delta-2026-11-16.md`). Key numbers: the four lifted features
    alone add **432** crates beyond the baseline; with recursive types also
    lifted, **775** crates (2,028 ports); recursive types alone add **74**;
    **38%** of the 701 crates behind the four also need recursive types; and the
    baseline shrinks from **566 to 492** because 74 crates need recursion. No
    real cryptography becomes newly reachable; the gain is parsers and codecs.
    **Correction:** the 2026-11-14 per-feature labels were quoted permuted; the
    correct pairs are floats 26, collections 4, opaque types 35, generics 36.
    (Decision 6 above and the text of section 3.3 gave the numbers without
    labels; they now carry them.)
15. **Recursive types are lifted.** The spec is **VER04s** (recursive algebraic
    types), written **after** VER04o (collections plus generics, in progress on
    another branch). Implementation follows the four features (WX-R0, WX-R1;
    track C14, C15; `warrant-grammar` queue item 9).
16. **The `trig` ports converge on one reference algorithm,** as separate work
    (backlog row TRIG-CONV), spec first. Every port is updated in its own PR, and
    the floats pilot (VER04p) waits for it. This is VER04p FL-15.
17. **RSA pilot stage P-b: verify the lower crates first.** The two unverified
    lower crates, `x509-subject-public-key-info` and `der-asn1` (the two path
    dependencies of `x509-rsa-public-key`, which a maker plan calls), must each
    have their own Warrant evidence before P-b starts (backlog rows WX-O4a,
    WX-O4b). **This reverses** the VER04n review's recommendation (OWN-4, "accept
    it, printed"); VER04n section 20 and its pilot staging are updated.
18. **The reviewers' remaining recommendations are accepted,** and each open
    question is now a resolved decision dated 2026-11-16 in its spec:
    VER04p FL-14, `within_ulps` at infinity is strict (same infinity only);
    VER04p FL-16, Go float evidence is amd64-only; VER04p FL-10, `f32_all` runs
    on compiled lanes only and the model's exhaustive self-check on Rust only;
    VER04r Q1, the report reads the nightly `full` manifest as a digest-matched
    artifact, only `slim` is committed and there are no bot commits; VER04n OWN-1,
    no `eq=observational`, an explicit `key_eq` operation instead; VER04n OWN-2,
    sealing is not an adoption gate (`forgeable` fails, `not-established` blocks
    only `--require-closed`); VER04n OWN-3, no big-integer model for the RSA
    pilot.
19. **Stream tags and diagnostic codes have one registry** (VER04i section
    2.6.1). VER04l, VER04r and VER04p collided in `60` to `63`; the
    earliest-merged allocation was kept: VER04l `60`/`61`, VER04p `62`/`63`, and
    VER04r moved to `64` (tuple, record) and `65` (variant); VER04n keeps
    `80`/`81`. Diagnostic codes had no collision (VER04n's `W0750` to `W0759` is
    disjoint from every other block). Future specifications check the registry
    and add their row in the same PR.
20. **Conformance-kit follow-up** (backlog WD-100b, no code in this change). Two
    cross-lane gaps found in review are pinned in
    `warrant-corpus-conformance-v1` and applied to Python, TypeScript, Go and the
    Rust loader: Windows reserved device names (`CON`, `NUL`, `COM1` to `COM9`,
    `LPT1` to `LPT9`, `AUX`, `PRN`, with or without extensions, plus trailing dots
    and spaces) are refused, as the Rust loader in PR #19492 already does and the
    lanes do not; and `win32` case-insensitive comparison folds ASCII only, with
    non-ASCII compared exactly (fail closed), where Go and Python currently differ
    on `ſ`, `Σ` and `İ`. The rules are written into VER04i section 9.1 now, marked
    pending implementation.

Owner decisions, 2026-11-16, on VER04o's open questions.

21. **VER04o OWN-6: verify first, and the generics pilot moves down a layer.**
    `hash-functions` is verified first (backlog WX-P1a; `Bytes<N>` has landed,
    and its T0 functions are adopted first). `hash-map` becomes the generics
    pilot (WX-P1; it needs collections, generics and stateful objects, and its
    only path dependency is `hash-functions`). `hash-set` follows as the second
    adopter (WX-P1b). Consistent with decision 17; no exception.
22. **VER04o OWN-1: the evidence label is `sample`, not `sampled`.** It is the
    owner's original wording and the value manifests already emit. VER04o,
    VER04l, VER04m, VER04p, this document and the LP-012 backlog row are
    changed; `exhaustive-sampled` (a VER29 rung) is a different word and stays.
23. **VER04o OWN-2: hash-flooding evidence is not a language-wide shipping
    gate.** It is decided per crate, and the report always prints
    `flood-resistance: not evidenced`.
24. **VER04o OWN-5: generic claims across VER29 layers fail closed**
    (`generic-not-published`, VER04o CT10).
25. **VER04s owner questions OWN-R1 and OWN-R2 (2026-11-16).** OWN-R1: keep the
    VER30 order; recursive types are implemented after the four features
    (opaque types, collections, generics, floats), and only the groundwork slice
    R1-S0 may start early. OWN-R2: `canonical-cbor` is confirmed as the
    recursive-types pilot, as a partial adoption of 3 of 8 entries (`encode`,
    `try_encode`, `decode`), with the other five tabled as not covered;
    `bounded-json` is the second adopter (WX-R-PILOT2). Backlog rows WX-R0 to
    WX-R-PILOT2 and the VER04s amendment rows are in `VER-BACKLOG.md` and the
    portable Warrant queue.
26. PHY00a (trig convergence) owner answers: NaN beyond 2^29, 3-ulp gate, pinned tan/atan2 conventions, minor version bump per port (2026-11-16).
