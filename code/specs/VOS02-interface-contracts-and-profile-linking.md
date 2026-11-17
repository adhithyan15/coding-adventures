# VOS02 -- Interface Contracts and Profile Linking

**Status:** Draft. The rules below are specified. `vos-manifest` (the `contract.vos`, part and document parsers) and `vos-link gen-doc` / `check-doc` exist (VOS-033, section 10.1); `check-profile`, `check-owners` and the profile parsers do not (VOS-034).
The acceptance spike of section 13 ran the toy of
[VOS01](VOS01-component-contract-model.md) section 5 through today's Warrant
evidence generator (VER04h) with the ghost face of section 4: the document was
generated and run, 10 exhaustive claims and 328,960 enumerated cases passed, and
the spike found seven places where the generator and this program disagree
(section 13.4). Those are recorded in section 13 and drive the interim rules of section 4.4 and the
VER requests of section 9.3. The spike is not committed; the commands are in
section 13 so it can be rerun.

**Layer:** Program model, below VOS01. It answers the two questions VOS01
left open: how several implementations realise one contract package (the
interface-id decision, VOS-D003) and how a profile names, checks and ships a
chosen set (VOS-D009). VOS10 and every later component spec cite this document
for document layout, implementation kinds, the profile manifest and SH16.

**Parent:** [VOS00](VOS00-verified-os-program.md) (the verified OS program),
sections 4.4, 4.5, 4.7, 8 and 14.3 (decision D1).

**Builds on:** [VOS01](VOS01-component-contract-model.md) (component shape,
SH1-SH16, tiers, the `-any` recipe), [EMU00](EMU00-emulator-framework.md)
(board format, section 5.1; the `stack` token, Q14),
[VER29](VER29-layer-contract-linking.md) (contract ids, discharge, pins, reach),
[VER04h](VER04h-warrant-derived-evidence.md) (generated evidence),
[VER04j](VER04j-warrant-v1-extensions.md) (bindings, faults),
[VER04m](VER04m-warrant-stateful-objects.md) (E8, E11, E15, E18, V9),
[VER04n](VER04n-warrant-opaque-types.md) (opaque types),
[VER04i](VER04i-polyglot-warrant-evidence.md) (lane manifests) and the
[VER backlog](VER-BACKLOG.md) rows LV-002, LV-005, LV-006, LV-007, LV-008,
WX-G, WX-S1, WX-SV, WX-O0/O1 and LP-012.

---

## 0. How to read this document

Marks are those of VOS01 section 0: **today** means it exists on this branch in
the spec or code cited; **NEW(row)** means a row of the [VOS
backlog](VOS-BACKLOG.md) or the [VER backlog](VER-BACKLOG.md) closes it. A rule
is stated once, with its reason, and every later section refers to it by number
(`R1` to `R9`, `AM1` to `AM5`, `SH16` to `SH18`).

Four words are used strictly.

* **Contract package.** The package that owns an interface: its Rust trait, its
  Warrant text and its ceilings (`vos-ready-contract`).
* **Implementation.** A package that implements the trait. It has a **kind**
  (section 3): `production`, `reference` or `any`.
* **Profile.** One assembly of one implementation per interface, plus a spine and
  a HAL, in a named **class**: `model`, `evidence` or `image` (section 6).
* **Obligation.** A claim the contract package owns and every implementation
  that wants a tier must carry, character for character (R1, R4).

What this document does **not** do, so nobody expects it: it adds no contract id
shape, no domain check of caller arguments (VER29 VQ12; the `site_` claims of
section 5 stay VOS-owned), and no change to VER29 section 9.1 (no alias, no
deprecation). VER29 is amended only in its reach rules and its feature rule
(section 9.1).

---

## 1. Decision record

### 1.1 What was open

[VOS00](VOS00-verified-os-program.md) section 4.7 left one decision (D1 in
section 14.3): the contract package owns the ids and implementations discharge
them (option 1), or each implementation re-publishes under its own id with an
equivalence claim (option 2). It said "the program's direction is option 1".
Three backlog rows depended on it: VOS-D003 (how an implementation relates to
the contract document), VOS-D006 (how a by-value component with private state is
bound) and VOS-D009 (how a profile keeps `-any` and `ghost` out of an image).

### 1.2 Facts that forced the answer

* **VER29 is entirely unimplemented.** `warrant-grammar` has no `contract_id`,
  `ca-assurance-report` has no `contracts` block, the bindings reader has no
  `state=` key and the analyzer has no `--emit-uses` (checked by search on this
  branch). Every choice below must work with a validator that reads files, and
  must not need a VER29 slice that has not landed.
* **VER29's ids are tied to the implementing package.** The `.warrant` stem is the
  package directory (VER29 section 3.1), existence is read from the named
  package's own document (section 3.2), a link follows a direct dependency edge
  (sections 5.1(a) and 10), and one id has one text (the pin `claim_sha256`,
  sections 5.3 and 9).
* **VER04n does not solve the by-value state problem.** It forbids the shape;
  section 1.4.

### 1.3 Decision D1: option (c), generated documents with contract-owned text

Ids stay stock VER29: `contract.<impl-pkg>.<claim>`, published by the
implementation. "Interface conformance" is **digest equality**, checked by a VOS
validator: the implementation's claim has exactly the token stream the contract
package wrote. VER29 is amended only in reach (its section 8) and in its feature
rule (sections 5.1.1 and VQ14), never in sections 3, 5.1(a)-(e), 6 or 9. The rules
are R1 to R9 in section 2.1.

This is a refinement of VOS00's option 2: each implementation publishes under its
own id, as option 2 says, but the equivalence claim is replaced by copying, with
a digest and a regeneration check, so that nothing can drift and nothing is typed
twice.

**Why not option (a), "interface ids realised by implementations".**

| Reason | The VER29 text it collides with |
|---|---|
| An interface document has only lemmas, and a lemma earns no strength (VER04m V9, VOS-D005), so the contract package has nothing to publish | Section 3.2: existence is read from the named package's own `.warrant` |
| The realiser is chosen by the profile, which is a sibling of the spine, not a dependency of it | Sections 5.1(a) and 10: a link follows a direct dependency edge |
| Refinement claims mention the implementation's own concrete state, so "one id, one text" cannot hold across implementations | Sections 5.3 and 9: `claim_sha256` means one id has one text |
| Making (a) work needs a new id family and a resolver, which is a rewrite of slice 2 before slice 2 exists | Section 13: slice 2 is frozen for LV-006 |

**Why not a bare option (b), implementations simply publish their own ids.**
Nothing would tie an implementation's `pick_safe` to the contract's obligation:
an implementation could publish a weaker claim of the same name and every link
would resolve. Digest equality (R4) is the missing tie.

**Costs, stated.** The Warrant text is duplicated into N documents (mitigated by
generation, R2). The conformance authority is a VOS tool, which adds to the trusted
base (section 10). R5 excludes representations that are not canonical from the
cheap path (they carry the `representation` tier of section 2.7, a contract-owned closure argument, and are reviewed).

### 1.4 Decision D2: VER04n does not help, so a ghost face now and WX-SV later

VOS-D006 asked whether VER04n's opaque types let a component keep its `State`
private. They do not; the shape is out of scope there:

* VER04n OK2 forbids an opaque value in the result of an `op` and as `inout`;
  OK3 says an operation that returns a handle is not an operation. A by-value step
  `op(State, In) -> (Out, State)` returns the state.
* Section 18 puts transformers (`T -> T`) out of scope (deferred as O5).
* OB1: a handle binder is never `full`, so a step claim over handles could not be
  `exhaustive`, which VOS01's bar needs.
* OK2: no `==` on handles, and refinement is equality of abstract states.
* It needs the `v2` header and the v2 corpus.

OR5 routes "immutable, all fields readable, total builder" to the VER04m `codec`
(E11), which is specified for `inout` state only. So the workaround for now needs
no generator change: keep `State` private, and put a public-field **state image**
and forwarding wrappers in a `ghost` module (section 4). The Warrant document is
written against the image and does not change when a codec or a by-value codec
arrives. The VER requests that make the wrapper unnecessary are one backlog row,
WX-SV (section 9.3).

### 1.5 Reversal of VOS00 section 4.7

VOS00 section 4.7 stated option 1 as the program's direction. This document
reverses that. The reasons are the table of section 1.3; the consequences are the
rewrites of section 14 (VOS00 sections 4.5, 4.7 and 14.3; VOS01 sections 2.10, 5.3-5.6
and Q9), made in the same PR, and a decision-log entry in the
[backlog](VOS-BACKLOG.md).

---
## 2. Document layout: one generated document per implementation

### 2.1 The rules

These nine rules are the normative core. Each has its reason; sections 3 to 9
apply them.

**R1. Three kinds of text belong to the contract package.**

1. The **contract block**: abstract state M, abstract operations, `inv`, the tier
   relations and the `pre_` models (VOS01 section 2). It holds **models only**.
2. One **obligations block** per tier: `safety`, `policy`, `canonical` and `any`.
   Each is a list of whole `claim` declarations, as the contract author wrote
   them.
3. The **ceilings and configuration names** (section 2.5).
4. Optionally, a **lemmas part** (`warrant/lemmas.part`): the model-level lemmas of
   the contract (closure, totality, tier ordering). It is a contract-package part
   and is admitted by the same rules as the others, but it is **never embedded** in
   an implementation's document and `check-doc` never requires it (VOS-D064): a
   lemma calls no bound operation, so it is a fact about the abstract model and
   never subject evidence (VER04m V9), and today's generator refuses a claim that
   calls no operation (W0701) until a `lemma` kind exists (WX-SV item 3, VOS-D005).
   The contract's own model-level evaluation (VOS10 section 12) runs it over the
   contract block. Re-embedding it can be reconsidered when the lemma kind lands.

*Why:* the text an implementation must satisfy has one author. An implementation
author who may restate it may weaken it (section 1.3, option (b)).

**R2. The implementation document is generated.** `code/specs/<impl>.warrant` is
assembled as the contract block, then the hand-written **implementation block**,
then the obligations blocks the implementation carries. Blocks 1 and 2 of R1 are
copied verbatim between marker comments (section 2.3) with their SHA-256
recorded. Block 3 (`contract.vos`) is **not** copied: it is not Warrant, it has no place
in a `.warrant` document, and its digest **will be** pinned by the lock (`ceilings-sha256`,
section 6.3; `check-profile`, VOS-034), so a change to a ceiling regenerates no document
(section 10.1, D-GD1).  Until then `check-doc` binds only the tier list of `contract.vos`;
the ceilings and `config N` are not yet compared with the bound rows (VOS-D055). A regeneration check (`vos-link check-doc`, section 10) fails on drift,
in the style of `--check-module` (VER23a section 2.1) and `gen_evidence --check`
(VER04h section 1).

*Why:* VER29 section 15 says a Warrant document has no imports and section 3.1
says its stem is the package directory. Copying with a check is the only way to
get one text in N documents without a module system.

**R3. The implementation block declares a fixed vocabulary**, identical in every
implementation:

| Item | Form | Role |
|---|---|---|
| state record | `record Impl(...)` | the concrete state; fields differ, the name does not |
| initial state | `op init() -> Impl` | VOS01 section 2.2 |
| interface operations | `op <name>(...)` with the interface's own op names | the steps |
| abstraction | `op alpha(s: Impl) -> M` | VOS01 section 2.3 (interim form: section 4.4) |
| canonical builder | `model build(m: M) -> Impl` | a state of the image of M |
| implementation invariant | `model inv_impl(s: Impl) -> Bool` | the states the implementation can reach |
| configuration | `op cfg_<name>() -> ...` plus an `example` pinning it | VOS01 section 2.9 |

The `lin_`, `any_` and per-implementation record names of VOS01's first draft are
gone. *Why:* an obligation claim then has identical tokens in every
implementation, hence an identical `claim_sha256` (VER29 section 5.3), and
digest equality (R4) is decidable without understanding Warrant. What equal
tokens do **not** pin is what `build` and `alpha` mean in each implementation;
the `canonical` block (R5) and review constrain those, and section 12 lists the
residue.

**R4. Conformance means two things for every obligation the profile needs.**

1. `claim_sha256(contract.<impl>.<obligation>)` equals the contract package's
   digest of that claim, and
2. VER29 sections 5.1(b) to (e) hold for it: the claim is in the implementation's
   manifest, every suite passed, at least one is not an example, the
   implementation's own report passes `--check`, and it is not under a fault mode.

This is VER04n OF1's "restate and match" done by copy instead of by restatement.
Condition (a), the dependency edge, is carried by the profile package (R7).

**R5. Obligations quantify over `build(M)`.** The `safety` and `policy`
obligations state each step claim for every concrete state `build(m)`, `m` ranging
over M (section 2.6), so the binder is a complete domain of the abstract state
and the claim is `exhaustive` where the instance fits the budget (VOS01 section
5.6). The `canonical` block adds, for every step, `build(alpha(post)) == post`: the
reachable states are exactly the image of `build`. `alpha(build(m)) == m` is a
**safety-tier** obligation (VOS10 names it `safety_alpha_build`), not a canonical one:
it is field-agnostic and every implementation needs `build`, `alpha` and `dec` pinned,
including the representation-mode and `any` implementations that do not carry
`canonical`.
*Why:* without the canonical block, "every state `build(m)` behaves" says nothing
about a state the step reaches that is not in the image. An implementation whose
representation has history (a ring buffer's head position) is not canonical; it
carries the `representation` tier instead of `canonical` (section 2.7): the contract states, once and
with one digest, that `init` is in the image of an implementation-supplied `rep`, that every step from
an image state lands back in the image, and that safety holds over the whole image. It is flagged in
the report and reviewed. This is
VER04m E15's `codec-roundtrip` and `rebuild-equivalence` idea for by-value
state.

**R6. Kind is a `vos-link`-enforced rule over a declared kind, not something evidence alone establishes.**

| Kind | Must discharge | May not be used for |
|---|---|---|
| `production` | **exactly one of** the `canonical` block or the `representation` block (section 2.7), and every tier the profile needs | nothing |
| `any` | the `any` block (soundness and coverage); it may **not** carry `canonical` (a rule of `vos-link`, below) | an `image` profile (SH16) |
| `reference` | a deterministic executable model with no `bound` rows | an `image` profile |

**What `vos-link` enforces (the authority).** The kind is declared in the crate's
`vos.toml` and in the profile manifest row. Two author-written values that agree prove
nothing, so `vos-link` compares **both against a central crate-to-class-and-kind
registry** that it owns (backlog VOS-D030, the authority list of section 7.4); a crate
absent from the registry has no kind and cannot be a manifest row. For each crate
`vos-link` then checks, from the generated document and the crate's files (never from
evidence alone):

