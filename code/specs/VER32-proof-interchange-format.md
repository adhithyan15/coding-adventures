# VER32 — The Proof Interchange Format, and logic gates as the first proof target

**Status:** Draft for owner approval. Spec only; no code exists. The two
design decisions in section 0 are owner decisions of 2026-11-24; the open
questions are in section 12. Implementation is the `PIF-*` and `G-M*` queue in
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
| D3 | Every producer is untrusted: the Lean exporter, Rocq, the HOL family, and our own (claim elaborator, case-split decision procedure, SAT/LRAT reconstruction, arithmetic certificates). Lean is one optional producer, no longer a dependency. Every extra principle of a source logic enters as a **named axiom** that shows in the claim's axiom footprint and VER31 bucket | 1, 6 |
| D4 | The kernel derives recursors, quotient constants and literal support itself. A producer never supplies them; they are not even in the format | 3.4 |
| D5 | The reader of PIF is in the trusted base and is at most 600 lines (VER11a §4.2, "Export reader"). Whether it is hand-written and differentially tested, or generated from the grammar, is owner question Q1 | 5 |
| D6 | The first proof target is **logic gates**, ahead of ct-compare, because it is easier at every level (model, input contract, code). ct-compare follows and reuses the pipeline | 9 |
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
| The pin file and `lean-kernel-ffi` wrapper (VER11a §4.3) | Yes, as in VER11a | Unchanged |
| Producers: Lean exporter, Rocq, HOL, claim elaborator, case-split, LRAT, arithmetic | **No** | Their output is a candidate; a wrong candidate is rejected |
| Converters (Lean→PIF, Rocq→PIF, HOL→PIF) | **No** | Separate crates; a converter that drops or alters a declaration yields a different environment which the kernel checks as it is |
| `pif-fmt`, `pif-pp` (canonical checker, pretty-printer) | No | Convenience; the kernel verdict does not depend on them |
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
| emits a well-typed proof of a *different* statement than the claim | The gate compares the theorem's type with the signed goal (VER11a §7.3): rejected |
| redefines a trusted definition (`obs`, a gate's model) | The allow-listed environment digests plus "a claim file may not declare a constant that occurs in the goal" (VER11a §7.3 step 3): rejected |
| adds an axiom to make the proof go through | Every axiom the theorem depends on is listed in the footprint and classified (VER11a §4.5); a claim that depends on an unregistered axiom is rejected |
| supplies a wrong recursor, quotient constant or literal rule | Cannot: the format has no place for them (section 3.4) |
| is a converter that *mistranslates a source statement* (for example, reads Rocq's `nat` as a different type) | The result is a true Lean-logic theorem about something else. For third-party libraries we make **no claim that the converted statement is the source's statement**. For our own claims the statement is the signed goal, so a mistranslation is a goal the signer would reject or whose digest would not match |

The last row is the honest limit of the whole design and is stated again in
section 6: *a converter's faithfulness decides what a theorem is about, not
whether its proof is valid*. The first is covered by the signed goal for our
claims; the second is the kernel's job.

### 1.4 Source logics' extra principles are named axioms

A source logic may assume something Lean's type theory does not (classical
logic and Hilbert choice in HOL; Rocq's `Classical_Prop.classic`, or the
`functional_extensionality` axiom). The converter has two options and must pick
one per principle:

1. **Derive it**: the principle is a theorem in Lean's logic (HOL's excluded
   middle follows from `Classical.choice`, `propext` and function extensionality,
   which itself follows from `Quot.sound`). The converter emits the proof, and
   the kernel checks it. No new axiom.
2. **Name it**: declare an `axiom` whose name starts with `Src.<profile>.`, for
   example `Src.rocq.Classical_Prop.classic`. It then appears in the axiom
   footprint of every theorem that uses it and is classified per VER11a §11.1.
   An unclassified `Src.*` axiom is rejected for claims (VER11a §4.5, "Anything
   else"); for library import (K-M0 style) it is accepted and listed.

The three standard Lean axioms (`propext`, `Classical.choice`, `Quot.sound`)
keep their Lean names in every profile, because they are the logic's own
(`kernel.logic.consistent`, VER11a §11.2).

---

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
| `axiom` | none | add; classified in the footprint |
| `def` | value, reducibility hint (`@abbrev`, `@opaque`, `@regular h`) | check value against type, add |
| `theorem` | value | check; the type must be a `Prop` (VER11a §3.3) |
| `opaque` | value | check, add with the body hidden from δ |
| `inductive` (a block) | see 3.3 | check, then **derive** constructors' use, recursors and the block's facts |
| `quot` | none | check that `Eq` has the expected shape, then add `Quot`, `Quot.mk`, `Quot.lift`, `Quot.ind` |
| `prim role name` | a role and the constant playing it | validate the declaration against the role (3.5) |
| `goal id name digest` | claim id, theorem name, digest of the printed goal | no kernel action; recorded for the gate (3.6) |

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
expected definitions (VER11a §3.3, "Primitive definitions are validated"). PIF
therefore says which constant plays which role, and the kernel validates the
declaration before enabling the rule:

Shown with constant names for readability; on disk the operand is a name index.

```
prim nat Nat           -- the type Nat
prim nat.zero Nat.zero
prim nat.succ Nat.succ
prim nat.add Nat.add   -- … and sub, mul, div, mod, gcd, beq, ble, land, lor,
                       --   xor, shiftleft, shiftright, pow, log2
prim string String     -- and string.mk, char, char.ofnat, list.nil, list.cons
prim eq Eq             -- the equality type, used by the quotient package check
```

The set of roles is closed and belongs to the kernel-logic version (section 8).
A role the kernel does not know is an error. A `prim` record is valid only after
the named constant exists. **A file that uses no literal acceleration and no
quotient declares neither feature and contains no `prim` record**: the gates'
first milestone (section 9) is such a file, which keeps the literal and
quotient machinery out of its trusted footprint.

### 3.6 Goal bindings

```
goal "<claim id>" <theorem name> sha256:<64 hex>
```

maps a theorem to the Warrant claim goal it is meant to prove and to the digest
of the **printed goal** (VER11a §7.3 and §7.4). The kernel does not interpret a
goal record. The reader checks that the theorem exists and was accepted, and
exposes the pairs. The release gate does the binding: it elaborates the claim
itself, compares the elaborated goal with the theorem's type, and requires the
digest to be the one in the signed `goals_sha256`. The digest is of the printed
text a human read; its print format is the claim elaborator's (VER12, VER11a
KRN-011), not PIF's.

### 3.7 Files compose by digest, not by index

A PIF file is a **delta over an environment**: its header may `import`
other PIF files by SHA-256. Indices (names, levels, expressions) are local to
a file; two files refer to each other **only by constant name**. The reader
loads imports first, in order, then the file. A claim's gate allow-list names
the digests of the prelude and libraries it accepts (VER11a §7.3 step 2).

---

## 4. The concrete syntax

Defined normatively by `code/grammars/pif/pif.tokens` and
`code/grammars/pif/pif.grammar` (a backlog item, PIF-001; this section is the
intended content). The format is designed so that the *reader is trivial*:
split a line on single spaces, look at the first word.

