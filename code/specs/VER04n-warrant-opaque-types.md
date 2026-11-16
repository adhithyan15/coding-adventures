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
adversarial review**: O1 does not start until the review's findings are closed
in the change record (section 21).

**Parent:** [VER04](VER04-warrant-language.md) (sections 2 to 8),
[VER04h](VER04h-warrant-derived-evidence.md) (domains, budget, manifests, controls),
[VER04i](VER04i-polyglot-warrant-evidence.md) (corpus, lanes, the eighteen-lane
order), [VER04j](VER04j-warrant-v1-extensions.md) (records, bindings, bounds),
[VER04k](VER04k-warrant-v1-wave4.md) (model references, drivers, `inout`, closed
binding vocabularies), [VER04l](VER04l-warrant-text.md) (the `v2` header, whose
rules this document reuses), [VER29](VER29-layer-contract-linking.md) (contract
ids and discharge: foreign handles are cited through it).

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
| two stream tags, the `corpus.json` section, the binding vocabulary | 9 |
| a lane binding contract for eighteen lanes, one sealing vocabulary for all | 10 |
| nine sabotage controls | 11 |
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
| N4 | `eq=observational`, opt-in (memo O-1) | **not in O1**. A claim over an explicit `op key_eq(a: K, b: K) -> Bool` states the same thing with no new feature (open question OQ-1) | a language feature whose meaning is "equal observations" adds nothing the observers do not already say, and `==` on a handle stays unconditionally an error, after the `len(t)` precedent (VER04l decision b) |
| N5 | `D(T)` is the makers applied to their argument domains, "kept when the result is `Ok`" | kept when `admits` holds, a **model-side** filter; the label counts `admitted of tried` (OV1) | the subject's own verdict cannot choose the domain it is then tested on |
| N6 | no rule for secret material | a maker parameter or observer result classed `secret` is refused (W0750, OT2) | recipes are written into a committed corpus (OC1) |
| N7 | "lanes without enforced privacy print not established" | one four-valued sealing vocabulary and a per-lane in-lane probe for all eighteen lanes (OL2) | Go's zero value, C# `default`, Kotlin `copy` and Python's `object.__new__` are forging paths in "private" lanes; the status must come from a probe, not from the language's reputation |
| N8 | `make` and `observe` rows "are ordinary `op` signatures" | they are **interface declarations**, not callable operations: a claim cannot call a maker, and an observer applied to a handle binder is the model function its `means` induce (OM4) | it removes the provenance question "is this observer call a model or a subject call?"; the subject's observers are tested by the derived grounding claim (OV6) |
| N9 | tags 80 and 81 | tags `80` and `81`, to be confirmed free by O1-S0 (the memo's float tags `60` and `61` are VER04l's text tags) | OC2 |

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
  argument tuple satisfying `Q` the maker MUST return `Err(v)` (or `None`).
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

### 3.5 What a maker failure means

A maker that returns an error for an argument tuple outside `admits` is not wrong:
the document did not forbid it. A maker that returns an error for an admitted tuple
fails the grounding claim with the failure `maker-rejected-admitted`. To pin a
rejection, add a `rejects` clause; the report prints `rejection: partial` when any
maker with a `Result` or `Option` result has no `rejects` clause (OB6).

## 4. Surface syntax

### 4.1 Grammar

