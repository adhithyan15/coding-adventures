# VER29 — Layer Contract Linking: publishing, assuming and discharging claims between packages

**Status:** Draft specification. Nothing here is implemented, and no code
changes with it. It is the specification that VER27 section 4.14 and backlog
row LV-005 call for before any code. It does not edit VER04 or VER13: the
exact text of both edits is in section 16 and lands with the implementation
slices of section 13.

**Amendment (2026-11-14), owner decision, with [VER04m](VER04m-warrant-stateful-objects.md):**
review item H3 is reworded from "stateless and deterministic binding (`state none`)" to
**"pure in its explicit state"**. What changed: section 6.1 row 4, section 6.2 (the H3
paragraph and its closing remark), section 14 conditions 5 and 8 and the verdict paragraph,
decision D26 and the H3 label control of section 13.1. Why: with the old rule a flip-flop or a
register could never be `exhaustive`, so condition 1 of section 14 failed for the whole 4004 chain
by construction. `exhaustive` is now allowed only for a step claim over explicit state whose state
set is complete and whose binding passes the hidden-state obligations; an operation executed by a
driver, and any claim over a bounded set of call sequences, is never `exhaustive` (it is
`bounded-set`, at most `exhaustive-sampled`, with the depth and call alphabet printed). No contract
id, no rung of the label set and no other rule changes. The mechanism (the `state=explicit` key,
reach classes, obligations, labels) is VER04m.

**Parent:** [VER27](VER27-layered-validation-roadmap.md) (P9; sections 4.13,
4.14 and 4.15; conflict C14; question Q8), [VER00](VER00-verified-shipping-program.md)
(section 7.5, assume-guarantee; section 7.5.6, owned layers).

**Builds on:** [VER04](VER04-warrant-language.md) (sections 2, 3 and 10),
[VER04h](VER04h-warrant-derived-evidence.md) (manifests, derived report, section
9 checks), [VER04i](VER04i-polyglot-warrant-evidence.md) (lanes and lane
manifests), [VER04j](VER04j-warrant-v1-extensions.md) (the v1 version header,
bindings), [VER13](VER13-assumption-catalogue.md) (catalogue, families, rules
AC-1..AC-15), [VER08a](VER08a-rust-capability-analyzer.md) (what the lexical
analyzer can and cannot see), [VER20](VER20-resource-timing-proofs.md).

**Backlog:** LV-005 (this specification), LV-002 (CI re-selection, section 11),
LV-006 (the first adopter, the Intel 4004 chain).

**First publisher:** `logic-gates`, in PR #19252 (branch
`claude/warrant-adopt-logic-gates`), whose `LG01` section "Contracts this layer
publishes (pending LV-005)" lists 88 claims under stable names. That section is
the template this specification turns into a mechanism.

**Direction (owner, 2026-11-12):** each layer is verified alone and every layer
above takes that result as an assumption (VER27 P9). The coordinator's design
decisions for this specification were: ids of the form
`contract.<package>.<claim>`; every named claim published implicitly; a
new generated id family; discharge that fails closed; strength that never
rounds up; coverage by kind; exact linking with no fuzzy matching; direct
dependencies only; re-selection in CI. Where this document refines one of them
or departs from it, section 17 says so.

**Why the number.** The brief for this document said `VER28`. VER27 (header,
"Why the number") records that `VER28` is cited by the layer diagrams of VER05,
VER06, VER06b and VER07 as "VER01 / VER28 Assurance Report & Ship Gate", so it
is reserved for the ship gate even though no `VER28` file exists. This
document therefore takes the next free number, `VER29`: no file, no citation in
`code/specs/`, and no remote branch uses it. If the owner wants the file to be
`VER28`, the rename is a one-line change in each pointer listed in section 16.

---

## 1. Purpose and the design in one page

Today a package cannot say "I rely on exactly what that package proved".
`assume platform:` in a Warrant file names dotted identifiers, and VER04 section
3 requires them to resolve to the shared VER13 catalogue, so "packages cannot
invent private platform axioms". A Warrant module cannot import another (VER04
section 2). The consequence is that the strongest cross-package statement a
report can make is a hand-written catalogue entry such as
`gate.primitives.correct` ("`logic-gates` and `arithmetic` primitives implement
the Boolean functions"), which no tool ties to the evidence that `logic-gates`
actually produced.

The mechanism in one page:

1. **Publish.** Every named `claim` in a package's `.warrant` is implicitly
   published under the id `contract.<package>.<claim>` (section 3).
   Proof-backed obligations (timing, resource) and capability facts are
   published under longer derived ids.
2. **Name.** An upper package writes the id in its own `assume platform:` block.
   It writes nothing else: no formula, no paraphrase.
3. **Discharge.** The upper package's assurance report marks the assumption
   *discharged* only if all five conditions of section 5 hold: the package really
   depends on the lower one; the lower evidence contains the claim and every
   suite passed; the lower report passes `--check`; the lower digests are
   pinned in the upper report and still match; and the claim is not under a
   fault mode. Otherwise it stays *open* with a reason code. A name that resolves
   to nothing is an error, not an open assumption.
4. **Propagate strength.** A discharged assumption carries the weakest evidence
   behind the lower claim; an upper claim is never labelled stronger than the
   weakest contract it rests on (section 6).
5. **Cover and bound.** An upper claim of kind K rests only on contracts that
   cover K (section 7), and every call into the lower package must land on a
   contracted operation or be listed as an unverified dependency (section 8).
6. **Exact linking.** An id either matches a published claim exactly or breaks
   loudly. There are no aliases (section 9).

What this buys today, said plainly. At the layers VER27 section 4.15 lists, the
upper layer's evidence is produced by running the upper tests against the *real*
lower crate. Discharging an assumption therefore does not reduce the upper
layer's test work, and it does not make the upper layer's tests independent of
the lower implementation. It makes three things true and checkable that are
not today: the upper report names exactly which lower claims, at which digests
and at what strength, its evidence rests on; a lower change that weakens or
breaks one of them fails the upper report closed instead of leaving a stale
label; and an upper claim cannot be shown stronger than what it stands on. The
larger saving (replacing the lower implementation by its contract inside an
upper proof) needs the upper layer to have a proof engine on its own code, and
is section 14's subject.

---

## 2. Vocabulary

| Term | Meaning |
|---|---|
| Lower package, upper package | The package that publishes a claim, and the package whose `.warrant` names it. The upper package must list the lower one as a direct dependency in its build manifest (section 5, condition a). |
| Claim | A named `claim` block of a Warrant document (VER04 section 5). An `example` is not a claim and is not published (section 3.5). |
| Contract | A published claim: the pair of an id and the evidence the lower package produced for it. |
| Contract id | `contract.<package>.<claim>`, or one of the longer forms of section 3.1. |
| Link | One contract id in an upper package's `assume platform:` block, together with its resolution in the upper report. |
| Discharged, open | The two states of a link whose id resolves (section 5.2). A link whose id does not resolve is an error, not a state. |
| Pin | A digest of lower-package content recorded in the upper report (section 5.3). |
| Strength | The kind of evidence a claim rests on, from `proved` down to `example` (section 6). |
| Lane | A language lane of VER04i (`rust python typescript go ruby c cpp java kotlin csharp fsharp elixir lua perl haskell swift dart ocaml`). |
| Kind | `functional`, `timing`, `resource` or `capability` (section 7). |

---

## 3. Contract ids

### 3.1 Shapes

| Kind | Id shape | Example | Segments |
|---|---|---|---|
| functional | `contract.<package>.<claim>` | `contract.logic-gates.and_truth_table` | 3 |
| timing | `contract.<package>.timing.<op>` | `contract.ct-compare.timing.eq` | 4 |
| capability | `contract.<package>.capability.<category>` | `contract.logic-gates.capability.proc` | 4 |
| resource | `contract.<package>.resource.<op>.<metric>` | `contract.ct-compare.resource.eq.steps` | 5 |

The id has **no lane segment** (revision after review, item H5; section 17, D20).
The `.warrant` file is language-neutral, one file shared by every lane, so an id
written in it must mean the same thing in every lane's use of that file. The lane
is supplied by the context: it is the lane of the report being generated or
checked, read from the report's own package id (`rust/arithmetic` is lane `rust`).

Segment rules:

* `<package>`: the package's directory name under `code/packages/<lane>/` for the report's own lane,
  which is also the stem of `code/specs/<package>.warrant` and the suffix of the
  report's package id (`rust/logic-gates`). The pattern is
  `[A-Za-z0-9][A-Za-z0-9_-]*`. Hyphens are allowed because real package names
  have them; they are what makes a contract id not a Warrant identifier (section
  3.3). The Warrant module name inside the file (`logic_gates`) is a different
  string and is never used in an id.
* `<claim>`, `<op>`: a Warrant identifier (`[A-Za-z_][A-Za-z0-9_]*`, VER04
  section 2). A claim and an operation are never confused because the segment
  count differs.
* `<metric>`: `steps`, `heap` or `depth`. `<category>`: a VER08 capability
  category (`proc fs ui stdout rand thread net env time` today; the list is the
  analyzer's, not this document's).
* The whole id matches VER13's id pattern `[A-Za-z0-9._-]+`, is at most 256
  bytes, and has no empty segment.

A package may define a claim named `timing`, `capability` or `resource`. That
is a 3-segment id and cannot collide with the longer forms.

### 3.2 Which ids exist

An id *exists* (can be resolved) only if the lower package's own Warrant
document declares what it names:

* functional: a `claim <claim>:` block in `code/specs/<package>.warrant`;
* timing: an operation `<op>` with at least one `labels` row classed `secret`
  (so the document asks for a constant-time obligation);
* resource: an operation `<op>` with a `bound` row for `<metric>`;
* capability: `<category>` is a capability category the analyzer knows.

Existence is read from the lower `.warrant`, never from its evidence. This is
deliberate (section 9): a claim that has lost its evidence still exists, and so
an upper link to it is an *open* assumption with a reason, whereas a name that
the lower document never declared is an *error*.

The kinds other than functional are specified here so that the id space is
complete and rule (a) can be stated, but they are not published by the first
implementation (section 13, slice 2): until their slice lands, a resolvable
non-functional id is open with reason `kind-not-yet-published`.

### 3.3 Where an id is written, and the grammar change

An upper package writes ids in the existing `assume platform:` block:

```text
  assume platform:
    contract.logic-gates.xor_truth_table
    contract.logic-gates.and_truth_table
    contract.logic-gates.or_truth_table
  assume environment:
    none
```

Two lexical facts make this not a one-line change. First, VER04's platform row
is a `qualified_name`, and `QNAME` is `identifier(.identifier)+`, which has no
hyphen; `logic-gates` would lex as `logic`, `-`, `gates`. Second, a global token
that swallows `contract.` followed by anything would break a model that has a
binder named `contract` and projects a field. The specification therefore
requires:

* a platform row may be a `contract_id`, recognised **only at a platform-row
  position** (like the contextual names of VER04j section 1.2), by re-scanning
  the row's source span; there is no new reserved word and no new global
  token;
* the tokens of one row are span-adjacent (no whitespace inside an id), and the
  row is validated against the shapes of section 3.1;
* a `contract_id` in a document without the `v1` header is `W0101` ("v1
  construct in a v0 document"), like every other v1 construct (VER04j section
  1.1);
* a duplicate id within one document is a name-resolution diagnostic (a new code
  in the `W02xx` family, number assigned by slice 1);
* the existing syntax corpus and the three adopters' type-checker output stay
  byte-identical (the test VER04j section 1.3 already uses).

`none` stays a legal sole row. A document may mix catalogue ids and contract
ids.

### 3.4 Lane

Evidence is per lane even though the `.warrant` file is language-neutral. One
`code/specs/logic-gates.warrant` yields one set of declared claims. The id
`contract.logic-gates.and_truth_table`, written once in an upper `.warrant`, is
resolved by each lane's report against **that lane's** package
(`code/packages/<lane>/logic-gates`), manifests, sources and digests. The Rust
report can discharge it while the Python report leaves it open, because the Python
`logic-gates` has no manifest. A lane with no manifest for the package, or no
binding for the claim's operation, resolves to open (`lower-evidence-absent` or
`claim-not-in-manifest`), never to another lane's evidence.

**Cross-lane resolution is an error.** A report never looks in a lane other than
its own. If the named package does not exist in the report's lane, or exists only
as a different lane's package (a Rust crate that a Perl package reaches through a
cdylib), the id fails with `ContractUnknownClaim` (naming the missing lane
directory), and a package that is not a direct dependency in the report's own
lane's build manifest fails with `ContractNotDependency`. Cross-lane links are a
real shape in this repository and are a non-goal here (section 15).

### 3.5 Implicit publication: no `publish` list

**Decision.** Every named `claim` is published. There is no `publish`, `export`
or `contract` keyword.

Why:

* VER04 section 2 already makes `(module, block kind, declaration name)` the
  stable identity of a claim, tied to the exact source revision. A second list
  of names would be a second identity to keep equal to the first.
