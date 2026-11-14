# VER04m — Warrant Stateful Objects (WX-S)

**Status:** Specification only. Nothing here is implemented and no code changes
with it. This is the WX-S0 deliverable of the backlog: the normative text that
WX-S1 (explicit state), WX-S2 (protocols) and WX-S3 (deferred) implement, in that
order. Owner decisions of 2026-11-14 are recorded in section 1 and applied
throughout. The Warrant fragments in sections 5 and 6 are **sketches**: none has
been run through the checker, and the case counts in them were computed by hand
from the stated domains (each is re-derived in the text so a reviewer can redo it).
Claims about what a crate does are "from reading": the source was read, not run.

**Parent:** [VER04](VER04-warrant-language.md) (sections 3 to 8),
[VER04h](VER04h-warrant-derived-evidence.md) (domains, budget, manifests, controls),
[VER04i](VER04i-polyglot-warrant-evidence.md) (corpus, lanes),
[VER04j](VER04j-warrant-v1-extensions.md) (v1 header, records and variants, bounds),
[VER04k](VER04k-warrant-v1-wave4.md) (WX-D, WX-F drivers in section 3, WX-G `inout`
and `ensures` in section 4).

**Amends (in the same commit series, see section 13):**
[VER29](VER29-layer-contract-linking.md) sections 6.1, 6.2, 14 and decision D26 (the
H3 rewording) and [VER04k](VER04k-warrant-v1-wave4.md) sections 4.3, 4.6 and 4.12
(lists of records, three host forms, one slot clarification).

**Related:** [VER08b](VER08b-runtime-capability-conformance.md) (the runtime jail,
section 9 below), [VER30](VER30-warrant-expansion-roadmap.md) (section 4.3 states the
requirements this document answers; section 4.5 the 4004 chain), the sibling
specification for text, VER04l (this document uses its alphabet-domain predicate
`within` in section 6 only; section 6.3 says what happens if VER04l is not yet
merged), backlog rows WX-S0 to WX-S3 in [VER-BACKLOG](VER-BACKLOG.md).

**Design source:** an advisory design memo (not committed), Part B and Parts C and D.
Where this document departs from it, section 1.2 says so and why.

## 0. Why this exists

Warrant documents are pure: an `op` is a function from arguments to a result
(VER04 section 3). A large part of the repository is not shaped like that. The
VER30 inventory counts 268 crates whose primary shape is a `&mut self` method
protocol, and the Intel 4004 chain (VER29 section 12, VER30 section 4.5) is blocked
at its second layer, the sequential logic of `logic-gates`, by the same shape:

| Shape | Real API (from reading) | What a one-shot `op` cannot say |
|---|---|---|
| state passed in and out | `logic_gates::sequential::d_flip_flop(data, clock, &mut FlipFlopState) -> (u8, u8)`; `register(&[u8], u8, &mut [FlipFlopState]) -> Vec<u8>`; `PixelContainer::set_pixel(&mut self, ...)` over public fields; `Intel4004Simulator::step(&mut self) -> Result<Intel4004Trace, Intel4004Error>` | "from every state and every input, one step equals this model", and "every error leaves the state unchanged" |
| private state, method protocol | `irc_framing::Framer::{feed, frames, reset, buffer_size}`; `lz78::TrieCursor`; `block_ram::SinglePortRAM::{tick, dump}`; `der_tlv::DerCursor` | "any sequence of calls agrees with this model", and "the object does not grow without limit" |

The design keeps the one principle of VER04k section 3 and 4: **the document stays
pure.** A stateful object is described by pure functions; the binding turns method
calls into those functions. No mutation, no `old()`, no state enters the language.

Two modes, distinguished by one question: is the object's whole mutable state a
Warrant value that the harness can both build and read?

* **Mode E, explicit state** (yes). The method is an `op` with an `inout` state
  parameter, exactly as in WX-G. The harness builds a host object from any state
  value, calls the method, reads the state back. Section 4.
* **Mode P, protocol** (no). The method calls are the elements of a `List<Call>`;
  a binding-level driver (WX-F) dispatches each `Call` constructor to a method and
  collects the replies, so the object's trace function is an ordinary `op`.
  Section 6.

### 0.1 Slices, and how rules are marked

Every normative rule in this document carries a prefix that names its slice:

| Prefix | Slice | Content | Sections |
|---|---|---|---|
| `E1`, `E2`, ... | **S1** | explicit state through WX-G `inout`: `state=explicit`, invariants and reach classes, the `codec` line, `omit=`, domains, the four state-completeness obligations, frame faults, the H3 rule | 4, 5 |
| `P1`, `P2`, ... | **S2** | protocols: `dispatch` drivers on a `Call` variant, the `abs` observer, trace domains, protocol obligations and faults, the `retained` bound | 6 |
| `D1`, `D2`, ... | **S3** | deferred: the `object` block, the derived inductive row, functional-equivalence proofs, published state facts | 7 |
| `L1`, `L2`, ... | all | labels and counting (section 8.1 and 8.2) | 8 |
| `V1`, `V2`, ... | all | vacuity guards (section 8.3) | 8 |
| `J1`, `J2`, ... | all | runtime jail and lane rules (section 9) | 9 |

A rule marked `E` binds an implementation of S1 and nothing else; an S1
implementation MUST NOT be required to understand a `P` rule, and a document that
uses a `dispatch` driver or `retained` row before S2 is refused (W0731, W0733).
The rules of section 8 apply to whichever slices are implemented.

