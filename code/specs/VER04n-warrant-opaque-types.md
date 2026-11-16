# VER04n — Warrant Opaque Types (WX-O)

**Status:** Specification only. Nothing here is implemented and no code changes
with it. This is the WX-O0 deliverable of the backlog: the normative text that
slices O1 to O4 (section 19) implement. The owner decisions of 2026-11-16 are
recorded in section 1 and applied throughout. The Warrant fragments are
**sketches**: none has been run through the checker, predicate and model names are
illustrative until the O1-S0 errata pass (section 19) fixes them, and every case
count in the text was computed by hand from the stated domains, each re-derived so
a reviewer can redo it. Claims about what a crate does are "from reading": the
source was read, not run. The freeze on opaque types is lifted **spec first, with
adversarial review**: the review of 2026-11-16 is recorded in section 21 (findings
R1 to R24, each closed in this text). O1 does not start until the owner has answered
the questions of section 20.1.

**Parent:** [VER04](VER04-warrant-language.md) (sections 2 to 8),
[VER04h](VER04h-warrant-derived-evidence.md) (domains, budget, manifests, controls),
[VER04i](VER04i-polyglot-warrant-evidence.md) (corpus, lanes, the eighteen-lane
order), [VER04j](VER04j-warrant-v1-extensions.md) (records, bindings, bounds),
[VER04k](VER04k-warrant-v1-wave4.md) (model references, drivers, `inout`, closed
binding vocabularies), [VER04l](VER04l-warrant-text.md) (the `v2` header, whose
rules this document reuses), [VER29](VER29-layer-contract-linking.md) (contract
ids and discharge: foreign handles are cited through it), and the draft **VER04r**
(`warrant-corpus-v2`, on branch `claude/warrant-corpus-v2-spec`, not yet on `main`):
the language-neutral corpus this feature is encoded in (section 9). Every reference
to VER04r is to that draft as read on 2026-11-16; O1-S0 re-reads it.

**Amends (clearly marked, section 17):** [VER29](VER29-layer-contract-linking.md)
sections 3.1, 3.2, 3.3, 3.5, 5.1, 5.2, 5.3, 5.5, 6.2, 6.4 and 13, and
[VER04m](VER04m-warrant-stateful-objects.md) sections 3.5, 4.4, 6.1, 9.1 and 14.
Both documents carry a one-paragraph pointer at their top; the normative text of
the amendments is section 17 here and is applied by the implementation slices
that make it true, as VER29 section 16 does for its own amendments.

**Related:** [VER04m](VER04m-warrant-stateful-objects.md) (stateful objects; opaque
types are its immutable sibling, section 7 here),
[VER08b](VER08b-runtime-capability-conformance.md) (the runtime jail, section 13),
[VER30](VER30-warrant-expansion-roadmap.md) (section 4.2 says the layered
`*-link` and `*-binding` validators wait for a handle that VER29 can cite),
[VER-BACKLOG](VER-BACKLOG.md) (rows needed are listed in section 19.2; this
document does not edit it).

**Design source:** an advisory design memo (not committed), section 1, opaque types
(WX-O). Where this document departs from it, section 1.2 says so and why.

## 0. Why this exists

Warrant documents are pure and speak about values the document can write down. A
large class of real signatures takes a value the document **cannot** write down,
because the library hides how it is made:

| Shape | Real API (from reading) | What a plain `op` cannot say |
|---|---|---|
| a validated token | `x509_rsa_public_key::ValidatedRsaPublicKey<'a>`: private fields, `Copy`, observers `modulus()`, `modulus_bits()`, `public_exponent()`; produced only by `validate_rsa_public_key(spki, policy, limits)` | "for every key the validator can produce, `recover_encoded_message(key, sig, limits)` equals RFC 8017 RSAVP1 over that key's modulus and exponent" |
| a checked configuration | `rsa_public_operation::RsaPublicOperationLimits`: private field, `new(n) -> Result<Self, Error>` rejecting zero, observer `maximum_modulus_bytes()` | "every limit the constructor accepts behaves as its observed ceiling says" |

The VER30 blocker table lists opaque foreign types (cross-crate handles) as a
Tier-3 blocker for 201 crates, and the design memo counts 48 security-flagged crates
among those that opaque types gate (an upper bound: ordering and other blockers are
folded in, and the memo says so). The
layered validators (`*-link`, `*-binding`) take another package's handle and cannot
be adopted until VER29 can express a handle as a cited claim (VER30 section 4.2).
Without this feature the X.509 and RSA chain stops at its second layer for the
same reason the 4004 chain stopped at `logic-gates::sequential` before VER04m.

The design keeps the one principle of VER04k and VER04m: **the document stays
pure and never contains a host notion.** An opaque value is not a pointer, an
object or a reference. It is the **recipe that built it**, and the model sees it
only through a declared, finite **observation interface**:

* a **maker** is a constructor the library exposes, written with a pure model of
  when it succeeds and what the handle then observably is (its `means`);
* an **observer** is a function from a handle to an ordinary Warrant value;
* every claim about an operation that takes a handle is written over the
  observations of that handle, never over its insides.

The evidence checks the subject without seeing inside it by (a) building handles
through the real makers, (b) comparing what the real observers report with what
the document says they must report, (c) comparing what the real operations return
with the model applied to the observations, and (d) re-observing after every call.
Section 6 walks one case through all four.

What is added:

| Added | Where |
|---|---|
| the declaration `opaque T` with `make`, `observe`, `means`, `admits`, `rejects`, and the flag `sealed` | 4 |
| the recipe literal `make m(args)` in `example` rows | 4.4 |
| typing rules: no `==`, no literal, no pattern, no nesting, no result position | 5 |
| foreign handles: `opaque T from contract.<pkg>.opaque.<T>`, restated and pinned | 6 |
| handle domains `D(T)`, observation classes, five obligations with controls | 7 |
| the relation to explicit state and protocols (VER04m) | 8 |
| the `warrant-corpus-v2` encoding: two stream tags, a `recipes` descriptor, the IR level `ir.opaque`, faults as data | 9 |
| a lane binding contract for eighteen lanes, one sealing vocabulary for all | 10 |
| eleven sabotage controls | 11 |
| labels: a handle binder is never `full` | 12 |
| the pilot, `rsa-public-operation` | 14 |

### 0.1 Slices, and how rules are marked

| Prefix | Content | Section |
|---|---|---|
| `OM1`, ... | the semantic model | 3 |
| `OX1`, ... | surface syntax | 4 |
| `OK1`, ... | checker (type) rules | 5 |
| `OF1`, ... | foreign handles | 6 |
| `OV1`, ... | evidence, obligations, domains | 7 |
| `OR1`, ... | relation to VER04m | 8 |
| `OC1`, ... | corpus and encoding | 9 |
| `OL1`, ... | lane binding contract | 10 |
| `OS1`, ... | sabotage controls (faults) | 11 |
| `OB1`, ... | labels and vacuity guards | 12 |
| `OT1`, ... | security considerations | 13 |

Slices: **O1** local opaque types (no `from`), **O2** the lane rollout, **O3**
foreign handles with the VER29 amendment, **O4** the pilot adoption. A rule binds
the slices that implement what it names; an O1 implementation MUST NOT be required
to understand an `OF` rule, and a document that uses `from` before O3 is refused
(W0752). Section 19 lists the slices and their prerequisites.

## 1. Decisions of 2026-11-16, and departures from the memo

### 1.1 Decisions

| # | Decision | Applied in |
|---|---|---|
| a | The freeze on opaque types is lifted: specification first, with adversarial review before any implementation | the status paragraph, 19, 21 |
| b | Foreign opaque handles may be cited only through VER29 **published lower contracts**: no handle without a published lower contract. An upper crate whose lower package has no adopted Warrant document waits (fail closed) | 6, 17.1 |
| c | Honest labels: `full`, `bounded-set`, `sample` (the LP-012 kinds). A handle binder is never `full` | 12 |
| d | Fail closed everywhere; verification is layered (a lower contract is assumed above, never re-proved); **the runtime capability jail stays mandatory** | 6, 12, 13 |
| e | Pilot: `rsa-public-operation` | 14 |
| f | The language-neutral corpus is moving to `warrant-corpus-v2` (draft VER04r); the opaque encoding fits v2, not v1 | 9 |

Decision (b) in plain words: an upper layer can say "this key came from the real
`validate_rsa_public_key`" only if the lower layer published, and the report
discharged, the claim that this maker produces keys whose observations are what the
lower document says. There is no mock, no local redeclaration of another package's
type, and no way to assume a handle's meaning without the lower evidence.

### 1.2 Where this document departs from the memo

Each item is a point where the memo is silent, or inconsistent with a merged
specification, or weaker than the owner's labels. The resolution is the text of the
cited rules.

| # | Memo says | Here | Why |
|---|---|---|---|
| N1 | the model sees `obs(h)`; the generator groups recipes by observation tuple; the source of `obs(h)` is not stated | each maker carries a pure **`means`** model giving the observation tuple from its arguments; `admits` says when it must succeed (OM3, OX3) | the corpus is enumerated in-lane and the expected stream is computed by the model evaluator **without the subject** (VER04i section 2.2). If observations came from the subject, a maker that parsed the wrong modulus and an observer that reported it would agree with a model fed by both |
| N2 | `opaque T from contract.<pkg>.opaque.T` "imports another package's type with its makers and observers" | the upper document **restates** the full interface after `from`; the resolver compares the restatement to the lower document and pins its digest (OF1, OF3) | VER04 section 2: "imports ... do not exist"; VER29 section 3.3: a contract entry "adds no name to the symbol namespace". A restated, pinned, exactly-equal interface keeps the upper document self-contained for generation and keeps VER29 rule (c) (exact linking) |
| N3 | obligations O1 adequacy, O2 immutability, O3 unforgeability | five obligations, renamed (`adequacy`, `immutability`, `unforgeability`, `freshness`, `grounding`); adequacy is **implied by the claim shape** plus a multiplicity guard, and is `n/a (injective)` when the means are injective (OV3 to OV8) | with `means` the equal-observation classes are known to the generator, so adequacy is a coverage question, not a separate test; this also avoids the memo's `O1`/`O2` clashing with the slice names |
| N4 | `eq=observational`, opt-in (memo O-1) | **not in O1**. A claim over an explicit `op key_eq(a: K, b: K) -> Bool` states the same thing with no new feature (owner question OWN-1) | a language feature whose meaning is "equal observations" adds nothing the observers do not already say, and `==` on a handle stays unconditionally an error, after the `len(t)` precedent (VER04l decision b) |
| N5 | `D(T)` is the makers applied to their argument domains, "kept when the result is `Ok`" | kept when `admits` holds, a **model-side** filter; the label counts `admitted of tried` (OV1) | the subject's own verdict cannot choose the domain it is then tested on |
| N6 | no rule for secret material | a maker parameter or observer result classed `secret` is refused (W0750, OT2) | recipes are written into a committed corpus (OC1) |
| N7 | "lanes without enforced privacy print not established" | one four-valued sealing vocabulary and a per-lane in-lane probe for all eighteen lanes (OL2) | Go's zero value, C# `default`, Kotlin `copy` and Python's `object.__new__` are forging paths in "private" lanes; the status must come from a probe, not from the language's reputation |
| N8 | `make` and `observe` rows "are ordinary `op` signatures" | they are **interface declarations**, not callable operations: a claim cannot call a maker, and an observer applied to a handle binder is the model function its `means` induce (OM4) | it removes the provenance question "is this observer call a model or a subject call?"; the subject's observers are tested by the derived grounding claim (OV6) |
| N9 | tags 80 and 81, in the v1 stream | tags `80` and `81` of `warrant-stream-v2`, checked free against VER04i, VER04l and the drafts VER04r and VER04p (OC2); the whole encoding is restated for corpus v2 (decision f) | 9 |
| N10 | one handle per case, observed and then used | each handle parameter is built **twice**: a witness that is observed and never used, and a subject that is used and only then observed (OV5) | observing before the call would hide an operation that misbehaves on a handle no observer has touched (a lazily computed field) |
| N11 | (silent) the maker is one host constructor | a maker is a **plan**: up to eight host calls, the last in the declaring crate, earlier ones in it or in a direct dependency (OL1) | the pilot's real maker takes host values of two further packages (an SPKI view and ASN.1 limits); without a plan the lower package could not bind its own maker |

Alternatives considered and rejected: **handles as transparent records through the
VER04m codec** (rejected: the host type has private fields and may have no total
builder; a codec requires a canonical bidirectional mapping, section 8); **a local
redeclaration of a foreign type with a mock maker** (rejected by decision b);
**evidence over recipes alone, with no observers** (rejected: the claim would say
nothing a caller can rely on).

## 2. Versioning and compatibility

**OC0.** (Header.) Opaque types are a **v2** feature (VER04l section 2.1): a document
is v2 exactly when its header is `warrant name v2`, and v2 means everything in v1
plus the elements of every specification that names v2. An opaque construct in a
v0 or v1 document is W0101 ("v2 construct in a v1 document"), never ignored. O1-S0
depends on T1-S0 having made the checker accept `v2` (VER04l section 12); if it has
not, O1-S0 makes that change itself.

**OC0a.** (Reserved words.) `opaque`, `make`, `observe` and `sealed` become reserved
in v2. `admits`, `means`, `rejects` and `from` are contextual: recognised only at
the positions of section 4, like the contextual names of VER04j section 1.2.
Because v2 documents are new, no existing document changes meaning; the
byte-identity gate of VER04l section 2.1 covers v0 and v1 unchanged.

**OC0b.** `CheckedWarrantModule` gains an `opaque` table (types, makers, observers)
that is empty for every document without the feature. Every new prelude part (the
recipe decoder, the observer harness, the grounding suites) is emitted only for a
document that declares an opaque type, as the driver parts of VER04j section 14 are.

## 3. The semantic model

### 3.1 A handle is a recipe

**OM1.** A value of an opaque type `T` is a **recipe**: a maker name and Warrant
values for its parameters. A binder `h: T` ranges over the recipes the declaration
admits (OV1). Nothing else is a value of `T`: there is no literal, no way to write
one in an operation, no constructor expression, no field, no pattern, no `==`.