### 4.1 Rules of the lexical form (line discipline)

| # | Rule | Why |
|---|---|---|
| L1 | The file is UTF-8, lines end in a single LF (`\n`), the last line too. No CR, no tabs, no NUL | One way to write a file |
| L2 | Fields are separated by **exactly one** space. No leading or trailing space on a line, no blank lines, no comments | Canonical bytes |
| L3 | Indices and numbers are decimal, no sign, no leading zeros, no `_`. Index `0` is only the anonymous name and the level `zero` | Canonical |
| L4 | A string is `"…"` with printable ASCII (0x20 to 0x7e) except `"` and `\`, which are written `\"` and `\\`; every other code point is `\u{h}` with lowercase hex and no leading zeros. Names with non-ASCII text (`α`) are therefore ASCII on disk | Canonical, no normalisation issue |
| L5 | A digest is `sha256:` and 64 lowercase hex digits | Canonical |

Rules L1 to L5 are checked by the reader (and by `pif-fmt`). They are stated as
a pre-pass rather than as grammar productions, because a grammar that carried
an explicit space token on every field would be noisy and would obscure the
production structure. The grammar's lexer skips single spaces; the pre-pass
rejects runs of them. Section 5 requires both reader implementations to run
the same pre-pass.

### 4.2 `pif.tokens` (intended content)

```
# pif.tokens — Proof Interchange Format, version 1
# @version 1
# Lines are separated by NEWLINE; fields by single spaces (the pre-pass of
# VER32 section 4.1 rejects any other spacing). No comments exist.

NAT      = /0|[1-9][0-9]*/
STRING   = /"([\x20\x21\x23-\x5b\x5d-\x7e]|\\["\\]|\\u\{[0-9a-f]{1,6}\})*"/
SHA256   = /sha256:[0-9a-f]{64}/
IDENT    = /[a-z][a-z0-9_.]*/
HABBREV  = "@abbrev"
HOPAQUE  = "@opaque"
HREGULAR = "@regular"
LBRACKET = "["
RBRACKET = "]"
NEWLINE  = "\n"

keywords:
  pif logic profile features import
  nstr nnum lsucc lmax limax lparam
  evar esort econst eapp elam epi elet enat estr eproj
  axiom def theorem opaque inductive itype ictor quot prim goal end
  default implicit strict inst

skip:
  SPACE = / /
```

`features`' operands (`lits`, `quotient`, `mutual`, `nested`), profile ids
(`lean4`, `rocq`, `hol`, `native`) and `prim` roles (`nat.add`, `string.mk`, …)
are `IDENT`s; the kernel-logic version fixes which are valid. (`quotient`, not
`quot`, so that the feature name does not collide with the `quot` keyword.)

### 4.3 `pif.grammar` (intended content)

```
file      = header { record } trailer ;

header    = "pif" NAT NEWLINE
            "logic" NAT NEWLINE
            "profile" IDENT STRING NEWLINE
            "features" { IDENT } NEWLINE
            { "import" SHA256 NEWLINE } ;

record    = name_rec | level_rec | expr_rec | decl ;

name_rec  = "nstr" NAT NAT STRING NEWLINE      # id, parent, component
          | "nnum" NAT NAT NAT NEWLINE ;

level_rec = "lsucc" NAT NAT NEWLINE            # id, level
          | "lmax" NAT NAT NAT NEWLINE
          | "limax" NAT NAT NAT NEWLINE
          | "lparam" NAT NAT NEWLINE ;         # id, name

expr_rec  = "evar" NAT NAT NEWLINE             # id, de Bruijn index
          | "esort" NAT NAT NEWLINE            # id, level
          | "econst" NAT NAT levels NEWLINE    # id, name, levels
          | "eapp" NAT NAT NAT NEWLINE
          | "elam" NAT binfo NAT NAT NAT NEWLINE    # id binfo name type body
          | "epi" NAT binfo NAT NAT NAT NEWLINE
          | "elet" NAT NAT NAT NAT NAT NEWLINE      # id name type value body
          | "enat" NAT NAT NEWLINE
          | "estr" NAT STRING NEWLINE
          | "eproj" NAT NAT NAT NAT NEWLINE ;       # id struct-name index expr

decl      = "axiom" NAT params NAT NEWLINE               # name, params, type
          | "def" NAT params NAT NAT hint NEWLINE        # name params type value hint
          | "theorem" NAT params NAT NAT NEWLINE
          | "opaque" NAT params NAT NAT NEWLINE
          | inductive
          | "quot" NEWLINE
          | "prim" IDENT NAT NEWLINE
          | "goal" STRING NAT SHA256 NEWLINE ;

inductive = "inductive" params NAT NAT NEWLINE itype { itype } ;
itype     = "itype" NAT NAT NAT NEWLINE { ictor } ;      # name type n-ctors
ictor     = "ictor" NAT NAT NEWLINE ;                    # name type

levels    = LBRACKET { NAT } RBRACKET ;
params    = LBRACKET { NAT } RBRACKET ;
binfo     = "default" | "implicit" | "strict" | "inst" ;
hint      = HABBREV | HOPAQUE | HREGULAR NAT ;

trailer   = "end" NAT NEWLINE ;
```

**LL(1).** Every alternative of `record`, `expr_rec`, `level_rec`, `name_rec`,
`decl` and `hint` starts with a distinct token; the one repetition that is not
decided by its first token, the constructors of a type, is decided by whether
the next token is `ictor` (`itype`, another record keyword or `end` otherwise).
Counts (the number of types in a block, of constructors in a type, of fields in
`params` and `levels`) are checked by the reader, not the grammar, as a grammar
cannot count. `ictor`'s own `NAT NAT` and the `itype`'s third `NAT` (its
constructor count) are therefore *validated*, never trusted.

### 4.4 What each record means: semantics the grammar does not carry

| Rule | Statement |
|---|---|
| Indices | Names, levels and expressions have **separate** index spaces. Name `0` is the anonymous name; level `0` is `zero`. Every other record defines the next index of its space: **the first of a kind is 1 and the indices are dense and increasing**. A record may reference only an index already defined (backward only), so a forward or self reference is a syntax error, and cycles cannot be written |
| `nstr i p s` | name `i` is the name `p` extended by the string component `s` (`p` is `0` for a top-level name) |
| `lparam i n` | level `i` is the universe parameter named by name `n` |
| `elam i b n t e` | `e` is the body in which the binder is index 0. The reader does not check de Bruijn indices against the binder depth; the kernel's closedness check does |
| `params [n…]` | name indices of the universe parameters, in order, distinct |
| `econst i n [l…]` | level indices; the count must match the constant's declaration (the kernel checks) |
| `inductive ps np nt` | `ps` universe parameters, `np` parameters, then **exactly** `nt` `itype` records |
| `end k` | `k` is the number of record lines between the header and `end`, counting every physical line (an `inductive` block counts its `inductive`, `itype` and `ictor` lines). A truncated file, or one with a record removed, fails |