1. `kind = any`: the document carries the `any` block and **carries no `canonical`
   claim** (matched by `claim_sha256`); the `any` obligations type-check against its
   `Impl`, which therefore has the oracle field they name (`choice`), a structural
   check made by the Warrant type checker on the generated document; the crate name ends
   in `-any`; the kind never appears in an `image` profile (SH16).
2. `kind = production`: the document carries **exactly one of** the `canonical` block and the
   `representation` block (each by `claim_sha256`, every obligation of the tier) **and no `any`
   block**. For a `canonical` carrier a **structural** state check also holds: every field of
   `Impl` is in the cone of `alpha` and `build` (the Warrant `build` model sets it from M, and the
   ghost `alpha` wrapper reads it, SH17), so no field is an oracle or history register outside that
   cone; the crate's `evidence.bindings` binds no oracle-write export (the ghost `set_oracle`
   path). A `representation` carrier is **not** subject to the cone half (its history register is
   outside the cone by design): the representation block's own closure claims constrain it
   instead (section 2.7), and `rep` and `hist`, which that block names, exist and have the right
   types because the document type-checks (CD04). For `kind = any` the converse of the cone check
   holds: at least one `Impl` field is outside the cone (the oracle the `any` block names), and
   the document carries **neither** `canonical` **nor** `representation`. This is a check on the
   state's structure, not on any name; the crate name is only the lint below.
3. `kind = reference`: no `bound` rows, never in an `image` profile.

**What this rule is not.** It separates `any` from `production` in the *documents*; it is
not the barrier that keeps `-any` out of a shipped image. That barrier is SH13 (the
oracle-write path and `alpha` live behind the `ghost` feature), SH16 (no `-any` crate and
no evidence feature in the **resolved** graph of an `image` profile), the closed-world
manifest rows of section 7.2 and the hermetic build of VOS22. A mislabelled kind could
fool rule 2; it could not get past those.