Analogy: a recipe is a menu entry ("order 3, no onions"), not the dish. The kitchen
(the library) makes the dish; the document describes only what the diner can taste.
Two different orders can taste the same. That is exactly the case adequacy covers.

### 3.2 The observation interface

**OM2.** Each opaque type declares at least one and at most eight **observers**,
unary functions `T -> R` with `R` a Warrant type containing no opaque type. The
**observation tuple** `obs(h)` is the tuple of all observers, in declaration order.
It is a Warrant value and compares with ordinary structural equality.

### 3.3 Makers and `means`

**OM3.** Each opaque type declares at least one and at most eight **makers**. A
maker has Warrant parameters (no opaque types, no secret-class parameters, OK6), a
result of the form `T`, `Option<T>` or `Result<T, E>` (`E` opaque-free), and three
clauses, all pure first-order models of its parameters (VER04k section 2.3 sense;
no operations, no handles, no observers, no quantifiers; metered as VER04k section
2.7):

* `admits P`: **a sufficient condition** for success. For every argument tuple
  satisfying `P`, the maker MUST return `T` (or `Ok`/`Some`) and the observers of
  the result MUST equal the `means` values. Where `P` is false the document says
  nothing, unless a `rejects` clause covers the arguments.
* `means o = e`, one per observer `o`: the value `o` returns for the built handle,
  as a model of the maker's arguments.
* `rejects Q with v` (repeatable, `Result` and `Option` makers only): for every
  argument tuple satisfying `Q` the maker MUST return `Err(v)`. For an `Option`
  maker the row is `rejects Q` with no `with` part and the maker MUST return `None`.
  `admits` and `rejects` conditions MUST be disjoint on the declared domain; the
  generator checks this by evaluation and reports an overlap as W0750.

`admits` is mandatory (write `admits true` for a total maker): a type with no
admitted recipe has an empty domain, which is W0601, so the omission would otherwise
be a silent vacuity (OB4).

**OM4.** (Observers in claims.) In a `claim`, `property` or `example`, an observer
applied to a handle binder `h` denotes the **model function induced by the means**:
`o(h)` is `means_o` evaluated at h's recipe arguments. It is never a subject call.
The subject's observers are exercised by the compiler-derived **grounding claim**
of each maker (OV6) and by the re-observation of every case (OV5). This is the
semantic point that keeps the expected values independent of the subject (N1).

### 3.4 What the evidence does and does not establish

| Establishes | Does not establish |
|---|---|
| for every recipe in the stated domain, the real maker builds a handle whose real observers report the `means` | anything about a handle the makers could not build, or a handle forged by other means (that is `unforgeability`, a per-lane printed status, OV7) |
| for every recipe in the domain, the operation's result equals the model applied to the `means`, for every other binder value in its domain | that the result is the same for **all** handles with equal observations, unless the domain has two or more members in an observation class (adequacy, OV3) |
| the observers do not change across the call (re-observation) | that the call has no hidden effect on state the observers cannot see (an unobservable cache is allowed, and invisible) |
| a handle is built fresh for each case (freshness) | timing, resources, or anything outside the binding |
| the makers behave as `admits`, `rejects` and `means` say, on the stated argument domain | a **type invariant** ("every value of `T` satisfies `I`"). `admits` is sufficient, not necessary: a maker may also succeed outside it, and such a value is unconstrained. No layer may import an invariant from an opaque contract (OF8) |
| the operation behaves as the model says on a handle that **no observer has touched**, each handle parameter being its own build (OV5, OV8) | behaviour when one host value is passed for two parameters (aliasing), and anything that depends on a handle's identity |

### 3.5 What a maker failure means

A maker that returns an error for an argument tuple outside `admits` is not wrong:
the document did not forbid it. A maker that returns an error for an admitted tuple
fails the grounding claim with the failure `maker-rejected-admitted`. To pin a
rejection, add a `rejects` clause. The report prints, per maker, `rejection:
complete` only when every tuple of the maker's argument domain satisfies `admits` or
some `rejects` row (decided by evaluation, never by the presence of a clause), and
otherwise `rejection: partial (k of n tuples unspecified)` (OB6).

## 4. Surface syntax

### 4.1 Grammar

```text
declaration  += opaque_decl
opaque_decl   = "opaque" NAME [ "sealed" ] [ "from" contract_id ] NEWLINE
                INDENT { maker_decl | observer_decl } DEDENT ;
maker_decl    = "make" NAME "(" [ params ] ")" "->" maker_result NEWLINE
                INDENT admits_row { rejects_row } means_row { means_row } DEDENT ;
admits_row    = "admits" expr NEWLINE ;
rejects_row   = "rejects" expr [ "with" expr ] NEWLINE ;   -- "with" iff a Result maker
means_row     = "means" NAME "=" expr NEWLINE ;
observer_decl = "observe" NAME "(" NAME ":" NAME ")" "->" type NEWLINE ;
maker_result  = NAME | "Option" "<" NAME ">" | "Result" "<" NAME "," type ">" ;
type         += NAME            -- when NAME is a declared opaque type
expr         += "make" NAME "(" [ args ] ")"     -- recipe literal, section 4.4
```

`NAME` after `opaque` is the type name. In `maker_result` the `NAME` MUST be the
declared type. `contract_id` after `from` is the second position where VER29
section 3.3's contextual `contract_id` is recognised (amendment 17.1). A
`means_row` is required once per observer (W0750 if one is missing or duplicated),
and the `means` expression's type MUST equal the observer's result type.

### 4.2 A complete local example (illustrative)

A small immutable set of bytes, built from a list that may contain duplicates and
any order. It is the running example for the model and for adequacy; it is not a
claim about any crate.

```text
warrant byte_set v2

opaque ByteSet sealed
  make from_list(xs: List<U8>) -> ByteSet
    admits true
    means  elements = m_norm(xs)
    means  size     = len(m_norm(xs))
  observe elements(s: ByteSet) -> List<U8>
  observe size(s: ByteSet) -> Nat

op set_contains(s: ByteSet, x: U8) -> Bool

claim contains_agrees:
  forall s: ByteSet, x: U8. set_contains(s, x) == m_has(elements(s), x)

example dup_collapses: set_contains(make from_list([1u8, 0u8, 1u8]), 0u8) == true
```

`m_norm` (sort, remove duplicates) and `m_has` are ordinary `model` functions
written with the WX-D forms of VER04k section 2; `elements(s)` in the claim is the
means-induced model function of OM4, so the right-hand side is
`m_has(m_norm(xs), x)` for the recipe `from_list(xs)`.

### 4.3 A maker that can fail

```text
opaque Limits sealed
  make limits_new(max_modulus_bytes: Nat) -> Result<Limits, LimitsError>
    admits  max_modulus_bytes >= 1
    rejects max_modulus_bytes == 0 with ZeroMaximumModulusBytes
    means   maximum_modulus_bytes = max_modulus_bytes
  observe maximum_modulus_bytes(l: Limits) -> Nat
```

Here `admits` and `rejects` partition `Nat` exactly. The derived grounding claim
(OV6) is `limits_new` versus these rows over the whole maker domain.

### 4.4 Recipe literals

**OX1.** `make m(args)` in expression position is a **recipe literal**: it denotes
the recipe of maker `m` applied to the argument values, with type `T`. It is allowed
only as an argument of an operation call, and only in two places where every
argument is ground when the row is evaluated: an `example` row, and the `holds`
expression of a `vectors` table (VER04j section 5), where the arguments may be the
table's columns. `admits` MUST hold for the arguments, checked by evaluation for the
example and for **every row** of the table (else W0750, and no row is used: a table
row that the document does not admit is not evidence about the document's type). A
recipe literal is not a value: it cannot be compared, projected or bound, and it
cannot appear in a `claim`, `property` or `fuzz` row, whose handles come only from
`D(T)` (OV1). The reserved word `make` is unambiguous in expression position because
it is reserved (OC0a). A table row with a recipe literal is an **example** for every
label and rung (OB1): a known-answer table never widens a handle domain.

### 4.5 Restrictions

At most eight makers, eight observers and eight opaque types per document; at most
four opaque-typed parameters per `op`; a maker has at most eight parameters. An
opaque type with no observer (a pure capability token) is W0750: nothing could be
said about it. Type
parameters on an opaque type are W0750 until VER04o (generics) amends this.

## 5. Checker rules

**OK1.** (Nominal.) An opaque type is equal only to itself. Two declarations of one
name, an opaque name that collides with another type, record or variant, or an
opaque name used before its declaration is a name-resolution error (the usual
diagnostics).

**OK2.** (Forbidden uses, W0751.) A value of opaque type MUST NOT appear: as an
operand of `==`, `!=` or any ordering; in a pattern; as a field of a record or
variant; as an element of `List`, `Set`, `Map`, a tuple or `Option` (nesting is out
of scope, section 18); as a parameter or result of a `model`; as a quantifier
binder anywhere but a `forall` of a `claim`, `property` or `fuzz` row; as an
`inout` parameter (OR1); in the result of an `op`.

**OK3.** (Results.) An operation that returns a handle is not an operation: the only
position where a type containing `T` is a result is a `maker_result`. This is why
the memo's "makers are ops" is not adopted (N8). An op that the library really
exposes with a handle result (a *transformer*, `T -> T`) is out of scope for O1
(section 18) and is refused, with the diagnostic's note naming the maker it could be
described as.

**OK4.** (Observer arity.) An observer's single parameter has the declared type and
its result contains no opaque type (this makes the observation tuple an ordinary
Warrant value). Observers may not be called with a non-binder argument: the argument
of an observer is a handle-typed binder (OM4). Because an observer in a claim is a
model function, it is **not** an operation call: a claim that mentions only
observers ("`size(s) == len(elements(s))`") calls no operation and is refused by the
existing rule that every claim calls at least one (VER04k, erratum E13). The fact it
would state is a property of the document's own `means`, not of the subject.

**OK5.** (Maker clauses.) `admits`, `rejects` and `means` are pure first-order models
of the maker's parameters (OM3). They MAY call `model` functions of the document;
they MUST NOT call an operation, observer or maker, and MUST NOT contain a
quantifier. A `means` whose type differs from its observer's result type is W0750.

**OK6.** (Secrets.) A maker parameter, an observer result, or a `means` expression
whose type or value is classed `secret` by a `labels` row, or whose parameter name
appears under a `labels ... secret` row of any operation that consumes `T`, is
refused (W0750). Rationale: OT2.

**OK7.** (Binders.) A binder `h: T` has the handle domain of OV1. It MAY carry a
refinement `from_maker(h, m)` selecting one maker's recipes, and a refinement over
the observers (`size(h) <= 2`) evaluated by the model; any other refinement on a
handle binder is W0751.

**OK8.** (Sealed.) `sealed` is a declaration that the library intends the maker rows
to be the only public way to obtain a `T`. It changes no typing rule; it obliges the
unforgeability probe of OV7 and prints per lane (OL2).

**OK9.** (From.) A `from` clause obliges the full interface to be restated and is
validated by OF1 to OF8. The `from` clause is the document's contract link; it is
**not** also listed under `assume platform` (a duplicate is `ContractDuplicate`,
VER29 section 5.2).

**OK10.** (Operations over handles.) An `op` with at least one opaque-typed
parameter MUST NOT be `fresh`-classed or carry `inout` on a handle. All other op
forms (WX-A to WX-H, `labels`, `bound`, `output`) apply unchanged to the
non-handle parameters.

**OK11.** (One mechanism per host type.) A host type is described once: a type
declared `opaque` MUST NOT also be the object of a VER04m driver (Mode P) or the
record of a `codec` line (Mode E) in the same document, and the binding loader
refuses two declarations that name one host type (W0753). Section 8 says which of
the three applies.

## 6. Foreign handles (slice O3)

### 6.1 Why a handle crosses a package boundary, and what may be said about it

`rsa-public-operation::recover_encoded_message` takes a
`x509_rsa_public_key::ValidatedRsaPublicKey`. The type belongs to the lower package.
The upper document cannot construct one; it can only be handed one by the lower
maker. What the upper layer may assume about such a key (its modulus is normalised,
its exponent allowed by policy) is a **claim the lower package's evidence supports
or does not**. That is exactly the VER29 shape: a published lower contract, assumed
above, never re-proved above (VER29 section 1).

### 6.2 The `from` clause

```text
opaque ValidatedRsaPublicKey sealed from contract.x509-rsa-public-key.opaque.ValidatedRsaPublicKey
  make key_from_parts(modulus: Bytes, exponent: U64) -> Result<ValidatedRsaPublicKey, RsaPublicKeyError>
    admits  m_key_ok(modulus, exponent)
    means   modulus = modulus
    means   public_exponent = exponent
    means   modulus_bits = m_bit_length(modulus)
  observe modulus(k: ValidatedRsaPublicKey) -> Bytes
  observe public_exponent(k: ValidatedRsaPublicKey) -> U64
  observe modulus_bits(k: ValidatedRsaPublicKey) -> Nat
```

(A sketch: the lower document's real maker rows are the lower adopter's work and may
differ; the upper restatement must equal them. From reading, the real constructor is
`validate_rsa_public_key(spki, policy, limits)`, whose arguments are host values of
further packages, so the lower maker is a **plan** (OL1) and its policy bounds are
either maker parameters or literals of that plan. A policy with a small minimum
modulus size admits toy keys, which section 14.3 relies on.)

**OF1.** (Restate and match.) A `from` declaration restates the entire interface of
the lower type: every maker with its parameters, result, `admits`, `rejects` and
`means`, every observer, the `sealed` flag, and the transitive closure of the models
those clauses call. The restatement MUST equal the lower declaration in its
**canonical form**: comments and layout removed; makers, observers and `means` rows
in the lower declaration's order; maker, observer, parameter and error-case names
and every type **significant**; only the names of called models and of variables
bound inside a model replaced by their position (alpha-equivalence). Declaration
order and names are significant because the recipe's maker index (OC1), the
observation tuple's order (OM2) and the binding's `param.<name>` (OL1) all depend on
them: an interface that differed only in order would hash the same recipes to
different handles. Inequality is `ContractInterfaceMismatch` (a VER29 report error,
section 17.1), raised by the resolver, never by the checker: the checker validates
shape only (VER29 section 3.3). `sealed` MUST match.

**OF2.** (Existence and publication.) A contract id `contract.<package>.opaque.<Type>`
exists only if the lower document declares `opaque <Type>` **without** `from`
(VER29 section 3.2 as amended). A `from` declaration is never itself published: no
re-export, so an interface has one owner.

**OF3.** (Pinning.) The report records, in the link's pins (VER29 section 5.3 as
amended) and beside the lower manifest and report digests, two SHA-256 values: the
`opaque_interface_digest` of the canonical interface text of OF1, and the
`opaque_binding_digest` of the lower package's `opaque` binding entry for the type
(its maker plans, observer rows, `release`, `backing`; OL1). A change to either
after the pin is a `ContractPinMismatch` at `--check`, as for any pin.