### 4.5 The canonical form

Two files that mean the same must be the same bytes, so that digests (the pin
file, the gate's allow-list, `goals_sha256`'s inputs, report fields) are stable.
A **canonical PIF file** satisfies the line discipline (L1 to L5) and, in
addition:

| # | Rule | Checked by |
|---|---|---|
| C1 | Maximal sharing: no two records of one space have identical content (an expression `eapp 7 3 4` may exist once) | `pif-fmt`; the reader also refuses a duplicate expression because the kernel's hash-consing returns the existing node |
| C2 | Emission order is the post-order of the declarations: for each declaration in file order, every name, level and expression it uses and has not been emitted yet is emitted first, depth-first, left-to-right in field order, immediately before the declaration's first line. An inductive block's records come before the `inductive` line; the block lines are contiguous | `pif-fmt` |
| C3 | No record is unreferenced (every name, level or expression record is used by a later record or a declaration) | `pif-fmt` |
| C4 | Declaration order is the producer's; the canonical form does not reorder declarations, because order is part of the meaning (backward references) | n/a |

**Decision / alternatives / why.** *Decision:* the **reader enforces L1 to L5,
dense indices, backward references and the record count**, and the untrusted
`pif-fmt --check` enforces C1 to C3. *Alternatives:* (i) the reader enforces the
whole canonical form; (ii) nothing is enforced and digests are over "whatever
the file says". *Why:* a non-canonical file is not a soundness problem (the
kernel checks exactly the records it reads), only a stability problem; (ii) lets
two bytes-different files with the same meaning get different digests, which
breaks only convenience, and (i) would put an expression-sharing table check and
a reachability pass into the trusted reader and push it past its budget. The
gate requires the file's digest to be on its allow-list, and allow-listed files
are canonical because `pif-fmt` is a CI gate on committed files.

### 4.6 Worked example