Technical prerequisites. S1 needs WX-G (VER04k section 4) and the merged WX-D1
sequence constructors; it needs no text. It needs WX-D2 only where a document states a
list-state invariant with `all` (the register example of 5.1), which the owner's order
lands first; `d_flip_flop`, `pixel-container` and the CPU step need no `fold`. S2 needs
WX-F (VER04k section 3), WX-D2 (`fold`, for the model's `run`) and the alphabet-domain predicate
(section 6.3). The owner's order (decision b) places WX-D2, WX-F and the LP-012 case
count split before S1 as scheduling; only WX-G is a technical prerequisite of S1,
and LP-012's split is a hard prerequisite of *shipping* either slice, because
without per-claim kinds a family or trace suite would be counted as exhaustive
(L1, V2).

## 1. Decisions of 2026-11-14, and departures from the memo

### 1.1 Decisions

| # | Decision | Applied in |
|---|---|---|
| a | Two specifications: VER04l (text) and VER04m (stateful objects) | this document; VER04l is separate |
| b | Order: implement WX-G, WX-D2, WX-F and LP-012's count split first; then **S1** (explicit state); then text T1; then **S2** (protocols); then **S3** | section 0.1, backlog |
| c | H3 is reworded to "**pure in its explicit state**". `exhaustive` is allowed ONLY for a step claim whose state binder is complete and whose binding passes the hidden-state obligations. A bounded set of call sequences is never `exhaustive`; it is `bounded-set`, with the depth and call alphabet printed | E14, L2, section 13.1 |
| d1 | State injection is accepted, with invariant-or-total per claim | E5 to E8 |
| d2 | The `codec` binding line is added in S1 | E11 |
| d3 | The derived inductive row and the `object` block are deferred to S3 | section 7 |
| d4 | A `retained` bound is added in S2 | P12 |
| d5 | `omit=` declared omissions are allowed and always printed | E12 |
| d6 | Proofs are not-live initially (`proofs_not_live=state` or `driver`); the Mode E functional-equivalence obligation is funded next | E19, section 7.3, backlog WX-SP1 |
| d7 | The jail plan is `rcg-baseline-2` (measure `mremap`), plus the other jail items | section 9 |
| d8 | The inventory's figure for the two features is an upper bound: re-measure with "explicit state", "protocol" and "text-only" as separate tags before scheduling adopters | O-8, backlog WX-S0 |

Decision (c) in plain words, because it is the one with consequences for every
layer above: the only way a stateful operation earns `exhaustive` is by enumerating
its **transition relation** over a state set the document declares complete (a
step claim), with the hidden-state checks passed. Nothing about a *sequence* of
calls is ever `exhaustive`. "All traces agree, by induction" is an inference, not a
suite, and is S3 (section 7.2).

### 1.2 Where this document departs from the memo

Each item is a point where the memo is inconsistent with itself or with a merged
specification. The resolution is the text of the cited rules.

| # | Memo statement | Problem | Resolution |
|---|---|---|---|
| M1 | Section 0.5 and B4 call driven suites "at most `exhaustive-sampled`"; B3 and open question I-1 call them `bounded-set` | two names for one thing; owner decision (c) names `bounded-set` | The LP-012 *kind* is `bounded-set`; the VER29 *rung* it maps to is `exhaustive-sampled` (rank 2). Both are printed (L2) |
| M2 | B3 labels a footprint-complete claim over a model-built frame `full`; B4 and B9.7 cap the CPU contract at `exhaustive-sampled` | the headline count would show a family of states as exhaustive while the strength says otherwise | Reach classes (E6): the *kind* follows state coverage, so a family claim is `bounded-set` and caps strength; only a declared-complete state set is `full` |
| M3 | B5 names a `stale-state` fault "WX-G `stale-post` on the state slot" | one mechanism, two names | S1 reuses `stale-post` (VER04k section 4.8); `stale-state` is not introduced (E17) |
| M4 | Contract 2 `split_independent` compares `framer_run([Feed(a), Feed(b), Frames])` with `framer_run([Feed(concat(a, b)), Frames])` | the replies are lists of length 3 and 2, never equal | compare the replies after the feeds with `drop` (section 6.6) |
| M5 | Contract 3 `Cpu` has a tuple field `stack: (U16, U16, U16)` | VER04k section 4.3 admits records of admissible fields and tuples are not admissible | three `U16` fields (section 5.3) |
| M6 | Contract 3 frame claim quantifies `s: Cpu` and "runs in the random suites" | a binder domain over the budget is W0706 (VER04h section 3); there is no random-only claim | a model-built family of whole-machine frames indexed by a `Nat`; open question O-2 asks whether a declared random-only claim should exist |
| M7 | Contract 1 `dff_rejects_non_bit_data` fixes the state | a rejection contract over one state is `family(distinct=1)` and would cap the layer | the four state fields become `Bool` binders, 8,192 cases, `full` (section 5.1) |
| M8 | Memo working name WX-I for alphabet domains | the backlog already uses WX-I for recursive sums | this document says "the alphabet-domain predicate `within`" and never uses the name |
| M9 | WX-G slot S4 says the host returns `Result<_, E>`; Contract 1's host `d_flip_flop` returns `(u8, u8)` and panics on a non-bit | the document needs `Result<(.., State), GateError>` and the host has no `Result` | VER04k section 4.6 amendment: with `error=<CASE>` an S2/S4 slot may be served by a host that returns the remaining components directly; a panic is `Err` and has no post-state (section 13.2) |
| M10 | B5 lists `order-independence` under Mode E and says it is mandatory for driven operations too | stated twice, differently | one rule, mandatory for every `state=explicit` and driven operation (E16, P8) |
| M11 | B2 widens WX-G `inout List<R>` "for `register`" | VER04k section 4.3 reads "`List<T>` with `T` one of those" and does not say whether a record is one of those | section 13.2 makes it explicit |

## 2. Versioning and compatibility

**C1.** There is no `v2` and no per-feature switch. A document is v1 exactly when
its header is `warrant name v1` (VER04j section 1.1). S1 adds no grammar, no token,
no reserved word. S2 adds exactly one contextual bound metric, `retained` (P12),
which is W0101 in a v0 document, as `output` is (VER04j section 6.6). S3 would add
the contextual declaration word `object` (section 7.1) and is not part of this
specification's normative surface.

**C2.** Every v0 document and every v1 document written for the wave-3 or wave-4
tool means exactly what it did and regenerates byte-identically. Every new prelude
part (state-completeness obligations, the dispatch loop, the `retained` observer
hook) is gated like the `comp`, `obs` and driver parts of VER04j section 14 items
25 and 33: emitted only for a document that uses the feature. `CheckedWarrantModule`
gains no field (VER04k section 1.3 item 3).

**C3.** New surface elements, all outside the Warrant grammar except `retained`:

| Element | Kind | Slice | In a v0 document |
|---|---|---|---|
| `state=explicit`, `inv=`, `states=`, `omit=` on an `op` line | binding keys | S1 | W0501 (the checker never sees them; a v0 *document* cannot carry `inout`, so a binding that uses them on a v0 operation is W0730) |
| `codec` line, `strmap` host type, `slice-of:struct:` and `vec-of:struct:` host forms | binding vocabulary | S1 | as above |
| `frame-leak:<field>`, state obligations | faults, obligations | S1 | not applicable |
| `dispatch=` driver entries, `abs=`, `reset=` on a `driver` line | binding vocabulary | S2 | W0711 (a driver on a v0 operation) |
| `skip-call:<Ctor>`, `shared-instance`, protocol obligations | faults, obligations | S2 | not applicable |
| `retained <= e` in a `bound` block | contextual metric | S2 | **W0101** |
| `object` blocks, derived rows | declaration word, report row | S3 | not specified here |

## 3. The model

### 3.1 Mode E: explicit state

A method with explicit state is an `op` whose state is an `inout` parameter
(VER04k section 4.2):

```text
step : (State, Input) -> (Output, State)      -- op step(x: In, s: inout State) -> (Out, State)
```

Parameter names denote the **pre-state**; the result carries the **post-state**
through slot rules S1 to S4 of VER04k section 4.2; the adapter turns the `&mut`
call into that pure function and enforces pre-state isolation and read-back
(VER04k section 4.6). The arguments of the host call are passed in the operation's
parameter order, so an operation whose host method has a `&mut self` receiver
declares the state parameter **first** (`op step(cpu: inout Cpu)`), and one whose host
function takes `&mut State` last declares it last (`op d_flip_flop(data, clock,
state: inout FlipFlopState)`).

**Abstraction.** A reference model is a Warrant record or type `M` and pure models.
For Mode E the abstraction function is the identity (the `State` record is the host
state read field by field), or a pure model `alpha` from the host-shaped record to a
smaller `M` when the author prefers one.

### 3.2 Mode P: protocol

The state is private. The object is described by its **trace function**

```text
run : (CtorArgs, List<Call>) -> List<Reply>     -- an ordinary op, executed by a driver
```

`Call` and `Reply` are WX-A variants with one constructor per method. The driver
creates one fresh object, performs the calls in order, collects the replies. History
is the argument; nothing is hidden from the quantifier. Reference model:

```text
m_init(args) -> M      m_step(s: M, c: Call) -> (Reply, M)      inv(s: M) -> Bool
m_run(args, cs) = the replies of folding m_step over cs from m_init(args)
```

If the object has a snapshot method (`dump`, `snapshot`, `buffer_size`) the binding
names it `abs` (P4): the driver calls it after every step, and the claim compares the
abstract state after every call, a forward simulation checked on every visited state.

The *same contract* is Mode E in a functional lane and Mode P in an object lane.
That is fine: the document is the same pure function either way and only the binding
differs (section 9.3).

### 3.3 Three different things called "invariant"

| Kind | Statement | Evidence | Reported as |
|---|---|---|---|
| model lemma | `inv(m_init(a))` and `inv(s) => inv(m_step(s, c).1)` | evaluated on models only; no subject is called | a **lemma** (V9), never subject evidence, excluded from every case count and from published strength |
| implementation invariant | an `ensures` row that holds on every call (Mode E) or `inv(abs)` after every call (Mode P) | monitored on every call, VER04k section 4.7 | `monitored` |
| closure | every state a step claim quantifies over satisfies `inv`, and every post-state does | the two rows above together | `closure: evidenced` or `closure: open` (E8) |

### 3.4 What a step claim and a trace claim establish

| Claim | Establishes | Does not establish |
|---|---|---|
| Mode E step claim over a declared-complete state set and all inputs | the transition function equals the model on that set | that reachable states lie in the set (needs closure) |
| closure (`inv` holds initially and `ensures` preserves it) | reachable states satisfy `inv` | anything about outputs |
| step claim and closure, both complete, hidden-state obligations passed | by induction on trace length, every finite trace agrees with the model: **an inference, S3 prints it** | timing, resources, anything outside the binding's state |
| Mode P trace claim to depth `K` over a call alphabet | agreement on those traces | agreement on any longer trace |
| Mode P with `abs` per step | forward simulation on visited states | unvisited states |

### 3.5 Hidden state and equality

The pure reading is sound only if the host object has no state outside what the
document names. That is an obligation with controls (E15, E16, P6 to P9), never an
assumption. **Equality of states** is structural equality of the Warrant value, so a
codec MUST be canonical: two host states that behave identically must observe equal
(a heap array's internal order is the known counter-example, VER04k question 11).
Where no canonical observer exists, use Mode P.

### 3.6 Failed calls

VER30 section 4.3 requires "what a failed call does to the object state, stated per
operation". This is the choice of slot rule, and the report prints it per operation:

| Document form | Meaning | Report line |
|---|---|---|
| slot S3 (`(Result<O, E>, State)`) with an `ensures` row `Err(e) -> result.1 == <state>` | an error leaves the state unchanged, monitored on every call | `on error: state unchanged (ensures row i)` |
| slot S3 without such a row | the post-state is observed on errors and constrained by the claims only | `on error: state observed` |
| slot S4 (`Result<(O, State), E>`) | on an error there is no post-state; the host may have partially mutated | `on error: state unspecified` |

An operation with a fallible host and explicit state SHOULD use S3 and state the
error rule; S4 is what a panic-normalised host gives (M9) and the report says so.
**E1.** The report MUST print the failed-call line for every `state=explicit`
operation whose result type can be an error.

## 4. Slice S1: explicit state through WX-G `inout`

Everything in this section is S1 and is **binding-level**. A document uses
`inout`, `ensures` and ordinary models and claims exactly as VER04k section 4
specifies them; this section adds binding keys, one report classification, four
obligations and one label rule.

### 4.1 Operations, step claims, and the `state` key

**E1.** (Failed calls.) See section 3.6: the report prints the failed-call line for
every `state=explicit` operation whose result can be an error.

**E2.** (The `state` key.) An `op` line MAY carry exactly one `state=` key with value
`none` or `explicit` (the H3 key of VER29 section 6.2, validated like the other
binding keys). Without it the classifier is conservative, as VER29 already says.

* `state=explicit` declares: *this operation's `inout` parameter carries object
  state, and the operation is a pure function of its explicit arguments including
  that state.* It requires at least one `inout` parameter (W0730 otherwise) and a
  v1 document (W0730).
* `state=none` remains correct for an `inout` operation that is a pure function of a
  buffer and carries no object state (`zeroize`, `sqlite-file::varint::write`): the
  `inout` there is an output buffer, not state. VER29 section 6.2 is unchanged for
  such operations except that the hidden-state obligations of E15 are **not** run for
  them (their controls are WX-G's `stale-post` family).
* An `export=` of an operation with explicit state is a Rust path (VER04k section
  4.6); `Type::method` for a `&mut self` method, `module::function` for a function
  taking `&mut State`.

A **step claim** is any claim that calls a `state=explicit` operation. A claim that
calls it more than once, with a later call's state argument built from an earlier
call's post-state (the nested form of section 5.1), is a step claim with
`steps=k`; the report prints `k` (L5).

**E3.** (Parameter order.) The adapter passes the host arguments in the operation's
parameter order (VER04k section 4.6). The state parameter is first for a `&mut self`
method and in the host's own position for a function.

**E4.** (Lists of records.) `inout List<R>` with `R` an admissible record is
admitted (VER04k section 4.3 as amended, section 13.2). Its host forms are
`slice-of:struct:<Name>` (`&mut [Name]`, length cannot change, checked against the
pre-state length) and `vec-of:struct:<Name>` (`&mut Vec<Name>`, any length), each
element read back field by field. Slot rules are unchanged: the post-state is the
list. A refinement on a list-of-records parameter constrains the pre-state, as for
any `inout`.

### 4.2 Invariants, reach classes, and what "complete" means for state

Injection means the claims quantify over states that may be unreachable (decision
d1). The danger is the converse of vacuity: a claim over states the implementation
may legitimately mishandle, or a claim made vacuous by an invariant that is too
strong. The rules below make the choice visible and mechanical.

**E5.** (Invariant or total, mandatory.) Every `state=explicit` operation MUST carry
exactly one of:

* `inv=<model>`: a document model whose parameters are the operation's `inout`
  parameters in order and whose result is `Bool` (typed like an `ensures` row;
  W0730 if absent, mistyped, or if it calls an operation or contains a quantifier);
* `inv=total`: the operation declares that it has no invariant and every step claim
  quantifies over every value of the state type (E9).

**E6.** (Reach classes.) For each step claim the generator derives, from the claim
and the binding alone, how the **root state arguments** (the arguments at `inout`
positions that do not themselves contain a call of a state operation) cover the
state type. The weakest class over the claim's root state arguments is the claim's
reach class, recorded in the manifest and printed:

| Class | Derivation | Meaning |
|---|---|---|
| **R1 `all-states`** | each root state argument is a binder, or a tuple, record or list constructor over binders, of the state type, with no refinement that mentions it, and every binder is of a complete type | the claim quantifies over the whole state type |
| **R2 `inv-complete(M)`** | `inv=M` and either (a) each root state argument is a binder with a top-level refinement conjunct `M(binder)` over a complete type, or (b) the binding gives `states=<claim>:<N>` and the generator verifies both that `M` holds on every root state value of every admitted case and that exactly `N` distinct root state values occur | the claim quantifies over the invariant's state set: by construction in (a), by a **declared and printed** count in (b) |
| **R3 `family(<builder>; distinct=D)`** | anything else: root state arguments built by a model from binders, or sampled | the claim covers a named family of states and says nothing about the rest |

`states=` is a comma-separated list of `<claim name>:<N>` entries on the `op` line
(one count per claim, because a width or a frame fixed by the claim changes the
count: the register has 256 invariant states at width 4 and 4,096 at width 6). `N`
is the author's assertion of how many states satisfy the invariant *in the shape the
claim fixes*; it is a reviewable fact in the document (the invariant is a model a
reviewer can read), it is printed as `states=N declared`, and the generator checks
the half it can check: a count the claim does not reach (`distinct < N`) fails
generation with W0732 (the claim is not complete), and a state outside the invariant
fails the claim with the note `state-outside-invariant`. The generator cannot check
that `N` is not understated; the report says `declared`, never `verified`.

**E7.** (Kind and strength follow reach.) The LP-012 kind of a step claim's cases is
`full` only if its reach class is R1 or R2 **and** every binder is complete. A
claim of class R3 is **`bounded-set`**, whatever its binders, and caps the claim's
VER29 strength at `exhaustive-sampled` (resolving M2). The report prints the class
beside every step claim:

```text
dff_step_all_states   full          exhaustive           steps=1  reach=inv-complete(bit_state) states=16 declared, distinct=16
add_reg_all_operands  bounded-set   exhaustive-sampled   steps=1  reach=family(nibbles; distinct=8192)
```

**E8.** (Closure, printed.) For an operation with `inv=M`, the operation's `ensures`
block SHOULD contain a **closure row**: after removing an outer wrapper of the form
`match result with Ok(r) -> E | Err(e) -> true` (slots S2 and S4), a row of the form
`M(p) => M(q)` where `p` is the `inout` parameter and `q` is the post-state
projection the slot rules name (for a record state of slot S3, `result.1`). The
report prints `closure: evidenced (ensures row i)` when such a row exists and
`closure: open` otherwise. The closure row is monitored on every call of every suite
(VER04k section 4.7), so it is evidence on every pre-state the suites visit, not a
separate claim. Closure never changes a claim's own label; it decides whether the
claim meets VER29 section 14 condition 5 (E14). The report also prints
`init: evidenced (example <name>)` when an `example` of the form
`M(<ground initial state>) == true` exists, else `init: not stated`.

**E9.** (`inv=total`.) With `inv=total`, a step claim whose reach class is not R1 is
W0732. `inv=total` is realistic only for small state types (a record of `Bool`
fields; the `d_flip_flop` record with `U8` fields is `inv=bit_state`, not total).

**E10.** (Counts, and no empty claims.) The manifest records, per step claim,
`admitted`, `discarded`, the invariant used, and `distinct` root state values. A step
claim with zero admitted cases is W0601, as an invariant that excludes everything
would otherwise pass vacuously. The budget `B = 400,000` and the 64 MiB domain-byte
limit apply to the product of binder domains **before** refinement (VER04h section 3),
so a refined whole-state binder such as `s: Cpu | cpu_inv(s)` is W0706; use a
model-built family (class R3) or a `Bool`-built state (class R1 or R2).

### 4.3 State domains

| State space | Domain the document uses | Class | Kind |
|---|---|---|---|
| small and finite (flip-flop, 16 bit-valued states; register of width `n`, `4^n` consistent states) | the full product of its fields, built from `Bool` binders, with `inv=` and `states=` | R2 | `full` |
| record of small fields, fully enumerable | the record as a binder | R1 | `full` |
| finite but large (a CPU) | *footprint complete, frame fixed*: the fields an instruction reads and writes enumerated completely inside one fixed frame; a separate frame claim over a model-built family of whole-machine frames | R3 | `bounded-set` (printed with `distinct`) |
| unbounded (a `Vec`-backed container) | the ordinary domain of the type, or model-built canvases | R3 | `bounded-set` |

Initial states are ordinary states: `m_init` values are examples, and the claims
quantify over all states satisfying the invariant, which is stronger than starting
from the initial one.

### 4.4 The `codec` binding line

Most explicit-state hosts are all-public structs, for which the WX-A `struct` line
(`struct T path=P fields=name:ht,...`, VER04j section 14 item 22) already is the
codec. Objects whose state is *readable and buildable* but not by field assignment
alone need a codec (VER04k open question 11, decision d2).

**E11.** (Codec.) A `codec` line refines an existing `struct` line of the same record:

```text
struct Cpu path=intel4004_simulator::Intel4004Simulator fields=acc:u8,carry:bool,regs:bytes,pc:u16,...
struct Intel4004State path=intel4004_simulator::Intel4004State fields=acc:u8,carry:bool,regs:bytes,pc:u16,...
codec Cpu type=intel4004_simulator::Intel4004Simulator build=new()+fields observe=snapshot(ref)->Intel4004State
```

JSON form of the same record, closed like every binding (unknown fields are errors):

```json
"codecs": { "Cpu": {
  "type":    "intel4004_simulator::Intel4004Simulator",
  "build":   { "mode": "new+fields", "new": { "fn": "new", "args": [] } },
  "observe": { "method": "snapshot", "receiver": "ref", "host": "Intel4004State" } } }
```

* `type` is a Rust path of at most 8 identifier segments and MUST equal the `path` of
  the record's `struct` line (W0730 otherwise). These strings are spliced into
  generated source, so anything else is refused before generation, never escaped
  (VER04k section 3.4 rule 2).
* `build` is one of `fields` (a struct literal; every host field named by the
  `struct` line, and no field omitted), `new(<literal>,...)+fields` (call an
  associated function with integer or bool literals, then assign the named public
  fields), or `fn:<ident>(<field>,...)` (call an associated function with the
  named record fields in order). Nested host arrays flatten in row-major order to
  `Bytes<N>` (`[[[u8; 16]; 4]; 4]` is `Bytes<256>`).
* `observe` is one method with a `ref` receiver returning a declared `struct` whose
  fields are the record's fields; the observation is **canonical** (3.5). It is read
  field by field, never by `memcmp` or `Debug` text.
* The inout host form of the operation is `codec:<Record>`. The adapter builds a
  fresh host object from the pre-state, calls the method, and observes the result
  through `observe`: the read-back of VER04k section 4.6 rule 2 happens through the
  codec, from the same object that was called. Pre-state isolation holds as before.
* The codec is part of the harness, hence of the `tcb` record of the report
  (section 10). Heaps, hash maps and anything without a canonical observer stay
  Mode P.

### 4.5 Declared omissions

**E12.** (`omit=`.) Real return types carry fields the model does not describe
(`Intel4004Trace.mnemonic`, a `String`; error payloads). Decision d5 allows declared
omissions and forbids silent ones:

* `omit=<field>[,<field>...]` on a `struct` line names host fields the record does
  not carry. Allowed on a record used only in **result** positions or only as an
  observed codec target (the host value is read with a `{ a, b, .. }` pattern).
  On a record the adapter must **build** (an `inout` state without a codec build that
  supplies every field), an omission is W0730: the adapter cannot construct a host
  value from a partial record.
* `omit=payload` on an `op` line declares that the host error enum carries payloads
  the document's `error` cases do not (VER04j section 14 item 22: payload-carrying
  host errors are matched with `{ .. }`).
* **Every omission is printed** in the report as `uncovered: <Struct>.<field>` or
  `uncovered: error payloads of <op>`, in a section that is part of the report digest.
  A package with omissions is never reported as covering the full API surface (V6).

**E13.** (`strmap`, until text exists.) A host parameter of type `&str` that selects
among a closed set of constants (`shift_register`'s `direction`) is declared in the
document as a variant (`variant Dir = Left | Right`) and in the binding with the
closed host type `strmap(Left=left|Right=right)`, each literal matching
`[a-z0-9_-]{1,32}`. The adapter passes the literal as `&'static str`. Every
`strmap` automatically adds `uncovered: <op>.<param> accepts strings outside the
map (not exercised)` to the report: the host's rejection of other strings is **not
covered** until VER04l text exists, and the report says so. After text, the parameter
becomes `Text` with a `rejects` row and the `strmap` is removed.

### 4.6 Obligations: the state-completeness checks

**E14.** (The H3 rule.) This rule replaces the "stateless and deterministic binding"
condition of VER29 section 6.2 (see section 13.1 for the amended text).

> An `exhaustive` suite supports the label `exhaustive` only if (1) every binder of
> the claim is a complete finite type, and (2) every operation the claim calls is
> **pure in its explicit arguments**: its binding declares `state=none`, or declares
> `state=explicit` and, for that operation, the claim's reach class (E6) is R1 or R2
> and the state-completeness obligations of E15 passed, with their controls, in the
> same manifest. An operation executed by a driver (WX-F or a protocol, section 6) is
> never `exhaustive`, and neither is any claim that quantifies over a trace domain
> (`within`): such a claim is `bounded-set` and its strength is at most
> `exhaustive-sampled`.

A fixed nested composition (`dff_edge_captures_data` of section 5.1: two calls from
every state in the declared set) is **not** a trace domain: its call sequence is
fixed and only states and inputs are enumerated, so it is `full` and prints
`steps=2`. Quantifying over a *list of calls* is what makes a claim `bounded-set`.

**E15.** (Obligations for explicit state.) For every `state=explicit` operation the
generator registers these tests, ids `warrant/<module>/state/<op>/<obligation>`,
manifest kind `state`:

| Obligation | Applies to | What runs | Fails when | Its control |
|---|---|---|---|---|
| `codec-roundtrip` | `codec:` host forms; `n/a` otherwise (printed, V5) | `observe(build(s)) == s` over every distinct root state value the op's claims build, at most 4,096 (`sampled(4096 of N)` printed beyond) | the codec loses or invents state | a codec with one field forced to zero MUST fail |
| `rebuild-equivalence` | `codec:` host forms; `n/a` otherwise | for 64 deterministic `(s, x1, x2)` drawn from the claims' cases: continue on the live object after `x1`, versus rebuild from the observed post-state and apply `x2`; compare results and observed states | the object has state the codec does not carry | a wrapper with a hidden call counter that alters the second result MUST fail |
| `deterministic` | every `state=explicit` op | 16 fixed-seed `(s, x)` run on two separately built hosts | global or ambient state | the second run is given a different state and MUST then differ for at least one of the 16 |
| `order-independence` | every `state=explicit` op | the first 256 cases of the op's suites run in two deterministic orders (seeded from the evidence seed) in one process | state shared between instances (statics, caches) | a wrapper with a process-wide counter mixed into the result MUST fail |

**E16.** (`order-independence` is mandatory.) One test process runs every case, so a
static cache makes later cases depend on earlier ones without any syscall (section
9, J4). The obligation cannot be omitted for any `state=explicit` or driven
operation, and a lane that cannot reorder cases is `not-live` for that operation.

### 4.7 Faults

Faults are rewrites of the adapter's read-back; the crate is never modified. Every
applicable fault MUST be listed in `faults=` (W0502 if one is missing, W0701 if one
that does not apply is listed), and each is caught by a suite or its control fails
(VER04 section 6).

**E17.** (Fault set for explicit state.)

| Fault | Rewrite | Applies when | Bug modelled |
|---|---|---|---|
| `stale-post` | WX-G's (VER04k section 4.8): the subject is called on a copy and the post-state is the pre-state | every `inout` operation | mutation applied to a copy |
| `frame-leak:<field>` | one **observed** field `f` of the record state (or of the **last** element of a record list) is perturbed in the post-state: `U8` low bit flipped, other integers `+1` wrapping, `Bool` negated, the last byte of a `Bytes` field low-bit flipped | the state is a record, `f` is not omitted, and `f` can be non-trivial in the domain; one entry per such field | a step that touches state it should not. It forces **frame claims** to exist: a suite that compares only some fields lets the fault through and its control fails |
| WX-G's `skip-last-write`, `drop-last-push`, `clobber-prefix`, `cursor-off-by-one` | VER04k section 4.8, per host form | per VER04k | per VER04k |

`stale-state` is not a fault name (M3).

### 4.8 Manifest, proofs

**E18.** (Manifest additions, all optional fields.) Per operation: `state:
none|explicit|driven`. Per step claim: `reach` (`all-states`, `inv-complete`,
`family`), `invariant`, `admitted`, `discarded`, `distinct`, `states_declared` when
given, `steps`, `closure`. Obligation entries with `kind: "state"`. `lemma` as a
distinct entry kind for claims that call no operation (V9). The codec, with its
`type`, is recorded in the `tcb` section.

**E19.** (Proofs.) A `state=explicit` operation is `not-live` for proof obligations
(VER04h section 7): its `op` line carries `proofs_not_live=state`, or generation
fails with W0730 when a `labels` row with `secret` or a `bound` row would otherwise
produce an obligation the prover cannot lower. The Mode E **functional-equivalence**
obligation that would make the lowest stateful layers `proved` is funded next and
specified in section 7.3.

## 5. Slice S1: worked examples

All three are sketches (not run through the checker). Binding lines follow the
`evidence.bindings` line forms of VER04j section 14 item 22 and VER04k sections 3.2
and 4.6; the host names are illustrative where the crate's real variants are not
quoted. The examples use the invariant models of E5 to E8 and the labels of
section 8.

### 5.1 `logic-gates::sequential`: the 4004 chain's second layer

The crate (from reading): `FlipFlopState` has four public `u8` fields, default
`(0, 1, 0, 1)`; `d_flip_flop(data, clock, &mut FlipFlopState) -> (u8, u8)` runs a
master `d_latch` with enable `not clock` and a slave `d_latch` with enable `clock`
fed by the master's new `q`; `register(&[u8], u8, &mut [FlipFlopState]) -> Vec<u8>`
asserts equal non-zero lengths; `shift_register(serial_in, clock, &mut [..],
direction: &str) -> (Vec<u8>, u8)` panics for any direction other than `"left"` and
`"right"`. A value that is not a bit panics, as `validate_bit` is a hard check in the
combinational layer (SEC-007).

```text
warrant logic_gates_sequential v1                     -- v1 + WX-G; S1 needs no new grammar
  error GateError = INVALID_INPUT
  record FlipFlopState(master_q: U8, master_q_bar: U8, slave_q: U8, slave_q_bar: U8)
  variant Dir = Left | Right

  model b2u(x: Bool) -> U8 = if x then 1u8 else 0u8
  model junk(x: U8) -> U8 = 2u8 + unwrap_or(rem(x, 254u8), 0u8)        -- never 0 or 1
  model ffs(mq: Bool, mqb: Bool, sq: Bool, sqb: Bool) -> FlipFlopState =
    FlipFlopState(b2u(mq), b2u(mqb), b2u(sq), b2u(sqb))

  -- Invariants. bit_state: every field is a bit (16 states, including q == q_bar pairs).
  -- consistent: a bit state whose q_bar is not q (4 states per flip-flop).
  model bit_state(s: FlipFlopState) -> Bool =
    s.master_q < 2u8 and s.master_q_bar < 2u8 and s.slave_q < 2u8 and s.slave_q_bar < 2u8
  model consistent(s: FlipFlopState) -> Bool =
    bit_state(s) and s.master_q_bar == 1u8 - s.master_q and s.slave_q_bar == 1u8 - s.slave_q
  model all_consistent(l: List<FlipFlopState>) -> Bool = all(l, consistent)    -- WX-D2

  -- The model: two level-sensitive latches. A latch whose enable is high takes (d, not d);
  -- otherwise it holds the pair it has, consistent or not.
  model latch_q(d: Bool, en: Bool, q: Bool) -> Bool = if en then d else q
  model latch_qb(d: Bool, en: Bool, qb: Bool) -> Bool = if en then not d else qb
  model dff(d: Bool, c: Bool, mq: Bool, mqb: Bool, sq: Bool, sqb: Bool) -> FlipFlopState =
    ffs(latch_q(d, not c, mq), latch_qb(d, not c, mqb),
        latch_q(latch_q(d, not c, mq), c, sq), latch_qb(latch_q(d, not c, mq), c, sqb))

  op d_flip_flop(data: U8, clock: U8, state: inout FlipFlopState)
       -> Result<((U8, U8), FlipFlopState), GateError>            -- slot S4 (M9)

  -- THE STEP CONTRACT: every bit state (16) and every bit input (4): 64 cases.
  claim dff_step_all_states:
    forall d: Bool, c: Bool, mq: Bool, mqb: Bool, sq: Bool, sqb: Bool.
      d_flip_flop(b2u(d), b2u(c), ffs(mq, mqb, sq, sqb)) ==
        Ok(((dff(d, c, mq, mqb, sq, sqb).slave_q, dff(d, c, mq, mqb, sq, sqb).slave_q_bar),
            dff(d, c, mq, mqb, sq, sqb)))

  -- A fixed two-step composition: low then high captures the data, from every bit state.
  model after(r: Result<((U8, U8), FlipFlopState), GateError>, s: FlipFlopState) -> FlipFlopState =
    match r with Ok(x) -> x.1 | Err(e) -> s
  claim dff_edge_captures_data:                                     -- 5 Bool binders: 32 cases, steps=2
    forall d: Bool, mq: Bool, mqb: Bool, sq: Bool, sqb: Bool.
      match d_flip_flop(b2u(d), 1u8,
              after(d_flip_flop(b2u(d), 0u8, ffs(mq, mqb, sq, sqb)), ffs(mq, mqb, sq, sqb)))
      with Ok(r) -> r.0.0 == b2u(d) | Err(e) -> false

  -- The rejection contract quantifies the state (M7): 256 junk values x 2 clocks x 16 states.
  claim dff_rejects_non_bit_data:
    forall j: U8, c: Bool, mq: Bool, mqb: Bool, sq: Bool, sqb: Bool.
      d_flip_flop(junk(j), b2u(c), ffs(mq, mqb, sq, sqb)) == Err(GateError.INVALID_INPUT)

  ensures d_flip_flop:
    match result with Ok(r) -> r.0.0 == r.1.slave_q and r.0.1 == r.1.slave_q_bar | Err(e) -> true
    match result with Ok(r) -> (bit_state(state) => bit_state(r.1)) | Err(e) -> true   -- closure row (E8)

  -- A register of width 4, state a list of consistent flip-flops (4^4 = 256 states).
  op register(data: Bytes, clock: U8, state: inout List<FlipFlopState>)
       -> Result<(Bytes, List<FlipFlopState>), GateError>         -- slot S4
  model cff(m: Bool, s: Bool) -> FlipFlopState = ffs(m, not m, s, not s)
  claim register_step_width_4:        -- 4 data + 1 clock + 4 x (m, s) = 13 Bool binders: 8,192 cases
    forall d3: Bool, d2: Bool, d1: Bool, d0: Bool, c: Bool,
           m0: Bool, s0: Bool, m1: Bool, s1: Bool, m2: Bool, s2: Bool, m3: Bool, s3: Bool.
      register(to_bytes([b2u(d3), b2u(d2), b2u(d1), b2u(d0)]), b2u(c),
               [cff(m0, s0), cff(m1, s1), cff(m2, s2), cff(m3, s3)]) == reg4_model(d3, d2, d1, d0, c, m0, s0, m1, s1, m2, s2, m3, s3)
        -- reg4_model applies the model dff to each element; written out in the document
  ensures register:
    match result with Ok(r) -> (all_consistent(state) => all_consistent(r.1)) | Err(e) -> true

  -- shift_register: direction is a variant until text exists (E13).
  op shift_register(serial_in: U8, clock: U8, state: inout List<FlipFlopState>, dir: Dir)
       -> Result<((Bytes, U8), List<FlipFlopState>), GateError>    -- slot S4, tail (Bytes,U8)
  claim shift_register_step_width_4:  -- 1 + 1 + 8 + 1 = 11 Bool binders: 2,048 cases (not shown)

  assume platform:
    contract.logic-gates.not_truth_table
    contract.logic-gates.nor_truth_table
  assume environment:
    none
```

Bindings (sketch):

```text
struct FlipFlopState path=logic_gates::sequential::FlipFlopState fields=master_q:u8,master_q_bar:u8,slave_q:u8,slave_q_bar:u8
op d_flip_flop export=sequential::d_flip_flop inout=state:struct:FlipFlopState state=explicit inv=bit_state states=dff_step_all_states:16,dff_edge_captures_data:16,dff_rejects_non_bit_data:16 error=INVALID_INPUT proofs_not_live=state faults=stale-post,frame-leak:master_q,frame-leak:master_q_bar,frame-leak:slave_q,frame-leak:slave_q_bar
op register export=sequential::register inout=state:slice-of:struct:FlipFlopState state=explicit inv=all_consistent states=register_step_width_4:256 error=INVALID_INPUT proofs_not_live=state faults=stale-post,skip-last-write,frame-leak:master_q,frame-leak:master_q_bar,frame-leak:slave_q,frame-leak:slave_q_bar
op shift_register export=sequential::shift_register inout=state:slice-of:struct:FlipFlopState args=serial_in:u8,clock:u8,state,dir:strmap(Left=left|Right=right) state=explicit inv=all_consistent states=shift_register_step_width_4:256 error=INVALID_INPUT proofs_not_live=state faults=stale-post,skip-last-write,frame-leak:master_q,frame-leak:master_q_bar,frame-leak:slave_q,frame-leak:slave_q_bar
```

What the generator and report say about it:

| Claim | Binders | Cases | Reach (E6) | Kind (L1) | Strength (L2) |
|---|---|---|---|---|---|
| `dff_step_all_states` | 6 `Bool` | 64 | R2(b), `states=16` declared, `distinct=16` | `full` | `exhaustive` |
| `dff_edge_captures_data` | 5 `Bool` | 32 (`steps=2`) | R2(b), `distinct=16` | `full` | `exhaustive` |
| `dff_rejects_non_bit_data` | `U8` and 5 `Bool` | 8,192 | R2(b), `distinct=16` | `full` | `exhaustive` |
| `register_step_width_4` | 13 `Bool` | 8,192 | R2(b), `states=256` declared, `distinct=256` | `full` | `exhaustive` |
| `shift_register_step_width_4` | 11 `Bool` | 2,048 | R2(b), `distinct=256` | `full` | `exhaustive` |

Each strength reads `exhaustive` only if the E15 obligations passed with their
controls and `faults=` caught everything. Honest limits the report prints beside the
table: `inv=bit_state` covers 16 of the 2^32 values the four `u8` fields can hold
(whether the host rejects or tolerates a non-bit state is not claimed here; a claim
of that kind is a later addition, and until then the contract is about bit states
only); `closure: evidenced (ensures row 2)` for `d_flip_flop`; `uncovered:
shift_register.dir accepts strings outside the map (not exercised)`; the
`codec-roundtrip` and `rebuild-equivalence` obligations print `n/a` because every
host form is a public struct or a slice of them. Notes the pilot must settle by
running the 64 cases: whether the hold behaviour on an inconsistent pair
(`q == q_bar`) is what `d_latch` and `sr_latch` do after their NOR feedback settles.
If it is not, the author either writes the NOR equations into the model or restricts
`inv` to `consistent` and relies on the closure row; either is a visible choice in the
document. Published contract ids are ordinary functional ids
(`contract.logic-gates.dff_step_all_states`, `contract.logic-gates.register_step_width_4`),
as VER29 section 3 defines them; the CPU layer assumes them (section 8.4).

### 5.2 `pixel-container`: a variable-size state

The crate (from reading): `PixelContainer { pub width: u32, pub height: u32, pub data:
Vec<u8> }`, `data.len() == 4 * width * height`, RGBA row-major; `set_pixel` ignores
out-of-bounds coordinates; `pixel_at` returns `(0,0,0,0)` out of bounds; `fill`
writes every four-byte group; `new` computes `4 * width * height` with checked
arithmetic and then allocates it (a caller-chosen allocation: the pilot's `heap`
row will say what it can state, unconfirmed). The record is public, so the
`struct` line is the codec and the host form is `struct:PixelContainer`.

```text
warrant pixel_container v1
  record Px(width: U32, height: U32, data: Bytes)
  model px_inv(s: Px) -> Bool = len(s.data) == 4 * to_nat(s.width) * to_nat(s.height)
  model n4(b1: Bool, b0: Bool) -> Nat = 2 * b2n(b1) + b2n(b0)              -- 0..3
  model chan(b: Bool) -> U8 = if b then 165u8 else 17u8                   -- never a background byte
  model canvas(w: Nat, h: Nat, p: Bool) -> Px =                           -- background 0 or 90
    Px(to_u32(w), to_u32(h), to_bytes(replicate(if p then 90u8 else 0u8, 4 * w * h)))
  model put(s: Px, x: Nat, y: Nat, c: Bytes) -> Px = ...                  -- take/concat/drop at (y*w+x)*4, or s if out of bounds

  op set_pixel(img: inout Px, x: Nat, y: Nat, r: U8, g: U8, b: U8, a: U8) -> Px     -- slot S1
  op fill(img: inout Px, r: U8, g: U8, b: U8, a: U8) -> Px                           -- slot S1
  op pixel_at(img: Px, x: Nat, y: Nat) -> (U8, U8, U8, U8)                            -- state=none

  -- 13 Bool binders: w(2) h(2) p x(2) y(2) r g b a = 8,192 cases; w, h, x, y each 0..3, so
  -- in-bounds and out-of-bounds both occur.
  claim set_pixel_matches_model:
    forall wb1: Bool, wb0: Bool, hb1: Bool, hb0: Bool, p: Bool, xb1: Bool, xb0: Bool,
           yb1: Bool, yb0: Bool, r: Bool, g: Bool, b: Bool, a: Bool.
      set_pixel(canvas(n4(wb1, wb0), n4(hb1, hb0), p), n4(xb1, xb0), n4(yb1, yb0),
                chan(r), chan(g), chan(b), chan(a)) ==
        put(canvas(n4(wb1, wb0), n4(hb1, hb0), p), n4(xb1, xb0), n4(yb1, yb0),
            to_bytes([chan(r), chan(g), chan(b), chan(a)]))
  claim set_pixel_changes_only_four_bytes: ...    -- the frame claim: bytes outside [o, o+4) unchanged
  claim fill_matches_model: ...                   -- 4 + 1 + 4 = 9 Bool binders: 512 cases

  ensures set_pixel:
    px_inv(img) => px_inv(result)
  ensures fill:
    px_inv(img) => px_inv(result)
```

Bindings: `struct PixelContainer path=pixel_container::PixelContainer fields=width:u32,
height:u32,data:bytes`; `op set_pixel export=PixelContainer::set_pixel
inout=img:struct:PixelContainer state=explicit inv=px_inv proofs_not_live=state
faults=stale-post,frame-leak:width,frame-leak:height,frame-leak:data`.

The counts: 8,192 cases; the distinct root states are the canvases `(w, h, p)`:
`w, h` in `0..=3` gives 16 dimension pairs, 9 non-empty (each with 2 backgrounds, 18
states) and 7 empty (one state each), so `distinct=25`. Reach **R3**
`family(canvas; distinct=25)`, kind **`bounded-set`**, strength at most
`exhaustive-sampled`, even though every binder is a `Bool`: 25 canvases are not the
state space, and the report says so. The frame claim exists because the `frame-leak`
faults need a suite that compares `data` outside the written pixel; the model-equality
claim alone would also catch them, which is the point of having the independent frame
statement when the model is a transliteration of the implementation (V12). A claim over
the ordinary `Bytes` domain (`D(Bytes)`, 347 strings) is a second, `bounded-set` family.
Closure is `evidenced (ensures row 1)` for both mutators. Adopter facts from the
VER30 inventory: 22 direct dependents, 16 ports.

### 5.3 `intel4004-simulator`: one instruction

The crate (from reading): `Intel4004Simulator` has public fields (accumulator, sixteen
registers, carry, ROM, `pc`, a three-level stack, 4x4x16 RAM nibbles, ...);
`step(&mut self) -> Result<Intel4004Trace, Intel4004Error>`; `snapshot(&self) ->
Intel4004State`; the documented promise "every error leaves architectural state
unchanged". `Intel4004Trace.mnemonic` is a `String`; the error enum carries payloads.
The abstract state is what the ISA defines, with a codec (E11).

```text
warrant intel4004_simulator v1
  error CpuError = PROGRAM_TOO_LARGE | ADDRESS_OUT_OF_RANGE | TRUNCATED | UNKNOWN_OPCODE
                 | HALTED | INVALID_STATE
  -- ram is the 4 x 4 x 16 nibbles flattened; the stack is three U16 fields (M5).
  record Cpu(acc: U8, carry: Bool, regs: Bytes<16>, pc: U16, stack0: U16, stack1: U16,
             stack2: U16, sp: U8, rom: Bytes, ram: Bytes<256>, halted: Bool)
  record Trace(address: U16, raw: U8, acc_before: U8, acc_after: U8,
               carry_before: Bool, carry_after: Bool)           -- mnemonic: omitted (E12)

  op step(cpu: inout Cpu) -> (Result<Trace, CpuError>, Cpu)    -- slot S3: observed on errors too

  model cpu_inv(c: Cpu) -> Bool = c.acc < 16u8 and c.sp < 3u8 and len(c.rom) <= 4096   -- sketch
  model nib(b3: Bool, b2: Bool, b1: Bool, b0: Bool) -> U8 = 8u8 * b2u(b3) + 4u8 * b2u(b2) + 2u8 * b2u(b1) + b2u(b0)
  model idle() -> Cpu = ...                       -- zeroed machine, empty ROM, not halted
  model frame(a: U8, c: Bool, r: U8, v: U8) -> Cpu = ...   -- idle with acc=a, carry=c, regs[r]=v, rom=[0x80 | r]
  model add_model(a: U8, v: U8, c: Bool) -> Nat = to_nat(a) + to_nat(v) + (if c then 1 else 0)

  -- ADD Rr: 16 x 16 x 2 x 16 = 8,192 cases. Three U8 binders would be 2^25 before refinement
  -- (W0706), so nibbles are built from Bool binders (the arithmetic.warrant idiom).
  claim add_reg_all_operands:
    forall a3: Bool, a2: Bool, a1: Bool, a0: Bool, v3: Bool, v2: Bool, v1: Bool, v0: Bool,
           c: Bool, r3: Bool, r2: Bool, r1: Bool, r0: Bool.
      step(frame(nib(a3,a2,a1,a0), c, nib(r3,r2,r1,r0), nib(v3,v2,v1,v0))).1.acc ==
        wrap<U8>(unwrap_or(rem(add_model(nib(a3,a2,a1,a0), nib(v3,v2,v1,v0), c), 16), 0)) and
      step(frame(...)).1.carry == (add_model(...) >= 16) and
      step(frame(...)).1.pc == 1u16

  -- Frame rule (M6): nothing else changes. wild(k) is a model-built whole-machine frame with
  -- registers, RAM, stack and carry filled from a fixed generator; k ranges over Nat's 65 values.
  claim add_reg_touches_only_acc_carry_pc:                      -- 65 x 16 = 1,040 cases
    forall k: Nat, r3: Bool, r2: Bool, r1: Bool, r0: Bool.
      same_except_acc_carry_pc(
        step(with_rom(wild(k), add_opcode(nib(r3,r2,r1,r0)))).1,
        with_rom(wild(k), add_opcode(nib(r3,r2,r1,r0))))

  ensures step:
    cpu_inv(cpu) => cpu_inv(result.1)                            -- closure row
    match result.0 with Ok(t) -> true | Err(e) -> result.1 == cpu   -- every error leaves the state unchanged

  assume platform:
    none               -- the behavioural simulator uses host integers and assumes no ALU contract
  assume environment:
    none
```

Bindings (sketch): `struct Cpu` and `struct Intel4004State` lines as in E11, the
`codec Cpu` line of E11, `struct Trace path=intel4004_simulator::Intel4004Trace
fields=address:u16,raw:u8,acc_before:u8,acc_after:u8,carry_before:bool,carry_after:bool
omit=mnemonic`, and `op step export=Intel4004Simulator::step inout=cpu:codec:Cpu
state=explicit inv=cpu_inv errors=<host error path>/<Variant>=<CASE>,... omit=payload
proofs_not_live=state faults=stale-post,frame-leak:acc,frame-leak:carry,frame-leak:regs,
frame-leak:pc,frame-leak:stack0,frame-leak:stack1,frame-leak:stack2,frame-leak:sp,
frame-leak:rom,frame-leak:ram,frame-leak:halted`.

What the report says: `add_reg_all_operands` is 8,192 cases, `distinct=8192` (the ROM
byte differs with `r` and `regs[r]` with `v`), reach **R3** `family(nibbles)`, kind
`bounded-set`, strength at most `exhaustive-sampled`: the claim is **complete over the
footprint** (accumulator, carry, one operand register, `pc`) in one fixed frame, and the
**whole-machine frame is a family of 65**, so the CPU contract is honestly
`exhaustive-sampled` unless a frame argument is made by proof (L6). The 8,192 are not
"the machine". `frame-leak` faults on `ram`, `rom` and the stack are caught only by
`add_reg_touches_only_acc_carry_pc`; if that claim is deleted, those controls fail and
generation fails (E17). `uncovered: Trace.mnemonic`, `uncovered: error payloads of
step`; `on error: state unchanged (ensures row 2)`; `closure: evidenced (ensures row
1)`. `codec-roundtrip` and `rebuild-equivalence` run: the second is what catches a
cycle counter inside the simulator that `snapshot` does not carry. A first group of 5
to 10 instructions is the S1 pilot; one ISA model is later shared by nine language
ports and by the gate-level CPU, which is the real independence between model and
implementation (V12).

`riscv-simulator`'s `decode(&self, raw: u32, token: &mut PipelineToken)` is the same
shape with a single `inout` record and is a fourth S1 pilot; `execute(&mut self,
token, reg_file)` has two `inout` parameters (a record and a register file) and uses
slot S3 with `k = 2` (VER04k section 4.2), within the cap of four.

## 6. Slice S2: protocols through a WX-F dispatch driver

Everything in this section is S2. It extends the WX-F driver (VER04k section 3) with
a dispatch table; it adds no Warrant grammar except the `retained` bound metric
(P12). Its prerequisites are WX-F, WX-D2 (`fold`) and the alphabet-domain predicate
(6.3).

### 6.1 The dispatch driver

A WX-F driver has one `step` method fed by the elements of one driven parameter.
A protocol has one method *per call kind*. **P1.** A driver record MAY carry
`dispatch` in place of `step`: a table from `Call` constructors to methods. A WX-F
driver is the special case of one constructor. JSON form (one operation; closed;
unknown fields are errors):

```json
"framer_run": {
  "driver": {
    "state":  "irc_framing::Framer",
    "new":    { "fn": "new", "args": [] },
    "drive":  "calls",
    "dispatch": {
      "Feed":   { "method": "feed",        "receiver": "mut", "args": [ { "from": "elem.0", "host": "bytes" } ], "returns": "unit",        "reply": "Fed" },
      "Frames": { "method": "frames",      "receiver": "mut", "args": [],                                      "returns": "list<bytes>", "reply": "Framed" },
      "Reset":  { "method": "reset",       "receiver": "mut", "args": [],                                      "returns": "unit",        "reply": "Cleared" },
      "Size":   { "method": "buffer_size", "receiver": "ref", "args": [],                                      "returns": "usize",       "reply": "Sized" }
    },
    "reset":  "Reset",
    "result": "collect"
  },
  "faults": ["drop-last-call", "reorder-calls", "duplicate-last-call", "early-stop",
             "reset-per-call", "skip-call:Feed", "skip-call:Frames", "skip-call:Reset",
             "shared-instance"]
}
```

Line form (the `op` line carries the bare `driver`, as VER04k section 3.2; one
`driver` line carries the record, `dispatch=` repeated once per constructor):

```text
op framer_run driver faults=drop-last-call,reorder-calls,duplicate-last-call,early-stop,reset-per-call,skip-call:Feed,skip-call:Frames,skip-call:Reset,shared-instance
driver framer_run state=irc_framing::Framer new=new() drive=calls dispatch=Feed:feed(mut,elem.0:bytes)->unit=Fed dispatch=Frames:frames(mut)->list<bytes>=Framed dispatch=Reset:reset(mut)->unit=Cleared dispatch=Size:buffer_size(ref)->usize=Sized reset=Reset result=collect
```

**P2.** (Typing of `Call` and `Reply`.) The driven parameter has type `List<Call>`
for a variant `Call`. Every constructor of `Call` appears in `dispatch` exactly once
and every `dispatch` key is a constructor (W0731). `elem.<i>` names the `i`th field
of the constructor, so `Feed(data: Bytes)` supplies `elem.0`. The `reply` constructor
belongs to the declared `Reply` variant; it has no fields when `returns` is `unit`
and exactly one field, of the Warrant type the WX-A host mapping assigns to the host
return, otherwise. A `Reply` constructor that no entry produces is W0731 (dead).
A fallible method (`returns` of the form `result<ht>`, with `errors=` as VER04k
section 3.2) is bound to a reply constructor whose payload is a `Result`; **unlike a
WX-F step, a failing dispatch does not stop the drive**: a failed call is data, and
the calls after it run, so "an error leaves the state unchanged" is observable.
Panics are as for WX-F (VER04k section 3.5).

**P3.** (Receivers.) A dispatch receiver is `mut` (`&mut self`) or `ref` (`&self`). A
consuming method (`self`) ends the object and is expressed by WX-F's `finish`, which
`result=both` may combine with the collected replies. At most 64 dispatch entries
(W0501), each method a Rust identifier, each type from the closed vocabulary of
VER04k section 3.2.

**P4.** (Abstraction observer.) An optional `abs=<method>(ref)-><ht>` names a
snapshot method. With `result=collect+abs` the driver calls it after **every** step
and the operation returns `List<(Reply, A)>`: the claim then compares the abstract
state after every call, so a divergence is caught at the step that causes it. A
reference model then also supplies `inv(s)`, and the manifest records
`abs_states_visited`.

**P5.** (`reset`.) `reset=<Ctor>` declares that a `Call` constructor is the object's
reset method; it enables the `reset-equals-new` obligation (P9).

### 6.2 The call alphabet and trace domains

**P6.** (Inherited WX-F obligations.) The generator registers WX-F's `shape`,
`empty`, `deterministic` and `steps-cap` for every dispatch driver, each with its
control (VER04k section 3.5), with `empty-element` where an element is a list or
bytes. Calls per trace are bounded by `MAX_DRIVER_STEPS = 65,536`.

**Call alphabet.** A ground `List<Call>` in the document names the argument values
that matter (an empty feed, a bare CR, a bare LF, a line with a tail). **Trace
domain.** The sequences over that alphabet up to a depth, written
`within(cs, alphabet(), K)` (6.3). Constructor arguments
(`SinglePortRAM::new(depth, width, mode)`) are ordinary binders in front of the trace,
usually refined to two or three shapes. States that need long prefixes (a 510-byte
line, a three-deep stack, a full dictionary) are reached by (a) *prefix models*:
`forall cs | within(cs, A, 4). run(concat(fill_stack(), cs)) == ...` where the prefix is
a model, so the claim stays total; (b) the property generator, traces to length 64
with the alphabet's calls weighted 3/4 and type-generic arguments 1/4; (c) fuzzing,
where each input byte selects a call and following bytes its arguments. A trace
claim over the default `D(List<T>)` of VER04j section 4.7 (length 2 over 16, length
3 over 4) remains legal but is too shallow to be the main evidence: the generator
warns (not fails) when a protocol's only exhaustive claim uses it.

### 6.3 `within`, the alphabet-domain predicate

The predicate and its domain rule are specified in VER04l, which is authoritative. This
document requires only the following of it, and S2 MUST NOT be scheduled before an
implementation exists:

* `within(xs, alphabet, k)` is a total `Bool` predicate: every element of `xs` occurs in
  `alphabet` and `xs` has at most `k` elements (`Text`, `Bytes` and `List<T>`).
* When a binder's refinement has a top-level conjunct `within(binder, A, K)` with `A`
  ground and `K` a literal, the binder's exhaustive domain is exactly the sequences over
  `A` of length at most `K`, ordered by length and then lexicographically in the order of
  `A`; duplicates in `A` are W0301; the size `sum_{i<=K} |A|^i` is computed with checked
  arithmetic and counted against `B` and the 64 MiB limit (W0706 over budget).
* The label is **`bounded-set`**, printed with the alphabet and depth:
  `traces(calls=8, depth<=5) = 37,449` (L3). It is never `exhaustive` (E14).
* Caps: `|A| <= 256`, `K <= 16`.

Sizes to keep in mind: 8 calls to depth 5 is 37,449; 8 calls to depth 6 is 299,593; 7 calls
to depth 6 is 137,257; 4 calls to depth 6 is 5,461.

### 6.4 Obligations for protocols

**P7.** (Protocol obligations, ids `warrant/<module>/driver/<op>/<obligation>`, each with
a control in the WX-F style.)

| Obligation | What runs | Fails when | Control |
|---|---|---|---|
| `observer-pure` | for each method bound with receiver `ref`: insert an extra call of it at every position of 16 traces; the other replies must not change | an "observer" mutates (generalises `finish-observational`) | a wrapper whose `ref` method counts calls into the state MUST fail |
| `instance-isolation` | two objects driven with interleaved traces versus separately | instances share state | a wrapper with a class-level shared buffer MUST fail |

**P8.** (`deterministic` and `order-independence` are mandatory.) As E16: `order-independence`
is registered for every driven operation (the first 256 traces of its suites run in two
orders in one process), with its control.

**P9.** (`reset-equals-new` and `drop-runs`.)

| Obligation | What runs | Fails when | Control |
|---|---|---|---|
| `reset-equals-new` | only when `reset=` is declared: `prefix, reset, ds` versus `ds` on a new object, for 16 prefixes and 16 `ds` | reset leaves residue | a wrapper whose reset keeps one byte MUST fail |
| `drop-runs` | the object is dropped **inside the armed phase** after each trace | a destructor panics or performs an effect, seen by the jail | a wrapper with a `Drop` that makes a forbidden syscall MUST be killed by the gate (J2) |

### 6.5 Faults, and the vacuity of protocols

**P10.** (Fault set; each rewrites the drive plan or the read-back, the crate is never
modified; each applicable one MUST be listed, W0502.)

| Fault | Rewrite | Bug modelled |
|---|---|---|
| `drop-last-call`, `reorder-calls`, `duplicate-last-call`, `early-stop`, `reset-per-call` | WX-F's five (`drop-last-chunk`, `reorder-chunks`, `duplicate-last-chunk`, `early-finish`, `reset-per-step`), renamed for calls | lost, reordered or repeated operations; state re-created per call |
| `skip-call:<Ctor>` (one per constructor whose method has receiver `mut`) | omit every call of that method and reply with its reply constructor's default (the nullary constructor, or the payload's zero) | **a method whose effect no claim observes**. Mandatory per mutating method: the per-method vacuity check |
| `shared-instance` | reuse one object for two consecutive traces | missing isolation |

**P11.** (No observer, no protocol.) A protocol in which **no** `Reply` constructor
carries data makes `run == m_run` trivially true; W0601 at generation. A `skip-call`
fault that no suite catches fails its control (VER04 section 6), which is how a method
whose effect is never observed is found.

### 6.6 The `retained` bound, and protocol totals

A per-call bound (`steps`, `heap`, `output`) cannot say "this object does not grow
without limit". **P12.** `bound <op>: retained <= e` is a new contextual metric (v1
only; **W0101** in v0). It is valid only on an operation executed by a driver
(W0733 otherwise).

* **Meaning.** After each step of the drive, once the adapter has converted the host
  return into a reply and dropped it, the bytes still live and attributable to the
  subject, relative to the bytes live before `new`, are at most `e` evaluated at the
  instance's arguments. `e` is an `Int` or `Nat` expression over the operation's
  parameters (as in VER04j section 6.2).
* **Observation.** The lane's allocation-capped observer (VER04j section 6.3). The
  adapter's own allocations (the collected replies, the converted arguments) MUST be
  excluded, by reading the counter before conversion and dropping the host return
  first; a lane without an observer is `not-live` for `retained` and the target is
  blocked, never run uncapped. The manifest records `max_observed_retained`,
  `retained_bound_at_max` and `steps_observed`.
* **Control.** The fault `retain-extra` keeps `e + 1` bytes alive in the window after
  the first call; the `retained` row MUST fail. A row that no instance can violate is
  therefore caught.

**P13.** (Protocol totals and the caps that keep CWE-400 honest.) `steps` and `heap`
rows on a driven operation bound the **whole trace** as a function of the trace
(`fold` over the calls); per-method rows need S3's `object` block (7.1). Caps:

| Quantity | Cap | Enforced |
|---|---|---|
| calls per trace | `MAX_DRIVER_STEPS = 65,536` | adapter, before `new` |
| trace domain | `B` and 64 MiB domain bytes; `within` alphabets `<= 256`, depth `<= 16` | generator and prelude |
| dispatch constructors per protocol | 64 | binding parser |
| live objects per case | 2 (`instance-isolation`), otherwise 1 | adapter |
| `ensures` and `abs` evaluation | charged to the enclosing meter | evaluator |
| subject heap over a whole trace | one allocation window for the whole drive when the operation has a `heap`, `output` or `retained` row | capped observer |

**P14.** (Manifest.) Per protocol: `calls`, `depth`, `traces`, `calls_executed`,
`steps_max_observed`, `abs_states_visited`; with an `abs` observer and an invariant,
`reached_states_outside_inv` (MUST be 0; a non-zero value fails the claim) and
`inv_states_reached` over `inv_states` (reported, not gated). `calls_executed` is kept
apart from cases (L4). **P15.** (Proofs.) Driven operations carry `proofs_not_live=driver`
(VER04k section 3.4 rule 5).

### 6.7 Worked example: `irc-framing`

The crate (from reading): `Framer::{new, feed(&mut self, &[u8]), frames(&mut self) ->
Vec<Vec<u8>>, reset, buffer_size(&self)}`. `feed` only appends; `frames` drains up to a
line feed, strips a CR before it, and drops a line over 510 bytes **only when its line
feed arrives**. Input with no line feed is therefore retained for ever (a CWE-770
question that a one-shot test cannot ask; logged as an unconfirmed finding, backlog
SEC-011).

```text
warrant irc_framing v1                                -- needs WX-D2, the alphabet-domain predicate, retained
  variant Call = Feed(data: Bytes) | Frames | Reset | Size
  variant Reply = Fed | Framed(lines: List<Bytes>) | Cleared | Sized(n: Nat)

  -- Abstract state: the unframed tail. split_lines (a fold over the buffer, not shown) returns the
  -- complete lines (CR stripped, lines over 510 bytes dropped) and the tail.
  model m_step(buf: Bytes, c: Call) -> (Reply, Bytes) = match c with
    Feed(d) -> (Fed, concat(buf, d))
    | Frames -> (Framed(split_lines(buf).0), split_lines(buf).1)
    | Reset -> (Cleared, b"")
    | Size -> (Sized(len(buf)), buf)
  model m_acc(acc: (List<Reply>, Bytes), c: Call) -> (List<Reply>, Bytes) =
    (concat(acc.0, [m_step(acc.1, c).0]), m_step(acc.1, c).1)
  model m_run(cs: List<Call>) -> List<Reply> = fold(cs, ([], b""), m_acc).0

  model alphabet() -> List<Call> =
    [Feed(b""), Feed(b"61"), Feed(b"0d"), Feed(b"0a"), Feed(b"610d0a62"), Frames, Reset, Size]
  model big() -> Bytes = to_bytes(replicate(97u8, 600))
  model growth_alphabet() -> List<Call> = [Feed(big()), Feed(b"0a"), Frames, Reset]

  op framer_run(calls: List<Call>) -> List<Reply>
  bound framer_run:
    retained <= 1024                                  -- S2; the row the current crate cannot satisfy

  claim refines_model_depth_5:                        -- 8 calls, depth <= 5: 37,449 traces, bounded-set
    forall cs: List<Call> | within(cs, alphabet(), 5). framer_run(cs) == m_run(cs)
  claim split_independent:                            -- M4: compare the replies after the feeds
    forall a: Bytes, b: Bytes.
      drop(framer_run([Feed(a), Feed(b), Frames]), 2) == drop(framer_run([Feed(concat(a, b)), Frames]), 1)
  claim reset_is_new:                                 -- 585 x 585 = 342,225 trace pairs, within B
    forall cs: List<Call>, ds: List<Call> | within(cs, alphabet(), 3) and within(ds, alphabet(), 3).
      drop(framer_run(concat(cs, concat([Reset], ds))), len(cs) + 1) == framer_run(ds)
  claim growth_is_bounded:                            -- 4 calls, depth <= 6: 5,461 traces
    forall cs: List<Call> | within(cs, growth_alphabet(), 6). framer_run(cs) == m_run(cs)
```

The model is the unframed tail; `m_step(Feed)` grows `buf` without limit, so no
invariant `len(buf) <= N` holds, which is the same finding seen from the model side. The
`growth_is_bounded` suite reaches `Feed(big()), Feed(big())` (1,200 bytes with no line
feed) at depth 2, so the `retained <= 1024` row fails on the current crate: that is the
expected finding, and a fix (a maximum buffered length with a defined reply) changes the
model and the row together. Counts (L3): `refines_model_depth_5` is 37,449 traces,
`split_independent` is `D(Bytes)` squared, 347 x 347 = 120,409, `bounded-set`;
`reset_is_new` 342,225 (under `B = 400,000`); each label prints its alphabet and depth.
Every one of them is `bounded-set` and none is `exhaustive`, whatever its size.

Other S2 pilots (sketch only): `block-ram::SinglePortRAM` (`tick(clock, address,
data_in, write_enable) -> Vec<u8>` and `dump` as the `abs` observer; hardware, feeds both
CPU chains); `lz78::TrieCursor` (hidden arena; the model is an association list, which is
independent of the implementation's trie, V12); `der-tlv::DerCursor` is plain WX-F (a
decoder with a borrowed input and `finish`) and should be done as soon as WX-F lands,
before S2.

## 7. Slice S3: deferred

Nothing in this section is required of an S1 or S2 implementation. It is recorded so
that S1 and S2 do not preclude it, and so that the next specification starts from a
stated shape. The recommendation (decisions d3 and d6) is not to ship any of it before
two S2 adopters have written the long form (no feature before an adopter asks).

### 7.1 The `object` block (checked sugar)

The zero-grammar protocol form has two costs: boilerplate (`Call`, `Reply`, `m_step`
with a `match`), and, more important, per-method `input` and `bound` rows have nowhere
to attach because there is only the one `run` operation. **D1.** A thin declaration
fixes both and desugars to exactly the S2 form:

```text
object Framer
  model FramerM = framer_empty()                    -- abstract state type and initial state
  feed(data: Bytes)              = m_feed            -- m_feed(s, data) -> FramerM
  frames() -> List<Bytes>        = m_frames          -- m_frames(s) -> (List<Bytes>, FramerM)
  reset()                        = m_reset
  buffer_size() -> Nat  observer = m_size            -- m_size(s) -> Nat; must not change the state
  invariant framer_inv
```

Desugaring: `variant Framer.Call`, `variant Framer.Reply`, `model Framer.step`, `op
Framer.run(calls: List<Framer.Call>) -> List<Framer.Reply>`, the claim `Framer/refines`, and the
two model lemmas for `invariant`; `input Framer.feed.data:` and `bound Framer.feed:` then name
a method. `object` would be a contextual word at declaration level, as `ensures` was.

### 7.2 The derived inductive row

**D2.** If a package has step claims of reach class R1 or R2 for every step operation,
closure `evidenced` for each, `init` stated, and the E15 obligations passed, then "every
finite trace agrees with the model" follows by induction on trace length. The report MAY
print this as a **derived row** labelled `exhaustive (by induction over: <claim ids>)`, listing its premises and
their strengths, with effective strength the **minimum** of the premises. It is an inference
made by the report tool, so it belongs to the `tcb`; it is printed only when the owner
enables it, and not before the Mode E functional-equivalence proof of 7.3 exists for the
flip-flop to calibrate it. Mode P never yields it (E14).

### 7.3 Proofs

**D3.** In order of value; each is `not-live` until its specification lands.

1. **Mode E functional equivalence of a loop-free step** (`d_flip_flop`, a 4-bit register):
   the step is a straight-line function of a struct, so a bit-blasted equivalence between
   the lowered Rust and the lowered Warrant model is a small miter. VER04h section 7 has
   constant-time and step-bound obligations only; this needs a new obligation kind,
   `functional-equivalence`, whose negative control is a sabotaged step that must yield a
   counterexample. It would give `proved` for exactly the layers the 4004 chain wants and
   is the calibration VER29 question VQ4 asks for. **Funded next, after S1** (decision d6,
   backlog WX-SP1); it lifts `proofs_not_live=state` for the operations it covers.
2. Bounded model checking of a protocol: unroll `K` calls symbolically; label
   `bounded-proof(k=K)` in **steps**, printed as such.
3. k-induction with the declared invariant: `proved` for all traces; research-grade for
   anything with a heap.
4. Timing: "one evaluation per call regardless of data" for a CPU step is a constant-time
   obligation with state fields labelled secret (`labels step: cpu.acc secret`; field-path
   labels exist, VER04j section 4.6); not-live until item 1 exists.

### 7.4 Published state facts

**D4.** One published fact per stateful package would let an upper report refuse to treat
a driven lower contract as inductive: `contract.<pkg>.state.<op>` with value `none |
explicit | driven`, published with VER29's second slice (the non-functional contract
kinds). Until then the strength label and the printed reach class carry the information.