**OF4.** (Discharge.) The id is discharged when the lower report shows, at the pinned
digests and under the five conditions of VER29 section 5.1 (as amended), **all** of:
every maker's derived grounding claim passed and is not example-only; every suite in
the lower package that takes the type passed its `immutability` and `freshness`
obligations; the lower controls that apply to the type's makers and observers
(`observer-drift`, `swap-observer`, `admit-flip`, `admit-rejected`, section 11) each
have verdict `pass` (VER29 section 5.5 item 3: otherwise open with
`lower-fault-mode`); and the lower lane's sealing status for the type is not
`forgeable`. Until it is, the upper claims that quantify over the type are open with
the lower reason; the report never prints a strength for them that the open link
would not support.

**OF5.** (Building the handle above: the lower plan, never an upper one.) Upper
evidence builds handles by running the **lower package's own maker plans and
observer rows**, read from the lower package's binding at the pinned
`opaque_binding_digest`. The upper binding for a `from` type is the single line
`opaque <T> from` and MUST NOT carry `type`, `make`, `observe`, `release` or
`backing` rows (W0753). This is what stops an upper layer assuming more than the
lower layer publishes: the lower grounding evidence is about the lower plan, and an
upper-written plan (another constructor, an unchecked or test-only one, other policy
constants) would yield handles that evidence never covered. There is no mock. The
lower package MUST be a direct dependency in that lane (VER29
`ContractNotDependency`). A lane that lacks the lower package records the operation
`not-live`, never another lane's evidence (VER29 section 3.4).

**OF6.** (No handle without a published lower contract.) If the lower package has no
adopted Warrant document, or its opaque id does not resolve, the upper document's
`from` clause is `ContractUnknownClaim`: an error, not an open assumption. The upper
crate waits (decision b). A local redeclaration of another package's type without
`from` is refused by the lane binding (W0753). The test is the type's **defining
crate**, taken from the lane's export discovery (VER04 section 6), not the text of
the path: a re-export (`pub use lower::T`) inside the upper crate does not make the
type local.

**OF7.** (Lower fault.) In every upper evidence run the adapter checks, for each
handle parameter built from a foreign maker, that the real observers of the witness
build report the restated `means` (OV5). A mismatch is the failure
`lower-contract-violated: <T>.<observer>` and is attributed to the lower layer: the
upper suite fails, and the report names the lower contract as the cause. The control
`lower-fault` (OS8) proves the attribution works. Because of this step the upper
run does not *assume* `means` for its own recipes: it re-checks them. What it takes
from the lower report is the grounding over the lower package's own domain, the
controls, and the sealing status.

**OF8.** (No imported invariant.) An opaque contract publishes exactly this: on the
stated argument domain the makers accept what `admits` says, reject what `rejects`
says, and yield handles whose observers report `means`. It does not publish "every
`T` satisfies `I`" (section 3.4), so an upper claim MUST NOT rest on such an
invariant, and the checker gives it no way to: an upper handle binder ranges over
admitted recipes only, and a fact about them is a fact about the restated `admits`
and `means` models, evaluated, not assumed. Where a maker's `rejection` is `complete`
and the lane's sealing status is `enforced`, the two printed lines together are
evidence, on the domain and for that lane, that no other value of `T` arises; they
remain printed annotations and never a rung (OB2, OT4).

**OF9.** (Lanes.) VER29's dependency reader exists for the Rust lane only. In every
other lane a suite that takes a `from` type is `not-live` with the reason
`lane-not-supported` (draft VER04r section 12) until that lane has a reader. It is
never discharged by inference from the Rust lane.

## 7. Evidence: domains, obligations, and how the subject is checked unseen

### 7.1 Handle domains

**OV1.** (Domain.) `D(T)`, the handle domain, is a **finite, explicit list of
recipes** chosen by the generator and committed in the corpus (OC4). It is built in
this order:

1. for each maker in declaration order, enumerate its argument tuples over the
   ordinary parameter domains (VER04h section 3) in the canonical product order,
   stopping at `MAX_RECIPE_SCAN = 1,048,576` tuples per maker;
2. keep the tuples for which `admits` evaluates true (a **model-side** filter, N5);
   call their number `A`;
3. if `A <= MAX_RECIPES_PER_MAKER = 256`, keep them all; otherwise keep the 256 at
   admitted ordinals `floor(i * (A - 1) / 255)` for `i = 0..=255` (evenly spread,
   first and last included, so the choice is not the first corner of the product);
4. concatenate the makers' lists in declaration order (at most 8 x 256 = 2,048
   recipes per type);
5. apply the claim's handle refinements (OK7) as an ordinary admission predicate.

A lane never repeats steps 1 to 4: it reads the list (the rule of draft VER04r
section 5.1 that a consumer never re-runs demotion). The kind (OB1) is
**`bounded-set`** when step 1 covered the maker's whole argument domain and step 3
kept every admitted tuple, and **`sample`** when the scan bound or the cap cut the
list. It is never `full`. The label prints the makers and the counts:

```text
bounded-set  handles(via limits_new: 64 of 64 admitted, 65 scanned)
sample       handles(via key_from_parts: 256 of 3,104 admitted, 130,819 scanned)
```

(The second line shows the form only; its numbers are invented.) For `Nat` the
VER04h domain is `0..=64`, 65 values; `admits max >= 1` keeps 64. The same domain
gives `rejects` its one tuple (`0`), which the grounding claim checks.

**OV2.** (Budget.) The product of binder domains, with `|D(T)|` for each handle
binder, is charged to the budget in force (`B = 400,000` in VER04h section 3; the
`budget` of `corpus.profile` under draft VER04r section 5.3) and to the 64 MiB limit
**before** enumeration; an over-budget claim is W0706. A handle contributes its
recipe's domain bytes (VER04j section 4.7). The scan of step 1 is charged to the
generator's meter, and a recipe list is at most 1 MiB of canonical JSON (W0706).

### 7.2 Observation classes and adequacy

**OV3.** (Adequacy.) The **observation class** of a recipe is its `means` tuple. For
the ByteSet example over lists of `U8` drawn from `{0, 1}` of length at most 2, the
domain is 7 recipes and 4 classes:

| class (elements) | recipes |
|---|---|
| `[]` | `[]` |
| `[0]` | `[0]`, `[0, 0]` |
| `[1]` | `[1]`, `[1, 1]` |
| `[0, 1]` | `[0, 1]`, `[1, 0]` |

7 recipes, 4 classes, 3 classes with two members. Adequacy, "behaviour depends only
on the observations", is **evidenced by the claim shape**: if an operation's claim
is `f(h, ...) == m(obs(h), ...)` over every recipe, then equal-observation recipes
give equal results, because both equal `m` of the same tuple. What can still go
wrong is vacuity: a domain in which no two recipes share a class cannot show it. So
adequacy is decided **per claim, over that claim's refined handle domain** (a
refinement can remove every multi-member class a type has), the manifest records
`recipes`, `classes` and `classes_with_2plus_members` per claim and handle binder,
and the claim's adequacy line is exactly one of:

* `adequate (c of n classes have two or more recipes)` when `c >= 1`;
* `n/a (injective)` when the type has **exactly one maker** and every parameter of
  it is the whole of some observer's `means` (the bare parameter, as `limits_new`'s
  is). Then the recipe **is** its observation tuple, for every argument and not
  only on the domain, and there is nothing for adequacy to add. The test is
  syntactic on purpose: a `means` that merely happens to be one-to-one on the
  enumerated domain (`x mod 1000` over `0..=64`), or two makers that can produce
  one observation tuple, is not injective;
* `untested` otherwise (OB3).

**OV4.** (Distinguishing pairs.) For each observer `o` that the claim's model does
**not** mention, the generator checks that some pair of recipes in the claim's
domain agrees on every observer the model does mention and differs on `o`. If none
exists the claim prints `unseen-observer: o`: a subject that secretly depended on
`o` could not have been caught. When `o` is a function of the mentioned observers on
that domain (for the ByteSet claim above `size` is not mentioned, and every pair
that agrees on `elements` agrees on `size`) the line reads `unseen-observer: size
(determined by the mentioned observers on this domain)`, which is the harmless case.
A document cannot remove the line with a claim over observers alone: that claim
would call no operation (OK4).

### 7.3 The case, walked through

One case of `contains_agrees` (recipe `from_list([1, 0, 1])`, `x = 0`), and what the
lane does:

```text
1. read the recipe         maker=from_list, xs=[1,0,1]             (stream tag 80)
2. the generator already computed, with no subject:
     means elements = m_norm([1,0,1]) = [0,1]   size = 2
     expected result = m_has([0,1], 0) = true
3. WITNESS build: a fresh handle w through the real maker
4. witness check: real elements(w) == [0,1]; real size(w) == 2      (grounding, per case)
   then w is released; it is never passed to an operation
5. SUBJECT build: a second fresh handle h, on which no observer is called
6. the lane calls the real set_contains(h, 0) and compares to true
7. re-observation: real elements(h) == [0,1]; real size(h) == 2     (immutability)
8. the lane releases h inside the armed phase                       (OT7)
```

The subject was never inspected: only called, through its public maker, observers and
operation. A failure at 4 is a maker or observer bug (or, for a foreign type, a lower
contract violation, OF7); at 6 an operation bug; at 7, given that 4 passed for the
same recipe, a mutation by the call.

Why two builds (N10). If the lane observed `h` before calling the operation, the
operation would only ever see a handle whose observers had already run. A type that
computes a field on first observation would then pass even if the operation read
that field before any observer had filled it. The witness keeps the per-case check
that the maker and observers agree with `means`; the subject build is used exactly as
a caller that never observes would use it.

### 7.4 The five obligations

Each obligation has a control (section 11) and appears in the manifest with its
own count line. An obligation that does not apply prints `n/a` and is never counted
as passed (OB5).

**OV5.** (Witness check and re-observation: `immutability`.) Every case performs
steps 3 to 8 above **for each handle parameter**: a witness build, observed and
released, and a subject build, passed to the operation unobserved and observed after
the call. Observer results are compared by value after being copied out of the host
(OL7). An operation whose host signature **consumes** the handle (a by-value
parameter of a type that cannot be copied or cloned, a C function that frees its
argument) leaves nothing to re-observe: the binding declares `pass=consume`, the
obligation prints `immutability: n/a (consumed)` for that operation, and the adapter
does not release the handle a second time. A type with a mutating method is not
opaque: it is a VER04m Mode P object (OR1).

**OV6.** (`grounding`.) For each maker the compiler derives, with no author text, the
claim

```text
forall <maker params> | admits(...).    make_obs(maker, params) == Ok(means(params))
forall <maker params> | rejects_i(...). make_obs(maker, params) == Err(v_i)
```

over the maker's own argument domain (not over handles), where `make_obs` is the
subject call that builds a fresh handle through the maker plan, calls every observer
once in declaration order, releases the handle, and yields the observation tuple or
the normalised rejection (OC5). Its kind follows its **binders**: a total maker over
a complete type, such as `Bool`, can be `full` and `exhaustive` under the H3 rule; a
maker over `Nat` (`0..=64`) is `bounded-set`. It is published as part of the type's
contract (VER29 section 17.1). Without it an observer-relative claim would be
circular (N1); a claim over `T` whose type has no passed grounding suite is **not
publishable** and prints `ungrounded: <observer>`.

**OV7.** (`unforgeability`.) For a `sealed` type, independent export discovery (VER04
section 6) plus the lane's in-lane probe (OL2) yields exactly one of four statuses,
printed per lane and never merged across lanes:

| Status | Meaning |
|---|---|
| `enforced` | the language or linker makes a public path to `T` other than the makers impossible, and the probe (a compile-fail or link-fail test, or a header inspection) confirmed it |
| `rejected-at-use` | a forged value can be made, but every operation taking `T` rejects it **with one of its declared errors** (a brand or validity check), and the probe confirmed it |
| `forgeable` | the probe made a value that an operation accepted, or that made an operation crash, panic or raise anything but a declared error (`forgeable (crash)`): an undeclared failure on a forged value is not a check |
| `not-established` | the lane has no probe for this binding |

`forgeable` fails the `unforgeability` obligation (the lane's claim row for `sealed`
types is `fail`); `not-established` is printed and does not fail. No status changes
a rung (OB2): sealing is a property of the host, printed, never proved here.

**OV8.** (`freshness`.) Every build is fresh: one witness and one subject build per
handle parameter per case, each released inside the armed phase, none reused across
cases or across parameters. Two parameters of one type that receive **equal
recipes** still receive two separate builds, so an operation is never handed one
host value twice (section 3.4 states that aliasing is therefore untested). A handle
with shared backing (a borrowed buffer, a cached instance) is rebuilt from the
recipe. Freshness is a rule about the runner, so it is checked in two ways: the
manifest's `handle_builds` MUST equal the count the corpus predicts (two per handle
parameter per case, one per grounding case), as `calls` must equal `expected_calls`;
and the `stale-handle` fault (OS2), which hands the operation another case's
recipe, MUST be caught.

### 7.5 Property and fuzz suites

A `property` or `fuzz` row over a handle binder generates **recipes** only, by the
maker parameter generators (VER05, VER06), filtered by `admits` and run through the
same adapter. They are labelled `randomized` (VER29 section 6.1), count as `sample`
cases, and never exceed the label of the deterministic suites.

## 8. Relation to stateful objects (VER04m)

| Question about the type | Answer | Specification |
|---|---|---|
| Is the whole state a Warrant record the harness can build and read, field by field? | transparent state, `inout` parameter | VER04m Mode E |
| Is the state hidden **and mutable**, driven by method calls? | trace function over a `Call` alphabet | VER04m Mode P |
| Is the value hidden and **immutable**, determined by how it was made? | recipe plus observers | this document |