**What canonical evidence does and does not exclude.** The canonical block (R5) starts
from `build(M)` and says every step lands back in the image of `build`. It excludes an
implementation whose representation carries history that `alpha` forgets (a rotation
pointer): that is why a round-robin scheduler carries the `representation` tier. It does **not**
exclude `-any`: `build` sets the oracle to 0, an oracle nothing moves is canonical, and
the evaluation in `vos-sched-contract-check` shows an oracle scheduler passing every
canonical claim. A claim that did exclude it (the first draft's `canon_build_alpha`)
would have to name the fields of `Impl`, so it cannot be one text with one digest across
implementations (R3). So the separation of `any` from `production` is the rule above,
checked structurally by `vos-link` (VOS-033, VOS-034), and the name pattern `vos-*-any`
is a two-way lint (section 7.2), never the authority.

**R7. The spine's document assumes no implementation ids.** The **profile
package** is the package with the direct dependency edges (VER29 section 5.1(a)),
so its generated `.warrant` carries the `assume platform:` rows, derived from the
manifest. A swap is one authored manifest line plus regenerated rows. Because the
links change but no claim text does, the commit needs no `Contract-Change-Accepted`
trailer (VER29 section 5.3.1: a new or removed link needs none).

**R8. Transfer from `-any` is a printed derived row, never a rung.** "Refines
safety", "`any` covers safety" and "this theorem holds over `-any`" appear in the
profile report as a derived line, `transferred, single-trace` (section 11). VER29
has no rung for it and this document adds none (VOS01 section 3.4, VOS-D007).

**R9. Keep the contract-block source inside the contract package directory** and
generate the `code/specs` copy. Until LV-002 lands, an edit to
`code/specs/*.warrant` selects nothing in CI (VER29 section 11.1); a contract
block edited inside the contract package selects every package that depends on
it by path prefix, which includes every implementation.

### 2.2 Where each block lives

```text
code/packages/rust/vos-ready-contract/        contract package (class K, trait only)
    src/lib.rs                                the Rust trait, Upd, Pick, Profile
    warrant/contract.part                     R1.1, the contract block source (models only)
    warrant/lemmas.part                       R1.4, model-level lemmas, NOT embedded
    warrant/obligations-safety.part           R1.2, one file per tier
    warrant/obligations-policy.part
    warrant/obligations-canonical.part
    warrant/obligations-any.part
    contract.vos                              R1.3, section 2.5
code/packages/rust/vos-ready-bitmap/          implementation, kind production
    src/lib.rs                                private State, impl ReadySet, ghost module
    warrant/impl.part                         the hand-written implementation block
    evidence.bindings                         VER04h bindings (section 4.5)
code/specs/vos-ready-bitmap.warrant           GENERATED by vos-link gen-doc; never edited
```

The generated document has no author. Review of a swap reads three things: the
contract package's `.part` files (changed only by a contract change), the
implementation's `impl.part` and its Rust, and the diff of the profile manifest.

### 2.3 Marker comments and digests

A marker is a Warrant comment line, so the document type-checks unchanged
(section 13 confirmed it). Markers are the only syntax this document adds.

```text
-- vos02:generated by vos-link gen-doc; do not edit; contract-package=<crate>
-- vos02:contract-begin sha256=<64 hex>
...                                  the contract block, byte for byte
-- vos02:contract-end
-- vos02:impl-begin
...                                  the implementation block
-- vos02:impl-end
-- vos02:obligations-begin tier=<safety|policy|canonical|any> sha256=<64 hex>
...                                  that tier's obligations, byte for byte
-- vos02:obligations-end
```

The digest of a copied block is the SHA-256 of the bytes between its begin and end
marker lines, exclusive, with LF line endings (the `.part` files already carry the
two-space indentation, so the copy is the file and its digest is the file's). The marker carries it so a reader can see drift without the tool.
Separately, each obligation has its VER29 `claim_sha256` (the comment-free token
stream of the claim, section 5.3 there), which is what the lock pins (section 6.3).
A marker digest is a file-level check; `claim_sha256` is the identity.

**Markers cannot be forged.** The markers are written only by `gen-doc` and read by a
strict parser:

* `gen-doc` **rejects** any line beginning `-- vos02:` (after optional spaces) in every
  `.part` file and in `impl.part`, and rejects every control character except LF (CR and tab included; ASCII only) in them. Digests
  are over LF bytes; CR is rejected, never normalised, so two byte strings cannot
  share a digest by line-ending trick.
* Each marker appears **exactly once** and in the fixed order shown (generated,
  contract, impl, then the tiers in sorted order); a repeated, missing, reordered or
  unknown marker fails `check-doc`. A marker-looking text inside a Warrant string or
  comment elsewhere in the document is a failure for the same reason.
* `carries` (section 6.3) is **computed from the parsed Warrant claims and their
  `claim_sha256`** matched against the contract package's obligations, never from the
  `tier=` attribute of a marker. A claim moved between blocks, or a block relabelled,
  changes nothing about what is carried.

### 2.4 A generated document, worked

The listing is the real output of the acceptance spike (section 13), elided where
marked: the VOS01 section 5 toy, implementation 1 (the bitmap), carrying all three
tiers it can. The digests are real. The names `insert_out`, `insert_img`,
`alpha_w` and `dec` are the interim vocabulary of section 4.4; they exist because
today's generator can bind no fault to a tuple result.

```text
warrant vos_ready_bitmap v1
  -- vos02:generated by vos-link gen-doc; do not edit; contract-package=vos_ready_contract
  -- vos02:contract-begin sha256=ce96dafc6e7b521126786ebc700a9ceb340d18ebe2a0bd0e8c87287b9954bb12
  variant Upd = Changed | Unchanged | OutOfRange
  variant Pick = NoneReady | Member(p: U8)
  ... 23 more lines of the contract block, byte for byte from vos-ready-contract/warrant/contract.part
  -- vos02:contract-end
  -- vos02:impl-begin
  record Impl(mask: U8)
  record AbsW(w: U8)
  record CfgN(v: U8)
  model dec(x: AbsW) -> Ready = (
  ...                                   (the 8-way bit_and body of dec)
  op init() -> Impl
  op insert_out(s: Impl, p: U8) -> Upd
  op insert_img(s: Impl, p: U8) -> Impl
  op remove_out(s: Impl, p: U8) -> Upd
  op remove_img(s: Impl, p: U8) -> Impl
  op pick_out(s: Impl) -> Pick
  op pick_img(s: Impl) -> Impl
  op alpha_w(s: Impl) -> AbsW
  op cfg_n() -> CfgN

  model b2m(b: Bool, k: Nat) -> U8 = if b then shl(1u8, k) else 0u8
  model build(m: Ready) -> Impl = (
    Impl(bit_or(bit_or(bit_or(bit_or(bit_or(bit_or(bit_or(b2m(m.p0, 0), b2m(m.p1, 1)), b2m(m.p2, 2)), b2m(m.p3, 3)),
           b2m(m.p4, 4)), b2m(m.p5, 5)), b2m(m.p6, 6)), b2m(m.p7, 7))))
  model inv_impl(s: Impl) -> Bool = true

  example n_is_eight:
    cfg_n() == CfgN(8u8)
  example init_is_build_of_empty:
    init() == build(empty())
  example alpha_reads_bit_i_as_priority_i:
    dec(alpha_w(Impl(5u8))) == Ready(true, false, true, false, false, false, false, false)
  labels insert_out: ... labels alpha_w:    (7 labels rows, all `public`)
  ...                                   (bound rows against the ceilings of contract.vos go here)
  -- vos02:impl-end
  -- vos02:obligations-begin tier=safety sha256=9ce29b29ff60f33c81bf1b5e4e1d398029f6a31a788b6ad401e0926df6663073
  claim insert_safe:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool, p: U8. (
      insert_out(build(Ready(b0, b1, b2, b3, b4, b5, b6, b7)), p) == abs_insert(Ready(b0, b1, b2, b3, b4, b5, b6, b7), p).0 and
      dec(alpha_w(insert_img(build(Ready(b0, b1, b2, b3, b4, b5, b6, b7)), p))) == abs_insert(Ready(b0, b1, b2, b3, b4, b5, b6, b7), p).1)
  ...                                   (remove_safe, pick_safe)
  -- vos02:obligations-end
  -- vos02:obligations-begin tier=policy sha256=69693029bde7044f65d0282fa56b222f774e8ea82e8a82e670f190b5a8b17cbd
  claim pick_policy:
  ...                                   (pick_policy: the same shape, with policy_pick)
  -- vos02:obligations-end
  -- vos02:obligations-begin tier=canonical sha256=3880c68f72bfad98ce37f3b03fb61dc4527b2ad9b4ab7b7711cfd97802189666
  claim canon_alpha_build:
    forall b0: Bool, b1: Bool, b2: Bool, b3: Bool, b4: Bool, b5: Bool, b6: Bool, b7: Bool. (
      dec(alpha_w(build(Ready(b0, b1, b2, b3, b4, b5, b6, b7)))) == Ready(b0, b1, b2, b3, b4, b5, b6, b7))
  ...                                   (canon_insert_post, canon_insert_img, canon_remove_img, canon_pick_img)
  -- vos02:obligations-end
  assume platform:
    none
  assume environment:
    none
```

(The `canon_alpha_build` of this listing is, in VOS10's real contract package, the
**safety** claim `safety_alpha_build`: R5 above.)

What the author wrote: `impl.part` (lines from `impl-begin` to `impl-end`) and,
in the contract package, the `.part` files. Everything else is assembled. Two
properties to check in the listing: the claim `insert_safe` is the same text in the
linear-scan implementation's document (only `build`, `alpha_w` and the `Impl`
fields differ, and those live in the implementation block), and the document ends
with `assume platform: none`, because an implementation assumes nothing; the
profile package's document carries the rows (R7, section 6.4).

### 2.5 The contract file: ceilings and configuration

`contract.vos` is the contract package's authored text outside Warrant: the
interface name, the tiers it defines, the configuration names with their values,
and the ceilings. It uses the canonical-text discipline of section 6.1 (one
spelling, so its digest is its identity).

```text
vos-contract 1
name vos-ready-contract
iface ready-set
tier any
tier canonical
tier policy
tier safety
config N 8
ceiling init steps 160
ceiling insert depth 2
ceiling insert heap 0
ceiling insert steps 160
ceiling pick depth 2
ceiling pick heap 0
ceiling pick steps 160
ceiling remove depth 2
ceiling remove heap 0
ceiling remove steps 160
end
```

Grammar (sorted and unique throughout, one spelling, parse then re-emit byte-equal,
caps of section 6.8):

```text
contract = "vos-contract 1" LF "name " PKG LF "iface " IDENT LF
           { "tier " TIER LF }                         ; sorted: any, canonical, policy, safety
           { "config " KEY SP DEC LF }                 ; sorted by key
           { "ceiling " OPNAME SP METRIC SP DEC LF }   ; sorted by (op, metric)
           "end" LF
METRIC   = "depth" | "heap" | "steps"
OPNAME   = [a-z] [a-z0-9_]{0,31}      ; a Rust and Warrant operation name: underscores allowed (VOS-D051)
```

The figures are VOS01 section 5.1's placeholders (`16 N + 32 = 160` steps). The
validator compares each implementation's `bound` rows with them (section 8) and
the lock pins the file's digest.

### 2.6 The vocabulary, and which names are free

Fixed by R3: `Impl`, `init`, the interface op names, `alpha`, `build`,
`inv_impl`, `cfg_<name>`. Fixed by the contract: every model and claim of the
contract block and the obligations. One document is generated **per instance**: `config N 8` in `contract.vos`
is the instance, and the `.part` files spell it out (VOS01 Q1: a template generator
is later work). Free to the implementation: the fields of
`Impl`, any helper model (`b2m`), any extra `claim` or `example` (named with no
collision: a name equal to an obligation's is an error, which the Warrant checker
already reports as a duplicate declaration). Extra claims are published by VER29
like any other and printed as `impl-own` in the report; they never count towards
conformance (R4 reads obligations only).

### 2.7 Modes

A production implementation's *mode* is **derived, never declared**: it is the tier its document
carries (R6). Two author-written values that agree prove nothing (R6), and a `mode` key in
`vos.toml` would be exactly that, so there is none. (An earlier draft of this section, VOS-D061,
had one; see "Why there is no lint" below.)

| Mode | Carries | Meaning |
|---|---|---|
| `canonical` | the `canonical` tier | every reachable state is in the image of `build` (R5) |
| `representation` | the `representation` tier | the state carries history that `alpha` forgets (a rotation pointer); the reachable states are the image of `rep`, below |

A `representation` implementation is never silently equal to a canonical one: `vos-link`'s
report header and the profile report print `mode representation` beside the kind, derived from
`carries`.

**The representation tier is the contract's.** The tier is a fifth obligations block
(`tier representation` in `contract.vos`, `warrant/obligations-representation.part`), sorted
between `policy` and `safety`, pinned by `claim_sha256` like every other. The implementation
supplies two models in its `impl.part`:

```text
model rep(m: M, h: U8) -> Impl      -- an abstract state and a history code: the state they name
model hist(s: Impl) -> U8           -- the history code of a state
```

The block names no field of `Impl`, no `inv_impl` and no packing model; the **contract owns the
packing**: the history is a `U8` *code* of which the text uses three bits (`bit_and(k, 7u8)`), and
the one extra `U8` binder `k` also carries the thread argument (`shr(k, 3)`, 0 to 31), so a claim
has nine binders (eight `Bool`s and `k`: 65,536 cases) and not ten (a third `U8` binder is a hard
W0706). An implementation with more than eight history values needs a contract revision, a new
tier text with a new digest, not a local workaround. With `M` = `ready_of(b0, .., b7)`,
`S` = `rep(M, bit_and(k, 7u8))`, `T` = `shr(k, 3)` and
`R(P)` = `rep(dec(alpha_w(P)), bit_and(hist(P), 7u8))` (the abbreviations are **expanded
verbatim** in the part, which holds claims only), the claims are:

| Claim | Says |
|---|---|
| `rep_init` | `init() == R(init())`: the start state is in the image of `rep` |
| `rep_alpha` | `dec(alpha_w(S)) == M`: `alpha` forgets the history and nothing else |
| `insert_rep`, `remove_rep` | the output and the abstraction of the post-state of `insert` (`remove`) from `S` are the abstract step's: `abs_insert(M, T)` |
| `insert_rep_post`, `remove_rep_post` | `R(insert_img(S, T)) == insert_img(S, T)`: the post-state is again in the image |
| `dispatch_rep` | `safe_dispatch(M, dispatch_out(S), dec(alpha_w(dispatch_img(S))))`: safety from `S` |
| `dispatch_rep_post` | `R(dispatch_img(S)) == dispatch_img(S)` |

*Why it closes.* This is the `canonical` argument with `rep` for `build`: `init` is in the image,
every step from an image state lands back in it (the `_post` claims, the `canon_*_post` pattern),
and the safety relation holds from every image state. So safety holds over every reachable state,
not only over `build(M)` (after one `dispatch` a pointer has left its `build` value). A degenerate
`rep` (one that ignores `h`, or derives it from the mask) is not an image of the reachable states
for a real round-robin and fails a `_post` claim: coverage is closed by **evidence**, not by a
lint over the shape of `rep`.

**`<op>_det` is optional and not gated.** An implementation may add claims `<op>_det` (the output
and the full post-state of a step equal an authored op-free model); they are `impl-own`
(section 2.6), never count towards R4, and nothing reads them. They remain useful as a mutation
sanity of the history (a dispatch that does not rotate but stays in the image still passes the
tier, and that is fine: safety, not fairness, is what the tier promises).

**Why there is no lint.** The first design checked authored `<op>_rep`/`<op>_det` claims
structurally (a `mode` key, a CD07-R pass over the claim shapes). Two security reviews showed
that a syntactic lint over author-written claims does not converge: constant `rep` arguments,
refinements, `not (a != b)`, wrapper models, `exists`/`not forall` polarity, tautological
`_rep` and `rep_init`, `implies`-narrowing and an alpha inferred from the author's own claims each
passed, and a representation crate was an escape hatch from the canonical block. Moving the claims
into the contract removes the surface: there is nothing authored to lint, and a tampered or
weakened claim is a digest mismatch (CD06), like any other tier.

**What the tier does not do.** `-any` could truthfully carry it (it is deterministic from `init`
and closed under its steps); the tier does not separate kinds. Kind separation stays with the
declared kind, the `any` block, SH13 and SH16 and the registry (VOS-D030, R6). The tier also says
nothing about fairness or rotation order, and `inv_impl` is not named: an implementation
invariant is whatever `rep` ranges over.

---
## 3. Implementation kinds and the `-any` discipline

### 3.1 Three kinds, enforced by `vos-link` (R6)

VOS01 section 3 gave `-any` its recipe (a bitmap plus an oracle register that
`alpha` drops, propose-validate-default, soundness and coverage with witnesses).
This section fixes how a *kind* is decided, because VOS-D009 asked that SH16 not
depend on the crate name.

| Kind | What the evidence is | What it earns | Where it may appear |
|---|---|---|---|
| `production` | the `canonical` block, and the tier blocks it carries | real behaviour; ceilings met | model, evidence, image |
| `reference` | a deterministic model with no `bound` rows, no ceilings claim | an executable oracle for monitors and tests | model, evidence |
| `any` | the `any` block: soundness, coverage and its negative controls | every safety-tier behaviour, for single-trace theorems (R8) | model, evidence |

A crate declares its kind in its `Cargo.toml`:

```toml
# vos.toml in the crate directory (Cargo.toml carries no VOS keys)
kind = "production"             # or "reference", "any"
evidence-features = ["ghost"]   # features that exist for evidence only; must be in the central list of section 7.3
contract = "vos-ready-contract" # the contract package this implementation realises (an implementation only)
obligations = ["canonical", "policy", "safety"]   # the tiers gen-doc embeds; sorted, unique, each a `tier` of contract.vos
```

The kind is **checked by `vos-link`**, not merely trusted, with the rules listed under
R6: a crate that declares `production` must have a generated document whose carries
include exactly one of `canonical` and `representation` (section 2.7) and no `any` block,
and a crate that declares `any` must carry the `any` block, may not carry `canonical` or `representation`
and has the oracle field the `any` block names. These are rules over the declared kind
and the document structure; canonical evidence alone does not separate the two (see
"What canonical evidence does and does not exclude" under R6). The name
pattern `vos-<iface>-any` is a lint in both directions: a crate named `-any` that
declares `production` is an error, and a `production` crate that carries an `any`
block is an error (negative controls NC4 and NC5, section 12), as is an `any` crate
that carries `canonical` (NC4b).

### 3.2 Why `any` is kept out of `canonical`, and by what

`-any` is the bitmap plus an oracle register `choice` (VOS01 section 3.2), so its
`Impl` is `Impl(mask: U8, choice: U8)` and `alpha` drops `choice`. A state with an
oracle is not a function of the abstract state, so `-any` is not canonical in the
sense of "every state of `Impl` is `build` of its abstraction"
(`build(alpha(s)) == s` fails for `choice != 0`). **But the canonical block of R5 cannot
say that**: the claim would name `Impl`'s fields, which differ per implementation, and
the block starts from `build(M)`, where the oracle is 0. Every step from such a state
keeps `choice` where it was, so an `-any` passes the R5 claims (evaluated in
`vos-sched-contract-check`). The earlier draft of this section and the listing in 2.4
claimed otherwise; that was wrong. The separation is therefore the `vos-link` rule of R6
(declared kind plus the structural checks), and SH16 (never in an `image`) and the
closed-world manifest rows of section 7.2 carry the safety consequence: `-any` cannot be
shipped because the manifest and the resolved graph say so, not because canonical
evidence fails.

### 3.3 The `any` block, in the fixed vocabulary

The soundness and coverage claims of VOS01 sections 3.3 and 5.5 move into the
contract package as the `any` obligations block, in the R3 names. They quantify
over `Impl(mask, c)` with the oracle as a binder, which is why they live in a
different block from the tier obligations (which quantify over `build(M)`):

```text
  -- obligations-any.part (illustrative; the oracle is a binder, so Impl has two fields)
  claim pick_sound:
    forall mask: U8, c: U8. (
      safe_pick(alpha(Impl(mask, c)), pick(Impl(mask, c)).0, alpha(pick(Impl(mask, c)).1)) and
      pick(Impl(mask, c)).1.choice == c)
  claim pick_covers_members:
    forall mask: U8, p: U8 | p < 8u8. (
      mem(alpha(Impl(mask, p)), p) => pick(Impl(mask, p)).0 == Member(p))
  claim pick_covers_none:
    forall mask: U8. pick(Impl(mask, 255u8)).0 == NoneReady
```

The text names fields of `Impl` (`choice`), so these obligations can only be
carried by an implementation whose state has them: carrying them is the `any`
kind. Coverage is itself falsifiable (a sabotaged `-any` must be rejected); that
needs variant faults inside tuple components, which is WX-SV item 2 (section 9.3).
Until it lands, the coverage claims have no negative control and the report prints
`controls: none (WX-SV item 2)`, not a label the controls would earn (VOS01 section
7, item 19).

### 3.4 What a profile may do with the kinds

A profile that uses `-any` for an interface says so on every theorem it
evidences (section 11, `rests on -any`). A `hyper` theorem (noninterference) may
cite only interfaces realised by a `production` implementation carrying `policy`
(VOS-D007): refinement can remove nondeterminism in a way that breaks a
hyperproperty, so a theorem over `-any` does not transfer (R8, VOS01 section 3.4).

---

## 4. The ghost face

### 4.1 The conflict

VOS00 rule 2 wants `State` fields private. The generator builds and reads a by-value
record from a `struct T path=P fields=...` binding line, which needs public fields
(VER04j section 14.1, item 22). VER04m E11's `codec` line is the designed answer
but is specified for `inout` state only and depends on WX-G, which is queued
behind WX-D2, WX-F and LP-012 (VER-BACKLOG, WX-S1). VOS01 Q3 recommended private
fields plus a ghost builder and deferred the binding support to VOS-D006; this
section resolves it with the generator that exists.

### 4.2 The shape

```rust
// vos-ready-bitmap/src/lib.rs  (natural form, with the SH17 prefix; the spike ran the
// interim form of section 4.4 with unprefixed export names)
#![no_std]
#![forbid(unsafe_code)]
use vos_ready_contract::{Pick, ReadySet, Upd};

#[derive(Copy, Clone, PartialEq, Eq)] pub struct Bitmap { mask: u8 }   // private: rule 2
pub struct BitmapReady;
impl ReadySet for BitmapReady { type State = Bitmap; const N: u8 = 8; /* init, insert, remove, pick */ }

#[cfg(feature = "ghost")]                       // SH13: evidence build only
pub mod ghost {
    use super::*;
    pub struct Image { pub mask: u8 }            // public fields, equal to State's (SH17)
    pub fn vos_ghost_insert(s: Image, p: u8) -> (Upd, Image) {
        let (o, Bitmap { mask }) = <BitmapReady as ReadySet>::insert(Bitmap { mask: s.mask }, p);
        (o, Image { mask })                      // exhaustive destructure, one UFCS call, restructure
    }
    /* vos_ghost_init, _remove, _pick, _alpha, _cfg_n: the same four-line shape */
}
#[cfg(feature = "ghost")]
pub use ghost::{vos_ghost_alpha, vos_ghost_cfg_n, vos_ghost_insert /* ... */};   // sections 4.3, 4.6
```

The wrappers forward to the production code through the same UFCS call the spine
uses (section 5), so the code the evidence runs **is** the code that ships, behind a
four-line adapter that a lexical rule can read.

### 4.3 Why wrappers, and why at the crate root

The spike resolved the first probe (section 13): a trait's associated function has
no `self`, and the bindings reader accepts only a **plain Rust identifier** as an
`export=` (`binding.rs`, `is_rust_identifier`; a path such as
`vos_ready_bitmap::ghost::insert` is rejected with "is not a Rust identifier").
The generated harness resolves every export through `use <crate>::*;`. So an
operation can be bound only if the crate root exports a free function of that
name; `BitmapReady::insert` is not nameable. Therefore each ghost wrapper is
re-exported at the root, under the same feature, by a `pub use`. The brief's
`export=...::ghost::insert` form needs VER04k section 4.6's path-valued `export=`,
which is implemented with WX-G.

### 4.4 The interim vocabulary

The spike found that today's generator cannot attach a negative control to the
natural form of the toy (section 13.3). Until WX-SV item 2 lands, the ghost
face of a **tuple-returning** operation exposes it as a projection pair, and the
abstraction and configuration are returned as one-field records of integers:

| Natural form (target, after WX-SV) | Interim form (today) | Why |
|---|---|---|
| `op insert(s: Impl, p: U8) -> (Upd, Impl)` | `insert_out(s, p) -> Upd` and `insert_img(s, p) -> Impl` | no fault applies to a tuple whose second component is not an integer |
| `op alpha(s: Impl) -> M` | `alpha_w(s: Impl) -> AbsW`, `model dec(x: AbsW) -> M` | no fault applies to a record of `Bool`s; `field-zero` needs an integer field |
| `op cfg_n() -> Nat` | `op cfg_n() -> CfgN`, `record CfgN(v: U8)` | no fault applies to a nullary scalar |

The interim names are part of the vocabulary of R3 for as long as they are needed.
When WX-SV lands the obligation text changes (one claim per operation instead of
an `_out` claim and an `_img` claim), the obligations digests change, every
implementation regenerates, and that is a **contract-text change event**
(VOS00 section 8), planned, in-tree and mechanical. It is the price of having run
today; the alternative was no evidence at all.

The `canonical` block uses a second spike finding. A negative control attaches to
a claim by its **subject operation**, the first operation found by a shallow,
left-to-right search (`find_operation_in_expr` in `warrant-grammar/src/codegen.rs`).
A refinement claim written `dec(alpha_w(insert_img(...))) == ...` has subject
`alpha_w`, so `insert_img` has no control that can catch a fault in it. The fix is
a claim whose subject is the image operation itself, the commuting form
`insert_img(build(m), p) == build(abs_insert(m, p).1)`, which needs no `alpha` and
is equivalent to refinement given `alpha(build(m)) == m`. R5's canonical block
carries these (`canon_insert_img`, `canon_remove_img`, `canon_pick_img`).

### 4.5 The bindings file

The generated harness needs bindings (VER04 section 6, VER04j section 14.1). The
spike's, for the bitmap, abridged (the spike ran with unprefixed `export=` names; SH17 now requires the `vos_ghost_` prefix, which changes only the `export=` and the wrappers' names):

```text
package vos_ready_bitmap
spec code/specs/vos_ready_bitmap.warrant
output code/packages/rust/vos_ready_bitmap/tests/generated/warrant_evidence.rs
struct Impl path=vos_ready_bitmap::ghost::Image fields=mask:u8
struct AbsW path=vos_ready_bitmap::ghost::AbsW fields=w:u8
enum Upd path=vos_ready_contract::Upd ctors=Changed,Unchanged,OutOfRange
enum Pick path=vos_ready_contract::Pick ctors=NoneReady,Member(0:u8)
op insert_out export=vos_ghost_insert_out args=Impl,u8 ret=Upd faults=wrong-ctor
op insert_img export=vos_ghost_insert_img args=Impl,u8 ret=Impl faults=field-zero
op alpha_w export=vos_ghost_alpha_w args=Impl ret=AbsW faults=field-zero
```

The package declares the test target with `required-features = ["ghost"]` and
the dev-dependencies the generated harness needs (`coding_adventures_sha256`,
`coding_adventures_test_bootstrap`), so a plain `cargo build` never compiles
the ghost module and a plain `cargo test` never runs the evidence.