## 8. Honest evidence: labels, counting, and vacuity guards

### 8.1 Labels

**L1.** (LP-012 kinds.) Every generated case is counted in exactly one kind, as backlog
LP-012 requires: `full` (the claim's whole stated domain, every binder complete), `sampled`
(an `edge(N)` or seeded sample), `bounded-set` (a chosen finite set: `Bytes set(347)`, a trace
domain, a family of states). A claim of mixed binders takes the weakest kind among its binders
and its reach class (`sampled` below `bounded-set` below `full`); if LP-012's merged text
orders them differently, that text governs this precedence and this document governs only the
assignments of the table below. Only `full` may be shown as exhaustive.

**L2.** (Rung.) The VER29 label (section 6.1) follows from the kind and the H3 rule (E14):
`full` with the obligations of E15 passed is `exhaustive`; every other kind is
`exhaustive-sampled`. **Both are printed**, because the rung says what a layer above may rely on
and the kind says what was enumerated:

| Claim form | Example | LP-012 kind | VER29 rung |
|---|---|---|---|
| step claim, reach R1 or R2, complete binders, E15 passed | `dff_step_all_states` | `full` | `exhaustive` |
| fixed nested composition from R1 or R2 states | `dff_edge_captures_data` (`steps=2`) | `full` | `exhaustive` |
| step claim, reach R3 (a family) | `add_reg_all_operands`, `set_pixel_matches_model` | `bounded-set` | `exhaustive-sampled` |
| step claim whose E15 obligations are missing, failed, or `not-live` | any | per binders and reach | at most `exhaustive-sampled` |
| step claim over an `edge` or `Nat` binder | a `U64` operand | `sampled` | `exhaustive-sampled` |
| step claim over the ordinary `D(Bytes)` domain | a container over `Bytes` | `bounded-set` | `exhaustive-sampled` |
| trace claim over `within(cs, A, K)` | `refines_model_depth_5` | `bounded-set` | `exhaustive-sampled` |
| trace claim over the default `D(List<T>)` | `[a, b]` over 16 members | `bounded-set` | `exhaustive-sampled` |
| any claim calling a driver-executed operation | `framer_run(...)` | at most `bounded-set` | at most `exhaustive-sampled` |
| `property` and `fuzz` suites | any | not a count kind | `randomized` |
| a claim that calls no operation | a model lemma | not a subject case: `lemma` | none, excluded from published strength |
| `ensures` rows, closure rows | monitored on every call | no cases; `monitored_calls` | none of its own |
| E15 and P7 obligations | `codec-roundtrip` | kind `state` or `driver`, own count line | gate, not a rung |

**L3.** (Trace labels.) A trace claim prints its alphabet and depth in the label:
`bounded-set  traces(calls=8, depth<=5) = 37,449`. `text_over(12 symbols, <=5) = 271,453`
is the form VER04l prints for text.

**L4.** (Counting.) A trace is one case. The manifest also records `calls_executed`, and the
report never presents calls as cases: prefix-closed traces overlap heavily. A step claim
counts one case per admitted binder tuple, and a nested claim also prints `steps=k`.
The headline figure is the `full` count alone.

**L5.** (Steps.) A claim that calls a state operation `k` times prints `steps=k`.

**L6.** (Footprint, not machine.) A claim complete over a footprint inside a fixed frame is
reported as `family(...; distinct=D)`, kind `bounded-set`, with the sentence "complete over
the footprint in one frame; the whole-machine frame is sampled". The count of footprint
cases MUST NOT stand for the machine.

### 8.2 The report block

The derived report prints, per stateful operation and per step claim, lines of this shape
(sketch; the exact digest layout belongs to VER04h section 9):

```text
op d_flip_flop    state=explicit  inv=bit_state  on error: state unspecified (slot S4)
  dff_step_all_states    full         exhaustive           cases=64   steps=1  reach=inv-complete(bit_state) states=16 declared, distinct=16  closure: evidenced (ensures row 2)
  obligations:  codec-roundtrip n/a   rebuild-equivalence n/a   deterministic pass (control ok)   order-independence pass (control ok)
op step           state=explicit  inv=cpu_inv    on error: state unchanged (ensures row 2)
  add_reg_all_operands   bounded-set  exhaustive-sampled   cases=8192 steps=1  reach=family(nibbles; distinct=8192)  closure: evidenced (ensures row 1)
  add_reg_touches_only_acc_carry_pc  bounded-set  exhaustive-sampled  cases=1040  reach=family(wild; distinct=65)
  uncovered:    Trace.mnemonic   error payloads of step
  obligations:  codec-roundtrip pass (control ok)   rebuild-equivalence pass (control ok)   deterministic pass   order-independence pass
op framer_run     state=driven
  refines_model_depth_5  bounded-set  exhaustive-sampled   traces(calls=8, depth<=5) = 37,449  calls_executed=...
```

### 8.3 Vacuity guards

Each guard names the failure it prevents and the rule or control that stops it.

| # | Failure | Prevention |
|---|---|---|
| V1 | **Hidden state makes the pure reading false.** The design rests on "the object is a function of its explicit state or its history" | `codec-roundtrip`, `rebuild-equivalence`, `deterministic`, `order-independence` (E15), `instance-isolation` (P7), each with a control; the VER08b effect and capability evidence that H3 already requires. Residual: vDSO clock reads (J3) |
| V2 | **A depth-5 trace suite read as "the object is verified"**, or 37,449 traces shown as exhaustive cases | E14 (driven is never `exhaustive`), kind `bounded-set` (L1), the trace domain in the label (L3), `calls_executed` apart from cases (L4). **LP-012's split MUST be implemented before either slice ships** |
| V3 | **Refinement that observes nothing**: replies carry no data, so `run == m_run` is trivially true | `skip-call:<Ctor>` per mutating method MUST be caught (P10); a protocol with no data-carrying reply is W0601 (P11) |
| V4 | **A step claim made vacuous by a too-strong invariant** | zero admitted cases is W0601 (E10); admitted, discarded and `distinct` printed; `states=N` with `distinct < N` is W0732 (E6); `reached_states_outside_inv == 0` once an `abs` observer exists (P14); `inv(m_init)` as an example (E8) |
| V5 | **A check that did not run, counted as passed** | an obligation that does not apply prints `n/a` and is never counted as passed (E15) |
| V6 | **Silent omission of API surface** | `omit=` and `strmap` omissions are printed and part of the report digest (E12, E13) |
| V7 | **Unreachable-state noise hiding real failures, or a model weakened to make junk states pass** | the invariant is the only admitted way to exclude states; it is a named, reviewed model with a closure row (E5, E8); a failure on a state inside the invariant is a finding, a failure outside it is reported as such |
| V8 | **The footprint counted as the machine** (8,192 complete cases of a four-field slice of a state with thousands of bits) | reach classes (E6, E7), the label rule (L6), and the mandatory `frame-leak:<field>` faults that force frame claims to exist (E17) |
| V9 | **Model lemmas counted as evidence about the code**: a claim that calls no operation tests Warrant models against each other | a distinct manifest kind `lemma`, excluded from every subject case count and from published strength (E18, L2) |
| V10 | **Shallow traces miss deep states**: depth 5 never fills a 510-byte line, a three-level stack or a full dictionary | prefix models and property and fuzz generation (6.2); a document SHOULD state a `reach` example naming each deep state it relies on and the trace that reaches it, and the report prints `deep states: none stated` when it does not, so the absence is visible |
| V11 | **Composition overstated**: a CPU claim tested over the real ALU is not a proof over the ALU contract | VER29 section 1's sentence stays in every layered report; this feature does not change it (8.4) |
| V12 | **The model is a transliteration of the implementation**, sharper for state machines because a model looks like the code | SHOULD: prefer an algebraic model (a list) over the implementation's structure (a trie); for the CPU chains the model is the datasheet ISA, shared by nine independent ports and by the gate-level implementation, which is real independence. The independent frame claim (5.2) is a second line of defence |
| V13 | **Adapter complexity becomes the trusted base** | closed binding vocabularies validated before generation (VER04k section 6.3), compile-time `shape` pins, round-trip controls; the codec and the dispatch tables are recorded in `tcb` |
| V14 | **Cost** | a depth-6 suite over 8 calls is 299,593 traces of up to 6 calls with a model `fold` each: about 1.8 million subject calls and model steps per claim; fine in Rust, and in Lua or Perl it needs the sharding VER04i question 4 anticipates; the budget `B` and the 64 MiB limit are enforced before enumeration |

### 8.4 What the 4004 chain's strengths come out as

Mechanics are VER29 as written. A CPU step claim assumes the lower layers:

```text
  -- intel4004-gatelevel.warrant
  assume platform:
    contract.arithmetic.alu_add_4_bit_exhaustive
    contract.arithmetic.alu_sub_4_bit_exhaustive
    contract.logic-gates.dff_step_all_states
    contract.logic-gates.register_step_width_4
```

The CPU's ISA model computes ADD with Warrant integers; it never mentions the ALU. The claim
is "one gate-level step equals the ISA step". Evidence is produced by running the real CPU over
the real ALU (VER29 section 1: discharge does not remove test work), and the link records which
lower claims, at which digests and strengths, the CPU claim stands on and caps its effective
strength. Rule (a), coverage: a per-instruction functional claim assumes functional contracts
only. Rule (b), reach-around: `bits.rs` and any host-integer path stay listed as unverified
dependencies. VER29 condition 6 (caller domain) is the weak point for state: the ALU contract
is over nibbles and nothing checks that the CPU only passes nibbles. An implementation invariant
(`acc < 16`, monitored) plus closure is the evidence that addresses it; VQ12 stays open.

| Layer | Contract | Own strength | Why |
|---|---|---|---|
| gates | truth tables | `exhaustive` | VER29 as written |
| adders | 4-bit adder | `exhaustive` | VER29 as written |
| flip-flop | `dff_step_all_states` | `exhaustive` once the E15 obligations pass | R2, `states=16` declared, hidden-state checks evidenced |
| register | `register_step_width_4` | `exhaustive` once E15 passes | R2, `states=256` declared |
| CPU | per-instruction `add_reg_all_operands` | `exhaustive-sampled` | R3: complete over the footprint, whole-machine frame sampled (L6); stays so unless a frame argument is made by proof (7.3) |

Effective strength is the minimum over the chain, so the CPU contract is honestly
`exhaustive-sampled`, and the 4004 report SHOULD say so rather than let the footprint count stand
for the machine (V8, V11).

## 9. Runtime jail and polyglot lanes

### 9.1 The runtime jail (VER08b)

**J1.** (`rcg-baseline-2`.) Long-lived objects that grow call `realloc`; on glibc a large
`realloc` is `mremap`, which `rcg-baseline-1` deliberately excludes (measured as unused by the
`ct-compare` pilot, VER08b section 4). Protocol suites over `Vec`-backed objects will be killed
until the baseline is revised. The plan is `rcg-baseline-2`, adding `mremap` and possibly
`madvise`, **justified by measurement on the first stateful pilot**, as the first baseline was:
the baseline is never widened on expectation, the new version string is recorded in evidence,
and VER08b's spec-sync test lists any added syscall. Until it exists a stateful suite that needs
`mremap` records `not-live` for the runtime rung, never an unarmed run.

**J2.** (Destructors.) The harness leaves through `_exit`, so destructors of live objects never
run. The `drop-runs` obligation (P9) drops every object **inside the armed phase**, so that a
`Drop` implementation performing I/O is seen and killed.

**J3.** (Time and randomness.) Simulators are the crates most likely to read a clock or seed a
hash map. `time:read` through the vDSO is invisible to both rings (VER08b section 9; backlog
CAP-RT-006), so `deterministic` and `order-independence` are the **only** defence there, and the
report says so for every `state=explicit` or driven operation.

**J4.** (Process-wide state.) One test process runs every case; a static cache makes later cases
depend on earlier ones without any syscall. `order-independence` is therefore mandatory (E16, P8).

**J5.** (Threads.) An object that spawns a worker needs `thread`, which the gate refuses (RCG006)
unless `allow_coarse_thread`: such objects are `not-live` for now.

**J6.** (Phase-A warm-up, per lane.) Each lane drives one unrecorded warm-up case per operation in
phase A (before arming), so that lazily initialised runtime state (allocator arenas, hasher seeds,
interpreter caches) is created before the filter is armed. A warm-up drive is never evidence and its
result is discarded; because `order-independence` runs armed, a warm-up cannot hide state shared
between cases.

### 9.2 Lanes

The same contract is Mode E in a functional lane and Mode P in an object lane (3.2). Per lane
(from the memo's reading; each hazard has a control, not an assumption):

| Lane | Object form | Hazard | Handling |
|---|---|---|---|
| Rust | `&mut self`, consuming `self`, `'a`-borrowing cursors | a borrowed input must outlive the object | WX-F `param.<name>` kept alive by the adapter for the whole drive |
| C | `init`/`step`/`free` over a struct | leak, double free, use after free, uninitialised padding in a by-value state compare | adapter owns allocation; binding names `free`; compare field-wise, never `memcmp`; ASan where available |
| C++ | class with destructor, copy and move | a by-value pass copies the state (the `stale-post` bug for real) | binding declares reference passing; the `stale-post` control |
| Go | pointer-receiver methods | a value receiver silently copies | the same control |
| Java, Kotlin, C# | classes | finalizers, static fields | `order-independence`, `instance-isolation` |
| Python, Ruby, Lua, Perl | objects | class-level mutable attributes shared across instances; mutable default arguments | `instance-isolation` (precisely its target) |
| JavaScript, TypeScript | classes, closures | module-level state; typed-array views aliasing arguments | fresh argument copies (WX-F rule) |
| Haskell, OCaml, Elm, F# | `step :: S -> X -> (S, R)` | none: already state-passing | host form `returned`, `receiver: own` everywhere; Mode E is natural even where Rust needs Mode P |
| Elixir, Erlang | pure state-passing, or a process (`GenServer`) | a process needs threads and messages | pure form as Haskell; process form `not-live` (J5) |
| Swift | `mutating` methods on structs, or classes | value versus reference semantics differ by declaration | the binding states which |

### 9.3 Corpus (VER04i)

No new value encoding: calls and replies are WX-A variants (encoded by constructor name,
VER04i section 2.7), states are records. `corpus.json.ops.<op>` gains `"state": "explicit"`
(with the `"inout"` and `"post_slot"` of VER04k section 4.9) or `"protocol": {...}` (the dispatch
table in neutral form); the lane runner implements the dispatch loop once in `warrant-corpus`
(WX-F's 40-line loop plus a `match`). The stream digests need no change: the inputs stream
hashes the call list, the expected stream hashes the model's reply list. Trace domains are
named descriptors (`list_over(<alphabet sha256/8>, K)`, defined with VER04l) so lanes enumerate
in-lane, and `corpus.json` carries the alphabet's canonical encoding. A lane without a state or
dispatch adapter records the operation `not-live` (W0701); it is never counted as covered.

## 10. Security considerations

* **Bindings are untrusted input spliced into generated source.** Every new string (`type`,
  `build.new.fn`, `observe.method`, `dispatch` method names, `states=` claim names, `strmap`
  literals, `omit=` field names) is validated against a closed vocabulary or an identifier or
  path grammar before generation and is never escaped (VER04k section 6.3). A binding that fails
  validation generates nothing (fail closed).
* **Resource caps** (CWE-400): `MAX_DRIVER_STEPS = 65,536`; `B = 400,000` and 64 MiB over the
  product of binder domains before refinement; `within` alphabets `<= 256`, depth `<= 16`;
  dispatch entries `<= 64`; at most 4 `inout` parameters; `codec-roundtrip` at most 4,096 states;
  `rebuild-equivalence` 64 triples; `order-independence` 256 cases; at most 2 live objects per
  case; `ensures`, `abs` and invariant evaluation charged to the enclosing meter. A state record
  with a `Bytes<256>` field costs its payload per element in the domain-byte budget (VER04j
  section 4.7).
* **Retained growth** (CWE-770) is what P12 exists to state; a bound that cannot be met is a
  finding, not a skipped row.
* **Trust base.** The codec, the dispatch tables, the adapter, the `states=` declarations and,
  in S3, the derived rows, are recorded in the report's `tcb` section. `states=N` is the one
  *declared* number in this design and is printed as declared (E6).
* **Subject hazards** are those of VER04k section 6.5 plus J2 to J5.
* **Not an erasure or timing argument.** Reading a state back makes writes observable; it says
  nothing about dead-store elimination (VER04k section 4.12) or timing (7.3 item 4).

## 11. Diagnostics

Existing codes keep their meaning and are reused wherever the condition is one they already
name. Four codes are new. `W0730` to `W0733` were checked unused in every specification on
`origin/main` (the highest code in use is `W0713`) and are a reserved block for this document;
if VER04l allocates the same numbers first, the later document renumbers at merge.

| ID | Slice | Phase | Trigger |
|---|---|---|---|
| **W0730** | S1 | generation (binding against the checked document) | a state-binding rule: `state=explicit` on an operation with no `inout` parameter or on a v0 document; `inv=` absent, not a `(State..) -> Bool` model, or impure; `states=` malformed, naming a claim that is not a step claim of the operation, or given with `inv=total`; a `codec` whose `type` differs from its `struct` path, or whose `build` or `observe` fields do not match the record; `omit=` on a record the adapter must build; a malformed or misplaced `strmap`; a `state=explicit` operation with a proof-producing `labels` or `bound` row and no `proofs_not_live=state` |
| **W0731** | S2 | generation | a dispatch driver inconsistent with its operation: a `Call` constructor missing from or extra in `dispatch`, a `reply` constructor not in `Reply`, a payload mismatch, a dead `Reply` constructor, a consuming receiver, a `reset=` that names no constructor, `abs` with a type mismatch |
| **W0732** | S1 | generation | a step claim that is not complete as declared: `inv=total` with a claim whose reach class is not R1, or `states=<claim>:<N>` with `distinct < N` |
| **W0733** | S2 | checking and generation | a `retained` row on an operation that is not executed by a driver, or `retained` without a lane allocation observer when the lane claims `live` |

Reused codes, with the new uses:

| ID | New uses |
|---|---|
| W0101 | `retained` in a `bound` block of a v0 document (C1) |
| W0301 | duplicate symbols in a `within` alphabet (VER04l) |
| W0501 | a malformed `state=`, `inv=`, `states=`, `omit=`, `codec`, `dispatch`, `abs=` or `reset=` entry, or more than 64 dispatch entries |
| W0502 | a `faults=` list that omits an applicable fault of E17 or P10 (`stale-post`, `frame-leak:<field>`, `skip-call:<Ctor>`, the five call faults) |
| W0601 | a step claim with zero admitted cases (E10); a protocol none of whose replies carries data (P11); a dispatch driver that no claim exercises with two or more non-empty calls |
| W0701 | a fault that does not apply to the operation's shape; a lane with no state or dispatch adapter (`not-live`) |
| W0704 | an `inout` host form (`slice-of`, `vec-of`, `codec`) that cannot carry the parameter's type |
| W0706 | a trace domain or a state domain over the exhaustive budget, including a refined whole-state binder (E10) |
| W0711 | a driver on an operation of a v0 document; the existing conditions, applied to a dispatch driver |
| W0713 | an `inout` list whose element is not an admissible record; the existing `inout` and `ensures` rules |

## 12. Conformance

An implementation of a slice conforms when every item below for that slice holds and every item of
"All slices" holds. "Control" means the negative control named in the rule; a rule without a
passing control is not implemented.

**All slices.**

1. Every v0 document and every wave-3 and wave-4 v1 document regenerates byte-identically (C2);
   `CheckedWarrantModule` gains no field.
2. LP-012's count split is implemented and the report shows only `full` as exhaustive (L1, V2).
3. `lemma` entries never enter subject counts or published strength (V9).
4. An obligation that does not apply prints `n/a` and is not counted as passed (V5); every omission
   is printed (V6).

**S1.**

5. `state=explicit` with no `inout` parameter, on a v0 document, or without `inv=` is refused
   (W0730); `inv=total` with a non-R1 claim is W0732.
6. The reach class of every step claim is derived and printed (E6): the 5.1 claims classify R2(b), the
   5.2 and 5.3 claims R3, a claim over a binder refined by `M(binder)` R2(a), a whole-type binder R1.
7. `states=N` with `distinct < N` is W0732; a root state outside the invariant fails the claim with
   `state-outside-invariant`; a step claim with zero admitted cases is W0601.
8. Kind and rung follow E7 and the table of L2: an R3 claim is `bounded-set` and `exhaustive-sampled`
   although all its binders are `Bool`.
9. The closure row is recognised in the plain, `Ok`-wrapped (S2 and S4) and S3 forms (E8) and
   `closure: open` is printed when absent.
10. `inout List<R>` is admitted with both host forms, a `slice-of` host cannot change length, and the
    read-back is field-wise (E4).
11. The codec is validated (E11); a codec that zeroes one field fails `codec-roundtrip` (control); a
    hidden call counter fails `rebuild-equivalence` (control); the `deterministic` control (the second run
    given a different state) reports a difference for at least one of the 16 cases; a process-wide
    counter fails `order-independence` (control).
12. `stale-post` and each `frame-leak:<field>` are caught on every pilot; deleting the frame claim of
    5.3 makes the `frame-leak` controls fail (E17).
13. `omit=` on a record the adapter must build is W0730; each omission and each `strmap` appears in the
    printed uncovered list.
14. The H3 rule of E14 and VER29 section 6.2 as amended agree: a `state=none` operation is unchanged,
    a `state=explicit` operation is `exhaustive` only under R1 or R2 with E15 passed, and a driven
    operation never is.
15. **Acceptance pilots.** `d_flip_flop` yields 64 `full` cases with `distinct=16` and the closure row;
    `register` width 4 yields 8,192; `set_pixel` yields 8,192 `bounded-set` cases with `distinct=25`;
    one 4004 instruction yields 8,192 with the CPU contract `exhaustive-sampled`. The design is
    wrong if `step` on a register file (VER30 section 4.3) cannot be written; 5.3 is that test.

**S2.**

16. Every `Call` constructor is dispatched once; `dispatch` and `step` are mutually exclusive; a
    consuming receiver is W0731; a failing dispatch does not stop the drive (P2).
17. `observer-pure` is failed by a `ref` method that mutates; `instance-isolation` by a shared buffer;
    `reset-equals-new` by a residue; `drop-runs` by a `Drop` that makes a forbidden call, killed by the
    gate (controls, P7, P9).
18. `skip-call:<Ctor>` is required for every mutating method (W0502) and is caught; a protocol with no
    data-carrying reply is W0601 (P10, P11).
19. A trace claim is `bounded-set` with its alphabet and depth printed and is never `exhaustive`; the
    manifest's `calls_executed` is separate from the case count (E14, L3, L4).
20. `retained` is W0101 in v0 and W0733 off a driver; `retain-extra` fails the row; the adapter's own
    allocations do not count (P12); the `irc-framing` pilot's `retained <= 1024` row fails on the current
    crate and passes on a fixed one.
21. A claim with `abs`: `reached_states_outside_inv` is 0 and a deliberately wrong invariant makes it
    non-zero (P14).

**S3.** Conformance is stated when S3 is specified.

## 13. Amendments in this series

### 13.1 VER29 sections 6.1, 6.2, 14 and decision D26 (H3)

The amended text is in VER29 itself, with a changelog note at its top. In summary:

* **6.2.** The paragraph headed "Stateless and deterministic binding (review item H3)" is replaced by
  "Pure in its explicit state (review item H3, reworded 2026-11-14)": an `exhaustive` suite supports the
  label `exhaustive` only if every operation the claim calls is pure in its explicit arguments (its binding
  declares `state=none`, or declares `state=explicit` with the claim's state set complete and the
  state-completeness obligations passed with their controls), and an operation executed by a driver, or a
  claim over a trace domain, is never `exhaustive`: it is `bounded-set` and at most `exhaustive-sampled`,
  with the depth and call alphabet printed. The paragraph's remark that the sequential layer will not be
  `exhaustive` "until a sequence driver makes the history part of the quantifier" is replaced by the
  statement that it becomes `exhaustive` through step claims over explicit state (this document).
* **6.1.** Row 4 says "pure in its explicit state (section 6.2)" in place of "stateless, deterministic
  binding".
* **14.** Condition 5 ("Stateful layers have trace contracts") is met by a step claim of reach class R1
  or R2 plus closure, both evidenced, and says so; condition 8 ("The code is pure and deterministic") is
  reworded to "pure in explicit state, with state completeness evidenced". The verdict paragraph's
  "(and only once their bindings declare `state none`)" becomes "`state none`, or `state=explicit` with
  the obligations passed".
* **D26** and the test-plan label control follow.

### 13.2 VER04k sections 4.3, 4.6 and 4.12

Also in VER04k, with a changelog note at its top:

* **4.3** `inout` types: `List<T>` is admitted for `T` a `Bool`, a fixed-width integer, `Nat`, `Bytes`,
  `Bytes<N>` **or an admissible record** (M11); nesting of records inside the list's element is as for
  record `inout`. The rule that a tuple is not admissible stands (M5).
* **4.6** host forms: `slice-of:struct:<Name>` (`&mut [Name]`), `vec-of:struct:<Name>` (`&mut Vec<Name>`)
  and `codec:<Record>` (VER04m E4, E11). Slot clarification (M9): with `error=<CASE>`, an S2 or S4 slot may
  be served by a host that returns the remaining components directly; a panic is `Err(E.CASE)` and carries no
  post-state.
* **4.12** the bullet "Not stateful objects" now points to this document.

### 13.3 Backlog and index

`VER-BACKLOG.md`: WX-S0 is "spec drafted (PR pending)"; WX-S1 describes the S1 slice with its pilots (the
`logic-gates` sequential layer first); rows WX-S2, WX-S3, WX-SP1 and CAP-RT-007 are added; two unconfirmed
suspected defects found by reading are logged (SEC-010, SEC-011), each needing a failing test first.
`README.md` indexes this document.

## 14. Open questions

| # | Question | Default recorded here |
|---|---|---|
| O-1 | When may the report print the derived inductive row (D2)? | Not before the functional-equivalence proof for the flip-flop exists (7.3); owner enables it |
| O-2 | Should a claim whose exhaustive domain is over budget be declarable *random-only* (property and fuzz suites only, no W0706), so that a whole-state frame claim `forall s: Cpu | cpu_inv(s)` needs no model-built family (M6)? | No: use a model-built family now; revisit when a second adopter needs it |
| O-3 | Publish `contract.<pkg>.state.<op>` (D4)? | Yes, with VER29's second slice |
| O-4 | Should `states=N` be replaced by a generator-computed support count when the state type is small enough to enumerate under `B`? | Keep the declared count (printed `declared`); compute and cross-check when feasible in a later revision |
| O-5 | `object` block shape (D1) | As sketched in 7.1, after two S2 adopters |
| O-6 | Does a fixed nested composition ever need a depth cap (E14)? | No: its length is fixed in the document; the report prints `steps=k` |
| O-7 | May a claim refined by `within` ever be `exhaustive` under VER29? | No (decision c); revisit only if a layer above needs it |
| O-8 | Inventory: the VER30 figure of 400 crates for the two features is an upper bound (`wasm-validator`, `wasm-module-parser`, `huffman-tree` and parts of `http-core` need recursive or opaque types or maps) | Re-measure with "explicit state", "protocol" and "text-only" as separate tags before scheduling adopters (decision d8) |

## 15. Specification change record

* Initial VER04m (2026-11-14): normative WX-S in slices S1 (explicit state: `state=explicit`, invariants and
  reach classes, the `codec` line, `omit=`, `strmap`, four obligations, `frame-leak`), S2 (protocols:
  `dispatch` drivers, `abs`, trace domains, protocol obligations and faults, `retained`) and S3 (deferred:
  `object`, derived inductive rows, functional-equivalence proofs, published state facts); the labelling and
  vacuity rules of section 8; the jail plan of section 9. Departures from the design memo are M1 to M11 of
  section 1.2. New diagnostics W0730 to W0733. Amends VER29 (6.1, 6.2, 14, D26) and VER04k (4.3, 4.6, 4.12);
  no VER04, VER04h, VER04i, VER04j or VER08b text changes, and no v0 or wave-3 or wave-4 v1 meaning changes.