**OR1.** (Immutability boundary.) A type with a method that changes what an observer
reports is not opaque. The `immutability` obligation fails it, and the author
moves it to Mode P. An opaque type MUST NOT be an `inout` parameter (W0751).

**OR2.** (Shared adapter code.) The `observe` host form of this document is the same
code as the `observe` of the VER04m `codec` line (E11) and the `abs` observer of a
Mode P driver (P4): a method with a `ref` receiver, read field by field, never by
`memcmp` or `Debug` text. A codec is a maker plus an observer over a **transparent**
record; a type with private fields and no total builder is opaque instead.

**OR3.** (Handles in Mode P.) A Mode P object whose constructor takes a handle
(`Framer::new(limits)`) is the intended composition: constructor arguments of a
`dispatch` driver MAY be handle-typed (recipe handles, built fresh per drive), and
`Call` payloads MUST NOT be (calls are data; amendment 17.2). Mode E state records
MUST NOT contain a handle field in O1 (section 18).

**OR4.** (Cost.) A handle in a Mode P constructor multiplies the trace domain by
`|D(T)|`; the budget `B` charges it before enumeration.

**OR5.** (No duplicate mechanism, no gap.) The three rows above are exclusive and a
host type takes exactly one (OK11). The cases that could be argued either way are
settled here:

| Host type | Mechanism | Why |
|---|---|---|
| immutable, all fields readable and a total builder from a record exists | VER04m `codec` (Mode E) | the value is transparent; a recipe would hide what the document can write down |
| immutable, private fields or a fallible constructor | opaque | no canonical two-way mapping exists |
| an object with a constructor and observers but no mutating call, **used as a parameter** of free operations | opaque, not a Mode P driver with an empty `Call` alphabet | Mode P drives one object through calls; it cannot pass that object to another operation |
| a builder whose methods return a new value (`with_x(self) -> Self`) | one maker per reachable shape, as a plan (OL1) | a transformer is out of scope (section 18); a plan of up to eight steps states "built, then extended" as a recipe |
| immutable, but with no observer at all (a capability token) | neither; refused (section 4.5) | nothing could be compared |
| hidden state that changes | VER04m Mode P | OR1 |

The one gap that remains is deliberate and listed in section 18: an operation that
both takes and returns a handle.

## 9. Corpus and encoding (`warrant-corpus-v2`)

Owner decision (f): the encoding targets the `warrant-corpus-v2` profile of the
draft VER04r, not the v1 corpus of VER04i. Opaque types are never emitted into a v1
corpus: a v1 reader rejects the profile (VER04r section 1.1), and a suite with a
handle binder is outside the v1 type set, so it uses the `warrant-stream-v2` header
(VER04r section 1.2). Section 9.6 lists what this asks VER04r to add; because VER04r
is itself a draft, those are requests, and O1-S0 reconciles the two texts.

### 9.1 Streams

**OC1.** (Recipes, not bytes.) An opaque **input** is its recipe: tag `80`, a u32
little-endian maker index (declaration order), a u32 little-endian argument count,
then the encoded arguments in parameter order using the encodings of VER04r section
2.4. The count makes the value self-delimiting, as VER04r requires of every
composite. An **observation tuple** (the result of `make_obs`, OC5) is tag `81`, a
u32 little-endian observer count, then each observer value in declaration order; a
maker result in the expected stream is that tuple inside the ordinary `Ok`/`Err` or
`Option` encoding. **No host byte of a handle ever enters the corpus**, so the
corpus is language-independent and a foreign handle's layout is irrelevant.

**OC2.** (Tags.) `80` and `81` are free. Checked on 2026-11-16 against every
specification that allocates stream tags, merged or draft: VER04i section 2.6
(`00 01 10 20 30 40 41 50 51`), VER04l (`60 61`), draft VER04r (`11 60 62`) and
draft VER04p (`62 63`). The `8x` row is reserved for this document. (Those four
texts do not agree with each other about `60` to `62`; that is theirs to settle and
does not touch `8x`.) A lane decoder MUST reject an unknown tag.

**OC3.** (JSON and TFV.) In `corpus.json` a recipe value is
`{"make": "<maker>", "args": [v1, ..]}`, each argument in the JSON form of its
declared parameter type (VER04r section 2.3: integers are decimal strings), exactly
two keys. In `cases.tfv` it is the map `M{make: S<name>, args: L[...]}`, since TFV
maps have string keys (TF01 section 2). The type name, as everywhere in v2, comes
from the declared type, never from the value.

### 9.2 The handle descriptor

**OC4.** A handle binder's resolved descriptor (VER04r section 5.1) is the leaf

```json
{"d": "recipes", "t": "ByteSet", "size": 7,
 "list": [ {"make": "from_list", "args": [[]]}, {"make": "from_list", "args": [["0"]]} ]}
```

(the list is shown truncated; `size` is its real length, 7 for the ByteSet example).

The list is `D(T)` of OV1, in order, at most 2,048 entries. `size()` is its length
and `nth(i)` its `i`-th entry, so chunk jumping (VER04r section 5.4) works and a
lane never evaluates `admits` to enumerate. The independent oracle (VER04r section
10) recomputes the list from the maker rows by OV1 and fails the corpus if it
differs: the list is committed, not trusted. A claim's handle refinements are part
of the suite's ordinary `admit` predicate.

### 9.3 `corpus.json`

```json
"opaque": { "Limits": {
  "sealed": true, "from": null, "interface_digest": "<sha256 hex>",
  "makers": [ { "index": 0, "name": "limits_new",
                "params": [["max_modulus_bytes", "Nat"]], "ret": "Result<Limits,LimitsError>",
                "admits": "<Bool IR>", "rejects": [ { "when": "<Bool IR>", "with": "ZeroMaximumModulusBytes" } ],
                "means": [["maximum_modulus_bytes", "<IR>"]],
                "rejection": "complete", "requires": ["ir.core", "ir.opaque"] } ],
  "observers": [["maximum_modulus_bytes", "Nat"]],
  "injective": true } },
"ops": { "recover": { "params": [["k", "ValidatedRsaPublicKey"], ["sig", "Bytes"], ["l", "Limits"]],
                      "ret": "Result<Bytes,RecoverError>", "requires": ["ir.core", "ir.opaque"] } }
```

Ops keep the row shape of VER04r section 6: a handle parameter is an ordinary
`[name, type]` pair whose type names an entry of `opaque`. For a `from` type,
`from` holds the contract id, and the id is also copied into `assumes` (VER04r
section 12). Per-claim class counts (OV3) are in the suite rows, not here. A
`corpus.json` whose `opaque` section differs in digest from the document is the
corpus error `OpaqueDigest`; unknown fields are errors, as everywhere.

### 9.4 The IR level `ir.opaque`

**OC5.** Opaque types add one feature level (VER04r section 4.1). A suite, op or
fault that uses it lists it in `requires`; a lane that does not declare it records
the suite `not-live`, never skipped.

| Tag | Form | Meaning | Where allowed |
|---|---|---|---|
| `obs` | `["obs", "<Type>", "<observer>", h]` | the `means` of that observer at the recipe `h`: pure, model-side, no subject call (OM4) | claims, `admit`, faults |
| `maker_is` | `["maker_is", "<Type>", "<maker>", h]` | Bool: `h` was made by that maker (the `from_maker` refinement, OK7) | `admit` |
| `make_obs` | `["make_obs", "<Type>", "<maker>", [args]]` | a **subject call**: run the maker plan, call every observer once, release; yields `Ok(tuple)`, `Err(case)` or the `Option` form | only as the subject side of a derived grounding suite |
| `recipe_is` | `["recipe_is", h, <recipe literal>]` | Bool: `h` is exactly that recipe | **faults only** (`class-split`) |
| `recipe_next` | `["recipe_next", h]` | the recipe after `h` in the binder's list, cyclically | **faults only** (`stale-handle`) |

A recipe literal is `["lit", {"type": "<Type>", "value": {"make": .., "args": ..}}]`.
`make_obs` counts as one call in the call counts of VER04r section 4.4 and, like
`subject_op`, is forbidden in every model body and fault expression. A
`subject_op` whose parameter is a handle is still one call; the witness and subject
builds and the observer calls the adapter makes for it (OV5) are counted separately
in the manifest (9.5), with values the corpus predicts. `recipe_is` and
`recipe_next` exist only for faults and MUST be rejected in a claim: recipe equality
is not part of the language (OK2).

### 9.5 Digests and manifest fields