```text
declaration  += opaque_decl
opaque_decl   = "opaque" NAME [ "sealed" ] [ "from" contract_id ] NEWLINE
                INDENT { maker_decl | observer_decl } DEDENT ;
maker_decl    = "make" NAME "(" [ params ] ")" "->" maker_result NEWLINE
                INDENT admits_row { rejects_row } means_row { means_row } DEDENT ;
admits_row    = "admits" expr NEWLINE ;
rejects_row   = "rejects" expr "with" expr NEWLINE ;
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
only as an argument of an operation call inside an `example` row (including as a
`domain` seed row), and only when `admits` holds for the arguments (else W0750,
checked by evaluation). It is not a value: it cannot be compared, projected or
bound. The reserved word `make` is unambiguous in expression position because it is
reserved (OC0a).

### 4.5 Restrictions

At most eight makers, eight observers and eight opaque types per document; at most
four opaque-typed parameters per `op`; a maker has at most eight parameters. Type
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
of an observer is a handle-typed binder (OM4).

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
validated by OF1 to OF4. The `from` clause is the document's contract link; it is
**not** also listed under `assume platform` (a duplicate is `ContractDuplicate`,
VER29 section 5.2).

**OK10.** (Operations over handles.) An `op` with at least one opaque-typed
parameter MUST NOT be `fresh`-classed or carry `inout` on a handle. All other op
forms (WX-A to WX-H, `labels`, `bound`, `output`) apply unchanged to the
non-handle parameters.

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
differ; the upper restatement must equal them.)

**OF1.** (Restate and match.) A `from` declaration restates the entire interface of
the lower type: every maker with its parameters, result, `admits`, `rejects` and
`means`, every observer, the `sealed` flag, and the models those clauses call. The
restatement MUST equal the lower declaration after normalisation (names, order of
declaration, comments and layout ignored). Inequality is `ContractInterfaceMismatch`
(a VER29 report error, section 17.1), raised by the resolver, never by the checker:
the checker validates shape only (VER29 section 3.3). `sealed` MUST match.

**OF2.** (Existence and publication.) A contract id `contract.<package>.opaque.<Type>`
exists only if the lower document declares `opaque <Type>` **without** `from`
(VER29 section 3.2 as amended). A `from` declaration is never itself published: no
re-export, so an interface has one owner.

**OF3.** (Pinning.) The report records the SHA-256 of the canonical normalised
interface text as `opaque_interface_digest` in the link's pins (VER29 section 5.3 as
amended), beside the lower manifest and report digests. A change to the lower
interface after the pin is a `ContractPinMismatch` at `--check`, as for any pin.

**OF4.** (Discharge.) The id is discharged when the lower report shows, at the pinned
digests and under the five conditions of VER29 section 5.1 (as amended): every
maker's derived grounding claim passed; every suite in the lower package that takes
the type passed its re-observation and freshness obligations; and no fault mode
applies (VER29 section 5.5). Until it is, the upper claims that quantify over the
type are open with the lower reason; the report never prints a strength for them
that the open link would not support.

**OF5.** (Building the handle above.) Upper evidence builds handles by calling the
real lower makers in the lane, through the binding of the *lower* package's lane
crate (OL1). There is no mock. The lower package MUST be a direct dependency in that
lane (VER29 `ContractNotDependency`). A lane that lacks the lower package records the
operation `not-live`, never another lane's evidence (VER29 section 3.4).

**OF6.** (No handle without a published lower contract.) If the lower package has no
adopted Warrant document, or its opaque id does not resolve, the upper document's
`from` clause is `ContractUnknownClaim`: an error, not an open assumption. The upper
crate waits (decision b). A local redeclaration of the same Rust type without `from`
is refused by the lane binding (W0753): a binding path that names a type outside the
declaring package's own crate is only allowed under `from`.

**OF7.** (Lower fault.) In every upper evidence run the adapter checks, for each
handle built from a foreign maker, that the real observers report the restated
`means` (the handle-sanity step of OV5). A mismatch is the failure
`lower-contract-violated: <T>.<observer>` and, under VER29 section 5.5, is attributed
to the lower layer. The control `lower-fault` (OS8) proves the attribution works.

## 7. Evidence: domains, obligations, and how the subject is checked unseen

### 7.1 Handle domains

**OV1.** (Domain.) `D(T)`, the handle domain, is built in this order:

1. for each maker in declaration order, enumerate its parameter domains (VER04h
   section 3; the ordinary `D(.)` of each parameter type, with the claim's
   refinements) in the canonical order;
2. keep the tuples for which `admits` evaluates true (a **model-side** filter, N5);
3. cap at `MAX_RECIPES_PER_MAKER = 256` and, per type, `MAX_RECIPES_PER_TYPE =
   1,024`, interleaving makers round-robin in declaration order when the cap binds,
   so no maker is starved.

The kind is **`bounded-set`**, never `full` (OB1). The label prints the makers and
the counts:

```text
bounded-set  handles(via limits_new: 64 of 65 admitted)
```

For `Nat` the VER04h domain is `0..=64`, 65 values; `admits max >= 1` keeps 64. The
same domain gives `rejects` its one tuple (`0`), which the grounding claim checks.

**OV2.** (Budget.) The product of binder domains, including `|D(T)|`, is charged to
the budget `B = 400,000` and the 64 MiB limit **before** enumeration (VER04h section
3); an over-budget claim is W0706. A handle contributes its recipe's domain bytes
(VER04j section 4.7).

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
wrong is vacuity, so the manifest records `classes`, `classes_with_2plus_members`
and `injective` per type, and:

* the claim's adequacy line is `adequate` when `classes_with_2plus_members >= 1`;
* `n/a (injective)` when every observation class has exactly one member by
  construction (each maker parameter appears as a bare observer `means`, as
  `limits_new`'s does), because then the recipe **is** its observation;
* `untested` otherwise, and the claim prints it (OB3).

**OV4.** (Distinguishing pairs.) For each observer `o` that the claim's model does
**not** mention, the generator checks that some pair of recipes in `D(T)` agrees on
every observer the model does mention and differs on `o`. If none exists the claim
prints `unseen-observer: o`: a subject that secretly depended on `o` could not have
been caught. (For the ByteSet claim above, `size` is not mentioned; the pair
`[0]` versus `[0, 1]` does not agree on `elements`, so no pair works, and
`unseen-observer: size` is printed. It is correct and cheap: `size` is a function
of `elements` and the document may say so with a claim.)

### 7.3 The case, walked through

One case of `contains_agrees` (recipe `from_list([1, 0, 1])`, `x = 0`), and what the
lane does:

```text
1. decode the recipe      maker=from_list, xs=[1,0,1]            (stream tag 80)
2. the generator already computed, with no subject:
     means elements = m_norm([1,0,1]) = [0,1]   size = 2
     expected result = m_has([0,1], 0) = true