The file below is complete and canonical. It defines `Bool` and `Eq` as
inductive types, defines `Bool.and` with `Bool.rec`, and proves that `and` is
commutative over `Bool` by case analysis, with a goal binding. It needs no
literals, no quotient and no imports. (The numbering was produced by a throwaway
script that follows C1 and C2; the claim that the file type-checks is the
corpus's first positive test, PIF-005.)

```
pif 1
logic 1
profile native "pif-case-split 0.1"
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
goal "gates.and_comm" 13 sha256:1ce456c5ee7a492801703243f6d69713c9069191c95c42fa0e8d5c2f4bc2c704
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
| 100 | the goal binding for the gate: the digest is SHA-256 of the printed goal `forall (a b : Bool), Bool.and a b = Bool.and b a` followed by a newline (the print format here is illustrative) |
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
8,500-line budget) and replaces the lean4export reader in that row. Its
intended parts:

| Part | Lines (budget) | Contents |
|---|---|---|
| Line discipline and tokenising | ≤ 150 | L1 to L5, fields, bounded line reading (7) |
| Index tables and node builders | ≤ 250 | per-space tables; dense, backward-only indices; depth bookkeeping without recursion |
| Declarations and blocks | ≤ 100 | `axiom`, `def`, `theorem`, `opaque`, inductive blocks with counts, `quot`, `prim` |
| Header, imports, trailer, limits | ≤ 100 | version checks, feature check, `end k`, error type |

It does no I/O except reading a bounded byte stream given to it, uses no
recursion on input-controlled depth, has `#![forbid(unsafe_code)]`, and is
literate like the rest of the kernel.

### 5.2 Owner question Q1: how the reader is built

The repository rule is that lexers and parsers are grammar-driven, never
hand-written. A *trusted* reader must also be small and auditable. These
conflict, as VER11a Q9 said for JSON. The grammar of section 4 is normative in
both options. The options:

| | **(a) Generated from the grammar** | **(b) Tiny hand-written reader, differentially tested** |
|---|---|---|
| How | `grammar-tools` generates the lexer and parser from `pif.tokens` and `pif.grammar`; a thin semantic layer feeds the kernel | A ≤ 600-line reader that splits lines and matches the first word; the grammar-driven parser exists too, and the two are run **on every corpus file and on generated files**, comparing accept/reject and the full record stream |
| Trusted base added | `grammar-tools` (lexer and parser engines, grammar compiler, the generated code), counted against the budget | Only the reader itself |
| Size | The generated code is large and not meant to be read | The 600 lines are the whole thing |
| Failure mode | An engine bug (a token matched too liberally) silently changes what the kernel is shown | A divergence between reader and grammar parser is found by the differential, in CI, before any gate relies on the reader |
| Rule compliance | Full | Needs an owner exemption: a hand-written *trusted* reader of a grammar-defined format, justified by the differential |
| Cost of the differential | n/a | A grammar-driven random generator of valid and mutated files (cheap: the grammar is the generator's input) |

**Recommendation: (b).** A reader's job here is `split on a space and read the
first word`; the format was designed so that the trusted code is the easy part.
Under (a) the trusted base would include an engine of several thousand lines to
parse a format whose parser is a hundred lines of obvious code, and a bug in the
engine would be invisible, whereas under (b) the grammar-driven parser is an
independent implementation that *checks* the reader instead of being it. Option
(b) therefore has strictly more independent checking at a much smaller trusted
size. The exemption is narrow (this reader only), and (a) can replace (b) later
if `grammar-tools` is ever audited to the level of the kernel (listed in the
backlog as PIF-013).

**Either way** the differential harness is built (it is the test of (b) and the
oracle for (a)), and the reader is covered by the hostile corpus of section 10.

---

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
| Named axioms | `propext`, `Classical.choice`, `Quot.sound` keep their names; `Lean.ofReduceBool` and `Lean.trustCompiler` are oracle axioms (VER11a §4.5); `sorryAx` is forbidden for claims |
| Unsupported at first | None by design (it is the identity); the first milestone is `Init` (K-M0) |
| Why it is optional now | Claims no longer need Lean in the build path (VER11a Q2's tool exception shrinks to library import and differential testing) |

### 6.2 Rocq (`rocq`)

| Aspect | Mapping |
|---|---|
| `Prop` | Lean's `Prop` |
| `Set` | `Type 0`, but Rocq's `Set` is *predicative* by default (impredicative Set is unsupported, below), so it is an ordinary universe |
| `Type@{i}` | `Sort (i+1)` in Lean's numbering (`Type` is `Sort 1`) |
| `SProp` | mapped to `Prop`: Lean's `Prop` already has definitional proof irrelevance, which is the property `SProp` is for. Anything that checks in Rocq with `SProp` checks in Lean with `Prop`; the converse is not claimed and not needed |
| Cumulativity | Rocq has `Prop ≤ Set ≤ Type_i` and (with cumulative inductives) cumulative parameters; Lean has no subtyping between universes. The converter inserts **explicit lifts** (`PLift`, `ULift`, which are ordinary inductives) at every use of cumulativity. This is the main cost of the converter and the main source of its refusals |
| Universe constraints | The converter solves Rocq's universe constraint graph and writes the resulting level expressions (`max`, `succ`, `param`); the kernel re-checks them, so a wrong solution is rejected |
| Inductives | Mapped to `inductive` blocks when positivity and universe rules match; Rocq's template polymorphism and its special elimination rule for `Prop` inductives are re-expressed or refused |
| `Fixpoint` | Compiled to the derived recursor by the converter when the recursion is structural on one argument of an inductive type at top level. This is untrusted translation work (it is a real compiler) |
| Records | Rocq primitive records map to structures; η for records matches Lean's structure η |
| Modules and functors | Flattened by the converter |
| Named axioms | `Src.rocq.<qualified name>` for every `Axiom`/`Parameter` the development depends on; the classical ones (`Classical_Prop.classic`, `functional_extensionality`) may instead be *derived* (section 1.4) |
| Unsupported at first | **Impredicative Set** (`-impredicative-set`); **primitive integers, floats, arrays and strings** (`PrimInt63`, `PrimFloat`, `PArray`, `PString`), which need a native-arithmetic extension of the kernel's logic version; **coinduction and `CoFixpoint`**; **non-structural or nested guarded fixpoints**; **`Unset Universe Checking`** and other checks turned off; **Rocq's `Prop` elimination into `Set` where Lean's subsingleton rule forbids it**; type-in-type |
| What a converted theorem means | A theorem *in Lean's logic* about a translation. Faithfulness to the Rocq statement is the converter's tested property, not a kernel property (section 1.3) |

### 6.3 The HOL family (`hol`)

Covers HOL Light, HOL4, and Isabelle/HOL (through its proof terms).

| Aspect | Mapping |
|---|---|
| Types | Simple types with type variables: `Type u` for a type variable of level `u`; function types are `→`; `bool` is `Prop`; `ind` is `Nat` (Lean has it; HOL's axiom of infinity becomes a theorem) |
| Terms | Simply-typed λ-terms; equality at `bool` is `Iff`/`Eq` on `Prop`; the HOL primitive `=` is `Eq` |
| Classical logic | Excluded middle is derived from `Classical.choice`, `propext` and function extensionality (a theorem in Lean from `Quot.sound`) |
| Hilbert's ε | `Classical.choice` (with the non-emptiness proof HOL's `SELECT_AX` provides) |
| Extensionality | Function extensionality (derived from `Quot.sound`) and `propext` |
| Definitions | HOL's constant definitions become `def`; its **type definitions** (`new_type_definition` from a non-empty predicate) become a `Subtype`-style `def` plus the abstraction and representation functions as `def`s, with the bijection laws as theorems — not a new axiom |
| Named axioms | **None beyond the three Lean axioms** (all of HOL's principles are derivable). If a converter chooses to assume one instead, it must name it `Src.hol.<name>` |
| Proof terms | HOL Light, HOL4 and Isabelle all record inference-level proofs (OpenTheory articles, Isabelle's proof terms); the converter replays them as Lean terms using lemmas for each primitive rule |
| Unsupported at first | **Isabelle type classes, locales and sort constraints** (dictionary passing is a separate translation); **ad hoc overloading**; **Isabelle's `Pure` framework layer** except where it reduces to `bool`; **HOL4's theory-level definitions by recursion schemas** unless unfolded to primitive definitions |

### 6.4 Native producers (`native`)

The repository's own producers emit PIF directly: no source logic, so
**no extra axioms**: the profile is Lean's type theory itself.

| Producer | Emits | Notes |
|---|---|---|
| **Claim elaborator** (VER11a §7.1, KRN-011) | The *goal*, as a PIF statement (a type), and its `goal` binding | Untrusted; its output is signed (VER11a §7.4) |
| **Case-split decision procedure** (PIF-006) | A proof by exhaustive case analysis over finite inductives (`Bool`, `Option`, finite enumerations), `Bool.rec` and `Option.rec` trees closed by `Eq.refl` | The gates' prover (section 9). Fails closed: if a leaf is not closed by `Eq.refl` the kernel rejects |
| **SAT/LRAT reconstruction** | LRAT steps as proof terms (VER11a §9.5, KRN-009) | Later |
| **Arithmetic certificates** | Linear-arithmetic and bound proofs over `Nat` (VER11a KRN-012a) | Later |
| **Prelude generator** (PIF-004) | The small checked prelude: `Bool`, `Nat`, `Int`, `Eq`, `And`, `Or`, `False`, `True`, `Option`, `List`, `Prod` with the same names and types as Lean's `Init`, so a Lean-converted environment and the native prelude agree declaration by declaration | The gate allow-lists its digest (VER11a §7.3 step 2) |

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
| Records per file | 4,000,000 | Same as the kernel's new-node cap; also bounds the index spaces |
| Name components | 32 | `nstr`/`nnum` chain depth, computed at insert |
| String component or literal | 64 KiB | |
| `nat` literal digits | 100,000 | Plus the kernel's `Nat.pow` exponent cap for acceleration |
| Level nesting depth | 256 | Computed at insert, not by recursing |
| Expression depth | 1,000 | Equals the kernel's `max_term_depth`; the reader stores each node's depth at insert (1 + the max of its children) and rejects, so it never recurses |
| Universe parameters per declaration | 64 | |
| Levels per `econst` | 64 | |
| Declarations per file | 1,000,000 | |
| Inductive block | 64 types, 4,096 constructors per type, 256 parameters | The 256 is below the kernel's `MAX_BVAR_INDEX` |
| Imports | 64, no cycles by construction (digests), loaded depth ≤ 8 | |
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
5. **Duplicate and forward references are errors**, never repaired.
6. **Resource use of the kernel is the kernel's** (`Limits`); the reader
   forwards them unchanged.

Error kinds are a closed enum so that the corpus can assert the *reason* for
each rejection (`LimitExceeded(kind)`, `BadLexicalForm`, `UnknownVersion`,
`UnknownFeature`, `FeatureNotDeclared`, `IndexNotDense`, `ForwardReference`,
`DuplicateRecord`, `CountMismatch`, `TrailerMismatch`, `UnknownRole`,
`UnknownImport`, and the kernel's own `KernelError`s).

---

## 8. Versioning and hashing

### 8.1 Three versions, three meanings

| Header line | Name | What it versions | Who reads it |
|---|---|---|---|
| `pif N` | **Format version** | Syntax: the tokens, grammar, line discipline, canonical rules | The reader. A value it does not know is a hard error |
| `logic N` | **Kernel-logic version** | Semantics: the set of declarations the kernel accepts and the meaning of each record (including the closed set of `prim` roles and `features`) | The kernel. A value it does not know, or a *withdrawn* one, is a hard error |
| `profile P "…"` | **Producer profile** | Which converter and version produced the file | The report only; never trusted |

The first release is `pif 1`, `logic 1`. `logic 1` is Lean 4's kernel theory as
fixed by VER11a §3.3 at the Lean pin of KRN-003.

### 8.2 Compatibility rules

| # | Rule |
|---|---|
| V1 | The reader supports an explicit **list** of format versions. It never guesses a newer one |
| V2 | A format change that is only additive in meaning (a new record kind, a new feature) increments `pif` and, if the new records need new kernel behaviour, `logic`. A file that uses none of the new things keeps its old header and digest, and the digest is never rewritten |
| V3 | A **soundness fix** that makes the kernel *reject* something it used to accept (a rule that was too liberal) **withdraws** the older `logic` value. Files declaring a withdrawn value are rejected with `WithdrawnLogicVersion` and must be regenerated. This is deliberate: an old checked proof of a statement a buggy kernel accepted must not keep its standing |
| V4 | A new `prim` role or feature is added in a new `logic` version, so a kernel that knows less rejects the file instead of ignoring the role |
| V5 | Converters and `pif-fmt` target one `pif` version each; an `pif-upgrade` tool (untrusted) may rewrite a file to a newer syntax, and the result is a **new file with a new digest** whose header records the upgrade |
| V6 | The pin file (VER11a §4.4) records the `pif` and `logic` versions the kernel accepts, the Lean toolchain tag and lean4export version used by the Lean converter, and nothing else. Moving any of them is its own PR with the corpora of section 10 rerun |

### 8.3 Hashes

| Digest | Definition |
|---|---|
| `file_sha256` | SHA-256 of the file's exact bytes. Because canonical files are byte-for-byte unique, it identifies the file |
| `environment_sha256` | Folded over the imports and the file: `H₀ = SHA-256("pif-env-1\n")`; `Hᵢ = SHA-256(Hᵢ₋₁ ‖ file_sha256ᵢ)`. A checked claim's report records the environment digest of everything it was checked on |
| Goal digest | SHA-256 of the printed goal text, in the `goal` record (3.6). The `goals_sha256` of the VER31 sign-off aggregates them and is VER31's to define |
| Certificate id | `pif:sha256:<file digest>` replaces `lean-export:` as the VER11a §11.3 certificate kind (an amendment, Q5) |

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

**Why gates are easier, at every level.**

| Level | Gates | ct-compare |
|---|---|---|
| **Model** | The specification is a finite case analysis over `Bool`: every statement is decided by splitting on at most two bits and closing each leaf by computation. No loops, no induction, no arithmetic | Needs lengths, loops and a relational (two-run) property |
| **Input contract** | A gate accepts only 0 and 1 and rejects everything else (LG01, "Bit representation"). A kernel proof of "rejects every non-bit" quantifies over **every integer**, not the 254 values of `u8` per argument that exhaustive tests walk, and in lanes with unbounded integers (Python, Ruby, Haskell `Integer`, Elixir, Perl) tests *cannot* walk them. The proof is also short: `Int` has two constructors, `Nat` two, and one `Nat.rec` step | Input contracts are over slices and lengths |
| **Code** | The Rust gates are straight-line functions: a validation, one expression, no loops, no memory, and no two-run relational property. Lowered to WIR they are a handful of instructions each | Loops, memory, secrets and a two-run relation (VER11a §8) |
| **Kernel features needed** | Only the earliest ones: non-recursive and simple recursive inductives, recursors, ι, proof irrelevance, function η (9.3) | The full VER11a §14.2 stack |
| **Review** | Each theorem can be read in full by a person | Requires trusting a VC generator's output |

**An honest note on what the kernel adds for the Rust lane.** The Rust gates take
`u8`: 65,536 input pairs, which LG02's corpus already walks exhaustively. The
kernel proof's *gain* is (i) the same statement for every integer, which is the
statement the other 17 lanes need, (ii) an independent check of the exhaustive
walk's conclusion, and (iii) the first complete exercise of the whole pipeline.
It does not make the Rust gates safer than their exhaustive tests make them. The
real step up in assurance is G-M2 and G-M3 (code level and every-width adders).

### 9.2 G-M1: the model level

**Domain.** The model's input type is `Int` (Lean's integer, an inductive with
constructors `ofNat : Nat → Int` and `negSucc : Nat → Int`), the widest domain
every lane's integer type embeds into. The Rust `u8` (and any bounded integer)
is a corollary by restriction. The `u8` choice is owner question Q4.

**The model, in the gates' actual API** (LG01):

```
toBit : Int → Option Bool
  -- ofNat 0 ↦ some false, ofNat 1 ↦ some true, anything else ↦ none
gate  : Int → Int → Option Int        -- the primitive and NAND-built gates
not'  : Int → Option Int
```

A gate returns `none` where the Rust gate panics. This is the model of LG01's
"rejected by a panic in every build profile": the claim binding normalises a
panic to `Err(INVALID_INPUT)`, and LG01 states that the single case means "this
call panicked", with no distinction between a non-bit first argument and a
non-bit second one, so `none` is the faithful value. A binary gate is
`bind (toBit a) (fun x => bind (toBit b) (fun y => some (fromBit (f x y))))`,
with `f` the boolean function and `fromBit` the embedding back to `ofNat 0` or
`ofNat 1`. The NAND-built gates are the same shape with `f` assembled from `nand`
exactly as in the Rust source (`nand_not a = nand a a`, `nand_and a b = nand_not
(nand a b)`, `nand_or a b = nand (nand_not a) (nand_not b)`, `nand_xor` the
four-NAND form).

**Theorems** (each is a PIF `theorem` with a `goal` binding; there are no
`sorry`s, no oracle and no axioms beyond those PIF already has):

| Group | Statement | Proof shape |
|---|---|---|
| **T1 domain** | `toBit` is exactly the set {0, 1}: `toBit (ofNat 0) = some false`, `toBit (ofNat 1) = some true`, `∀ n, toBit (ofNat (n+2)) = none`, `∀ n, toBit (negSucc n) = none`, and **every `Int` is one of the four shapes** | `Eq.refl` (ι on `Nat.rec` with a symbolic `n` whose head is `succ (succ _)`); the exhaustiveness is one `Int.rec` and one `Nat.rec` |
| **T2 truth tables** | for each of the seven primitives, every row of its table, with rows taken from the LG02 corpus (an independent oracle) *not* derived from the model's own boolean operators (the OCaml lesson in the logic-gates backlog: expected values computed with the same operator test nothing) | seven `Eq.refl` conjunctions. There are 2 + 6 × 4 = 26 rows |
| **T3 equivalence** | for each NAND-built gate `g'` and its primitive `g`: `∀ a b : Int, g' a b = g a b` (over **every** integer, including both `none` cases) | Generalise over `toBit a`, `toBit b` (`Option.rec`), then `Bool.rec` on the bits; four equations |
| **T4 rejection** | for each of the eleven gates and each argument position: `∀ a b : Int, a ∉ {0,1} → g a b = none`, and the same for `b`, and `not'`'s one argument | T1 then the `none` case of `bind` |
| **T5 non-vacuity** | for each gate: a witness input pair accepted (`g 0 1 = some …`) and a witness rejected (`g 2 0 = none`), following VER11a §7.5's witness rule | `Eq.refl` |

**Producer.** The case-split decision procedure (PIF-006): untrusted, emits PIF
for T1 to T5 from the model's definitions (a table of the gates, nothing else).
Our kernel checks the PIF file. **Lean's kernel is an optional differential
oracle**: a PIF→Lean replay harness (PIF-010) feeds the same declarations to
`Lean.Environment.addDecl`, and its verdict is recorded as evidence, never a
gate dependency (VER11a §4.3, "The oracle's harness").

**VER31 rung.** The model-level theorems reach rung `checked`
(VER31 §4: "a certificate covers every input, but the result still rests on our
own unverified tools or an unsigned statement"), **relative to the model-to-code
ids** of 9.5: a person must sign that `toBit`/`gate` are what LG01 says, and
nothing here shows the Rust code *is* the model (that is G-M2).

**What G-M1 needs from the kernel** (so the milestone is a precise acceptance
test for KRN-002 slice 3a):

| Feature | Used for |
|---|---|
| Non-recursive inductives | `Bool`, `Option`, `Prod`, `True`, `False`, `And`, `Or`, `Int` |
| Simple recursive inductive, one type, no indices | `Nat` |
| Inductive with an index | `Eq` |
| Recursors derived by the kernel, ι-reduction | `Bool.rec`, `Nat.rec`, `Int.rec`, `Option.rec`, `Eq.rec`, `False.rec` |
| β, δ (hints), ζ | unfolding the gates |
| Proof irrelevance | proofs of `And`, `Or` as arguments |
| Function η | motives and `Nat.rec` on eta-expanded cases |
| Large elimination for `Prop` inductives with one constructor (`And`, `Eq`) | eliminating `Eq` into data in the rejection proofs |
| **Not needed** | literals (`lits`), quotients, mutual and nested inductives, structure η, `Nat` acceleration |

**Assumption ids** (VER31 buckets; ids marked *new* need registration by
PIF-016 with the owner's sign-off, as VER11a §11.2 prescribes):

| Id | Bucket | G-M1 |
|---|---|---|
| `kernel.checker.correct` | (c) | **Opens/relies on**; closed per claim by the independent agreement route (VER11a Q6) |
| `kernel.logic.consistent` | (a) proposed | Relied on |
| `pif.reader.correct` *(new)* | (c) | **Opens**: the reader is trusted (section 5). Closed by the differential (Q1) and the second checker's own reader |
| `spec.goal.faithful` | (b) | **Opens**: the printed goals are signed |
| `gates.model.faithful` *(new)* | (b) | **Opens**: `toBit` and the gate definitions say what LG01 says, including "reject by panic" modelled as `none` |
| `gates.model.code-link` *(new)* | (c) | **Opens**: nothing connects the model to the Rust code. Closed by G-M2, which replaces it by the WIR ids |
| Closes | none | G-M1 closes no existing (c) id; it adds a `checked` model theorem and a reviewed pipeline |

### 9.3 G-M2: the code level

**Target.** The Rust lane's gates, lowered to WIR by the existing front end
(VER25a, VER25b), with the semantics of **straight-line WIR**: the subset of
VER11a §6 (`run`, the frame stack, `ip`, cost schedule from VER25 §8) restricted
to blocks without back edges, no allocation, no memory access, calls only to
other gates. This is the part of KRN-007 (`warrant-lib`) that a loop-free
program needs, so G-M2 does **not** wait for the loop and relational machinery.

**Theorems.** The G-M1 theorems restated over the WIR programs:

- T2' / T3': for all 8-bit (or all `Int` for lanes where that is the type)
  argument values, `run n gatesWir (entry …)` ends in `ret v` with `v` as the
  model says, and never in `trap` or `stuck`;
- T4': every argument outside {0,1} ends in the **panic outcome** the semantics
  gives `assert!` (a trap with the gate's message identity), with no effect;
- `no_panic` on every input in {0,1}: `ret`, never `trap`;
- the **link theorem**: `run … = ret (fromOption (gate a b))` for all `a`, `b`,
  which is the statement that *the code is the model*, closing
  `gates.model.code-link` into the existing WIR ids.

**A finding to settle before G-M2 (owner question Q2).** The brief for this
milestone assumed the Rust gates are straight-line with no loops or memory. That
holds for `validate_bit` and for the NAND-built gates, which only call `nand_gate`.
The seven **primitive** gates, however, delegate to the `transistors` crate's CMOS
model (`CMOSInverter::evaluate_digital`, `CMOSNand`, …), a floating-point voltage
simulation, and are not in the lowered subset (floating point, `Result`,
`Option::unwrap`, constructors with `None` arguments). So:

| Option | What G-M2 covers | Cost |
|---|---|---|
| **A. Call-contract first (recommended)** | `validate_bit` and the four NAND-built gates fully; `nand_gate` is treated as a **call with a stated contract** (`nand_gate` of two bits returns the bit-level NAND and otherwise panics), registered as the (c) id `gates.nand.primitive-contract`. The other six primitives follow when the `transistors` digital path enters the subset | Honest; shows the call/frame machinery working; leaves one named (c) id open |
| B. Pure-bit bodies | Change the Rust gates to compute `a & b` etc. after validation and keep the transistor model as a differential check | Changes a package about 47 crates depend on (LG01) and changes what the library *is* |
| C. Lower the transistor model | The full chain from the gate to its CMOS evaluation | Needs floating point in the WIR semantics; far larger |

**Kernel features G-M2 adds to G-M1:** the WIR semantics definitions in the
library (structural step-count fuel, VER11a §6.2), `Nat`/`UInt8` arithmetic
facts for `validate_bit`'s comparisons (`u8` is `Fin 256` or `BitVec 8`; core
lemmas from the library), and **no literals if the semantics is written over
unary-coded constants**. If the WIR program uses numeric constants and the
semantics' `UInt8`s are literal-based, G-M2 declares `lits` and the `prim`
validations of 3.5 become part of the trusted footprint, which is recorded as the
reason to import Lean's `Init` through the Lean converter at that point.

**Assumption ids.** Opens: `wir.lowering.faithful`, `wir.semantics.lean-faithful`,
`wir.typing.checker-guarantees`, `wir.interp.differential-only` (existing (c)
ids), `gates.nand.primitive-contract` *(new, (c), option A only)*. Closes:
`gates.model.code-link` (replaced by the four above). The **count of open (c) ids
does not fall** at G-M2: the model-to-code gap is converted from one coarse id
into the precise ones that K-M2 closes for every package.

### 9.4 G-M3: compositions, and the first "for every size" theorem

**Targets.** In order, each model level first:

1. **Half adder**: `sum = xor`, `carry = and`. Statement: correct on every bit pair
   and rejecting non-bits (the gate rejection composes).
2. **Full adder**: from two half adders and an OR. Statement: for bits `a b cin`,
   `2 * carry + sum = a + b + cin`.
3. **n-bit ripple-carry adder**, correct for **every width n**, by induction:
   for every `n`, every two `n`-bit vectors `xs ys` (LSB first, as `arithmetic`'s
   `ripple_carry_adder` and LG01's convention) and every carry-in `c`,
   `value (sum xs ys c) + 2^n * (carry xs ys c) = value xs + value ys + c`, where
   `value` is the little-endian reading of the bit list.

The third is the first theorem in the repository that no test and no bounded
proof can reach: it is a statement about **all** widths. It is proved with
`Nat.rec` (or `List.rec`) induction; the induction hypothesis is exactly the
statement for the shorter vector.

| Feature | Notes |
|---|---|
| `List` (a parametric recursive inductive) | the bit vector |
| `Prod` and structure projections (`proj`) | `(sum, carry)` results |
| `Eq.rec` / `congrArg` / `Eq.symm`, K-like reduction for `Eq` | rewriting in the induction step |
| `Nat` addition and multiplication lemmas | **Option 1 (recommended):** a small native lemma set (`Pif.Nat`: commutativity, associativity, distribution, defined by structural recursion with no literals); **Option 2:** import Lean's `Init` through the Lean converter, which needs `lits` and K-M0 |
| Literals | **not** needed under option 1 |

**Code level for G-M3 (G-M3b).** A loop in the Rust adder (it iterates over the
bit vectors) is the point where the code-level proof needs the **block-invariant
rule** of VER11a §9.3, so G-M3b is sequenced *after* KRN-012/013a. The model-level
G-M3a does not wait for them.

**Assumption ids.** G-M3a opens `spec.goal.faithful` for the adder specification
(the meaning of `value` and of "every width" is signed, (b)) and
`gates.model.faithful`-style ids for the composition (`gates.compose.faithful`,
*new*, (b)). It closes nothing in (c). G-M3b closes `gates.compose.code-link`
(*new*, (c)) for the adder once its WIR program is covered.

### 9.5 The assumption story, end to end

| After | Gates claims at rung | Open (c) (gates) | Open (b) |
|---|---|---|---|
| Today (Rust lane) | `tested` / `exhaustive` (domain not emitted yet, VER31 §4) | the lane's `wir.*`/`tcb.*` ids | corpus statement |
| G-M1 | model-level theorems `checked`; package claims unchanged | `kernel.checker.correct`, `pif.reader.correct`, `gates.model.code-link` | `spec.goal.faithful`, `gates.model.faithful` |
| G-M2 | gate claims over WIR `checked` | `kernel.checker.correct`, `pif.reader.correct`, the `wir.*` ids, `gates.nand.primitive-contract` | same |
| K-M2 (ct-compare, shared) | `proved` possible | `kernel.checker.correct` per claim (independent agreement) | signed |

---

## 10. The corpus, the oracles, and how the reader is tested

| Corpus | Contents | Used by |
|---|---|---|
| **Positive** | The worked example of 4.6; the G-M1 files; the native prelude; small Lean-converted libraries | reader, kernel, `pif-fmt`, differential |
| **Negative, lexical** | One file per rule L1 to L5 and per record kind: CRLF, tab, double space, trailing space, blank line, comment, leading zero, bad escape, uppercase hex, short digest | reader and grammar parser (both must reject, with the same reason class) |
| **Negative, structural** | Forward reference, self reference, index gap, duplicate content, wrong counts in `params`, `itype`, `inductive`, `end k` too small or too large, unknown feature, undeclared feature use, unknown `prim` role, `prim` before its constant, duplicate declaration, unknown import | reader |
| **Hostile (limits)** | One file per row of section 7: a 10 MB line, a million-deep expression chain written with backward references only, a 100,000-digit `nat`, a 65-type block, a header claiming a huge count | reader (must reject quickly and with bounded memory) |
| **Negative, kernel** | Ill-typed theorems, non-positive inductives, a universe-inconsistent inductive, a bad `prim nat.add`, a `theorem` whose type is not a `Prop`, a recursor supplied as an axiom named `Bool.rec` (rejected as redeclaration after derivation), an axiom `False`, Lean's kernel-rejection tests converted by the Lean converter | kernel, differential vs the official kernel |
| **Mutation** | Every record of the positive files mutated (one field, one record dropped, two swapped); each mutant must be rejected by the reader, the kernel, or the goal comparison | CI |
| **Differential** | (i) reader vs the grammar-driven parser on every corpus file and on grammar-generated and mutated files; (ii) kernel-derived recursors vs Lean's exported ones; (iii) our verdict vs the official Lean kernel through the PIF→Lean harness; (iv) later, vs the second checker (KRN-019) | CI; evidence for `pif.reader.correct` and `kernel.checker.correct` |

Oracles are **differential evidence only, never a dependency of a check**.

---

## 11. Backlog

The same rows are added to [VER-BACKLOG.md](VER-BACKLOG.md) under "Proof
interchange format (VER32) and first proof target (logic gates)". Order is
dependency order; one bounded change per PR; spec before code.

| Priority / ID | Work | Prerequisite and acceptance |
|---|---|---|
| P1 / PIF-000 | This spec, with an adversarial review and the owner's answers to section 12 | spec reviewed; Q1 to Q7 answered |
| P1 / PIF-001 | `code/grammars/pif/pif.tokens` and `pif.grammar` (4.2, 4.3); the line-discipline pre-pass as a documented companion; both parse the worked example and the lexical corpus; ci-full-build wiring | after PIF-000 |
| P1 / PIF-002 | **Trusted reader** in `lean-kernel` (≤ 600 lines, literate, `forbid(unsafe_code)`): header, features, imports, records, counts, limits (7), canonical lexical form; builds `Declaration`s and inductive blocks; budget gate counts it in the 8,500 | after PIF-001 and KRN-002 slice 2 (declarations, axioms, defs, theorems); inductive blocks, `quot`, `prim` after KRN-002 slice 3a, slice 3b and literals |
| P1 / PIF-003 | `pif-check` binary (replaces `lean-check` in KRN-003): reads files, imports by digest, prints accepted declarations, axiom footprint and goal bindings; `pif-fmt` (canonical check C1 to C3, untrusted); `pif-pp` | after PIF-002 |
| P1 / PIF-004 | Native prelude generator: `Bool`, `Nat`, `Int`, `Eq`, `And`, `Or`, `True`, `False`, `Option`, `List`, `Prod`, same names and types as Lean's `Init`; digest allow-listed | after PIF-002 and KRN-002 slice 3a |
| P1 / PIF-005 | PIF corpus: the positive set (the 4.6 example is the first), lexical, structural, hostile and kernel negative files, mutation runner; a CI job | after PIF-003 |
| P1 / PIF-006 | **Case-split producer** (untrusted): emits PIF proofs by exhaustive case analysis over finite inductives and `Eq.refl` leaves; fails closed | after PIF-004, PIF-005 |
| P1 / PIF-007 | Differential harness (i): the reader vs the grammar-driven parser, on all corpus files plus grammar-generated and mutated files (decides Q1's evidence) | after PIF-001, PIF-002 |
| P1 / G-M1 | **Gates model level**: model definitions, T1 to T5 for the eleven gates as PIF produced by PIF-006, goal bindings, report fields, VER31 rung `checked` for model-level claims; ids opened as in 9.2; registered by PIF-016 | after PIF-004, PIF-006, KRN-002 slice 3a |
| P2 / PIF-008 | Lean→PIF converter (untrusted crate; **replaces KRN-003's lean4export reader**): reads the pinned lean4export output, drops recursors, emits `prim`/`quot`/features, skips and lists `unsafe`; round-trips the corpus | after PIF-002; pin file (KRN-003 re-scoped) |
| P1 / PIF-009 | **K-M0 via PIF**: Lean's `Init` converted and checked by the kernel; agrees with the official kernel on every safe declaration; kernel-derived recursors equal the exported ones | after PIF-008, KRN-002 (full), KRN-005 (re-expressed) |
| P2 / PIF-010 | PIF→Lean replay harness (oracle only, a Lean program that reads PIF lines and calls `addDecl`); recorded agreement as evidence; replaces the harness of VER11a §4.3 | after PIF-003 |
| P1 / PIF-016 | VER13 slice: ids `pif.reader.correct`, `gates.model.faithful`, `gates.model.code-link`, `gates.nand.primitive-contract`, `gates.compose.faithful`, `gates.compose.code-link` with bucket entries (owner signs the bucket file, like KRN-011); VER11a §11.3 certificate kind becomes `pif:sha256:…` | with G-M1 |
| P1 / G-M2 | **Gates code level**: straight-line WIR semantics (KRN-007's loop-free subset), the G-M1 theorems and the link theorem over the lowered Rust gates; scope per Q2 | after G-M1, KRN-006a/b/f, KRN-007 (straight-line part) |
| P1 / G-M3a | **Compositions, model level**: half adder, full adder, n-bit ripple-carry adder for every width by induction; native `Pif.Nat` lemma set | after G-M1 |
| P2 / G-M3b | Adder at code level: the loop's block invariant and the link theorem | after G-M2, KRN-012, KRN-013a |
| P2 / PIF-011 | Rocq→PIF converter (separate untrusted crate; profile 6.2); refuses the unsupported list | after PIF-009 |
| P2 / PIF-012 | HOL family converters (OpenTheory first; HOL Light and HOL4 through it; Isabelle/HOL proof terms later); profile 6.3 | after PIF-009 |
| P3 / PIF-013 | Reader option (a): a `grammar-tools`-generated reader behind a flag, compared with (b); adoption decided on size and audit | after PIF-007; Q1 |
| P3 / PIF-014 | Compact binary form (mechanical bijection of the text records; digest taken over the text) | after PIF-009 measures `Init` |
| P2 / PIF-015 | ct-compare on PIF: the existing K-M1 queue (KRN-013a/013) emits PIF through the claim elaborator and the generic producers instead of Lean source; Lean's toolchain optional | after G-M2 |

---

## 12. Owner questions

| # | Question | Recommendation | Alternatives |
|---|---|---|---|
| Q1 | How is the trusted PIF reader built? (carried over from VER11a Q9) | **(b)** a ≤ 600-line hand-written reader, with the grammar-driven parser as an independent implementation and a differential on every corpus file and on generated files; a narrow owner exemption to the "no hand-written parsers" rule for this one trusted reader (section 5.2) | **(a)** generate it from `pif.grammar` with `grammar-tools`, which puts grammar-tools and its generated code into the trusted base and the 8,500-line count |
| Q2 | What does G-M2 target for the seven primitive gates, which today delegate to the floating-point `transistors` model, outside the lowered subset? | **A**: the validation and the four NAND-built gates fully, with `nand_gate` as a call under a stated (c) contract (`gates.nand.primitive-contract`); the six remaining primitives follow when the `transistors` digital path enters the subset | **B** change the gates to compute the bit function after validation (a change to a package 47 crates use); **C** lower the transistor model (needs floating point in the semantics) |
| Q3 | May a *claim's* axiom footprint contain a `Src.<profile>.*` axiom from a converter (for example, Rocq's `classic`)? | **No**, until a VER13 id and bucket are registered for that specific axiom: for claims, only the three Lean axioms and `Warrant.Assume.<id>` are allowed (VER11a §4.5); library import (K-M0 style) may accept and list `Src.*` axioms | Allow any named axiom, listed in the report and classified (c) by default |
| Q4 | What is G-M1's input domain? | **`Int`** (every integer): the widest domain every lane's integers embed in; `u8` is a corollary. Costs one extra inductive (`Int`) and one extra recursor step | `Nat` (no negative inputs; lanes allowing negatives would need a side lemma); `Fin 256` (exactly the Rust type, but then the proof adds nothing over the exhaustive walk) |
| Q5 | Rename the certificate kind `lean-export:` of VER11a §11.3 and the VER31 gate to `pif:` (a VER31 amendment) | **Yes**: the certificate is a PIF digest; the Lean export becomes an optional input of one converter | Keep `lean-export` as an alias for Lean-profile files only |
| Q6 | Where does the native prelude come from for G-M1: our own minimal one (PIF-004), or Lean's `Init` through the Lean converter? | **Our own minimal prelude** for G-M1 and G-M3a (no Lean toolchain in the path, no literals), checked declaration-for-declaration against `Init` by the differential; import `Init` when G-M2's semantics or Rocq/HOL converters need it | Import `Init` from the start (needs `lits` and K-M0 first, and the Lean toolchain in the build path) |
| Q7 | Is `Src.*` naming enough, or should converters also print a *profile report* (the list of unsupported constructs they hit) as part of the evidence? | **Yes**: a converter's refusal list and axiom list are printed in the report | The report lists axioms only |

---

## 13. Relationship to other specifications

| Spec | Relationship |
|---|---|
| VER11a | Logic unchanged (§3.3). §4.3 amended: the kernel's input is PIF. Q9 resolved. §4.2's "Export reader" row now describes the PIF reader. §5.3 and Q2: the Lean toolchain is now one optional producer. §7, §9 to §11 unchanged |
| LEAN00 | L1 is "the kernel checks Lean's core library"; it now does so through the Lean→PIF converter. LEAN00's later layers produce PIF as one more producer |
| VER31 | Buckets and rungs as in section 9; the certificate kind and the new ids (section 11, PIF-016) are VER31 §3.4 / §11 amendments |
| VER13 | Gains the ids of 9.2 to 9.4 |
| LG01, LG02 | The model of 9.2 is their contract; the gates' truth tables are the LG02 corpus |
| VER25, VER25a/b, VER11a §6 | G-M2 uses the straight-line fragment of the Lean WIR semantics |
| VER11a §14 (K-M1) | ct-compare becomes the second adopter on the same pipeline (PIF-015) |

## 14. What this spec does not promise

- That the reader is *correct*. It is small, tested against an independent
  implementation, and counted as trusted (`pif.reader.correct`, (c)). Verifying
  it is VER11a's long-term direction.
- That a converted third-party theorem is *the source prover's theorem*. Only that
  it is a theorem of Lean's logic (section 1.3).
- Support for every Rocq or Isabelle feature. The unsupported lists are part of
  the contract.
- That the gates' code-level proof (G-M2) reaches all eleven gates at once: it
  depends on Q2.
- A compact binary form, an upgrade tool beyond a sketch, or Mathlib.