The inputs stream hashes recipes (tag 80); the expected stream hashes the model's
results; a grounding suite's expected stream hashes tag-81 tuples. Cross-lane
agreement on these digests is therefore also an agreement on every observer's
meaning in every lane that runs. The lane manifest (`warrant-lane-evidence-v2`,
VER04r section 9.1) gains, per suite row with a handle: `handle_builds` and
`observer_calls` (each compared with the corpus's prediction), `recipes`, `classes`,
`classes_with_2plus_members`, `adequacy`, `unseen_observers`; and per opaque type:
`sealing` (one of the four statuses, with the probe's id), `rejection`, `dispose`
and, in the c and cpp lanes, `memcheck` (OT8). A slim run (VER04r section 5.6)
labels a handle suite like any other: it never rounds up.

### 9.6 Requests to VER04r

These are what the draft VER04r must gain for this feature; none changes a byte of
a corpus that has no opaque type. They are recorded here because this document does
not edit another branch.

1. stream rows `80` (recipe) and `81` (observation tuple), with the JSON and TFV
   value forms of OC3;
2. the descriptor leaf `recipes` (OC4), and the oracle's recomputation of it;
3. the level `ir.opaque` with the five tags of OC5, and `make_obs` as a second
   subject-call tag in the call-count rule;
4. fault **targets**: VER04r section 8.1 addresses a fault to one `op`. Opaque
   faults also address one `maker` (a `post` rewrite of a `make_obs` result) or one
   `observer` with a `phase` of `witness` or `after-call` (a `post` rewrite of that
   observer's result). The three kinds `pre`, `post` and `replace` are unchanged, and
   a lane still implements each wrapper once (section 11);
5. the binding directives `opaque`, `make` and `observe` (section 10.1);
6. the manifest fields of 9.5;
7. conformance vectors for all of the above, as a category `v2-opaque` of the
   fixture family (VER04r section 11), before any lane merges.

## 10. Lane binding contract

### 10.1 The binding vocabulary

A per-package binding gains an `opaque` section, closed like every binding (unknown
fields are errors, W0501). In the line form of VER04j section 14 and of the v2
binding file (VER04r section 7.1; every line is data, none is code):

```text
opaque  Limits type=rsa_public_operation::RsaPublicOperationLimits sealed [release=<fn>]
make    limits_new plan=fn:RsaPublicOperationLimits::new(param.max_modulus_bytes) result=result
observe maximum_modulus_bytes method=maximum_modulus_bytes receiver=value
op      recover export=recover_encoded_message pass=k:copy,l:copy
opaque  ValidatedRsaPublicKey from
```

and in JSON:

```json
"opaque": { "Limits": {
  "type": "rsa_public_operation::RsaPublicOperationLimits", "sealed": true,
  "makers": { "limits_new": {
      "plan": [ { "fn": "RsaPublicOperationLimits::new", "args": ["param.max_modulus_bytes"] } ],
      "result": "result" } },
  "observers": { "maximum_modulus_bytes": { "method": "maximum_modulus_bytes", "receiver": "value" } },
  "release": null, "backing": null } }
```

**OL1.** (The maker plan.) A maker's `plan` is a list of **1 to 8 steps**; each is
one host function or associated function path (an identifier grammar, never
escaped, VER04k section 6.3), with arguments drawn from a closed vocabulary:
`param.<name>`, an integer, bool or byte literal, `step.<k>` (the host value of an
earlier step) and `model.<fn>(...)`, the value of a first-order document model
evaluated by the lane evaluator. The last step's value is the handle (or the
`Result` or `Option` of it); the lane's normal error normalisation applies to it
(VER04i section 5, the per-lane error models). `model.<fn>` is what lets a maker take semantic parameters
(`modulus`, `exponent`) while the host takes an encoded form (a DER string built by
a document model): the adapter does not grow an encoder per lane, it reuses the
evaluator. `backing` names the steps that own buffers the handle borrows (Rust
`'a`, a Go slice, a Swift `Data`) and keeps them alive until the handle is released.

Rules on the steps:

* the **last** step MUST be an export of the declaring package's own crate; an
  earlier step MAY be an export of that crate or of one of its **direct
  dependencies** (the real validator takes an SPKI view and ASN.1 limits that other
  crates build). Anything else is W0753. Every crate a plan touches is printed in
  the report's `tcb` line for the type;
* every step is a **public, safe** export found by the lane's export discovery: the
  generated adapter never contains `unsafe` (Rust), never reaches a private or
  test-only item, and never uses reflection. The C lane, which has no such
  distinction, is limited to functions declared in the public header;
* an earlier step that fails on a tuple `admits` or a `rejects` row covers fails the
  case with `plan-step-failed: <k>`: an intermediate failure is never read as the
  maker's rejection;
* the grounding claim (OV6) is about the **whole plan**. What the type's contract
  publishes is therefore "this plan, at this binding digest" (OF3), which is why an
  upper layer may not substitute its own (OF5).

**OL1a.** (Ownership.) `release=<fn>` names the function that destroys a handle. It
is **required** in the c lane and wherever the host type has an explicit dispose
(OL4), and absent where scope or the collector releases it. An `op` row states how
each handle parameter is passed: `pass=<param>:ref`, `copy` (the host type is
copied, the adapter keeps its own) or `consume` (the operation takes ownership: no
re-observation and no release by the adapter, OV5). The default is `ref`. The
adapter releases every build exactly once on every path, including a failed case
and an injected fault; a build it cannot account for is the failure `handle-leak`.

**OL2.** (Sealing probe, one vocabulary for every lane.) Each lane supplies an
in-lane probe producing one of the four statuses of OV7, executed by the lane
runner; the status appears in the lane's manifest and report header. The probe is a
generated source file the runner tries to build or call from **outside** the
declaring module. A lane may not report `enforced` without running it.

A compile-fail probe proves nothing if it fails for the wrong reason (a typo fails
to compile too). So every probe is a **pair**: the forging file, which must be
rejected, and a control file identical to it except that it obtains the value
through the public maker, which must build and run. `enforced` requires both
outcomes and, where the toolchain has stable diagnostic identifiers (rustc error
codes, `javac` keys, C# `CS` numbers), a match against the expected one; a lane
without them records `enforced (unmatched diagnostic)`. In the c and cpp lanes the
probe inspects declarations and never fabricates a pointer; in every lane a probe
that calls an operation with a forged value does so in a **child process** under
the same jail, so a crash is an observation (`forgeable (crash)`, OV7), not the end
of the run.

**OL3.** (Equal rigour.) Every lane MUST implement every obligation: grounding,
re-observation, freshness and the fault controls of section 11. A lane may differ
only in what it can report for `unforgeability`. A lane that cannot implement an
obligation records the operation `not-live` (W0701); it is never counted as covered
and never inherits another lane's result.

### 10.2 Per-lane contract

The eighteen lanes are those of the VER04i rollout table; "from reading" means the
hazard is the known behaviour of the language and is confirmed by the lane's PR
and by the probe, not assumed. The best status shown is what the probe can reach;
it is not a promise.

| Lane | Handle form and maker result | Hazard | Sealing probe and best status |
|---|---|---|---|
| rust | struct with private fields, `Copy` or owned; `Result<T, E>` | borrowed handles (`ValidatedRsaPublicKey<'a>`): the adapter owns `backing`; `Drop` side effects (OT7) | a `compile_fail` build of a struct literal and `T::default()` from a sibling crate: `enforced` |
| python | class instance; exceptions normalised | `object.__new__(T)` and `copy` forge any class; class-level state shared across instances | forge via `object.__new__`, call each op: `rejected-at-use` only if a brand check rejects it, else `not-established` or `forgeable` |
| typescript | class instance | TS `private` is erased at runtime; ES `#private` fields give a runtime brand test (`#x in o`) | a structurally typed literal passed through `as unknown`: `rejected-at-use` when the op does a brand test, else `not-established` or `forgeable` |
| go | pointer to struct, or struct value; `(T, error)` | `var z T` is always a legal value: the **zero value** is a forging path | zero-value probe: `rejected-at-use` if every op rejects `T{}`, `forgeable` if one accepts it; `enforced` only for an interface type or an unexported pointer type whose nil is rejected |
| ruby | object; exceptions | `allocate` and `Marshal` forge; `private_class_method :new` is bypassable by `send` | `allocate` probe: `rejected-at-use` or `not-established` |
| c | pointer to an **incomplete struct type** declared in the public header; a create function and a `free` function | no RAII: leak, double free, use after free. The adapter owns create and `release` (OL1a), releases each build exactly once on every path, and never keeps a host handle past its case: `stale-handle` is a rewrite of the **recipe**, so the wrong handle it supplies is freshly built and no fault ever touches freed memory. A consuming function is declared `pass=consume` | header inspection (no complete definition public) and a compile-fail of `sizeof(T)`: `enforced`; the memory checker of OT8 |
| cpp | class with private or deleted constructors; copyable or move-only | a by-value pass copies a handle (fine: immutable); static data shared across handles | compile-fail of a direct construction: `enforced` |
| java | final class, private constructor | reflection and `Serializable` forge; GC finalisation timing | probe asserts the class is not `Serializable`; compile-fail of `new T()` from another package: `enforced` against ordinary code, reflection out of scope (OT4) |
| kotlin | final class, private constructor | a `data class` exposes `copy` and public component functions: **a forging path** | the probe fails a data class with a public `copy`; otherwise `enforced` as java |
| csharp | sealed class or `readonly struct`, internal constructor | `default(T)` on a struct is a legal value (the zero-value hazard); finalisers | zero-value probe for structs as go; compile-fail otherwise: `enforced` |
| fsharp | record or class with `private` representation | a public record permits `{ r with ... }` forging | compile-fail of record construction: `enforced` only with a private representation |
| elixir | a struct or an opaque map; tagged tuples | `@opaque` is checked by Dialyzer only; any map literal matching the struct forges | a hand-built map passed to each op: `rejected-at-use` if the op matches a brand, else `not-established` |
| lua | a table with a metatable, or a closure | any table can be given the metatable; closures hold state in upvalues (hidden mutable state is possible: re-observation is the only defence) | a hand-built table probe: `rejected-at-use` or `not-established` |
| perl | a blessed reference | `bless` forges any reference into any class | a hand-blessed probe: `rejected-at-use` or `not-established` |
| haskell | an abstract type, constructors not exported; `Either e T` | `coerce` needs the constructor in scope; laziness: observers and results forced before comparison | a compile-fail of the constructor from an external module: `enforced` |
| swift | struct or final class, non-public initialiser | `public` memberwise initialisers must not exist; value copy versus reference semantics must be stated in the binding | a compile-fail from another module: `enforced` |
| dart | class with a library-private (`_`) constructor | Dart's privacy is per library, not per class | a compile-fail from another library: `enforced` |
| ocaml | an abstract type in the `.mli` | `Obj.magic` forges (out of scope, OT4); structural equality on abstract types must never be used by the adapter | a compile-fail of the constructor outside the module: `enforced` |

**OL4.** (Garbage-collected lanes.) A handle whose host type has a finaliser or a
dispose hook is released **inside the armed phase** by the adapter's explicit
release (`close`, `dispose`, `free`, a drop of the last reference followed by a
collection where the runtime offers it); a lane that cannot force release records
`dispose: not-forced` in its manifest. The harness leaves through `_exit`, so a
finaliser that has not run by then never runs (VER04m J2).

**OL5.** (Fresh arguments.) Operation arguments that are byte or list values are
fresh copies per case (the WX-F rule), so a host that keeps a reference to an
argument cannot carry it into the next case.

**OL6.** (Typed, lowered values.) The model values compared with the host's results
(`modulus(k)`, a `List<U8>`) follow the ordinary lane lowering of VER04i section 2.7
and VER04r section 7.2 (one exact evaluator, host types range-checked at the
boundary; unsigned 64-bit discipline in Java and Kotlin). Nothing about handles
changes it. Opaque types have no type parameters (section 4.5), so erased generics
in Java and Kotlin raise no question here: a handle is a nominal class and each
observer result is a concrete lowered type.

**OL7.** (Observers return copies; a leaked interior is a mutation path.) An observer
that returns a container (a Python `list`, a Java array, a Go slice, a Lua table, a
C pointer into the handle) may be returning the handle's own storage. The adapter
copies the result out before comparing. Then, in every lane where the host value is
mutable, it runs the `observer-leak` check once per observer per suite: it alters
the value the observer returned (never its own copy) and observes again. If the
second observation changed, a caller can mutate the handle through its observer, the
type is not immutable, and `immutability` fails with `observer-leak: <o>`. In the c
lane the check is made only when the observer's declared result is a non-`const`
pointer; writing through a `const` pointer is not attempted, and the lane prints
`observer-leak: n/a (const)`.

**OL8.** (Dynamic lanes.) In lua, perl, python, ruby and elixir a handle is an
ordinary host value and nothing stops a caller building a look-alike; that is what
the probe reports (`rejected-at-use`, `forgeable` or `not-established`, never
`enforced`). Every other obligation is as strong there as anywhere: the recipe, the
`means`, the witness check and the re-observation do not depend on host privacy.
`rejected-at-use` may be reported only when the binding names the brand check the
operation performs **and** the probe saw the declared rejection.

## 11. Sabotage controls (faults)

A fault is a deliberately wrong subject, and a control passes only if a named
obligation or claim **fails** on it. Under corpus v2 a fault is data (VER04r section
8): a `pre`, `post` or `replace` rewrite in pure IR, addressed to exactly one
operation, maker or observer (9.6 item 4), so a lane writes each wrapper once and no
lane hand-codes a sabotaged library. Every fault below that applies MUST be listed
in the `faults=` row, else W0502; a fault that does not apply is W0701 (section 15).
Controls are `negctl` entries with `required_claims`, and an unrelated failure never
credits one (VER04r section 8.3).

| # | Fault | Target and rewrite | Must be caught by |
|---|---|---|---|
| OS1 | `ignore-handle` | op, `pre` on a handle parameter: the first recipe of the binder's list | the operation claim, on a recipe whose observation tuple differs from the first's |
| OS2 | `stale-handle` | op, `pre` on a handle parameter: `recipe_next(h)`, another case's recipe | the operation claim; with `handle_builds`, the check of `freshness` (OV8) |
| OS3 | `swap-observer` | observer, `post`, phase `witness`: the `means` of another observer of the same result type | `grounding` and the witness check. Needs two observers of one type that differ on some recipe; otherwise `swap-observer: n/a (no second observer of that type)` |
| OS4 | `mutate-handle` | observer, `post`, phase `after-call`: a generic rewrite by result type (`off-by-one`, `invert`, `truncate-last`) | `immutability` (re-observation) |
| OS5 | `class-split` | op, `post`: perturb the result when `recipe_is(h, r)` for one recipe `r` of a multi-member class | the operation claim, because another recipe of that class has the same expected value. Without such a class: `n/a (no multi-member class)` |
| OS6 | `forge` | a binding row names a maker or observer the document does not declare | rejected at **load** (W0753), before any case runs; this control tests the loader, not the subject |
| OS7 | `zero-handle` | the fixture's deliberately unchecked type, whose operation accepts a default or zero value | `unforgeability`: the probe MUST report `forgeable`. A probe control on the fixture, run in every lane that has a zero or default value (go, csharp structs) |
| OS8 | `lower-fault` | observer of a `from` type, `post`, phase `witness` | the upper suite fails with `lower-contract-violated: <T>.<o>`, attributed to the **lower** layer (OF7) |
| OS9 | `admit-flip` | maker, `post`: an `Ok` result of `make_obs` becomes the maker's first declared rejection | `grounding`, with `maker-rejected-admitted`. An infallible maker: `n/a (maker cannot fail)` |
| OS10 | `observer-drift` | observer, `post`, phase `witness`, **one control per observer**: the generic rewrite for its result type | `grounding` and the witness check. It always applies, so **every observer has a control that shows its comparison can fail** |
| OS11 | `admit-rejected` | maker, `post`: an `Err` or `None` result of `make_obs` becomes `Ok` of a fixed tuple | `grounding`, on a `rejects` row. A maker with no `rejects` row: `n/a (no pinned rejection)` |

A control that cannot fail prints its condition and is never counted as caught
(OB5). `swap-observer`, `class-split`, `admit-flip` and `admit-rejected` each have
one; `observer-drift` has none, which is why it was added (review finding R6: a
type with one observer previously had no control on that observer at all). A type
that no operation takes (a document holding only the type, like pilot stage P-a)
has no operation claim and no `after-call` phase, so `ignore-handle`, `stale-handle`,
`class-split` and `mutate-handle` print `n/a (no operation takes the type)`.

## 12. Honest labels, counting and vacuity guards

### 12.1 Labels

**OB1.** (Kinds.) The kinds are the three of LP-012 and of draft VER04r section 9.3:
`full`, `bounded-set`, `sample`. A handle binder is `bounded-set` when its list
holds every admitted tuple of the makers' argument domains, and `sample` when the
scan bound or the cap cut it (OV1). The kind of a claim is the weakest among its
binders. A handle binder is never `full`: the domain is the image of the makers
over a stated argument domain, and "the image of a maker is not a type";
completeness would additionally need adequacy and unforgeability, both tested here
and neither proved. A `vectors` row or `example` with a recipe literal is an
example, whatever the size of the table (OX1).

**OB2.** (Rungs.) The VER29 rung of a handle claim is at most `exhaustive-sampled`.
A grounding claim takes the kind of its **maker's argument binders** (OV6): over a
complete argument type it may be `full` and `exhaustive`, under the H3 rule of
VER04m E14 and VER29 section 6.2. The unforgeability status, the witness and
freshness gates, and `adequacy: ...` are **printed annotations**, never rungs.
An operation over a foreign handle is at most the minimum of its own strength and
the lower opaque contract's (VER29 section 6.3 as amended).

| Claim form | Example | Kind | VER29 rung |
|---|---|---|---|
| operation versus model over `forall h: T` | `contains_agrees` | `bounded-set  handles(via from_list: 7 of 7 admitted, 7 scanned)` | `exhaustive-sampled` |
| operation versus model over `forall h: T, x: U8` | the same, 7 x 256 = 1,792 cases | `bounded-set` | `exhaustive-sampled` |
| the same with a capped or cut handle list | a key type | `sample` | `exhaustive-sampled` |
| derived grounding claim over a complete argument type | a `Bool`-parameter maker | `full` | `exhaustive` (H3 satisfied) |
| derived grounding claim over `Nat` | `limits_new` | `bounded-set` | `exhaustive-sampled` |
| an operation over a foreign handle | `recover` over a `ValidatedRsaPublicKey` | per binders | at most the lower contract's rung |
| a claim whose type has no passed grounding suite | any | n/a | not publishable (`ungrounded`) |
| `property` and `fuzz` over handles | recipes generated | `sample` | `randomized` |
| a `vectors` table whose `holds` uses a recipe literal | the pilot's known answers | n/a (rows) | `example` |

**OB3.** (Printed annotations.) A handle claim prints, beside its label, one
`adequacy` line (`adequate (c of n ...)`, `n/a (injective)` or `untested`), the
`unseen-observer` list (OV4), the sealing status per lane (OV7), and each maker's
`rejection` line. `adequacy: untested` is a valid published state at
`exhaustive-sampled`; it makes `--require-closed` fail for the link that rests on
it (VER29 section 5.6), and never a stronger label.

### 12.2 The report block (sketch)

The numbers are a toy: 4 keys, 6 signatures and 64 limits give 4 x 6 x 64 = 1,536
cases, and two handle parameters with two builds each give 4 x 1,536 = 6,144 builds.

```text
opaque Limits  sealed   makers: limits_new     observers: maximum_modulus_bytes
  grounding            bounded-set  exhaustive-sampled  cases=65  rejection: complete
  adequacy:            n/a (injective: every maker parameter is an observer)
  unforgeability:      rust enforced   python not-established   go rejected-at-use   ...
op recover  (handles: ValidatedRsaPublicKey from contract.x509-rsa-public-key.opaque..., Limits)
  recover_matches_model bounded-set  exhaustive-sampled
        handles(via key_from_parts: 4 of 4 admitted, 6 scanned; via limits_new: 64 of 64 admitted, 65 scanned)
        cases=4*6*64=1,536   adequacy: n/a (injective)   unseen-observer: modulus_bits (determined ...)
  obligations: immutability pass (control ok)  freshness pass (handle_builds=6,144 as predicted)
               witness check pass (lower contract x509-rsa-public-key: discharged, re-checked per case)
  tcb: makers' means and admits (restated, pinned), lower plan (pinned; crates: ...), adapter
```

### 12.3 Vacuity guards

| # | Failure | Prevention |
|---|---|---|
| OB4 | **An empty or tiny handle domain proves nothing**: every recipe rejected, or one recipe tried | zero admitted recipes is W0601; the label prints `admitted of tried`; a type whose `D(T)` has one recipe prints `handles: 1` and the claim cannot be called anything but a single example |
| OB5 | **A check that did not run, counted as passed** | an obligation that does not apply prints `n/a`; a not-live lane never inherits a pass (OL3); every control prints its vacuity condition (section 11) |
| OB6 | **Silent partial rejection**: a `Result` maker whose `rejects` rows do not cover everything `admits` leaves out | `rejection` is computed by evaluation over the argument domain and always printed; `complete` is never inferred from a clause being present (section 3.5) |
| OB7 | **Circular observers**: expected values taken from the subject | `means` evaluated by the model, never the subject (OM4); `ungrounded` blocks publication (OV6) |
| OB8 | **A claim that cannot tell an ignored observation from an unvaried one** | `unseen-observer` (OV4) |
| OB9 | **Sealing implied by the language's reputation** | one four-valued probe per lane (OV7, OL2); `forgeable` fails |
| OB10 | **A lower fault blamed on the upper layer, or hidden by it** | the witness check and `lower-fault` (OF7, OS8) |
| OB11 | **A stateful type passed off as immutable** | `immutability` (OV5) |
| OB12 | **The report implies "the handle proves validation happened"** | OT4: sealing is a host property, printed per lane, never a rung; no invariant is imported (OF8) |
| OB13 | **An observation interface so weak that anything passes** | a weak interface makes operation claims *harder* to satisfy, not easier: the model sees only `obs(h)`, so an operation that depends on anything else fails on a multi-member class, and where there is none the claim prints `adequacy: untested`. A type with no observer is refused (section 4.5) |
| OB14 | **Observations that are declared and never compared** | every observer is called and compared in every grounding case and twice in every operation case (OV5), and each has its own `observer-drift` control (OS10) |
| OB15 | **A claim about observers alone, counted as evidence about the subject** | it calls no operation and is refused (OK4) |
| OB16 | **Observing a handle before using it hides a lazy-initialisation fault** | the witness and subject builds (OV5, N10) |
| OB17 | **A known-answer table passed off as a domain** | recipe literals in `vectors` are examples (OX1, OB1); a table no fault can fail is W0601 (VER04j section 5.3) |
| OB18 | **An upper layer builds foreign handles its own way** | the lower plan at a pinned digest, no upper plan (OF5) |

## 13. Security considerations

**OT1.** **Bindings are untrusted input spliced into generated source.** Every new
string (`type`, `plan` function paths, `method`, `backing`, `model.<fn>` names) is
validated against an identifier or path grammar and a closed vocabulary before
generation and never escaped (VER04k section 6.3). A binding that fails validation
generates nothing (fail closed).

**OT2.** **No secrets in the corpus.** A recipe is written into a committed corpus
(OC1). A maker parameter or observer result classed `secret` is refused (OK6), so a
private key, a MAC key or any key material can never be a recipe. Public-key handles
(the pilot) are admitted precisely because their recipes are public.

**OT3.** **No host `Debug` or `Display` of a handle in a failure message.** A runner
prints the recipe and the observation tuples, never the host's own text for the
handle, which may contain addresses, secrets or nondeterministic bytes.

**OT4.** **Unforgeability is a property of the host, reported and not proved.** The
probe (OV7) finds the public paths of ordinary code. Reflection, unsafe code,
`Obj.magic`, serialisation and FFI are out of scope. A layered argument of the form
"this value was validated because the type cannot be forged" rests on the sealing
status of the **lane the code runs in**; the report prints that status and never
upgrades a rung on it.

**OT5.** **Resource caps (CWE-400).** At most 8 makers, 8 observers, 8 opaque types
per document; `MAX_RECIPE_SCAN = 1,048,576` tuples and `MAX_RECIPES_PER_MAKER = 256`
per maker, so at most 2,048 recipes and 1 MiB of recipe list per type; at most 4
handle parameters per operation, hence at most 8 live handles per case (a witness
and a subject build each) plus those of constructors, and at most 16 observer calls
per handle parameter per case (8 on the witness, 8 after the call), so at most 64
per case; a maker plan of at most 8
steps; `admits`, `rejects`, `means` and `model.<fn>` plan arguments are charged to
the enclosing meter (VER04k section 2.7); the budget in force and 64 MiB over the
product of binder domains including `|D(T)|`, enforced before enumeration (OV2).

**OT6.** **Pins and hand-edited lower evidence.** A foreign handle's soundness rests
on the lower manifest, the lower report and the interface digest (OF3). A hand-edited
lower manifest is what the digests and VER29 condition (c) are for.

**OT7.** **The runtime jail stays mandatory** (owner decision d). Makers, observers
and operations run inside the armed phase of VER08b. A maker that needs a capability
(`fs`, `net`, `proc`, `time`, `rand`, `env`) is stopped by the gate, and the type is
`not-live` for the runtime rung until the capability is allowed, never run unarmed.
Handles over OS resources are out of scope for exactly this reason. **A handle is
not a way to carry a capability across the gate**: every handle built before arming
(the phase-A warm-up builds) is released before the gate arms and is never passed to
a recorded case, and a maker plan cannot name a function outside the crates of OL1,
so a binding cannot use a plan step to open a resource the jail would refuse later. Destructors run
inside the armed phase (OL4, VER04m J2). The warm-up and order-independence rules of
VER04m J4 and J6 apply: one unrecorded warm-up case per operation in phase A, and
`order-independence` runs armed.

**OT8.** **Memory safety in C and C++.** The handle-specific risks (double free,
use after free, a leaked handle) are the adapter's, so ownership is explicit in the
binding (`release`, `pass`, OL1a), each build is released exactly once on every
path, and no fault keeps a host handle across cases (section 10.2, the c row). The
c and cpp lanes MUST run their opaque suites under a memory checker (AddressSanitizer
with leak detection, or Valgrind) in at least the nightly full run, and the manifest
records `memcheck: asan`, `valgrind` or `not-run`. `not-run` is printed in the
report and blocks `--require-closed`; it is never silent. If the checker cannot run
under the armed jail, it runs the same suites in a second, unrecorded pass whose
only verdict is the checker's.

**OT9.** **Trust base.** The restated interface (and its digest), the `admits` and
`means` models, the binding plan, the `model.<fn>` host arguments and the adapter
are recorded in the report's `tcb` section. `means` is the one *declared* modelling
claim in this design and is printed as declared.

**OT10.** **Not an erasure or timing argument.** Re-observation makes writes
observable; it says nothing about dead-store elimination of secrets, nor about
timing.

**OT11.** **Forgery through the adapter itself.** The adapter is the one place that
holds real handles, so it must not become a forging path: it obtains a handle only
as the last step of a declared plan, calls only public safe exports (OL1), emits no
`unsafe`, reflection or serialisation, and the `forge` control (OS6) shows the
loader refuses a row the document does not declare.

## 14. The pilot: `rsa-public-operation`

### 14.1 Why this crate

From reading: `rsa_public_operation` is a security-flagged crate with 17 dependents
in the inventory, whose single operation
`recover_encoded_message(public_key: ValidatedRsaPublicKey<'_>, signature: &[u8],
limits: RsaPublicOperationLimits) -> Result<Vec<u8>, RsaPublicOperationError>`
takes two handles: one **local** (`RsaPublicOperationLimits`, constructor `new`
rejecting zero, observer `maximum_modulus_bytes`) and one **foreign**
(`ValidatedRsaPublicKey`, private fields, observers `modulus`, `modulus_bits`,
`public_exponent`, produced by `x509_rsa_public_key::validate_rsa_public_key`). It
exercises both halves of this specification and nothing else the freeze still holds
(no floats, collections or generics). Its error enum
(`ModulusTooLong`, `SignatureLengthMismatch`, `SignatureRepresentativeOutOfRange`)
is closed and redacted. Also from reading: both handle types are `Copy`
(`pass=copy`); the checks run in the order limit, length, range; the result is the
big-endian value padded to the modulus length; and the lower validator requires an
odd modulus, an odd exponent of at least 3 that is below the modulus and allowed by
the policy, and a bit length inside the **caller's** policy range. Its constructor
takes an `X509SubjectPublicKeyInfo`, an `RsaPublicKeyPolicy` and `Asn1Limits`, host
values of the lower crate and of two further crates, so the lower maker is a plan
(OL1, N11).

### 14.2 Staging

* **Stage P-a (slice O1, local only).** Adopt only the **local** type. A document
  with `opaque Limits sealed` (section 4.3), its derived grounding claim (65 cases
  over `Nat` `0..=64`, 64 admitted and 1 rejected, `rejection: complete`,
  `n/a (injective)`) and the sealing probe. The crate's one operation also needs the
  foreign key, so **no operation takes the type in this stage**: the controls that
  apply are `observer-drift`, `admit-flip`, `admit-rejected` and `forge`, and the
  other operation-level ones print `n/a (no operation takes the type)` (section 11).
  Stage P-a therefore proves the maker, observer, corpus and probe path on a real
  crate and nothing about an operation over a handle; that half of O1 is accepted on
  the ByteSet fixture (conformance item 7). It needs no VER29 change.
* **Stage P-b (slice O3 and O4, foreign handle).** The key handle, cited through
  `contract.x509-rsa-public-key.opaque.ValidatedRsaPublicKey`. **Blocked until
  `x509-rsa-public-key` has an adopted Warrant document and the VER29 slice that
  publishes the opaque id has landed** (decision b, OF6). The fallback pilot, if the
  lower adoption is late, is `x509-rsa-algorithm-policy` (6 dependents); it is
  recorded so the schedule does not stall, not chosen here (owner question OWN-4).

### 14.3 The upper document (sketch)

```text
warrant rsa_public_operation v2

opaque Limits sealed
  make limits_new(max_modulus_bytes: Nat) -> Result<Limits, LimitsError>
    admits  max_modulus_bytes >= 1
    rejects max_modulus_bytes == 0 with ZeroMaximumModulusBytes
    means   maximum_modulus_bytes = max_modulus_bytes
  observe maximum_modulus_bytes(l: Limits) -> Nat

opaque ValidatedRsaPublicKey sealed from contract.x509-rsa-public-key.opaque.ValidatedRsaPublicKey
  -- the restated interface of section 6.2, equal to the lower declaration

op recover(k: ValidatedRsaPublicKey, sig: Bytes, l: Limits) -> Result<Bytes, RecoverError>

-- (1) the whole behaviour, errors and success, against a model, for toy keys
claim recover_matches_model:
  forall k: ValidatedRsaPublicKey | len(modulus(k)) <= 4, sig: Bytes, l: Limits.
    recover(k, sig, l) ==
      m_recover(modulus(k), public_exponent(k), sig, maximum_modulus_bytes(l))

-- (2) real-size public keys: three results that need no big arithmetic
vectors rsa_public_keys:
  source "rsa-public-operation-v1/public-keys.tfv"
  sha256 b"..."
  rows 24
  column n: Bytes
  column e: U64
  holds  recover(make key_from_parts(n, e), m_zero(len(n)), make limits_new(len(n)))
           == Ok(m_zero(len(n)))
     and recover(make key_from_parts(n, e), m_one(len(n)), make limits_new(len(n)))
           == Ok(m_one(len(n)))
     and recover(make key_from_parts(n, e), m_clear_low_bit(n), make limits_new(len(n)))
           == Ok(m_clear_low_bit(n))

-- (3) real-size known answers from an independent implementation
vectors rsavp1_known_answers:
  source "rsa-public-operation-v1/rsavp1.tfv"
  sha256 b"..."
  rows 48
  column n: Bytes
  column e: U64
  column sig: Bytes
  column expected: Bytes
  holds  recover(make key_from_parts(n, e), sig, make limits_new(len(n))) == Ok(expected)
```

(The row counts 24 and 48 are placeholders for the adopter; the `sha256` values
are elided.)

**Is `Nat` wide enough for an RSAVP1 model? Settled: not at real sizes, and it does
not need to be.** The evaluated `Nat` is a `u64` in the shipped Rust evaluator
(VER04e) and an exact integer of magnitude at most `2^127` under draft VER04r
section 3.2. A 2,048-bit modulus fits neither, and widening `Nat` to unbounded
integers in eighteen lane evaluators is a language change this document does not
make (owner question OWN-3). The pilot instead states the success value in three
layers, each labelled for what it is:

1. **`recover_matches_model`: a full model on toy keys.** The lower validator's
   size floor is the policy's, so a plan whose policy admits small moduli yields
   real `ValidatedRsaPublicKey` values with moduli of one to four bytes. For a
   modulus below `2^32` every intermediate of square-and-multiply is below `2^64`,
   so `m_recover` is ordinary `Nat` arithmetic (`from_be`, `rem`, a 64-step fold
   over the exponent bits) in both evaluators. This claim covers all three error
   paths, their order, and the success value, over every toy key, signature and
   limit in the domain: kind `bounded-set` or `sample` (OV1), rung
   `exhaustive-sampled`. It is where `ignore-handle`, `stale-handle` and
   `class-split` are caught.
2. **`rsa_public_keys`: three exact results at real sizes.** For any valid key,
   `0^e mod n = 0`, `1^e mod n = 1`, and, because `e` is odd, `(n-1)^e mod n = n-1`.
   `n` is odd, so `n - 1` is `n` with its lowest bit cleared: all three expected
   values are byte-wise models (`m_zero`, `m_one`, `m_clear_low_bit`) with no
   external data, and the third depends on the key, so a subject that ignores the
   key fails. These rows exercise the multi-limb paths of the subject's big-integer
   code at real sizes with expected values the document derives itself.
3. **`rsavp1_known_answers`: known answers.** Rows produced by an independent
   implementation, pinned by digest (VER04j section 5).

**What the table is, honestly.** Layers 2 and 3 are `vectors` tables: their rows
are **examples** for every count, label and rung (OB1, OB2 table), and a recipe
literal in a row never widens the handle domain (OX1). The report prints, for
`recover`, the line `success value: model on toy keys (modulus <= 4 bytes); at
larger sizes 3 fixed points per key over <r1> keys and <r2> known-answer rows; no
model above 4 bytes`. The fixture README records the tool that produced the known
answers, its version and the generation procedure, and that tool is listed in the
report's `tcb` line; the tool MUST NOT be the subject or share its big-integer
code. Each table is subject to the existing rule that some applicable fault must
fail it (W0601 otherwise), and the adopter's tables MUST contain two rows with the
same signature and different keys and two with the same key and different
signatures, so that neither argument can be ignored. Every row's recipe is checked
against the restated `admits` (OX1), and its handle goes through the witness check
like any other. Moduli of five bytes up to the smallest table key are covered by
nothing; the report line above says so.

`m_error_path` of the first draft is folded into `m_recover`. Counts for the toy
domain used in the report sketch (12.2) are `4 keys x 6 signatures x 64 limits =
1,536` cases, computed by multiplying the three binder domains; the generator, not
this document, fixes the real sets.

### 14.4 Acceptance

* Stage P-a: the Limits type yields 65 grounding cases, `rejection: complete`,
  `adequacy: n/a (injective)`; `observer-drift`, `admit-flip`, `admit-rejected` and
  `forge` are caught and the operation-level controls print their `n/a`; the Rust
  sealing probe prints `enforced`, with its control file building.
* Stage P-b: `recover_matches_model` prints the foreign handle's label and
  `witness check pass (lower contract ...: discharged, re-checked per case)`;
  `ignore-handle`, `stale-handle`, `mutate-handle` and `lower-fault` are caught, the
  last as `lower-contract-violated` attributed to `x509-rsa-public-key`; a lower
  report whose own controls did not pass leaves the upper link open with
  `lower-fault-mode`; a mismatched restated interface is
  `ContractInterfaceMismatch`; an upper binding that carries its own plan for the
  key is W0753; both tables pass, each fails under at least one fault, and the
  report prints the `success value` line.
* **The design is wrong** if `recover` cannot be stated over a handle it did not
  build (the key) while running the real lower plan with no mock, or if no policy
  the lower package can publish admits a modulus of at most four bytes (then layer 1
  has no domain and the question of OWN-3 must be answered first).

## 15. Diagnostics

Existing codes keep their meaning and are reused wherever the condition is one they
already name. Four codes are new. `W0750` to `W0753` were checked unused on
2026-11-16 in every specification on `origin/main` (the highest code in use is
`W0733`) and in the drafts VER04p and VER04r, neither of which allocates a new code.
The block `W0750` to `W0759` is reserved for this document, the last six unassigned.
If another document merges the same numbers first, the later one renumbers at merge;
O1-S0 repeats the check.

| ID | Slice | Phase | Trigger |
|---|---|---|---|
| **W0750** | O1 | checking | an opaque declaration rule: no observer, no maker, a `means` missing, duplicated or of the wrong type, `admits` absent, an impure or ill-typed clause, an overlap of `admits` and `rejects`, a recipe literal that violates `admits`, a secret-class parameter or observer (OK6), a type parameter, a count over the caps of section 4.5, an observer result containing an opaque type, a `rejects` row with or without `with` against the maker's result form, a `vectors` row whose recipe literal is not admitted |
| **W0751** | O1 | checking | a forbidden use of an opaque value (OK2, OK3, OK7, OX1): comparison, pattern, nesting, a `model` signature, an `inout` handle, a handle in an op result, an unsupported binder refinement, a recipe literal outside an `example` or a `vectors` `holds` |
| **W0752** | O3 | checking | a `from` clause that is malformed (not a 4-segment `opaque` id), a `from` in an O1 build, a `sealed` mismatch against the id, or a restated interface not shaped like a declaration |
| **W0753** | O1/O3 | generation | a binding against the checked document: a maker `plan` inconsistent with the declaration (arity, type, more than 8 steps, a last step outside the package's crate, an earlier step outside it and its direct dependencies, a step that is not a public safe export), an observer missing or extra, a row naming a maker or observer the document does not declare (the `forge` control), a `sealed` flag that differs from the declaration, a missing `release` where OL1a requires one, any row but `opaque <T> from` for a `from` type (OF5), a type whose defining crate is another package without `from` (OF6), one host type bound by two mechanisms (OK11) |

Reused: **W0101** (a v2 construct in a v0 or v1 document, OC0), **W0501** (a
malformed `opaque` binding entry), **W0502** (a `faults=` list that omits an
applicable fault of section 11), **W0601** (a handle domain with zero admitted
recipes), **W0701** (a fault that does not apply, a lane with no opaque adapter or without the
level `ir.opaque`), **W0706** (a handle domain or recipe list over budget), the
existing "claim calls no operation" diagnostic (OK4), and the VER29 errors
`ContractIdMalformed`, `ContractUnknownClaim`, `ContractNotDependency`,
`ContractDuplicate`, `ContractPinMismatch` plus the new `ContractInterfaceMismatch`
(section 17.1).

## 16. Conformance

An implementation of a slice conforms when every item for that slice holds. A
"control" is the negative control named; a rule without a passing control is not
implemented.

**All slices.**

1. Every v0 and v1 document regenerates byte-identically (OC0b); `CheckedWarrantModule`
   gains an empty `opaque` table only.
2. A handle binder never prints `full` (OB1); LP-012's count split is implemented.
3. An obligation that does not apply prints `n/a`, never `pass` (OB5).

**O1.**

4. Each rule OK1 to OK11 has a failing example producing its named diagnostic.
5. The ByteSet example yields 7 recipes, 4 classes, 3 multi-member classes; the
   ByteSet claim `contains_agrees` yields 7 x 256 = 1,792 cases (OV3, 12.1).
6. `limits_new` yields 65 grounding cases, 64 admitted and one rejected, `injective`
   (OV1, OV3).
7. `ignore-handle`, `stale-handle`, `mutate-handle`, `class-split`, `forge` and
   `observer-drift` (once per observer) are each caught on the ByteSet fixture.
   `swap-observer` prints its `n/a` there (the two observers have different types)
   and is caught on a fixture type with two `Nat` observers; `admit-flip` and
   `admit-rejected` are caught on `Limits`, where `class-split` prints its `n/a`
   (section 11).
7a. A fixture type that fills an observed field on first observation and whose
   operation reads it unfilled **fails** (the witness and subject builds, OV5); the
   same type passes a runner that wrongly observes before the call, which is the
   negative control of this rule.
7b. A claim that mentions observers and no operation is refused (OK4); a `means`
   that is one-to-one only on the domain prints `adequacy: untested`, not
   `n/a (injective)` (OV3); a refinement that removes every multi-member class
   turns `adequate` into `untested` for that claim.
8. A claim over a type with no passed grounding suite prints `ungrounded` and is
   not published (OV6).
9. A maker with a secret-class parameter is W0750 (OK6, OT2).
10. The Rust lane's sealing probe prints `enforced` for a private-field struct and
    `forgeable` for a struct with public fields (the control, OV7); a probe whose
    control file does not build is not `enforced` (OL2).
11. The pilot Stage P-a of section 14.4.

**O2 (per lane).**

12. The lane passes the shared fixture (`opaque-fixture`, ByteSet and Limits) as a
    `warrant-corpus-v2` corpus, with identical digests on the inputs and expected
    streams as the Rust reference (section 9.5), and passes the `v2-opaque`
    conformance category (9.6 item 7). The `Limits` half needs `ir.core` and
    `ir.opaque`; the ByteSet half also needs the level that carries its sort model
    (`ir.seq`), and is `not-live` in a lane until that lane has it.
13. The lane implements every one of the eleven controls that applies to it and the
    sealing probe pair, prints one of the four statuses (OL2, OL3), reproduces the
    predicted `handle_builds` and `observer_calls`, and runs the `observer-leak`
    check where its host values are mutable (OL7).
13a. The c and cpp lanes record `memcheck` and release every build exactly once,
    including under each fault and for a `pass=consume` operation (OL1a, OT8).
14. The per-lane hazard row of 10.2 is exercised by a test that fails when the hazard
    is introduced (zero value, `copy`, `__new__`, `allocate`, `bless`, a hand-built
    map, a literal as `unknown`).

**O3.**

15. `ContractInterfaceMismatch` (including an interface that differs only in maker
    order), `ContractUnknownClaim` for a lower package with no Warrant, and
    `ContractNotDependency` each have a failing example (OF1, OF5, OF6); an upper
    binding with its own plan for a `from` type, and a re-exported lower type
    declared without `from`, are each W0753.
16. `lower-fault` is attributed to the lower layer (OF7, OS8).
17. A restated interface that differs in one `means` expression changes the digest
    and fails `--check`; so does a change to the lower plan (the binding digest, OF3).
18. A lower report whose `observer-drift` control did not pass, or whose sealing
    status is `forgeable`, leaves the upper link open (OF4).

**O4.** Stage P-b of section 14.4.

## 17. Amendments in this series

These are the exact edits. They are applied in the implementation slice whose code
makes each true (VER29 section 16's practice); each target document carries a
pointer note at its top now.

### 17.1 VER29

* **Section 3.1, table.** Add the row
  `opaque | contract.<package>.opaque.<Type> | contract.x509-rsa-public-key.opaque.ValidatedRsaPublicKey | 4`.
  `<Type>` is a Warrant identifier. The four-segment shapes are told apart by their
  third segment (`timing`, `capability`, `opaque`), a keyword that a three-segment
  claim id cannot collide with.
* **Section 3.2.** Add: `opaque`: the lower package's own document declares
  `opaque <Type>` without `from`. A `from` restatement does not create an id in the
  restating package.
* **Section 3.3.** After the platform-row rule add: a `contract_id` is also recognised
  after the contextual word `from` in an `opaque` declaration (VER04n 4.1). It is not
  a platform row, adds no name to the symbol namespace, and is `W0101` in a document
  without the `v2` header (the `v1` wording of this section applies to `v1`
  constructs; opaque types are `v2`). An id written under `from` and again under
  `assume platform` is `ContractDuplicate`.
* **Section 3.5.** Every non-`from` `opaque` type is published implicitly as one id
  covering its whole interface.
* **Section 5.1.** For an opaque id, "the claim" is the **evidence set** of OF4: every
  maker's derived grounding claim, the re-observation and freshness obligations of
  every suite in the lower package that takes the type, with the five conditions
  applied to each member, and a lower sealing status that is not `forgeable`.
  Non-Rust lanes report `lane-not-supported` (OF9).
* **Section 5.2, errors.** Add `ContractInterfaceMismatch`: the restated interface
  after `from` differs from the lower declaration, or its `sealed` flag does.
  (No new open reason: a failing member surfaces as `suite-failed`.)
* **Section 5.3, pins.** Add `opaque_interface_digest` and `opaque_binding_digest`
  per `from` link (OF3).
* **Section 5.5 item 3.** For an opaque contract the negative controls read are the
  **lower** package's own controls on the type's makers and observers
  (`observer-drift`, `swap-observer`, `admit-flip`, `admit-rejected`; VER04n section
  11); a verdict other than `pass` makes the contract open with `lower-fault-mode`.
  `lower-fault` (VER04n OS8) is a control of the **upper** package: it shows that an
  upper run notices lower drift and attributes it. It is listed in the upper
  manifest and is not what item 3 reads. A faulted run writes no manifest, so
  neither control can open or close a link by itself.
* **Section 6.2, "Complete domain".** Add: a binder of opaque type is never a
  complete type, whatever its maker arguments (VER04n OB1).
* **Section 6.4.** An upper claim that quantifies over a handle type rests on that
  type's opaque contract **in addition to** the contracts of the operations it uses;
  before the reach pass (slice 4) such a claim is `unattributed`, like any other.
* **Section 8.3.** A note that the classification word `opaque` there means "a
  construct the reach pass cannot see through" and is unrelated to opaque types.
* **Section 13.** Add slice 6, **opaque contracts**: the `opaque` id family, the
  `from` recognition position, the interface digest, resolution and discharge of OF4,
  the `lower-fault` attribution, `ContractInterfaceMismatch`. It follows slice 2.

### 17.2 VER04m

* **Section 3.5.** After the paragraph on hidden state add a three-way table: a state
  the harness can build and read field by field is Mode E; a hidden mutable state is
  Mode P; a hidden **immutable** value determined by how it was made is opaque
  (VER04n). Add: "a type that has a mutating method is never opaque".
* **Section 4.4 (E11), final bullet.** Append: "An immutable type whose fields are
  private and which has no total builder from a record is not described by a codec; it
  is an opaque type (VER04n). The `observe` adapter code is shared (VER04n OR2)."
* **Section 6.1.** Add: constructor arguments of a `dispatch` driver MAY be
  handle-typed when the handle's type is declared opaque (v2 documents only); `Call`
  payloads MUST NOT be. A handle-typed constructor argument is a recipe built fresh
  for each drive. Both S2 and O1 are required (VER04n OR3).
* **Section 9.1, J2.** Append: "The same rule applies to handles of VER04n: they are
  released inside the armed phase."
* **Section 14, O-8.** Append: "opaque types are specified in VER04n; the inventory
  figure for them is re-measured with an `opaque` tag as for the other two features."

### 17.3 Not edited

VER04, VER04h, VER04i, VER04j, VER04k, VER04l, VER30 and `VER-BACKLOG.md` are not
edited by this document. The rows the backlog needs are listed in section 19.2;
another change updates the backlog and VER30.

## 18. Out of scope

* **Transformers**: operations `T -> T` or `T -> Option<T>` that take a handle and
  return a handle (section 20.3, OQ-2).
* **Nested handles**: a handle inside a `List`, record, variant, `Option`, `Set`,
  `Map`, or as a maker argument; and a handle field of a Mode E state record.
* **Generic opaque types** (`HashSet<T>`): VER04o amends section 4.5.
* **Handles over OS resources** (files, sockets, threads, processes): the jail
  forbids them (OT7).
* **Secret-bearing handles**: private keys, MAC keys, key schedules (OK6, OT2).
* **Identity equality**, address equality, hash codes of handles; **aliasing** (one
  host value passed for two parameters, OV8).
* **Type invariants** imported from an opaque contract (OF8).
* **Types with no observer** (capability tokens, section 4.5).
* **Drop semantics as behaviour** (only that destructors run armed, OL4); `dyn Trait`
  objects; stored callbacks; serialised handles; handles sent across threads.
* **`eq=observational`** (N4, owner question OWN-1).
* **Proofs**: all opaque rows are `not-live` for the proof rung at first, as in
  VER04m, acknowledged in the binding.
* **Reflection, unsafe code and FFI forging** (OT4); **timing** of handle
  operations (VER20); **interior mutability that no observer can see** (an
  unobservable cache is allowed and, by definition, invisible).

## 19. Implementation slices and backlog rows

### 19.1 Slices

| Slice | Scope | Prerequisites |
|---|---|---|
| **O0** | this specification, adversarial review recorded in section 21 | none |
| **O1-S0** | errata pass against the then-current VER04j, k, l, m and VER29; reconcile section 9 with the then-current VER04r and file the requests of 9.6; re-confirm tags `80` and `81` and the code block `W0750` to `W0759`; accept `opaque`-gated constructs in the `v2` header parser | O0 reviewed; T1-S0 (`v2` accepted) |
| **O1-S1** | checker and ground evaluator: declarations, rules OK1 to OK11, `admits` and `means` evaluation, the recipe list of OV1, classes, injectivity, recipe literals in `example` and `vectors` | O1-S0 |
| **O1-S2** | evidence generator: the v2 `corpus.json` section, tags 80 and 81, the `recipes` descriptor, level `ir.opaque`, derived grounding claims, five obligations, labels, the eleven controls as fault data, report block; the same level in the independent oracle and the `v2-opaque` conformance category (the four-PR pattern of VER04r section 13) | O1-S1; LP-012 count split; VER04r slices S3 to S5 (v2 generator, oracle, fixture) |
| **O1-S3** | Rust lane runner: binding reader (`opaque` section, plans, `release`, `pass`, `backing`), witness and subject builds, sealing probe pair, the shared fixture `opaque-fixture` in Rust, pilot **Stage P-a** | O1-S2; the shared-crate queue of VER30 section 5.3 |
| **O2a** | lanes python, typescript, go (the VER04i order): the opaque adapter in the lane's **v2** runner, fixture port, probe pair, controls, hazard-row test, `observer-leak` | O1-S3; the lane's v2 runner (VER04r S6, S7); a lane may not skip ahead (VER04i) |
| **O2b** | lanes ruby, c, cpp, java, kotlin, csharp, fsharp, elixir; the c and cpp memory checker (OT8) | O2a |
| **O2c** | lanes lua, perl, haskell, swift, dart, ocaml | O2b; toolchains in CI first, as VER04i requires |
| **O3** | foreign handles: the VER29 slice 6 of 17.1, the `from` checker rule, the interface and binding digests, the lower plan read from the lower binding, `lower-fault`; the O3-checker work of OF1 to OF9 (Rust lane only, OF9) | O1-S3; VER29 slice 2 (resolution and discharge); the lower package adopted |
| **O4** | pilot **Stage P-b**: `rsa-public-operation` over the foreign key | O3; `x509-rsa-public-key` Warrant adoption |
| **O5** (deferred) | transformers, nested handles, generic opaque types | demand, VER04o |

### 19.2 Backlog rows needed (this document does not edit `VER-BACKLOG.md`)

* **WX-O0**: spec opaque types (this document), owner decisions of 2026-11-16;
  adversarial review required.
* **WX-O1**: slices O1-S0 to O1-S3, with pilot Stage P-a (`rsa-public-operation`'s
  `Limits`), after WX-S1 and T1-S0.
* **WX-O2a, WX-O2b, WX-O2c**: lane rollout, with the `opaque-fixture` port per lane
  as its own package (BUILD, README, CHANGELOG, `required_capabilities.json`).
* **WX-O3**: foreign handles and the VER29 slice 6 (`LV-` row for the id family
  `contract.<pkg>.opaque.<T>`).
* **WX-O4**: pilot Stage P-b.
* **Prerequisite**: adopt `x509-rsa-public-key` (a Warrant document and evidence); a
  VER30 wave-2 crate, "may be T3" there, now unblocked as the lower layer.
* **VER30 / inventory**: re-measure the opaque-types row with a `recursive-types`
  note, as the memo recommends, before scheduling adopters beyond the pilot.

## 20. Open questions

### 20.1 For the owner

Four questions are choices of policy or scope, not of engineering. Each has a
recommendation, and the text above already encodes it, so a "yes" needs no edit.

| # | Question | Recommendation |
|---|---|---|
| OWN-1 | **`eq=observational`.** The design memo proposed (O-1) that an opaque type may opt in to `==` meaning "equal observations". This document leaves it out (N4). Confirm the departure? | **Leave it out.** `==` on a handle stays an error. Where a library exposes equality, state it as an operation: `op key_eq(a: K, b: K) -> Bool` with the claim `key_eq(a, b) == (obs(a) == obs(b))`, which is checked against the real function and needs no new feature. An implicit `==` would be a second, unchecked meaning of equality. Reconsider only if two adopters need handles as map keys (VER04o) |
| OWN-2 | **Does a sealing status gate adoption?** Dynamic lanes can never print `enforced` (OL8). Should a security-flagged crate's `sealed` type be adoptable in a lane that prints `not-established`? | **Yes, adoptable, printed, never a rung.** `forgeable` fails the lane's row; `not-established` is printed and blocks only `--require-closed`. Making `enforced` mandatory would exclude five lanes from every sealed type for a property the language cannot offer, while the other four obligations are as strong there as anywhere |
| OWN-3 | **A big-integer model.** Should Warrant gain an unbounded integer (or a model level for multi-limb arithmetic) so that RSAVP1 at real sizes is a model claim and not a table? | **Not for the pilot.** It is a language and eighteen-evaluator change for one adopter. The three layers of 14.3 (a full model on toy keys, three key-dependent fixed points at real sizes, pinned known answers) are honest about what each covers. Revisit when a second big-integer adopter appears, as a VER04r feature level |
| OWN-4 | **Pilot stage P-b and its lower chain.** P-b waits for `x509-rsa-public-key` to be adopted, and that package's maker plan calls into two further crates (the SPKI view and ASN.1 limits) that have no Warrant evidence; under OL1 they sit in the printed trust base of the type. Accept that for the pilot, or adopt those crates first? | **Accept it, printed.** The grounding claim tests the whole plan end to end on its domain, so the unverified crates are exercised, not assumed away, and the report's `tcb` line names them. Requiring the whole chain first delays the first foreign handle by three adoptions. Use the fallback (`x509-rsa-algorithm-policy`) only if the lower adoption slips past the O3 slice |

### 20.2 Resolved by the review (engineering)

| Was | Question | Resolution |
|---|---|---|
| OQ-3 | Nested makers (a handle built from a handle) | A maker is a plan of up to eight host steps (OL1), which covers construction through intermediate host values. A handle-typed maker **parameter** stays out of scope (section 18) |
| OQ-4 | An `iff` form of `admits` | Not added. `admits` stays sufficient; `rejection: complete` is computed from `admits` and `rejects` together (section 3.5), which gives the same information without a second form |
| OQ-5 | Is `Nat` wide enough for RSAVP1? | No at real sizes (`u64` today, `2^127` under draft VER04r), yes for moduli of at most four bytes; the pilot uses three layers (14.3). The remaining policy part is OWN-3 |
| OQ-6 | Minimum adequacy evidence | Per claim, over the refined domain: one class with two recipes, with the counts printed; `injective` is syntactic and single-maker (OV3) |
| OQ-7 | `rejected-at-use` in dynamic lanes | Only with a brand check named in the binding and a declared rejection seen by the probe (OL8, OV7) |
| OQ-8 | May sealing change a rung? | No (OB2, OT4, OF8). The adoption-policy part is OWN-2 |
| OQ-9 | Are stream tags `80`, `81` free? | Yes, against VER04i, VER04l and the drafts VER04r and VER04p (OC2); `8x` is reserved here |
| (15) | Are `W0750` to `W0753` free? | Yes, on `origin/main` and in both drafts; `W0750` to `W0759` reserved (section 15) |
| OQ-10 | A lane whose lower package has another observer set | `ContractInterfaceMismatch`; never adapted. Non-Rust lanes are `lane-not-supported` for foreign handles until VER29 has a reader there (OF9) |
| OQ-11 | Fixture ownership | One `opaque-fixture` port per lane, as a v2 corpus (conformance item 12) |

### 20.3 Deferred, with the reason

| # | Item | Why deferred, and what would reopen it |
|---|---|---|
| OQ-2 | Transformers (`T -> T`, `T -> Option<T>`) | The result's observations would need their own `means` over the argument's observations (a `maps` clause), and the result would be a handle no recipe names. Builder-style construction is already expressible as a plan (OR5). Reopen with two adopters whose operation cannot be restated as a maker |
| OQ-12 | Nested handles (in a `List`, record, `Option`, or as a maker parameter) | A recipe containing recipes needs a recursive descriptor and a rule for shared sub-handles (aliasing, OV8). Reopen with the collection types of VER04o |

## 21. Specification change record

* Initial VER04n (2026-11-16): normative WX-O: the semantic model (recipes, observers,
  `means`, `admits`, `rejects`); the surface syntax and typing rules; foreign handles
  through VER29 by restatement and pinning; handle domains, observation classes and
  five obligations (adequacy, immutability, unforgeability, freshness, grounding);
  the relation to VER04m; the corpus encoding (tags `80`, `81`); a binding vocabulary
  and a per-lane contract for eighteen lanes with one four-valued sealing vocabulary;
  nine sabotage controls (eleven after review); honest labels (a handle binder is never `full`); security
  considerations; the pilot `rsa-public-operation` in two stages. Departures from the
  design memo are N1 to N9 of section 1.2. New diagnostics `W0750` to `W0753`.
  Amends VER29 (3.1, 3.2, 3.3, 3.5, 5.1, 5.2, 5.3, 5.5, 6.2, 6.4, 13) and VER04m
  (3.5, 4.4, 6.1, 9.1, 14); no VER04, VER04h to VER04l or VER08b text changes, and no
  v0, v1 or other v2 meaning changes.
* **Adversarial review (2026-11-16): done; findings R1 to R24, all closed in the
  text.** Each line is the defect and where it is fixed.
  * R1 The encoding was written for the v1 corpus. Restated for `warrant-corpus-v2`
    (decision f): header, tags with counts, JSON form, level `ir.opaque`, faults as
    data, manifest fields, requests to VER04r (section 9).
  * R2 `D(T)` (filter, cap, round-robin) could not be enumerated in a lane by
    `nth`, and a cap applied to the first corner of the product could admit
    nothing. Now an explicit committed recipe list with a scan bound and an even
    spread, recomputed by the oracle (OV1, OC4).
  * R3 Observing a handle before the call hid lazy-initialisation faults. Witness and
    subject builds (OV5, N10).
  * R4 The text invited a claim over observers alone, which is evidence about the
    document's own models only. Refused by the existing no-operation rule (OK4).
  * R5 Adequacy was per type and `injective` could be read off a small domain or
    claimed with two makers. Per claim, counts printed, syntactic single-maker test
    (OV3).
  * R6 A type with one observer had no control on that observer, and no control
    covered a maker that accepts what `rejects` pins. `observer-drift` and
    `admit-rejected` (OS10, OS11).
  * R7 `rejection: partial` depended on a clause being present. Computed by
    evaluation (section 3.5, OB6).
  * R8 An upper layer could write its own plan for a foreign type and so assume more
    than the lower layer evidenced. The lower plan at a pinned binding digest (OF5,
    OF3).
  * R9 Interface matching ignored names and order, on which the maker index and
    the observation order depend. Canonical form (OF1).
  * R10 Nothing said an opaque contract is not a type invariant, and discharge did
    not read the lower controls or sealing status (OF8, OF4).
  * R11 `lower-fault` was `lower-fault-mode` in one place and `suite-failed` in
    another. Separated: lower controls are read by VER29 5.5; `lower-fault` is an
    upper control (17.1, 14.4).
  * R12 "A path outside the crate" is defeated by a re-export. The defining crate
    (OF6).
  * R13 The real lower maker takes host values of other crates, which the plan rule
    forbade, and four steps were too few. Eight steps, earlier ones in direct
    dependencies, printed in the trust base (OL1, N11).
  * R14 The binding had no `release` or `pass` although the security text relied on
    them; consuming operations were unaddressed (OL1a, OV5).
  * R15 A compile-fail probe can fail for the wrong reason, and a crash on a forged
    value read as a rejection. Probe pairs, diagnostic match, `forgeable (crash)`
    (OL2, OV7).
  * R16 An observer returning the handle's own storage is a mutation path.
    `observer-leak` (OL7).
  * R17 Caps contradicted each other (four handle parameters, two live handles)
    (OT5).
  * R18 A handle built in the unarmed warm-up could carry a capability into the
    armed phase (OT7); the adapter as a forging path (OT11).
  * R19 "ASan where available" let the c and cpp lanes run with no memory checker
    silently. Recorded and printed (OT8).
  * R20 `sampled` is not an LP-012 kind, and a `Nat` grounding claim is
    `bounded-set`; a cut handle list is `sample` (OB1).
  * R21 The pilot sketch was not well formed (a recipe literal in a claim, an
    undefined `is_error_per`, a quantifier over a table, "domain seed rows" that no
    specification defines). Rewritten with `vectors` and an extended OX1 (14.3).
  * R22 Stage P-a claimed operation-level controls although no operation takes
    `Limits` alone (14.2, 14.4, section 11).
  * R23 `Nat` width was left open. Settled, with a three-layer statement of the
    success value and an explicit account of what the table is (14.3).
  * R24 A host type could be described by two mechanisms, and a type with no
    observer fell between them (OK11, OR5).
  * Not this document's to fix, reported to the owner: VER04l (`60`, `61`), draft
    VER04r (`60`, `62`) and draft VER04p (`62`, `63`) allocate overlapping stream
    tags among themselves.