3. the lane builds a FRESH handle through the real maker            (freshness)
4. handle-sanity: real elements(h) == [0,1]; real size(h) == 2      (grounding, per case)
5. the lane calls the real set_contains(h, 0)  and compares to true
6. re-observation: real elements(h) == [0,1], size(h) == 2 again    (immutability)
7. the lane drops h inside the armed phase                          (OT7)
```

The subject was never inspected: only called, through its public maker, observers and
operation. A failure at 4 is a maker or observer bug; at 5 an operation bug; at 6 a
mutation; at 3 a stale or shared handle.

### 7.4 The five obligations

Each obligation has a control (section 11) and appears in the manifest with its
own count line. An obligation that does not apply prints `n/a` and is never counted
as passed (OB5).

**OV5.** (Re-observation and sanity: `immutability`.) Every case performs steps 4
and 6 above. Step 4 is also `handle-sanity`. A type with a mutating method is not
opaque: it is a VER04m Mode P object (OR1).

**OV6.** (`grounding`.) For each maker the compiler derives, with no author text, the
claim

```text
forall <maker params> | admits(...).  maker(params) builds T with observers == means(params)
forall <maker params> | rejects_i(...). maker(params) == Err(v_i)
```

over the maker's own argument domain (not over handles). Its kind follows its
**binders**, so a total maker over a complete type, such as `Bool`, can be `full`
and `exhaustive` under the H3 rule; a `Nat` maker is `sampled`. It is published as
part of the type's contract (VER29 section 17.1). Without it an observer-relative
claim would be circular (N1); a claim over `T` whose type has no passed grounding
suite is **not publishable** and prints `ungrounded: <observer>`.

**OV7.** (`unforgeability`.) For a `sealed` type, independent export discovery (VER04
section 6) plus the lane's in-lane probe (OL2) yields exactly one of four statuses,
printed per lane and never merged across lanes:

| Status | Meaning |
|---|---|
| `enforced` | the language or linker makes a public path to `T` other than the makers impossible, and the probe (a compile-fail or link-fail test, or a header inspection) confirmed it |
| `rejected-at-use` | a forged value can be made, but every operation taking `T` rejects it (a brand or validity check), and the probe confirmed it |
| `forgeable` | the probe made a value that an operation accepted |
| `not-established` | the lane has no probe for this binding |

`forgeable` fails the `unforgeability` obligation (the lane's claim row for `sealed`
types is `fail`); `not-established` is printed and does not fail. No status changes
a rung (OB2): sealing is a property of the host, printed, never proved here.

**OV8.** (`freshness`.) A handle is built fresh for each case and dropped inside the
armed phase. The runner never reuses a built handle across cases (the `stale-handle`
fault must fail, OS2), and a handle with shared backing (a borrowed buffer, a cached
instance) is rebuilt from the recipe.

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

## 9. Corpus and encoding

### 9.1 Streams

**OC1.** (Recipes, not bytes.) An opaque **input** is its recipe: tag `80`, a u32
little-endian maker index (declaration order), then the encoded arguments in
parameter order using the existing encodings (VER04i section 2.6). An opaque
**maker result** is hashed in the grounding suite as its observation tuple: tag
`81`, then each observer value encoded in declaration order. **No host byte of a
handle ever enters the corpus**, so the corpus is language-independent and a
foreign handle's layout is irrelevant.

**OC2.** (Tags.) `80` and `81` are the next free tags after those of VER04i section
2.6 (`00 01 10 20 30 40 41 50 51`) and VER04l (`60 61`); O1-S0 confirms they are
still free against every specification then merged (N9). A lane decoder MUST reject
an unknown tag.

**OC3.** (TFV.) A handle in `cases.tfv` is a map
`M{make: S<name>, args: L[...]}`, since TFV maps have string keys (TF01 section 2).

### 9.2 `corpus.json`

```json
"opaque": { "Limits": {
  "sealed": true, "from": null, "interface_digest": "<sha256 hex>",
  "makers": [ { "index": 0, "name": "limits_new",
                "params": [ {"name": "max_modulus_bytes", "type": "Nat"} ],
                "admits": "<model IR>", "rejects": [ { "when": "<IR>", "with": "<IR>" } ],
                "means": { "maximum_modulus_bytes": "<IR>" } } ],
  "observers": [ { "name": "maximum_modulus_bytes", "type": "Nat" } ],
  "classes": 64, "classes_with_2plus_members": 0, "injective": true } },
"ops": { "recover": { "params": { "k": {"opaque": "ValidatedRsaPublicKey"},
                                    "l": {"opaque": "Limits"} } } }