### 4.6 SH17 and SH18, new shape rules

These extend VOS01 section 1.4's table (the shape check, VOS-D002).

| Id | Rule | Why | Checked by |
|---|---|---|---|
| SH17 | In the `ghost` module: **every item, and every crate-root re-export of one, is named with the prefix `vos_ghost_`** (the Warrant operation name is free; the binding's `export=` carries the prefixed name); every `Image` struct has exactly the fields of the `State` struct it mirrors, same names, types and order; every wrapper body is an **exhaustive destructure** of the production `State` (a struct pattern with no `..`), one UFCS call into the production trait, and a restructure into the image; no other statement | a field added to `State` and forgotten in the image breaks the build (the spike confirmed E0027 and E0063), so the image cannot silently describe a different state than the one that ships | NEW(VOS-D002): lexical, VER08a lexer |
| SH18 | No path containing `ghost::`, no `use` of a ghost item and no `cfg(feature = "ghost")` item appears in **non-ghost class K source**, except the `mod ghost` item itself and `pub use ghost::...` items that are themselves gated by the same feature. **Extension:** no `cfg(feature = "<f>")` or `cfg_attr` mentioning an **evidence feature** `<f>` (`ghost`, `cov`, or any name in `evidence-features`) appears anywhere in the crate except on that one `mod` and its gated `pub use` items; and every `cfg(feature = ...)` in the crate names a feature declared in `Cargo.toml` (section 7.3) | the ghost face must not become reachable from the shipped build; a gated re-export (section 4.3) is the one exception, and it is gated | NEW(VOS-D002): lexical |

SH17 is the structural half of "the ghost face is faithful": the wrapper cannot
compute anything, so what it forwards is what the production code returns. It
does not prove the image's *fields* are the abstraction the claims need; `alpha`
and the `canonical` block do that.

### 4.7 The `realises=` key

A binding-level key used by the reach pass (AM2, section 9.1): each `op` line of the
implementation's `evidence.bindings` may name the trait item it implements,
`realises=vos_ready_contract::ReadySet::insert`. It is the only place the
implementation's export (a ghost wrapper) is tied to the interface's trait item.
Until LV-008 lands, `vos-link check-doc` checks the keys against the contract
crate's trait by name (section 10).

---
## 5. The UFCS rule, the `impl Profile` audit target and `site_` claims

### 5.1 The rule

Spine call sites use UFCS through the profile's associated type
(VOS01 section 1.2):

```rust
let (upd, ready) = <P::Ready as ReadySet>::insert(k.ready, t);
```

`ReadySet` is a trait of the **contract crate**, which is a direct dependency of
the spine. The path `<P::Ready as ReadySet>::insert` therefore names an
*interface item* `(vos_ready_contract, ReadySet, insert)`, never an implementation.
The spine source is the same for every profile (rule 5 of VOS00 section 4.3), and
its Cargo manifest depends on contract crates and `core` only.

### 5.2 The audit target: the `impl Profile` block

One `impl Profile for <ProfileType>` block per profile type names the
implementations:

```rust
pub struct RtosMin;
impl Profile for RtosMin { type Ready = vos_ready_bitmap::BitmapReady; /* one line per interface */ }
```

The audit target of a profile is that block plus the profile manifest
(section 6). Rules, all lexical and checked by the shape check once it exists
(`vos-shape-check`, section 10), by hand until then:

1. Exactly one `impl <Profile trait> for <T>` item per profile type `T`
   (AM5, negative control NC12).
2. Each `type A = <path>;` names a type of a crate that is a manifest row (the
   static closed-world rule of section 7.2).
3. The spine and the leaf crates contain no path into an implementation crate.
4. Every interface use in the spine is UFCS through `P::<Assoc>` (VOS00 rule 5); a
   method-call form is a finding.

### 5.3 What the reach pass cannot see, and the amendment

VER29 section 8.2 resolves paths the binding closure can resolve into the lower
crate and lists generics and trait paths as not reachable (limit 4, "calls through
... generics"). The UFCS form is a stopgap: it keeps a grep-level audit possible
until the pass learns trait paths. AM1 to AM3 (section 9.1) teach it exactly this
one shape, and nothing else.

### 5.4 `site_` claims

VOS01 section 6.3 requires **one claim per spine call site**, in the spine's
document, named `site_<entry>_<step>`: the arguments satisfy the contract's
`pre_` model, discharged from the safety tier and the spine invariant. This
document adds two things.

* The match between a UFCS call site and its claim is lexical: a call site with no
  `site_` claim of that name is a conformance failure, and a `site_` claim with
  no call site is a stale claim. Checked by hand until `vos-shape-check` reads
  call sites (NEW(VOS-D002), second half).
* The `site_` claims stay VOS-owned. No VER29 domain check (VQ12) is requested:
  `covered (domain unchecked)` is printed and the `site_` claims are the actual
  evidence that the domain holds.

---

## 6. Profile and board manifests

### 6.1 Canonical text, authored and generated

EMU00 section 5.1 already fixed a canonical text format for boards: one record per
line, LF endings, ASCII only, one spelling per object, the file's SHA-256 is its
identity, and `parse` followed by `re-emit` must be byte-equal. **No board
manifest is invented here**: a profile references the board by name and the lock
pins its digest. The profile uses the same discipline, split like Cargo's pair:

| File | Who writes it | Role |
|---|---|---|
| `profile.vos` | a person | the authored choice: which implementation realises which interface, in which class |
| `profile.lock` | `vos-link check-profile --write-lock` | the derived pins, in VER29 section 5.3's names |

A swap is one authored line plus a regenerated lock. Both files are parsed by
`vos-manifest` (class P, section 10): total, no panic, size caps before
allocation, fail closed, and a re-emit equal to the input, so no two byte strings
describe one profile.

### 6.2 The authored grammar

```text
profile  = "vos-profile 1" LF
           "name " IDENT LF
           "class " ( "model" | "evidence" | "image" ) LF
           [ "target " TRIPLE LF ]                        ; image only
           [ "board " IDENT LF ]                          ; names an EMU00 board file
           { "slot " IDENT SP "artifact=" IDENT LF }      ; image slots, sorted by slot
           [ "config " KEY "=" DEC { SP KEY "=" DEC } LF ]; keys sorted
           { "iface " IDENT SP "contract=" PKG SP "needs=" TIERS LF }   ; sorted by iface
           { "impl " IDENT SP "crate=" PKG SP "kind=" KIND SP "mode=" MODE LF }
           "spine crate=" PKG LF
           [ "hal crate=" PKG LF ]
           { "theorem " IDENT SP "needs=" NEEDS SP "property=" ( "trace" | "hyper" ) LF }
           { "build " BUILDNAME SP "target=" TRIPLE SP "features=" FEATURES LF }
           "end" LF
KIND     = "production" | "reference" | "any"
MODE     = "canonical" | "representation" | "none"   ; "none" only with kind=any (section 3.2)
TIERS    = TIER { "," TIER }            ; sorted: any, canonical, policy, safety
NEEDS    = IFACE "." TIER { "," IFACE "." TIER }
BUILDNAME= "evidence" | "image"
FEATURES = "none" | IDENT { "," IDENT }
IDENT    = [a-z] [a-z0-9-]{0,31}        PKG = IDENT        DEC = "0" | [1-9][0-9]*
TRIPLE   = "host" | [a-z0-9] [a-z0-9_.-]{0,63}   ; "host", or a name in `rustc --print target-list`
PATH     = [A-Za-z0-9_.-]+ { "/" [A-Za-z0-9_.-]+ }   ; relative, no "..", no leading "/", at most 256 bytes
KEY      = [A-Za-z] [A-Za-z0-9_]{0,31}   VALUE = [0-9A-Za-z_.,=-]+    ; VALUE only in the lock
```

Ordering is fixed (header, `name`, `class`, `target`, `board`, `slot`s, `config`,
`iface`s sorted by name, `impl`s sorted by interface name, `spine`, `hal`,
`theorem`s, `build`s, `end`), no comments, no blank lines, no trailing space. The class decides which rows are required: `model` and
`evidence` need no `board`, `slot` or `target`; an `image` profile needs `target`,
`board`, at least one `slot` and both builds.

### 6.3 The lock

The lock carries nothing a person decides. Every row is derived and every pin name
is VER29 section 5.3's, so when LV-008 and slice 2 land the `contracts` block of the
profile package's report is derived from the same inputs and migration is nil.

```text
lock = "vos-lock 1" LF
       "profile-sha256 " HEX LF
       [ "board " IDENT SP "sha256=" HEX SP "stack=emu:" IDENT "@" HEX16 LF ]  ; EMU00 Q14
       { "iface " IDENT SP "contract-sha256=" HEX SP "ceilings-sha256=" HEX
              { SP "obligations." TIER "=" HEX } LF }
       { "impl " IDENT SP "crate=" PKG SP "kind=" KIND SP "mode=" MODE SP "carries=" TIERS LF
         "pin " IDENT SP KEY "=" VALUE { SP KEY "=" VALUE } LF      ; spec_sha256 source_sha256 cargo_sha256
                                                                     ; build_rs_sha256 manifests report_sha256
                                                                     ; certificate_sha256 toolchain features
         { "obl " IDENT "." TIER "." CLAIM SP "claim_sha256=" HEX SP "claim_entries_sha256=" HEX LF }
         { "cfg " IDENT SP KEY "=" DEC SP "example=" IDENT LF } }
       [ "build image " ...  LF ]   ; record, image and toolchain digests: defined by VOS22
       { "theorem " IDENT SP "profile-sha256=" HEX SP "transfer=" ( "none" | "single-trace" ) LF }
       "end" LF
```

`features=` on an `impl` pin is the resolved feature set of that crate in **each
named build** (`evidence` and `image`); for an image it is a record of what
was found, checked against the policy of section 7.2 rule 4, never an expectation.
Every comparison that decides a gate uses the **full 64-hex SHA-256**; the 16-hex
`stack=` token is display only and is never compared (EMU00 Q14 states that the
full digest is what AC-7 compares). `cfg` rows record that the
`cfg_<name>` example literal was found and equals the authored `config` value, and
that a `const` assert in the profile crate repeats it (VOS01 section 2.9).

### 6.4 Worked example 1: a two-interface evidence profile

Illustrative: the second interface and its `-any` do not exist yet (VOS10, VOS-012).
The profile checks the scheduler's safety-tier theorem over `-any` and its
policy tier on the bitmap.

```text
vos-profile 1
name sched-evidence
class evidence
config N=8
iface ready contract=vos-ready-contract needs=policy,safety
iface sleepq contract=vos-sleepq-contract needs=safety
impl ready crate=vos-ready-bitmap kind=production mode=canonical
impl sleepq crate=vos-sleepq-any kind=any mode=none
spine crate=vos-spine
theorem ready-in-set needs=ready.safety,sleepq.safety property=trace
theorem top-priority-runs needs=ready.policy property=trace
build evidence target=host features=ghost
end
```

An `any` implementation has no mode of its own (it is not canonical, section 3.2), so
its row reads `mode=none`, which is legal only with `kind=any`; `production` and
`reference` rows declare `canonical` or `representation`.

The generated lock for it, digests abbreviated to 16 hex:

```text
vos-lock 1
profile-sha256 <64 hex>
iface ready contract-sha256=ce96dafc6e7b5211... ceilings-sha256=... obligations.canonical=3880c68f72bfad98... obligations.policy=69693029bde7044f... obligations.safety=9ce29b29ff60f33c...
iface sleepq contract-sha256=... ceilings-sha256=... obligations.any=... obligations.safety=...
impl ready crate=vos-ready-bitmap kind=production mode=canonical carries=canonical,policy,safety
pin ready spec_sha256=... source_sha256=... cargo_sha256=... build_rs_sha256=absent report_sha256=... certificate_sha256=... toolchain=<rustc version and target> features=ghost
obl ready.safety.insert_safe claim_sha256=... claim_entries_sha256=...
obl ready.safety.remove_safe claim_sha256=... claim_entries_sha256=...
obl ready.safety.pick_safe claim_sha256=... claim_entries_sha256=...
obl ready.policy.pick_policy claim_sha256=... claim_entries_sha256=...
cfg ready N=8 example=n_is_eight
impl sleepq crate=vos-sleepq-any kind=any mode=none carries=any,safety
...
theorem ready-in-set profile-sha256=<64 hex> transfer=single-trace
theorem top-priority-runs profile-sha256=<64 hex> transfer=none
end
```

The `contract-sha256` and `obligations.*` values of `ready` are the spike's real
digests for the toy, cut to 16 hex (section 2.4); `<64 hex>` and `...` stand for
values this document does not compute. `transfer=single-trace` on the first
theorem is R8: it rests on `-any`; the second rests on a production implementation
and transfers nothing.

### 6.5 Worked example 2: an image profile on the RV32 board

The board is EMU00 section 5.2's `rv32-virt-lite` (`isa rv32i`, a UART, a timer,
RAM with a `kernel` slot and a ROM with a `bootrom` slot). The profile references
it by name; the lock pins its SHA-256 and the `stack` token EMU00 Q14 defines.
The image has **two named builds** (the mechanism of the image build is VOS22, section 7.1): the `evidence` build on the host with
`ghost` on, and the `image` build for the target with no features.

```text
vos-profile 1
name rtos-min
class image
target riscv32i-unknown-none-elf
board rv32-virt-lite
slot bootrom artifact=bootrom
slot kernel artifact=kernel
config N=8
iface ready contract=vos-ready-contract needs=policy,safety
iface threads contract=vos-threads-contract needs=safety
impl ready crate=vos-ready-bitmap kind=production mode=canonical
impl threads crate=vos-threads-table kind=production mode=canonical
spine crate=vos-spine
hal crate=vos-hal-rv32
theorem isolation needs=ready.safety,threads.safety property=trace
build evidence target=host features=ghost
build image target=riscv32i-unknown-none-elf features=none
end
```

Illustrative names: `vos-threads-table` and `vos-hal-rv32` are VOS-024 and VOS-025.
The lock's board row reads

```text
board rv32-virt-lite sha256=<board file digest> stack=emu:rv32-virt-lite@<first 16 hex>
```

and `check-profile` requires the board's `isa` (`rv32i`) to equal the target's ISA
(the `riscv32i-` prefix) and every `slot` named in the profile to be an `image` slot
of the board (EMU00 section 5.1, rule V2 and V10). Every `impl` of an image profile
must be `production`; `any` and `reference` rows are errors here.

### 6.6 Eligibility

| Kind | `model` profile | `evidence` profile | `image` profile |
|---|---|---|---|
| `production` | yes | yes | yes |
| `reference` (deterministic, no `bound` rows) | yes | yes | no |
| `any` | yes | yes | no (SH16) |

Further checks, each a named negative control (section 12):

1. `needs` must be a subset of the implementation's `carries` (an implementation
   that does not carry `policy` cannot be required to).