* The feature freeze (VER27 Q7) asks for the smallest change that serves the
  named adopter. The adopter (`logic-gates` and its uppers) is served by
  implicit publication; an opt-in list adds a keyword, a grammar production and
  a "claim exists but is not published" state the adopter does not need.
* Implicit publication does not make renames expensive in practice. A claim's
  name matters only to the packages that assume it, and a package that assumes a
  claim says so in a file the build tool can read (section 11). Renaming an
  unassumed claim breaks nothing.

The cost, stated: every claim name becomes an interface of the package, including
ones no upper layer would ever want (`*_rejects_non_bit_*`). An opt-in list would
let an author keep internal claims private and rename them freely. If friction
shows up in LV-006, an `export` list can be added later without changing any id
(open question VQ1).

`example` blocks are named but are **not** published. An example is a single
case with no quantifier; its strongest honest strength is `example`, and a link
to it would invite exactly the near-miss paraphrase rule (c) rules out.

---

## 4. Resolving conflict C14

VER27 conflict C14: VER04 section 3 requires platform ids to resolve to the
VER13 catalogue, VER13's id families are a closed set (`spec src wir cc mc
uarch gate phys tcb os gov att`), and a package-published id is in neither. VER27
offered two ways out: add a `contract` family with a validity rule, or keep a
package-published registry beside the catalogue.

**Resolution: a new, generated family `contract`.** The set becomes `spec src
wir cc mc uarch gate phys tcb os gov att contract`. Properties of the family:

| Property | Value |
|---|---|
| Hand-catalogued? | No. No entry is added to `code/assumptions/catalogue-v1.json`, and `catalogue-v1.json`'s digest, counts and AC-14 spec-sync test are unchanged. |
| Where is an id valid? | In a report, if and only if section 5's resolution derives it from the named package's `.warrant` and evidence at the pinned digests. |
| Who resolves it? | `ca-assurance-report`, at `--generate` and again at `--check`. The Warrant checker (`warrant-grammar`) validates shape only. |
| Layer in the VER13 sense? | None. A contract id is consumed at the layer of the upper package; AC-5 (layer monotonicity) and AC-8 (floor entries) do not apply to it. |
| Status in the VER13 sense? | None. `assumed` / `evidenced` / `discharged` are catalogue statuses. A contract link has the two states of section 5.2, derived anew at each run and never stored as authority. |

Why not a registry beside the catalogue: a second registry is a second place a
report can cite an id that no check ties to evidence, which is the defect VER13
exists to remove (section 1 there). A generated family has no stored text to
drift: its truth is a function of files that are already checked in and already
digested.

The exact amendment text for VER04 section 3 and for VER13 is in section 16.

---

## 5. The discharge rule

### 5.1 Five conditions, all required

An upper package's assurance report marks a `contract.*` assumption
**discharged** only if every condition holds. The check is performed at
`--generate` and re-derived at `--check`; a report that says `discharged` where
the derivation says otherwise fails `--check`.

| | Condition | Reads | Fails as |
|---|---|---|---|
| (a) | The upper package depends on **the in-tree lower package** (section 5.1.1): a non-optional, non-target-specific, non-feature-gated `[dependencies]` entry of `Cargo.toml` that is a `path` dependency whose path canonicalises to `code/packages/<lane>/<package>`, whose package name matches, with no `[patch]`, `[replace]` or `package =` rename affecting it. A dev-dependency or build-dependency does not count. | upper `Cargo.toml`, workspace manifest | error `ContractNotDependency` (not an open state) |
| (b) | The lower evidence manifest contains the claim, every suite entry for it has verdict `pass`, and at least one suite is not an example. | lower `evidence/test-manifest.json` (and the proof manifest for non-functional kinds) | open: `claim-not-in-manifest`, `no-suite`, `suite-failed` |
| (c) | The lower package's own report passes `--check`, run on the same tree with the flags the lower package's own BUILD uses (section 5.1.2). Its `unverified::` ids are carried, not hidden. | lower `assurance_report.json` and everything its own check reads | open: `lower-report-absent`, `lower-report-fails-check` |
| (d) | The lower digests are pinned in the upper report and every pin still matches the files on disk (section 5.3). | the pins and the lower files | error `ContractPinMismatch` at `--check`; at `--generate` the pins are simply (re)computed |
| (e) | The claim is not under any sabotage or fault mode (section 5.5). | lower manifests, environment | open: `lower-fault-mode` |

In addition, these are required, which the brief did not list, because they
are what makes "the lower claim holds" mean something to the upper layer:

* the lower claim's own contract assumptions are all discharged (otherwise it
  holds only conditionally, and the upper link is open with
  `lower-claim-conditional`);
* the lower claim's evidence and the upper link were built the same way: the
  lower package's features, `cfg` flags and `build.rs` outputs in its evidence
  build are the ones the upper package links it with. If they differ or cannot be
  shown equal, the link is open with `build-config-unpinned` (section 5.1.1);
* the lower and upper reports agree on the stack of the claims involved (VER13
  rule AC-7; open with `stack-mismatch`). `stack` is recorded **per claim proof**
  in the report (`host-rustc` today, `owned-rv32i` for a rung R2 proof), not per
  report, so the comparison is made per operation row: a contract's stack is the
  one on the lower operation's proof block, or `host-rustc` for test-only
  evidence.

### 5.1.1 Binding condition (a) to the in-tree package

A dependency name alone is not enough: a `path` to a fork, a `package =` rename,
a registry or git dependency, a `[patch]` or `[replace]` section, or an optional,
target-specific or feature-gated dependency would each satisfy "depends on
`logic-gates`" while linking different code from the code whose evidence was read.
The Rust-lane reader therefore requires **all** of:

* a `[dependencies]` entry (not `[dev-dependencies]`, `[build-dependencies]` or a
  `[target.*]` table) that is not `optional = true` and is not enabled only by a
  feature;
* a `path` key that, canonicalised (symlinks refused), equals
  `code/packages/<lane>/<package>`, with no `git`, `registry` or `version`-only form;
* the lower `Cargo.toml` `[package] name` equal to the dependency key, and no
  `package =` rename on the entry;
* no `[patch]` or `[replace]` entry, in the upper manifest or the workspace root
  manifest, that names the lower package or its source;
* the lower package's `Cargo.toml` and `build.rs` (if any) digested and pinned
  (section 5.3).

If the lower crate has features or `cfg` that differ between its evidence build
(the default features, as the evidence BUILD runs `cargo test`) and how the upper
package links it (a `features =`, `default-features = false`, or a `cfg` on the
upper side), the link is open with `build-config-unpinned`. Until the manifests
record the feature set and `cfg` list of the evidence build, the reader treats any
`features` or `default-features` key on the dependency entry as that case.
Anything else that falls outside these requirements is `ContractNotDependency`.

### 5.1.2 Re-checking the lower report: two choices, the second chosen

The lower report could be re-checked strictly (no `--allow-unverified`), or checked
as its own BUILD checks it, with its `unverified::` ids carried upward. Strict
re-checking would make every link to `ct-compare` open today, because its report
still cites nine `unverified::` attestations and its BUILD passes
`--allow-unverified` (VER04h section 9, check 15; backlog WD-008). **Decision: the
second.** The lower report is checked with the flags its own BUILD uses, every
other rule applying in full, and every `unverified::` id it cites, plus those it
inherits from its own links, is carried into the upper claim rows'
`conditional_on` as `inherited-unverified: <id> via <contract id>`. They are never
inherited silently: a claim row that rests on a lower report with unverified
attestations cannot print without them. When WD-008 removes the flag, the carried
list is empty and the two choices coincide.

### 5.2 Errors and open states

Two outcomes, kept apart on purpose:

* **Error.** The report is invalid, `--generate` refuses to write it and
  `--check` fails. These are mistakes a person must fix: the id is wrong, the
  dependency is not declared, a pin is stale, a label was forged.
* **Open.** The id is valid and resolves to a claim the lower document declares,
  but the claim is not currently proved to the standard of section 5.1. The
  report is valid and lists the assumption as open with a reason. CI policy
  decides whether an open link blocks (section 5.6).

Errors (each is a named failure in the report checker, with a falsification
test):

| Failure | Raised when |
|---|---|
| `ContractIdMalformed` | An id is not one of the shapes of section 3.1, has an empty or over-long segment, a disallowed character, or a lane segment (ids carry none). |
| `ContractSelfLink` | The id names the upper package itself. |
| `ContractNotDependency` | Condition (a) fails: the named package is not a direct normal dependency of the upper package, or is a dev-dependency only. |
| `ContractUnknownClaim` | The lower package (in the report's own lane), its `.warrant`, or the named claim, operation or category does not exist (section 3.2). |
| `ContractDuplicate` | An id appears twice in one document or one report. |
| `ContractCycle` | Resolving the link re-enters a package already being resolved (section 10). |
| `ContractPinMismatch` | A pinned digest differs from the file on disk at `--check`. |
| `ContractDischargeMismatch` | The report's status, reason or strength differs from the one re-derived from the files. |
| `ContractStrengthOverclaim` | An upper claim row is labelled stronger than section 6 allows. |
| `ContractKindOverclaim` | An upper claim row carries a strength for kind K that rests on a contract that does not cover K (section 7). |
| `ReachClaimUnsupported` | The report states `call-level` reach without the output of the pass of section 8, or states anything but `reach-around unchecked` before that pass exists. |

Open reasons. When several apply, the first in this order is the `reason` and
the rest go in `also`:

| Reason code | Meaning |
|---|---|
| `lane-not-supported` | The lane's build-manifest reader or evidence reader is not implemented. Only the Rust lane is, at first. |
| `lower-evidence-absent` | No evidence manifest at the lane's path (`evidence/test-manifest.json` for Rust; `evidence/<lane>/test-manifest.json` for the other lanes, VER04i section 8). |
| `lower-report-absent` | No `assurance_report.json` in the lower package. |
| `lower-report-fails-check` | Condition (c): the lower report does not pass its own `--check`. The lower failure is quoted in `detail`. |
| `claim-not-in-manifest` | The lower document declares the claim; the manifest has no entry for it (stale manifest, or this lane does not bind the claim's operation). |
| `no-suite` | The manifest names the claim but has no non-example suite for it. |
| `suite-failed` | A suite entry for the claim has a verdict other than `pass`. |
| `lower-fault-mode` | Condition (e), section 5.5. |
| `build-config-unpinned` | The lower crate's features or `cfg` in the evidence build differ from, or cannot be shown equal to, how the upper package links it (section 5.1.1). |
| `lower-claim-conditional` | The lower claim rests on a contract link of its own that is open; the open ids are listed. |
| `stack-mismatch` | The lower report's `stack` differs from the upper's. |
| `kind-not-yet-published` | The id is a timing, resource or capability id and its slice has not landed. |

Why unknown is an error and stale is open: if both were open, renaming a claim
would silently turn an upper layer's discharged assumption into an open one, and
an open assumption does not fail a build by default. Rule (c) asks that a rename
"break the link loudly", so the unresolvable case is the error. A stale or
failing claim is the lower layer's state of health, which the upper report
should show as open, not refuse to be generated.

### 5.3 Pins

Condition (d) pins, per link, in the upper report (section 5.4 shows the
field):

| Pin | What it is | Why |
|---|---|---|
| `spec_sha256` | SHA-256 of the lower `code/specs/<package>.warrant` | The statement of every claim. |
| `manifests` | SHA-256 of each lower manifest the link read, by name (`test`, and `proof` or `bench` for the other kinds); the same digests VER13 AC-3 already computes | The evidence. |
| `cargo_sha256`, `build_rs_sha256` | SHA-256 of the lower `Cargo.toml` and `build.rs` (or the literal `absent`) | Features, dependencies and build-time code the evidence build used. |
| `unverified_deps` | per listed unverified **path** dependency of the lower package (for `logic-gates`, `transistors`): its `source_sha256` and `Cargo.toml` digest | The code the lower claim stands on without a contract. A change there is not invisible. |
| `toolchain` | the toolchain id of the evidence build (`rustc` version and target, as VER13 AC-9 pins) | The evidence is for one build. |
| `source_sha256` | the lower manifest's `source_sha256` (the `warrant-src-v1` framing of every file under the lower `src/`, VER04h section 8), recomputed from disk | The code the evidence ran. |
| `claim_sha256` | SHA-256 of the claim declaration's canonical form: the lexer's comment-free token stream of the declaration, tokens joined by single spaces | Exactness: it is the claim's text, so a rename or a changed formula is visible as one changed line in the upper report's diff. |
| `claim_entries_sha256` | SHA-256 of the claim's manifest entries, canonical JSON, sorted | Lets the check say "the pinned claim's evidence is unchanged, only an unrelated claim moved" (a message, not a relaxation). |

The brief names three pins (spec, manifest, source or revision). `claim_sha256`
and `claim_entries_sha256` are additions (section 17). The fourth pin a reader
might expect, a git revision, is deliberately absent: a report is committed in
the same commit as the files it digests and cannot contain its own commit id.
All pins are content digests, so they are reproducible from a checkout. The
`version` of the lower package is recorded as a label, not a pin.

Any pin that no longer matches the file on disk fails `--check` closed with
`ContractPinMismatch`, whether or not the change looks harmless. The remedy is
to regenerate the upper report, which is then a reviewable diff.

### 5.3.1 Regeneration must not accept a downgrade silently

Regenerating recomputes pins, and could therefore turn a weakened or changed
claim into a clean, green report. The gate is in CI, not in `--check` (which sees
one tree): the slice-3 script `contract_regression_gate` compares each package's
`contracts` block against the one at the diff base (`--diff-base`, as the build
tool does) and **fails** if, for any link present in both, the strength decreased,
the status went from discharged to open, or `claim_sha256` changed. A
**commit trailer** in any commit of the range accepts it, one per id:

```text
Contract-Change-Accepted: contract.logic-gates.xor_truth_table: tightened to cover 3-valued input
```

The reason text is required and non-empty. Choice of trailer over a CLI flag: it
travels with the change, is visible in review and in `git log`, and cannot be left
enabled in a BUILD file. A new link, a removed link, and a strength increase need
no acceptance.

### 5.4 What the report records

A new top-level `contracts` object (`schema: "ver29-contract-links-v1"`) in
`assurance_report.json`, validated by VER01's report schema once VER01 gains it.
Illustrative shape (digests abbreviated to `…`):

```json
"contracts": {
  "schema": "ver29-contract-links-v1",
  "lane": "rust",
  "reach": {"state": "unchecked", "text": "reach-around unchecked"},
  "dependencies": [
    {"package": "rust/logic-gates", "section": "dependencies",
     "linkage": "linked",
     "assumed": ["contract.logic-gates.and_truth_table",
                 "contract.logic-gates.or_truth_table",
                 "contract.logic-gates.xor_truth_table"]}
  ],
  "assumed": [
    {"id": "contract.logic-gates.xor_truth_table",
     "kind": "functional", "status": "discharged", "strength": "exhaustive",
     "limited_by": [],
     "lower": {"package": "rust/logic-gates", "version": "0.1.0",
               "stack": "host-rustc",
               "spec_sha256": "…", "manifests": {"test": "…"},
               "source_sha256": "…", "cargo_sha256": "…", "build_rs_sha256": "absent",
               "unverified_deps": {"rust/transistors": {"source_sha256": "…", "cargo_sha256": "…"}},
               "toolchain": "rustc 1.xx.x x86_64-unknown-linux-gnu",
               "claim_sha256": "…", "claim_entries_sha256": "…"},
     "evidence": [
       "rust/logic-gates:warrant/logic_gates/exhaustive/xor_truth_table",
       "rust/logic-gates:warrant/logic_gates/property/xor_truth_table",
       "rust/logic-gates:warrant/logic_gates/fuzz/xor_truth_table"]}
  ],
  "published": [
    {"id": "contract.arithmetic.add4_modular_sum", "kind": "functional",
     "own_strength": "exhaustive", "strength": "unattributed",
     "limited_by": [], "claim_sha256": "…", "claim_entries_sha256": "…",
     "evidence": ["warrant/arithmetic/exhaustive/add4_modular_sum", "…"]}
  ]
}
```

* `assumed` has one entry per id in the package's `.warrant`, discharged or open.
  An open entry has `status: "open"`, `reason`, `also`, `detail`, and no
  `strength`.
* `published` has one entry per **Warrant claim**, and it is where per-claim
  strength lives. The report's own claim rows are per **operation**, not per claim
  (`derive_claims` makes one row `<symbol>_soundness` per operation by summing that
  operation's suites; `wasm-leb128` has 6 such rows for 42 claims, with mixed
  domains, and `logic-gates` has 22 rows for 88 claims). A row-level strength, where
  one is shown, is the **minimum** over the strengths of that operation's claims,
  never the maximum, so a row can never read stronger than its weakest claim.
  `published` has one entry per claim the package publishes. A package that
  assumes nothing and is assumed by nothing still has the block (it is how the
  next layer up finds strengths); `logic-gates` has 88 rows.
* `evidence` ids inside `contracts` are written `<package>:<id>` with the lower
  package's prefix. They are **not** added to a claim's own `evidence` list:
  VER04h section 9 check 13 forbids an evidence id repeating across manifests, and
  a lower id belongs to the lower manifest. VER13 AC-3 for a contract id means
  "each cited id exists in the named lower manifest, with verdict `pass` and the
  pinned digest", which `contracts` carries.
* `dependencies` lists **every** direct normal dependency, whether or not any
  contract is assumed from it. The dependency-level value is `linked` (the package
  assumes at least one contract of it) or `unverified` (it assumes none).
  `linked` says **nothing** about which operations are covered: it is the
  dependency-level fact only. The word `covered` is reserved for call level
  (section 8.3) and means "the operation used has a contract", with the argument
  domain and the caller's preconditions unchecked. Third-party registry crates and
  `std` are listed under `external` and are unverified by definition. This is the
  reported unverified dependency of section 8, available before any call analysis
  exists.
* Each operation row of the report gains a `contract` object:
  `{"kind", "strength", "strength_state", "limited_by", "conditional_on"}` (section
  6). `strength` is the minimum over the operation's claims; `strength_state` is
  `own` (no contract link applies), `attributed` (call-level pass landed, section
  8) or `unattributed` (section 6.4). The existing per-operation `proof` block and
  VER13 labels are unchanged. `stack` stays on that `proof` block, where it is
  recorded per claim proof (`host-rustc` or `owned-rv32i`); it is never a
  report-wide value.

### 5.5 Fault and sabotage modes (condition e)

A claim is under a fault mode if any of these hold:

1. a lane manifest carries `faulted: true` (VER04i section 8). A Rust run with
   `WARRANT_EVIDENCE_FAULT` set never writes a manifest (VER04h section 6), so a
   Rust manifest that exists was not written by a faulted run; but a hand-edited
   one is what condition (c) and the digests are for.
2. the environment of the process generating or checking the upper report has
   `WARRANT_EVIDENCE_FAULT`, `WARRANT_EVIDENCE_FILTER` or `WARRANT_PROOF_FAULT`
   set. The generator refuses to run, rather than recording a result computed
   while a fault is injected.
3. **Interpretation beyond the brief:** a negative-control entry (`kind: negctl`)
   for the claim's operation has a verdict other than `pass`. A negctl is the
   evidence that the suites can fail; if one does not, the suites for that
   operation are not shown to detect a fault, and the claim is open with
   `lower-fault-mode` and a `detail` naming the control. This is stricter than
   the brief's wording; it follows VER04h section 6 ("a suite that cannot fail is
   not evidence"). **Limit:** negative controls are per (suite kind, operation,
   fault), and a control passes when *some* claim of that kind for the operation
   catches the fault (`caught_by`). A passing control therefore does not show that
   the *assumed claim's own* suite can fail; another claim of the same operation may
   have caught every fault. Requiring a control that targets the assumed claim is
   open question VQ11.

### 5.6 Policy: when an open link blocks

Open is a valid report state. Whether it blocks is a flag, not a property of
the report, in the pattern of `--allow-unverified` and `--strict`:

| Flag | Effect |
|---|---|
| (none) | Open links and unverified dependencies are printed and the check passes. |
| `--require-contracts` | `--check` fails (`OpenContract`) if any link in the package's `.warrant` is open. **Recommended default for the BUILD of any package whose `.warrant` assumes a contract**, so that an adopter cannot silently run with an open link. |
| `--require-closed` | Implies `--require-contracts`, and also fails if any dependency is `unverified` or any call is unaccounted for (section 8). This is a publish-gate flag (VER27 section 8.1), not a default. |

---

## 6. Strength propagation

### 6.1 The label set

Strength says what kind of evidence stands behind a claim. Ordered strongest to
weakest:

| Rank | Label | Meaning | Source in the manifests |
|---|---|---|---|
| 5 | `proved` | A certificate-checked proof (`proved` in VER13 section 5.4). | proof manifest entry with label `proved` |
| 4 | `exhaustive` | Every value of the claim's whole input domain was enumerated and the claim held, by a binding that is pure in its explicit state (section 6.2). | `exhaustive` suite whose every binder is a complete finite type, on a binding that is pure in its explicit state |
| 3 | `bounded-proof(k=K)` | A proof up to a stated bound `K` (`bounded(K)`). `K` is **always printed** (`bounded-proof(k=128)`): a proof up to length 128 says nothing about length 129, which an enumeration of a complete domain does not leave out. | proof manifest entry with label `bounded(K)` |
| 2 | `exhaustive-sampled` | Enumerated over a stated finite domain that is not the whole input domain (a representative set, edge values, widths up to a bound). | any other `exhaustive` suite |
| 1 | `randomized` | Property or fuzz evidence only. | `property` or `fuzz` suites |
| 0 | `example` | Examples only. No current generator produces a claim with example-only evidence; the label exists so that a downgrade has a floor to land on. | `example` entries |

The brief's order was proof, exhaustive, property or fuzz, example. Two things
were added because the manifests make them unavoidable, and one was kept tied.
(Review revision: `bounded-proof` ranks below `exhaustive`, not between `proved`
and it, so a bound can never outrank a complete enumeration.) The order is a
**convention for taking a minimum**, not an implication: `bounded-proof(k=K)` and
`exhaustive-sampled` are not comparable in general (a proof to length 128 and a
hand-picked sample answer different questions), and `exhaustive-sampled` ranking
above `randomized` is a choice, not a theorem. Both are placed where a consumer
who takes the minimum errs towards the weaker label.

* `exhaustive-sampled`. VER04h section 3 says plainly that "exhaustive" is only
  ever relative to a stated finite domain, and the domain table gives `Bytes`
  a representative set, `Nat` the range `0..=64` and `U64` 377 edge values.
  `wasm-leb128`'s own manifest has `"domain": "v: U64 edge(377)"` under the kind
  `exhaustive`. Calling that `exhaustive` in a contract link would round a sample
  up to a proof by enumeration. `logic-gates` shows the split in one table:
  `and_truth_table` (`Bool set(2)` twice) is `exhaustive`;
  `and_n_truth_table_every_width` (`n: Nat set(65)`) is `exhaustive-sampled`.
* `bounded-proof(k=K)` is VER13's own `bounded(k) < proved`, kept as a separate
  rung and printed with its bound. When two bounded proofs meet in a minimum, the
  smaller `K` wins.
* Property and fuzz share the label `randomized` (the brief's tie); the row in
  the report says which suites exist.

### 6.2 Deciding "complete" (exhaustive versus sampled)

An `exhaustive` suite supports the label `exhaustive` only if **both** parts hold.

**Pure in its explicit state (review item H3, reworded 2026-11-14).** Enumerating inputs from
one initial state of a stateful operation (a flip-flop's `clock` method called from a freshly built
object for each case) proves the transition from that state only; it says nothing about the next
state, the history, or other initial states. The evidence is `exhaustive` only if every operation
the claim calls is a pure function of its **explicit arguments**: its binding declares `state=none`
(a package-level line or a per-operation key, validated like the other binding keys), **or** declares
`state=explicit` and, for that operation, (a) the claim's state set is complete (a claim over the whole
state type, or over the set of states satisfying a declared invariant, as VER04m E6 defines) and (b) the
state-completeness obligations of VER04m E15 (`codec-roundtrip`, `rebuild-equivalence`, `deterministic`,
`order-independence`) passed, each with its negative control, in the same manifest. In both cases the
package's effect certificate and capability audit show no clock or random source
(`effect-certificate::proved`, `capability-audit::rand-empty`; the runtime gate of VER08b observes the
rest). An operation without either declaration is classified at most `exhaustive-sampled`, whatever its
binders.

An operation **executed by a driver** (a WX-F driver or a protocol dispatch driver, VER04m section 6) is
never `exhaustive`, and neither is any claim that quantifies over a bounded set of call sequences: such a
claim is `bounded-set` (backlog LP-012) and is at most `exhaustive-sampled`, printed with the depth and the
call alphabet (`traces(calls=8, depth<=5) = 37,449`). A fixed composition of explicit-state steps from
every state of a complete set (a two-step "low then high captures the data" claim) is not such a quantifier
and may be `exhaustive`; the report prints its step count. "All traces agree with the model, by induction"
is an inference over step claims and a closure argument, not a suite; it is not a rung and is not printed by
this specification (VER04m section 7.2).

This matters for the `sequential` layer of the 4004 chain (registers, flip-flops): their step claims are
`exhaustive` through explicit state (VER04m section 5.1) once their bindings declare `state=explicit` and
the obligations pass. The `state none` key is a small producer change in the bindings reader; absent a
declaration the classifier is conservative, so `logic-gates` stays `exhaustive-sampled` until its bindings say
`state none`, and its sequential layer until they say `state=explicit` with the obligations evidenced.

**Complete domain.** The suite is complete when every binder of the claim is of a type
whose whole value set the generator enumerates: `Bool`, `U8`, `U16`, `I8`,
`I16`, each recorded as `set(2)` or `full(n)` with `n` equal to the size of the
type. `Nat`, `Int`, `U64`, `Bytes` and any binder recorded as `edge` or listed in
`demoted` make the suite sampled, even when a refinement in the claim happens to
bound the binder to what was enumerated. That is conservative by design: the
classifier cannot prove that a refinement closes a domain, and a conservative
label is a sound one.

Today the manifest's `domain` is free text (`a: U8 full(256); b: U8 full(256);
c: Bool set(2)`). The first implementation parses it with one strict
expression and falls down to `exhaustive-sampled` on any text it does not match.
A structured `binders` array in the manifest is a producer change that
regenerates every adopter's manifest; it is a follow-up (section 13, slice 2),
not a prerequisite.

### 6.3 Own strength, effective strength, and the rule

For a published claim:

* **own strength** is the strongest label among the claim's suites, provided all
  of them passed. Several suites of one claim are independent attempts at the
  same formula, so the strongest valid one is the evidence; this reads "the
  weakest evidence kind behind the lower claim" as weakest *across the chain of
  contracts*, not across sibling suites (section 17 records the interpretation:
  taking the minimum over a claim's own exhaustive, property and fuzz suites would
  label every claim `randomized` and defeat the ladder).
* **effective strength** is `min(own strength, effective strength of every
  contract the claim's package assumes, of the same kind)`.

An upper claim is never labelled stronger than its effective strength.
`ContractStrengthOverclaim` is the check.

A discharged link carries the lower claim's *effective* strength, which is
what the lower report published. Because the lower effective strength already
contains the lower layer's own links, a three-layer chain composes by reading one
report per step: layer 3 reads layer 2's `published` rows, layer 2's strengths
already include layer 1.

### 6.4 Attribution: an upper claim rests only on the contracts of the operations it uses

`assume platform:` is document-level (VER04 section 2: "Assume sections occur once
per kind"); a claim cannot say which of the document's contracts it uses. Taking a
strength from "any contract on the dependency" is wrong in both directions (review
item C1):

* `arithmetic` assumes only `contract.logic-gates.not_truth_table`, yet its adders
  call `xor_gate`, `and_gate` and `or_gate`: a strength taken from the `not` link
  would be printed for code that rests on three uncontracted operations.
* An upper claim proved over nibbles through an adapter and a refinement
  (`add4_modular_sum`, `a < 16`) says nothing about a caller that passes 12-bit
  values to the same export.
* `hmac` assumes `timing.eq` and also calls a sibling operation that is not
  constant time.

**Rule.** An upper claim's strength may rest only on contracts for the operations
that claim's operation actually uses. Until the reach pass of section 8 can say
which operations those are, **no contract strength is taken for an upper claim at
all**: the claim's strength state is `unattributed`, printed as
`unattributed (own: exhaustive)` (its own suites' strength, never a contract's). In
a minimum, `unattributed` is below every label, so a claim that rests on an
unattributed lower claim is itself `unattributed`. A package with no contract link
at all (the bottom layer) has `strength_state: own`. After slice 4, each claim is
attributed to its operation's used lower operations and its effective strength is
the minimum over *those* operations' contracts of the claim's kind. A used
operation with no assumed contract makes the claim conditional on it (section 7.2,
rule C2), never silently covered.

This replaces the first revision's package-wide cap, which was sound only in the
direction of being weak but could still print a label for the wrong operation.

### 6.5 How it appears

Strength lives in `published`, one row per Warrant claim. A report's per-operation
row shows the **minimum** over that operation's claims (never the maximum; the
operation row is a summary, not a per-claim fact). Before slice 4:

```text
published add4_modular_sum   own exhaustive   strength unattributed (own: exhaustive)
published half_adder_sum     own exhaustive   strength unattributed (own: exhaustive)
link      xor_truth_table    discharged       exhaustive   (the lower claim's strength; for the lower layer, strength_state own)
op row    add4               unattributed (own: exhaustive)    [min over its claims]
```

After slice 4, with each claim attributed to the operations it uses:

```text
published half_adder_sum     exhaustive   limited_by []  uses [xor_gate, and_gate]  covered (domain unchecked)
published full_adder_sum     exhaustive   limited_by []  uses [xor_gate, and_gate, or_gate]
published and_n_user         exhaustive-sampled  limited_by [contract.logic-gates.and_n_truth_table_every_width]
```

`conditional_on` lists open links of the claim's kind that its operation uses,
every contract-less dependency, and inherited `unverified::` ids (section 5.1.2).
The one rule for what prints (review item C2, section 7.2): a **functional**
strength prints together with `conditional_on: [<dependencies without contracts>]`;
**timing, resource and capability** strengths are *withheld* for any contract-less
dependency, external crates and `std` included. The rendering of a conditional
functional claim is `exhaustive, conditional on 1 unverified dependency`; the words
are never dropped. A claim with no `conditional_on` and no `limited_by` is printed as
its label alone.

---

## 7. Rule (a): coverage by kind

### 7.1 The four kinds and how each is derived

The kind of a claim or link is **derived from where its evidence comes from**,
never declared by an author, because a declaration could be wrong and nothing
would catch it.

| Kind | Derived from | Evidence | Strength source |
|---|---|---|---|
| functional | a `claim` block (and `ensures no_panic, terminates`) | test manifest | section 6.1 |
| timing | an operation with a `secret` label: its `constant-time` obligation | proof manifest (`constant-time`) | `proved`, `bounded-proof`; `unchecked` or `not-live` is not publishable |
| resource | a `bound` row: `steps-bound`, `heap-bound`, `depth-bound` obligations; the zero-allocation bench entries | proof manifest, bench manifest | proofs as above; a bench observation is `example` |
| capability | the package's effect certificate and capability audit (VER08a, VER23a, VER08b) | `effect_certificate.json`, `capability-audit::*` attestations, runtime conformance | to be settled with that slice (open question VQ5) |

### 7.2 The rule

An upper claim of kind K may only rest on contracts that cover K.

* A **constant-time** upper claim (kind timing) needs a discharged *timing*
  contract for every lower operation it reaches. A functional truth table says
  nothing about time (VER27 rule (a); LG01 states it for the gates: "nothing in
  this document claims the gates are constant time").
* A **resource** upper claim needs resource contracts below it; a **capability**
  claim needs capability contracts below it.
* A **functional** upper claim needs functional contracts.

Mechanically, for each upper claim of kind K, **for every operation the claim's
operation uses** (section 8) in a dependency D, a discharged contract of kind K
covering exactly that operation must be assumed. Until the pass can name the used
operations, every direct dependency of the package counts as used. If a K contract
is missing, the K-strength is withheld: the claim row has `strength: null`,
`reason: "kind-uncovered"` and names the dependency and, after slice 4, the
operation. Writing a strength there anyway is `ContractKindOverclaim`.

### 7.2.1 One rule for dependencies without contracts (review item C2)

The first revision said three inconsistent things about a dependency that
publishes no contract (`logic-gates` over `transistors`; external crates; `std`).
There is one rule:

| Claim kind | A used dependency has no contract of that kind (path dependency, external crate or `std`) |
|---|---|
| functional | The strength **prints**, and the claim carries `conditional_on: [<each such dependency>]`. A functional claim over host `std` is the normal case and is not withheld. |
| timing | The strength is **withheld** (`strength: null`, `kind-uncovered`). |
| resource | The strength is **withheld**. |
| capability | The strength is **withheld**. |

Why the asymmetry: a functional claim is a statement about outputs that the tests
check on the real stack, so an uncovered dependency is a visible condition on a
real result. A timing, resource or capability claim says something about what the
whole stack does underneath, so any layer the claim has no contract for is a hole
in the statement itself; `std` and external crates are such layers, and a
`timing` strength over them is withheld until a contract (or a VER13 catalogue
entry for the host) covers them.

### 7.2.2 `covered` is narrow

`covered`, at call level, means only "the operation used has a contract". It does
**not** say the arguments the upper layer passes lie in the contract's domain, or
satisfy the refinement its claim carried (`a < 16u8`), or that the upper layer
respects the preconditions the contract states by its `rejects` rows. A
nibble-domain contract used at 12 bits is `covered`. The report prints
`covered (domain unchecked)` wherever it prints the word, and section 15 lists the
limit. A future domain-compatibility check is open question VQ12.

### 7.3 Example: constant time all the way down

`hmac` over `ct-compare`: an `hmac_verify` claim that the comparison is
constant time needs `contract.ct-compare.timing.eq`. If `ct-compare`
publishes only functional claims, `hmac`'s timing strength is withheld with
`kind-uncovered(ct-compare)`, and CWE-208 stays open exactly as it does
today. Discharging the timing id is then one `assume platform:` row. Its strength
is the label of `ct-compare`'s constant-time proof (`proved` or
`bounded-proof(k=K)`), which caps `hmac`'s, **only for the operations that
contract names**: if `hmac_verify` also calls a sibling `ct-compare` operation
with no timing contract (or calls `std::cmp`), the timing strength stays withheld
for that claim, with the uncovered operation named once slice 4 lands. Package-
granular `linked` is not enough; the used operation must be covered.

---

## 8. Rule (b): no reaching around

### 8.1 The requirement

Every call the upper package's code makes into the lower package must land on
an operation covered by an assumed contract. If it does not, it must appear in
the report as an explicit unverified dependency. Silence is the only forbidden
outcome.

### 8.2 Which tool does it

The Rust capability analyzer (`ca-capability-analyzer`, VER08a) does **not** have
a call graph and cannot be asked for one. It is a lexical analyzer with a
use-and-alias binding table (VER08a section 3.2), a method-call heuristic, and a
list of things it cannot see (section 8 there). So the answer to "the analyzer's
call graph, or a new pass" is: a **new pass that reuses the analyzer's lexer and
binding closure**, not a feature of the call graph the analyzer does not have.
The WIR front end (VER25b) is not an alternative: it lowers a subset and refuses
most real crates (`not-live` for `wasm-leb128` and `logic-gates`).

What the pass can and cannot do, so nobody expects more:

| Reachable | Not reachable |
|---|---|
| Every path in the upper `src/**/*.rs` that the binding closure resolves into the lower crate (`logic_gates::gates::and_gate`, or `and_gate` after `use logic_gates::gates::{and_gate, ..}`, as `arithmetic/src/adders.rs` writes it) | Method calls on a value of a lower type (`reg.write(..)`): the receiver's type is not known lexically; these are counted and listed as `unresolved` |
| The lower item's identity is its **defining path**, found by resolving the written path through the lower crate's actual module tree and `pub use` re-exports (read from the lower `src/lib.rs` and the files it declares), not by matching the last segment. `logic_gates::gates::and_gate` and the root re-export `logic_gates::and_gate` resolve to one definition, which is then matched to the lower's bound exports (`evidence.bindings`, `export=`). A different item that merely shares the name (`other::and_gate`, a private function in another module) is a different definition and does not match. A re-export the pass cannot follow (a glob it cannot expand, a macro-generated item) leaves the use `unresolved` | Calls through function pointers, trait objects, generics and macros that expand to a call; code from `build.rs`, `OUT_DIR`, `include!`, `#[path]` modules and macro expansion |
| Uses of a lower constant or type (`MAX_DECODER_WIDTH`, a struct) as item uses | Which of the listed uses a given upper operation actually executes: a package-level list, not a per-operation one, until the pass follows own-package calls |

The pass is an additive mode of the analyzer (`--emit-uses`, JSON out); existing
invocations in every Rust package's BUILD are unchanged.

### 8.3 Classification

For each resolved use:

| Class | Condition | Report |
|---|---|---|
| `covered` | the lower item (by defining path) is a bound operation and an assumed, discharged contract's manifest entries name it (`op`). **Only that:** the arguments' domain and the caller's preconditions are not checked (section 7.2.2) | counted, printed `covered (domain unchecked)` |
| `uncontracted-op` | the lower item is a bound operation but no assumed contract covers it | listed: unverified dependency |
| `unbound-item` | the lower item is not in the lower package's bindings (a constant, a type, the `sequential` module of `logic-gates`) | listed: unverified dependency |
| `unresolved` | a method call or other use the pass cannot resolve | listed, with file and line |
| `opaque` | a construct the pass cannot see through: `build.rs`, `OUT_DIR`, `include!`, a `#[path]` module, a macro expansion, a generic or trait-object call into a lower type | listed, counted |
| `uncontracted-helper` | an own-package function reachable from a bound operation of the upper package, by path-resolved calls, that is itself neither a bound operation nor covered by any upper claim | listed (below) |

### 8.4 The `bits.rs` example

`intel4004-gatelevel` depends directly on `logic-gates`, `arithmetic` and
`intel4004-simulator`. Its `src/bits.rs` defines `int_to_bits` and `bits_to_int`,
which convert between integers and LSB-first bit vectors using host shifts and
masks (`(masked >> i) & 1`). Those functions call nothing in `logic-gates`; they
are outside the gate contract, so no gate truth table says anything about them.
The pass reports them in two different ways:

* a call from a CPU operation to `logic_gates::*` or `arithmetic::*` is classified
  against the assumed contracts (`covered` or not);
* a call from a CPU operation to `bits::int_to_bits` is an own-package function that
  no `claim` binds: `uncontracted-helper`. The report lists it as an unverified
  internal helper and, as a triage tag only, notes that its body does host integer
  arithmetic (`&`, `>>`, `<<`, `as`).

VER27 rule (b) says such a helper "needs its own claim or removal". With the pass,
the choices become visible and checkable: either the 4004 Warrant document binds
`int_to_bits` and `bits_to_int` as operations with a round-trip claim (then the
helper is `covered` by an upper claim, and the claim is itself published as
`contract.intel4004-gatelevel.bits_roundtrip` for the layers above), or the
helper is removed, or it stays listed as unverified and `--require-closed` fails.

The pass is **not** a general "every private function needs a claim" rule. The
`uncontracted-helper` list is information; it blocks only under
`--require-closed`.

### 8.5 Staging, and what the report says before the pass exists

The pass lands after the id and discharge MVP (section 13, slice 4). Until it
does, `contracts.reach` is exactly

```json
"reach": {"state": "unchecked", "text": "reach-around unchecked"}
```

and the human report prints the phrase wherever it prints a contract strength.
What *does* exist before the pass is the dependency-level view of section 5.4
(`dependencies`, with `linkage: linked` or `unverified`): a dependency with no
assumed contract is reported even without any call analysis. After the pass,
`reach.state` is one of:

| State | When |
|---|---|
| `unchecked` | the pass has not run; the text is exactly `reach-around unchecked` |
| `call-level-partial` | the pass ran and the count of `unresolved` plus `opaque` uses is greater than 0; the report prints both counts and every listed item. Review revision: the first draft said `call-level` here, which would have let a method call into a lower type pass unseen |
| `call-level` | the pass ran and the `unresolved` and `opaque` counts are 0 |

Both non-`unchecked` states carry the SHA-256 of the pass output, so `--check`
rejects them without it (`ReachClaimUnsupported`). `build.rs`, `OUT_DIR`,
`include!`, `#[path]`, macros and generics are listed items: a package that has
any of them cannot reach `call-level` without a recorded justification per item
(open question VQ13), and a `--require-closed` run fails on `call-level-partial`.

### 8.6 A dynamic complement (proposal, not committed)

A static pass has the limits above. A sound dynamic complement exists because the
functional contracts are executable Warrant models: run the upper package's own
claims with each lower operation swapped, at the binding, for its Warrant model
(or for a deliberately different implementation that satisfies the contract and
differs everywhere else). If the upper claims still pass, the upper layer relies
only on the contract. If they fail, it was reaching around. This is the real
test of assume-guarantee composition and the only one that does not depend on a
lexer. It needs a binding-level injection point in the generated harness (the
mechanism VER04h section 6 uses for faults). Whether it is worth building is open
question VQ6; it is listed here because it changes what Q8 (section 14) can
conclude.

---

## 9. Rule (c): exact linking

* **No fuzzy matching.** An id resolves by string equality of every segment
  against the lower `.warrant` (section 3.2). Case matters. No prefix, edit
  distance, alias, wildcard or "closest claim" resolution exists, and a diagnostic
  may list *all* of the lower package's claims for the reader but never proposes
  one.
* **Renaming a claim breaks the link loudly.** Renaming `xor_truth_table` makes
  every upper link to it `ContractUnknownClaim`, an error that fails
  `--generate` and `--check` for each assuming package, in the same CI run that
  contains the rename.
* **Exact means the claim, not only its name.** The pin `claim_sha256` records the
  text of the claim the upper layer was generated against. If the lower author
  keeps the name and changes the formula, the pin differs, `--check` fails with
  `ContractPinMismatch` naming `claim_sha256`, and regenerating the upper report
  shows the change as one line for review. Regeneration is the explicit act of
  accepting the new claim.
* **The text of an assumption is the lower formula, not a paraphrase.** The upper
  `.warrant` contains the id only. LG01 states the obligation ("the text of an
  upper layer's assumption must be that formula"); the id and `claim_sha256`
  enforce it, because the upper author has nowhere to write a different one.

### 9.1 No deprecation or alias mechanism

**Decision: there is none.** Reasons:

* An alias is two names for one claim, which is the paraphrase hazard rule (c)
  exists to remove. It also makes the report ambiguous about which text an upper
  layer relied on.
* This is a monorepo and the change is atomic. A rename is one commit that edits
  the lower `.warrant`, edits each assuming `.warrant` and regenerates their
  reports; the CI of section 11 enforces that all of it is in the same change.
* The set to edit is small and findable: only the packages whose `.warrant`
  contains the id, and the build tool can list them (section 11).

A deprecation window (keep the old name resolving for a release) is the one case
this decision would not serve, and it arises only when packages are versioned and
published independently, which is a non-goal here (section 15; open question VQ8).
Removing a claim is the same operation as renaming it: uppers break loudly.

---

## 10. The dependency DAG and transitivity

* **Contracts follow dependencies.** A link from upper U to lower L is legal only
  if L is a direct normal dependency of U (condition a). The contract graph is
  therefore a subgraph of the package dependency graph, acyclic when that graph is.
* **Cycles are defended against anyway.** Cargo forbids cyclic normal
  dependencies, but the other lanes' manifests and dev-dependency edges may not.
  Resolution keeps a stack of packages being resolved, depth-capped at 64; re-entry
  is `ContractCycle`. A package can never link to itself (`ContractSelfLink`).
* **Only direct dependencies.** Decision, matching condition (a): layer 3 cannot
  name a layer 1 contract unless layer 3 directly depends on layer 1. A reliance
  that goes through layer 2 is expressed by layer 3 assuming a layer 2 contract,
  whose effective strength already contains layer 1's.
* **A direct dependency that is also reachable transitively may be named.**
  `intel4004-gatelevel` depends directly on both `logic-gates` and `arithmetic`
  (and `arithmetic` depends on `logic-gates`), so it may assume contracts of both.
  The rule is the edge, not the distance.
* **Why not "any contract below you".** A transitive name would let a package
  assume a lower layer it has no declared connection to, skipping the layer whose
  contract is supposed to mediate it; that is the reaching around of rule (b)
  moved into the assumption list.
* **Failure propagates up the chain.** If layer 1's evidence changes, layer 2's
  pins mismatch and layer 2's report fails `--check`; layer 3 reads layer 2's
  report as condition (c), so layer 3's link is open with
  `lower-report-fails-check` until layer 2 is regenerated. Nothing in the middle
  can be skipped.
* **Cost of the recursion.** Condition (c) runs the whole check of the lower
  report, which runs its own. Results are memoised per process by lower package
  path, so a diamond is checked once.

---

## 11. CI and cost

### 11.1 What has to be re-selected

The build tool (`code/programs/go/build-tool`) maps changed paths to packages by
prefix and selects dependents through the package graph; Cargo `[dependencies]`
already give it every `logic-gates` dependent. Two things are not covered:

1. `code/specs/<package>.warrant` is **outside** the package directory, so editing
   the Warrant document of `logic-gates` selects nothing today. LV-002 is the row
   that adds a consumer rule for adopters' `.warrant` files
   (`repository_inputs.go`, the table that already carries
   `rust/warrant-grammar`). Contract linking extends that rule and depends on it.
2. The re-check an upper package needs after a lower change is the *link check*
   (`ca-assurance-report --check`, which hashes files and re-derives), not a full
   rebuild with the test suite.

### 11.2 The rule

Let a **link edge** U -> L exist when `code/specs/U.warrant` contains a
`contract.L.*` id. The build tool derives the set of link edges at start-up by
scanning the `.warrant` files (a grep over a few dozen files). Then:

| Changed path | Selected |
|---|---|
| `code/specs/L.warrant` | every U with an edge to L, in every lane where U and L both exist: the **link check** of U |
| `code/packages/x/L/evidence/**`, `code/packages/x/L/evidence.bindings`, `code/packages/x/L/assurance_report.json` | the same |
| `code/packages/x/L/src/**` and anything else under `L` | already selected through the Cargo dependency graph with a full build of every dependent; the link check runs inside U's BUILD as it does today for `--check` |
| `code/specs/U.warrant` | U (full), plus every package with an edge from it upward |
| a path in a lower package's pinned **unverified path dependency** (for `logic-gates`, `code/packages/rust/transistors/**`), or the lower `Cargo.toml` or `build.rs` | the link check of every U whose pins list it (section 5.3: `unverified_deps`, `cargo_sha256`, `build_rs_sha256`) |
| the **generator, prelude or report tooling**: `code/packages/rust/warrant-grammar/src/` (`evidence.rs`, `evidence_prelude.rs.txt`, `codegen.rs`, `binding.rs`), `code/packages/rust/ca-assurance-report/**`, `code/packages/rust/ca-capability-analyzer/**`, the slice-3 gate script | **all** link checks, every U with any link edge: these change what a manifest or a report means, so no pin comparison can be trusted across the change |

A lower change selects uppers **transitively along link edges**: L's change
selects U, and U's report is the lower report that V (an upper of U) checks, so V
is selected too. Because the check is cheap (it hashes and re-derives, it does not
run tests), the transitive link-check selection is the right shape; the full
rebuilds follow the Cargo graph as before.

### 11.3 Cost for `logic-gates`

About 47 `Cargo.toml` files name `logic-gates`. Two cases:

* **A source edit in `logic-gates`** already selects its 47 dependents through the
  Cargo graph (full build, with their tests), contract linking or not. Linking adds
  a link check only for the dependents that assume a contract, which is one to
  three at first. Marginal cost: negligible.
* **A `.warrant`-only edit** (a claim added, a formula tightened) selects nothing
  today. A naive rule, "select every dependent when a lower `.warrant` changes",
  would build 47 packages and their test suites to learn that one assumes a
  contract. The rule above selects the link check of the assuming packages only.
  This is why the link edges are derived from the `.warrant` files rather than from
  Cargo edges, and why the first implementation should resist the shortcut.

The pin granularity has a cost the spec accepts and flags: `source_sha256` covers
every file under the lower `src/`, so a comment edit in `logic-gates` changes it and
forces a regeneration of every assuming report (open question VQ7 asks whether
a per-claim slice would be enough).

---

## 12. Worked example: layer 2, `arithmetic`, over layer 1, `logic-gates`

This is an illustration. `arithmetic` has no Warrant file today, and the file
below is written to show the mechanism, not to be copied; the syntax follows the
`logic-gates.warrant` of PR #19252 (`warrant logic_gates v1`). The real file lands
with LV-006.

### 12.1 What layer 1 publishes

`logic-gates` publishes, among 88 claims, these three (exhaustive cases from
LG01, `Bool set(2)` for each binder):

| Id | Operation | Exhaustive cases | Own strength |
|---|---|---|---|
| `contract.logic-gates.xor_truth_table` | `xor_gate` | 4 | `exhaustive` |
| `contract.logic-gates.and_truth_table` | `and_gate` | 4 | `exhaustive` |
| `contract.logic-gates.or_truth_table` | `or_gate` | 4 | `exhaustive` |

Each has an exhaustive, a property (4,096 cases) and a fuzz (5,000 iterations)
suite. The logic-gates report has no contract links of its own (nothing below it is
a Warrant package: it depends on `transistors`, which has no `.warrant`), so it
lists `rust/transistors` under `dependencies` with `linkage: "unverified"`.
Every logic-gates functional claim therefore prints as `exhaustive, conditional on
1 unverified dependency` (rule C2, section 7.2.1: functional strengths print with
the condition; a timing strength would be withheld); that is correct, and it is the
first thing a reader of the chain learns. The label `exhaustive` also assumes
`logic-gates`' bindings declare `state none` (section 6.2); until they do, the
classifier reads the claims as `exhaustive-sampled`.

### 12.2 The upper `.warrant` (sketch)

```text
warrant arithmetic v1
  model b2u(x: Bool) -> U8 = if x then 1u8 else 0u8
  model total3(a: Bool, b: Bool, c: Bool) -> U8 = b2u(a) + b2u(b) + b2u(c)
  error AdderError = INVALID_INPUT

  op half_adder(a: U8, b: U8) -> Result<(U8, U8), AdderError>
  op full_adder(a: U8, b: U8, cin: U8) -> Result<(U8, U8), AdderError>
  op add4(a: U8, b: U8, cin: U8) -> Result<(U8, U8), AdderError>

  claim half_adder_sum_carry:
    forall a: Bool, b: Bool. half_adder(b2u(a), b2u(b)) ==
      Ok((b2u(a != b), b2u(a and b)))

  claim full_adder_sum_carry:
    forall a: Bool, b: Bool, c: Bool. full_adder(b2u(a), b2u(b), b2u(c)) ==
      Ok((rem(total3(a, b, c), 2u8), if total3(a, b, c) >= 2u8 then 1u8 else 0u8))

  -- 4-bit ripple-carry: the binding adapter turns a nibble into LSB-first bits,
  -- calls ripple_carry_adder_with_carry and turns the bits back.
  claim add4_modular_sum:
    forall a: U8, b: U8, c: Bool | a < 16u8 and b < 16u8. add4(a, b, b2u(c)) ==
      Ok((rem(a + b + b2u(c), 16u8), if a + b + b2u(c) >= 16u8 then 1u8 else 0u8))

  assume platform:
    contract.logic-gates.xor_truth_table
    contract.logic-gates.and_truth_table
    contract.logic-gates.or_truth_table
  assume environment:
    none
```

Nothing in `arithmetic`'s `.warrant` says what an XOR is. It proves how the half
adder, the full adder and the ripple chain compose gates. The nibble-to-bits
conversion lives in the binding adapter, which is harness code the claim itself
exercises (an adapter bug surfaces as a failing claim); production callers such as
`intel4004-gatelevel` do their own conversion, and `bits.rs` is the case section
8.4 treats.

### 12.3 The 512-case adder claim

`add4_modular_sum` enumerates the binders as `a: U8 full(256); b: U8 full(256);
c: Bool set(2)`: 131,072 combinations before the refinement, under VER04h's
400,000 budget. The refinement `a < 16u8 and b < 16u8` admits `16 x 16 x 2 = 512`
and discards 130,560. The manifest entry:

```json
{"claim": "add4_modular_sum", "kind": "exhaustive", "op": "add4",
 "cases": 512, "discarded": 130560,
 "domain": "a: U8 full(256); b: U8 full(256); c: Bool set(2)",
 "id": "warrant/arithmetic/exhaustive/add4_modular_sum", "verdict": "pass"}
```

All three binders are complete finite types, so the own strength is `exhaustive`,
and the claim is over every nibble pair and carry-in: the 512 cases of VER27
section 4.15, "(a+b+c) mod 16".

### 12.4 The upper report rows

Contract links (`contracts.assumed`), which are the same before and after the
reach pass:

| Link | Status | Strength | Lower evidence | `limited_by` |
|---|---|---|---|---|
| `contract.logic-gates.xor_truth_table` | discharged | `exhaustive` | 3 suites, all `pass` | none |
| `contract.logic-gates.and_truth_table` | discharged | `exhaustive` | 3 suites, all `pass` | none |
| `contract.logic-gates.or_truth_table` | discharged | `exhaustive` | 3 suites, all `pass` | none |

`dependencies`: `rust/logic-gates` is `linked`. Claims, **before the reach pass**
(slice 2; section 6.4: no contract strength is taken until uses are attributed):

| Published claim | Own | Strength state | Printed |
|---|---|---|---|
| `half_adder_sum_carry` | `exhaustive` (4 cases) | `unattributed` | `unattributed (own: exhaustive)`; `conditional_on`: inherited unverified `rust/transistors` via the three links; `reach-around unchecked` |
| `full_adder_sum_carry` | `exhaustive` (8 cases) | `unattributed` | as above |
| `add4_modular_sum` | `exhaustive` (512 cases) | `unattributed` | as above |

The operation rows of the report (`half_adder`, `full_adder`, `add4`: one row per
operation, not per claim, section 5.4) show the minimum over their claims, here
`unattributed (own: exhaustive)`.

**After the reach pass** (slice 4), each claim is attributed to the lower
operations it uses (`half_adder` uses `xor_gate` and `and_gate`; `full_adder`
also `or_gate`; the ripple chain uses all three). Each use is `covered (domain
unchecked)` by the matching link, so:

| Published claim | Uses | Strength | `conditional_on` |
|---|---|---|---|
| `half_adder_sum_carry` | `xor_gate`, `and_gate` | `exhaustive` (`min(exhaustive, exhaustive, exhaustive)`) | inherited unverified `rust/transistors` |
| `full_adder_sum_carry` | `xor_gate`, `and_gate`, `or_gate` | `exhaustive` | as above |
| `add4_modular_sum` | the same three | `exhaustive` | as above |

printed as `exhaustive, conditional on 1 unverified dependency (inherited)`, with
`reach` `call-level` only if no use is `unresolved` or `opaque`, else
`call-level-partial`.

**The same claims if `arithmetic` had assumed only `not_truth_table`** (review item
C1): the links are discharged, but `xor_gate`, `and_gate` and `or_gate` are
`uncontracted-op`. Each adder claim keeps its own strength, prints
`conditional_on` the three uncontracted operations, and takes **no** strength from
the `not` link; the dependency-level value `linked` is true and tells the reader
nothing about the adders.

**Caller domain.** `add4_modular_sum` is proved over nibbles (`a < 16u8`). If
`intel4004-gatelevel` calls the same export on 12-bit values, the use is
`covered (domain unchecked)`. Nothing compares the 12-bit values with the claim's
refinement (section 15, limit 9; question VQ12).

If `logic-gates` had only a `randomized` truth-table suite for `xor_truth_table`,
that link would be discharged at `randomized`, and after slice 4 every claim that
uses `xor_gate` would be capped at `randomized` (`limited_by` naming the link),
while a claim that uses only `and_gate` would not be.

### 12.5 If `logic-gates`' manifest changes

| Change in `logic-gates` | `arithmetic` `--check` | `arithmetic` `--generate`, and the CI regression gate |
|---|---|---|
| An unrelated claim is added; manifest and `.warrant` are regenerated | `ContractPinMismatch`: `spec_sha256` and `manifests.test` differ; the message adds that `claim_sha256` and `claim_entries_sha256` of the three pinned claims are unchanged | Succeeds; the diff shows only the changed pins. Strength unchanged, so the gate (section 5.3.1) passes with no trailer. |
| `xor_truth_table` is renamed `xor_table` | (cannot get that far) | `ContractUnknownClaim`, naming the package and listing its claims; no alias is tried |
| The `xor_truth_table` formula is tightened, name unchanged | `ContractPinMismatch` including `claim_sha256` | Succeeds, but the gate **fails** (`claim_sha256` changed) unless a commit carries `Contract-Change-Accepted: contract.logic-gates.xor_truth_table: <reason>` |
| The exhaustive suite for `xor_truth_table` is removed; property and fuzz remain | `ContractPinMismatch` (manifest digest) | Succeeds with strength `randomized`; the gate **fails** (strength decreased) without the trailer, so a downgrade cannot be regenerated into a green build unseen |
| An exhaustive suite for `xor_truth_table` fails (verdict `fail`) | `ContractDischargeMismatch` if the checked-in report still says discharged | The link is `open`, reason `suite-failed` (and `lower-report-fails-check` in `also`); the gate fails (discharged to open) without the trailer; the claims print `conditional on 1 open contract` |
| `transistors`, which `logic-gates` stands on without a contract, is edited | `ContractPinMismatch` on `unverified_deps` | Succeeds; the gate passes. The change is visible in the diff of the pin. |
| `logic-gates` is edited by hand so a manifest entry claims `pass` | `ContractPinMismatch` for the pinned digest; if the upper report was also regenerated, `lower-report-fails-check`, because the lower `--check` finds `source_sha256` or `spec_sha256` stale (VER04h section 9, check 5 and 12) | Open, `lower-report-fails-check` |
| `arithmetic` removes `logic-gates` from `[dependencies]`, or points it at a fork by `path` or `package =` | `ContractNotDependency` | `ContractNotDependency` |
| `arithmetic` enables a feature on `logic-gates` that its evidence build did not | (valid report) | The links are `open`, reason `build-config-unpinned` |

---

## 13. Implementation slices

Each slice is one PR with its own spec edits (section 16), tests and negative
controls. The order below is the brief's. Would-build counts are estimates from the
Cargo path edges and consumer rules on `origin/main`, with the three Rust
adopters (`ct-compare`, `gf256`, `wasm-leb128`) and `logic-gates` once PR #19252
merges counted as the packages whose BUILD runs the report tools. Re-measure with
the build tool's plan before opening each PR; the estimate is a planning
number, not a promise.

| Slice | Scope | Would-build (estimate) |
|---|---|---|
| 1. Id parsing | `warrant-grammar`: contextual `contract_id` at a platform row (section 3.3), shape validation (`ContractIdMalformed` at parse), duplicate diagnostic, v0 rejection (`W0101`); fixtures in `code/specs/fixtures/warrant-syntax-v1/`; VER04 amendment A | about 7: `warrant-grammar`, its Cargo dependents `ca-assurance-report` and `warrant-ir`, and the four adopters re-selected by LV-002 for a grammar change |
| 2. Resolution and discharge | `ca-assurance-report`: read upper `Cargo.toml` dependencies; resolve against the lower `.warrant`, test manifest and report; conditions a-e including the in-tree binding of section 5.1.1; pins including `Cargo.toml`, `build.rs`, unverified path dependencies and toolchain; the lower-report check with carried `unverified::` ids (section 5.1.2); `contracts` block; strength labels; `unattributed` state; the `state none` binding key (section 6.2); `published`; `dependencies` and `external`; checks of section 5.2; flags of section 5.6; VER13 amendment B. Timing, resource and capability ids resolve to `kind-not-yet-published`. | about 5: `ca-assurance-report` and the four adopters whose reports regenerate (each gains a `contracts` block and a `published` list); the first real upper package (`arithmetic`) joins with LV-006 |
| 3. CI re-selection | build-tool consumer rule of section 11.2 (link-edge scan, `.warrant`, `evidence/**`, pinned-dependency and generator-change mapping), on top of LV-002; `contract_regression_gate` and the `Contract-Change-Accepted` trailer (section 5.3.1); conformance test script | 1 Go package plus the build-tool conformance scripts; verify that a build-tool edit does not force a wider rebuild before relying on this number |
| 4. Reach-around check | `ca-capability-analyzer`: `--emit-uses` mode resolving through the lower crate's module tree and `pub use` (section 8.2); `ca-assurance-report`: classification of section 8.3 including `opaque`, per-claim attribution, `reach.state` `call-level-partial` or `call-level` with pass digest, `--require-closed` | about 6: the analyzer, `ca-assurance-report`, the four adopters. The mode is additive, so no other package's BUILD output changes (a requirement of the slice, with a test). |
| 5. Amendments and closure | the parts of the amendments not already landed in slices 1 and 2; VER27 C14 closed and the section 4.14 pointer; VER01 `contracts` schema; backlog; LG01's "pending LV-005" header; `README` | docs only; `VER04` is a `warrant-grammar` input (about 3 selected) |

The brief listed the amendments last. Applying each amendment in the slice whose
code makes it true (VER04 with slice 1, VER13 with slice 2) avoids a window in
which normative text and implementation disagree; slice 5 keeps the rest (section
17).

### 13.1 Tests and negative controls

Every rule has a positive and a negative test, in the style of VER13's
`tests/test_assumptions.rs`. A negative control that passes is a defect of the
gate, as in VER04h section 6.

**Slice 1**

* Positive: the three adopters' documents parse and check byte-identically
  (VER04j section 1.3); a v1 document with each id shape of section 3.1 parses.
* Negative: `contract.logic-gates` (three segments); an empty segment; an
  an id carrying a lane segment (`contract.rust.logic-gates.x`); a 257-byte id; an id with a space inside; `contract..x`;
  a contract id in a v0 document (`W0101`); the same id twice (duplicate
  diagnostic); a model with a binder named `contract` that projects a field still
  parses as an expression.

**Slice 2**

* Positive: the worked example of section 12 resolves, with strengths
  `exhaustive`; a `randomized`-only lower caps the upper claims; a three-layer
  chain composes (layer 3 reads layer 2's `published` rows).
* Negative controls, one test each: a **forged manifest** (a lower manifest edited
  to say `pass`, or to add an exhaustive suite) fails `lower-report-fails-check`
  or `ContractPinMismatch`; a **digest swap** (the upper report's pins replaced with
  another package's digests, or with each other) fails `ContractPinMismatch`; a
  **missing dependency** (the contract names a package absent from `[dependencies]`,
  and one present only in `[dev-dependencies]`) fails `ContractNotDependency`; a
  **renamed claim** fails `ContractUnknownClaim`, and a near-miss name (`xor_table`
  for `xor_truth_table`) is not resolved; a **sabotaged lower suite** (the lower
  manifest has a `fail` entry, then a failing `negctl`) leaves the link open with
  `suite-failed` and `lower-fault-mode`, and a report that still says `discharged`
  fails `ContractDischargeMismatch`; a lower run with `WARRANT_EVIDENCE_FAULT` set
  produces no manifest, so the link is open with `lower-evidence-absent`; the
  generator refuses to run with the fault variables set; a **strength overclaim**
  (an upper claim row labelled `exhaustive` over a `randomized` link) fails
  `ContractStrengthOverclaim`; a **kind overclaim** (a timing strength resting
  only on functional links) fails `ContractKindOverclaim`; a **cycle** between two
  fixture packages fails `ContractCycle`; a **self-link** fails as named, and an id whose package exists only in another lane fails `ContractUnknownClaim`; a **sampled domain** (`n: Nat set(65)`, `v: U64 edge(377)`) is
  `exhaustive-sampled`, never `exhaustive`; unparseable `domain` text falls to
  `exhaustive-sampled`.
* **Binding controls (C4):** a `path` to a copy of the lower crate outside
  `code/packages/<lane>/<package>`; a `package =` rename; a registry or git
  dependency; a `[patch]` entry; an `optional = true` entry; a `[target.*]` entry; a
  `features =` key (open with `build-config-unpinned`); a `build.rs` or `Cargo.toml`
  edit in the lower crate changes the pin.
* **Attribution controls (C1, C2):** an upper package that assumes one contract of a
  dependency and uses a different operation has `unattributed` strength and prints no
  contract label; a functional claim over a contract-less dependency prints with
  `conditional_on`, a timing claim over one is withheld (also for an external crate
  and for `std`); an operation row never reads stronger than the weakest of its
  claims (a fixture with 6 operations and mixed-domain claims).
* **Lower-report controls (H1):** a lower report with `unverified::` ids yields a
  `conditional_on` entry for each; a report that drops one fails the check.
* **Label controls (H2, H3):** `bounded-proof(k=128)` ranks below `exhaustive` and
  always prints `k`; an operation with neither `state none` nor `state=explicit`
  (with the claim's state set complete and the VER04m E15 obligations passed), an
  operation executed by a driver, and a claim over a bounded set of call sequences,
  are at most `exhaustive-sampled`.
* **Downgrade control (H4):** regenerating after a strength decrease, a
  discharged-to-open change or a `claim_sha256` change fails the gate without the
  trailer and passes with it.
* **Pin controls:** an edit to `transistors` changes `unverified_deps`; a different
  toolchain id fails the check.
* Property tests: effective strength never exceeds own strength or any cap
  (for random chains); regenerating a report is a fixed point.

**Slice 3**

* Positive: an edit of the generator, prelude or report tool selects every link
  check; a change under a pinned unverified dependency selects the assuming link
  checks; a `.warrant` edit of a lower selects the link check of exactly the
  assuming packages and not its 47 dependents; an `evidence/**` edit selects the
  same; a source edit selects dependents through the graph as before.
* Negative: the build tool fails closed (selects everything it cannot classify)
  when a `.warrant` it scans has a malformed id; a link edge to a package that does
  not exist is an error, not an omission; a changed path under a lookalike
  directory (`logic-gates2`) selects nothing.

**Slice 4**

* Positive: `arithmetic`'s `adders.rs` uses resolve to `covered (domain
  unchecked)` for `and_gate`, `or_gate`, `xor_gate` (through `gates::`, one
  definition with the root re-export); the `intel4004-gatelevel` fixture lists
  `bits::int_to_bits` as `uncontracted-helper`.
* Negative controls: an upper operation calling a lower operation with no assumed
  contract is `uncontracted-op`; a use of an unbound item (`MAX_DECODER_WIDTH`) is
  `unbound-item`; a method call on a lower type is counted `unresolved`; a report
  claiming `call-level` or `call-level-partial` without the pass digest fails
  `ReachClaimUnsupported`; a glob import and an alias rename of a lower function are
  still resolved (the analyzer's own corpus cases are reused); a same-named function
  in another module of the lower crate is **not** matched; `build.rs`, `include!`,
  `#[path]` and a macro-expanded call are `opaque` and force `call-level-partial`;
  `--require-closed` fails on each listed class and on `call-level-partial`.

---

## 14. Q8: does composition replace a full-netlist miter?

VER27 Q8: with each layer proved independently and linked by contract, is the
top-level proof for the RV32I path (VER13 section 4.4, rung R3) only a proof of
composition, so the full-netlist SAT miter is no longer needed? The owner has said
it can be; the decision waits for the 4004 chain to show the method on a small
stack. This section answers as far as a specification can, honestly.

**What this specification gives.** The mechanics of linking, and a label that
makes some preconditions checkable. It does not, by itself, make a composition
argument sound.

**Conditions under which composition replaces the miter** (all are necessary):

1. **Every contract in the chain is `proved` or `exhaustive` (a complete
   domain, stateless deterministic binding), and none is `bounded-proof(k=K)`,
   sampled or randomized.** A miter is a proof over all inputs. A bounded proof
   (rank below `exhaustive`, section 6.1) is not enough: it says nothing past `K`.
   Section 6's minimum rule means a single `bounded-proof`, `randomized` or
   `exhaustive-sampled` link anywhere caps the top claim below the standard a miter
   meets, so the label alone refuses
   the replacement when it is not earned. For gates and a 4-bit adder
   (4 and 512 cases) this holds; for a width-64 datapath it does not, until the
   contract is a proof (a bit-blasted or inductive one), not an enumeration.
2. **The composition is itself proved, not tested.** Today the upper layers'
   evidence is a test run against the real lower crate (section 1). That is
   evidence that the composed code is right on the enumerated cases, not a proof
   about composition. Replacement needs an upper-layer proof in which each lower
   operation is an abstract function constrained only by its contract, discharged
   by a proof engine (VER09 to VER11, VER19) on the upper code. For most packages
   the WIR front end refuses the source (`not-live`), so this does not exist yet.
3. **No reaching around, at call level, and not only for calls.** Section 8's
   pass must be `call-level` and clean, including the unverified helpers
   (`bits.rs`): a host integer routine on the path is outside every contract, and
   an abstract proof of the composition says nothing about it. The dynamic
   substitution run of section 8.6 would be the strongest evidence of this.
4. **The coverage kinds are all present.** For the RV32I constant-time claim the
   miter compares observation traces. A composition proof needs a *timing*
   contract at every layer ("one evaluation per call regardless of data") **and** a
   proof that the glue between components is secret-independent (branches, array
   indices, step counts). Functional contracts say nothing about either. Timing
   composition is not implied by functional composition, and VER20's
   self-composition check is exactly what a composition proof would have to repeat
   for the glue.
5. **Stateful layers have trace contracts.** Flip-flops and registers need a
   step-relation contract and a statement of the clocking abstraction
   (`gate.sequential.ideal-dff`). A truth table covers combinational blocks only.
   (Amended 2026-11-14.) The condition is met by a **step claim over explicit state**
   whose state set is complete (VER04m reach class R1 or R2) **plus a closure row**
   (every post-state of the step lies in the invariant), both evidenced and printed;
   with the hidden-state obligations of VER04m E15 passed, the step claims and the
   closure imply by induction that every finite trace agrees with the model, an
   inference the report does not print before VER04m section 7.2 is enabled. A bounded
   set of traces never meets it.
6. **Every caller satisfies each contract's refinement and domain** (review item
   C1). A contract proved over nibbles says nothing about a 12-bit use, and `covered`
   does not check it (section 7.2.2). A composition proof needs, for each use, a
   proof that the actual argument lies in the contract's domain and meets its
   preconditions.
7. **The evidence and the composition use the same build configuration and
   binary.** Features, `cfg` flags, `build.rs` output, toolchain and optimisation
   level that differ between the lower evidence build and the artefact the
   composition is about make the contract a statement about a different program
   (section 5.1.1 pins the manifest-level part; the binary-level part is not pinned).
8. **The code is pure in its explicit state, and deterministic.** No clock, random
   source, global state or hidden history: a contract over a stateful operation proved
   from one initial state (section 6.2) does not compose. (Amended 2026-11-14.) An
   operation whose whole state is a value the harness builds and reads (`state=explicit`)
   is pure in that sense **when state completeness is evidenced** (VER04m E15, with
   controls); an operation executed by a driver is not.
9. **Panics and rejections are part of the contract actually used.** The gate
   contract says a non-bit panics, normalised by the binding to `INVALID_INPUT`; an
   upper layer that relies on a result must also account for the rejection path of
   every operation it calls, and the contract used must be the one that states it
   (for `logic-gates`, `*_rejects_non_bit` is a separate claim from `*_truth_table`).
10. **The netlist is the composition.** A miter is about one concrete netlist; a
   composition argument is about a model assembled from contracts. Replacing the
   first by the second needs a claim that the netlist (or the simulator the
   netlist is traced from, VER13 section 4.4) is that assembly: an extraction
   fidelity claim. No contract in this specification states it, and no tool checks
   it.

**What remains assumed even when all ten hold:** the host language and toolchain
(the evidence runs compiled code, VER27 section 5); Warrant and its evidence
generator (`tcb`); the datasheet transcription at the CPU layer; the soundness of
the abstraction step (that an abstract function with only the contract's constraints
over-approximates the real operation, which for an exhaustively enumerated finite
operation is exact and for a sampled one is not); and the L0 physical floor. A
contract is evidence about an *implementation in a simulator*, not about silicon.

**Verdict.** The answer the specification can support is **conditional, and
currently not met for RV32I**: composition may replace the miter for a layer whose
contracts are `exhaustive` or `proved`, whose composition is proved with abstract
lower operations, and whose reach-around check, caller domains, build
configuration, purity, rejection paths and trace/timing coverage are clean
(conditions 1 to 10). The 4004 chain meets condition 1 for its lowest layers
at most (and only once their bindings declare `state none`, or `state=explicit` with
the VER04m obligations evidenced for the sequential layer), and none of 2 to 10. Recommendation, not a decision (open question VQ4, for
the owner): when the chain has a composition proof for one small layer (the 4-bit
adder over the gates), **compute the miter for the same layer and compare the two
verdicts**. That calibrates Q8 on a stack where both are cheap, which is what
VER27 asked the chain to do.

---

## 15. Non-goals and honest limits

**Not goals.**

* Cross-lane links (a Perl package assuming a Rust contract through a cdylib). The
  resolution is an error (`ContractUnknownClaim` or `ContractNotDependency`). It is a real shape in this repository and deserves its
  own design.
* Packages published and versioned independently (a registry release of the lower
  package). Digests identify checked-in files, and the resolution reads sibling
  directories of one checkout.
* A new keyword, a module system or imports. A contract entry is a reference by id
  (VER04 section 2 is unchanged in substance).
* Publishing examples, `labels` rows or `input` rows as contracts.
* Changing any existing report field or label. `proof.label`, the VER13 ladder and
  the catalogue keep their meaning; strength is a separate axis.
* Replacing `gate.primitives.correct`. Whether the catalogue entry becomes
  `discharged_by` the set of gate truth-table contracts is a question (VQ3), not a
  consequence.
* Proving anything about real silicon, the transistors, or the datasheet.

**Honest limits.**

1. **Discharge is evidence about a checked-in tree, not a proof.** A fully
   consistent forgery (a manifest with a valid hash, a matching `source_sha256`,
   fabricated test results) is not detected by anything in this specification;
   what catches it is that CI regenerates every manifest from a real run and a
   dirty tree fails (VER04h section 8). WD-009 ("a hand-written manifest with a
   valid but unrelated certificate", VER04h section 10) is the same class for
   proofs.
2. **Strength is only as good as the classification.** `exhaustive` is conservative
   (section 6.2) and depends on a free-text `domain` until the manifest gains
   structured binders.
3. **Before the reach pass no contract strength is taken at all** (section 6.4: the
   state is `unattributed`); attribution needs section 8's pass, and the pass is
   lexical (limit 4).
4. **The reach pass is lexical.** Method calls, trait objects, function pointers,
   macros, `build.rs`, `OUT_DIR`, `include!` and `#[path]` are not resolved; they are
   counted as `unresolved` or `opaque` and keep the state at `call-level-partial`.
5. **A dependency is not a use.** Condition (a) says the upper package is allowed to
   link to the lower; it does not say the upper code calls it, and a link to a
   dependency the code never touches is harmless but uninformative.
6. **Test evidence at an upper layer is not independent of the lower
   implementation** (section 1). It is conditional on it, and discharge makes the
   conditionality visible.
7. **Only the Rust lane is live.** Every other lane resolves to
   `lane-not-supported` or `lower-evidence-absent`; the id space, the pins and the
   rules are lane-neutral, the readers are not.
8. **`unverified dependency` includes `std` and third-party crates.** They are listed
   as external and carry no contract; the VER13 `src` and `cc` families are where
   their assumptions live.
9. **Caller-side refinements and domains are never checked.** `covered` means the
   operation used has a contract. A claim proved over nibbles (`a < 16u8`) is
   `covered` for a 12-bit caller; nothing compares the caller's argument domain with
   the contract's refinement, and the preconditions a contract states by `rejects`
   rows are not checked against the caller (section 7.2.2; VQ12).
10. **`proved` is not exhaustive evidence of the build you ship.** Features, `cfg`,
    `build.rs` output and the binary are pinned only at manifest level
    (section 5.1.1); the binary the composition is about is not.
11. **An assumed claim's own suite is not shown able to fail**, only that some claim
    of its operation catches each fault (section 5.5, VQ11).

---

## 16. Amendment

This section is the exact text the implementation PRs apply. VER04 and VER13 are
**not** edited by this specification.

### 16.1 VER04 section 3 (slice 1)

Replace the final paragraph's sentence

> Platform entries must resolve to the shared versioned VER13 catalogue and its
> lane conformance evidence; packages cannot invent private platform axioms.

with

> Platform entries are of two forms. A *catalogue entry* is a dotted identifier
> that must resolve to the shared versioned VER13 catalogue and its lane
> conformance evidence. A *contract entry*, allowed in v1 documents only, is a
> contract id (`contract.<package>.<claim>` or a longer form of VER29
> section 3.1; no lane segment, the lane is that of the report that resolves it)
> naming a claim that another package's own Warrant document declares. The Warrant checker validates a contract entry's shape only. The
> assurance report resolves it from the named package's evidence, fail closed
> (VER29 section 5): the entry counts as discharged only if that evidence proves
> the claim at the pinned digests, and a name that resolves to nothing is an
> error, not an open assumption. A package still cannot invent a private platform
> axiom: a contract entry cannot name a statement the declaring package wrote
> itself, and nothing the named package's document defines becomes visible to this
> module's formulas.

Add to section 2, after "Imports, includes and host paths do not exist in this
profile.":

> A contract entry in `assume platform` (section 3) is a reference by id, not an
> import: it adds no name to the module's symbol namespace.

Grammar (`warrant.grammar`, `platform_row`):

```text
platform_row = qualified_name NEWLINE | contract_id NEWLINE ;
```

with `contract_id` recognised only at a platform-row position (VER29 section 3.3).
A `contract_id` in a v0 document is `W0101`.

Section 10 (change record), new bullet: "VER29: contract entries in `assume
platform` (v1 only); a platform id may now resolve to a generated `contract`
family id."

### 16.2 VER13 (slice 2)

Section 5.2, the `family` row of the field table: the set becomes

> `spec|src|wir|cc|mc|uarch|gate|phys|tcb|os|gov|att|contract`

and, after the field table, a new paragraph:

> `contract` is a **generated** family. No entry of it appears in
> `catalogue-v1.json`; an id of the family is valid in a report if and only if
> VER29 section 5 resolves it from the named package's `.warrant` and evidence.
> It has no layer, no status and no `discharge` plan in this catalogue's sense.
> AC-14's counts and spec-sync test are over the stored entries and do not change.

Section 6, rule edits:

| Rule | Edit |
|---|---|
| AC-1 | "...is neither an `id` nor an alias in the pinned catalogue" becomes "...is neither an `id` or alias in the pinned catalogue nor a `contract.*` id that VER29 section 5 resolves (shape valid; package, lane and claim declared)". |
| AC-3 | Add: "For a `contract.*` id, `evidence` is the list in the report's `contracts` block: each id must exist in the named lower package's manifest with verdict `pass`, and each pinned digest must equal the digest on disk." |
| AC-4 | Add: "The closure includes every contract id in the package's Warrant `assume platform` block, for every claim." |
| AC-5, AC-8, AC-10, AC-13 | Add: "A `contract.*` id has no layer and no status, so this rule does not apply to it." |
| (new) AC-16 | "**Contract links.** A `contract.*` id in a report fails with the named failure of VER29 section 5.2 when it is malformed, names a package that is not a direct dependency, names a claim the lower document does not declare, has a pin that no longer matches, is shown discharged when its re-derivation is open, or is labelled stronger than VER29 section 6 allows." |

Section 5.4 (label vocabulary): add one sentence: "Contract strength (VER29
section 6) is a separate axis and does not change `proved`, `bounded(k)` or the
ranks above."

`evidence-kinds.json`: one new pattern row, `contract-link`, for the
package-qualified evidence ids of VER29 section 5.4.

Section 8 (relationships): add a row for VER29.

### 16.3 Other edits the implementation PRs make

| Document | Edit | Slice |
|---|---|---|
| VER01 | The report schema gains `contracts` and each claim row's `contract` object | 2 |
| VER27 | C14 recorded as resolved by VER29; section 4.14 points here; Q8 points to VER29 section 14 | 5 |
| VER-BACKLOG | The LV-005 row's state moves from "proposed; spec first" as slices land | each |
| LG01 (in the `logic-gates` adopter PR) | "pending LV-005" becomes a pointer to VER29; the ids are the section 3.1 ids | 5 |
| README | The specs README row for this document | this commit |

---

## 17. Decisions beyond the brief, and deviations

| # | What | Why |
|---|---|---|
| D1 | The document number is `VER29`, not `VER28`. | VER28 is reserved by existing citations (section "Why the number"). |
| D2 | Non-functional kinds get id shapes of 4 and 5 segments (`timing`, `capability`; `resource`), against 3 for functional. | The brief names the kinds but only the functional id form; named claims cannot cover obligations that are not `claim` blocks. |
| D3 | The kind is derived from the evidence source, never declared. | A declared kind could be wrong with nothing to catch it. |
| D4 | Unresolvable id is an **error**; resolvable but unproved is **open**. | Rule (c) says a rename breaks the link loudly; open does not fail a build. The brief said "otherwise open", which would let a rename silently downgrade a link. |
| D5 | Two additional strength rungs: `bounded-proof(k=K)` (below `exhaustive`, `K` always printed; review H2) and `exhaustive-sampled`. The order is a convention for taking a minimum, not an implication. | VER13 already has `bounded(k)`; VER04h section 3 makes most "exhaustive" suites sampled. |
| D6 | "Weakest evidence kind behind the claim" is read as weakest across the chain of contracts, with a claim's own strength being its strongest passing suite. | Taking the minimum over a claim's sibling suites would label every claim `randomized`. |
| D7 | Two pins beyond the brief's three: `claim_sha256` and `claim_entries_sha256`. No git revision. | Exactness of the claim itself; a report cannot contain its own commit. |
| D8 | Condition (e) also covers a failing `negctl` for the claim's operation. | A suite not shown to detect a fault is not evidence (VER04h section 6). Stricter than the brief's wording. |
| D9 | The lower claim's own links must be discharged, stacks must agree per claim proof, and the build configuration must be pinned, for a discharge. | Otherwise a discharge says less than it appears to (VER13 AC-7). |
| D10 | Contract ids are recognised contextually at a platform row, not as a global token; v1 documents only. | A global token breaks a binder named `contract`; VER04j section 1.2 requires no change to valid existing documents. |
| D11 | Reach pass: a new additive mode built on the analyzer's lexer and binding closure, not the analyzer's "call graph". | The analyzer has none (VER08a section 8). |
| D12 | `dependencies` and `unverified` are reported from the first slice, without call analysis. | Gives rule (b) a dependency-level floor before the pass exists. |
| D13 | Open links do not block by default; `--require-contracts` is recommended for adopters' BUILD and `--require-closed` for the publish gate. | Matches `--allow-unverified` and `--strict`. |
| D14 | Revised after review (C1): until the reach pass attributes uses, an upper claim takes **no** contract strength (`unattributed (own: ...)`). The first draft's package-wide cap is withdrawn. | `assume platform` is document-level, and a strength from a contract for a different operation is wrong. |
| D15 | A link edge for CI is derived from `.warrant` text, not Cargo edges; link-check selection, not full rebuild. | Section 11.3. |
| D16 | Amendments land with the slices that make them true, not all last. | Avoids a window of contradiction. |
| D17 | A dev-dependency or build-dependency does not count as a dependency for condition (a). | The shipped artefact does not contain it. Open question VQ3 revisits. |
| D18 | No `publish` list, no alias or deprecation, and examples are not published. | Sections 3.5 and 9.1. |
| D19 | The report field is `contracts` with `published` rows in the lower report, so each layer reads the one below. | Composition by one-step reads (section 6.3). |
| D20 | **Ids are lane-less**: `contract.<package>.<claim>`, resolved per the report's own lane; cross-lane resolution is an error (review H5). **Deviation from the coordinator's design** (`contract.<lane>.<package>.<claim>`). | The `.warrant` is language-neutral and shared by every lane; a lane in the id would make one file say different things per lane. |
| D21 | `linked` (dependency level) versus `covered` (call level, domain unchecked) (review C1). | A dependency with some contract is not covered. |
| D22 | One rule for contract-less dependencies: functional strengths print with `conditional_on`; timing, resource and capability are withheld, including for external crates and `std` (C2). | The first draft contradicted itself. |
| D23 | Per-claim strength lives in `published`; an operation row shows the minimum (C3). | Report rows are per operation, not per claim. |
| D24 | Condition (a) binds to the in-tree path package; `Cargo.toml` and `build.rs` are pinned; differing features give `build-config-unpinned` (C4). | A name is not an identity. |
| D25 | The lower report is checked with its own BUILD's flags and its `unverified::` ids are carried into `conditional_on` (H1). | Works today; never inherits silently. |
| D26 | `exhaustive` requires a binding that is pure in its explicit state (H3, reworded 2026-11-14 from "stateless, deterministic, `state none`"): `state none`, or `state=explicit` with a complete state set and the VER04m E15 obligations passed; a driven operation, or a bounded set of call sequences, is never `exhaustive`. | One initial state proves one transition; a complete set of explicit states with hidden-state checks proves the transition relation, and a bounded set of sequences proves only itself. |
| D27 | Regeneration cannot accept a downgrade silently: CI gate against the diff base, accepted by a `Contract-Change-Accepted` commit trailer (H4). | A regenerated pin would otherwise hide a weakened claim. |
| D28 | Reach resolution is by defining path through `pub use`; `call-level-partial` unless unresolved and opaque uses are 0; pins gain `unverified_deps` and toolchain; generator and tooling changes select all link checks (review medium items). | Last-segment matching and a `call-level` that ignores unresolved uses were unsound. |

---

## 18. Open questions

For the owner or the coordinator. None blocks slices 1 to 3.

| # | Question | Default until decided |
|---|---|---|
| VQ1 | Should an opt-in `export` list be added so internal claims stay private? | No; implicit publication (section 3.5). |
| VQ2 | `assume platform` is document-level, so no contract strength is taken until the reach pass attributes uses (section 6.4). Should attribution come from the pass alone (no keyword), or from a per-claim annotation? | `unattributed` until the pass. |
| VQ3 | Do dev-dependencies and build-dependencies count as link targets? Tests run against a dev-dependency, but the shipped artefact does not contain it. Separately: does `gate.primitives.correct` become `discharged_by` the gate contract set, partially, given its transistor-equivalence half is open? | Neither counts; the catalogue entry is unchanged. |
| VQ4 | Q8: run the miter and the composition proof for one small layer of the 4004 chain and compare, as the calibration (section 14)? | Yes, recommended. |
| VQ5 | How is a capability contract's strength classified? The analyzer is lexical and incomplete (VER08a section 8), the effect certificate is audited, runtime conformance is observed. | Capability ids stay `kind-not-yet-published`. |
| VQ6 | Is the dynamic substitution run of section 8.6 worth building? It is the only check of rule (b) that does not depend on a lexer. | Not committed. |
| VQ7 | Pin granularity: `source_sha256` covers all of `src/`, so any lower edit regenerates every assuming report. Is a per-claim slice (spec text of the claim plus its manifest entries, without the source) enough, given the lower report already pins its own source? | Whole-file pins, as the brief requires. |
| VQ8 | If packages are ever versioned and published independently, an alias with an expiry may be needed. Revisit then. | No alias. |
| VQ9 | Should `--require-contracts` be on by default for any package whose `.warrant` assumes a contract id, rather than a BUILD choice? | BUILD choice, recommended on. |
| VQ10 | Cross-lane links (native bridges): a design of their own, or never? With lane-less ids (D20) the id could name a package in any lane, so a design would have to add a lane qualifier at resolution, not in the `.warrant`. | Error (section 3.4). |
| VQ11 | Require a negative control that targets the assumed claim itself (a fault only that claim can catch), so a passing control shows that claim's suite can fail? Today controls are per operation (section 5.5). | No; documented as a limit. |
| VQ12 | A domain-compatibility check: compare the caller's argument domain with the contract's refinement and rejection rows (for example from a call-site claim, or a type)? Needed for Q8 condition 6. | None; `covered (domain unchecked)`. |
| VQ13 | What justification lets a package with `build.rs`, `include!`, `#[path]`, macros or generics reach `call-level` rather than `call-level-partial`? | Never; partial. |
| VQ14 | Record the features and `cfg` list of the evidence build in the manifests so `build-config-unpinned` can be decided exactly rather than treating any `features` key as a mismatch (section 5.1.1)? | Treat any `features` or `default-features` key as a mismatch. |

---

## 19. Relationship to other specifications

| Spec | Relationship |
|---|---|
| VER27 | P9, sections 4.13 to 4.15, C14, Q8. This document is the specification LV-005 calls for; section 14 answers Q8 as far as it can. |
| VER04 | Section 3 amended (section 16.1); section 2's "no imports" is unchanged in substance. |
| VER04h | Manifests, `source_sha256` framing and section 9 checks are reused unchanged; section 9's check 13 (evidence ids) is why lower ids live under `contracts`. |
| VER04i | Lane manifests and the lane list; non-Rust lanes resolve to open until their readers exist. |
| VER04j | The v1 header gates contract entries; the grammar-compatibility tests apply to slice 1. |
| VER13 | A new generated family; rules AC-1, AC-3, AC-4 extended; new AC-16 (section 16.2). |
| VER08a | The reach pass builds on its lexer and binding closure; its section 8 limits are section 15's limit 4. |
| VER20 | Timing contracts rest on its constant-time obligations; its self-composition check is what a composition proof of glue would repeat (section 14). |
| LG01 (PR #19252) | The first publisher; its claim table is the template for `published`. |
| LV-002 | Section 11 depends on and extends its consumer rule. |
| LV-006 | The adopter chain: gates, combinational, arithmetic, sequential, CPU step, programs. |