```

The model IR is the ordinary VER04i IR; every lane evaluates `admits` and `means`
with the evaluator it already has. A `corpus.json` whose `opaque` section differs in
digest from the document is the corpus error `OpaqueDigest`; unknown fields are
errors, as everywhere.

### 9.3 Digests

The inputs stream hashes recipes (tag 80); the expected stream hashes the model's
results; the grounding suite's expected stream hashes tag-81 tuples. Cross-lane
agreement on these digests is therefore also an agreement on every observer's
meaning in every lane that runs.

## 10. Lane binding contract

### 10.1 The binding vocabulary

A per-package binding gains an `opaque` section, closed like every binding (unknown
fields are errors, W0501). In the line form of VER04j section 14:

```text
opaque Limits type=rsa_public_operation::RsaPublicOperationLimits sealed
make    limits_new plan=fn:RsaPublicOperationLimits::new(param.max_modulus_bytes) result=result
observe maximum_modulus_bytes method=maximum_modulus_bytes receiver=value
```

and in JSON:

```json
"opaque": { "Limits": {
  "type": "rsa_public_operation::RsaPublicOperationLimits", "sealed": true,
  "makers": { "limits_new": {
      "plan": [ { "fn": "RsaPublicOperationLimits::new", "args": ["param.max_modulus_bytes"] } ],
      "result": "result" } },
  "observers": { "maximum_modulus_bytes": { "method": "maximum_modulus_bytes", "receiver": "value" } },
  "backing": null } }
```

**OL1.** (The maker plan.) A maker's `plan` is a list of **1 to 4 steps**; each is
one host function or associated function path (an identifier grammar, never
escaped, VER04k section 6.3), with arguments drawn from a closed vocabulary:
`param.<name>`, an integer, bool or byte literal, `step.<k>` (the host value of an
earlier step) and `model.<fn>(...)`, the value of a first-order document model
evaluated by the lane evaluator. The last step's value is the handle (or the
`Result` or `Option` of it); the lane's normal error normalisation applies to it
(VER04i section 5, the per-lane error models). `model.<fn>` is what lets a maker take semantic parameters
(`modulus`, `exponent`) while the host takes an encoded form (a DER string built by
a document model): the adapter does not grow an encoder per lane, it reuses the
evaluator. `backing` names the step that owns buffers the handle borrows (Rust
`'a`, a Go slice, a Swift `Data`) and keeps them alive until the case ends.
A path outside the declaring package's own crate is W0753, except for a `from` type,
where it MUST be inside the lower package's lane crate (OF5, OF6).

**OL2.** (Sealing probe, one vocabulary for every lane.) Each lane supplies an
in-lane probe producing one of the four statuses of OV7, executed by the lane
runner; the status appears in the lane's manifest and report header. The probe is a
generated source file the runner tries to build or call from **outside** the
declaring module. A lane may not report `enforced` without running it.

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
| c | pointer to an **incomplete struct type** declared in the public header; a create function and a `free` function | leak, double free, use after free: the adapter owns create and free, frees each handle exactly once, and the `stale-handle` fault keeps the previous handle **live** until the next case ends (it never uses freed memory) | header inspection (no complete definition public) and a compile-fail of `sizeof(T)`: `enforced`; ASan where available |
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
(unsigned 64-bit discipline in Java and Kotlin, arbitrary precision masking in
Python, Ruby and Elixir). Nothing about handles changes it.

## 11. Sabotage controls (faults)

A fault is a deliberately wrong subject variant. "Caught" means a named obligation
or claim fails on it. Every fault below that applies to an operation MUST be listed
in its `faults=` row, else W0502; a fault that does not apply is W0701 (section 15).
The controls are part of the manifest's `negctl` entries (VER04h section 6).

| # | Fault | The wrong subject | Must be caught by |
|---|---|---|---|
| OS1 | `ignore-handle` | the operation always uses the **first** handle built | the operation claim, on any recipe whose observation differs from the first |
| OS2 | `stale-handle` | the operation receives the **previous case's** handle | `freshness`, and the operation claim |
| OS3 | `swap-observer` | two observers return each other's values | `grounding` and the handle-sanity step; requires at least two observers that differ on some recipe (a type with one observer, or observers equal on every recipe, prints `swap-observer: n/a` as a vacuity finding) |
| OS4 | `mutate-handle` | an operation changes a field an observer reports | `immutability` (re-observation) |
| OS5 | `class-split` | the operation reads a hidden field that differs between recipes of one observation class | the operation claim over a class with two or more members; if the type has none the fault prints `n/a (no multi-member class)` |
| OS6 | `forge` | a binding row names a maker or observer the document does not declare | rejected at **load** (W0753), before any case runs; this control tests the loader, not the subject |
| OS7 | `zero-handle` | an operation accepts a default or zero value of the host type | `unforgeability` (the status becomes `forgeable`); applies only to lanes with a zero value (go, csharp structs) |
| OS8 | `lower-fault` | the cited lower maker is sabotaged (observers drift) | `handle-sanity` fails with `lower-contract-violated`, and VER29 section 5.5 attributes the failure to the **lower** layer, not to the upper one |
| OS9 | `admit-flip` | the maker rejects a recipe `admits` accepts | `grounding` with `maker-rejected-admitted` |

