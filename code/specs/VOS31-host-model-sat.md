# VOS31 -- The Host Model-SAT Lane

**Status:** Draft; 040a and 040b built (backlog VOS-040-0 wrote the spec). No crate, no table and no BUILD line is added by
this spec. Everything marked **NEW(row)** is built by the row named. Sizes and times are
**arithmetic or expectation, not results**: nothing in this document has been run, and section 16
lists each such statement with the PR that will settle it.

**Layer:** Program model, a sibling of [VOS30](VOS30-proof-route-and-capacity.md) (the X3 route on
compiled code). VOS30 proves things about *compiled implementations*. This lane proves things about
the *contract's own model*, on the host, with no compiler in the loop.

**Parent:** [VOS04](VOS04-component-catalogue.md) sections 5.2, 5.3, 5.6, 6.2 and 8
(the only prior text; this spec replaces its one-paragraph description of the lane).

**Builds on:** [VOS10](VOS10-scheduler-contract.md) section 9.2a (the ten lemmas),
[VOS30](VOS30-proof-route-and-capacity.md) sections 4.5, 4.6, 6.1, 6.3 and 8.6.1 (collapse,
vacuity, one DRAT per obligation, default checker limits, the route budget),
[VER09](VER09-cdcl-sat.md), [VER10](VER10-bit-blasting.md) and [VER11](VER11-drat-proofs.md)
(the solver, the bit-blaster, the checker).

**Why a new `VOS31`, not a VOS04 section 5.7.** `VOS30` is the proof-route spec and `VOS31` is the
next free number in the `VOS3x` (proof) family; `git grep VOS31` found no use. VOS04 is a catalogue
and a set of rules and already holds more than 1,500 lines; this lane has its own crate, CLI,
table and CI rule, which is what a child spec is for. VOS04 keeps a pointer (the amendments of
section 15) and the rule that a contract may ship the lane, nothing more.

**Owner and coordinator decisions, 2026-11-25** are recorded in section 14 and in the backlog's
decision log, with the owner's words verbatim.

---

## 0. How to read this document

Marks are those of VOS01 section 0. **Today** exists on this branch; **NEW(row)** does not.
"Fact" means someone read it in the file cited; "assumption" means it was inferred and the PR named
in section 16 must check it.

Words used strictly:

* **Lemma:** one `claim` of a contract's `warrant/lemmas.part`. It calls no bound operation, so it is
  a fact about the abstract model (VER04m V9; VOS-D005). It has no published strength.
* **Obligation:** one propositional formula whose UNSAT is one fact, with its own CNF and its own
  DRAT proof (VOS30 6.1). Here an obligation is "lemma L at width N".
* **Control:** a formula that must be **SAT**. The lemma's mutant encoder is the control: if it is
  also UNSAT, the lemma proved nothing.
* **Premise witness:** a SAT answer for the antecedent of an implication lemma alone (VOS30 4.6).
* **The ladder:** the four widths N = 8, 16, 32 and 64 (section 4).
* **The encoder:** a Rust function from a width N to an AIG, written by hand from `contract.part`.
  It is not generated, and that is the whole trust problem (section 5).

---

## 1. What this lane is, and what it is not

An exhaustive evaluation of a lemma stops at N = 8 (65,536 cases for eight of the ten lemmas, 256
for two; `warrant/lemmas.table`). SAT does not stop there. The repository already has a bit-blaster
(`bit-blaster`: `BitVec`, `Aig`, `Tseitin`), a CDCL solver with proof logging (`cdcl-sat`) and a DRAT
checker (`drat-check`). This lane uses them to solve each lemma, negated, at the four widths of the
ladder and to check the proof.

**What it proves.** For each lemma and each ladder width: the lemma's *hand-encoded* negation is
UNSAT, with a DRAT proof the default checker accepts.

**What it does not prove, and must never print as proof of:**

1. **That the encoder is the contract.** There is no Warrant-to-QF_BV translator (VOS-D072; the
   VER request is filed, section 14). The encoder agrees with the Warrant evaluator on the whole
   domain at N = 8 and with a plain Rust reference at N = 1 to 10 (section 6). Nothing relates N = 8
   to N = 64 except the reference, so the wider rows are evidence about the encoder family, not
   proof about `contract.part`.
2. **Anything about an implementation.** A lemma has no published strength (VOS-D005), so it is
   printed "lemma" and never counted (VOS04 5.3). Implementations keep printing **"no SAT evidence"**
   (VOS04 5.2, line 10), and that line fails if it prints a SAT claim for one.
3. **Anything about compiled code.** That is VOS30 and VOS-029.

---

## 2. Scope

**In scope (coordinator decision Q2):** the ten lemmas of
`code/packages/rust/vos-sched-contract/warrant/lemmas.part`, on the `sched` contract:
`closure_at_m`, `insert_outside_pre_changes_nothing`, `remove_outside_pre_changes_nothing`,
`insert_then_remove_restores`, `top_is_the_maximum`, `safety_is_idle_or_a_member`,
`safe_dispatch_only_removes_the_choice`, `policy_implies_safety`, `policy_admits_only_top` and
`config_n_is_the_id_range`.

**Out of scope, each with its reason:**

| Not in scope | Why |
|---|---|
| Tier ordering | Already a lemma: `policy_implies_safety` |
| Pairwise spine-invariant closure | There is no spine yet (VOS-026); a lane over nothing proves nothing |
| The `representation` tier's eight claims | They call bound operations of `rr`'s model, so they are obligations, not lemmas |
| A Warrant-to-QF_BV translator | A request to the VER backlog (section 14), not a deliverable |
| A `lemma` claim kind | VER-lane work (VOS-D005, WX-SV item 3) |
| Any second contract | `thread` (item 16) is the second contract and pays its own encoder cost (section 10.3) |