2. A theorem with `property=hyper` needs every cited interface realised by a
   `production` implementation that carries `policy` (VOS-D007).
3. An implementation's `bound` rows are at or under the contract's ceilings
   (section 8, resource).
4. The board's `isa` equals the target ISA, and the `class` rows required by
   section 6.2 are present.
5. Every obligation the profile needs for an implementation satisfies R4.
6. A `config` value equals the implementation's pinned `cfg_<name>` example literal
   and a `const` assert in the profile crate.

### 6.7 What a swap changes

The swap of VOS01 section 5.8 is, at the profile level: one `impl` line in
`profile.vos`; a regenerated lock; a regenerated profile `.warrant` whose
`assume platform:` rows follow the manifest (R7); and the per-image artifacts VOS01
lists. The spine, the other implementations, the contract and their documents are
byte-identical, which `check-doc` proves by digest.

### 6.8 Parser bounds and path containment

`vos-manifest` is class P and reads files a contributor can edit, so every parser is
total, allocation-bounded and fail closed (a fuzz target per parser, VER06). Caps,
checked **before** allocation:

| Item | Cap |
|---|---|
| `profile.vos`, `contract.vos` | 64 KiB each; lines at most 1,024 bytes |
| `profile.lock` | 1 MiB; lines at most 4,096 bytes |
| each `.part` and `impl.part` | 1 MiB; lines at most 4,096 bytes |
| `vos.toml` | 4 KiB; at most 16 keys |
| rows | `iface`, `impl` at most 64 each; `slot` 16; `config` keys 64; `theorem` 256; `build` 2; `obl` 4,096; `pin` keys per row 32; `tier` 4; `ceiling` 1,024 |
| `DEC` | digits only, no leading zero except `0`, at most 20 digits and at most `u64::MAX`; anything else is an error, never a saturation |
| names | `IDENT` at most 32 bytes (grammar); `KEY` at most 32; `VALUE` at most 256 |

A duplicate is an error: a repeated `config` key, `pin` key, `iface`, `impl`, `slot`,
`theorem`, `obl`, `tier` or `ceiling`, and any row out of its fixed order.

**Paths.** Every path the tools read or write (`spec` and `output` in the bindings,
`.part` files, the board file, `vos.toml`, the closed-world crate directories) is
**canonicalized** and must be a descendant of the repository root, with no symlink at
any component (the rule `ensure_no_symlinks` already applies in `gen_evidence`).
`..` in a bindings `spec` or `output` path is rejected before canonicalization.
`gen-doc` writes only `code/specs/<impl>.warrant`, where `<impl>` is the validated
package name of a package whose directory the manifest names; it never writes
elsewhere, and a pre-existing symlink at that path fails.

---

## 7. SH16: the requirement, and the static rules VOS02 owns

### 7.1 The requirement

**SH16 (normative).** An `image`-class build contains no `-any` crate, no `ghost` or
other evidence feature, and no ghost code. This document states the requirement and the
checks that are cheap and sound **now**, on source and manifests: kind keyed on
evidence (R6), the lexical rules, the file bans of section 7.2, the central
evidence-feature list and the lock authority. **How an image is built so that the
requirement can be checked on what was actually compiled** (a hermetic, recorded
build) is **not** specified here: it is VOS22 (`VOS22-hermetic-image-build.md`, not yet written), which
**must exist before any image is built** (backlog VOS-036). Until it does, no VOS
profile may be `class image`, and `check-profile` rejects the class. The appendix
records every build-gate finding of the security review as the input to VOS22; none is
dropped by the split.

### 7.2 Static rules (no code executed)

1. **Closed world, by manifests.** Every dependency of an image profile's package is a
   manifest row, `core` or `compiler_builtins`; every source is an in-tree path (no
   registry, git, `[patch]`, `[replace]`, `package =`; VER29 section 5.1.1).
2. **No build scripts, no proc macros, by file as well as key.** In every closed-world
   crate: no `build =` key of **any** value (`false` included), no `build.rs` file at
   the package root (Cargo auto-detects it), no custom-build target, and no
   `proc-macro = true`; unless the crate is a manifest row of a declared kind that pins
   the script by SHA-256 (none today).
3. **Kinds.** Each node's kind is `contract`, `production`, `spine`, `hal` or `profile`;
   its `vos.toml` kind equals the manifest's and agrees with the evidence (a `production`
   crate's **parsed** claims match the `canonical` obligations by `claim_sha256`; no node
   carries an `any` block, and no `any` node carries `canonical`; R6 states what `vos-link`
   checks, as rules over the declared kind). The `*-any` name is a lint only (R6).
4. **Features, as resolved by the manifests.** Every non-default feature of every node is
   off, and no node enables an evidence feature (section 7.3). This is a static reading of
   `Cargo.toml`; it is **not** proof of what was compiled (VOS22 closes that).
5. **Lexical.** SH17, SH18 and its `cfg` containment (section 4.6) over every class K
   crate, and the cross-check of section 7.3.

### 7.3 Evidence features are policy, not self-declaration

`vos.toml` (a dedicated file in each crate directory, 4 KiB cap, parsed by
`vos-manifest`; `Cargo.toml` carries no VOS keys) declares `kind` and
`evidence-features`, but the authority is a **central list in `vos-link`** (initially
`ghost`, `cov`). A declared name outside the list is an error. Any feature on a class K
crate outside the central image allowlist (initially empty) fails for an image. Lexical
cross-check: every `cfg(feature = ...)` or `cfg_attr` in a crate names a feature in its
`[features]`, and a feature that gates code is either in `evidence-features` (SH18's
gating shape applies) or absent from the class K crate; a feature that gates code and is
declared nowhere is a finding.

### 7.4 Lock authority

A lock the tool regenerates and CI then trusts blesses tampering, so:

1. CI runs `check-profile` **read-only** and fails on any diff from the recomputed lock.
   The image feature policy is hard-coded in `vos-link`; the lock records findings,
   never expectations.
2. **CODEOWNERS** guards the authority paths: `profile.vos`, `profile.lock`,
   `contract.vos`, `**/*.part`, each crate's `vos.toml`, every `evidence.bindings`,
   `.cargo/**`, `rust-toolchain*`, root `Cargo.toml` and `Cargo.lock`,
   `.github/workflows/**`, `CODEOWNERS` itself, and the sources of `vos-link`,
   `vos-manifest` and `vos-shape-check`. A commit trailer was rejected: it is written by
   the change's author and authorises nothing (VER29 section 5.3.1 is a visibility
   device).
3. `vos-link check-owners` evaluates, per path, the **effective owner by last-match-wins**
   and fails if any authority path has none; it prints `git diff base...HEAD --name-only`
   filtered to those paths.
4. **External prerequisite, recorded here:** branch protection with "Require review from
   Code Owners" on the default branch. `check-owners` verifies it through
   `gh api repos/{owner}/{repo}/branches/{branch}/protection` where the token scope
   allows; an `unverified` result **fails closed for a release** and is only a warning
   elsewhere. Limit, stated: with a single maintainer or admin bypass the gate protects
   nothing the maintainer does not choose to be protected from, and the report says so.
5. The report prints the lock digest and the base-branch lock digest.

### 7.5 Controls VOS02 owns, and the pointer

The lock rows VOS02 itself needs are in section 6.3. Rows for the image build (record,
image and toolchain digests) are **defined by VOS22**. The negative controls of the
build mechanism (environment, argv, caching, record spoofing, toolchain and target
checks, the image scan, repo `.cargo`, resolver 1) are listed in the appendix for VOS22.

| # | Setup | Expected |
|---|---|---|
| S1 | `ghost` enabled through a third crate | rule 4 fails |
| S2 | an `-any` crate renamed, declares `kind = "any"` | rule 3 fails by kind |
| S3 | an `-any` relabelled `production` in `profile.vos` | rule 3 fails (parsed claims) |
| S4 | `ghost` in a crate's `default` features | rule 4 fails |
| S5 | `[patch]` or `package =` in the graph | rule 1 fails |
| S6 | `ghost` only through a dev-dependency | **passes** (not in the normal or build graph) |
| S7 | a `build.rs` file with no `build =` key; `build = false` beside a `build.rs`; a proc-macro crate | rule 2 fails |
| S8 | `profile.lock` regenerated by the author to allow ghost | the hard-coded policy fails it; `check-owners` flags the paths |
| S9 | `class image` in `profile.vos` before VOS22 exists | `check-profile` rejects the class |

---
## 8. The four id kinds: what works now, what waits

VOS00 section 4.7 and VOS01 section 2.10 require the decision to cover all four
kinds of VER29 section 3.1. VER29 publishes only the functional kind in slice 2
and nothing in the other three. The table says, for each, what a validator that
reads files can check **now** and what the `contract.<impl>.<...>` id would add
**later**. The lock uses VER29 section 5.3's pin names, so nothing migrates.

| Kind | Now | Later (id shape of VER29 section 3.1) |
|---|---|---|
| functional | Obligation digests (R4) plus the implementation's VER04h manifest and `ca-assurance-report --check`. The report prints VER04h domain kinds and LP-012 kinds, not VER29 rungs: the H3 classifier does not exist, and a rung printed without it would round a sample up (VER29 section 6.2) | `contract.<impl>.<obligation>` rows in the profile's `.warrant` (LV-008 and slice 2) |
| resource | The implementation's `bound` row is at or under the contract ceiling for the same operation and metric, compared mechanically. The evidence is an emulator count (VOS01 section 2.5), capped at `example` (VER29 section 7.1, VOS-D004) | `contract.<impl>.resource.<op>.<metric>` |
| capability | Read `effect_certificate.json` and the capability audit; require the empty set. Permanent until VQ5 settles how a capability strength is classified | `contract.<impl>.capability.<category>` |
| timing | `timing: none claimed`. A profile that requires a timing tier fails closed (VOS-031) | `contract.<impl>.timing.<op>` |

### 8.1 A new finding: contract-less dependencies withhold three kinds

VER29 section 7.2.1 says a timing, resource or capability strength is **withheld**
for any used dependency with no contract of that kind, and its text names `std` and
external crates; `core` is such a dependency too. Every class K crate depends on
`core` (SH1). So, once the reach pass attributes uses, no VOS component can print a
resource, timing or capability strength until a VER13 catalogue entry covers
`core`. That is a request to the VER backlog (section 9.3, row AS-008), not a
VOS rule: the VOS checks above (empty effect set, ceilings) hold without it, but
they are VOS-owned checks, not VER29 strengths.

---

## 9. The VER29 amendment and the VER requests

### 9.1 The amendment (normative text)