OS3 and OS5 each have a vacuity condition (they cannot fail on a type with no
second observer or no multi-member class); the report prints the condition rather
than a pass, so a control never silently counts as caught.

## 12. Honest labels, counting and vacuity guards

### 12.1 Labels

**OB1.** (Kinds.) A claim whose binders include a handle binder has the kind
**`bounded-set`** at best: the kind of a mixed claim is the weakest among its
binders (`sampled` below `bounded-set` below `full`, LP-012). A handle binder is
never `full`: the domain is the image of the makers over a stated argument domain,
and "the image of a maker is not a type"; completeness would additionally need
adequacy and unforgeability, both tested here and neither proved.

**OB2.** (Rungs.) The VER29 rung of a handle claim is at most `exhaustive-sampled`.
A grounding claim takes the kind of its **maker's argument binders** (OV6): over a
complete argument type it may be `full` and `exhaustive`, under the H3 rule of
VER04m E14 and VER29 section 6.2. The unforgeability status, the sanity and
freshness gates, and `adequacy: ...` are **printed annotations**, never rungs.
An operation over a foreign handle is at most the minimum of its own strength and
the lower opaque contract's (VER29 section 6.3 as amended).

| Claim form | Example | Kind | VER29 rung |
|---|---|---|---|
| operation versus model over `forall h: T` | `contains_agrees` | `bounded-set  handles(via from_list: 7 of 7 admitted)` | `exhaustive-sampled` |
| operation versus model over `forall h: T, x: U8` | the same, 7 x 256 = 1,792 cases | `bounded-set` | `exhaustive-sampled` |
| derived grounding claim over a complete argument type | a `Bool`-parameter maker | `full` | `exhaustive` (H3 satisfied) |
| derived grounding claim over `Nat` | `limits_new` | `sampled` | `exhaustive-sampled` |
| an operation over a foreign handle | `recover` over a `ValidatedRsaPublicKey` | per binders | at most the lower contract's rung |
| a claim whose type has no passed grounding suite | any | n/a | not publishable (`ungrounded`) |
| `property` and `fuzz` over handles | recipes generated | `sample` | `randomized` |

**OB3.** (Printed annotations.) A handle claim prints, beside its label, one
`adequacy` line (`adequate`, `n/a (injective)` or `untested`), the
`unseen-observer` list (OV4), the sealing status per lane (OV7), and
`rejection: partial` where applicable. `adequacy: untested` is a valid published
state at `exhaustive-sampled`; it makes `--require-closed` fail for the link that
rests on it (VER29 section 5.6), and never a stronger label.

### 12.2 The report block (sketch)

```text
opaque Limits  sealed   makers: limits_new     observers: maximum_modulus_bytes
  grounding            sampled   exhaustive-sampled   cases=65  rejection: complete
  adequacy:            n/a (injective: every maker parameter is an observer)
  unforgeability:      rust enforced   python not-established   go rejected-at-use   ...
op recover  (handles: ValidatedRsaPublicKey from contract.x509-rsa-public-key.opaque..., Limits)
  recover_matches_model bounded-set  exhaustive-sampled
        handles(via key_from_parts: 4 of 6 admitted; via limits_new: 64 of 65 admitted)
        cases=4*6*64=1,536   adequacy: n/a (injective)   unseen-observer: none
  obligations: immutability pass (control ok)  freshness pass (control ok)
               handle-sanity pass (lower contract assumed: x509-rsa-public-key, discharged)
  tcb: makers' means and admits (restated, pinned), bindings, adapter plan
```

### 12.3 Vacuity guards

| # | Failure | Prevention |
|---|---|---|
| OB4 | **An empty or tiny handle domain proves nothing**: every recipe rejected, or one recipe tried | zero admitted recipes is W0601; the label prints `admitted of tried`; a type whose `D(T)` has one recipe prints `handles: 1` and the claim cannot be called anything but a single example |
| OB5 | **A check that did not run, counted as passed** | an obligation that does not apply prints `n/a`; a not-live lane never inherits a pass (OL3); every control prints its vacuity condition (section 11) |
| OB6 | **Silent partial rejection**: a `Result` maker with no `rejects` clause | `rejection: partial` is always printed |
| OB7 | **Circular observers**: expected values taken from the subject | `means` evaluated by the model, never the subject (OM4); `ungrounded` blocks publication (OV6) |
| OB8 | **A claim that cannot tell an ignored observation from an unvaried one** | `unseen-observer` (OV4) |
| OB9 | **Sealing implied by the language's reputation** | one four-valued probe per lane (OV7, OL2); `forgeable` fails |
| OB10 | **A lower fault blamed on the upper layer, or hidden by it** | handle-sanity and `lower-fault` (OF7, OS8) |
| OB11 | **A stateful type passed off as immutable** | `immutability` (OV5) |
| OB12 | **The report implies "the handle proves validation happened"** | OT4: sealing is a host property, printed per lane, never a rung |

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
per document; `MAX_RECIPES_PER_MAKER = 256`, `MAX_RECIPES_PER_TYPE = 1,024`; at most
4 handle parameters per operation and 2 live handles per case plus those of
constructors; a maker plan of at most 4 steps; at most 16 observer calls per case;
`admits`, `rejects`, `means` and `model.<fn>` plan arguments are charged to the
enclosing meter (VER04k section 2.7); `B = 400,000` and 64 MiB over the product of
binder domains including `|D(T)|`, enforced before enumeration.

