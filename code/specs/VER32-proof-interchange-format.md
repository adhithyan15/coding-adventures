# VER32 — The Proof Interchange Format, and logic gates as the first proof target

**Status:** Draft for owner approval, revised after the first adversarial
review (section 15). Spec only; no code exists. The design decisions in
section 0 are owner decisions of 2026-11-24; the open questions are in
section 12. Implementation is the `PIF-*` and `G-M*` queue in
[VER-BACKLOG.md](VER-BACKLOG.md) (section 11 here), which starts only after the
owner approves this design and an adversarial review has been answered.

**Layer:** Proof machinery, layer 3, the interface of the kernel of
[VER11a](VER11a-lean4-proof-kernel.md). VER11a decides *what the kernel
checks* (Lean 4's type theory). This document decides *how a proof reaches it*.

**Builds on:** [VER11a](VER11a-lean4-proof-kernel.md) §0, §1.1, §3.3, §4.2 to
§4.5, §5.3, §7, §9 to §11, §13, §14 and §16;
[LEAN00](LEAN00-lean-on-the-generic-pipeline.md);
[VER31](VER31-tiered-release-standard.md) (claim rungs, buckets `a|b|c`);
[LG01](LG01-logic-gates-contract.md) and
[LG02](LG02-basic-gates-cross-lane-contract.md) (the gates);
[VER-BACKLOG.d/logic-gates-everywhere.md](VER-BACKLOG.d/logic-gates-everywhere.md).

**Amends:** VER11a §4.3 (the kernel's input is PIF, not lean4export) and
VER11a Q9 (resolved by this document); backlog row KRN-003 (re-scoped).

**Consumed by:** the converters (Lean, Rocq, HOL family), the repository's own
proof producers, the release gate's `proved`/`checked` evidence, and every
later proof-carrying package.

---

## 0. The design in one page

**The problem.** VER11a fixes the kernel's logic but also made its *input* the
lean4export format. That ties the trusted base to one prover's file format
(JSON or a compact text form that has changed between releases), makes a
grammar-driven JSON reader a trusted component (VER11a Q9), and makes the
official Lean toolchain the only way to get most proofs. The owner wants the
opposite: any prover, or one of our own small tools, can supply a proof, and
the trusted part stays small.

**The fix.** A neutral, versioned, append-only **Proof Interchange Format
(PIF)**. Everything that produces proofs is an *untrusted producer* that
converts into PIF. The kernel reads PIF and nothing else.

| # | Decision | Section |
|---|---|---|
| D1 | The kernel's logic is unchanged: **Lean 4's type theory** as specified in VER11a §3.3. It does not become a logical framework. The alternative (a Dedukti-style λΠ-modulo kernel with a theory per source logic) is recorded as considered and rejected | 2 |
| D2 | **PIF** is our own format: line-oriented, LL(1), backward references only, one record per line, canonical so that SHA-256 digests are stable, defined normatively by `code/grammars/pif/pif.tokens` and `pif.grammar` | 3, 4 |
| D3 | Every producer is untrusted for *validity*: the Lean exporter, Rocq, the HOL family, and our own (claim elaborator, case-split decision procedure, SAT/LRAT reconstruction, arithmetic certificates). Lean is one optional producer, no longer a dependency. Every extra principle of a source logic enters as a **named axiom** whose *type is pinned*, shows in the claim's axiom footprint and VER31 bucket, and is **not known to be consistent** with the kernel logic. A claim file may declare no axiom of its own | 1, 3.8, 6 |
| D4 | The kernel derives recursors, quotient constants and literal support itself. A producer never supplies them; they are not even in the format | 3.4 |
| D5 | The reader of PIF is in the trusted base and is at most 600 lines (VER11a §4.2, "Export reader"); every check it makes is one numbered rule in section 5.3, and the grammar describes **exactly** the language it accepts. Whether it is hand-written and differentially tested, or generated from the grammar, is owner question Q1 | 5 |
| D6 | The first proof target is **logic gates**, ahead of ct-compare, because the model and the input contract are easier (the code level is easier only for part of them, 9.1). ct-compare follows and reuses the pipeline | 9 |
| D8 | The gate **model comes from Warrant** (owner direction, 2026-11-24): the model definitions and goals are elaborated from the logic-gates Warrant file into a separate, reviewed model file, never written by the proof producer. Warrant growing into the repository's algorithm definition language is VER33 | 9.2, 9.6 |
| D9 | A signature binds `(claim id, goal digest, environment digest)`, where the environment is the exact ordered file list the gate fixes, not one the claim file chooses | 3.6, 3.7 |
| D7 | Versions: format, kernel logic and producer profile are three separate numbers in the header; unknown ones fail closed | 8 |

**What this resolves.** VER11a Q9 ("how does the kernel read lean4export's JSON
without a hand-written parser") is resolved in favour of "our own format"
(owner, 2026-11-24). The remaining question is narrower: how the reader of that
format is built (Q1).

**What it costs.** A converter per source prover, which has to be written and
tested. The converters are outside the trusted base, so a wrong converter fails
by producing something the kernel rejects (or something true but not what was
meant, which the signed goal catches, section 1.3). It never produces a wrong
proof of the statement that was signed.

**What it buys.**

- The trusted base no longer depends on any prover's export format; moving a
  Lean pin changes a converter, not the kernel.
- Proofs from Lean, Rocq and the HOL family are checked by one kernel, and each
  one's *extra* assumptions are visible by name.
- The repository's own decision procedures produce proofs with no Lean
  toolchain (VER11a §5.3 and Q2 become optional rather than required).
- A first, real, complete proof pipeline (the gates, section 9) that exercises
  every part of the machinery on a target small enough to review in full.

---

## 1. Purpose and trust model

### 1.1 Who is trusted

| Component | Trusted? | Why |
|---|---|---|
| `lean-kernel` (VER11a §4) | **Yes** (bucket (c) id `kernel.checker.correct`) | It decides what is a theorem |
| The **PIF reader** (section 5) | **Yes** (new bucket (c) id `pif.reader.correct`) | It decides *which declarations the kernel is shown*. A reader that drops or alters a record changes the environment |
| **SHA-256** and the **import resolver** (the code that maps the gate's ordered file list to bytes and verifies each digest) | **Yes** (part of `pif.reader.correct`; counted with the reader, not inside its 600 lines) | A wrong digest check lets a different file stand in for an allow-listed one |
| The **gate-side binding code** (theorem-kind check, footprint classification, allow-list, goal-term comparison; VER11a §7.3) | **Yes** (new (c) id `tcb.pif.gate-binding`, with the always-present `tcb.report.checker-correct`) | It decides which theorem is accepted for which claim |
| The pin file and `lean-kernel-ffi` wrapper (VER11a §4.3) | Yes, as in VER11a | Unchanged |
| The **goal printer** (the human-readable form a signer reads) | **Yes for signing** (new (c) id `tcb.goal.printer`) unless the gate checks `parse(print(G)) = G` with an independent parser (3.6) | A printer that maps two different goals to one text lets a signer sign the wrong one |
| Producers: Lean exporter, Rocq, HOL, claim elaborator, case-split, LRAT, arithmetic | **No, for validity** | Their output is a candidate; a wrong candidate is rejected. They are trusted for *meaning* only through what a human signs (1.3) |
| Converters (Lean→PIF, Rocq→PIF, HOL→PIF) | **No, for validity** | Separate crates; a converter that drops or alters a declaration yields a different environment which the kernel checks as it is. **Axiom naming** is the one place trust moves into a converter (1.4) |
| The **prelude** and the **model file** | **Reviewed text, signed by digest** (new (b) ids `pif.prelude.faithful`, `gates.model.faithful`) | They define what `Eq`, `Bool`, `Nat` and the gate functions mean; a producer must not be able to redefine them |
| `pif-fmt`, `pif-pp` (lint, pretty-printer) | No | Convenience; the kernel verdict does not depend on them |
| The grammar files and any grammar-driven parser | Depends on Q1 | Normative either way; trusted only under option (a) |

### 1.2 The picture

```
  untrusted                                         trusted
  ──────────────────────────────────────────────    ─────────────────────────────

  Lean toolchain ── lean4export ──► Lean→PIF ─┐
  Rocq / Coq ──────────────────────► Rocq→PIF ─┤
  HOL Light / HOL4 / Isabelle/HOL ─► HOL→PIF ──┤
  Warrant claim elaborator (goal) ─────────────┤          ┌──────────┐   ┌──────────┐
  case-split decision procedure ───────────────┼─ PIF ──► │ PIF      │──►│ lean-    │──► accepted
  SAT solver ─► LRAT reconstruction ───────────┤  text    │ reader   │   │ kernel   │    declarations,
  arithmetic certificates ─────────────────────┘          │ (≤ 600)  │   │ (≤ 8,500)│    axiom footprint
                                                          └──────────┘   └──────────┘
                                                                              │
  signed goals (VER31, goals_sha256) ──────────────────────────► release gate ◄┘
  allow-listed prelude digests ────────────────────────────────►  compares theorem
                                                                  type with goal
  oracles, differential only, never a dependency:
  official Lean kernel (via a PIF→Lean replay harness), second checker (KRN-019)
```

### 1.3 What a wrong producer can and cannot do

| A producer that is wrong… | Outcome |
|---|---|
| emits an ill-typed term | The kernel rejects the declaration: no theorem |
| emits a well-typed proof of a *different* statement than the claim | The gate compares the theorem's type with the signed goal term (3.6): rejected |
| redefines a trusted definition (`obs`, a gate's model, `Eq`) | The environment is the **exact ordered file list the gate fixes** (3.7), the model and prelude files are separate reviewed files, and a claim file may not declare a constant that occurs in the goal (VER11a §7.3 step 3): rejected |
| declares an axiom to make the proof go through, **including one named like a standard axiom** (`axiom propext : False`) | A claim file may declare **no axiom** except pinned ones (3.8); a name is trusted only if the kernel validated its *type* (3.5): rejected |
| points a `goal` at an axiom, an imported theorem or a duplicate claim id | `GoalTargetNotTheorem`, `GoalTargetImported`, `DuplicateGoal` (5.3) |
| supplies a wrong recursor, quotient constant or literal rule | Cannot: the format has no place for them (3.4), and a name the kernel would derive may not be declared (3.8) |
| is a converter that *mistranslates a source statement* | The result is a true Lean-logic theorem about something else. For third-party libraries we make **no claim that the converted statement is the source's statement**. For our own claims the statement is the signed goal. A mistranslation of a signed goal is caught only if the signer reads the goal; this is why the signed object is the *canonical term* (3.6) |

The last row is the honest limit of the whole design: *a converter's
faithfulness decides what a theorem is about, not whether its proof is valid*.
It also marks where the "untrusted" label is not the whole truth: producers are
untrusted for **validity**, and trust moves into them for **meaning** (the
signed goal) and for **axiom naming** (1.4).

### 1.4 Source logics' extra principles: named, pinned, and not known consistent

A source logic may assume something Lean's type theory does not (classical
logic and Hilbert choice in HOL; Rocq's `Classical_Prop.classic`, or the
`functional_extensionality` axiom). The converter must pick one of two options
per principle:

1. **Derive it**: the principle is a theorem in Lean's logic. The converter emits
   the proof; the kernel checks it. No new axiom. (HOL's excluded middle follows
   from `Classical.choice`, `propext` and function extensionality, and function
   extensionality follows from `Quot.sound`.)
2. **Name it**: declare an `axiom` whose name starts with `Src.<profile>.`, for
   example `Src.rocq.Classical_Prop.classic`.

Three rules make a name mean something:

| # | Rule |
|---|---|
| A1 | **A logic axiom is logic base only if the kernel validated its type.** `propext`, `Classical.choice` and `Quot.sound` count as the kernel logic's own axioms (bucket (a), `kernel.logic.consistent`) **only** if the constant of that name was bound by a `prim` role of the same name (`axiom.propext`, `axiom.choice`, `axiom.quotsound`, 3.5) and the kernel checked its declared type and universe parameters against the fixed statement. A constant merely *named* `propext` is a user axiom and is classified as such. (Attack closed: `axiom propext : False`.) |
| A2 | **A claim file may declare no `axiom` except** `Warrant.Assume.<id>` and registered `Src.*` axioms, and each must **match the registered statement exactly**, by type and universe parameters (the VER11a §11.1 match, extended to `Src.*`). Everything else is `AxiomNotAllowed`. Library import (K-M0 style) may declare other axioms; they are listed and classified (c) |
| A3 | **Registering a `Src.*` axiom requires a consistency argument relative to Lean's logic**, and its bucket is **never (a)**. Unregistered `Src.*` axioms are rejected for claims (Q3) |

**Consistency is not preserved by naming.** An extra principle that is
consistent in its source logic can be inconsistent with Lean's kernel theory:

- HOL's types are non-empty, but Lean's may be empty: `Src.hol.select : ∀ α,
  (α → Prop) → α` proves `False` at `α := False`.
- Rocq's univalence contradicts Lean's definitional proof irrelevance (UIP);
  Church's thesis and continuity principles contradict `Classical.choice`.

So every report that lists a `Src.*` axiom says **"relative to Lean + these
axioms; consistency with the kernel logic not established"**. The converter's
mapping of HOL avoids the first problem by *relativisation* (6.3), not by
axiom.

## 2. Decision D1: the kernel's logic stays Lean 4's

### 2.1 The decision

The kernel implements the type theory of VER11a §3.3 and nothing else. PIF
carries *terms and declarations in that theory*. It does not carry a logic.

### 2.2 The alternative that was considered and rejected

**A logical framework** (Dedukti and its λΠ-calculus modulo rewriting): the
kernel is a small checker for a dependent type theory with user-supplied
rewrite rules. Each source logic (Lean, Rocq, HOL) is a *theory* written in the
framework, and a proof imported from a prover is checked against that theory.

| | **Fixed kernel logic (chosen)** | **Logical framework, theory per logic (rejected)** |
|---|---|---|
| Kernel size | Fixed, ≈ 8,500 lines (VER11a §4.2) | Smaller core (a few thousand lines) |
| What is trusted besides the kernel | Nothing per source logic: translations are untrusted | **Each theory**: its rewrite rules *are* the logic. A bad rule set proves `False`, and checking confluence and termination of the rules is the trusted part and the hard part |
| Cross-prover reasoning | A Rocq lemma and a Lean lemma live in one logic and can be combined | Different theories do not combine without a translation between them, and Dedukti-style systems have never made that routine |
| Reuse of Lean's library and our own Lean programme (LEAN00) | Direct: the logic *is* Lean's | Needs Lean's theory encoded, then everything checked through the encoding, with a large constant-factor cost for computation |
| Computation in the kernel | Native definitional reduction, `Nat` literal acceleration (VER11a §3.3) | Reduction is rewriting; arithmetic acceleration has to be added per theory |
| The oracles of VER11a §3.1 | Official Lean kernel, nanoda, lean4lean check the same declarations | None available for our encoding |
| Fit with the owner's decisions (VER11a D1, 2026-11-23) | Exact | Reverses them |

### 2.3 Why

A framework moves trust from "the kernel" to "the kernel plus every theory",
and the theories are the part nobody can read quickly. The owner's rule is that
everything except the compiler must be verified (VER31 §3.4); a framework
multiplies the number of things that must be. A fixed logic keeps exactly one
thing to trust, and the cost of supporting another prover is a *translation*,
which is untrusted and testable. The price (a translation can only target
what Lean's logic can express, so Rocq features outside it are unsupported,
section 6) is accepted: the repository's goals are the gates, ct-compare and
Lean, all of which fit.

---

## 3. The data model

Abstract, independent of syntax. Everything here has a one-to-one image in the
kernel crate (`lean-kernel`: `Name`, `Level`, `Expr`/`ExprStore`,
`Declaration`, `Environment`). The text syntax is section 4.

### 3.1 Names, levels, expressions

**Names.** A name is a list of components, each a string or a natural number,
as `lean_kernel::Name` (`NamePart::Str`, `NamePart::Num`). The empty list is
the anonymous name. Names have **no** hygiene or macro scopes at this level.

**Universe levels.** `zero`, `succ l`, `max l₁ l₂`, `imax l₁ l₂`, `param n`.
Equality is up to the normalisation the kernel implements (`levels_equiv`).

**Expressions** form a DAG: every node has an index in a numbered table, and a
node may be referenced by any later record. The kinds are exactly those of
`ExprKind` (VER11a §3.3), with the free-variable kind excluded because
declarations are closed:

| Kind | Fields | Meaning |
|---|---|---|
| `var i` | de Bruijn index | bound variable |
| `sort l` | level | `Sort l` (`Prop` is `sort zero`, `Type` is `sort (succ zero)`) |
| `const n [l…]` | name, levels | constant with universe arguments |
| `app f a` | two nodes | application |
| `lam b n t e` | binder info, binder name, domain, body | `fun (n : t) => e`, body has the binder as index 0 |
| `pi b n t e` | same | dependent function type |
| `let n t v e` | name, type, value, body | `let n : t := v; e` |
| `nat k` | natural number | `Nat` literal (needs feature `lits`) |
| `str s` | string | `String` literal (needs feature `lits`) |
| `proj s i e` | structure name, field index, node | projection |

Binder info (`default`, `implicit`, `strict`, `inst`) and binder names are
carried because producers and printers need them; the kernel ignores them for
definitional equality, as VER11a §3.3 says metadata is ignored. Metavariables
and free variables **do not exist** in PIF. A term with a loose bound variable
is rejected.

### 3.2 Declarations

Every declaration has a name, a list of universe parameter names, and a type.

| Kind | Further fields | Kernel action |
|---|---|---|
| `axiom` | none | add; classified in the footprint. In a claim file only the pinned axioms of rule A2 (1.4) are allowed |
| `def` | value, reducibility hint (`@abbrev`, `@opaque`, `@regular h`) | check value against type, add |
| `theorem` | value | check; the type must be a `Prop` (VER11a §3.3) |
| `opaque` | value | check, add with the body hidden from δ |
| `inductive` (a block) | see 3.3 | check, then **derive** constructors' use, recursors and the block's facts |
| `quot` | none | check that `Eq` has the expected shape, then add `Quot`, `Quot.mk`, `Quot.lift`, `Quot.ind` |
| `prim role name` | a role and the constant playing it | validate the declaration against the role (3.5); single assignment |
| `goal id name digest` | claim id, theorem name, digest of the goal term | no kernel action; recorded and checked by the reader (5.3) and the gate (3.6) |

The environment is **append-only**. A declaration may mention only declarations
accepted before it (in this file or an imported one). Redeclaring a name is an
error. This is the kernel's existing rule (VER11a §4.1), now also a property of
the syntax: all references go backwards.

`unsafe` and `partial` declarations do not exist in PIF. A converter that meets
one skips it and lists it in its own report (VER11a §4.3); a later declaration
that depended on it cannot be written.

### 3.3 The inductive block

The shape is Lean's `inductDecl`, and the one the kernel's inductive slice
implements (KRN-002 slice 3a):

```
inductive  <universe parameters>  <number of parameters>  <number of types>
  type  <name> <type>  <number of constructors>
    ctor  <name> <type>
    ...
  (more types, for a mutual block)
```

- `universe parameters`: the block's level parameter names, shared by all its
  types and constructors.
- `number of parameters`: how many leading arguments are parameters (the same
  for every type of the block); the kernel checks that every type and
  constructor begins with that many identical binders.
- Each **type** has a name and a full type (`Π params indices, Sort u`).
- Each **constructor** has a name and a full type that returns the type it
  belongs to, with the parameters as its leading binders.

What the producer does **not** give: recursors, `casesOn`, `below`, `brecOn`,
`noConfusion`, `sizeOf`, any auxiliary definition. They are definitions
produced by Lean's *elaborator*, not by its kernel, and a producer that needs
them writes them as ordinary `def`s on top of the derived recursor. The fields
of Lean's `InductiveVal` that are *consequences* of the declaration (number of
indices, `isRec`, `isReflexive`, `isNested`, `all`, number of nested auxiliary
types) are computed by the kernel, never read. The kernel's checks are those of
VER11a §3.3 and §4.2: positivity, universe constraints, the large-elimination
(subsingleton) rule, K-like reduction eligibility, structure detection.

A mutual block uses the feature `mutual`; a block with nested occurrences
(`List` of the type being defined, say) uses `nested`. The kernel rejects a
block needing a feature the file does not declare, and rejects a feature it does
not implement yet (so slice 3a can ship with single non-nested blocks, and the
format does not change when mutual and nested support arrive).

### 3.4 Why the kernel derives recursors, and why the format has no field for them

A recursor's type and its ι rules are a function of the inductive declaration.
If a producer could supply a recursor, a wrong or hostile one (a recursor whose
rule maps `Nat.rec … (succ n)` to something else) would add an inconsistent
reduction rule to the trusted logic. Lean's kernel generates them itself for
this reason, and so does ours (VER11a §3.3). The converter from Lean drops the
exported recursors; the differential harness (section 10) compares the
recursors the kernel *derived* with the ones Lean exported, which is a strong
test of the derivation.

### 3.5 Primitive bindings

Literal acceleration is sound only if the constants it accelerates *are* the
expected definitions (VER11a §3.3, "Primitive definitions are validated"), and
a logic axiom's name means something only if its *type* is the expected one
(1.4, rule A1). PIF therefore says which constant plays which role, and the
kernel validates the declaration before the role takes effect. Shown with
constant names for readability; on disk the operand is a name index.

```
prim nat Nat           -- the type Nat
prim nat.zero Nat.zero
prim nat.succ Nat.succ
prim nat.add Nat.add   -- … and sub, mul, div, mod, gcd, beq, ble, land, lor,
                       --   xor, shiftleft, shiftright, pow, log2
prim string String     -- and string.mk, char, char.ofnat, list.nil, list.cons
prim eq Eq             -- the equality type, used by the quotient package check
prim axiom.propext propext     -- type checked against the fixed statement
prim axiom.choice Classical.choice
prim axiom.quotsound Quot.sound
```

The set of roles is closed and belongs to the kernel-logic version (section 8).
Rules (each an error in 5.3):

- A role the kernel does not know is an error (`UnknownRole`).
- **Roles are single-assignment per environment** (`DuplicateRole`): once `nat`
  is bound, no later file may rebind it. Otherwise the node `nat 5` would change
  type between two declarations and every per-node cache would be wrong.
- `enat` and `estr` are errors until the roles they need (`nat`, `string`, `char`,
  `list.*`) are bound (`LiteralBeforeRole`).
- A `prim` record is valid only after the named constant exists, and the
  kernel's validation is of the declaration *as it is in the environment*.
- **Features and bindings are environment-wide and printed in the report.** The
  **gate fixes which features a claim's environment may enable**; a claim file
  that declares `lits` or `quotient` when the gate did not allow it is rejected
  (`FeatureNotAllowed`). Today a claim file could otherwise turn on literal
  acceleration itself.

**A file that uses no literal acceleration and no quotient declares neither
feature and contains no `prim` record for them**: the gates' first milestone
(section 9) is such a file, which keeps the literal and quotient machinery out
of its trusted footprint. (Its *prelude* binds `axiom.propext`, `axiom.choice`
and `axiom.quotsound` only if it declares those axioms, which G-M1 does not
need; a footprint then simply lists none.)

### 3.6 Goal bindings

```
goal "<claim id>" <theorem name> sha256:<64 hex>
```

A goal record maps a theorem to the Warrant claim it is meant to prove. The
rules, each a numbered check in 5.3 or a gate rule:

| # | Rule |
|---|---|
| G1 | The target must be a **`theorem` declared in this file**: not an axiom, not a definition, not an imported constant. (A target that were an `axiom T : G` would "exist" and, with a value-only footprint walk, have an empty footprint.) |
| G2 | A constant's **footprint includes itself**: the axiom walk starts from the constant, not only from its value |
| G3 | **Claim ids are unique per environment** (`DuplicateGoal`) |
| G4 | The **digest in the record is a hint**. The gate always recomputes the goal digest from the theorem's type and from its own elaboration of the claim, and compares |
| G5 | The digest is over the **goal term's canonical encoding (CTE)**, not over a printed text (below) |
| G6 | The signature covers `(claim id, goal digest, environment_sha256)` (3.7). A goal digest alone binds the *text of a statement*, not what its names mean |

**The canonical term encoding (CTE).** A printed goal is not injective, so
the object the signature binds is the term itself, serialised by a fixed
tree-unfolded prefix encoding that depends on no index and no sharing:

```
name   ::= '<' [comp { ',' comp }] '>'      comp ::= STRING | NAT
level  ::= 'z' | 's(' level ')' | 'm(' level ',' level ')' | 'i(' level ',' level ')' | 'p' name
expr   ::= 'v' NAT | 'S' level | 'C' name '[' [level { ',' level }] ']'
         | 'A(' expr ',' expr ')'
         | 'L' b '(' name ',' expr ',' expr ')' | 'P' b '(' name ',' expr ',' expr ')'
         | 'Z(' name ',' expr ',' expr ',' expr ')' | 'N' NAT | 'T' STRING
         | 'J(' name ',' NAT ',' expr ')'                 b ::= 'd' | 'i' | 's' | 'c'
```

STRING and NAT are written exactly as in PIF (4.1). The digest is the SHA-256 of
this byte string, in UTF-8 (it is ASCII). The CTE of a theorem's *type* is
computed by the gate (a tree unfolding bounded to 1 MiB; a goal larger than that
is refused) and by the claim elaborator, and the two must be equal. The worked
example's CTE begins `Pd(<"a">,C<"Bool">[],Pd(<"b">,C<"Bool">[],A(A(A(C<"Eq">[s(z)],…`.

**The printer.** The text a human signs is the printer's output. Printing is not
injective unless the inputs are restricted, so:

- name components are restricted (3.8): no `.`, no control or format characters,
  not empty;
- the printer **refuses shadowed binders** (`∀ (a a : Bool), f #1 #0` and
  `f #0 #0` would otherwise print alike) and **never takes binder names from the
  shared expression store** (it takes them from the CTE; `ExprStore` keeps the
  binder metadata of whichever spelling it interned first, which may have been
  chosen by an attacker);
- the gate checks `parse(print(G)) = G`, in CTE, using a parser of the print
  format that shares no code with the printer; or, until that exists, the printer
  is the (c) id `tcb.goal.printer`.

The signed object is therefore the *CTE digest, accompanied by the readable
text*, and the gate recomputes both.

### 3.7 Files compose by digest, and the environment is fixed by the gate

A PIF file is a **delta over an environment**. Indices (names, levels,
expressions) are local to a file; two files refer to each other **only by
constant name**.

| # | Rule |
|---|---|
| E1 | **The gate supplies the environment**: an ordered list of files (prelude, model, libraries, then the claim file), fixed per claim by the gate, not chosen by the claim file |
| E2 | The environment is a **linear chain**: a file's `import` lines are exactly the digests of **all files before it in the environment, in order**. The reader checks `import` lines against the list the resolver was given (`ImportMismatch`). This makes diamond imports impossible by construction, and "load each digest once" trivial |
| E3 | Each file is loaded once. `environment_sha256` (8.3) is folded over this flattened order |
| E4 | **Every file in the chain must be on the gate's allow-list**, transitively; there is no implicit trust of an imported file's own imports |
| E5 | The resolver (a trusted component, 1.1) maps a digest to bytes and verifies SHA-256 of the bytes it hands the reader; the reader does no I/O |
| E6 | **The signature covers the environment digest** (G6), so a claim file cannot pick imports under which the same names mean something else |

### 3.8 Reserved names, derived names and name components

**Derived names.** A `inductive` block derives, for each type `T`: the recursor
`T.rec` (and for a nested block its auxiliary types and their recursors); `quot`
derives `Quot`, `Quot.mk`, `Quot.lift`, `Quot.ind`. The kernel **rejects** an
`inductive` block or `quot` if **any name it would derive is already taken**
(`DerivedNameTaken`): so `axiom Foo.rec` before `inductive Foo`, a constructor
named `Foo.rec`, and `Quot.mk` declared before `quot` are all errors. A later
declaration with a derived name is already an error by the append-only rule
(`AlreadyDeclared`).

**The anonymous name** (index 0) may not name a declaration, a binder-bearing
constant or a parameter; it appears only as the parent of a top-level name
(`AnonymousName`).

**Name components.** A string component must be non-empty, must **not contain
`.`**, and must contain no control character (U+0000 to U+001F, U+007F to
U+009F) and no format character (U+200B to U+200F, U+202A to U+202E, U+2060 to
U+2064, U+2066 to U+206F, U+FEFF) or private-use character. A numeric component
`nnum p 3` and a string component `nstr p "3"` are **different names** and the
printer must print them differently (`3` against `"3"`). **Names a claim or a
model file *declares* are further restricted to the ASCII class
`[A-Za-z_][A-Za-z0-9_']*` per component**, so homoglyph and zero-width tricks
cannot occur in anything a signer reads; non-ASCII appears only in binder names
and in imported library names the signer is not asked to read.

## 4. The concrete syntax

Defined normatively by `code/grammars/pif/pif.tokens` and
`code/grammars/pif/pif.grammar` (a backlog item, PIF-001; this section is the
intended content). The format is designed so that the *reader is trivial*:
split a line on single spaces, look at the first word.

### 4.1 The lexical form: the grammar is the whole language

**Decision.** There is no pre-pass and no separate "line discipline". The
**tokens carry their own spacing**, so the grammar describes *exactly* the byte
strings the reader accepts, and a hand-written reader and a grammar-driven parser
cannot disagree about whitespace.

*Alternatives:* (i) lexer skips spaces and a shared pre-pass rejects runs (the
first draft: rejected, because a shared pre-pass is never tested by the
differential and `nstr 1 0"Bool"` or `[ 1 2 ]` passed the grammar); (ii) an
explicit `SP` token on every field (correct but noisy). *Why:* a field token that
**begins with its single separator space** is the same language as (ii) with the
noise inside the token file.

| Property | How it is enforced |
|---|---|
| The file is ASCII: only LF (0x0A) and bytes 0x20 to 0x7E occur | No token can match any other byte, so any other byte is a lexical error. (The first draft said UTF-8; a valid file is pure ASCII, because non-ASCII text is written `\u{h}`) |
| Exactly one space between fields; none leading, trailing or doubled; no tabs, CR, blank lines or comments | Every non-first field token is `" "` followed by the field. A line-start keyword has no space. Nothing else matches a space |
| Brackets are byte-exact: `[]`, `[1]`, `[1 2]` | The opening token is `" ["`, the first element has no space, later elements have one, the closing token is `"]"` |
| A **space inside a string is written `\u{20}`** | The STRING token excludes 0x20. So a line is exactly its fields joined by single spaces, and a reader may split on space |
| Bytes after the `end` line are an error | `file` ends with `trailer` and then end of input; there is no `skip` |
| Numerals: decimal, no sign, no leading zeros, no `_` | The NAT regexes |
| Strings: printable ASCII except `"`, `\` and space, with the escapes `\"`, `\\` and `\u{h}` (lowercase hex, no leading zeros) | The STRING regex (shape) plus the value rules R-LEX-1 to R-LEX-3 in 5.3 (the regex cannot say "a Unicode scalar value") |
| Digests: `sha256:` and 64 lowercase hex digits | The SHA256 token |

Numeric bounds (checked as values, 5.3): indices fit 32 bits and the per-file
record cap of section 7; an `nnum` name component fits 64 bits (the kernel's
`NamePart::Num` is `u64`); an `eproj` field index and `@regular` heights fit
32 bits (the index at most 65,535); a `nat` literal has at most 10,000 digits.

### 4.2 `pif.tokens` (intended content)

```
# pif.tokens — Proof Interchange Format, version 1
# @version 1
# No skip section: spacing is part of the tokens (VER32 section 4.1).
# A line-start keyword has no space; every other field begins with one space.

NAME      = /[a-z]+/                      # line-start record keywords only
NAT       = /0|[1-9][0-9]*/               # first element of a bracket list
FNAT      = / (0|[1-9][0-9]*)/
FSTRING   = / "([\x21\x23-\x5b\x5d-\x7e]|\\"|\\\\|\\u\{[1-9a-f][0-9a-f]{0,5}\})*"/
FSHA256   = / sha256:[0-9a-f]{64}/
FIDENT    = / [a-z][a-z0-9_.]*/           # roles, features, profile ids
LB        = " ["
RB        = "]"
FDEFAULT  = " default"
FIMPLICIT = " implicit"
FSTRICT   = " strict"
FINST     = " inst"
FABBREV   = " @abbrev"
FOPAQUE   = " @opaque"
FREGULAR  = " @regular"
NEWLINE   = "\n"

keywords:
  pif logic profile features import
  nstr nnum lsucc lmax limax lparam
  evar esort econst eapp elam epi elet enat estr eproj
  axiom def theorem opaque inductive itype ictor quot prim goal end
```

Features (`lits`, `quotient`, `mutual`, `nested`), profile ids (`lean4`, `rocq`,
`hol`, `native`) and `prim` roles (`nat.add`, `axiom.propext`, …) are `FIDENT`s;
the kernel-logic version fixes which are valid. (`quotient`, not `quot`, so the
feature name does not collide with the `quot` keyword.) The regular-expression
dialect is the repository's `.tokens` dialect; the byte classes above are the
intended meaning.

### 4.3 `pif.grammar` (intended content)

```
file      = header { record } trailer ;

header    = "pif" FNAT NEWLINE
            "logic" FNAT NEWLINE
            "profile" FIDENT FSTRING NEWLINE
            "features" { FIDENT } NEWLINE
            { "import" FSHA256 NEWLINE } ;

record    = name_rec | level_rec | expr_rec | decl ;

name_rec  = "nstr" FNAT FNAT FSTRING NEWLINE        # id, parent, component
          | "nnum" FNAT FNAT FNAT NEWLINE ;

level_rec = "lsucc" FNAT FNAT NEWLINE                # id, level
          | "lmax" FNAT FNAT FNAT NEWLINE
          | "limax" FNAT FNAT FNAT NEWLINE
          | "lparam" FNAT FNAT NEWLINE ;             # id, name

expr_rec  = "evar" FNAT FNAT NEWLINE                 # id, de Bruijn index
          | "esort" FNAT FNAT NEWLINE                # id, level
          | "econst" FNAT FNAT list NEWLINE          # id, name, levels
          | "eapp" FNAT FNAT FNAT NEWLINE
          | "elam" FNAT binfo FNAT FNAT FNAT NEWLINE # id binfo name type body
          | "epi" FNAT binfo FNAT FNAT FNAT NEWLINE
          | "elet" FNAT FNAT FNAT FNAT FNAT NEWLINE  # id name type value body
          | "enat" FNAT FNAT NEWLINE
          | "estr" FNAT FSTRING NEWLINE
          | "eproj" FNAT FNAT FNAT FNAT NEWLINE ;    # id struct-name index expr

decl      = "axiom" FNAT list FNAT NEWLINE               # name, params, type
          | "def" FNAT list FNAT FNAT hint NEWLINE       # name params type value hint
          | "theorem" FNAT list FNAT FNAT NEWLINE
          | "opaque" FNAT list FNAT FNAT NEWLINE
          | inductive
          | "quot" NEWLINE
          | "prim" FIDENT FNAT NEWLINE
          | "goal" FSTRING FNAT FSHA256 NEWLINE ;

inductive = "inductive" list FNAT FNAT NEWLINE itype { itype } ;
itype     = "itype" FNAT FNAT FNAT NEWLINE { ictor } ;   # name type n-ctors
ictor     = "ictor" FNAT FNAT NEWLINE ;                  # name type

list      = LB [ NAT { FNAT } ] RB ;
binfo     = FDEFAULT | FIMPLICIT | FSTRICT | FINST ;
hint      = FABBREV | FOPAQUE | FREGULAR FNAT ;

trailer   = "end" FNAT NEWLINE ;
```

**LL(1).** Every alternative of `record`, `expr_rec`, `level_rec`, `name_rec`,
`decl`, `binfo` and `hint` starts with a distinct token; `list` is decided by `NAT`
against `RB`; the constructors of a type are decided by whether the next token is
`ictor`. Counts (the number of types in a block, of constructors in a type, of
elements in a list) are checked by the reader (rules R-CNT in 5.3), because a
grammar cannot count; the counts written in a record are **validated, never
trusted**.

### 4.4 What each record means: semantics the grammar does not carry

| Rule | Statement |
|---|---|
| Indices | Names, levels and expressions have **separate** index spaces. Name `0` is the anonymous name; level `0` is `zero`. Every other record defines the next index of its space: **the first of a kind is 1 and the indices are dense and increasing**. A record may reference only an index already defined (backward only), so a forward or self reference is a syntax error and cycles cannot be written |
| `nstr i p s` | name `i` is the name `p` extended by the string component `s` (`p` is `0` for a top-level name); `s` obeys the component rules of 3.8 |
| `lparam i n` | level `i` is the universe parameter named by name `n` |
| `elam i b n t e` | `e` is the body in which the binder is index 0. The reader does not check de Bruijn indices against the binder depth; the kernel's closedness check does |
| list on a declaration | name indices of the universe parameters, in order, distinct |
| `econst i n [l…]` | level indices; the count must match the constant's declaration (the kernel checks) |
| `inductive ps np nt` | `ps` universe parameters, `np` parameters, then **exactly** `nt` `itype` records |
| `end k` | `k` is the number of record lines between the header and `end`, counting every physical line. It detects **accidental** truncation and corruption; it is not a security property (a hostile producer can write a consistent `k`) |

### 4.5 Canonical bytes, and what "canonical" does not mean

The tokens fix **lexical** canonical form: one spelling of every numeral, string
and digest, one separator. That is what makes `file_sha256` a function of the
file's content *as written*.

**It does not make the file a function of its meaning.** Two producers may write
the same declarations with different index numbering, sharing, record order
within a declaration, or binder names, and get different bytes and different
digests. Digests identify **exact bytes**. Nothing in the soundness argument
depends on canonicity: the kernel checks exactly the records it reads. Canonicity
matters only for (i) stable digests of files that are committed and allow-listed
(the file *is* its bytes), and (ii) a deny-list of known-bad digests, which an
attacker defeats by re-spelling. The first draft claimed "byte-for-byte
unique"; that was false and is withdrawn.

`pif-fmt` (untrusted) lints committed files for **recommended** shape, so that
reviewers see consistent output:

| # | Lint | Notes |
|---|---|---|
| C1 | No two records of one space have the same content | The reader does **not** enforce it: the kernel's interning ignores binder names and infos, names and levels are not interned, and the store is shared with imports and with terms the kernel derives itself (recursor types), so "duplicate" has no clean trusted definition |
| C2 | Each declaration's records are emitted in creation order of a reference emitter, immediately before the declaration | A convention, not a proof of uniqueness |
| C3 | No record is unreferenced | |

### 4.6 Worked example

The file below is complete and lexically canonical. It defines `Bool` and `Eq`
as inductive types, defines `Bool.and` with `Bool.rec`, and proves that `and` is
commutative over `Bool` by case analysis, with a goal binding. It needs no
literals, no quotient and no imports. **It is a format illustration in one file.**
For a gate claim the first three groups would be the prelude file (`Bool`, `Eq`),
the model file (`Bool.and`, elaborated from Warrant, 9.2) and the claim file (the
theorem and its goal), chained as in 3.7. The numbering was produced by a throwaway
script (creation order); that the file type-checks is the corpus's first
positive test (PIF-005), not something this document has run.

```
pif 1
logic 1
profile native "pif-case-split\u{20}0.1"
features
nstr 1 0 "Bool"
nstr 2 1 "false"
nstr 3 1 "true"
lsucc 1 0
esort 1 1
econst 2 1 []
inductive [] 0 1
itype 1 1 2
ictor 2 2
ictor 3 2
nstr 4 0 "Eq"
nstr 5 4 "refl"
nstr 6 0 "u"
lparam 2 6
esort 3 2
esort 4 0
evar 5 0
evar 6 1
nstr 7 0 "\u{3b1}"
nstr 8 0 "a"
nstr 9 0 "_"
epi 7 default 9 6 4
epi 8 default 8 5 7
epi 9 default 7 3 8
econst 10 4 [2]
eapp 11 10 6
eapp 12 11 5
eapp 13 12 5
epi 14 default 8 5 13
epi 15 default 7 3 14
inductive [6] 2 1
itype 4 9 1
ictor 5 15
nstr 10 1 "and"
nstr 11 1 "rec"
econst 16 11 [1]
econst 17 2 []
econst 18 3 []
nstr 12 0 "b"
elam 19 default 9 2 2
eapp 20 16 19
eapp 21 20 17
eapp 22 21 5
eapp 23 22 6
elam 24 default 12 2 23
elam 25 default 8 2 24
epi 26 default 12 2 2
epi 27 default 8 2 26
def 10 [] 27 25 @regular 1
econst 28 10 []
econst 29 4 [1]
eapp 30 28 6
eapp 31 30 5
eapp 32 28 5
eapp 33 32 6
eapp 34 29 2
eapp 35 34 31
eapp 36 35 33
epi 37 default 12 2 36
epi 38 default 8 2 37
elam 39 default 8 2 37
econst 40 11 [0]
econst 41 5 [1]
eapp 42 41 2
eapp 43 42 17
eapp 44 42 18
eapp 45 28 17
eapp 46 45 5
eapp 47 32 17
eapp 48 34 46
eapp 49 48 47
elam 50 default 12 2 49
eapp 51 40 50
eapp 52 51 43
eapp 53 52 43
eapp 54 53 5
elam 55 default 12 2 54
eapp 56 28 18
eapp 57 56 5
eapp 58 32 18
eapp 59 34 57
eapp 60 59 58
elam 61 default 12 2 60
eapp 62 40 61
eapp 63 62 43
eapp 64 63 44
eapp 65 64 5
elam 66 default 12 2 65
eapp 67 40 39
eapp 68 67 55
eapp 69 68 66
eapp 70 69 5
elam 71 default 8 2 70
nstr 13 1 "and_comm"
theorem 13 [] 38 71
goal "gates.and_comm" 13 sha256:b8800684c33a8496d5f64ea34b98bdddb6da7f0ebf1030d3a68e530afa484fa0
end 96
```

How to read it (the pretty-printer `pif-pp` produces this view; it is not part
of the file):

| Lines | Meaning |
|---|---|
| 5 to 14 | `inductive Bool : Type | false : Bool | true : Bool`. Name `1` is `Bool`, `2` is `Bool.false`, `3` is `Bool.true`; level `1` is `succ zero`; expression `1` is `Type`; expression `2` is the constant `Bool` |
| 15 to 37 | `inductive Eq.{u} {α : Sort u} (a : α) : α → Prop | refl : Eq a a`. Two parameters (`α`, `a`), one index. Expression `7` is `Π (_ : #1), Prop`, `8` adds `Π (a : #0)` and `9` adds `Π (α : Sort u)`, the type of `Eq`. The constructor's type (`15`) is `Π (α : Sort u) (a : α), Eq.{u} α a a` |
| 38 to 53 | `def Bool.and (a b : Bool) : Bool := Bool.rec.{1} (fun _ => Bool) false b a`. `Bool.rec` is **not declared in the file**; the kernel derived it from the block. Its minor premises come in constructor order: the `false` case first, so `and false b = false` and `and true b = b` |
| 54 to 99 | `theorem Bool.and_comm : ∀ (a b : Bool), Eq.{1} Bool (Bool.and a b) (Bool.and b a)`. The proof is `fun a => Bool.rec.{0} M caseF caseT a`, where `M` (expression `39`) is the motive `fun a => ∀ b, …` and each case splits on `b` with `Bool.rec.{0}` again. Every one of the four leaves is `Eq.refl.{1} Bool false` or `… true`; the kernel accepts them because `Bool.and` unfolds and `Bool.rec` reduces on a constructor (ι), so `Eq Bool (and false true) (and true false)` and `Eq Bool false false` are definitionally equal |
| 100 | the goal binding for the gate: the digest is the SHA-256 of the CTE (3.6) of the theorem's type, `Pd(<"a">,C<"Bool">[],Pd(<"b">,C<"Bool">[],A(A(A(C<"Eq">[s(z)],C<"Bool">[]),…`, not of a printed text |
| 101 | `end 96`: lines 5 to 100 are 96 record lines |

The universe levels are `Bool.rec.{1}` for the data motive (`fun _ => Bool` is
a `Type`, that is `Sort 1`) and `Bool.rec.{0}` for the propositional ones.

### 4.7 A compact binary form is deferred

A binary encoding (varints, string tables) would shrink Lean's `Init` by an
order of magnitude. It is deferred until K-M0 measures the text form (PIF-014).
It is defined as a mechanical bijection of the text records, with the same
canonical digest taken over the **text** form, so that a binary file and its
text are the same artefact.

---

## 5. The trusted reader

### 5.1 What it is and the budget

The reader turns a PIF byte stream into calls on the kernel: build names,
levels and expressions in the kernel's `ExprStore`, add declarations to the
`Environment`, build and add inductive blocks, add `prim` validations. It is
the "Export reader" row of VER11a §4.2 (≤ 600 lines, inside the kernel's
8,500-line budget) and replaces the lean4export reader in that row.

**What the 600 lines cover and what they do not.** 600 lines is tight but
credible for *parsing and the checks of 5.3* if duplicate-record detection is
dropped (4.5, C1). It does **not** include, and the trusted base counts
separately: the SHA-256 implementation, the import resolver (E5), big-number
parsing of `nat` literals (done by `bignum-core`, whose cost is quadratic in
the digit count, hence the 10,000-digit cap of section 7), the gate-side goal
checks (3.6) and the closed-form CTE computation. These are listed in 1.1.

| Part | Lines (budget) | Contents |
|---|---|---|
| Tokenising | ≤ 150 | the token rules of 4.2 as code, bounded line reading (7) |
| Index tables and node builders | ≤ 250 | per-space tables; dense, backward-only indices; depth bookkeeping without recursion |
| Declarations and blocks | ≤ 100 | `axiom`, `def`, `theorem`, `opaque`, inductive blocks with counts, `quot`, `prim`, `goal` |
| Header, imports, trailer, limits | ≤ 100 | version checks, feature check, `end k`, error type |

It does no I/O except reading the bounded byte stream the resolver gives it,
uses no recursion on input-controlled depth, has `#![forbid(unsafe_code)]`, and
is literate like the rest of the kernel.

### 5.2 Owner question Q1: how the reader is built

The repository rule is that lexers and parsers are grammar-driven, never
hand-written. A *trusted* reader must also be small and auditable. These
conflict, as VER11a Q9 said for JSON. The grammar of section 4 is normative in
both options, and describes exactly the accepted language (4.1). The options:

| | **(a) Generated from the grammar** | **(b) Tiny hand-written reader, differentially tested** |
|---|---|---|
| How | `grammar-tools` generates the lexer and parser from `pif.tokens` and `pif.grammar`; a thin semantic layer feeds the kernel | A ≤ 600-line reader; the grammar-driven parser exists too, and the two are run **on every corpus file and on generated files**, comparing the **sequence of kernel calls and the resulting environment** (5.4) |
| Trusted base added | `grammar-tools` (lexer and parser engines, grammar compiler, generated code), counted against the budget; **plus the semantic layer**, which is itself hand-written code of the same kind as (b)'s reader | Only the reader itself |
| Failure mode | An engine bug (a token matched too liberally) silently changes what the kernel is shown | A divergence between reader and grammar parser is found by the differential, in CI, before any gate relies on the reader |
| Rule compliance | Full, though the semantic layer is a second hand-written reader | Needs an owner exemption: a hand-written *trusted* reader of a grammar-defined format |

**Recommendation: (b).** Under (a) the "grammar-driven" parser only produces
tokens and a tree; the part that decides *which kernel call a record becomes* is
still hand-written, so (a) does not remove a hand-written reader, it adds an
engine under it. Under (b) the grammar-driven parser, plus its own semantic
layer written separately, is an independent implementation that *checks* the
reader. The exemption is narrow (this reader only), and (a) can replace (b) if
`grammar-tools` is ever audited to the level of the kernel (PIF-013).

**Fable's view (round 1): agree with (b), on four conditions**, all adopted:
(1) the differential compares kernel calls, not tokens (5.4); (2) the grammar and
the lexical rules describe the same language (4.1, done by construction);
(3) the grammar side's semantic layer is acknowledged as a second hand-written
reader (the table above, and PIF-007 writes it independently); (4) SHA-256 and
import handling are counted in the trusted base (1.1, 5.1).

### 5.3 The normative list of reader checks

Every check the reader makes is one rule below, with an error kind. The list
replaces the earlier spread over §4, §7 and §10; a corpus file exists for each
rule (section 10), and **both** the reader and the grammar-driven parser's
semantic layer must implement exactly this list.

| Rule | Check | Error |
|---|---|---|
| R-LEX-1 | The bytes are exactly those the tokens of 4.2 match (ASCII, one separator, bracket spacing, no bytes after `end`) | `BadLexicalForm` |
| R-LEX-2 | A `\u{h}` escape denotes a Unicode scalar value (not a surrogate, at most 10FFFF) | `BadEscape` |
| R-LEX-3 | A `\u{h}` escape is used only for a code point that has no direct spelling: not `"` or `\` (which use `\"`, `\\`), not a printable ASCII byte 0x21 to 0x7E | `NonCanonicalEscape` |
| R-LEX-4 | Numbers fit their bounds (4.1) | `NumberOutOfRange` |
| R-VER-1 | `pif N` is in the reader's list of format versions; `logic N` is known and not withdrawn | `UnknownVersion`, `WithdrawnLogicVersion` |
| R-VER-2 | Each `features` operand is implemented by the kernel and **allowed by the gate** for this environment | `UnknownFeature`, `FeatureNotAllowed` |
| R-IMP-1 | The `import` lines equal, in order, the digests of all files before this one in the gate's environment | `ImportMismatch` |
| R-IMP-2 | The resolver verified each file's SHA-256 | `DigestMismatch` |
| R-IDX-1 | Each record defines the next index of its space (dense, starting at 1; name 0 and level 0 are predefined) | `IndexNotDense` |
| R-IDX-2 | Every operand index refers to an index already defined (backward only) | `ForwardReference` |
| R-NAM-1 | Name components obey 3.8 (no `.`, not empty, no control or format characters) | `BadNameComponent` |
| R-NAM-2 | A declared name is not the anonymous name | `AnonymousName` |
| R-CNT-1 | `list` elements are distinct where a set is meant (universe parameters); `inductive` has exactly its declared number of `itype`, each `itype` exactly its `ictor`s | `CountMismatch` |
| R-CNT-2 | `end k` equals the number of record lines | `TrailerMismatch` |
| R-FEA-1 | A record that needs a feature uses a declared one: `enat`/`estr` need `lits`; `quot` needs `quotient`; an `inductive` with more than one type needs `mutual` | `FeatureNotDeclared` |
| R-PRM-1 | A `prim` role is known | `UnknownRole` |
| R-PRM-2 | A role is bound at most once per environment | `DuplicateRole` |
| R-PRM-3 | The named constant exists, and the kernel validated its declaration against the role (3.5) | `RoleValidation` |
| R-PRM-4 | `enat`/`estr` come only after the roles they need are bound | `LiteralBeforeRole` |
| R-AX-1 | In a claim file, an `axiom` is `Warrant.Assume.<id>` or a registered `Src.*` axiom matching its registered statement exactly (type, universe parameters) | `AxiomNotAllowed` |
| R-DER-1 | An `inductive` block or `quot` is rejected if a name it would derive is already taken (3.8) | `DerivedNameTaken` |
| R-GOL-1 | A `goal` target is a `theorem` **declared in this file** | `GoalTargetNotTheorem`, `GoalTargetImported` |
| R-GOL-2 | Claim ids are unique in the environment | `DuplicateGoal` |
| R-DEC-1 | A declaration name is not already declared (the kernel's `AlreadyDeclared`) | forwarded |
| R-LIM-1 | The limits of section 7 hold, checked **before allocation** | `LimitExceeded(kind)` |

Everything else (type correctness, positivity, universe constraints, the shape of
`Eq`, closedness of terms) is the kernel's, and its errors are forwarded as
`KernelError`. **There is no duplicate-content check** (4.5, C1) and no
canonical-order check.

### 5.4 What the differential compares, and how the reader is tested

The dangerous reader bugs are *semantic*: a `theorem` record added as an axiom,
an off-by-one in an index table, a dropped `prim`. Comparing tokens does not catch
them. So:

1. The reader and the grammar-driven parser (with its independently written
   semantic layer) each produce the **sequence of kernel calls**: for each record,
   the call kind, the name, the type, the value, the hint, the universe
   parameters, the inductive block's shape, the `prim` binding. The two sequences
   are compared for equality, and then the **resulting environments** (every
   declaration with its kind) are compared.
2. The **corpus** (section 10) includes an ill-typed theorem and a `theorem`
   written to look like an axiom, **routed through the reader** to the kernel, so
   that the reader's handling of kinds is exercised end to end.
3. **Mutation testing of the reader itself**: each rule of 5.3 is mutated in the
   reader's source (drop a check, off-by-one an index, swap two roles); the corpus
   must kill every mutant.
4. The reader is covered by the hostile corpus of section 10.

## 6. Logic profiles

A **profile** is a converter's contract with the kernel: how a source logic maps
onto Lean 4's type theory, which extra principles it adds as named axioms, and
what it does not support yet. The header's `profile` line is a *label* for
humans and the report; classification of axioms is computed from the declarations
present (section 1.4), never from the label.

Every converter is a **separate untrusted crate**. A feature a converter does
not support makes it *refuse the input* with a clear reason (never emit a
partial environment), and the refusal list is part of the converter's README and
tests.

### 6.1 Lean 4 (`lean4`)

| Aspect | Mapping |
|---|---|
| Logic | **Identity**: PIF is Lean's kernel language |
| Input | The pinned lean4export output (VER11a §4.4); the converter is the *only* component that reads it |
| Dropped | Exported recursors (re-derived by the kernel and compared in the differential), `Quot` constants (replaced by `quot`), `unsafe` and `partial`-implementation declarations (skipped and listed) |
| Primitives | `prim` records are emitted for `Nat`, its operations, `String`, `Char`, `List`, `Eq` when the file uses literals, and the `lits`, `quotient` and, when needed, `mutual`/`nested` features are declared |
| Named axioms | `propext`, `Classical.choice`, `Quot.sound` keep their names **and are logic base only when bound by the `axiom.*` roles with their type validated** (1.4, A1); `Lean.ofReduceBool` and `Lean.trustCompiler` are oracle axioms (VER11a §4.5); `sorryAx` is forbidden for claims |
| Unsupported at first | None by design (it is the identity); the first milestone is `Init` (K-M0) |
| Why it is optional now | Claims no longer need Lean in the build path (VER11a Q2's tool exception shrinks to library import and differential testing) |

### 6.2 Rocq (`rocq`)

| Aspect | Mapping |
|---|---|
| `Prop` | Lean's `Prop` |
| `Set` | `Type 0`, but Rocq's `Set` is *predicative* by default (impredicative Set is unsupported, below), so it is an ordinary universe |
| `Type@{i}` | `Sort (i+1)` in Lean's numbering (`Type` is `Sort 1`) |
| `SProp` | mapped to `Prop`: Lean's `Prop` already has definitional proof irrelevance, which is the property `SProp` is for. **This is a conjecture, not a proven translation**: whether everything that checks in Rocq with `SProp` checks in Lean with `Prop` is the Rocq feasibility spike's question (PIF-011a), and the converter refuses `SProp` constructs it cannot map |
| Cumulativity | Rocq has `Prop ≤ Set ≤ Type_i` and (with cumulative inductives) cumulative parameters; Lean has no subtyping between universes. The converter inserts **explicit lifts** (`PLift`, `ULift`, which are ordinary inductives) at every use of cumulativity. This is the main cost of the converter and the main source of its refusals |
| Universe constraints | The converter solves Rocq's universe constraint graph and writes the resulting level expressions (`max`, `succ`, `param`); the kernel re-checks them, so a wrong solution is rejected |
| Inductives | Mapped to `inductive` blocks when positivity and universe rules match; Rocq's template polymorphism and its special elimination rule for `Prop` inductives are re-expressed or refused |
| `Fixpoint` | Compiled to the derived recursor by the converter when the recursion is structural on one argument of an inductive type at top level. This is untrusted translation work (it is a real compiler) |
| Records | Rocq primitive records map to structures; η for records matches Lean's structure η |
| Modules and functors | Flattened by the converter |
| Named axioms | `Src.rocq.<qualified name>` for every `Axiom`/`Parameter` the development depends on; the classical ones (`Classical_Prop.classic`, `functional_extensionality`) may instead be *derived* (section 1.4). **Consistency with Lean's kernel theory is not established** for such axioms: univalence contradicts Lean's definitional proof irrelevance, and Church's thesis and continuity principles contradict `Classical.choice`; the converter refuses them rather than name them |
| Unsupported at first | **Impredicative Set** (`-impredicative-set`); **primitive integers, floats, arrays and strings** (`PrimInt63`, `PrimFloat`, `PArray`, `PString`), which need a native-arithmetic extension of the kernel's logic version; **coinduction and `CoFixpoint`**; **non-structural or nested guarded fixpoints**; **`Unset Universe Checking`** and other checks turned off; **Rocq's `Prop` elimination into `Set` where Lean's subsingleton rule forbids it**; type-in-type |
| What a converted theorem means | A theorem *in Lean's logic* about a translation. Faithfulness to the Rocq statement is the converter's tested property, not a kernel property (section 1.3) |

### 6.3 The HOL family (`hol`)

Covers HOL Light, HOL4, and Isabelle/HOL (through its proof terms).

| Aspect | Mapping |
|---|---|
| Types | Simple types with type variables: a HOL type variable is a Lean `Type u` variable **together with a non-emptiness hypothesis** (see *Relativisation* below); function types are `→`; `bool` is `Prop`; `ind` is `Nat` (Lean has it; HOL's axiom of infinity becomes a theorem) |
| **Relativisation** | HOL types are always non-empty and Lean's may be empty. The translation therefore takes a `Nonempty α` argument for **every type variable, every constant whose type mentions one, and every theorem** quantified over a type variable; a closed instance supplies the witness. Without it a principle like `select : ∀ α, (α → Prop) → α` is inconsistent at `α := False` |
| Terms | Simply-typed λ-terms; equality at `bool` is `Iff`/`Eq` on `Prop`; the HOL primitive `=` is `Eq` |
| Classical logic | Excluded middle is derived from `Classical.choice`, `propext` and function extensionality (a theorem in Lean from `Quot.sound`) |
| Hilbert's ε | `Classical.choice` applied to the `Nonempty α` argument of the relativisation above. HOL's `SELECT_AX` does **not** supply non-emptiness; the converter must |
| Extensionality | Function extensionality (derived from `Quot.sound`) and `propext` |
| Definitions | HOL's constant definitions become `def`; its **type definitions** (`new_type_definition` from a non-empty predicate) become a `Subtype`-style `def` plus the abstraction and representation functions as `def`s, with the bijection laws as theorems — not a new axiom |
| Named axioms | **None beyond the three Lean axioms** (all of HOL's principles are derivable under relativisation). A converter that assumes one instead must name it `Src.hol.<name>`, and it is subject to rules A2 and A3 (1.4): in particular the `select` axiom above is **not** registrable without relativisation |
| Proof terms | HOL Light, HOL4 and Isabelle all record inference-level proofs (OpenTheory articles, Isabelle's proof terms); the converter replays them as Lean terms using lemmas for each primitive rule |
| Unsupported at first | **Isabelle type classes, locales and sort constraints** (dictionary passing is a separate translation); **ad hoc overloading**; **Isabelle's `Pure` framework layer** except where it reduces to `bool`; **HOL4's theory-level definitions by recursion schemas** unless unfolded to primitive definitions |

### 6.4 Native producers (`native`)

The repository's own producers emit PIF directly: no source logic, so
**no extra axioms**: the profile is Lean's type theory itself.

| Producer | Emits | Notes |
|---|---|---|
| **Claim elaborator** (VER11a §7.1, KRN-011) | The *goal*, as a PIF statement (a type), and its `goal` binding | Untrusted; its output is signed (VER11a §7.4) |
| **Case-split decision procedure** (PIF-006) | A proof by exhaustive case analysis over finite inductives (`Bool`, `Option`, finite enumerations), `Bool.rec` and `Option.rec` trees closed by `Eq.refl` | The gates' prover (section 9). Also bounded-depth splits on `Nat`/`Int` with a symbolic remainder and `False.rec` on contradictory hypotheses (9.2). Declares no axiom and no constant occurring in a goal. Fails closed: if a leaf is not closed by `Eq.refl` the kernel rejects |
| **Warrant model elaborator** (PIF-017, first slice of VER33) | The reviewed **model file**: types, reference functions and goals elaborated from a Warrant document (9.2) | Untrusted; its output is reviewed by a person and signed by digest (`gates.model.faithful`) |
| **SAT/LRAT reconstruction** | LRAT steps as proof terms (VER11a §9.5, KRN-009) | Later |
| **Arithmetic certificates** | Linear-arithmetic and bound proofs over `Nat` (VER11a KRN-012a) | Later |
| **Prelude generator** (PIF-004) | The small checked prelude: `Bool`, `Nat`, `Int`, `Eq`, `And`, `Or`, `Not`, `False`, `True`, `Exists`, `Option`, `List`, `Prod`, `Result`, with the same names, types **and values** as Lean's `Init` where it has them, so the two are interchangeable | Declares no axiom (the logic-axiom names are reserved, A1); the gate allow-lists its digest and it is signed under `pif.prelude.faithful` |

---

## 7. Hostile-input limits

PIF is parsed from untrusted bytes. Limits are **explicit, enforced before
allocation, and fail closed** with a named error; none can be exceeded by
asking nicely. They match the kernel's `Limits` (call depth 200, term depth
1,000, steps 2,000,000, new nodes 4,000,000 in `lean-kernel::checker`) and are
configuration with **defaults** and hard ceilings. K-M0 (Lean's `Init`) needs
larger values and raises them in a named, reviewed configuration, never by
disabling a check.

| Limit | Default | Notes |
|---|---|---|
| File size | 64 MiB | Checked against the declared length and while reading |
| Line length | 1 MiB | Read bounded; a longer line is rejected *before* it is buffered |
| Records per file | 4,000,000 | Same as the kernel's new-node cap; also bounds the index spaces. Every index is also a 32-bit number |
| Name components | 32 | `nstr`/`nnum` chain depth, computed at insert |
| String component or literal | 64 KiB | |
| `nat` literal digits | 10,000 | Parsed by `bignum-core` (quadratic in the digits, so the cap bounds the cost to a few hundred million digit operations per file at the record cap), plus the kernel's `Nat.pow` exponent cap for acceleration |
| Level nesting depth | 256 | Computed at insert, not by recursing |
| Expression depth | 1,000 | Equals the kernel's `max_term_depth`; the reader stores each node's depth at insert (1 + the max of its children) and rejects, so it never recurses |
| Universe parameters per declaration | 64 | |
| Levels per `econst` | 64 | |
| Declarations per file | 1,000,000 | |
| Inductive block | 64 types, 4,096 constructors per type, 256 parameters | The 256 is below the kernel's `MAX_BVAR_INDEX` |
| Imports | 64 per file (the chain of 3.7 is linear, so cycles and diamonds cannot occur) | |
| Nesting of anything else | none exists | The grammar has no recursion; depth comes only from backward references |

Properties the reader must have, each a corpus test (section 10):

1. **Fail closed.** Any error stops the read; no partially read file is ever
   offered as an environment. The result is *all declarations or an error*.
2. **No recursion on input depth.** Depth is a stored number, so a hostile file
   cannot exhaust the stack.
3. **No unbounded allocation from a single field.** Allocation is proportional to
   bytes already read and bounded by the limits above.
4. **No arithmetic on unchecked numbers.** Indices are parsed as 32-bit values
   with overflow an error; `end k` is compared, never used as a size.
5. **Forward references and non-dense indices are errors**, never repaired. (The
   reader does not detect duplicate *content*, 4.5.)
6. **Resource use of the kernel is the kernel's** (`Limits`); the reader
   forwards them unchanged.
7. **A small DAG can denote an enormous tree.** A depth-1,000 chain
   `eapp i (i-1) (i-1)` denotes a tree of 2^1000 nodes while having only 1,000
   records. So the depth limit is *not* a size limit. **Every kernel traversal is
   memoised per node or charged against `Limits`** (`max_steps`, `max_new_nodes`),
   the CTE computation of 3.6 is bounded to 1 MiB of output, and a file of this
   shape is in the hostile corpus (section 10).

Error kinds are a closed enum so that the corpus can assert the *reason* for
each rejection; the full list is the right-hand column of 5.3, plus the kernel's
own `KernelError`s.

---

## 8. Versioning and hashing

### 8.1 Three versions, three meanings

| Header line | Name | What it versions | Who reads it |
|---|---|---|---|
| `pif N` | **Format version** | Syntax: the tokens and grammar, which are also the lexical rules | The reader. A value it does not know is a hard error |
| `logic N` | **Kernel-logic version** | Semantics: the set of declarations the kernel accepts and the meaning of each record (including the closed set of `prim` roles and `features`) | The kernel. A value it does not know, or a *withdrawn* one, is a hard error |
| `profile P "…"` | **Producer profile** | Which converter and version produced the file | The report only; never trusted |

The first release is `pif 1`, `logic 1`. `logic 1` is Lean 4's kernel theory as
fixed by VER11a §3.3 at the Lean pin of KRN-003.

### 8.2 Compatibility rules

| # | Rule |
|---|---|
| V1 | The reader supports an explicit **list** of format versions. It never guesses a newer one |
| V2 | A format change that is only additive in meaning (a new record kind, a new feature) increments `pif` and, if the new records need new kernel behaviour, `logic`. A file that uses none of the new things keeps its old header and digest, and the digest is never rewritten |
| V3 | `logic` changes **only when the specified logic changes** (a new feature, role or rule, or a specification fix that makes the kernel reject something the *specification* never allowed). A **specification** fix withdraws the older `logic` value: files declaring it are rejected with `WithdrawnLogicVersion` and must be regenerated, so an old checked proof of a statement a too-liberal rule accepted loses its standing. An **implementation bug** in the kernel or reader does *not* change `logic` (the producer writes that number and could simply not bump it); it withdraws evidence **by kernel crate version and commit**, which every certificate records (8.3) |
| V4 | A new `prim` role or feature is added in a new `logic` version, so a kernel that knows less rejects the file instead of ignoring the role |
| V5 | Converters and `pif-fmt` target one `pif` version each; an `pif-upgrade` tool (untrusted) may rewrite a file to a newer syntax, and the result is a **new file with a new digest** whose header records the upgrade |
| V6 | The pin file (VER11a §4.4) records the `pif` and `logic` versions the kernel accepts, the Lean toolchain tag and lean4export version used by the Lean converter, and nothing else. Moving any of them is its own PR with the corpora of section 10 rerun |

### 8.3 Hashes

| Digest | Definition |
|---|---|
| `file_sha256` | SHA-256 of the file's exact bytes. It identifies the bytes, not the meaning (4.5) |
| `environment_sha256` | Folded over the gate's **flattened, de-duplicated load order** (3.7): `H₀ = SHA-256("pif-env-1\n")`; `Hᵢ = SHA-256(Hᵢ₋₁ ‖ file_sha256ᵢ)`. It is part of what the signature binds (3.6, G6) and is recorded in every report |
| Goal digest | SHA-256 of the printed goal text, in the `goal` record (3.6). The `goals_sha256` of the VER31 sign-off aggregates them and is VER31's to define |
| Certificate id | `pif:sha256:<file digest>` replaces `lean-export:` as the VER11a §11.3 certificate kind (an amendment, Q5). The certificate also carries `environment_sha256`, the **kernel crate version and commit**, and the reader's |

SHA-256 is the repository's existing digest; the gate computes it itself over the
bytes it feeds the reader, never taking a producer's word for it.

---

## 9. First adopter: logic gates

### 9.1 Decision: gates before ct-compare

VER11a §14 has ct-compare as the first adopter (K-M1). The owner decided on
2026-11-24 that the **eleven basic logic gates** go first, with ct-compare next,
reusing the pipeline. They are the same eleven as the cross-lane program:
`not`, `and`, `or`, `xor`, `nand`, `nor`, `xnor` and the NAND-built `nand_not`,
`nand_and`, `nand_or`, `nand_xor` (LG02, `VER-BACKLOG.d/logic-gates-everywhere.md`).

**Why gates are easier, at the levels where they are.**

| Level | Gates | ct-compare |
|---|---|---|
| **Model** | The specification is a finite case analysis over `Bool`: every statement is decided by splitting on at most two bits and closing each leaf by computation. No loops, no induction, no arithmetic | Needs lengths, loops and a relational (two-run) property |
| **Input contract** | A gate accepts only 0 and 1 and rejects everything else (LG01, "Bit representation"). A kernel proof of "rejects every non-bit" quantifies over **every integer**, not the 254 values of `u8` per argument that exhaustive tests walk. `Int` has two constructors, `Nat` two, and one `Nat.rec` step suffices | Input contracts are over slices and lengths |
| **Code** | **Not uniformly easier.** The **NAND-built** gates are bare compositions of `nand_gate`, with no validation of their own (the validation happens inside `nand_gate`). The **seven primitives** are `validate_bit; validate_bit; CMOSx::new(None).evaluate_digital(a, b).unwrap()`: a call into the floating-point `transistors` model with a `Result`, an `Option` and a constructor. So the claim "straight-line, no memory, no two-run property" is true for `validate_bit` and the four NAND-built gates and **false for the seven primitives as shipped** (9.3). The first draft over-stated this | Loops, memory, secrets and a two-run relation (VER11a §8) |
| **Kernel features needed** | Only the earliest ones: non-recursive and simple recursive inductives, recursors, ι, proof irrelevance, function η (9.2) | The full VER11a §14.2 stack |
| **Review** | Each theorem can be read in full by a person | Requires trusting a VC generator's output |

**An honest note on what the kernel adds for the Rust lane.** The Rust gates take
`u8`: 65,536 input pairs, which LG02's corpus already walks exhaustively. The
kernel proof's *gain* is (i) the same statement for **every integer**, which is
the statement the lanes with wider integer types (Python `int`, Ruby, Elixir,
Haskell `Int`, TypeScript's `number` after its integrality check) are instances of,
(ii) an independent check of the exhaustive walk's conclusion, and (iii) the first
complete exercise of the whole pipeline. It does not make the Rust gates safer than
their exhaustive tests make them. The real step up in assurance is G-M2 (partly)
and G-M3 (the every-width adder).

### 9.2 G-M1: the model level

**The model comes from Warrant (owner direction, 2026-11-24).** The gate model
definitions are **not** written by the proof producer. They are elaborated from the
logic-gates Warrant file (`code/specs/logic-gates-basic.warrant`, the LG02
corpus-only slice of `logic-gates.warrant`) into a **separate, reviewed model PIF
file**, signed by its digest. This settles Fable finding 3(a): a producer that
emitted the model together with the theorems could define `nand_and := and`, or a
`toBit` that accepts 2, and prove its theorem by `rfl` under an identical printed goal.

What the Warrant file contributes, and what the model file is:

| Warrant source | Elaborated into the model file as |
|---|---|
| `error GateError = INVALID_INPUT` | `GateError` (an inductive with one constructor) and `Result α ε` (`ok`, `err`) |
| `model b2u(x: Bool) -> U8` | `b2u : Bool → Int` |
| The oracle expressions in the 22 claims (`not a`, `a and b`, `a != b`, `a == b`, `not (a and b)`…) | The reference boolean functions, over the prelude's `Bool` |
| The 11 `op` signatures and the claims' two forms (`<op>_truth_table`, `<op>_rejects_non_bit`) | A **specification function** per gate: `ref_g a b = if a > 1 or b > 1 then err INVALID_INPUT else ok (b2u (oracle …))`, assembled mechanically from the claim's own clauses, plus **the 22 claims as the goals** of the model-level theorems, generalised from `U8` to `Int` (Q4) |
| The NAND-built *constructions* (`nand_not a = nand a a`, …, as LG01 states and the Rust source implements) | **Not expressible in Warrant today** (a Warrant `op` is a signature for code, not a definition). They are a short **hand-written, reviewed structural section** of the same model file, until VER33 lets Warrant define them (9.6) |

The model file is elaborated by the **Warrant model elaborator** (PIF-017,
untrusted, the first slice of VER33), reviewed by a person, and its digest is what
`gates.model.faithful` (bucket (b)) is signed against. The prelude is a separate
file (3.7, `pif.prelude.faithful`). The claim file, produced by PIF-006, contains
only theorems and their goal bindings; **it may declare no constant that occurs in a
goal** (VER11a §7.3 step 3) and **no axiom** (A2).

**Domain.** The model's input type is `Int` (Lean's integer, an inductive with
constructors `ofNat : Nat → Int` and `negSucc : Nat → Int`), the widest domain the
fixed-width lanes' integers embed in. The Rust `u8` is a corollary by restriction.
What `Int` does **not** cover: the **dynamic lanes** (TypeScript `number` doubles,
which reject `-0` and non-integers; Python, which rejects `bool`, `int` subclasses
and floats; Elixir, Ruby, Lua and Perl, with their own wrong-type inputs) keep
their non-integer inputs covered **only by the LG02 type-confusion matrix**, and the
model **does not distinguish error identities** (LG01: the single case means "this
call panicked"). This is owner question Q4.

**The model's API** (LG01), with `Result` for what the Rust gate does by panicking:

```
toBit : Int → Option Bool      -- ofNat 0 ↦ some false, ofNat 1 ↦ some true, else none
ref_g : Int → Int → Result Int GateError   -- ok (b2u r) on two bits, err otherwise
```

A binary gate is `bind (toBit a) (fun x => bind (toBit b) (fun y => …))`. The
NAND-built gates assemble `f` from `nand` exactly as the Rust source does
(`nand_not a = nand a a`, `nand_and a b = nand_not (nand a b)`, `nand_or a b =
nand (nand_not a) (nand_not b)`, `nand_xor` the four-NAND form).

**Theorems** (each is a PIF `theorem` with a `goal` binding; no `sorry`, no oracle,
no axioms beyond PIF's):

| Group | Statement | Content |
|---|---|---|
| **T1 domain** | `toBit` is exactly the set {0, 1}: `toBit (ofNat 0) = some false`, `toBit (ofNat 1) = some true`, `∀ n, toBit (ofNat (n+2)) = none`, `∀ n, toBit (negSucc n) = none`; and **every `Int` is one of those shapes**, stated as an *elimination principle* (`∀ motive, motive 0 → motive 1 → (∀ n, motive (ofNat (n+2))) → (∀ n, motive (negSucc n)) → ∀ a, motive a`), so that no `Exists` is needed | Mostly restates `toBit`'s own definition by `Eq.refl`; **thin by design**, and said to be |
| **T2 truth tables** | for each of the seven primitives, every row, with the rows **checked by the gate against the LG02 corpus** (an independent oracle), not derived from the model's boolean operators (the OCaml lesson in the logic-gates backlog) | For the primitives these hold by construction of `ref_g`; they carry content only as the uniform, signed statement of the table. 26 rows (2 + 6 × 4) |
| **T3 equivalence** | for each NAND-built gate `g'` and its primitive `g`: `∀ a b : Int, g' a b = g a b`, over **every** integer including both error cases | **The group with real content**: the NAND constructions are not defined from the specification |
| **T4 rejection** | for each gate and argument position: `∀ a b : Int, a ∉ {0,1} → g a b = err`, and for `not` | Content for the NAND-built; by construction for the primitives |
| **T5 non-vacuity** | per gate: an accepted witness pair and a rejected one (VER11a §7.5) | `Eq.refl` |

**Producer.** The case-split decision procedure (PIF-006), untrusted: it emits PIF
for T1 to T5 from the model file's definitions. Its **scope is wider than "splits
over finite inductives"**: it needs bounded-depth splits on `Nat` and `Int` with
a symbolic remainder (the `n + 2` case), `False.rec` on contradictory hypotheses
(`a ∉ {0,1}` with `a = 0`), `Option.rec` and `Bool.rec` trees, and the elimination
principle of T1. Our kernel checks the file. **Lean's kernel is an optional
differential oracle** (PIF-010 replay harness), its verdict recorded as evidence
and never a dependency.

**VER31 status of a model theorem.** VER31's `checked` rung is computed for a
**package claim** and needs rung 1 (`tested`) as a floor; a model theorem has no
rung 1. So G-M1 needs either a **VER31 amendment** adding a separate *model
theorem* category (recommended, Q8) or it is reported as evidence attached to the
gates' claims only once G-M2 links the model to the code. Until then, the Rust
gates' claims stay at their present rung and G-M1 produces "model-checked, relative
to the model-to-code ids".

**What G-M1 needs from the kernel** (a precise acceptance test for **KRN-002
slice 3a**, defined in 11):

| Feature | Used for |
|---|---|
| Non-recursive inductives | `Bool`, `Option`, `Prod`, `True`, `False`, `And`, `Or`, `Result`, `GateError`, `Int` |
| Simple recursive inductive, one type, no indices | `Nat` |
| Inductive with an index | `Eq` |
| Recursors derived by the kernel, ι-reduction | `Bool.rec`, `Nat.rec`, `Int.rec`, `Option.rec`, `Eq.rec`, `False.rec` |
| β, δ (hints), ζ | unfolding the gates |
| Proof irrelevance | proofs of `And`, `Or` as arguments |
| Function η | motives and `Nat.rec` on eta-expanded cases |
| Large elimination for `Prop` inductives with one constructor (`And`, `Eq`) | eliminating `Eq` into data in the rejection proofs |
| **Not needed** | literals (`lits`), quotients, mutual and nested inductives, structure η, projections, `Nat` acceleration |

**Assumption ids** (VER31 buckets; ids marked *new* are registered by PIF-016a with
the owner's sign-off, as VER11a §11.2 prescribes):

| Id | Bucket | At G-M1 |
|---|---|---|
| `kernel.checker.correct` | (c) | Relied on; closed per claim by the independent-agreement route (VER11a Q6) |
| `kernel.logic.consistent` | **(c), open** until the owner decides (proposed (a), VER11a Q5) | Relied on; shown **open** in 9.5 |
| `pif.reader.correct` *(new, includes SHA-256 and the import resolver)* | (c) | Opens. **Not closed by a test campaign.** It folds into the independent-agreement route: the second checker (KRN-019) and the Lean replay harness (PIF-010) each **read the same PIF bytes with their own reader**; the comparison covers accepted declarations, theorem types and footprints |
| `tcb.pif.gate-binding` *(new)* | (c) | Opens: the gate-side theorem-kind check, footprint classification, allow-list and goal-term comparison; together with the always-present `tcb.report.checker-correct` |
| `tcb.goal.printer` *(new)* | (c) unless the gate checks `parse(print(G)) = G` | Opens |
| `pif.prelude.faithful` *(new)* | **(b)**, signed against the prelude file's digest; or a gate condition: declaration-for-declaration identity with Lean `Init`, **values included** | Opens |
| `spec.goal.faithful` | (b) | Opens: the goals (CTE digest plus readable text) are signed |
| `gates.model.faithful` *(new)* | (b), signed **against the model file's digest** | Opens: `toBit`, `ref_g`, the NAND constructions and "reject by panic modelled as `err`" say what LG01 says |
| `gates.model.code-link` *(new)* | (c) | Opens: nothing connects the model to the Rust code. Closed by G-M2, which replaces it by the WIR ids |
| Closes | none | G-M1 closes no existing (c) id; it adds a model-checked theorem and a reviewed pipeline |

### 9.3 G-M2: the code level

**Target.** The Rust lane's gates lowered to WIR by the existing front end
(VER25a, VER25b), with the semantics of **straight-line WIR**: the loop-free,
allocation-free fragment of VER11a §6 (`run`, the frame stack, `ip`, the cost
schedule of VER25 §8). The semantics is written as **native PIF by a generator**
(PIF-019), so G-M2 does **not** depend on Lean source, K-M0, KRN-005 or KRN-007.

**Theorems.** The G-M1 theorems restated over the WIR programs: for all argument
values, `run n gatesWir (entry …)` ends in `ret v` as the model says, or in the
**panic outcome** the semantics gives `assert!` (a trap with the gate's identity,
no effect) for every non-bit; and `no_panic` on every input in {0,1}. The **link
theorem** (`run … = ret (fromResult (ref_g a b))`) is the statement that the code
is the model, and closes `gates.model.code-link` into the existing WIR ids.

**What the code really is, and the scope question (Q2).**

| Gate group | Shipped body | Lowerable today? |
|---|---|---|
| `validate_bit` | one comparison, `assert!` | Yes |
| The seven primitives | `validate_bit; validate_bit; CMOSx::new(None).evaluate_digital(a, b).unwrap()`: a floating-point voltage model, a `Result`, an `Option`, a constructor | **No** (floating point, `Result`/`Option` in the subset not yet lowered) |
| The four NAND-built | bare compositions of `nand_gate`, **no validation of their own** | Yes, **as calls** into `nand_gate` |

| Option | What G-M2 covers | Cost |
|---|---|---|
| **A. Call contract** | The four NAND-built gates, **conditional on** `gates.nand.primitive-contract` (a contract for `nand_gate`: accepts exactly {0,1}², returns the bit-level NAND, otherwise panics; it includes the rejection and the `Ok`/no-panic part). The seven primitives are **not covered** | A never closes by itself. The contract's accepted path is a four-row finite statement, **already covered by exhaustive execution**, and is recorded as that evidence kind |
| B. Pure-bit bodies | Change the gates to compute the bit function after validation; keep the transistor model as a differential check | Changes a package about 47 crates depend on (LG01) and changes what the library *is* |
| C. Lower the transistor model | The full chain to the CMOS evaluation | Needs floating point in the WIR semantics; far larger |
| **D. Integer switch-level path (recommended)** | Add to `transistors` an **integer switch-level `evaluate_digital` path**: each transistor conducts or does not, no `f64`. The gates stay transistor-routed; the path is lowerable; the voltage model stays as the **differential check**. The existing digital path already uses only comparisons on `f64` (no `exp` or `powf`), so the four-row contract is cheap to discharge exhaustively in the meantime | A change in `transistors` and its tests, but no change to the gate API; all eleven gates become coverable |

**Recommendation: D**, with **A as an acceptable relabelled first slice**. The G-M2
row in 9.5 reads "**4 of 11 gates, conditional on `gates.nand.primitive-contract`;
7 primitives not covered**" until D lands, not "gate claims over WIR `checked`".

**Kernel features G-M2 adds to G-M1:** the WIR semantics definitions (structural
step-count fuel, VER11a §6.2), `Nat`/`UInt8` facts for `validate_bit`'s
comparisons. **No literals if the semantics is written over unary-coded
constants.** If numeric constants are literal-based, G-M2 declares `lits` (the gate
must allow it, 3.5) and the `prim` validations become part of the trusted
footprint; that is recorded as the reason to import Lean's `Init` through the Lean
converter at that point.

**Assumption ids.** Opens: `wir.lowering.faithful`, `wir.semantics.lean-faithful`,
`wir.typing.checker-guarantees`, `wir.interp.differential-only` (existing (c) ids),
`gates.nand.primitive-contract` *(new, (c), options A and D until discharged)*.
Closes: `gates.model.code-link` (replaced by the four above). The **count of open (c)
ids does not fall** at G-M2. New ids are registered with this milestone (PIF-016b).

### 9.4 G-M3: compositions, and the first "for every size" theorem

**Targets.** In order, each model level first:

1. **Half adder**: `sum = xor`, `carry = and`. Correct on every bit pair, and rejecting
   non-bits (the gate rejection composes).
2. **Full adder**: from two half adders and an OR. For bits `a b cin`,
   `2 * carry + sum = a + b + cin`.
3. **n-bit ripple-carry adder**, correct for **every width n**, by induction: for every
   `n`, every two `n`-bit vectors `xs ys` (LSB first, the convention of `arithmetic`'s
   `ripple_carry_adder` and of LG01) and every carry-in `c`,
   `value (sum xs ys c) + 2^n * (carry xs ys c) = value xs + value ys + c`.

**The model must say what the real adder does on bad input.** The Rust
`ripple_carry_adder` **asserts equal lengths and non-empty input**, allocates a `Vec`,
indexes slices and sets an overflow flag. So the model states: unequal lengths and
empty vectors are rejected, a non-bit element is rejected, and otherwise the equation
above holds. The **model-level** theorem is for every `n ≥ 1` of equal-length
vectors; the **code-level** statement (G-M3b) is likewise "for every `n ≥ 1`" and
**needs allocation and memory semantics** (VER11a §6.3), not only the straight-line
fragment.

This is the first theorem in the repository that no test and no bounded proof can
reach. It is proved by `Nat.rec` or `List.rec` induction.

| Feature | Notes |
|---|---|
| `List` (a parametric recursive inductive), `Prod` | The bit vector and `(sum, carry)`. **`Prod` is used through `Prod.rec`, not projections**, so slice 3a suffices; projections and structure η are slice 3b |
| `Eq.rec` / `congrArg` / `Eq.symm`, K-like reduction for `Eq` | Rewriting in the induction step |
| `Nat` addition and multiplication lemmas | **Option 1 (recommended):** a small native lemma set (`Pif.Nat`: commutativity, associativity, distribution, defined by structural recursion with no literals); **Option 2:** import Lean's `Init`, which needs `lits` and K-M0 |
| A **producer for induction proofs** | PIF-006 is case-split only. G-M3a needs a native induction and rewriting term builder (PIF-018), or admits that the producer is Lean's toolchain through PIF-008 |
| Literals | **Not** needed under option 1 |

**Assumption ids.** G-M3a opens `spec.goal.faithful` for the adder specification (the
meaning of `value` and of "every width" is signed, (b)), `gates.compose.faithful`
*(new, (b))*, **and `gates.compose.code-link` *(new, (c))*, which is open from G-M3a**
(nothing yet connects the model adder to the Rust adder). It closes nothing in (c).
G-M3b closes `gates.compose.code-link` once the adder's WIR program (with allocation
and memory) is covered. New ids are registered with the milestone that opens them
(PIF-016c).

### 9.5 The assumption story, end to end

| After | Gates claims | Open (c) | Open (b) |
|---|---|---|---|
| Today (Rust lane) | `tested` / `exhaustive` (domain not emitted yet, VER31 §4) | the lane's `wir.*`/`tcb.*` ids | the corpus statement |
| G-M1 | **model-checked** theorems (VER31 amendment, Q8); package claims unchanged | `kernel.checker.correct`, **`kernel.logic.consistent` (until the owner decides)**, `pif.reader.correct`, `tcb.pif.gate-binding`, `tcb.goal.printer`, `tcb.report.checker-correct`, `gates.model.code-link` | `spec.goal.faithful`, `gates.model.faithful`, `pif.prelude.faithful` |
| G-M2 (option A) | **4 of 11 gates over WIR, conditional on `gates.nand.primitive-contract`; 7 primitives not covered** | as above, with `gates.model.code-link` replaced by the `wir.*` ids and `gates.nand.primitive-contract` | same |
| G-M2 (option D) | all 11 gates over WIR | as above without the contract once discharged | same |
| K-M2 (ct-compare, shared) | `proved` possible | `kernel.checker.correct` per claim (independent agreement) | signed |

### 9.6 Forward pointer: VER33, Warrant as the algorithm definition language

The owner wants Warrant to grow into the repository's **algorithm definition
language** (recorded 2026-11-24): one definition, elaborated into kernel
definitions, proved, compiled to every lane, with each lane's code lifted back and
checked for equivalence with the definition. That is its own specification, **VER33**,
written after VER32, and this document does not pre-empt it. What VER32 does today is
the first slice: the Warrant model elaborator (PIF-017) turns the logic-gates Warrant
file's model functions and claims into a reviewed model PIF file (9.2), and the gap
it exposes (a Warrant `op` is a signature, not a definition, so the NAND constructions
are hand-written) is one of VER33's first requirements.

## 10. The corpus, the oracles, and how the reader is tested

One corpus file exists for **each rule of 5.3** and for each limit of section 7.

| Corpus | Contents | Used by |
|---|---|---|
| **Positive** | The worked example of 4.6; the G-M1 files (prelude, model, claim); the native prelude; small Lean-converted libraries | reader, kernel, `pif-fmt`, differential |
| **Negative, lexical** (R-LEX) | CRLF, tab, double space, trailing space, blank line, comment, `nstr 1 0"Bool"`, `[ 1 2 ]`, a raw space inside a string, bytes after `end`, a non-ASCII byte, leading zero, `\u{0041}`, `\u{0}`, a surrogate, a value above 10FFFF, an escaped printable character, uppercase hex, a short digest | reader and grammar parser (both must reject, with the same rule) |
| **Negative, structural** (R-VER, R-IMP, R-IDX, R-NAM, R-CNT, R-FEA, R-PRM, R-DER, R-GOL) | Forward reference, self reference, index gap, wrong counts, `end k` wrong, unknown or disallowed feature, undeclared feature use, import mismatch, a name with `.` or a control character, the anonymous name as a declaration, `prim` before its constant, a rebound role (`DuplicateRole`), `enat` before `prim nat`, `axiom Foo.rec` before `inductive Foo`, a constructor named `Foo.rec`, `Quot.mk` before `quot`, a `goal` on an `axiom`, on an imported theorem, a duplicate claim id | reader |
| **Negative, axioms** (A1 to A3) | `axiom propext : False` in a claim file; the same in the prelude; a registered `Src.*` name declared with a different type or universe parameters; an unregistered `Src.*`; a `Warrant.Assume.<id>` with the wrong statement | reader, gate |
| **Hostile (limits)** | One file per row of section 7: a 10 MB line, a million-deep chain written with backward references only, a 10,001-digit `nat`, a 65-type block, a header claiming a huge count, and **the 2^1000-node DAG** (`eapp i (i-1) (i-1)`), which the kernel must reject or finish within its `Limits` | reader, kernel (reject quickly, bounded memory) |
| **Negative, kernel** | Ill-typed theorems, non-positive inductives, a universe-inconsistent inductive, a bad `prim nat.add`, a `theorem` whose type is not a `Prop`, an axiom `False`, Lean's kernel-rejection tests converted by the Lean converter. **An ill-typed theorem and a `theorem` written like an axiom are routed through the reader** (5.4) | reader, kernel, differential vs the official kernel |
| **Mutation, files** | Every record of the positive files mutated (one field, one record dropped, two swapped); each mutant must be rejected by the reader, the kernel, or the goal comparison | CI |
| **Mutation, reader** | Each rule of 5.3 mutated in the reader's own source (drop a check, off-by-one an index, swap two roles); the corpus must kill every mutant | CI |
| **Differential** | (i) reader vs the grammar-driven parser with its independent semantic layer, comparing **kernel-call sequences and resulting environments**, on every corpus file and on grammar-generated and mutated files; (ii) kernel-derived recursors vs Lean's exported ones; (iii) our verdict vs the official Lean kernel through the PIF→Lean harness, **which reads the same PIF bytes with its own reader**; (iv) later, vs the second checker (KRN-019), the same way. The comparison covers accepted declarations, theorem types and footprints | CI; the route that closes `pif.reader.correct` and `kernel.checker.correct` |

Oracles are **differential evidence only, never a dependency of a check**.

---

## 11. Backlog

The same rows are added to [VER-BACKLOG.md](VER-BACKLOG.md) under "Proof
interchange format (VER32) and first proof target (logic gates)". Order is
dependency order; one bounded change per PR; spec before code. Kernel slices
3a and 3b are defined here because the milestones are acceptance tests for
them.

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / PIF-000 | VER32 spec, adversarial review (round 1 answered), owner answers to VER32 section 12 | revised spec in review; owner answers pending |
| P1 / KRN-002 slice 3a (defined by VER32) | **Single (non-mutual, non-nested) inductive types, including indexed families**: positivity, universe constraints, derivation of constructors' use and recursors, **ι-reduction**, the **large-elimination (subsingleton) rule**, **K-like reduction** for `Eq`-like types, name-collision checks (VER32 3.8). Delivers everything G-M1 and G-M3a need | in progress in parallel; slice 2 merged |
| P1 / KRN-002 slice 3b (defined by VER32) | **Mutual and nested** inductives, structure detection, **structure η** at recursor majors, **projection reduction**, unit-like types, `mutual`/`nested` features. Not needed by G-M1 or G-M3a (`Prod` is used through `Prod.rec`) | after slice 3a |
| P1 / PIF-001 | `code/grammars/pif/pif.tokens` and `pif.grammar` (VER32 4.2, 4.3); tokens carry their spacing, so there is no pre-pass; both parse the worked example and the lexical corpus; ci-full-build wiring | after PIF-000 |
| P1 / PIF-002 | **Trusted reader** in `lean-kernel` (≤ 600 lines, literate, `forbid(unsafe_code)`): exactly the rules of VER32 5.3, limits (7), `prim` single assignment, goal checks; counted in the 8,500. SHA-256, import resolver and `nat` parsing counted separately in the trusted base | after PIF-001 and KRN-002 slice 2; inductive blocks, `quot` and `prim` after slice 3a, slice 3b and the literals slice |
| P1 / PIF-003 | `pif-check` binary (replaces `lean-check`): resolver with SHA-256 verification, the gate-fixed ordered environment, accepted declarations, axiom footprint (a constant's footprint includes itself), goal bindings; `pif-fmt` (lint C1 to C3, untrusted); `pif-pp` | after PIF-002 |
| P1 / PIF-004 | Native prelude generator: `Bool`, `Nat`, `Int`, `Eq`, `And`, `Or`, `Not`, `True`, `False`, `Exists`, `Option`, `List`, `Prod`, `Result`; names, types **and values** equal to Lean's `Init` where it has them (needed for interchangeability, else G-M1 proofs must be regenerated at the `Init` import); logic-axiom names reserved; digest allow-listed; `pif.prelude.faithful` signed | after PIF-002 and KRN-002 slice 3a |
| P1 / PIF-005 | PIF corpus: positive (the VER32 4.6 example first), one file per rule of 5.3, hostile (limits, the 2^1000-node DAG), kernel-negative (including `axiom propext : False`, a registered `Src` name with a different type, a `goal` on an axiom, duplicate claim ids, derived-name collisions, `DuplicateRole`); reader mutation runner; CI job | after PIF-003 |
| P1 / PIF-007 | Differential harness: the reader against the grammar-driven parser **with an independently written semantic layer**, comparing the sequence of kernel calls and the resulting environments, on every corpus file plus grammar-generated and mutated files (evidence for Q1) | after PIF-001, PIF-002 |
| P1 / PIF-017 | **Warrant model elaborator** (untrusted; first slice of VER33): elaborates `logic-gates-basic.warrant`'s `error`, `model b2u`, oracle expressions, `op` signatures and 22 claims into the reviewed **model PIF file** (specification functions, goals), plus the hand-reviewed NAND-construction section; model digest signed under `gates.model.faithful` | after PIF-004 |
| P1 / PIF-006 | **Case-split producer** (untrusted): proofs by exhaustive case analysis over finite inductives **and** bounded-depth splits on `Nat`/`Int` with a symbolic remainder, `False.rec` on contradictory hypotheses, `Option.rec`/`Bool.rec` trees, the T1 elimination principle; may declare no axiom and no constant occurring in a goal; fails closed | after PIF-004, PIF-005, PIF-017 |
| P1 / PIF-020 | VER31 amendment: define the **goal-signing aggregate** `goals_sha256` as the SHA-256 of the sorted lines `claim-id goal-digest environment-digest`; the signature covers the exact gate-fixed environment; the model-theorem category (VER32 Q8); `pif:sha256:` certificate kind with kernel crate version and commit | after PIF-000; before G-M1 can be signed |
| P1 / G-M1 | **Gates, model level**: T1 to T5 for the eleven gates over `Int`, the model from the Warrant-derived model file (PIF-017), the proofs from PIF-006, goal bindings (CTE digests), report fields; ids opened as in VER32 9.2; registered by PIF-016a. Needs KRN-002 slice 3a | after PIF-004, PIF-006, PIF-017, PIF-020, KRN-002 slice 3a |
| P1 / PIF-016a | VER13 slice for G-M1: `pif.reader.correct`, `tcb.pif.gate-binding`, `tcb.goal.printer`, `pif.prelude.faithful`, `gates.model.faithful`, `gates.model.code-link`, with bucket entries (owner signs the bucket file, as for KRN-011) | with G-M1 |
| P1 / PIF-008 | Lean→PIF converter (untrusted crate): reads the pinned lean4export output, drops recursors, emits `prim` (including the `axiom.*` roles), `quot`, features, skips and lists `unsafe`; **replaces KRN-003's lean4export reader**. P1 because PIF-009 depends on it | after PIF-002; pin file |
| P1 / PIF-009 | K-M0 through PIF: Lean's `Init` converted and checked; agrees with the official kernel on every safe declaration; kernel-derived recursors equal the exported ones (replaces KRN-005's input) | after PIF-008, KRN-002 (full) |
| P2 / PIF-010 | PIF→Lean replay harness (oracle only, reads PIF with its **own reader**, calls `addDecl`); recorded agreement as evidence (part of the route that closes `pif.reader.correct`); replaces the harness of KRN-004 | after PIF-003 |
| P1 / PIF-019 | **Straight-line WIR semantics as native PIF** (a generator, not Lean source), so G-M2 does not depend on K-M0, KRN-005 or KRN-007 | after PIF-004, KRN-006 |
| P1 / G-M2 | **Gates, code level**: the Rust gates lowered to WIR; the G-M1 theorems and a link theorem over the straight-line semantics (PIF-019); scope per VER32 Q2 (option A: **4 of 11 gates, conditional on `gates.nand.primitive-contract`; 7 primitives not covered**; option D adds the integer switch-level path and covers all 11). Opens the `wir.*` ids, closes `gates.model.code-link`; registered by PIF-016b | after G-M1, PIF-019, KRN-006a/b/f |
| P1 / PIF-016b | VER13 slice for G-M2: `gates.nand.primitive-contract` and the `wir.*` bucket entries it relies on | with G-M2 |
| P2 / G-M2-D | `transistors`: an **integer switch-level `evaluate_digital` path** (no `f64`), the voltage model kept as a differential check; extend G-M2 to the seven primitives (VER32 Q2 option D) | after G-M2; Q2 |
| P2 / PIF-018 | **Native induction and rewriting term builder** (untrusted producer for G-M3a): motives, `Nat.rec`/`List.rec` induction, `Eq.rec`/`congrArg` rewriting. The alternative is to admit the Lean toolchain through PIF-008 as the producer | after PIF-006 |
| P1 / G-M3a | **Compositions, model level**: half adder, full adder, n-bit ripple-carry adder correct for every `n ≥ 1` by induction, with the model rejecting unequal lengths, empty vectors and non-bits; native `Pif.Nat` lemma set, no literals; opens `gates.compose.faithful` and `gates.compose.code-link`; registered by PIF-016c | after G-M1, PIF-018 |
| P1 / PIF-016c | VER13 slice for G-M3: `gates.compose.faithful`, `gates.compose.code-link` | with G-M3a |
| P2 / G-M3b | Adder at code level (allocation and memory semantics, VER11a §6.3; the loop's block invariant) and a link theorem, for every `n ≥ 1`; closes `gates.compose.code-link` | after G-M2, KRN-012, KRN-013a |
| P2 / PIF-011a | **Rocq feasibility spike** (time-boxed): translate a small development (inductives, structural `Fixpoint`, an `SProp` use and a cumulativity case); decide whether the `SProp` to `Prop` conjecture and the cumulativity lifts hold | after PIF-009 |
| P2 / PIF-011 | Rocq→PIF converter (untrusted; VER32 6.2; refuses impredicative Set, primitive ints/floats/arrays, coinduction, non-structural fixpoints, `Src` axioms of unestablished consistency) | after PIF-011a |
| P2 / PIF-012 | HOL-family converters with relativisation (OpenTheory first, then HOL Light and HOL4; Isabelle/HOL later; VER32 6.3) | after PIF-009 |
| P3 / PIF-013 | Reader option (a): a `grammar-tools`-generated reader behind a flag, compared with the hand-written one; adoption decided on size and audit | after PIF-007; Q1 |
| P3 / PIF-014 | Compact binary form (a mechanical bijection of the text records; digest over the text) | after PIF-009 measures `Init` |
| P2 / PIF-015 | ct-compare on PIF: KRN-013a/013 emit PIF through the claim elaborator and the generic producers; the Lean toolchain optional | after G-M2 |

---

## 12. Owner questions

Each row gives the recommendation and **Fable's view** (round 1) next to it.

| # | Question | Recommendation | Fable's view and the alternatives |
|---|---|---|---|
| Q1 | How is the trusted PIF reader built? (carried over from VER11a Q9) | **(b)** a ≤ 600-line hand-written reader, with the grammar-driven parser (and its own independent semantic layer) as a second implementation, differentially compared on **kernel calls and environments**; a narrow owner exemption to "no hand-written parsers" for this one trusted reader (5.2) | **Agree with (b), on four conditions**: the differential compares kernel calls; the grammar and the lexical rules are the same language; the grammar side's semantic layer is acknowledged as a second hand-written reader; SHA-256 and import handling are counted in the trusted base. All adopted. Alternative: **(a)** generate from the grammar with `grammar-tools` (puts the engine and the semantic layer in the base) |
| Q2 | What does G-M2 target for the seven primitive gates, which delegate to the floating-point `transistors` model, outside the lowered subset? | **D**: add an **integer switch-level `evaluate_digital` path** to `transistors` (each transistor conducts or not, no `f64`); the gates stay transistor-routed, the path is lowerable, the voltage model stays as a differential check. **A is acceptable only as a relabelled first slice**: 4 of 11 gates, conditional on `gates.nand.primitive-contract`; 7 primitives not covered | Agrees with A **as a first slice only**, relabelled; **A never closes on its own**; and **offered D**, which this document adopts as the recommendation. The existing digital path uses only `f64` comparisons (no `exp`/`powf`), so the four-row contract is cheap to discharge exhaustively meanwhile. Alternatives: B (pure-bit bodies, a change to a package about 47 crates use), C (lower the transistor model, needs floating point in the semantics) |
| Q3 | May a *claim's* axiom footprint contain a `Src.<profile>.*` axiom? | **No**, until a VER13 id and bucket are registered **with a consistency argument relative to Lean's logic** (bucket never (a)); a claim file may declare only pinned axioms (A1 to A3) | **Agree (no), strengthened** by the findings that axiom names are not bound to types (A1) and that source axioms need not be consistent with Lean's logic (1.4). Alternative: allow any named axiom, listed and classified (c) |
| Q4 | What is G-M1's input domain? | **`Int`** (every integer; fixed-width lanes are subsets). State the gaps: dynamic lanes' non-integer inputs stay covered by the LG02 type-confusion matrix only, and the model does not distinguish error identities | **Agree with `Int`**, but **drop** "the statement every lane needs", state the gaps (TypeScript `number` doubles reject `-0` and non-integers; Python rejects `bool`, `int` subclasses and floats; Elixir, Ruby, Lua and Perl have wrong-type inputs), and fix two factual errors (Haskell uses `Int`, not `Integer`; Perl is not an unbounded-integer lane). Done in 9.1, 9.2. Alternatives: `Nat`; `Fin 256` |
| Q5 | Rename the certificate kind `lean-export:` to `pif:`, a VER31 amendment | **Yes**; the certificate also carries the kernel crate version and commit and `environment_sha256` (8.3) | **Agree**, with those additions |
| Q6 | Where does the native prelude come from for G-M1 and G-M3a? | **Our own minimal prelude** (PIF-004), with **its own (b) id** `pif.prelude.faithful` (or a gate condition of identity with `Init`), names, types **and values** equal to `Init`'s where it has them, and the logic-axiom names reserved | **Agree, on three conditions**: its own (b) id or a gated identity check; "same names and types" becomes "same values" if the two are to be interchangeable (else G-M1 proofs are regenerated at the `Init` import); the logic-axiom names are reserved. All adopted. Alternative: import `Init` from the start (needs `lits`, K-M0 and Lean in the build path) |
| Q7 | Does a converter's report also list its unsupported constructs and axioms? | **Yes**; the report also lists the **active features** and the **`prim` bindings**, and marks every `Src.*` axiom "consistency with the kernel logic not established" | **Agree**, with those additions |
| Q8 | What status does a **model theorem** (G-M1) have in VER31, which computes `checked` for a package claim and needs rung 1? | A **VER31 amendment** adding a separate *model theorem* category (PIF-020), distinct from the package rungs, relative to the model-to-code ids | Found by the review (finding 14): `checked` cannot be claimed for a model theorem as the rung stands. Alternative: no label until G-M2 links the model to the code |

---

## 13. Relationship to other specifications

| Spec | Relationship |
|---|---|
| VER11a | Logic unchanged (§3.3). §4.3 amended: the kernel's input is PIF. Q9 resolved. §4.2's "Export reader" row now describes the PIF reader. §5.3 and Q2: the Lean toolchain is now one optional producer. §7's printed goal is now bound through the CTE (3.6). §4.5's axiom classification is tightened by 1.4 (A1 to A3) |
| LEAN00 | L1 is "the kernel checks Lean's core library"; it now does so through the Lean→PIF converter. LEAN00's later layers produce PIF as one more producer |
| VER31 | Buckets and rungs as in section 9; the certificate kind, the model-theorem category, `goals_sha256` and the new ids (section 11, PIF-016a/b/c, PIF-020) are VER31 amendments |
| VER13 | Gains the ids of 9.2 to 9.4 |
| LG01, LG02, `logic-gates-basic.warrant` | The model of 9.2 is elaborated from the Warrant file; the gates' truth tables are the LG02 corpus |
| VER25, VER25a/b, VER11a §6 | G-M2 uses the straight-line fragment of the semantics, written as native PIF (PIF-019) |
| VER11a §14 (K-M1) | ct-compare becomes the second adopter on the same pipeline (PIF-015) |
| VER33 (to be written) | Warrant as the algorithm definition language (9.6) |

## 14. What this spec does not promise

- That the reader is *correct*. It is small, tested against an independent
  implementation, and counted as trusted (`pif.reader.correct`, (c)), closed only
  by independent agreement, never by a test campaign.
- That a converted third-party theorem is *the source prover's theorem*, or that a
  source logic's extra axioms are consistent with Lean's. Only that it is a theorem
  of Lean's logic, relative to the listed axioms (1.3, 1.4).
- Support for every Rocq or Isabelle feature. The unsupported lists are part of the
  contract; the `SProp` mapping is a conjecture until the spike.
- That the gates' code-level proof (G-M2) reaches all eleven gates at once: it
  depends on Q2 (option D).
- That the kernel proof makes the Rust gates safer than the exhaustive walk does
  (9.1).
- A compact binary form, an upgrade tool beyond a sketch, or Mathlib.

## 15. Review record

**Fable adversarial review, round 1 (2026-11-24)**, of the first draft
(commit `5e37ec949c3`). Verdict: the core decision holds (fixed logic, own
format, untrusted producers); three holes let a claim be accepted or labelled
beyond its evidence; the gate milestones were overstated. 17 findings.

| # | Finding | Severity | Disposition |
|---|---|---|---|
| 1 | Axioms classified by name; `axiom propext : False` in a claim file passes as logic base; registered `Src.*` names are not bound to types | critical | **Fixed**: rule A1 (type-validated `axiom.*` roles), A2 (claim files declare only pinned axioms matched by type), A3 (1.4); `prim` roles 3.5; check R-AX-1; corpus cases (section 10) |
| 2 | A `goal` record can target a non-theorem; footprint walk over the value only; imported targets, duplicate ids and digest trust unspecified | critical | **Fixed**: G1 to G4 (3.6), R-GOL-1/2, footprint includes itself (PIF-003) |
| 3 | The goal digest binds the printed text, not the environment: producer-written model (a), allow-list a set (b), prelude without an id (c) | critical/major | **Fixed**: (a) the model is a **separate reviewed file elaborated from Warrant** (9.2, D8; this settles it as the coordinator directed); (b) the signature covers `(claim id, goal digest, environment_sha256)` over the gate-fixed ordered list (D9, 3.7, E1 to E6); (c) new (b) id `pif.prelude.faithful` or identity with `Init` including values (9.2, Q6) |
| 4 | The printer is in the trust path but listed as untrusted; printing is not injective | major | **Fixed**: digest over the **CTE** of the goal term (3.6); name-component rules (3.8); printer refuses shadowed binders and never uses store binder names; gate checks `parse(print(G)) = G` or the (c) id `tcb.goal.printer` (1.1). *Both* options taken rather than either |
| 5 | Source principles as named axioms do not preserve consistency (HOL empty types; Rocq univalence vs UIP; `SELECT_AX` misdescribed) | major | **Fixed**: 1.4 states non-established consistency, requires an argument, never bucket (a); `Nonempty` relativisation added (6.3); SProp→Prop labelled a conjecture (6.2, PIF-011a); converters trusted only for meaning and axiom naming (1.1, 1.3) |
| 6 | Reserved and derived names, anonymous name, `prim` rebinding unspecified; claim files can enable `lits`/`quotient` | major | **Fixed**: 3.8 (derived-name rejection, anonymous name, components); 3.5 single-assignment roles, `LiteralBeforeRole`, gate-fixed features (`FeatureNotAllowed`); R-DER-1, R-PRM-2/4, R-VER-2 |
| 7 | Duplicate-record rule contradicts itself and the kernel's interning | major | **Fixed by dropping the check**: the reader does not enforce C1 (4.5, 5.3, section 7); it is a `pif-fmt` lint. Chose Fable's first alternative |
| 8 | Grammar and the "split on single spaces" reader accept different languages; STRING regex too loose; shared pre-pass untested; L1 says UTF-8 | major | **Fixed**: tokens carry their own spacing so the grammar is the whole language and there is no pre-pass (4.1, 4.2); space inside strings is `\u{20}`; brackets byte-exact; STRING regex tightened plus R-LEX-2/3; bytes after `end` an error; file is ASCII; numeric bounds (4.1) |
| 9 | Imports under-specified (resolver, diamonds, order, transitive allow-list, SHA-256 in the trusted base) | major | **Fixed**: E1 to E6 (3.7): a gate-supplied **linear chain** (a refinement of Fable's fix: imports equal all preceding files, so diamonds are impossible by construction), each file loaded once, transitive allow-list, SHA-256 and the resolver added to 1.1 and 5.1 |
| 10 | Canonical-form claims overstated; the example is not C2-canonical; "byte-for-byte unique" false | minor | **Fixed**: 4.5 rewritten: lexical canonical form only, digests identify bytes, C1/C3 are lints, C2 a convention, `end k` for accidental truncation; example described as creation-order output of a throwaway script (4.6) |
| 11 | V3 withdrawal is keyed on a number the producer writes | minor | **Fixed**: V3 (8.2): specification fixes withdraw `logic`; implementation bugs withdraw by **kernel crate version and commit**, which certificates record (8.3) |
| 12 | Resource limits: DAG explosion; quadratic big-number parsing | minor | **Fixed**: section 7 item 7 (memoise or charge `Limits`, CTE bound, corpus file); `nat` digits cut to **10,000** and `bignum-core` named as the parser (5.1). *Deviation in degree*: Fable asked to "say who parses it"; the cap was also lowered |
| 13 | G-M2 option A proves almost nothing but is labelled as gate claims over WIR; the §9.1 "Code" row is false for 7 of 11 | major | **Fixed**: 9.1 corrected; G-M2 row reads "4 of 11 gates, conditional on `gates.nand.primitive-contract`; 7 primitives not covered" (9.3, 9.5); the contract includes rejection and `Ok`/no-panic and is recorded as exhaustive-execution evidence; **option D added and recommended** (Q2, coordinator decision) |
| 14 | G-M1's rung and "every integer" value overstated; `Int` is not every lane's input; Haskell and Perl errors | major | **Fixed**: model-theorem category instead of `checked` (9.2, 9.5, Q8); T1/T4 stated to be thin, only T3 (and T2 against LG02) has content; dynamic-lane gaps and error-identity limit stated; the Haskell (`Int`) and Perl errors corrected (9.1) |
| 15 | Assumption ids missing or loosely closed (prelude, printer, gate binding, SHA/resolver; T2 rows; `kernel.logic.consistent`; `pif.reader.correct`; G-M3 ids) | major | **Fixed**: ids and buckets in 9.2; `kernel.logic.consistent` shown (c)-open in 9.5; `pif.reader.correct` closed only by independent agreement over the same PIF bytes; T2 rows gate-checked against LG02; `gates.compose.code-link` opened at G-M3a; the Rust adder's asserts, allocation and slices, and "for every n ≥ 1" stated (9.4) |
| 16 | Missing backlog items and wrong dependencies | major | **Fixed** (section 11 and `VER-BACKLOG.md`): PIF-018 (induction term builder), PIF-006 scope widened, PIF-004 gains `Not`/`Exists`, PIF-019 (native straight-line semantics), PIF-020 (goal-signing aggregate), KRN-002 slices 3a/3b defined, PIF-008 raised to P1, PIF-016 split per milestone, PIF-011a spike |
| 17 | What the differential compares; what the 600 lines cover | major | **Fixed**: 5.3 is the single normative list of reader checks; 5.4 compares kernel-call sequences and environments, adds reader mutation testing and routes an ill-typed theorem through the reader; 5.1 lists what is outside the 600 lines (SHA-256, resolver, big-number parsing, goal checks) |

**Coordinator decisions applied with the revision (2026-11-24):** every fix as
Fable recommends it; the gate model comes from Warrant (9.2) with a VER33
forward pointer (9.6); option D added to Q2 and recommended; Fable's view
recorded on each owner question (section 12); the reader checks collected into
one list (5.3); the backlog in `VER-BACKLOG.md` kept identical to section 11.