VER29 is amended in two places only: its reach rules (section 8) and its feature rule
(section 5.1.1 and VQ14). It lands as VER04m and VER04n did: a dated pointer
paragraph at the top of VER29 (written in the same PR as this document), the
normative text **here**, and the code applied by a new **slice 7, "interface reach"**,
in VER29 section 13, after slice 4 (the reach-around check; slice 6 is VER04n's).
The backlog row is **LV-008** (LV-007 is the publish gate), with a pointer sentence
on LV-005. The amendment changes no id shape, adds no rung and does not touch
sections 3, 5.1(a)-(e), 6, 9 or 9.1.

**AM1 (VER29 section 8.2).** In the table of what the pass can and cannot reach,
add to the "Reachable" column: a path `<P::A as T>::f` or
`<<P as Q>::A as T>::f`, where `T` resolves by **defining path** (the rule of
section 8.2: through the crate's module tree and `pub use`) to a trait declared in a
crate that is a **direct dependency** of the analysed package, is an **interface
use** `(crate, T, f)`. Item identity is the trait item `T::f`. It is not
classified `opaque` and not `unresolved`. Any other generic, trait-object or method
call into a lower type is unchanged.

**AM2 (VER29 section 8.3).** Add the class `interface-use`. In a package that
contains exactly one `impl <ProfileTrait> for <ProfileType>` item per profile type
(the profile package), each `type A = <path>;` item of that impl composes the
interface use `(crate, T, f)` of every use whose bound `A: T` is declared by the
`ProfileTrait`, to the implementation item `<path>::f`. The pass resolves the
implementation item by a new binding key `realises=<crate>::<T>::<f>` on an `op`
line of the implementation's `evidence.bindings` (section 4.7), then classifies it
as usual (`covered`, `uncontracted-op`, `unbound-item`). Interface uses found in the
generic source of the profile package's direct dependencies that are parameterised
by `ProfileTrait` (the spine) are attributed to the profile package's report: the
spine is monomorphised at the profile, so its uses are the profile's uses. A use
with no `type A` in the block, or an implementation item with no `realises=` key,
is `uncontracted-op`, never silently covered.

**AM3 (VER29 section 8.5 and VQ13).** A use of the exact shape of AM1 that AM2 has
resolved counts as **resolved**: it contributes nothing to the `opaque` count and
does not force `call-level-partial`. Every other generic still counts as `opaque`.
VQ13's default ("never partial to full") is unchanged for them.

**AM4 (VER29 section 5.1.1 and VQ14).** The evidence manifests record the **features
and `cfg` list** of the evidence build (today the manifest has neither; the spike
checked). A feature is **evidence-only** only if it is in the **central
list** kept by the VOS validator (`ghost`, `cov`) **and** the lower package declares
it in `vos.toml` `evidence-features`; a self-declared name outside the
central list earns no exemption (VOS02 section 7.3). An
evidence-only feature that gates exactly one `mod` item, plus `pub use` items that
re-export from that module and are gated by the same feature, is **not**
`build-config-unpinned`: the link reads the profile lock's resolved features for the
**image** build instead (VOS22), which must not contain it. Any other feature
difference is `build-config-unpinned` as today. Without this clause every
ghost-built implementation's link would be open. (The "plus `pub use`" is a spike
correction: the bindings reader needs the wrappers at the crate root, section 4.3.)

**AM5 (negative controls for slice 7).** Each must fail with the named result:

1. two `impl Profile for T` blocks for one profile type: a new error
   `InterfaceAmbiguous` (slice 7) naming both blocks;
2. an implementation item with no `realises=` key: the use is `uncontracted-op`;
3. a wrapper that does not forward (SH17 violated: it computes instead of calling
   the production trait): the realising item's `claim_sha256` still matches, so the
   pass must read the `realises=` key from the binding and the shape check must
   fail the wrapper; neither alone is enough (the pair of controls is the test);
4. a `type A = <path>` into a crate that is not a manifest row: not a VER29 result;
   `check-profile` and the closed-world rule of section 7.2 reject it;
5. a use `<P::A as T>::f` where `T` resolves to a trait of a crate that is **not** a
   direct dependency (AM1's condition): `opaque`, as before.

**Not requested.** Any id shape; a VQ12 domain check; any change to section 9.1; a
second pass in the analyzer (the pass is the VER29 slice 4 mode extended, in
`ca-capability-analyzer`'s `--emit-uses`, and this document adds no mode to that
crate: a change there selects every link check, VER29 section 11.2).

### 9.2 Slice 7 in VER29 section 13

| Slice | Scope | Would-build (estimate) |
|---|---|---|
| 7. Interface reach | `ca-capability-analyzer --emit-uses`: AM1 (interface use) and AM2 (`type A` composition, `realises=` key); `ca-assurance-report`: AM3, AM4, the manifest `features` and `cfg` fields and `InterfaceAmbiguous`; the AM5 controls; the `realises=` key in the bindings reader | about 6: the analyzer, `ca-assurance-report`, `warrant-grammar` (bindings), and the adopters whose manifests regenerate; re-measure with the build tool's plan |

### 9.3 Requests to the VER backlog

Two rows are filed in the same PR (VER-BACKLOG: LV-008 and WX-SV; the catalogue
request is AS-008).

**WX-SV, by-value explicit state.** A carve-out of WX-S1 that must **not** depend
on WX-G. WX-S1 is queued behind WX-G, WX-D2, WX-F and LP-012; WX-SV needs only the
by-value step of VOS01 section 1.1, which has no `inout`.

| # | Item | Priority | Why VOS needs it |
|---|---|---|---|
| 1 | The E11 `codec` line (`build=fn:...`, `observe=...`) for **by-value argument and result positions**, with E15's two codec obligations (`codec-roundtrip`, `rebuild-equivalence`) required for the H3 rung once it exists | P2 | removes the ghost face's wrappers; the state may stay private |
| 2 | VER04j section 4.8 faults applied **inside tuple components** (`wrong-ctor` of component 0 of `(Upd, Impl)`, `field-zero` and friends on a tuple's records), plus `drop-state-update` (the by-value twin of `stale-post`), plus a fault that changes a record's `Bool`/enum/integer field when the initial value is already zero | P1 | the negative controls of refinement and of `-any` coverage; without them no by-value step has any control (section 13.3) |
| 3 | VER04m E8 `closure:` and `init:` lines for `state=none` steps, and the `lemma` kind (E18, V9) | P2 | covers VOS-D005: VOS00's X1 needs closure and init evidenced for by-value components |
| 4 | A manifest `config` object, fed by a `config=<NAME>` key on nullary ops and printed in labels | P2 | VOS00 section 9.3's printing rule; removes the `CfgN` carrier of section 4.4 |

**Probe (ii) answered.** The brief hoped `wrong-ctor` would reach tuple component 0
and make `const-variant:<Ctor>` unnecessary. It does not: VER04j section 4.8 and
item 23 apply faults to a top-level result or inside `Ok`/`Some` only, and the
generator rejects every fault on `(Upd, Impl)` (section 13.3). Item 2 is therefore
wider than first stated, and `const-variant:<Ctor>` stays on the table until a
component-level `wrong-ctor` exists (it is not requested separately).

**AS-008, a VER13 catalogue entry for `core`.** Section 8.1: so resource, timing
and capability strengths are not withheld for class K (VER29 section 7.2.1).

---

## 10. Tools and where they live

| Tool | Class | Home | What it does |
|---|---|---|---|
| `vos-manifest` | P (parser) | `code/packages/rust/vos-manifest` | parsers and canonical emitters for `contract.vos`, `profile.vos`, `profile.lock`; total, bounded, fail closed, a fuzz target per parser (VER06, as VOS-D001 requires of EMU00); `alloc` allowed (VOS01 section 1.6) |
| `vos-link` | H (host) | `code/programs/rust/vos-link` | `gen-doc`, `check-doc`, `check-profile` (`--write-lock`), `check-owners`; reads files; the image build command and its gate are VOS22 |
| `vos-shape-check` | tool (class H) | `code/packages/rust/vos-shape-check` | SH1-SH15, SH17, SH18, class tags; uses the VER08a lexer as a **library**; reads each crate's class from its README (VOS01 section 1.6) |

**Do not add modes to `ca-capability-analyzer`.** A change to that crate selects
every link check in CI (VER29 section 11.2), and SH rules are VOS policy, not
analyzer behaviour. `vos-shape-check` depends on the VER08a lexer crate as a
library, so VOS changes select only VOS crates.

`gen-doc` and `check-doc` are backlog row **VOS-033**, ahead of VOS-011: they are
small, they have a prototype (section 13.1), and they make R2 true. `check-profile`
(static part: sections 7.2 and 7.4) and `check-owners` are **VOS-034**, with VOS-012 (the
first interface with two implementations); the image part of SH16 (VOS22) arrives
with VOS-026. The shape
rules SH17 and SH18 and the audits of section 5 are VOS-D002 and VOS-D024. The trusted-base cost is that
the conformance authority is `vos-link`: its negative controls (section 12) are
its evidence, and its digest comparison is the whole of R4.

Commands, as the CI invocation will read them:

```text
vos-link gen-doc     code/packages/rust/vos-ready-bitmap            # writes code/specs/vos-ready-bitmap.warrant
vos-link check-doc   code/packages/rust/vos-ready-bitmap            # exit 1 on any drift
vos-link check-profile profiles/rtos-min/profile.vos [--write-lock]
vos-link check-owners                                                # CODEOWNERS covers the authority files (section 7.4)
```

`vos-link` spawns any subprocess from an **argv array, never a shell**. The image build
command (`vos-link build`, the recorder, the hermetic tree) and `sh16`'s build-time
checks are specified by VOS22. Every path it touches is contained as in section 6.8.

### 10.1 `gen-doc` and `check-doc`, as built (VOS-033)

Section 2 left these points open; the decisions, with their reasons, are below. They are
the contract of `vos-manifest` (the pure part) and `vos-link` (the file part), and every
one has a test.

**D-GD1. What is copied, and what is not.** The document holds blocks 1 and 2 of R1 and
the implementation block, nothing else (R2 as amended). `warrant/lemmas.part` (R1.4) is
admitted when present (a forged marker in it is a `GD01`) and never copied. `contract.vos` is parsed (it names
the tiers and the instance), never copied.  Its digest will be pinned by the lock (VOS-034);
until then `check-doc` binds only its tier list (each `tier` row against the
`obligations-*.part` files, both ways), and the ceilings and `config N` are compared with
nothing (VOS-D055).

**D-GD2. Which crate names which contract, and which tiers are embedded.** The crate's
`vos.toml` has two more keys, both required for kind `production`, `reference` and `any`:
`contract = "<package>"` (an `IDENT` naming `code/packages/rust/<package>`, whose own
`vos.toml` must say `kind = "contract"`) and `obligations = [..]` (the tiers `gen-doc`
embeds, in sorted order, each one a `tier` row of the contract's `contract.vos`). The
authority over what a document *carries* stays R6 and section 2.3: `carries` is parsed
from the claims, so a wrong `obligations` list produces a finding, never a silent pass.
`vos-shape-check` accepts both keys (its closed list grows by two).

There is **no `mode` key** (VOS-D061 resolved otherwise): a production crate's mode is the tier it
carries, derived by `check-doc` from the parsed claims (R6, section 2.7). A `representation`
crate lists the tiers it carries (`["representation", "safety"]` for the round-robin scheduler); the
tier set grows to five, so `obligations` takes up to five names and `vos-shape-check`'s bound grows
with it.

**D-GD3. Where files live.** The implementation directory must be exactly
`<root>/code/packages/rust/<impl>` and the contract directory exactly
`<root>/code/packages/rust/<contract>`; the document is exactly
`<root>/code/specs/<impl>.warrant`. `<root>` is derived from the implementation directory
(four levels up) or given with `--root`, which must then agree. `<impl>` and `<contract>`
are `IDENT`s; in the document they appear with `-` replaced by `_` (the header
`warrant vos_ready_bitmap v1`, the marker `contract-package=vos_ready_contract`), as the
listing of section 2.4 shows. Reads are bounded and never follow a symlink; the write goes
through a fresh file (`create_new`) in `code/specs` and a rename, and a symlink or a
non-regular file already at the target fails.

**D-GD4. The document trailer.** After the last obligations block `gen-doc` writes the
two rows `assume platform:` / `none` and `assume environment:` / `none` (R7: an
implementation assumes nothing). An implementation block may declare no `assume` row;
`check-doc` requires exactly one row of each, after the last marker, both `none`.

**D-GD5. The markers are a grammar, not a scanner.** The marker skeleton of section 2.3 is
`code/grammars/vos-document/` (a token grammar whose line tokens are the seven exact
markers, a catch-all token for any other line containing `-- vos02:`, newline and plain
line, and a parser grammar that fixes the order, so each marker appears once and in its
place). The tier order and uniqueness, which a grammar cannot express, are a post-parse
check. A part is lexed with the same token grammar and admitted only if it lexes to plain
lines: so a forged marker, an exact one or a near miss, **anywhere in a line**, including
inside a string or a trailing comment, is refused in a part and, in a document, fails
`check-doc`. `contract.vos` is `code/grammars/vos-contract/`, read the same way (D-GD6).

**D-GD6. `contract.vos`.** Tokens are `NAME` (`[A-Za-z][A-Za-z0-9_-]*`), `DEC`
(`0` or a digit run with no leading zero, at most 20 digits; the value must also fit
`u64`, which a grammar cannot count and a post-token check does) and `NEWLINE`; a space is
skipped by the lexer, a tab, a CR or any other byte is a lexical error. The word classes
(`PKG`/`IDENT` hyphen-only, `OPNAME` underscore-only, `KEY` letters, digits and underscore,
each at most 32 bytes) are a post-token check on the value, because one lexical pattern
cannot know which row it is in. Canonical form is the section 6.1 discipline: the
grammar accepts some non-canonical spellings (a doubled space, a blank line), so the **re-emitted** text must equal the input
byte for byte, which rejects them all, and sorted order and duplicates are explicit
errors first, so the message names the row. The digest of a contract is the SHA-256 of
those canonical bytes.

**D-GD7. `claim_sha256`.** VER29 section 5.3 defines it, and VER29 slice 2 is not
implemented, so `vos-manifest` computes it with the same definition until the VER29
function exists (VOS-D056). The canonicalisation, exactly: take the tokens of the claim
declaration (those inside its source span) from the Warrant lexer and layout pass of
`warrant-grammar`, comments already absent; drop the layout tokens `Indent`, `Dedent` and
`Newline`; write each remaining token as its **raw source slice** (so a literal keeps its
prefix and quotes: the lexer's own token text drops the quotes of some literals, and `b"00"`
must not hash like `00`); join with single spaces; SHA-256, 64 lowercase hex. Re-wrapping a
claim or editing a comment changes no digest; renaming a token or changing a literal does.
For words, symbols and numbers the slice is the token text, so this is VER29 5.3's
definition. All fifteen digests of VOS10 are pinned in `vos-manifest/tests/warrant.rs`,
produced by an independent Python implementation (`tests/claim_sha256_reference.py`).

**D-GD8. What `check-doc` checks, in order, and what each finding is.** The inputs have their own codes: `GD01` a part is not admitted or `impl.part` is missing, `GD02` `contract.vos` is invalid or names another package, `GD03` the `obligations-*.part` files and the `tier` rows disagree (either way), `GD04` an obligations part holds more than claims or a name repeats across tiers, `GD05` a `vos.toml` problem. The document checks all run on the
document on disk; every finding is reported, none stops the others except where a later
check needs an earlier result.

| Code | Check |
|---|---|
| CD01 | the document file exists and is a regular file; its marker skeleton parses (order, once each, no forged marker, tiers sorted and unique) |
| CD02 | each marker digest equals the SHA-256 of the lines between its markers |
| CD03 | the contract block and each obligations block equal the contract package's `.part` file for that tier, byte for byte |
| CD04 | the document parses and type-checks with `warrant-grammar`; its trailer is D-GD4's |
| CD05 | no claim in the document carries the name of a contract obligation with a different `claim_sha256` (NC2) |
| CD06 | `carries` is computed from the parsed claims; a tier is carried when every obligation of the tier appears with the contract's `claim_sha256`, and a **partly** carried tier is a finding; the carried set must equal the `obligations` of `vos.toml` |
| CD07 | the R6 rules that the document alone decides (below) |
| CD08 | regeneration: the document equals, byte for byte, what `gen-doc` would write now |

R6, as far as the document goes. `kind = production`: the document carries every obligation of
**exactly one of** the `canonical` and `representation` tiers (by `claim_sha256`: both, or
neither, is a finding) and no `any` obligation, the crate name does not end in `-any`, and, for a
`canonical` carrier only, **every field of `Impl` is in the cone of `build`**: the `build` model must be an `Impl(..)`
constructor application, and every argument must *depend on* the model's parameter, by a
taint walk: a mention does not count when it is an operand of `and`, `bit_and` or `*` whose
other operand is a literal `false` or zero, both sides of `x - x` or `bit_xor(x, x)`, or an
argument to a `model` whose body does not depend on that parameter (a field set from a
constant is an oracle or a register, not a function of M). `kind = any`: the document
carries every `any` obligation, **no** `canonical` and **no** `representation` claim (by `claim_sha256`), the crate name
ends in `-any`, and **at least one** `Impl` field is outside the cone of `build` (the oracle
the `any` block names). `kind = reference`: the document has no `bound` row. A `build` that is
not an `Impl(..)` application cannot be decided structurally and fails closed for
`production` and `any`.

A `representation` carrier (derived from the parsed claims; CD06 and CD08 are unchanged) skips the
cone walk and is checked for the rest by CD04 alone: the block references `rep` and `hist`, so a
document without them, or with the wrong types, is refused by the type check, and a tampered or
weakened tier claim is a digest mismatch (CD03/CD05/CD06). There is no claim-shape walk (the CD07-R of
the first draft is deleted, section 2.7), and `vos-link` prints `mode representation` in its header and
a note in place of the R6 cone note for a representation carrier (the cone is not checked there: the
state has history by design; the note says what a pass does and does not establish). The cone note stays
for canonical and `any` carriers.

**What a pass of the cone rule is, stated plainly.** It is a **structural lint** at document
level, not a proof that the state has no oracle. A syntactic walk is still evaded by a
constant produced by arithmetic on literals (`shl(1u8, 8)`), a constant condition
(`if true then 0u8 else m.p0`), a dependence hidden behind a bound operation, a projection of an
untainted component out of a tuple that has a tainted one (projection is field-insensitive),
and `x - x` written two ways. Known evasions, each confirmed by review: `m.p0 * (1u8 - 1u8)`,
`if false then m.p0 else 0u8` and `if true then 0u8 else m.p0`, `bit_and(x, shl(0u8, 1))`,
`bit_and(x, bit_xor(1u8, 1u8))`, a model `zz(x) = if x then 0u8 else 0u8` (both arms equal),
and a `match` whose arms are all constants. The walk is a lint: it computes each model's
dependence once (a memo per model, a conservative answer for a model met while it is being
summarised) and has a step budget of 1,000,000; exhausting it is a CD07 failure for
`production` and `any` (fails closed); `vos-link` prints that statement with every pass. The argument that this
is acceptable: an implementation whose `canonical`, `policy` and `safety` evidence holds
behaves correctly whatever its fields are called. The real hazard is an oracle **write**
path (a ghost export or binding that lets the harness set the field); that is closed by the
bindings half of R6 (VOS-D060, a **hard prerequisite of any `image` profile**), SH13 and
SH16, the central kind registry (VOS-D030) and `check-profile` (VOS-034), not by this lint.
`kind`, `contract` and `obligations` are **author-declared** in `vos.toml` until VOS-D030 and
VOS-034 compare them with a registry and a manifest row: `check-doc` does not catch a
production crate that lists only `["canonical"]` or an oracle crate that declares
`reference`. The gate is therefore described as *enforced at document level*, never as
unbypassable.

**What a pass of the representation tier is.** That the document carries the contract's closure
claims by digest and type-checks against the author's `rep` and `hist`; the evidence runs them. It
does not say that `-any` could not also carry it (it could, truthfully), so it is not a kind
separator: that stays with the declared kind, the `any` block, SH13, SH16, VOS-D030 and VOS-D060.

Not decided here, and so not claimed: that `alpha` reads every field
(the ghost wrapper, SH17), that `evidence.bindings` binds no oracle write, the registry
cross-check of kind and class (VOS-D030), the manifest row and the `image` rule (VOS-034).

**D-GD9. Exit codes** are `vos-shape-check`'s: 0 no findings, 1 findings, 2 no verdict.
No verdict means a rule could not even be applied: an unreadable path, a path outside the
repository or through a symlink, a file over its cap, a usage error. A rule that fails on
bytes that were admitted (a forged marker, a malformed contract, a drifted document) is a
finding. `gen-doc` runs every check but CD08 on the document it would write, and writes
nothing if any finding exists.

**D-GD10. What `vos-sched-contract-check` keeps.** Its string-joining assembler is replaced
by the tool's (`vos_manifest::document::assemble`), and its marker-discipline stand-in by
the tool's part admission. The crate stays: the ground evaluation of the lemmas and
obligations over Warrant models (about a minute) is a semantic suite, not a document check,
and the tool's own tests should stay fast.

---

## 11. Report lines

The profile package's report prints the following, derived from the lock and the
implementations' reports. Nothing is typed by hand (VOS00 section 9.3, rule 2).

**Instance constants (printing rule 1 of VOS00 section 9.3).**

```text
vos-ready-bitmap  kind production  mode canonical  N=8
  digests      contract <64 hex>  implementation-block <64 hex>  lock <64 hex>  base-lock <64 hex>
  refinement   exhaustive (full, max 65,536/claim, 328,960 cases)  stack host-rustc
  conformance  safety 3/3  policy 1/1  canonical 6/6  (obligations digest-equal)
  configuration  N=8  example n_is_eight  const-assert in vos-sched-profile
```

(The case total is the spike's, section 13.2; it prints from the manifest.)

**Rests on `-any`.**

```text
theorem ready-in-set  trace
  rests on vos-sleepq-any (kind any): soundness 1/1, coverage 2/2 (controls: none, WX-SV item 2)
  transferred, single-trace
```

**The derived transfer row (R8).** `transferred, single-trace` is printed, never
scored: it states that the theorem was evidenced over `-any` and holds for every
safety-tier implementation **for single-trace properties**. A `property=hyper`
theorem prints `transfer: none` and needs production implementations with the
policy tier (section 6.6).

**Per kind (section 8).**

```text
resource    insert <= 24 steps (ceiling 160)  example, emulator count  [capped, VOS-D004; VOS01 5.7 placeholder]
capability  empty effect set  effect_certificate current  (VOS-owned check, not a VER29 strength)
timing      none claimed
```

**Mode.** `mode=representation` is printed beside the kind on every line. `vos-link`'s own
report header prints `mode representation` after the kind of a crate whose document carries the
`representation` tier (derived from `carries`; a canonical crate prints no mode, as before).

**Digests.** The report prints the **implementation-block digest** (the SHA-256 of
the bytes between `impl-begin` and `impl-end`) beside the contract digest: the
contract block is shared and copied, but the implementation block is the one part a
swap authors, and it is where `build` and `alpha` live (section 12.3). It also prints
the lock digest and the base-branch lock digest (section 7.4). Every digest is the
full 64-hex value; shortened forms in this document are for reading only.

---
## 12. Conformance, negative controls and open questions

### 12.1 The checklist

A PR that adds or changes an implementation, a contract package or a profile
applies this list in addition to VOS01 section 7. An item that does not apply is
marked `n/a` with a reason.

1. The implementation's `code/specs/<impl>.warrant` is **generated**: `vos-link
   check-doc` is clean (R2).
2. The contract block digest in the marker equals the contract package's
   (`contract-sha256` in the lock), and so does every obligations digest.
3. For every tier the profile needs, every obligation has the contract's
   `claim_sha256` and passes VER29 sections 5.1(b)-(e) (R4).
4. The implementation block uses the fixed vocabulary of R3 (and, until WX-SV, the
   interim vocabulary of section 4.4) and declares no claim named like an
   obligation.
5. The kind is declared in the crate's `vos.toml` and passes the `vos-link` rules of R6:
   `production` carries exactly one of `canonical` and `representation` and no `any` block; `any` carries the `any` block, neither of the two, and has the oracle field.
6. The ghost face passes SH17 and SH18, is behind a feature listed in
   `evidence-features`, and the test target has `required-features`.
7. The profile is canonical text (parse then re-emit is byte-equal), the lock is
   current, and `check-profile` passes its six checks (section 6.6).
8. No profile is `class image` until VOS22 exists; the static rules of section 7.2
   pass; authority files changed only with CODEOWNERS review (section 7.4).
9. The report prints the instance constants, `rests on -any` and the transfer row
   (section 11). Any resource figure printed is marked `example`.
10. A contract change (any `.part` file) lists every implementation and profile it
    regenerates; a swap changes none of the files of section 6.7's "byte-identical" list.

### 12.2 Negative controls

Each must be rejected by the check named. A control that passes is a defect of
the gate (VER04h section 6).

| # | Control | Rejected by |
|---|---|---|
| NC1 | a hand edit inside the copied contract block | `check-doc` (marker digest and regeneration) |
| NC2 | an obligation weakened in the implementation block with the same name | `check-doc` (duplicate declaration) and `claim_sha256` mismatch |
| NC3 | an implementation that omits an obligation its profile needs | `check-profile` (`needs` is not a subset of `carries`) |
| NC4 | a `-any` crate declared `production` | R6: it carries the `any` block (and its `Impl` has the oracle field) |
| NC4b | an `-any` crate whose document also carries `canonical` | R6: the rule `any` may not carry `canonical` (it would pass the claims, VOS10 section 14) |
| NC5 | a `production` crate carrying an `any` block | R6 |
| NC6 | a stale lock after a contract change | `check-profile` (digest mismatch) |
| NC7 | an `any` implementation in an `image` profile | eligibility, section 7.2 rule 3 |
| NC8 | a `hyper` theorem over an `any` or non-policy interface | `check-profile` (VOS-D007) |
| NC9 | an implementation whose `bound` row exceeds the ceiling | `check-profile` (resource) |
| NC10 | a board whose `isa` differs from the target | `check-profile` |
| NC11 | every control S1-S9 of section 7.5 (unification, renamed or relabelled `-any`, `ghost` in `default`, `build.rs` file and proc macros, a regenerated lock, `class image` before VOS22; a dev-dependency-only ghost must pass) | SH16 static rules |
| NC12 | two `impl Profile` blocks; a missing `realises=`; a wrapper that does not forward | AM5 controls 1-3 |
| NC13 | an `Image` that lacks a field of `State`; a wrapper with `..` or extra statements | SH17 |
| NC14 | a `ghost::` path in non-ghost class K source | SH18 |
| NC15 | profile text with two spellings of one profile (reordered rows, different case) | `vos-manifest` (parse then re-emit) |
| NC16 | a `config` value that differs from the implementation's `cfg_<name>` example | `check-profile` |
| NC17 | a `.part` or `impl.part` containing a `-- vos02:` line or any control character other than LF; a duplicated, missing or reordered marker; a claim moved between blocks | `gen-doc` / `check-doc` (section 2.3); `carries` is parsed, not read from markers |
| NC18 | an oversized file, a duplicate key or row, a `DEC` over 20 digits or above `u64::MAX`, a `..` or symlink in a bindings path | `vos-manifest` and `vos-link` (section 6.8) |
| NC19 | an authority file (the list of section 7.4, including `vos.toml`, `.cargo/**`, workflows and the VOS tool sources) without an effective CODEOWNERS owner | `check-owners` (section 7.6) |
| NC20 | a self-declared `evidence-features` name outside the central list; a feature that gates code but is declared nowhere | section 7.3 |
| NC21 | a production crate whose document carries both `canonical` and `representation`, or neither; `representation` together with `any`; an `-any` crate carrying `representation` | `check-doc` (CD07) |
| NC22 | a representation crate whose `impl.part` lacks `rep` or `hist`; a tampered or weakened representation claim | `check-doc` (CD04 type check; CD03/CD06 digest) |

### 12.3 What equal tokens do not establish (the residue of R3)

Digest equality proves the obligation text is the contract's. It does not prove
that `build`, `alpha` or the interim carriers mean the same thing in each
implementation. Three things bound the residue: the `canonical` block (R5)
constrains `build` and `alpha` against each other; SH17 makes the ghost wrappers
non-computing; and `alpha` is reviewed (VOS01 Q2). A wrong `alpha` that is
consistent with a wrong `build` is a defect these checks do not see, which is
VOS01 section 7 item 7's `alpha_covers_m` review item, now with a digest to point
at.

**The trusted base, stated.** R4 and SH16 are only as good as the tools and the review
that guard them. The authority is `vos-link` (digest comparison, the sanctioned build,
the gate) together with `vos-manifest`, `vos-shape-check` and the **review gate on the
authority files** (CODEOWNERS, section 7.4, whose branch-protection prerequisite a sole maintainer or an admin can bypass): a contributor who can edit a `.part`, the
lock, `contract.vos` or `profile.vos` and merge without that review can move the
meaning of every obligation or the policy of the gate. Digest equality proves the
text is the contract's; it does not prove the contract is right, and it does not prove
the tool is. The negative controls of section 12.2 are the tools' evidence, and each
authority file is listed so a reviewer sees it move.

### 12.4 Open questions

Each has a recommended answer, which this document assumes until decided.

| # | Question | Recommendation |
|---|---|---|
| Q1 | Interim projection vocabulary (section 4.4) or wait for WX-SV item 2? | Interim now, because it produced real evidence and its replacement is mechanical; record the sunset in the backlog so the digest change is planned |
| Q2 | Should the `canonical` block use the commuting form for **all** steps (no `alpha` in refinement)? | Yes for canonical implementations: it attaches negative controls to the image operation (section 4.4) and is equivalent given the `alpha(build(m)) == m` claim |
| Q3 | Which package's report carries the reach for spine uses (AM2)? | The profile package's, because the spine is monomorphised there; confirm when slice 7 is designed |
| Q4 | Is `vos-link` the right home for SH16, or `vos-shape-check`? | `vos-link`: it needs `cargo tree` and the lock; the lexical half (SH18) is in `vos-shape-check` |
| Q5 | Should `reference` implementations exist in the first VOS-012 or wait? | Wait: no use until a monitor needs one (VOS-027) |
| Q6 | The `contract.vos` format (section 2.5): a new file or Warrant-side ceilings? | A new canonical-text file: ceilings are not Warrant (VER04 section 7 bounds are per document) and a compared constant needs one spelling |
| Q7 | Marker digests duplicate the lock's. Keep both? | Keep: the marker lets a reader see drift without the tool; the lock is the authority |

---

## 13. Acceptance spike

**Why.** VOS01 said "no evidence was generated". Every [probe] item of this
document depended on running the toy through today's generator, so the spike was
the acceptance test of the PR. It is **uncommitted**; its throwaway crates live in
the working scratch area of the session that wrote this document. The commands
below reproduce it from a checkout.

### 13.1 What was built

A scratch workspace laid out like the repository (`code/specs/`,
`code/packages/rust/`), because the generator reads
`code/packages/rust/<pkg>/evidence.bindings` and `code/specs/<pkg>.warrant`
relative to its repository root:

* `vos_ready_contract`: the VOS01 section 1.2 trait (`ReadySet`, `Upd`, `Pick`,
  `Profile`, `Kernel<P>`), `#![no_std]`.
* `vos_ready_bitmap`: a private-field `Bitmap`, `BitmapReady`, the section 5.3
  `clz8` and `pick`, a `ghost` module (feature `ghost`) with the image, the
  wrappers of section 4.4 and a root `pub use`; the bindings of section 4.5; the
  evidence test target with `required-features = ["ghost"]`.
* `driver`: a verbatim copy of `warrant-grammar`'s `gen_evidence.rs` example,
  placed so its `CARGO_MANIFEST_DIR.ancestors().nth(4)` lands on the scratch root,
  with path dependencies on the repository's `warrant-grammar` and `sha256` crates.
* A prototype `assemble.py` (about 40 lines): builds the document of section 2.4
  from `.part` files with marker digests and implements `--check`.
* A second workspace of five tiny crates for probe (iii).

The `mise` shim of this repository fails when `rustc` runs below the worktree, so
the spike used the real toolchain (`RUSTC=/opt/homebrew/bin/rustc`, Rust 1.94.0).

### 13.2 Commands and results

```text
python3 assemble.py && python3 assemble.py --check          # document built; drift check clean
cargo run -q -p driver                                      # gen_evidence: writes tests/generated/warrant_evidence.rs (2,766 lines)
cargo test -q -p vos_ready_bitmap --features ghost --test warrant_evidence      # 3 s
WARRANT_EVIDENCE_OUT=manifest.json  cargo test ...          # writes the evidence manifest
```

| Result | Value |
|---|---|
| Document type-checks (markers are comments, nullary ops need no `labels`) | yes |
| Generated harness compiles and runs | yes, about 3 s |
| Tests | 53 `ok`, 1 `not ok` (below) |
| Exhaustive claims | 10, all `pass`: `insert_safe`, `remove_safe`, `canon_insert_post`, `canon_insert_img`, `canon_remove_img` at 65,536 cases each; `pick_safe`, `pick_policy`, `canon_alpha_build`, `canon_build_alpha` (spike only, dropped from the obligations: it names `Impl`'s fields, R6), `canon_pick_img` at 256 |
| Exhaustive total | 328,960 cases (matches VOS01 section 5.6's 65,536-per-claim figure); every binder `Bool set(2)` or `U8 full(256)`, discarded 0 |
| Property, fuzz | 10 and 10, all `pass` |
| Examples | 3, all `pass` |
| Negative controls | 21 entries; 20 `pass`, 1 `fail` (`init` / `field-zero`: finding F4) |
| Drift control (`assemble.py --check` after a hand edit in the contract block) | exit 1, then 0 after regeneration |
| SH17 (a field added to `State`) | ghost build fails: 4 x E0027 and 6 x E0063; the non-ghost build still compiles |

### 13.3 The three probes

**(i) Can a trait associated function be named by a plain `export=` path, or are
wrappers required?** Wrappers are required, and they must be at the crate root.
`export=` must be a Rust *identifier* (a path is rejected: "export
`vos_ready_bitmap::ghost::insert_out` is not a Rust identifier"), and the harness
uses `use <crate>::*;` to resolve it. The brief's `export=...::ghost::insert`
therefore does not work today. Fix: the gated `pub use ghost::{...}` of section 4.3.

**(ii) Does `wrong-ctor` reach tuple component 0?** No. For the natural
`insert(s, p) -> (Upd, Impl)` the generator refuses generation (W0701) for every one
of the eight faults tried (`wrong-ctor`, `field-zero`, `drop-consumed`,
`swap-components`, `ok-as-err`, `off-by-one-consumed`, `missing-error`,
`flip-result-bit`): "fault `X` does not apply to the result type of the operation".
The applicability table (`composite_adapter` in `warrant-grammar/src/evidence.rs`)
tests the **whole result** or the inside of `Ok`/`Some`: a tuple takes
`drop-consumed` only if its second component is an integer, `swap-components` only
if it is homogeneous, `field-zero` only a single-constructor record with an integer
field, `wrong-ctor` only a variant of two or more constructors. And every `op` line
needs at least one fault (`needs faults= (at least one)`), so an operation with
no applicable fault cannot be bound at all. This is the strongest single argument
for WX-SV item 2.

**(iii) Does `cargo metadata` show `ghost` on through workspace unification while
`cargo tree -p X -e features` for the image invocation does not?** Yes, and the
combined invocation shows why SH16 must read the invocation.

```text
workspace: imp (feature ghost), evid -> imp[ghost], img -> imp, mid -> imp[ghost],
           img3 -> imp + mid, imgdev -> imp, dev-dependency mid
cargo metadata (whole workspace)                  imp features: ["ghost"]
cargo tree -p img  -e normal,build,features       imp feature "default" only
cargo tree -p evid -e normal,build,features       imp feature "ghost"
cargo tree -p img3 -e normal,build,features       imp feature "ghost"    (through mid)
cargo tree -p imgdev -e normal,build,features     no ghost               (dev-dependency only)
cargo build -p img -v                             imp compiled without feature="ghost"
cargo build -p img -p evid -v                     imp compiled WITH   feature="ghost"
cargo tree -p img3 -e normal,build --prefix none --format '{p} [{f}]'    prints "imp ... [ghost]"
```

The `--format '{p} [{f}]'` form lists each node's resolved features per package and
works for any `--target`; it is only a cross-check, while what was compiled is VOS22's question.

### 13.4 Findings that changed the brief

| # | Finding | Effect |
|---|---|---|
| F1 | `export=` is a bare identifier resolved through `use <crate>::*` (probe i) | sections 4.3, 9.1 AM4: the gated `pub use` at the crate root |
| F2 | No fault applies to a tuple result whose second component is not an integer (probe ii), nor to a record of `Bool`s, a nullary scalar, or a scalar with composite parameters | interim vocabulary (section 4.4); WX-SV item 2 widened (section 9.3) |
| F3 | A negative control attaches to a claim by its **subject operation**: the first operation found by shallow search. In `dec(alpha_w(insert_img(...))) == ...` the subject is `alpha_w`, so `insert_img` had no control that could pass (3 negctls `not ok` until the commuting claims were added) | R5's canonical block uses the commuting form (section 4.4) |
| F4 | `field-zero` on `init` is **inert** when the initial state's field is already 0: no claim can distinguish the faulted from the real value, so the `init` control fails (an example over the zero state had the same defect for `alpha_w` until it used a non-zero state) | WX-SV item 2 asks for a fault that changes a zero field; until then the `init` control is a known failure, recorded in the lock as `controls: init field-zero inert` |
| F5 | The evidence manifest has no `features`, `cfg` or `config` field | AM4 (features and `cfg`) and WX-SV item 4 (`config`) |
| F6 | The generated harness needs `coding_adventures_sha256` and `coding_adventures_test_bootstrap` as dev-dependencies | SH12 reads normal and build dependencies strictly and dev-dependencies against a closed allowlist (VOS03 SH12, VOS01 section 1.4), not "not at all"; the static closed-world rule of section 7.2 reads normal and build dependencies only (S6) |
| F7 | AM4 as briefed said "a feature that gates exactly one module item"; the root re-export is a second gated item | AM4 allows `pub use` items from that module (section 9.1) |
| F8 | Resolver 1 makes `cargo tree -e normal,build` report no `ghost` while the build compiles it (dev-dependency unification); `RUSTFLAGS` cfgs are invisible to `cargo tree`; `--unit-graph` is nightly-only | VOS22 (hermetic, recorded build); see the appendix |

Findings F1, F2 and F7 contradict the instructions this PR was written from (a
path-valued `export=`, a `wrong-ctor` that reaches tuple components, an AM4 that
gates one item); each is adjusted where it applies. To rerun, recreate the layout
of section 13.1 and use the commands above; the `.warrant` source is the document of
section 2.4 and the bindings are those of section 4.5.

---

## 14. What changes elsewhere in the same PR

The reversal of section 1.5 is applied, not left as a follow-up (the rule of
[the backlog](VOS-BACKLOG.md), "Discovered work").

| Document | Change |
|---|---|
| VOS00 sections 4.5, 4.7, 14.3 | section 4.7 rewritten to the decision of section 1.3 with a pointer here; the two options become a record of the reversal; section 4.5 and D1 point to VOS02 |
| VOS01 section 2.10, Q9, Q10 | the "document per implementation" design is now this document's R1 to R9; Q9's answer is "from the implementation's own claims, conforming by digest"; both point here |
| VOS01 sections 5.3-5.6 | the `lin_` and `any_` prefixes and per-implementation record names become the fixed vocabulary of R3, with a note that the listings are the natural form and today's generator needs the interim form of section 4.4 |
| VER29 | a dated pointer paragraph at the top announcing slice 7 and AM1-AM5; no other section edited |
| [VER backlog](VER-BACKLOG.md) | rows LV-008, WX-SV and AS-008; a pointer sentence on LV-005 |
| [VOS backlog](VOS-BACKLOG.md) | VOS-003 done, VOS-004 in review; D003, D006 and D009 resolved here with implementation rows; new rows VOS-033 and VOS-034 for `vos-link` and discovered work VOS-D021 to VOS-D025; D002 extended (SH17, SH18, class tags); the accepted ordering and the decision of this document in the decision log |

### Sources

[VOS00](VOS00-verified-os-program.md) sections 4.4-4.7, 8, 14.3;
[VOS01](VOS01-component-contract-model.md) sections 1.2, 1.4, 1.6, 2.10, 3, 5, 6.3;
[EMU00](EMU00-emulator-framework.md) section 5.1 and Q14; VER29 sections 3.1, 3.2,
5.1, 5.1.1, 5.3, 5.3.1, 7.1, 7.2.1, 8.2, 8.3, 9, 11.1, 13, 15, VQ12-VQ14; VER04j
sections 4.8 and 14.1 (items 22, 23); VER04m E8, E11, E15, E18, V9; VER04n OK2,
OK3, OB1, OF1, OR5 and section 18; and, in `code/packages/rust/warrant-grammar`,
`src/binding.rs` (`is_rust_identifier` on `export=`), `src/evidence.rs`
(`composite_adapter`, negative-control attachment) and `src/codegen.rs`
(`find_operation_in_expr`).

---

## Appendix A. Image-build findings carried forward to VOS22

The owner split the image-build mechanism out of this document after three rounds of
security review. This list is the durable brief for VOS22 (`VOS22-hermetic-image-build.md`, not yet written),
which must exist before any image is built (backlog VOS-036). Every build-gate finding is
here, with its essence and the proposed fix; the non-build findings were fixed in this
document.

**Spike facts** (scratchpad, Cargo 1.94 stable; section 13.3):
(a) under resolver 1, `cargo tree -e normal,build` shows no `ghost` while `cargo build -v`
compiles it (dev-dependency unification), so the tree can lie;
(b) a `RUSTFLAGS='--cfg feature="ghost"'` cfg is invisible to `cargo tree` and visible in
`cargo build -v`;
(c) `--unit-graph` is nightly-only on stable, so it cannot be the source of truth;
(d) `cargo build -p a -p b` unifies features across the selected packages.

**Round 1**
1. **Bypass routes** (HIGH): `RUSTFLAGS`, `CARGO_*RUSTFLAGS`, `.cargo/config` rustflags,
   `build.rs` `cargo:rustc-cfg`, proc macros, `RUSTC_WRAPPER`, config `[source]`/`[patch]`
   never appear in resolved features. Fix: do not deny routes one by one; remove ambient
   inputs (hermetic tree, env allowlist) and check what the compiler was asked.
2. **Gate versus shipped invocation** (confused deputy): the gate checked one build and
   release shipped another. Fix: one sanctioned build command; a CI argv goes through the
   same validator; bind argv, record and image digests.
3. **Resolver version**: resolver 1 hides dev-dependency unification. Fix: the generated
   build root fixes `resolver = "2"`; compare features per (package, host|target) unit.
7. **argv hygiene**: `TRIPLE` and package names reach a command line. Fix: argv array,
   never a shell; `--target=`/`--package=` forms; triple checked against the target list.
8. **`cargo tree` parsing**: dedupe markers and loose formats hide nodes. Fix: do not
   parse text; use a recorder, or at least `--no-dedupe` with a delimiter-safe format.

**Round 2**
- **R2-1** The verbose log is spoofable through newlines in `CARGO_PKG_*`, and crate-name
  matching is ambiguous. Fix: structured, length-prefixed records keyed by (manifest dir,
  crate name, `-C metadata`, host|target).
- **R2-2** A cached build prints `Fresh` and skips the log, so the check fails open. Fix:
  a fresh target directory; fail on any `Fresh` unit.
- **R2-3** The gate ran `build.rs` and proc macros before rejecting them. Fix: static phase
  first (manifest and file bans), build only if it passes.
- **R2-4** Cargo discovers `.cargo/config` from the cwd, not the package directory; config
  and argv pins were read back from the lock; the CI argv had no flag allowlist; machine
  paths ended up in the lock. Fix: build in a private tree with no `.cargo` above it, no
  config pins at all, a strict flag allowlist, relative paths in records.
- **R2-5** The env denylist was incomplete (`CARGO_BUILD_RUSTC*`, `CARGO_TARGET_<T>_LINKER`
  and `_RUNNER`, `CARGO_PROFILE_*`, `CARGO_UNSTABLE_*`, `RUSTC_BOOTSTRAP`, `RUSTUP_TOOLCHAIN`,
  `RUST_TARGET_PATH`); `.json` target specs; toolchain not pinned by binary; root
  `Cargo.toml`, `Cargo.lock` and `[profile]` not pinned. Fix: an env **allowlist**, refuse
  to run with stray `CARGO_*`/`RUST*`/`CC*`; reject `.json` targets; absolute toolchain
  paths with a pinned digest; generate the root manifest and profiles from tool policy.

**Round 3**
- **M1** Phase-1 `cargo metadata`/`tree` on the repo tree makes Cargo probe `rustc` through
  the repo's `.cargo/config` (`rustc`, `rustc-wrapper`) and `rust-toolchain.toml`. Fix: run
  even the static graph queries in a pristine copy, or read the manifests with the tool's
  own parser.
- **M2** The pinned `rustc` may not be the one Cargo runs (`RUSTC` unset, `PATH` undefined);
  the sysroot and `rust-lld` are unpinned; the recorder's handling of probe invocations
  (`-vV`, `--print`) is undefined. Fix: set `RUSTC` to the pinned absolute path, pin the
  whole sysroot by digest, and define recorder behaviour for probes (pass through, do not
  count as units).
- **M3** `build.rs` is auto-detected without a `build =` key. Fix (already a static rule,
  section 7.2): ban the file, the custom-build target and `build = false`; VOS22 repeats the
  check on the pristine copy.
- **M4** The copy step is unspecified (symlinks, special files, `git archive` versus the
  working tree, excluded directories), as are dev-dependencies, `workspace = true`
  inheritance and offline `Cargo.lock` resolution. Fix: copy from `git archive` of the
  snapshot commit, reject symlinks and special files, resolve `workspace = true` when
  generating the root, vendor or pre-fetch nothing and require `Cargo.lock` to be complete.
- **L1** The sandbox is stated, not enforced. Fix: name the runner mechanism (network
  namespace, read-only mounts) and have the gate probe it, or fail closed for release.
- **L2** The image scan relied on a `vos_ghost_` prefix that SH17 lacked. Fixed in this
  document (SH17 now requires the prefix); VOS22 keeps the scan heuristic and never treats a
  pass as evidence of absence.
- **L3** A bare-metal image needs a policy linker script and linker flags. Fix: the generated
  build root supplies them from tool policy; the recorder checks `-C link-arg`.
- **L4** An `unverified` branch-protection check must fail closed for release, and the
  `gh api` token scope must be stated. Fixed in section 7.4.
- **L5** `image-sha256` must be reproducible (fixed timestamps, `SOURCE_DATE_EPOCH`, path
  remapping); the recorder must be built from the base branch or digest-pinned; read JSON
  with a real parser; stale `[package.metadata.vos]` and old section references (VOS-D022,
  F6) were fixed in this document.

**Controls VOS22 must carry** (moved out of section 7.5): `RUSTFLAGS` or
`CARGO_BUILD_RUSTFLAGS`/`CARGO_TARGET_<T>_RUNNER`/`RUSTUP_TOOLCHAIN` set (gate refuses); a
repo `.cargo/config` (must **pass**, irrelevant to the pristine tree); resolver 1 with a
dev-dependency ghost (must pass: generated root is resolver 2); `RUSTC_WRAPPER` set; argv with
`--workspace`, `--config`, `-Z`, `--features`, a second `--package`; a `Fresh` unit; a
newline-in-metadata record spoof; a `.json` target or a triple not in the target list; a
toolchain digest mismatch; a missing or extra unit; ghost or `-any` names in the image, or a
stripped image; a probe invocation counted as a unit; a symlink or special file in the copy.