**OT6.** **Pins and hand-edited lower evidence.** A foreign handle's soundness rests
on the lower manifest, the lower report and the interface digest (OF3). A hand-edited
lower manifest is what the digests and VER29 condition (c) are for.

**OT7.** **The runtime jail stays mandatory** (owner decision d). Makers, observers
and operations run inside the armed phase of VER08b. A maker that needs a capability
(`fs`, `net`, `proc`, `time`, `rand`, `env`) is stopped by the gate, and the type is
`not-live` for the runtime rung until the capability is allowed, never run unarmed.
Handles over OS resources are out of scope for exactly this reason. Destructors run
inside the armed phase (OL4, VER04m J2). The warm-up and order-independence rules of
VER04m J4 and J6 apply: one unrecorded warm-up case per operation in phase A, and
`order-independence` runs armed.

**OT8.** **Memory safety in C and C++.** The handle-specific risks (double free,
use after free, a leaked handle) are the adapter's, so ownership is explicit in the
binding (`free`), each handle is released exactly once, and the `stale-handle` fault
never touches freed memory (OL2 table). ASan or an equivalent runs where available.

**OT9.** **Trust base.** The restated interface (and its digest), the `admits` and
`means` models, the binding plan, the `model.<fn>` host arguments and the adapter
are recorded in the report's `tcb` section. `means` is the one *declared* modelling
claim in this design and is printed as declared.

**OT10.** **Not an erasure or timing argument.** Re-observation makes writes
observable; it says nothing about dead-store elimination of secrets, nor about
timing.

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
is closed and redacted.

### 14.2 Staging

* **Stage P-a (slice O1, local only).** Adopt only the **local** type. A document
  with `opaque Limits sealed` (section 4.3), its derived grounding claim (65 cases
  over `Nat` `0..=64`, 64 admitted and 1 rejected, `n/a (injective)`), the sealing
  probe, and the four faults that apply to a single-observer type (`ignore-handle`,
  `stale-handle`, `mutate-handle`, `forge`). This is the O1 acceptance test and needs
  no VER29 change.
* **Stage P-b (slice O3 and O4, foreign handle).** The key handle, cited through
  `contract.x509-rsa-public-key.opaque.ValidatedRsaPublicKey`. **Blocked until
  `x509-rsa-public-key` has an adopted Warrant document and the VER29 slice that
  publishes the opaque id has landed** (decision b, OF6). The fallback pilot, if the
  lower adoption is late, is `x509-rsa-algorithm-policy` (6 dependents); it is
  recorded so the schedule does not stall, not chosen here.

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

claim recover_error_paths:
  forall k: ValidatedRsaPublicKey, sig: Bytes | one_of_sigs(sig), l: Limits.
    recover(k, sig, l) is_error_per
      m_error_path(modulus(k), sig, maximum_modulus_bytes(l))

claim recover_known_answers:
  forall r: KnownRow in rsavp1_table.
    recover(make key_from_parts(r.n, r.e), r.sig, make limits_new(r.cap)) == Ok(r.expected)