VOS04 5.3 lists "contract lemmas, tier ordering and pairwise spine-invariant closure". This spec
narrows that sentence to the first item; section 15 amends it.

---

## 3. The hard constraint: do not touch the recorded inputs

The capacity recording of VOS30 section 9 (`data/vos30-capacity-results.json`) records the tree ids
of **46 entries** (`meta::TREE_ENTRIES` in `vos-capacity-probe`; fact). They include the probe's own
`src`, `bit-blaster`, `cdcl-sat`, `drat-check`, `warrant-grammar`, `sha256`, and, for
`vos-sched-contract` and `vos-sched-fp-bitmap`, their `src` and `Cargo.toml` (and the generated
`.warrant` documents and harnesses). An edit to any one of them makes the recording stale, and
`sign` requires the verify's tree ids to equal the reviewer's working tree (VOS30 9.12). The
reviewer's sign-off is VOS-028c3d, which may trail this item.

> **Rule.** No VOS-040 PR (040-0, 040a, 040b, 040c or 040d) may edit any of the 46 recorded
> entries. The lane is a new crate that **depends on** those crates and edits none of them.

Consequences, each binding on the PRs of section 17:

* The lane is not added to the probe and the probe is not edited (decision Q3, section 7.3).
* `vos-model-sat` reaches `vos-sched-fp-bitmap` (the spike's subject) **only through its own
  dev-dependencies**, never by editing that crate's `src` or `Cargo.toml`.
* `vos-sched-contract` gains **only** its generated `BUILD` and `BUILD_windows` lines, its
  `vos.toml` key `sat` and its generated `warrant/lemmas.sat`. Not `src`, not `Cargo.toml`, not
  `warrant/*.part` and not `contract.vos`. (Whether `vos.toml` is a recorded entry is verified by
  PR 040c against `meta::TREE_ENTRIES` before it edits that file; section 16 item 7. If it is, the
  key is added after 9c4 signs.)
* PR 040c edits `vos-link` and the build tool, neither of which is a recorded input (fact: the
  entries of `meta.rs` were listed and none is either).
* The lane's tests do not stale the recording either: they live in `vos-model-sat`.

---

## 4. The width ladder

**Owner decision (2026-11-25), verbatim: "Ladder 8/16/32/64 (Recommended)".**

Each lemma is solved at N in {8, 16, 32, 64}. The labels are fixed (section 11), and the reason is
VOS-D068: `contract.vos` pins `config N 8` and no mechanism yet gives a second configuration, so
"the shipped N" is undefined for `sched`. Until VOS-D068 defines a second configuration, **no row is
called "shipped"**.

| N | Label | What stands behind it |
|---|---|---|
| 8 | **checked against the evaluator on the whole domain** | The encoder equals the Warrant evaluator on all 65,536 (or 256) cases, base and mutant (section 6.1) |
| 16, 32, 64 | **lemma family member, no shipped component has this width** | The encoder equals a plain Rust reference at N = 1 to 10 (section 6.3), and the reference equals the evaluator at N = 8. Nothing else relates the width to the contract |

At N = 64 the id binder is still 8 bits: a thread id is a `U8`, the idle sentinel is 255 and
64 < 255, so the encoder's `PickEnc` is `{ idle: Edge, id: BitVec(8) }` at every ladder width
(assumption; 040a confirms it against `contract.part`'s `pick_of`, whose input `k` is a `U8`).
`Ready` is the only type whose width is N.

N = 8, 16, 32 and 64 are the probe's A1 and A2 widths (VOS30 8.4; fact), so the cost is expected to
be small (section 7.4, assumption).

---

## 5. The crate and the encoder

**NEW(VOS-040a).** `code/programs/rust/vos-model-sat`: a library plus a binary, `publish = false`,
its own `[workspace]`, `vos.toml` class `P`, `BUILD`, `BUILD_windows`, `README.md`, `CHANGELOG.md`
and `required_capabilities.json`.

Dependencies (the only ones): `bit-blaster`, `cdcl-sat`, `drat-check`, and for the cross-checks
`warrant-grammar` (the evaluator, read-only), `vos-manifest`'s admission rules for reading author
files (size caps, path containment), `vos-sched-contract` (the contract's files), and the sha256
crate the repository already uses. **Not** `vos-capacity-probe` (section 7.3).

**As built by 040a (amended).** `vos-link` is also a dependency (its library surface, `load_inputs`,
`evaluation_document` and `splice`, which 6.1 already names; it supplies the bounded loader, so
`vos-manifest` is used directly only for mutant spans). `cdcl-sat`, `drat-check` and the sha256
crate are not dependencies of 040a (no solving); 040b adds them. The binary of 040a has one
subcommand, `cross-check <contract-crate-dir>`, which runs the three checks (exit 0, 1 or 2); `gen`
and `check` are 040b. The loader refuses a `..` in the crate path, so BUILD runs it from inside the
contract crate.

### 5.1 Shape

```text
struct Ready(BitVec)                       // width N; bit i is thread i
enum   PickEnc { idle: Edge, id: BitVec }  // id: 8 bits at every width

trait SchedModel {                         // one method per model function of contract.part
    fn mem(&self, aig: &mut Aig, m: &Ready, i: &BitVec)      -> Edge;
    fn put(..), subset(..), top(..), after(..), valid_pick(..), pick_of(..),
        abs_insert(..), abs_remove(..), pre_insert(..), pre_remove(..),
        safe_dispatch(..), policy_dispatch(..);
}

struct Encoder { n: u32 }                  // the honest implementation
struct Mutant  { lemma: LemmaId, n: u32 }  // one mutant implementation per lemma

fn lemma(name: &str, m: &dyn SchedModel, aig: &mut Aig) -> LemmaCircuit
struct LemmaCircuit { inputs: Vec<Edge>, premise: Option<Edge>, holds: Edge }
```

`Encoder` is parametric in N: a single `for i in 0..n` builds `mem`, `put` and `top`, so the same
source text is the N = 8 circuit that the evaluator checks and the N = 64 circuit that the solver
sees. `N` is bounded (1 to 64) and the number of lemmas is bounded (at most 64, the limit
`check-contract` already enforces: CK03); both are checked before any allocation.

### 5.2 The mutants

Each lemma has one mutant model function in `warrant/mutants/<lemma>.part` (fact: ten files). The
lane has the **same ten mutants as Rust code**: `Mutant { lemma, n }` is `Encoder` with exactly the
model function that `warrant/mutants/<lemma>.part` replaces, replaced by the same change. A mutant
exists because a lemma that is UNSAT and whose mutant is also UNSAT has proved nothing.

The mutant-to-part correspondence is **checked, not trusted**: at N = 8 the mutant encoder's failing
case set, per lemma, must **equal** the evaluator's failing case set on the mutant document, and its
size must equal the `failed` column of `lemmas.table` (section 6.1).
**As built by 040a (amended).** The mutants generalise to N as: `inv` is bit 0; `pre_insert` admits id N;`valid_pick` refuses thread N-1 (`policy_implies_safety`) or any id below N (`safety_is_idle_or_a_member`);`top` tests thread 0 first and then the honest chain without thread N-1. The `top` mutant equals the baseat N = 1, so its SAT control exists only for N >= 2 (the ladder starts at 8). On the honest encoder`closure_at_m` and `config_n_is_the_id_range` fold to the constant true (a `constant-fold` collapse, 7.5).

---

## 6. The three cross-checks

The encoder is the trust story. These three checks are what turn "a hand transcription" into
"evidence", and each is a test of `vos-model-sat`, not a promise. None edits `warrant-grammar`.

### 6.1 The lemma check: the evaluator at N = 8, base and mutants

For every lemma and every binder case of its whole X1 domain, `Aig::eval(holds)` (with the case as
input) must equal the Warrant evaluator's verdict on that case. This is done **twice**: on the base
document and on the mutant document.

The mutant half is the real comparison. On the base document alone, both sides answer "true" 65,536
times and the check proves nothing. On a mutant, the **sets of failing cases must be equal**, not just
equal in size, and the count must equal the `failed` column of `warrant/lemmas.table`. The expected
counts, read from the table (fact): `closure_at_m` 65,536; `insert_outside_pre_changes_nothing` and
`remove_outside_pre_changes_nothing` 63,488; `insert_then_remove_restores` 448;
`top_is_the_maximum` 512; `safety_is_idle_or_a_member` 1,024;
`safe_dispatch_only_removes_the_choice` 1,016; `policy_implies_safety` 128 (of 256);
`policy_admits_only_top` 255; `config_n_is_the_id_range` 1 (of 256).

The evaluator is reached through `vos-link check-contract`'s library surface (`evaluation_document`,
`config_preamble`, `run_lemma` and `LemmaRun`; all public, fact), so the lane evaluates the same
document `check-contract` evaluates.

### 6.2 The function check: each model function, N = 8

Each model function's output is compared with the evaluator over the **whole domain of that
function**, so a wrong `top` cannot hide behind a lemma that does not exercise it. The comparison
reads each function's value from the evaluator's ground call interface (generated probe claims of
the form `f(args) == value`). If that interface is not usable, 040a stops, records the refusal and
asks: it does **not** edit `warrant-grammar` (a recorded input; section 3).

**As built by 040a (amended, VOS-040a; what changed and why).**

* The "generated probe claims of the form `f(args) == value`" are `claim probe_<name>: forall ... .
  ((E) == (E))`, and only the left operand `E` is evaluated, so no expected value has to be known to
  the evaluator side. `warrant-grammar` is not edited.
* A Warrant claim has no `Pick` binder, so `after` and `valid_pick` have two probes each, `*_run`
  (`Run(k)`, all 256 values of `k`) and `*_idle`.
* **Divergence: `safe_dispatch` and `policy_dispatch` are NOT probed on their whole domain.** It
  is `2^8 x 257 x 2^8` = 16.8M cases per relation, over the evaluator route's 400,000-case
  budget. Each is probed on every `Ready`, every `pick_of(k)` and four candidates for the second
  `Ready` (`after(m, o)`, `m`, `empty()`, and `after(m, o)` with thread 0 flipped), chosen by two
  `Bool` binders: 2^18 cases each, both truth values occurring. `Run(255)` is not covered for the
  relations (it is for `after` and `valid_pick`). The relations are still covered whole by check (c)
  at N = 1 to 10 and, through the lemmas, by check (a).
* On a mutant document the function check runs only on the probes the replaced model can change
  (that model and its callers), to bound the work; the base document runs every probe.

### 6.3 The width check: a plain Rust reference at N = 1 to 10

The encoder is compared **exhaustively** at every N from 1 to 10 against a plain Rust reference model
written in the test (not derived from the encoder): all `Ready` values and all id values, by
`Aig::simulate`/`eval`. The reference is itself compared with the evaluator at N = 8. This is the
**only** check that relates two widths. Without it, the N = 64 row would rest on N = 8 alone
(VOS-D072). With it, it is still evidence, not proof (section 1).

### 6.4 Negative controls of the cross-checks

A deliberately wrong encoder (an off-by-one in `top`, `<=` for `<` in `pre_insert`) must be caught by
check 6.1, by check 6.2 and by check 6.3 **separately**: three checks are three chances, and a test
that catches the fault in only one has not shown the other two can fail.

---

## 7. Solving and certificates

### 7.1 One obligation

For lemma L at width N: build `LemmaCircuit` on a fresh `Aig`; assert `not holds` (`holds` is the whole
lemma, antecedent included); Tseitin-encode to a **fresh CNF** (route A of VOS30 6.1: one CNF per
obligation, no assumptions); solve with proof logging; check the DRAT with
`drat_check::Limits::default()`. The solver's conflict budget is the route budget below.

The certificate id is the **length-prefixed** digest of VOS30 6.1:
`sha256( u64be(len(DIMACS)) || DIMACS || u64be(len(DRAT)) || DRAT )`.

### 7.2 The rules every obligation must satisfy

1. **UNSAT**, with a proof accepted by `drat-check` under **`Limits::default()`** (VOS30 6.3: limits
   are never raised; a proof the default limits reject is an obligation that is too big, and the
   remedy is a better encoding).
2. **Inside the route budget**: at most **50,000 conflicts** and `check_work` at most **2^31**.
3. **A SAT control** at the same N: the lemma's mutant encoder is SAT; the model is checked against
   the CNF, replayed through `Aig::eval` (the lemma is false on it), and at N = 8 replayed through the
   Warrant evaluator on the mutant document (the case is in the mutant's failing set).
4. **A premise witness** for every lemma whose claim body is, at the top level, an implication
   (`P => Q`): `P` alone is SAT, and a model is replayed (VOS30 4.6). Whether a claim is of that form
   is read from the **Warrant parser's AST of `lemmas.part`**, never from the encoder, so an encoder
   cannot declare its own lemma exempt. An implication nested inside a conjunction (`closure_at_m`,
   `top_is_the_maximum`) is covered by rule 3.
5. **A collapse is labelled, never hidden** (section 7.5).

### 7.3 The route budget, and what this lane does and does not reuse

**The constants.** The budget is "**the route budget of VOS30 8.6.1**": 50,000 conflicts and `check_work`
at most 2^31. These two constants were derived **before any VOS-028c run** and are **frozen**
(VOS30 8.6.1). `vos-model-sat` has its own two constants, `ROUTE_MAX_CONFLICTS = 50_000` and
`ROUTE_MAX_CHECK_WORK = 1 << 31`, and **a test reads VOS30 section 8.6.1 and asserts that it states
both numbers** (the house pattern; compare `check-contract`'s 400,000). If the owner changes either
constant in VOS30, that test fails, the table header regenerates and the lane re-runs under the new
number: a specification change with a decision-log entry, as VOS30 says.

**Wording rule.** This spec, the crate's README and its report cite **only** "the route budget of
VOS30 8.6.1". They do not cite the R3 decision, K = 2, a cut density or any recorded conflict count
as the justification. Those are provisional until VOS-028c3d signs (VOS30 8.6, 9.12), and the lane
must stand whatever that sign-off decides (section 12).

**Harness (coordinator decision Q3).** The lane does not depend on `vos-capacity-probe`, as a library
or otherwise, and does not edit it. It depends on `bit-blaster`, `cdcl-sat` and `drat-check`
directly, and re-implements about 150 lines of solve loop and checker call (`solve_cnf`,
`check_default`, `model_satisfies`; the shape of the probe's `src/solve.rs`). Reasons, each a fact:

* the probe's `src` is one of the 46 recorded inputs (section 3), so extracting its `solve.rs` into
  a shared crate would stale the recording while 9c4 is unsigned;
* PR #20116 (VOS-028c3c-3b-1c) is open on the probe's `src` at the time of writing;
* the probe's library also includes the generated scheduler harness (assumption: it installs an
  allocation observer as the global allocator; not verified), which a model-SAT lane should not link.

**VOS04 6.2 said** "reuses the probe harness". That wording is amended (section 15). A discovered-
work row, **VOS-D145**, extracts a shared harness crate after VOS-028c3d signs, when editing the
probe's `src` no longer stales anything.

### 7.4 Expected cost (assumption)

Lemma obligations at these widths are expected to need **tens to hundreds of conflicts** each; the
nearest measured analogue is the probe's A1 and A2 miters, 547 conflicts worst (fact, probe results).
That is a factor of about 90 inside the budget. If one lemma is not, 040b stops and reports it
instead of raising a limit. Table-shaped models (`thread` and later) are expected to be much harder
than a bitmap; this is one reason the encoder rule is opt-in (section 10.3).

### 7.5 Collapsed lemmas (coordinator decision Q7)

A lemma whose negation folds to a constant, or is refuted by unit propagation alone, is UNSAT for no
reason that tells anything about the lemma. The lane records it and accepts it **only** when both:

1. it carries a `collapsed` reason from a **closed enum**:

   | Reason | Meaning (the subset of VOS30 4.5 that applies to a lemma) |
   |---|---|
   | `constant-fold` | `holds` is the constant true edge: the negation is the constant false from symbolic inputs |
   | `propagation-only` | the CNF is not trivial, but unit propagation alone refutes it |

   `identical-structure` and `frame-untouched` of VOS30 4.5 are miter and write-set reasons and
   never apply to a lemma.
2. the **SAT control passes** (section 7.2, rule 3).

`check` **re-derives** the reason, it never reads it as a claim: `constant-fold` from the AIG, and
`propagation-only` by the lane's own small unit-propagation routine on the regenerated CNF (about 40
lines, in this crate, so the field stays solver-independent). A recorded reason that does not hold on
the regenerated obligation fails. `closure_at_m` (whose `inv` is `true`, so it folds) is printed
**"trivial (inv is true)"**, as `lemmas.part` already says, and is **not counted**.

---

## 8. The committed table: `warrant/lemmas.sat`

Generated, committed, compared byte for byte, never parsed as author text: the principle of VOS04
5.6. The model is `warrant/lemmas.table` (VOS04 5.6, "Contract crates").

### 8.1 The committed-field rule (coordinator decision Q4)

A DRAT digest and a conflict count depend on `cdcl-sat`'s source. If they were committed and
compared, every edit to the solver would turn every contract crate red: the problem VOS-D141 met
with the recorded inputs. So:

* **Committed, compared byte for byte (solver-independent):** the lemma, N, AND count, variable
  count, clause count, `cnf_sha256`, the verdict, `collapsed` and its reason, the control's verdict,
  the premise-witness verdict, and the digests of `contract.part` and `lemmas.part`.
* **Gated at `check` but never committed:** conflicts (at most 50,000), `check_work` (at most 2^31)
  and acceptance under `Limits::default()`. They are printed in an **uncommitted report** (stdout, or
  a file the caller names) and fail the run when out of bounds, but no table byte depends on them.
* **DRAT files** are written **only on request** (`check --emit-drat <dir>`). "The manifest is
  committed, the proofs are not" (VOS04 5.3).

The committed CNF digest depends on `bit-blaster`'s variable numbering, so an edit that renumbers
variables there regenerates the table. That is intended (it is the trusted encoder) and it is
exactly what the build-tool row of section 10.4 selects the contract crate for. An edit to `cdcl-sat`
or `drat-check` changes no committed byte, and still re-runs the gates (same row).

### 8.2 Format

```text
vos-sat 1
contract-part <sha256 of contract.part>
lemmas-part <sha256 of lemmas.part>
route-budget conflicts 50000 check-work 2147483648
lemma <name> n <N> label <label> and <k> vars <v> clauses <c> cnf <sha256> verdict unsat collapsed <no|constant-fold|propagation-only> control sat premise <witnessed|none>
...                                            (one line per lemma per ladder width; lemma order of lemmas.part, then N ascending)
trivial closure_at_m
end
```

`label` is one of the strings of section 11, spelled with `_` for spaces so a row stays one line.
The `route-budget` line prints the two frozen constants; if VOS30 8.6.1 changes, the header
regenerates. The table holds ten lemmas times four widths: 40 rows plus the header.

---

## 9. The CLI

```text
vos-model-sat gen   <contract-crate-dir> [--root <repo-root>]    writes warrant/lemmas.sat
vos-model-sat check <contract-crate-dir> [--root <repo-root>] [--emit-drat <dir>]
                                                                  exit 0 pass, 1 findings, 2 no verdict
```

**`gen`** builds, solves and writes the table. It runs every rule of section 7.2 first and **writes
nothing if any fails**, so a table can never record a failing lemma.

**`check`** regenerates in memory, **runs every gate** (sections 6, 7.2 and 7.5) and compares the
table byte for byte. It does not trust the table, and it does not read a status from it. It is the
only command the BUILD line runs.

**Findings** use the `vos-link` shape (a code, a message, the first differing line): `MS01` inputs
(a path is a symlink, `vos.toml` does not say `sat = "model"`, a file over the size cap, a lemma
count or N out of bounds); `MS02` encoder cross-check (6.1 to 6.3); `MS03` lemma not UNSAT;
`MS04` proof rejected under default limits; `MS05` route budget exceeded; `MS06` control not SAT or
not replayed; `MS07` premise not witnessed; `MS08` collapse without a re-derivable reason; `MS09` the
table is absent or stale; `MS10` a lemma of `lemmas.part` has no row ("none ran = fail", section 10.2).

**Untrusted input.** `gen` and `check` read author-controlled `.part` and `vos.toml` files. They use
`vos-manifest`'s admission (size caps, path containment, no symlinks), bound N and the lemma count,
and never panic on a malformed file: every failure is a finding, never an abort (the contract of
`check-contract`).

**As built by 040b (amended; what changed and why).**

* **`sat` key.** `vos-link`'s loader rejects an unknown `vos.toml` key (`unknown key (allowed: class,
  kind, ...)`), so the real `sched` contract cannot carry `sat = "model"` until 040c teaches the
  loader the key. 040b therefore reads `vos.toml` itself for MS01 and adds `--allow-missing-sat-key`
  to `gen` and `check`, which skips only that half of MS01. 040c removes the need (BUILD runs
  `check .` without the flag).
* **Load failures are MS01, exit 1** for `gen` and `check` (a symlink, an oversize or malformed
  input), as section 9 lists; exit 2 is bad arguments and an evaluator that could not run.
  `cross-check` keeps exit 2 for an unreadable crate.
* **`gen` also runs the cross-checks** (MS02), so a table is never written from an unchecked
  encoder.
* **`and`** in the table is the AND count of the AIG built for the obligation (all gates built, not
  the cone of `holds`), so a constant-fold row shows a large `and` and one CNF variable.
* **MS08** fires in three ways: a re-derivation that fails for an UNSAT-with-no-conflict row, and,
  in `check`, a committed `collapsed` word that is not in the closed enum or does not hold on the
  regenerated obligation (an MS09 is reported with it).
* `check --emit-drat <dir>` writes `<lemma>-n<N>.cnf` and `.drat` (the CNF so a proof can be
  re-checked elsewhere).
* The solve has no wall clock: an obligation is bounded by the conflict count alone, so verdicts
  are the same on every machine.

---

## 10. Kit wiring (NEW, VOS-040c)

### 10.1 The BUILD line

`vos-link gen-build` for a contract crate whose `vos.toml` says `sat = "model"` generates, after the
`check-contract` line and before `check-component`:

```text
cargo run -q --release --manifest-path ../../../programs/rust/vos-model-sat/Cargo.toml -- check .
```

`--release`: SAT in a debug build is slow (assumption), so every cargo line of the lane is
`--release`, and `BUILD_windows` runs **the same line** (the lane uses no Unix-only process limit;
the probe's limits are Unix-only and are not reused). `check-component` requires the line **byte for
byte** as it requires every generated line (VOS04 5.6: generate and compare, never lint).

### 10.2 `check-component`: CK11, and "none ran = fail"

A new code, **`CK11`** (the next after `CK10`), fails when a contract crate whose `vos.toml` says
`sat = "model"` has no such BUILD line, no `warrant/lemmas.sat`, a `lemmas.sat` that does not name
every lemma of `lemmas.part` at every ladder width, or zero obligations. **None ran = fail:** an
empty table, or a lemma of `lemmas.part` with no row, fails, because a lane that is silently skipped
reports green (VOS04 5.2, lines 6 to 10).

And the implementation side: an **implementation** crate's evidence report must still print **"no
SAT evidence"** for its line 10 and `check-component` fails if it prints any SAT claim for one (this is
VOS04 5.2 line 10 as it stands; 040c adds a negative control for it).

### 10.3 The encoder rule: opt-in, decided later

**Owner decision (2026-11-25), verbatim: "Opt-in for now (Recommended)".**

The hand-written SAT encoder is **required for `sched`** and **opt-in per contract** through the
`vos.toml` key `sat = "model"`. A contract without the key ships no encoder and prints no lemma SAT
line. Whether the rule becomes mandatory for every contract is decided **after `thread` (item 16)
has paid the cost once** (VOS-D147). The cost is one N-parametric encoder, one mutant encoder per
lemma and three cross-checks per contract, for about 30 interfaces, all hand transcription until a
translator exists. This amends VOS04 6.2, whose "every contract PR ships SAT certificates" stated
the mandatory form.

### 10.4 The build-tool row

`repository_inputs.go` gains the contract row's paths: `vos-model-sat`, `bit-blaster`, `cdcl-sat`
and `drat-check` (the VOS-D066 pattern; `code/programs/go/build-tool/repository_inputs.go`), with its
Go test. Without it a solver edit never re-runs the contract crate's SAT line (a "selection gap"; the
build tool is diff-based). On Windows the build tool reads `deps=` only from the platform-selected
BUILD (issue #12171), so both `BUILD` and `BUILD_windows` carry the line.

`vos-link` is in `vosToolPaths`, so 040c reselects every VOS crate's evidence: expect a long CI run.
The key `sat` is parsed by `vos-link`, which is how the rule is per contract.

---

## 11. Labels: what is printed, and what is never printed

The strings are exact. A test asserts each.

| Row | Printed |
|---|---|
| N = 8 | `lemma, SAT-certified at N = 8; encoder checked against the evaluator on the whole domain` |
| N = 16, 32, 64 | `lemma family member, SAT-certified at N = <N>; the encoder is checked against the evaluator at N = 8 only; no shipped component has this width` |
| `closure_at_m` | `trivial (inv is true)`, and it is not counted |
| a collapse | the row above plus `collapsed (<reason>)` |
| spike success | `lowered helper` (section 13.4) |

**Never printed:** "proved", "verified", "shipped", "X3", or any SAT claim about an implementation
or a scheduler. A report that says "SAT-proved" about a scheduler is the over-claiming risk
(section 14), and a test on the printed label is its control.

---

## 12. What the lane depends on, and what it does not

**Depends on:** the two constants of VOS30 8.6.1 (50,000 conflicts and 2^31 check work); the default
`drat-check` limits (a requirement, VOS30 6.3, VOS-D042); `bit-blaster`, `cdcl-sat` and `drat-check`
being correct (the trusted base, VER10/VER09/VER11); `warrant-grammar`'s evaluator for the N = 8
cross-check; the contract's own files.

**Does not depend on:** the capacity decision R3, K = 2, cut density, the memory verdict, any
recorded conflict count, `vos-capacity-probe`, or the outcome of VOS-028c3d. The lane proves contract
lemmas on the host and uses no cuts, no sweeping and no compiled code.

**If the verify of VOS-028c3d is red**, the lane is not blocked in three of four cases:

1. *Rows drift or wall-limited rows flip* (the likely kind): the decision returns to provisional. R3,
   K and the memory verdict may move; none is an input here. Even "no lane" (R1) leaves this lane
   standing, and makes it the only SAT evidence the program has.
2. *The owner changes a budget constant:* the lane re-runs `check` under the new number (section 7.3).
3. *The red exposes a defect or non-determinism in `cdcl-sat`, `bit-blaster` or `drat-check`:* **this
   one stops VOS-040**, because the lane shares that trusted base. Stop, log it, and merge no
   certificate until it is understood.
4. *The red is caused by VOS-040 itself,* by editing a recorded input before the reviewer signs. This
   is avoided by construction (section 3).

---

## 13. The Rust-lowering spike (VOS-040d)

### 13.1 The question

VOS04 section 8, item 2: can `warrant-ir`'s Rust front end (`rust_subset::parse_rust_file`, then
`lower_source_file`) lower **any class K helper of a real VOS component**? Today that is known only
from a README, which says the front end refuses every real source except `ct-compare`.

### 13.2 The targets, in order

1. The whole file `code/packages/rust/vos-sched-fp-bitmap/src/lib.rs` (expected: refused: a trait impl,
   a ghost module and attributes; assumption).
2. `clz8` (`const fn clz8(x: u8) -> u8`, branch-free, using `wrapping_shl`, `wrapping_add` and `as u8`
   on a `bool`; fact).
3. `bit_of` (`const fn bit_of(t: u8) -> u8`).

Each helper is extracted **by span from the real file at test time**, never retyped. A test that
lowers a retyped copy proves nothing about the component. The expected refusal of `clz8` is on
`const fn`, the `wrapping_*` method calls (the lowerer rejects an unknown method call) or the
`bool as u8` cast (assumption; not run).

### 13.3 The time box and the stop rule

One PR; one working session of the implementer. **No change to `warrant-ir` or `warrant-grammar`**
(VER-owned, and `warrant-grammar` is a recorded input). If a target needs a front-end change to
proceed, the implementer stops, records the front end's own message for that target, and files a VER
row. The spike reaches `vos-sched-fp-bitmap` **only** through `vos-model-sat`'s dev-dependencies
(a dev-dependency edge on `vos-sched-fp-bitmap` and on `warrant-ir`, so an edit to either selects
the test: the VOS-D058 pattern), and edits neither's `src` or `Cargo.toml`.

### 13.4 Success, failure, and what is not acceptable

**Success.** At least one helper lowers **unmodified**; the symbolic executor (`sym.rs`) yields a
circuit; and a miter of it against the PR 040a encoder (`clz8(x)` against the model's `top` at N = 8,
under the bitmap-to-`Ready` correspondence that `vos-sched-fp-bitmap`'s abstraction function states)
is UNSAT with an accepted DRAT, with a SAT control (a mutated copy of the helper) and a concrete
differential over all 256 inputs against the compiled function. The whole-file `lowering-blocker`
stays.

**Owner decision (2026-11-25), verbatim: "Not evidence yet (Recommended)".** A successful lowering is
reported as **"lowered helper"**: *a helper lowered from source and proved equal to the model's `top`
at 8 bits*. The implementation keeps **"no SAT evidence"**, and a row, VOS-D146, decides whether the
evidence template's line 10 changes.

**Failure, an equally valid result.** Every target is refused. Each target then has a
`lowering-blocker` test that **passes while it is refused** and **fails loudly the day it lowers**
("implement the obligations"; the precedent is the six blocker tests in
`warrant-ir/tests/*_proofs.rs`, for example `gf256_proofs.rs:253`), and the front end's own message is
recorded per target. VOS04 5.3 then changes from "probably not available" to "refused as run on
<date>: <message>".

**Not acceptable.**

* A rewritten "lowerable" copy of `clz8` presented as the helper.
* A blocker test that matches on **message text**: it breaks on a reworded error and passes on a
  different refusal. The test asserts the refusal's **kind**, not its words.
* Any change to the component to please the front end.

---

## 14. Decisions and risks

### 14.1 Owner decisions, 2026-11-25 (verbatim answers)

| Question | Answer (verbatim) | Effect |
|---|---|---|
| Widths | "Ladder 8/16/32/64 (Recommended)" | Section 4; nothing is "shipped" until VOS-D068 defines a second configuration |
| Encoder rule | "Opt-in for now (Recommended)" | Section 10.3; amends VOS04 6.2; VOS-D147 decides "mandatory" after `thread` |
| Spike result | "Not evidence yet (Recommended)" | Section 13.4; VOS-D146 |
| VER requests | "File both now (Recommended)" | The VER-BACKLOG rows of section 14.3 |

### 14.2 Coordinator decisions (technical)

* **Q2, scope:** only the ten lemmas of `lemmas.part` (section 2).
* **Q3, harness:** depend on `bit-blaster`, `cdcl-sat` and `drat-check`; do not depend on or edit the
  probe; pin the two constants with a test that reads VOS30 8.6.1; cite "the route budget of VOS30
  8.6.1"; VOS-D145 extracts a shared harness after VOS-028c3d signs (section 7.3).
* **Q4, committed table:** solver-independent fields only; the rest gated, uncommitted (section 8.1).
* **Q7, collapsed lemmas:** a closed-enum reason that `check` re-derives, and a passing SAT control;
  `closure_at_m` printed trivial and not counted (section 7.5).
* **The hard constraint:** no VOS-040 PR edits a recorded input (section 3).

### 14.3 VER requests (filed by VOS-040-0)

1. **The Warrant-to-QF_BV translator (VOS-D072):** `code/specs/VER-BACKLOG.md` row `P2 / WX-QFBV`.
   Not a deliverable here and not a blocker. It is distinct from VER-BACKLOG `ADL-012a` (a WIR-to-SMT
   encoder, for Rust code), which consumes the Rust lane's WIR and not Warrant model text.
2. **The `lemma` claim kind (VOS-D005):** already covered by VER-BACKLOG `WX-SV` item 3 (the VER04m
   E18 `lemma` entry kind, V9). It is cross-referenced from VOS-D005 and not duplicated.

### 14.4 Risks

1. **Transcription (VOS-D072).** The encoder may agree with the evaluator at N = 8 and be wrong at N =
   64. Mitigated by cross-check 6.3 and by the labels; not removed.
2. **Vacuous green.** A lemma that folds to a constant, or an implication with an unsatisfiable
   premise, is UNSAT for no reason. Mitigated by the mutant control, the premise witness and the
   `collapsed` flag.
3. **Over-claiming.** "SAT-proved" about a scheduler. Mitigated by the "no SAT evidence" rule and a test
   on the printed label (section 11).
4. **Staling the capacity recording** before 9c4 signs (section 3).
5. **Selection gaps.** The build tool rebuilds only what it selects; a missing row means a solver edit
   never re-runs the contract's SAT line (section 10.4).
6. **Kit blast radius.** 040c reselects every VOS crate and changes the contract template for about 30
   interfaces, which is why the rule is opt-in (section 10.3).
7. **Windows and speed.** Release builds only; no Unix-only limit (section 10.1).
8. **Untrusted input.** Section 9.
9. **Scope creep.** The translator, the `lemma` kind, the `representation` tier and spine closure are
   each a reason this item could triple; section 2 keeps all four out.
10. **Coverage.** The repository rule is 95% for libraries and 80% for programs; this is a program with
    a library inside, and it aims for the library number.

---

## 15. Amendments to earlier specs (made by VOS-040-0)

Each is marked in place "Amended by VOS-040-0 (owner/coordinator decision, 2026-11-25)".

| Spec | Section | Change |
|---|---|---|
| VOS04 | 5.2 | The PR-C bullet and the row-10 note point at VOS31 and say the encoder rule is opt-in (`sat = "model"`), required for `sched` |
| VOS04 | 5.3 | The first bullet group: scope narrowed to the ten lemmas; "reuses" replaced by the ladder and the harness rule; the spike label "lowered helper" and "not evidence yet" |
| VOS04 | 5.6 | The contract-crate section gains the `sat` key and `CK11` |
| VOS04 | 6.1, 6.2 | "Reuses the probe harness" and "every contract PR ships" amended |
| VOS04 | 8 item 2 | The spike's result handling and the not-acceptable list point at VOS31 section 13 |
| VOS10 | 9.2a | "and by nothing else" amended: the lemmas are also encoded and certified by the lane of VOS31, labelled "lemma" |

---

## 16. Not verified: statements to settle

Each says which PR settles it.

1. **That every lemma is cheap** (tens to hundreds of conflicts), and that none needs a decomposition.
   Settled by 040b; if one is not, 040b stops and reports.
2. **That the id width is 8 bits at every ladder width.** Settled by 040a: `pick_of`'s input `k` is a
   `U8`, and the encoder's `PickEnc` carries an 8-bit id at every width (N <= 64 < 255).
3. **That the evaluator's ground-call interface is usable** for the function check of 6.2. Settled by
   040a: it is usable (`Evaluator::eval` on the left operand of a probe claim; see 6.2 as built).
4. **That `check_work` is reported by `drat-check`** under the name and units the route budget uses.
   Settled by 040b against the probe's `check_default` shape.
5. **That unit propagation re-derives `propagation-only` identically on every platform.** It is the
   lane's own deterministic routine, so it should; settled by 040b.
6. **That `closure_at_m` and several other lemmas collapse.** `closure_at_m` is known to fold; the others
   are expected to be refuted by propagation alone (assumption). Settled by 040b.
7. **Whether `vos.toml` of `vos-sched-contract` is a recorded entry.** Settled by 040c, by reading
   `meta::TREE_ENTRIES`, before it edits the file. If it is, the key lands after 9c4 signs.
8. **That the probe's global allocation observer would be linked by a library dependency.** Not
   verified; the lane does not depend on the probe, so it is not needed.
9. **That `clz8` is refused on `const fn`, the `wrapping_*` calls or the `bool as u8` cast.** Settled by
   040d by running it.
10. **The `bool` and `Ready` bit order** used by the spike's miter. Settled by 040d against
    `vos-sched-fp-bitmap`'s `alpha`.

**Settled by 040b (as run, release, Windows).** Items 1, 4, 5 and 6:

1. *Every lemma is cheap:* yes. All 40 obligations are UNSAT with an accepted DRAT; the worst is 590
   conflicts (`safe_dispatch_only_removes_the_choice`, N = 64), against 50,000. None needed a
   decomposition.
4. *`check_work`:* `drat-check`'s `Accepted.work`, in literal visits, the unit of `Limits::max_work`;
   the worst is 2,491,407 (`top_is_the_maximum`, N = 64), against 2^31.
5. *Unit propagation re-derives identically:* the routine is the lane's own and deterministic; no
   row needed it (below), so it is exercised by its unit tests only on this contract.
6. *Which lemmas collapse:* `closure_at_m`, `config_n_is_the_id_range` **and**
   `policy_admits_only_top` fold to the constant true on the honest encoder (`constant-fold`) at
   every width. **The assumption that others fall to propagation alone was wrong:** no lemma is
   `propagation-only`; the other seven are refuted by search (3 to 590 conflicts).

---

## 17. The PRs

All five are serial and each is one package (VOS04 rule 1). Each: spec first (this document covers
all), security review before push, babysit to green, squash auto-merge, reprioritise (backlog Delivery
rules).

| PR | Row | What | Touches a recorded input? |
|---|---|---|---|
| 040-0 | VOS-040-0 | This spec and the backlog surgery; docs only | No (`code/specs/**` only) |
| 040a | VOS-040a | `vos-model-sat`: encoder, mutants, the three cross-checks; **no solver** | No (a new crate) |
| 040b | VOS-040b | Solve, certify, control, premise witness, collapse rule, table, `gen`/`check`; the VOS30 8.6.1 pin test | No |
| 040c | VOS-040c | `vos-link` (`sat` key, BUILD line, `CK11`), the build-tool row, the generated `BUILD`/`BUILD_windows` and `warrant/lemmas.sat` of `vos-sched-contract`; VOS04 5.6 "as built" | Only if `vos.toml` is one (item 7 above) |
| 040d | VOS-040d | The Rust-lowering spike and its `lowering-blocker` tests; VOS04 5.3 and section 8 item 2 with the result as run | No (dev-dependencies of a new crate) |

040d is independent of 040b and 040c and may be run second if the coordinator wants the open question
settled early; it needs 040a's encoder only for the success-case miter.

---

## 18. Sources

* [VOS04](VOS04-component-catalogue.md) sections 5.2, 5.3, 5.6, 6.2 and 8.
* [VOS10](VOS10-scheduler-contract.md) section 9.2a: the ten lemmas.
* [VOS30](VOS30-proof-route-and-capacity.md) sections 4.5, 4.6, 6.1, 6.3, 8.4 and 8.6.1.
* [VER09](VER09-cdcl-sat.md), [VER10](VER10-bit-blasting.md), [VER11](VER11-drat-proofs.md).
* `code/packages/rust/vos-sched-contract/warrant/{contract.part, lemmas.part, lemmas.table, mutants/}`.
* `code/programs/rust/vos-link/src/contract_check.rs` (the evaluator route) and `src/build_gen.rs`.
* `code/programs/rust/vos-capacity-probe/src/{solve.rs, results.rs, meta.rs}` (read, never edited).
* The delivery backlog, [VOS-BACKLOG](VOS-BACKLOG.md): VOS-040a to 040d, VOS-D072, D068, D141,
  D145 to D147; and [VER-BACKLOG](VER-BACKLOG.md): `WX-SV`, `WX-QFBV`.