```

`recover_error_paths` is the length and range logic: modulus longer than the limit,
signature length different from the modulus length, representative not below the
modulus. All three are expressible as byte-wise models. The success value is
modular exponentiation over arbitrary-size integers. Whether Warrant's evaluated
`Nat` is wide enough for a full `m_recover` model is **not settled by VER04d**
(open question OQ-5); the pilot therefore states the value through a pinned
table of known-answer rows (WX-E, VER04j) generated by an independent tool, and
promotes it to a `m_recover` model claim only if a bignum model is admitted.
Counts for the toy domain used in the sketch are `4 keys x 6 signatures x 64 limits
= 1,536` cases, computed by multiplying the three binder domains; the generator, not
this document, fixes the real sets.

### 14.4 Acceptance

* Stage P-a: the Limits type yields 65 grounding cases, `rejection: complete`,
  `adequacy: n/a (injective)`, and all four applicable faults are caught; the
  Rust sealing probe prints `enforced`.
* Stage P-b: `recover_error_paths` prints the foreign handle's label,
  `handle-sanity: lower contract assumed, discharged`; sabotaging the lower maker
  (`lower-fault`) turns the upper link open with `suite-failed` attributed to
  `x509-rsa-public-key`; a mismatched restated interface is
  `ContractInterfaceMismatch`.
* **The design is wrong** if `recover` cannot be stated over a handle it did not
  build (the key) while binding the real lower maker with no mock.

## 15. Diagnostics

Existing codes keep their meaning and are reused wherever the condition is one they
already name. Four codes are new. `W0750` to `W0753` were checked unused in every
specification on `origin/main` (the highest code in use is `W0733`) and are a
reserved block for this document; if another document allocates the same numbers
first, the later renumbers at merge.

| ID | Slice | Phase | Trigger |
|---|---|---|---|
| **W0750** | O1 | checking | an opaque declaration rule: no observer, no maker, a `means` missing, duplicated or of the wrong type, `admits` absent, an impure or ill-typed clause, an overlap of `admits` and `rejects`, a recipe literal that violates `admits`, a secret-class parameter or observer (OK6), a type parameter, a count over the caps of section 4.5, an observer result containing an opaque type |
| **W0751** | O1 | checking | a forbidden use of an opaque value (OK2, OK3, OK7): comparison, pattern, nesting, a `model` signature, an `inout` handle, a handle in an op result, an unsupported binder refinement |
| **W0752** | O3 | checking | a `from` clause that is malformed (not a 4-segment `opaque` id), a `from` in an O1 build, a `sealed` mismatch against the id, or a restated interface not shaped like a declaration |
| **W0753** | O1/O3 | generation | a binding against the checked document: a maker `plan` inconsistent with the declaration (arity, type, more than 4 steps, a path outside the package's crate, or outside the lower crate under `from`), an observer missing or extra, a row naming a maker or observer the document does not declare (the `forge` control), a `sealed` flag that differs from the declaration |

Reused: **W0101** (a v2 construct in a v0 or v1 document, OC0), **W0501** (a
malformed `opaque` binding entry), **W0502** (a `faults=` list that omits an
applicable fault of section 11), **W0601** (a handle domain with zero admitted
recipes), **W0701** (a fault that does not apply, a lane with no opaque adapter),
**W0706** (a handle domain over budget), and the VER29 errors
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

4. Each rule OK1 to OK10 has a failing example producing its named diagnostic.
5. The ByteSet example yields 7 recipes, 4 classes, 3 multi-member classes; the
   ByteSet claim `contains_agrees` yields 7 x 256 = 1,792 cases (OV3, 12.1).
6. `limits_new` yields 65 grounding cases, 64 admitted and one rejected, `injective`
   (OV1, OV3).
7. `ignore-handle`, `stale-handle`, `swap-observer`, `mutate-handle`,
   `class-split`, `forge` and `admit-flip` are each caught on the ByteSet fixture;
   `swap-observer` and `class-split` print their vacuity condition on `Limits`
   (section 11).
8. A claim over a type with no passed grounding suite prints `ungrounded` and is
   not published (OV6).
9. A maker with a secret-class parameter is W0750 (OK6, OT2).
10. The Rust lane's sealing probe prints `enforced` for a private-field struct and
    `forgeable` for a struct with public fields (the control, OV7).
11. The pilot Stage P-a of section 14.4.

**O2 (per lane).**

12. The lane passes the shared fixture (`opaque-fixture`, ByteSet and Limits) with
    identical digests on the inputs and expected streams as the Rust reference
    (OC3, section 9.3).
13. The lane implements all nine controls it is applicable to and the sealing probe,
    and prints one of the four statuses (OL2, OL3).
14. The per-lane hazard row of 10.2 is exercised by a test that fails when the hazard
    is introduced (zero value, `copy`, `__new__`, `allocate`, `bless`, a hand-built
    map, a literal as `unknown`).

**O3.**

15. `ContractInterfaceMismatch`, `ContractUnknownClaim` for a lower package with no
    Warrant, and `ContractNotDependency` each have a failing example (OF1, OF5, OF6).
16. `lower-fault` is attributed to the lower layer (OF7, OS8).
17. A restated interface that differs in one `means` expression changes the digest
    and fails `--check` (OF3).

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
  applied to each member.
* **Section 5.2, errors.** Add `ContractInterfaceMismatch`: the restated interface
  after `from` differs from the lower declaration, or its `sealed` flag does.
  (No new open reason: a failing member surfaces as `suite-failed`.)
* **Section 5.3, pins.** Add `opaque_interface_digest` per `from` link (OF3).
* **Section 5.5 item 3.** The negative control for an opaque contract is
  `lower-fault` (VER04n OS8); a verdict other than `pass` makes the contract open
  with `lower-fault-mode`.
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
  return a handle (section 20, OQ-2).
* **Nested handles**: a handle inside a `List`, record, variant, `Option`, `Set`,
  `Map`, or as a maker argument; and a handle field of a Mode E state record.
* **Generic opaque types** (`HashSet<T>`): VER04o amends section 4.5.
* **Handles over OS resources** (files, sockets, threads, processes): the jail
  forbids them (OT7).
* **Secret-bearing handles**: private keys, MAC keys, key schedules (OK6, OT2).
* **Identity equality**, address equality, hash codes of handles.
* **Drop semantics as behaviour** (only that destructors run armed, OL4); `dyn Trait`
  objects; stored callbacks; serialised handles; handles sent across threads.
* **`eq=observational`** (N4, OQ-1).
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
| **O1-S0** | errata pass against the then-current VER04j, k, l, m and VER29; confirm tags `80` and `81` and codes `W0750` to `W0753`; accept `opaque`-gated constructs in the `v2` header parser | O0 reviewed; T1-S0 (`v2` accepted) |
| **O1-S1** | checker and ground evaluator: declarations, rules OK1 to OK10, `admits` and `means` evaluation, domains, classes, injectivity, recipe literals | O1-S0 |
| **O1-S2** | evidence generator: `corpus.json` section, tags 80 and 81, derived grounding claims, five obligations, labels, the nine controls, report block | O1-S1; LP-012 count split |
| **O1-S3** | Rust lane runner: binding reader (`opaque` section, plans, `backing`), sealing probe, the shared fixture `opaque-fixture` in Rust, pilot **Stage P-a** | O1-S2; the shared-crate queue of VER30 section 5.3 |
| **O2a** | lanes python, typescript, go (the VER04i order): runner, fixture port, probe, controls, hazard-row test | O1-S3; a lane may not skip ahead (VER04i) |
| **O2b** | lanes ruby, c, cpp, java, kotlin, csharp, fsharp, elixir | O2a |
| **O2c** | lanes lua, perl, haskell, swift, dart, ocaml | O2b; toolchains in CI first, as VER04i requires |
| **O3** | foreign handles: the VER29 slice 6 of 17.1, the `from` checker rule, the interface digest, `lower-fault`; the O3-checker work of OF1 to OF7 | O1-S3; VER29 slice 2 (resolution and discharge); the lower package adopted |
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

| # | Question | Default recorded here |
|---|---|---|
| OQ-1 | Should an opaque type be able to declare `eq=observational` so that `==` means equal observations? | No, not in O1 (N4); write an explicit `key_eq` operation and claim |
| OQ-2 | Transformers (`T -> T`): how are the result's observations specified? | Out of scope for O1. The candidate is a `maps` clause on an operation giving the result's `means` from its arguments and the argument's observations; needs two adopters |
| OQ-3 | Nested makers (a handle built from a handle) | Not in O1; the foreign handle crosses the boundary as a pinned **recipe**, not as an argument |
| OQ-4 | May `admits` be made necessary as well as sufficient (an `iff` form) to retire `rejection: partial`? | Keep sufficient-only; print `rejection: partial` (OB6). Revisit with a second adopter |
| OQ-5 | Is Warrant's evaluated `Nat` wide enough for a modular-exponentiation model (RSAVP1)? | Unverified. The pilot uses a pinned known-answer table (14.3) and promotes to a model only if admitted |
| OQ-6 | Minimum adequacy evidence for a non-injective type | At least one class with two members, else `adequacy: untested` and `--require-closed` fails (OB3) |
| OQ-7 | Should a dynamic lane be able to reach `rejected-at-use` only by a declared brand check in the binding? | Yes: the probe must see the rejection; a lane without one prints `not-established` |
| OQ-8 | May the sealing status ever raise or lower a VER29 rung? | No (OB2, OT4); revisit once an upper layer's soundness argument depends on it (the X.509 chain) |
| OQ-9 | Are the stream tags `80`, `81` free? | Believed free; O1-S0 confirms (N9) |
| OQ-10 | Interface identity across a lane that has the lower package but a different observer set | A lane whose lower package disagrees with the pinned interface fails the link with `ContractInterfaceMismatch`; it is not adapted |
| OQ-11 | Fixture ownership: one `opaque-fixture` per lane versus reusing real adopters | Per lane, as VER04i did with `ct-compare` ports; reconsider after wave O2a |

## 21. Specification change record

* Initial VER04n (2026-11-16): normative WX-O: the semantic model (recipes, observers,
  `means`, `admits`, `rejects`); the surface syntax and typing rules; foreign handles
  through VER29 by restatement and pinning; handle domains, observation classes and
  five obligations (adequacy, immutability, unforgeability, freshness, grounding);
  the relation to VER04m; the corpus encoding (tags `80`, `81`); a binding vocabulary
  and a per-lane contract for eighteen lanes with one four-valued sealing vocabulary;
  nine sabotage controls; honest labels (a handle binder is never `full`); security
  considerations; the pilot `rsa-public-operation` in two stages. Departures from the
  design memo are N1 to N9 of section 1.2. New diagnostics `W0750` to `W0753`.
  Amends VER29 (3.1, 3.2, 3.3, 3.5, 5.1, 5.2, 5.3, 5.5, 6.2, 6.4, 13) and VER04m
  (3.5, 4.4, 6.1, 9.1, 14); no VER04, VER04h to VER04l or VER08b text changes, and no
  v0, v1 or other v2 meaning changes.
* **Adversarial review: pending.** Findings and their resolutions are recorded here
  before O1-S0 starts.
