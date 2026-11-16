# VER04s — Warrant Recursive Types (WX-R)

**Status:** Specification only. Nothing here is implemented and no code changes
with it. It is the normative text for backlog item WX-R0 (recursive algebraic
types), which VER30 decision 15 lifted on 2026-11-16 to be written **after**
VER04o (collections and the first slice of generics), now merged. The
implementation slices are R1 to R3 (section 19). The Warrant fragments are
**sketches**: none has been run through the checker, names are illustrative
until the R1-S0 errata pass fixes them, and every count in the text was computed
by hand from the stated domains so a reviewer can redo it. Claims about what a
package does are "from reading": the public signatures and the code named in the
text were read, not run. The freeze is lifted **spec first, with adversarial
review**; one adversarial review ran on 2026-11-16 and its findings and fixes
are recorded in section 21. The lane-specialist read of the per-lane table of
8.3 (WX-R-REVIEW, 19.2) is still owed before the lane rollout R2. Section 20.1 lists the two
questions that are the owner's to answer; every other open question carries a
recommended default that the text already applies.

**Parent:** [VER04 §2 to §8](VER04-warrant-language.md) (§3 values and equality, §4
`match`, the acyclic-model rule, §5 static parameters),
[VER04d](VER04d-warrant-type-system.md),
[VER04h](VER04h-warrant-derived-evidence.md) (domains, budget `B`, the controls),
[VER04i](VER04i-polyglot-warrant-evidence.md) (the eighteen lanes, the polyglot rule
that every feature runs in every lane with equal rigour, the registry of 2.6.1),
[VER04j](VER04j-warrant-v1-extensions.md) (tuples, records, variants, the
no-recursion rule of 4.2 that this document relaxes),
[VER04k](VER04k-warrant-v1-wave4.md) (WX-D sequence built-ins and the meter of
2.7, on which every fold here is built),
[VER04l](VER04l-warrant-text.md) (`Text`, `within`, the header `v2`),
[VER04m](VER04m-warrant-stateful-objects.md) (stateful objects),
[VER04n](VER04n-warrant-opaque-types.md) (opaque types),
[VER04o](VER04o-warrant-collections-generics.md) (collections and generics, the
extension point of its section 19 and CM5),
[VER04p](VER04p-warrant-floats.md) (style; the relation `same`),
[VER04r](VER04r-warrant-corpus-v2.md) (`warrant-corpus-v2`: the model IR, slim and
full depth, `MAX_IR_VALUE`, fuel, total-work bounds, the independent oracle),
[VER29](VER29-layer-contract-linking.md) (strength rungs),
[VER30](VER30-warrant-expansion-roadmap.md) (decisions log, decision 15),
[VER-BACKLOG](VER-BACKLOG.md) LP-012 (the honest kinds), and the 2026-11-16
inventory (`code/specs/data/warrant-inventory/summary-2026-11-16.md`,
`method-delta-2026-11-16.md`).

**Section references.** As in VER04n, VER04o and VER04p, where this document needs
a corpus, binding or language rule from another specification it states a
**requirement** on that specification (section 17); it does not edit VER04, VER04d,
VER04j to VER04p, VER04r, VER29, VER30 or `VER-BACKLOG.md`. The one exception is the
allocation registry of VER04i section 2.6.1, whose own rule is that a specification
adds its row in the same PR (15.1). The backlog rows it needs are listed in
section 19.2. Every reference to another specification is to the text as read on
2026-11-16; R1-S0 re-reads them.

**Design source:** VER04o section 19 (what VER04s will need), the 2026-11-16
inventory (recursion is the fifth blocker), and the owner principles of the
program: `.warrant` is the single source of truth, compiled into evidence for
about eighteen language lanes with equal rigour; honest labels (`full`,
`bounded-set`, `sample`); fail closed; layered verification; the runtime
capability jail stays mandatory.

## 0. Why this exists

Warrant values are finite and flat. VER04j 4.2 says it in one line: "the relation
'type mentions type' must be acyclic (W0203 on a cycle)". So a **tree** cannot be
written: not a JSON value, not a CBOR item, not an expression, not a grammar, not
a WebAssembly type. These are the shapes real parsers, codecs and interpreters
are made of, and they are where malformed-input bugs live. The 2026-11-16
inventory counts them:

| Fact (inventory, 2026-11-16) | Number |
|---|---|
| crates that carry the `recursive-types` tag (a public signature reaches a cycle in the type-reference graph) | 344 (343 in the cumulative table) |
| crates unblocked by recursive types **alone**, on top of the lifted baseline | 74 (332 ports) |
| crates unblocked by the four lifted features (floats, collections, opaque types, generics) **without** recursion | 432 |
| the same four **with** recursion | **775** (2,028 ports) |
| share of the 701 crates behind the four features that also need recursion | 38 % (269 crates, 562 ports) |
| genuine untrusted-input parsers and codecs among the newly reachable (read by hand) | about 14: `json-value`, `json-serializer`, `bounded-json`, `json-parser`, `canonical-cbor`, `resp-protocol`, `wasm-types`, `wasm-validator`, `wasm-execution`, `wasm-runtime`, `wasm-wast-parser`, `html-tree-builder`, `state-machine-markup-json-serializer`, `state-machine-markup-json-deserializer` |

The top of the newly reachable list by value is `parser` (137 dependents),
`grammar-tools` (115), `json-value` (38), `json-serializer` (21), `wasm-types` (14),
`interpreter-ir` (68), `state-machine` (20), `canonical-cbor` (5), `symbolic-ir`
(43), `vm-core` (30), `jit-core` (24) and `wasm-validator` (4). Recursive types
unlock **parsers and data formats, not cryptography** (the inventory says so in
as many words), and parsers are where the hostile input arrives.

| Shape | Real API (from reading) | What a flat `op` cannot say |
|---|---|---|
| a recursive codec value | `canonical_cbor::CborValue` (`Array(Vec<CborValue>)`, `Map(Vec<(CborValue, CborValue)>)`, `Tag(u64, Box<CborValue>)`), `decode(&[u8]) -> Result<CborValue, CborError>`, `try_encode(&CborValue) -> Result<Vec<u8>, CborError>` | "`decode` of the canonical encoding of any small tree returns that tree; every accepted input is its own canonical encoding; depth 128 is accepted and depth 129 is `too-deep`" |
| a recursive document value | `bounded_json::JsonValue` (`Object(Vec<(String, JsonValue)>)`, `Array(Vec<JsonValue>)`, ...), `parse`, `serialize_with_depth_limit` | "`parse(serialize(v)) == v` for every small `v`; a depth bomb is rejected, never a crash" |
| an expression tree | `symbolic_ir::IRApply`, `interpreter-ir`, `grammar-tools::GrammarElement` | "evaluation, substitution and printing agree with a model written by structural recursion" |

The design keeps the one principle of VER04k, VER04m, VER04n, VER04o and VER04p:
**the document stays pure and never contains a host notion.** A host tree has
allocation identity, sharing, possibly cycles, a destructor that recurses and a
stack that runs out. A Warrant recursive value has none of them:

* a value is a **finite tree**, inductively defined, equal to another iff the two
  trees are equal node for node (3.1 to 3.5). There are no cycles, no infinite
  values, no laziness and no sharing that can be observed;
* **recursion in a model is structural only**, checked statically, and run under
  a meter (section 4). There is no general recursion and no loop beyond VER04k's
  folds;
* a recursive type has **infinitely many values**, so evidence over one is
  **never `full`**. A claim says "for every tree of depth at most `d`" or "for
  every tree of at most `n` nodes", and the label says `bounded-set` (section 5);
* the host never recurses on its own stack in proportion to the data: every
  adapter and every evaluator is **iterative with an explicit stack and a hard
  cap**, and a host value that is too deep, too big, shared exponentially or
  cyclic is a named failure, never a crash and never a hang (sections 6, 8, 13).

What is added:

| Added | Where |
|---|---|
| recursive and mutually recursive `variant`, `record` types; well-foundedness by inhabitation; the measures `depth`, `cells`, `nodes` | 3 |
| structural equality (iterative), `same` lifted, `ord` and key types extended, `witness` and `least` | 3.5, 3.6 |
| `rec model` with a `decreasing` parameter, the subterm and rank rules, the linear-use rule, the frame and fuel meter | 4 |
| the domain predicate `shape(v, depth <= d | nodes <= n, width <= w, T in [...])`, counting, enumeration, unranking, the default `rec_d1`, quantification forms, labels | 5 |
| resource bounds, depth bombs, exponential domains, the cost model under VER04r | 6 |
| the corpus encoding (no new stream tag, one value one encoding), type-table cycles, descriptors `rec_depth` and `rec_nodes`, the IR level `ir.rec` | 7 |
| the lane binding contract: iterative adapter, per-lane table for eighteen lanes, C structs and ownership, lazy and cyclic hosts | 8 |
| parser-style claims: round trip, malformed input, canonical form, depth bombs | 9 |
| three obligations and their controls, fifteen sabotage controls | 10, 11 |
| honest labels and vacuity guards | 12 |
| security considerations | 13 |
| the pilot `canonical-cbor`, chosen over `bounded-json`, `json-value`, `wasm-types` | 14 |

### 0.1 Slices, and how rules are marked

| Prefix | Content | Section |
|---|---|---|
| `RM1`, ... | the semantic model of recursive values | 3 |
| `RR1`, ... | structural recursion and the meter | 4 |
| `RD1`, ... | domains | 5 |
| `RB1`, ... | resource bounds and cost | 6 |
| `RC1`, ... | corpus and encoding | 7 |
| `RL1`, ... | lane binding contract | 8 |
| `RP1`, ... | parser-style claims | 9 |
| `RO1`, ... | evidence obligations | 10 |
| `RS1`, ... | sabotage controls | 11 |
| `RV1`, ... | labels and vacuity guards | 12 |
| `RT1`, ... | security | 13 |

Slices: **R1** recursive types in the checker, the ground evaluator, the
generator and the Rust and Python lanes, with the pilot `canonical-cbor`; **R2**
the lane rollout; **R3** generic recursive types (`Tree<T>`), which waits for the
generics slice G1 of VER04o. A rule binds the slices that implement what it names.
Section 19 lists the slices and their prerequisites.

## 1. Decisions, and departures

### 1.1 Decisions

The owner decided on 2026-11-16 (VER30 decision 15) that recursive algebraic types
are lifted, to be specified after VER04o, spec first, with adversarial review. The
rest of this table are **recommended defaults** taken here so the specification
is complete; each is reversible by an owner answer in section 20 and none is
presented as already decided.

| # | Decision | Applied in |
|---|---|---|
| a | **Recursive types are lifted** (owner, VER30 decision 15); spec first, with adversarial review before any implementation | the status paragraph, 19 |
| b | **Values are finite trees.** Only inductive data: no cycle, no infinite value, no laziness, no observable sharing. Coinduction is out of scope (18) | 3 |
| c | **Well-foundedness is inhabitation:** every type in a recursive group must have at least one finite value. A type with no base case is refused | 3.2 |
| d | **Recursion in models is structural only**, by a syntactic check with a decreasing parameter, plus the run-time fuel and frame meter of VER04r and VER04k as the second layer | 4 |
| e | **Evidence over a recursive type is never `full`.** It is `bounded-set`, with the bound printed; `sample` for random draws | 5.8, 12 |
| f | **The domain is bounded by depth or by node count**, deterministic, with the size computed before anything is enumerated | 5 |
| g | **No new stream tag.** A value of a recursive type is the ordinary variant (`65`) whose fields nest; one value has one encoding | 7.1 |
| h | **No host recursion in proportion to data.** Adapters and evaluators are iterative with an explicit stack, and every cap is checked before allocation | 6, 8 |
| i | **The runtime capability jail stays mandatory**, and a recursive claim is never published through VER29 above its layer as an unbounded fact | 13, 12 |
| j | **Pilot: `canonical-cbor`** (14) | 14 |

### 1.2 Where this document departs from VER04o section 19

VER04o section 19 listed nine things VER04s would need and said the design was
already shaped for them. Four of its sentences are changed here, each for a stated
reason.

| # | VER04o 19 said | Here | Why |
|---|---|---|---|
| D1 | row 5: "a `fold_rec` over a value is bounded by its leaf count" (a built-in fold) | a user-written **`rec model`** with a `decreasing` parameter and the subterm, rank and linear-use rules (4) | a catamorphism hands the step only the *results* for the children, not the children; `subst`, `simplify` and every function that matches on a child's shape need the children themselves. A fold-only language cannot state them |
| D2 | row 9: "VER04s is expected to need none (a recursive value is a variant, tag `65`); if it does, the lowest free value is `68`" | **no tag is taken**; `68` to `79` stay free (7.1). This confirms what VER04o expected and is listed for the record, not as a change | one value, one encoding: a second tag for the same variant would make two byte strings for one value, which the injective stream of VER04i 2.6 forbids |
| D3 | row 2: "decide whether a recursive type may be a key" | a recursive type **may** be a key type when every non-recursive component is (3.6); `List`, `Set`, `Map` stay non-key | `Expr` with `And(Expr, Expr)` is the common case and needs no host story beyond the one `Bytes` already has; a `List` key needs one (open question RQ-1) |
| D4 | row 4: "only a depth cap is new" | two caps: **nesting depth** and **cells** (6.1) | a tree of 65,536 empty arrays has no leaf and so no weight under the leaf cap of VER04o CT2; cells count every node |
| D5 | a first sketch of the JSON example wrote `Num(Int)` | `Num(I64)` | `Int` exists inside the model IR only and MUST NOT appear in an op signature, binder or value (VER04r 2.1) |

Alternatives considered and rejected: **a built-in `fold_rec` only** (D1);
**general recursion with a fuel limit as the only guard** (rejected: a document
could not be checked for termination, the oracle could not independently confirm
it, and a fuel limit hit by an honest model is a failure of the spec rather than
of the subject); **a new stream tag for recursive roots** (D2); **host values
admitted with sharing and cycles** (rejected: equality and hashing of the value
would depend on the host, 3.7); **lazy or infinite values** (rejected: the domain
would not be enumerable and no label could be honest); **depth measured in
type-graph nodes** (rejected: `Arr([])` inside `Arr([...])` is one level of the
user's tree and two cells, and the user and the corpus must mean the same by
"depth", 3.4).

## 2. Versioning and compatibility

**RM0.** (Header.) Recursive types are a **v2** feature (VER04l 2.1): a document is
v2 exactly when its header is `warrant name v2`. A type declaration that closes a
cycle in a v0 or v1 document is **W0203** as today (VER04j 4.2); a `rec model`, a
`shape` predicate or a recursive type in a v0 or v1 document is **W0101**. The
checker gives the two cases different text: "recursive type in a v1 document" for a
v2 construct, "cycle" for the old rule. Nothing about a v0 or v1 document changes,
which keeps the byte-identity gate of VER04j 1.3 and VER04l 2.3 true.

**RM0a.** (Reserved names.) In a v2 document the following become reserved (W0201 or
W0203 as a declaration or binder name): the declaration word `rec`, the clause
word `decreasing`, the predicate `shape`, the built-ins `depth_of`, `nodes_of`,
`cells_of` and `least`. In a v0 or v1 document they are ordinary identifiers. R1-S0
checks every in-repository v1 and v2 document for a collision, as VER04o CM0a does.
The reserved names of VER04o (`Set`, `Map`, `witness`, ...) apply unchanged, so a
constructor may not be named `Map` or `Set`: the CBOR pilot's map constructor is
`Dict` for that reason.

**RM0b.** (Prelude parts.) `CheckedWarrantModule` gains a `recursion` table (the
groups, their inhabitation results, the `rec` models, their ranks), empty for every
document without the feature. Every new prelude part (the iterative evaluator
frame stack, the unranker, the counting tables) is emitted only for a document that
uses it, so the output of every existing document stays byte-identical.

**RM0c.** (Dependencies.) Structural recursion over a sequence field uses the WX-D
forms (`fold`, `map`, `all`, `any`, `count`), which VER04r keeps in a **reserved**
level `ir.seq` until an amendment defines the v2 meter. A `rec model` that does
not use a WX-D form (an `Expr` with `Not(Expr)` and `And(Expr, Expr)`) needs only
the new level `ir.rec` (7.6) with `ir.core` and `ir.data`. R1 therefore has two
stages (section 19): R1-S1 to R1-S3 need no `ir.seq`; the pilot needs it.
Collections (`Map`, `Set`) appear in a recursive type only after VER04o C1-S1.

## 3. The semantic model

### 3.1 Declarations and values

**RM1.** A **recursive type** is a `variant` or `record` (VER04j 4.2) that
mentions itself, directly or through other declared types, through any chain of
tuples, records, variants, `List`, `Option`, `Set` and `Map`. The surface syntax
is the existing one; the only change is that the acyclicity rule of VER04j 4.2
and CM5 of VER04o is replaced by well-foundedness (3.2):

```text
warrant json_like v2

variant Json = Null
             | Bool(Bool)
             | Num(I64)
             | Str(Text)
             | Arr(List<Json>)
             | Obj(Map<Text, Json>)

variant Expr = Lit(Bool) | Not(Expr) | And(Expr, Expr)

record Node(label: Text, kids: List<Node>)
```

A **value** of a recursive type is a finite rooted tree: a constructor applied
to values of its field types, where a field of type `Json` holds a smaller
value of type `Json`, a field of type `List<Json>` a sequence of them, a
field of type `Map<Text, Json>` a finite partial function to them, and so on. The
set of values of a type is the **least** set closed under its constructors (an
inductive definition). Consequently:

* every value is finite, has a root, and is built by finitely many constructor
  applications;
* two values are equal iff they are the same constructor with equal fields
  (3.5); there is nothing else to a value, in particular no identity;
* `Json` has infinitely many values; `Expr` over `Lit(Bool)` has two values of
  one node, two of two nodes, six of three, and so on (5.6).

Analogy: a value is a **family tree drawn on paper**, finished, with every person
having a finite list of children. It is never a family in which somebody is their
own grandparent, and never an infinite line with no founder.

### 3.2 Well-foundedness: inhabitation

**RM2.** Define `inhabited` by the least fixed point, computed by iteration to a
fixed point on the declared types (at most `MAX_SCC_TYPES = 8` types per group, so at
most 8 rounds per group):

| Type | `inhabited` once |
|---|---|
| `Bool`, every integer type, `Nat`, `Bytes`, `Bytes<N>`, `Char`, `Text` | always |
| `Option<T>`, `List<T>`, `Set<K>`, `Map<K, V>` | always (`None`, `[]`, the empty collection) |
| `Result<T, E>` | always (an error case exists) |
| a tuple or record | every component type is inhabited |
| a variant | **some** constructor has every field type inhabited |

A recursive group is **well founded** iff every type in it is inhabited. Otherwise
the declaration is **W0770**, citing the type and the constructors that would need
a value of the type itself:

```text
variant Chain = Link(Chain)                    -- W0770: no constructor without a Chain field
record Loop(next: Loop)                        -- W0770
variant Ok2 = Leaf | Link(Ok2)                 -- fine: Leaf is the base case
record Node(label: Text, kids: List<Node>)     -- fine: kids may be []
variant Stuck = A(Stuck, U8) | B(Stuck)        -- W0770
```

This is the standard condition for an inductive type to have a least fixed point
with an element, and it is **all** that is checked: it is not required that a
variant has a payload-free constructor (`Node` above has none), only that a
finite value exists. An uninhabited type would make every domain over it empty and
every claim over it vacuous, which is why it is an error and not a warning.

**RM2a.** (Why not "contains a base constructor".) The shorter rule "every
recursive variant has a constructor with no recursive field" refuses `record
Node(label: Text, kids: List<Node>)` and `variant Cbor = ... | Array(List<Cbor>)`
(which does have a base case, `Uint`, but would also be wrongly refused if the
base case were the empty list only). Inhabitation accepts exactly the types that
have a finite value.

### 3.3 Mutual recursion

**RM3.** The **group** of a type is its strongly connected component in the relation
"type mentions type" (computed iteratively, never by host recursion). Types of one
group may be declared in any order within the document: **type declarations are
resolved in a first pass**, so a forward reference to a type is legal in a v2
document. (Models and operations keep the rule "declared before use"; the one
exception is `rec model`, 4.1.) A group has at most `MAX_SCC_TYPES = 8` types
(W0770 above). Example, an expression language with statements:

```text
variant Stmt = Skip | Assign(U8, Expr) | Seq(List<Stmt>) | If(Expr, Stmt, Stmt)
variant Expr = Const(I64) | Var(U8) | Add(Expr, Expr) | Block(Stmt, Expr)
```

`Stmt` and `Expr` form one group; the value `Block(Skip, Const(1))` has two nodes,
one of each type. All measures of 3.4 are taken over the whole group.

**RM3a.** Recursion through `Set` or `Map` is legal **in value position**
(`Obj(Map<Text, Json>)`); in key position it is legal only if the recursive type
is a key type (3.6), by the rule CK1 of VER04o unchanged. The limit "three
`Set`/`Map` levels in one type" of VER04o CK2 and CT2 counts the nesting of
the **type expression**: the recursion edge is a name, not a nested level, so
`Json` has one level. Value nesting is bounded separately by `MAX_NEST` (6.1).

### 3.4 Measures

**RM4.** Four measures, each defined by structural induction, each computed in
checked arithmetic, none by host recursion. In each, a **cell** is every scalar,
every nullary constructor, and every aggregate (constructor application, tuple,
record, `List`, `Option` `Some`, `Set`, `Map`); a map entry is not a cell of its
own (its key and value are children of the map cell).

| Measure | Definition | Used for |
|---|---|---|
| `cells_of(v)` | the number of cells of `v` | the value cap `MAX_VALUE_CELLS`; work charged; the termination measure of 4.4 |
| `nest(v)` | the number of cells on the longest root-to-leaf path (a scalar is 1) | the nesting cap `MAX_NEST` (the decoder and the adapter count it) |
| `depth_of(v)` | the largest number of **nested values of the recursive group** on a root-to-leaf path: a node of the group counts 1, and a `List` or `Map` between two nodes counts nothing | "trees of depth at most `d`" in domains and in claims |
| `nodes_of(v)` | the number of values of the recursive group in `v` | "values of at most `n` nodes" in domains and in claims |

`depth_of` and `nodes_of` are the measures a person means when they say "depth"
and "size" of a tree, so they are the ones the **domain** uses. `Json` examples:

```text
Null                                depth 1  nodes 1  cells 1
Arr([])                             depth 1  nodes 1  cells 2   (the Arr node and the empty List)
Arr([Null, Num(3)])                 depth 2  nodes 3  cells 5   (Arr, List, Null, Num, the I64 scalar 3)
Obj({"a": Arr([Null])})             depth 3  nodes 3
```

A chain is where the measures part company. The measure that bounds work and
stack is `nest` and `cells_of`, **not** `depth_of`: a `Json` chain of depth 128 has
`nest` about 257 (two cells per level, the node and its `List`), which is why
`MAX_NEST` is 1,024 and not 128 (6.1).

`depth_of` and `nodes_of` are total built-in functions on a value of a recursive
type (and, for a type outside any group, 0); a model may call them. They are
**defined by the same iteration the evaluator uses**, so a document cannot
disagree with the corpus about what depth means.

### 3.5 Equality and `same`

**RM5.** `==` on a recursive type is structural: the same constructor and equal
fields, componentwise. It is defined iff it is defined on every component type
(so a recursive type with an `F64` field has no `==`, W0301, as VER04p 4.3 says
for every type that contains a float; it has the relation `same` instead).
Equality is **extensional through collections**: `Obj` holds a `Map`, so two
objects with the same entries are equal (VER04o CM1). The relation `same` of
VER04p **lifts through every constructor, tuple, record, collection and recursion
by structural induction**, exactly as VER04o 3.5 lifts it through map values.

**RM5a.** (Implementation: iterative.) Equality, `same`, the measures and the
canonical encoding are computed with an **explicit work stack** of pairs of cells,
never by host recursion, with the caps of 6.1 checked as it goes. An evaluator
that recurses on the host stack in proportion to `nest` is non-conforming (the
conformance kit of section 16 has a 1,000-deep case). Two values that are equal
in the model but one of which a lane's evaluator shares as a DAG and the other
holds as a tree are **equal**; sharing is invisible (3.7).

### 3.6 The canonical order and key types

**RM6.** VER04o 3.2 defines the total order `ord` on a closed table of key types.
VER04s adds exactly two rows:

| Type | `ord` | Note |
|---|---|---|
| a **recursive** `variant` or `record` | constructor **declaration index** first, then the fields lexicographically in declaration order, depth first, as VER04o already says for a variant | a key type **only if the whole group is**: start with every type of the group assumed a key type, remove any member one of whose fields is then not a key type, and repeat until nothing changes (at most `MAX_SCC_TYPES` rounds). What is left are the key types. So `A = X(B)`, `B = Y(A) \| Z(List<U8>)` has **no** key type: `B` fails on the `List`, and then `A` fails on `B` |
| `Option<K>` of a recursive key type | unchanged | |

`List`, `Set`, `Map` and opaque types stay **non-key**, so `Json` and `Node` are
not key types (they contain a `List` or a `Map`) while `Expr` is:

```text
variant Expr = Lit(Bool) | Not(Expr) | And(Expr, Expr)     -- a key type
Lit(false) < Lit(true) < Not(Lit(false)) < Not(Lit(true)) < Not(Not(Lit(false))) < ...
                      < And(Lit(false), Lit(false)) < And(Lit(false), Lit(true)) < ...
```

`ord` compares the **constructor index first**, so every `Lit` is below every `Not`
and every `Not` below every `And`, whatever their sizes: `Not(And(Lit(false),
Lit(false)))` (4 nodes) is **below** `And(Lit(false), Lit(false))` (3 nodes), because
the root constructors are `Not` and `And`. So `ord` is **not** the enumeration order
of 5.3 (which is by size first), and neither is derived from the other; a domain is
enumerated in the order of 5.3 and a `Set` or `Map` of trees is canonicalised in the
order of `ord`.

**RM6a.** (Totality.) `ord` is total on a recursive key type by induction on
`nodes_of` of the smaller value: for two distinct values, walk both trees in
pre-order; the first position at which the constructor indices or the scalars
differ decides, and such a position exists because two distinct finite trees differ
somewhere, and a proper "prefix" case cannot arise because the walk compares
constructors, whose field counts are fixed by the constructor. `ord` is consistent
with `==`: `cmp(a, b) = 0` iff `a == b`, so VER04o's strict-order decoder rule CC2
applies to a `Set` of trees unchanged. The comparison is **iterative** (RM5a).
`ord` is **not** the bytewise order of the stream encoding (the stream carries
constructor *names*, and `ord` uses declaration *indices*), and nothing needs it to
be: what the encoding needs is that `cmp(a, b) = 0` exactly when the two encodings
are the same byte string, which is the injectivity of 7.1. A consumer MUST compare
decoded values with `ord`, never encoded bytes.

**RM6b.** (The constraints of VER04o 6.2.) A recursive type is `Eq` iff it has `==`;
it is `Ord` iff it is a key type; it is **never `Finite`**: every type of a
well-founded group has infinitely many values (RM2 makes every type inhabited, so
a value of the type can be wrapped by the recursive constructor again, without end).
It is never `Bits`.

**RM6c.** (`witness` and `least`.) VER04o 5.1 gives every key type a short list
`witness<T>()` for a variant: "for each constructor in declaration order, the
value built from the first member of each field's list (at most 8)". For a
recursive type a recursive field has no "first member" yet. The built-in
**`least<T>()`** is the first value of `shape(v, nodes <= n)` for the least `n` at
which the domain is non-empty, with `width <= 2` and the default alphabets of 5.5,
taken in the enumeration order of 5.3: the smallest
tree, ties broken by constructor index. `witness<T>()` of a recursive type uses
`least<T>()` for every recursive field. For `Expr`: `witness = [Lit(false),
Not(Lit(false)), And(Lit(false), Lit(false))]`. (`least<T>()` is also what the
fault `rec_truncate` of 11.1 puts in place of a pruned subtree.) For a type that
is not a key type, `least<T>()` still exists; `witness<T>()` is only for key types.

### 3.7 What a recursive value is not

* **Not a graph.** A host structure with sharing (`let a = Arr([n, n])` where both
  elements are one object) is, as a model value, the tree with the subtree
  **twice**. Its `cells_of` counts it twice. A host structure with a cycle is not
  a value at all and is the failure `value-depth` or `value-cells`, whichever cap
  the walk reaches first (8.2). Hash-consing, DAG
  equality and cyclic data are out of scope (18).
* **Not lazy.** A value is fully constructed. A host value that is a thunk, a
  generator or an infinite stream is forced by the adapter under the caps of 6.1
  or refused (8.5).
* **Not a handle.** A tree that a host library hands out as a mutable object with
  identity is an opaque type with a recursive-value observer (VER04n), or a VER04m
  object; the value type is the *observation* of it. An opaque type inside a
  recursive type, and a recursive value as an `inout` parameter or in a stateful
  object's state in the first slice, are **W0770** (VER04n OK2 and VER04o CK10 apply
  unchanged).
* **Not a reference.** Two fields equal as values may be two host objects; the
  adapter never compares host identity (VER04r 4.2 rule 5).

## 4. Structural recursion and folds in models

### 4.1 Surface

Rule 1 of VER04r 4.2 says "expressions are trees; there is no recursion, no
loop and no mutable state; model `m` may call only models declared before it", and
VER04k 2.8 extends it to function references (a cycle is W0203). Recursive values
need recursive models. The new declaration is the **`rec model`**:

```text
rec_decl = "rec" "model" NAME "(" params ")" "->" type "decreasing" NAME "=" expression NEWLINE ;
```

`decreasing p` names the one parameter on which the recursion is structural. A
`rec model` and the models it reaches that are themselves `rec` form a **group**
(the strongly connected component of the call graph, direct calls and function
references together). Rules for where a `rec model` may be called and may call:

* a `rec model` may be called, and may reference other `rec models`, **forward
  or backward**: the headers of all `rec model` declarations are collected in a first
  pass (the one exception to "declared before use");
* a `rec model` may call an ordinary `model` declared **before it**, never one
  declared after, and never an `op` (VER04 rule: models do not call the subject);
* an ordinary `model` may call a `rec model` only if the `rec model` is declared
  **before it** (so the ordinary acyclic rule is unchanged for them);
* at most `MAX_REC_MODELS = 16` models per group.

### 4.2 The structural recursion check

**RR1.** (Derived terms.) In a `rec model` with decreasing parameter `p`, the
**derived terms** of `p` are the least set containing, for every term `t` already
derived **or equal to `p`** (a *derived-or-equal* term):

1. a variable bound by a pattern of `match t with ...` at a position **below the
   root** of the pattern (`Arr(xs)` binds `xs`; `And(l, r)` binds `l` and `r`;
   `Some(x)` binds `x`; a tuple pattern `(a, b)` binds both);
2. a projection `t.i` or `t.field` of a tuple or record;
3. `at(t, i)` when `t` is a sequence;
4. the element parameter of the function reference in a WX-D form
   `fold(t, init, f)`, `map(t, f)`, `all(t, f)`, `any(t, f)`, `count(t, f)` when `t`
   is a sequence (a `List`) whose terms are derived, **for the callee `f`**: this
   is how a `fold` step receives a child, and it is the only way a callee other than
   the caller receives one without a `match`;
5. `map_values(t)`, `map_entries(t)` and `set_elems(t)` **used only as the
   sequence argument** of a WX-D form, in which case the element parameter of `f`
   is a derived term (a map entry `e` and its projections `e.0`, `e.1` are
   derived).

A call is anything that applies a group member: a direct call `g(args)`, or a
function reference `g` or `g(bound args)` as the function argument of a WX-D form.

**RR2.** (Strict and level calls.) Every call to a member of the group inside a
`rec model` `f` is one of:

* a **strict call**: the argument in the callee's decreasing position is a **derived
  term** of `f`'s decreasing parameter (below the root); or
* a **level call**: the argument in the callee's decreasing position is `f`'s
  decreasing parameter **itself** (the same variable), and the callee is declared
  **later in the document** than `f` (its *rank*, the position in document order
  among the group, is higher).

Any other call, including a call on a constructed value, on a value returned by
another call, on a value derived from a different parameter, or a call with the
decreasing position unfilled, is **W0771**.

**RR3.** (Why this terminates.) The measure of a call is the pair `(cells_of(arg),
MAX_REC_MODELS - 1 - rank(callee))` under the lexicographic order. A strict call
lowers the first component (a derived term is a proper subtree, so it has fewer
cells). A level call keeps the first and lowers the second, because it goes to a
member of higher rank and ranks are below `MAX_REC_MODELS = 16`. Both components
are natural numbers, the pair strictly decreases at every call, and there is no
infinite descending chain in `(N, {0..15})` ordered lexicographically, so
every chain of calls is finite, and therefore every evaluation terminates, **and
the number of nested calls on one path is at most `cells_of(p) * MAX_REC_MODELS`**.
This is the "size-change" argument of the structural-recursion checkers of Coq,
Agda and Lean, with `cells_of` as the size and declaration order as the tie-break.

**RR4.** (Linear use: the polynomial bound.) Termination alone allows exponential
cost: `f(Node(l, r)) = add(f(l), f(l))` calls `f` on `l` twice at every level, so
the work is `2^depth`. The checker therefore also requires that along every
**evaluation path** (the arms of `if` and `match` are different paths; the two
operands of `and`, `or`, `implies` and a `fold` over one sequence are one path) the
**decreasing arguments passed to group members are pairwise disjoint**: no two
calls receive the same term, or a term and one of its derived terms. The argument
of a **level call** (the decreasing parameter itself) counts, and it overlaps every
derived term, so a path with a level call has no other call to a group member.
Two terms `at(t, i)` and `at(t, j)` are disjoint only when `i` and `j` are
different literals; `at(t, i)` overlaps a WX-D form over `t`. A `fold` over a
derived sequence `xs` with a group member counts as **one** use of `xs`. A
violation is **W0771** with the text "term used by two recursive calls". Under this
rule each cell of the decreasing value is the decreasing argument of at most one
invocation per member of the group, so the invocation count is at most
`MAX_REC_MODELS * cells_of(p)`; the number of invocations **within one group** is
linear in the size of the data, not exponential in its depth.

**RR4a.** (Calls between groups: what the bound is not.) Groups form an acyclic
order (a `rec model` of one group that calls a `rec model` of another cannot be
called back by it, or the two would be one group). A call from a member of group
`G` to a model of another group `H` is **not** restricted by RR4, and `H` may walk
the whole argument again. So a chain of `s` groups, each calling the next at
every node (a `depth_like(l)` called beside `f(l)`), costs up to `cells^s`
invocations: **polynomial of degree `s`, not linear**. Termination (RR3) is not
affected. The static bound of 4.5 is computed with this in it, and the fuel of
RR7 is the limit that actually stops a costly model; a document is not refused
for it.

What RR4 refuses, and the remedy, stated so authors are not surprised: a model
that visits the elements of one list **twice** by two recursive folds on the path
(`count_deep(xs) + sum_leaves(xs)` where both recurse) is refused; the remedy is one
fold whose accumulator is a tuple. Calls to **non-recursive** models are not
restricted, and a non-recursive use of a derived term (`len(xs)`) is free.
Open question RQ-2 asks whether a document may declare a small constant multiplicity.

**RR5.** (Mutual recursion through function references.) The step function of a
`fold` is a member of the group when it reaches the caller again (the usual case);
VER04k 2.8's W0203 for a cycle through a function reference is **lifted for exactly
the cycles that pass through `rec model` members**, and stays W0203 for every cycle
among ordinary models. VER04k 2.8's `MAX_HO_NESTING = 4` is unchanged: a chain of
function-reference edges deeper than 4 is W0602.

### 4.3 Worked examples

An expression language, evaluated by structural recursion (a *strict* call on
each child, the two children disjoint):

```text
variant Expr = Lit(Bool) | Not(Expr) | And(Expr, Expr)

rec model eval(e: Expr) -> Bool decreasing e =
  match e with
    Lit(b)    -> b
  | Not(x)    -> not eval(x)                 -- strict: x derived from e
  | And(l, r) -> eval(l) and eval(r)         -- strict, disjoint: l and r
```

A document that writes `And(l, r) -> eval(l) and eval(l)` is W0771 (`l` is used by two
calls). A document that writes `Not(x) -> eval(Not(x))` is W0771 (a constructed
value). A document that writes `And(l, r) -> eval(e)` is W0771 (a level call must
reach a *later* member, and `eval` is itself).

The JSON writer of the pilot family, with a `fold` step in the same group (the step
is declared **before** `ser`, so the level call `step -> ser` goes to a later member):

```text
rec model arr_step(acc: Text, x: Json) -> Text decreasing x =
  if acc == t"" then ser(x) else concat(concat(acc, t","), ser(x))   -- level: x itself, ser is later

rec model ser(j: Json) -> Text decreasing j =
  match j with
    Null     -> t"null"
  | Bool(b)  -> if b then t"true" else t"false"
  | Num(n)   -> dec_text(n)                                          -- an ordinary model
  | Str(s)   -> quote(s)                                             -- an ordinary model
  | Arr(xs)  -> concat(concat(t"[", fold(xs, t"", arr_step)), t"]")  -- strict: elements of xs
  | Obj(m)   -> concat(concat(t"{", fold(map_entries(m), t"", obj_step)), t"}")

rec model obj_step(acc: Text, e: (Text, Json)) -> Text decreasing e =
  concat(concat(acc, ...), concat(concat(quote(e.0), t":"), ser(e.1)))   -- strict: e.1 derived from e
```

`ser` calls `arr_step` through `fold(xs, ...)` with `xs` a pattern-bound field of
`j`: each element is a derived term, so the call is strict. `arr_step` calls `ser`
on its own decreasing parameter `x`: a level call, legal because `ser` is declared
after `arr_step`. The cycle `ser -> arr_step -> ser` contains a strict edge, so by
RR3 it descends. The elements of `xs` are each passed to exactly one invocation of
`arr_step` (the one use of `xs` in the fold), and `x` is passed to exactly one call
of `ser`, so RR4 holds. (The `if acc == t""` separator test is an ordinary
expression; the real pilot model keeps a `(Text, Bool)` accumulator; the text is a
sketch.)

A counter-example, to show the check is not vacuous: `rec model bad(j: Json) -> Nat
decreasing j = match j with Arr(xs) -> bad(Arr(xs)) | _ -> 0` is W0771 (the
argument is a constructed value, not a derived term, even though it equals `j`).

### 4.4 Evaluation: frames and fuel

**RR6.** (Frames, not host depth.) A call of a `rec` member pushes one **frame** on
the evaluator's explicit stack. VER04k 2.7 charges *evaluation depth* as nested node
evaluations across model calls with a limit of 64 levels. A structural recursion
over a tree of `nest` 257 cannot fit that, and must not be forced to: for the
members of a group the depth counter is kept **per frame** (the 64-level limit now
bounds the expression nesting *inside one frame*) and the number of **live frames**
is bounded by `MAX_REC_FRAMES = 4,096`. (4,096 is four frames per nesting level at `MAX_NEST`; the pilot's `enc` uses at
most two.) Exceeding either is exhaustion: `W0703` at
checking and ground evaluation, and in a lane the case failure `rec-frames`
(subject-dependent) or the corpus error of the same name (subject-independent),
under the split of VER04r 3.2. The ordinary models keep the 64-level rule of
VER04k 2.7 exactly, which keeps every v1 document byte-identical.

**RR7.** (Fuel.) The **fuel** of an evaluation is VER04k's work-unit counter, limit
`MAX_IR_FUEL = 1,000,000` (the same number VER04k 2.7 gives the ground evaluator;
there is one meter and three implementations of it: the ground evaluator, the
generated model code and each lane's IR evaluator). Every node evaluation and every
frame push charges as VER04k 2.7 and VER04j 4.5 say (one unit, plus one per field
touched). Exhaustion is `ir-fuel`: a **corpus error** if the evaluation is
subject-independent, a **case failure** if a `subject_op` sits beneath it, exactly
the split of VER04r 3.2 for `ir-overflow`. This is the run-time step meter that VER04r 4.2 rule 6 said was unnecessary *because the IR
had no recursion*, now necessary and added.

**RR8.** (Two layers.) Structural recursion (RR1 to RR4) is the first layer: it
guarantees termination and a polynomial bound **for a conforming document**. Frames
and fuel are the second layer: they bound a **hostile or buggy corpus** that the
first layer was never run on, and they are enforced by every consumer regardless of
what the checker said. A consumer MUST NOT skip the meter because the corpus
declares a model `rec`; it MUST check that every `rec` annotation is consistent
(the decreasing parameter exists, the group is the call-graph component, forward
references stay inside the group) and the independent oracle recomputes RR1 to RR4
from the source (10.2 of VER04r).

### 4.5 Termination, restated for a reviewer

* every `rec model` call chain is finite (RR3);
* its length is at most `cells_of(p) * 16`, and the number of invocations of the
  members of one group on one argument is at most the same (RR4);
* the **static bound** `fuel_bound(G, c)` of a group `G` on an argument of at most
  `c` cells is `c * (sum over the members of G of the static cost of the body)`,
  where a body's static cost is the node count of VER04r 4.2 rule 6 (at most
  `MAX_IR_COST = 2,000`) with a call to a member of `G` counted as one node, a call
  to an ordinary model at its full cost, and a call to a `rec model` of another
  group `H` at `fuel_bound(H, c)` (RR4a);
* the bound **exists only** for a group none of whose bodies, and none of whose
  callees, uses a form whose charge depends on the size of a value: a WX-D form
  over a sequence (charged per element, VER04k 2.7), a collection built-in
  (charged per leaf, VER04o 3.6) or a constructor of a recursive value (charged
  per cell copied, RB6). A group that uses one has **no static bound** and is
  `dynamic`;
* the worst case is far above the fuel: 16 members of cost 2,000 on
  `cells_of(p) = 65,536` is `2.1e9`, **above** `MAX_IR_FUEL = 1,000,000`. Hence the
  fuel is a real limit and the report prints, per suite, whether fuel was **proven
  sufficient** (a static bound exists and, for the largest value in the domain,
  is under the cap), **measured** (the evaluation is subject-independent, so the
  generator ran every case and none exhausted, RB7) or **dynamic** (6.5). The
  pilot's writer `enc` folds over lists and concatenates, so it has no static
  bound; it is subject-independent, so it is `measured` (14.5).

## 5. Domains for evidence

A recursive type has infinitely many values, so a domain over it is a **bounded
subset**: a fixed, deterministic, size-bounded enumeration. It is **never** `full`.
This section defines the bounded domains, how they are counted, ordered and
unranked, and what a claim over them says.

### 5.1 The predicate `shape`

**RD1.** One built-in predicate, no grammar, in the manner of `within`
(VER04l 4.2) and `subset_of` (VER04o 5.3):

```text
shape(v, depth <= d, width <= w, T1 in L1, T2 in L2, ...) : Bool
shape(v, nodes <= n, width <= w, T1 in L1, ...)           : Bool
```

* `v` is a value of a recursive type (the group `G` of that type). **Only one
  recursive group may be reachable from the type of `v`**: a type of a second
  group reachable through a field (`Stmt` holding a `Json`) is **W0772** in R1,
  because a bound on the nodes of `G` does not bound the other group and the
  domain would be infinite. The remedy is a `rec model` that builds the inner
  value, or one group;
* exactly one of `depth <= d` or `nodes <= n`, with `d` a literal `1..=16` or `n` a
  literal `1..=64`: `depth_of(v) <= d` or `nodes_of(v) <= n` (3.4);
* `width <= w`, a literal `0..=4`, default `2`: every `List`, `Set` and `Map`
  field of every node of `v` has at most `w` entries;
* `T in L` for each **atomic leaf type** `T` reachable in `G` (a scalar, `Bool`,
  `Bytes`, `Bytes<N>`, `Text`, `Char`, `F32` or `F64`): `L` is a **ground** list
  (a literal or a model call with no free binder) of **at most 16 distinct values**
  of type `T`, and every leaf of type `T` anywhere in `v` is a member of `L`. The
  position of a member in `L` is its index in the enumeration (5.3). A leaf type
  with no clause has the default alphabet of 5.5; a float leaf has no default and
  needs its clause (members distinct under `same`, VER04p). A leaf of an `error`
  type takes no clause: its alphabet is every case, in declaration order.
  One alphabet serves every leaf of the type, wherever it sits: a tag number and a
  length that are both `U64` share it.

As a predicate it is total and has its ordinary meaning anywhere an expression is
allowed. When a binder's refinement has a **top-level conjunct** `shape(binder,
...)` with a ground alphabet and literal bounds, the binder's domain is exactly the
set the predicate describes, and the exhaustive domain of the binder is that set
(**the domain rule**, the precedent of VER04l 4.3). The checker rejects, as
**W0772**: a binder that is not of a recursive type; both or neither of `depth` and
`nodes`; a bound outside its range; a leaf type that does not occur in the group; a
duplicate clause; a duplicate or ill-typed member in `L`; an `L` with more than 16
members or not ground; a `T in L` for a non-atomic `T`; a second recursive group reachable from the
binder's type; an opaque type reachable from it; a float leaf with no clause; a
`nodes` shape over a type with a `Set` of group values or a `Map` keyed by them
(RD2); a shape whose domain is
empty (no value of the group satisfies it, which RM2 prevents for `nodes <= n` once
`n` reaches `least`, but a small `depth` or an `L = []` for the only base leaf can
reach it); a `shape` conjunct under `or`, `not` or `implies` (it must be a
top-level conjunct to be a domain).

### 5.2 The shape of a domain, and its size

**RD2.** The domain `S` of a binder `v: T` under a `shape` is defined by the following
**closed recurrences**, computed in checked arithmetic with saturation at `2^40`
(a size that saturates is over the budget, never wrapped).

Let `A(X)` be the number of members of the leaf alphabet of atomic type `X`.

For `depth <= d`: let `V_k(U)` be the number of values of type `U` (a group member)
with `depth_of <= k`, and `V_0(U) = 0`. A constructor `C(f1: F1, ..., fm: Fm)` of `U`
contributes `P_k(C) = product of c_k(Fi)` where `c_k(F)` is the number of values of
field type `F` at bound `k`:

| `F` | `c_k(F)` |
|---|---|
| an atomic leaf type `X` | `A(X)` |
| a type of the group | `V_{k-1}(F)` |
| a non-recursive `variant` outside the group | the sum over its constructors of the product of `c_k` of their fields (the same rule, so the leaf alphabets apply inside it; a declared type is expanded structurally, never given its "ordinary" domain, which for a record of two `U64` would be `2^128`) |
| `Result<F', E>` | `c_k(F')` plus the number of cases of `E` |
| `Option<F'>` | `1 + c_k(F')` |
| a tuple or record | the product of its components |
| `List<F'>` | `sum for i = 0..w of c_k(F')^i` |
| `Set<F'>` with key alphabet or child domain of size `s` | `sum for i = 0..min(w, s) of C(s, i)` |
| `Map<K, F'>` with key alphabet or child domain of size `s` | `sum for i = 0..min(w, s) of C(s, i) * c_k(F')^i` |

and `V_k(U) = sum over constructors C of U of P_k(C)`. A group is solved
simultaneously: `V_k` of every member from `V_{k-1}` of every member. A constructor
with a recursive field has `P_1 = 0` (no value of depth 0 exists); a constructor
whose recursive field sits under a `List`, `Set` or `Map` still has `P_1 >= 1` (the
empty collection), which is why `Arr([])` is a depth-1 value.

For `nodes <= n`: let `N_k(U)` be the number of values of `U` with **exactly** `k`
nodes of the group, and for a field type `F` let `c(F, m)` be the number of values
of `F` that hold exactly `m` nodes of the group:

| `F` | `c(F, m)` |
|---|---|
| an atomic leaf type `X` | `A(X)` if `m = 0`, else 0 |
| a type `U` of the group | `N_m(U)` (0 for `m = 0`) |
| `Option<F'>` | `[m = 0] + c(F', m)` |
| a tuple or record `(F1, ..., Fj)` | `T_1(m)`, where `T_i` counts the fields `Fi .. Fj`: `T_{j+1}(m) = [m = 0]` and `T_i(m) = sum for a = 0..m of c(Fi, a) * T_{i+1}(m - a)` (the first of them takes `a` nodes) |
| `List<F'>` | `sum for i = 0..w of L_i(m)`, where `L_0(m) = [m = 0]` and `L_i(m) = sum for a = 0..m of c(F', a) * L_{i-1}(m - a)` (the first element takes `a` nodes) |
| `Set<X>` of an atomic leaf type with alphabet size `s` | `sum for i = 0..min(w, s) of C(s, i)` if `m = 0`, else 0 |
| `Map<X, F'>` keyed by an atomic leaf type with alphabet size `s` | `sum for i = 0..min(w, s) of C(s, i) * L_i(m)` with `L` taken over `F'` |
| a non-recursive variant, `Result` | the sum over the alternatives, as for depth |

`[m = 0]` is 1 when `m` is 0 and 0 otherwise. A constructor `C(F1, ..., Fj)` of `U`
contributes to `N_k(U)` the tuple count `T_1(k - 1)` of its fields, and `N_k(U)` is
the sum over the constructors. `N_k` depends only on `N_1 .. N_{k-1}`, so the table
is filled by ascending `k`. A `Set` of group values, and a `Map` whose **key** is a
group type, have no such recurrence (the count of sets of distinct trees with a
given total size is not a convolution) and are **W0772** in a `nodes` shape; the
`depth` form covers them. The domain is `S = union of N_k for k <= n`.

**RD3.** (Checked before any enumeration.) A consumer computes `|S|` from RD2 before
it generates anything, and refuses a domain with `|S| > MAX_SUITE_PRODUCT = 2^28`
or a product over the budget `B` (VER04r 5.3): **W0706** at generation. A domain that
the author did not size (the default `rec_d1`, 5.5) is **demoted** to the largest
`d` or `n` that fits, recorded in the manifest as VER04r 5.3 requires. A demoted
binder is labelled **`sample`**, exactly as VER04r 9.3 labels every demoted binder,
and its scope line says `demoted from` (RV3): the smaller domain was chosen by the
budget, not by the author, so it is not presented as the author's bounded set. An
author who wants `bounded-set` writes the `shape`. If not even `nodes <= n` at the
least non-empty `n` fits, there is nothing to demote to and it is W0706.

### 5.3 Enumeration order

**RD4.** The order of the values of a domain is a **total, deterministic order that
every lane and the oracle implement identically**, never the host's:

* **`depth <= d`.** The domain is the variant descriptor of VER04r 5.1 applied to
  a *tree of descriptors*: the values of `U` at bound `k` are the concatenation, **in
  constructor declaration order**, of the product of the field domains, **last
  field varying fastest**, where a recursive field uses the domain of bound `k-1`.
  A `List` field is by length `0..=w`, and for each length **over the whole child
  domain**, lexicographic by child index, last position fastest (this differs from
  the `list` descriptor of VER04r 5.1, which reduces the alphabet; a shape never
  reduces it, because its size is already bounded by `w`). A `Set` field is by
  size `0..=w`, then by the ascending indices of the chosen members. A `Map` field
  is by size `0..=w`, then by the ascending indices of the chosen key-alphabet
  members, then the values lexicographic, last fastest. An `Option` field is
  `None` first.
* **`nodes <= n`.** By **node count `k` ascending**, and within one `k`, the
  constructors in declaration order, and within one constructor the values of its
  field tuple holding `k - 1` nodes, ordered as the recurrences of RD2 are
  written, outermost choice slowest: a **tuple** `(F1, ..., Fj)` holding `m` nodes
  by the node count `a` of `F1` ascending, then the values of `F1` with `a` nodes
  in their own order, then the tuple `(F2, ..., Fj)` holding `m - a` nodes in the
  same way (so the last component varies fastest); a **`List`** by length
  ascending, then as a tuple of that many elements; an **`Option`** `None`
  first; a **`Map`** by size ascending, then the ascending indices of the chosen
  keys, then its values as a tuple. Unranking divides by the count of the
  remainder at each step, so it never lists compositions. For two group-typed fields this is "the compositions of `k - 1` with
  the first slot's count ascending", which the worked example of 5.6 lists. The
  conformance case `rec/nth-nodes-list` (16) pins the order for a type with a
  `List` field, where a reader is most likely to choose differently.

**RD5.** (`nth`, `size`.) A descriptor of this kind has the two total functions of
VER04r 5.1, `size()` and `nth(i)`, so chunk jumping works. `nth(i)` is an
**unranking** over the counting tables of RD2: find the node count (or constructor)
by comparing `i` with the cumulative sizes, subtract, and recurse **iteratively**
into the field domains by mixed-radix division. The tables hold one row per type of
the group and per field type or tuple prefix or list length that the recurrences of
RD2 name, each of `max(d, n) + 1` entries; a descriptor whose tables would exceed
`MAX_SHAPE_TABLE = 65,536` entries is W0706 (a corpus error `shape-size` in a
consumer). They are built once per
descriptor at load, in checked arithmetic; a table entry that saturates, or a
declared `size` that differs from the recomputed one, is a corpus error
(`shape-size`). `nth` is `O(cells of the result * constructors)` and never
`O(i)`. The committed corpus carries the descriptor, never the values; the oracle
(VER04r 10) recomputes the tables and the unranking from RD2 and RD4 with its own
code and fails the corpus if they differ.

### 5.4 Descriptors in `corpus.json`

**RD6.** Two new nodes, resolved at generation so a lane never evaluates a
refinement (VER04r 5.1):

```json
{"d":"rec_depth","t":"Json","depth":3,"width":2,
 "leaves":{"Bool":["false","true"],"I64":["0","1"],"Text":["", "a"]},
 "size":"79011"}
{"d":"rec_nodes","t":"Expr","nodes":5,"width":2,
 "leaves":{"Bool":["false","true"]},"size":"66"}
```

`leaves` carries each alphabet as literals in the JSON form of its type
(VER04r 2.3), at most 16 per type; `size` is decimal text, recomputed by the
consumer (RD3). A recursive `shape` has **no `witness`/`one_of` resolution**: the
alphabets are the whole input. The descriptor tree is at most 16 levels deep
(VER04r 5.1); a `rec_*` node is a leaf of that tree (its own recursion is derived
from the `types` table, bounded by `d` or `n`, never by a nested descriptor).

### 5.5 The default domain `rec_d1`

**RD7.** A binder of a recursive type with **no** `shape` conjunct has the domain
`rec_d1`: `shape(v, depth <= d, width <= 2, <default alphabets>)` for the **largest
`d <= 3`** whose `size` is at most `2^20 = 1,048,576`; if even `d = 1` exceeds it,
the largest `n` with `nodes <= n` that fits. The default alphabets, so a default
domain contains the interesting boundary values and stays small:

| Leaf type | Default alphabet |
|---|---|
| `Bool` | `false, true` |
| unsigned `w`-bit, `Nat` | the first two members of `witness<T>()`: `0`, `2^(w-1)-1` |
| signed `w`-bit | the first two members of `witness<T>()`: `MIN`, `-1` |
| `Char` | `U+7F`, `U+80` |
| `Bytes` | `b""`, `00` |
| `Bytes<N>` | the first two members of `witness<Bytes<N>>()` |
| `Text` | `t""`, `t"a"` |

A document that needs the boundary values of a format (the CBOR argument
boundaries `23 24 255 256 65535 65536 2^32-1 2^32 2^64-1`) states them in a
`T in L` clause (14.3). `rec_d1` is **`bounded-set`** when it stands as defined
here (the choice of `d` by the size of the domain alone is its definition, and the
scope line prints the `d` chosen), and **`sample`** when the product with the
suite's other binders forced a further demotion (RD3). It is never `full` (5.8).

### 5.6 Worked examples

**Expression tree, by node count** (`Expr = Lit(Bool) | Not(Expr) | And(Expr,
Expr)`, `Lit` alphabet `false, true`). `N_k` is the number of values with exactly
`k` nodes: `N_1 = 2`, and for `k > 1`, `N_k = N_{k-1} + sum over i + j = k-1 (i, j >=
1) of N_i * N_j` (the `Not` term and the `And` term):

| `k` | `N_k` | from |
|---|---|---|
| 1 | 2 | `Lit(false)`, `Lit(true)` |
| 2 | 2 | `Not` of the 2 values of size 1 |
| 3 | 6 | `Not` of the 2 of size 2, plus `And` of `(1,1)`: `2*2 = 4` |
| 4 | 14 | `Not` of the 6 of size 3, plus `And` of `(1,2)` and `(2,1)`: `4 + 4` |
| 5 | 42 | `Not` of the 14 of size 4, plus `And` of `(1,3)`, `(2,2)`, `(3,1)`: `12 + 4 + 12` |

So `shape(e, nodes <= 3)` has `2 + 2 + 6 = 10` values and `nodes <= 5` has
`2 + 2 + 6 + 14 + 42 = 66`. The ten, in the order of RD4:

```text
index  value                          index  value
0      Lit(false)                      6      And(Lit(false), Lit(false))
1      Lit(true)                       7      And(Lit(false), Lit(true))
2      Not(Lit(false))                 8      And(Lit(true),  Lit(false))
3      Not(Lit(true))                  9      And(Lit(true),  Lit(true))
4      Not(Not(Lit(false)))
5      Not(Not(Lit(true)))
```

`nth(7)`: the cumulative sizes are `2, 4, 10`; `7 >= 4`, so `k = 3` and the offset
is `7 - 4 = 3`; at `k = 3` the constructors contribute `Lit` 0, `Not` 2, `And` 4,
so the offset `3 >= 2` selects `And` with offset 1; the only composition of `3 - 1 =
2` into two slots is `(1, 1)`, with `2 * 2` values; offset `1` is left index `1 / 2 =
0` and right index `1 mod 2 = 1`: `And(Lit(false), Lit(true))`. Matches the table.

**JSON-like tree, by depth** (`Json` of 3.1 with `Num(I64)`, alphabets `Bool`:
`false, true`; `I64`: `0, 1`; `Text`: `t"", t"a"`, `width <= 2`). `V_k` is the number
of values of depth at most `k`: the leaf constructors give `1 + 2 + 2 + 2 = 7`
(`Null`, `Bool`, `Num`, `Str`) at every `k >= 1`; `Arr` gives `sum_{i<=2} V_{k-1}^i`; `Obj`
(keys from the `Text` alphabet, `s = 2`) gives `sum_{i<=2} C(2, i) V_{k-1}^i`.

| `k` | `V_k` | computation |
|---|---|---|
| 1 | 9 | `7 + 1 + 1` (`Arr([])` and `Obj({})`) |
| 2 | 198 | `7 + (1 + 9 + 81) + (1 + 2*9 + 1*81) = 7 + 91 + 100` |
| 3 | 79,011 | `7 + (1 + 198 + 39,204) + (1 + 396 + 39,204) = 7 + 39,403 + 39,601` |
| 4 | 12,485,713,284 | `7 + 6,242,817,133 + 6,242,896,144` |

`depth <= 3` is a domain of 79,011 values, comfortably inside the budget;
`depth <= 4` has 12.5 **billion** and is refused before one value is generated:
`12,485,713,284 > 2^28 = 268,435,456` (W0706). That is the exponential domain that
section 6.4 guards against, in numbers.

### 5.7 How claims quantify

**RD8.** A claim over a recursive type says one of four things, and the report
prints which.

| Form | Example | Quantifies over | Strength |
|---|---|---|---|
| **R1 binder over trees of bounded depth** | `forall v: Json \| shape(v, depth <= 3, ...). parse(ser(v)) == Ok(v)` | **every** value of the group of depth at most 3 with the alphabets given | `bounded-set`, with the scope line (12.2) |
| **R2 binder over trees of bounded size** | `forall e: Expr \| shape(e, nodes <= 5). ...` | every value of at most 5 nodes | `bounded-set` |
| **R3 nodes of a given value** | `all_nodes_ok(v)` written as a `rec model` | exactly every node of **that** value, whatever its size | exact for the value; the strength comes from the binders that produced it |
| **R4 whole-value equality** | `decode(b) == Ok(v)` | structural `==` at once | the label of the binders |

**RD9.** (What a bounded claim does and does not say.) "For every tree of depth at
most 3" is a **small-scope** statement: bugs that need depth 4, a wider list or a
leaf outside the alphabet are outside it, and no argument from the domain to the
unbounded type is made (this is the same statement VER04o 6.7 makes for generics:
"one type suffices" is not evidence). The report prints the scope (12.2). The
`proof` rung is `not-live` for every recursive claim until an induction story
exists, acknowledged in the binding as in VER04m and VER04n; symbolic discharge of a
recursive claim is never `proved` (VER10), and a lower layer's recursive claim is
**assumed above, never re-proved** (decision i).

**RD10.** (`property` and `fuzz`.) A property or fuzz suite over a recursive binder
draws from the `shape` space by **`nth(i)` with `i` uniform in `0..size`**, from
the engine's PRNG; it is `sample`. Larger shapes (`nodes <= 64`, `depth <= 16`)
are reachable only this way, and a document that wants a deep value uses a model
that builds it (`fold` over `range(n)`), 9.5.

### 5.8 Labels

**RD11.** The kinds are the three of LP-012 (VER04o 12.1). The rule for recursion:

| Claim form | Kind | VER29 rung |
|---|---|---|
| `shape` or `rec_d1` over a recursive binder, any alphabets | **`bounded-set`** | `exhaustive-sampled` at most |
| a recursive binder **demoted** by the budget (RD3) | **`sample`** (VER04r 9.3: every demoted binder) | `randomized` at most |
| the same, all other binders also exhaustive | `bounded-set` (the weakest binder; **never** `full`, because the type is infinite) | `exhaustive-sampled` |
| `property` or `fuzz` over a recursive binder | `sample` | `randomized` |
| a `reference` claim, a `predicate` claim | independent expected values versus `relational` (VER04r 4.3) | a `relational` suite never counts as an independent reference |
| a claim whose binders are all non-recursive but whose **result** is a recursive value (`decode(b) == Ok(v)` with `b` from a finite alphabet) | the label of its binders | as before |

"Exhaustive" is never a total. A document MUST NOT claim `full` for a recursive
binder (W0772 on a `label full` assertion in a binding, and the report never prints
it). The leaf alphabets can be `full` for their own types; the **tree** is not.

### 5.9 Chunking, `slim` and `full`

**RD12.** Chunks are on **product ordinals** exactly as VER04r 5.4: a binder of
a recursive `shape` contributes `size(shape)` ordinals, and `nth(i)` is the
unranking of RD5, so a lane unranks the first ordinal of a chunk and enumerates
forward. The `slim` selection of VER04r 5.6 is unchanged (the base set from the
suite id, the witness chunks, the added chunks), and a `slim` run **never** claims
`full`. Three things are particular to trees.

**RD12a.** (What a chunk of trees is.) A chunk is a contiguous range of ranks. In
the `nodes` order the first chunk holds the smallest trees and the last the
largest. In the `depth` order the ranks go **by root constructor in declaration
order**, so the first chunk holds values of the first constructor, whatever their
size (for `variant T = Node(T, T) | Leaf` they are the largest trees, not the
smallest), and within one constructor the leading fields are fixed while the last
field's subtree varies. So what `slim` thins in a large tree domain is the **root
constructor and the leading children**: a constructor whose values all fall
between two selected chunks is not run at `slim` at all. This is the tree form of
the statement VER04r 5.6 makes about leading binders, and it is a coverage
reduction that the label `slim(n of m chunks)` already declares.

**RD12b.** (Constructor chunks.) So that a `slim` run cannot silently miss a whole
constructor, the generator adds to `slim_chunks`, for every binder with a `rec_*`
descriptor and every constructor of the binder's type that has a value in the
domain, the chunk that holds the **lowest admitted case** whose value for that
binder has that root constructor. The oracle recomputes them, as it does witness
chunks. They count against `MAX_SLIM_CHUNKS = 32`; a suite that would exceed it is
a generation error, never a shorter list (the remedy is a smaller shape or fewer
claims per suite). A constructor with no admitted case is printed in the report
(`ctors_admitted 8 of 9`), because a claim that never sees one constructor is
weaker than its scope line suggests.

**RD12c.** (Small domains.) For a domain of at most 16 chunks (the 812,160-ordinal
`canonical-cbor` domain of 14.4 has 13) `slim` is every chunk and the two rules
above add nothing. The witness chunk of a tree fault (11.1) is the lowest
admitted case at which the faulted value differs; a claim whose subject call does
not take the binder directly (`decode(enc(v))`) has no stand-in and its witness is
`unproven` (VER04r 5.6), so its controls must fail inside the slim set or the
lane fails.

## 6. Resource bounds and the cost model

### 6.1 The constants

Every bound below is checked **before the allocation** it guards, in checked
arithmetic, by every consumer (generator, oracle, each lane's decoder, adapter and
evaluator). A violation is a **corpus error** (exit 2) when the corpus is the
cause, and a **case failure** when the subject's value is (VER04r 3.2). The values
are the reference profile; `corpus.profile` may **lower** any of them, never raise.

| Constant | Value | Guards | Failure id |
|---|---|---|---|
| `MAX_SCC_TYPES` | 8 | types in one recursive group | W0770 |
| `MAX_REC_MODELS` | 16 | `rec` models in one group | W0771 |
| `MAX_NEST` | 1,024 | cells on one root-to-leaf path of any value (stream decode, adapter, evaluator, descriptor output) | `value-depth` |
| `MAX_VALUE_CELLS` | 65,536 | cells of any one value whose type contains a recursive type, **counting a shared subtree once per path** | `value-cells` |
| `MAX_REC_FRAMES` | 4,096 | live `rec` frames in one evaluation | `rec-frames` |
| `MAX_IR_FUEL` | 1,000,000 | work units of one evaluation (VER04k 2.7) | `ir-fuel` |
| `MAX_SHAPE_DEPTH`, `MAX_SHAPE_NODES`, `MAX_SHAPE_WIDTH` | 16, 64, 4 | the literals of a `shape` | W0772 |
| `MAX_SHAPE_ALPHABET` | 16 | members of one leaf alphabet | W0772 |
| `MAX_SUITE_PRODUCT` | `2^28` | product ordinals of one suite (VER04r 14, unchanged) | W0706 |
| `MAX_TOTAL_FUEL_SLIM`, `MAX_TOTAL_FUEL_FULL` | `2^30`, `2^34` (provisional, RB8) | fuel summed over a run (6.5) | W0706 |
| `MAX_SHAPE_TABLE` | 65,536 entries | the counting tables of one descriptor (RD5) | W0706, `shape-size` |

The existing bounds apply unchanged to what a recursive value *contains*: 4,096
bytes in a `Bytes` leaf, 4,096 entries in one `List`, `Set` or `Map`
(`MAX_COLLECTION`), `MAX_VALUE_LEAVES = 65,536`, `MAX_CASE_LEAVES = 262,144`
(VER04o CT2), the corpus parser limits of VER04r 14 (256 JSON levels, TFV's 64
containers). `MAX_VALUE_CELLS` and `MAX_NEST` apply **only** to a value whose type
contains a recursive type, so a document with no recursion is byte-identical.

**RB1.** (Why `MAX_NEST` is not 128.) The crates in the inventory cap their own
nesting at 128 (`bounded-json`, `canonical-cbor`); an honest depth-128 value must
be representable by the harness, or the claim "depth 128 is accepted" cannot be
made. A chain of 128 `Tagged` values has `nest` = 128 `Tagged` cells plus the
`Nil` cell, 129; a chain of 128 `Arr([...])` has two cells per level (the `Arr` and
its `List`), 257. `MAX_NEST = 1,024` leaves a factor of about four for a subject
that nests more than it should, so its failure is `value-depth` (the harness
rejects the value) and not a crash.

### 6.2 What is checked statically

* the group count, the inhabitation fixed point, and the size of every `shape`,
  by RD2, at checking; a size over `2^28` is **W0706** at generation, before the
  first value;
* every `rec` model against RR1 to RR4, with the diagnostic naming the call and the
  rule;
* the static cost of every body against `MAX_IR_COST = 2,000`;
* a **literal** value in a document (an example, a probe, a table row) against
  `MAX_NEST`, `MAX_VALUE_CELLS` and the **reader limits that bind it**: a
  materialised value in `corpus.json` costs about three JSON nesting levels per
  `Arr` level (the variant object, its `fields` array, the list array), so the 256-level
  strict reader caps a materialised `Json` at about 85 levels, and `cases.tfv`'s
  64-container limit (VER04r 2.5) at about 21. These limits bind **materialised**
  values only (examples, sabotage probes, WX-E rows). A deep value is **built by a
  model** (`fold` over `range(n)`) or **enumerated** from a descriptor, never
  written out, and a materialised value over the reader's limit is a corpus error
  (`value-depth`) at generation, not a truncation.

### 6.3 Depth bombs

A **depth bomb** is a small input that asks for a deep structure: `[[[[...` of
129 bytes, `c0 c0 ... 00` of 130 bytes, a `Tagged(0, Tagged(0, ...))` value of 129
levels. Three places can fail:

| Place | Defence | Failure |
|---|---|---|
| the **subject** (a parser, a serialiser, an encoder) | its own depth limit; the **claim** is that the limit holds: input at the limit is accepted, one past it is the closed error, never a crash and never a stack overflow (9.5) | a wrong result is a claim failure; a crash of the runner process is exit 1 `subject-crash` (8.6), never a pass |
| the **adapter** (host value to canonical value, and back) | iterative with an explicit stack, `MAX_NEST` and `MAX_VALUE_CELLS` checked as it walks (8.2) | `value-depth`, `value-cells` |
| the **corpus consumers** (stream decoder, evaluator, oracle) | the same caps, explicit stacks, `MAX_REC_FRAMES` | the corpus error of the same name |

The distinction matters: the *subject's* limit of 128 and the *harness's* limit of
1,024 are different numbers for different layers, and the claim about the first
needs the second to be larger.

### 6.4 Exponential domains

A domain over a recursive type grows as a tower: `V_4` of the JSON example above is
`1.2 * 10^10` from `V_3 = 79,011`. Three rules keep it honest:

**RB2.** The size is computed by RD2 **first**, saturating at `2^40`, and compared
with `MAX_SUITE_PRODUCT`, the budget `B` and the total-work bounds of VER04r 14
**before** any value is generated or any descriptor is written. An author-chosen
`shape` that is too big is **W0706**; it is never silently shrunk. Only the default
`rec_d1` is demoted, and the demotion is recorded (5.2, RD3).

**RB3.** A product with other binders multiplies: `shape(...)` of 79,011 values
with a `U8` binder of 256 is 20.2 million ordinals, over `2^24 = 16,777,216`, the
budget `B` of VER04r 5.3. The checker reports the product with the factor that
broke it.

**RB4.** The **table** of counts is itself a resource: it is computed once per
descriptor, has at most `MAX_SHAPE_TABLE` entries (RD5), and every entry is
saturating; a consumer builds it **before** accepting the descriptor's `size`.

### 6.5 The cost model under VER04r's IR-cost rules

VER04r 4.2 rule 6 bounds an expression statically (`MAX_IR_COST = 2,000` node
evaluations, a `model_call` charged at its body's full cost), says "because the IR
has no loop and no recursion, this static bound is the whole fuel", and bounds a
**value** by `MAX_IR_VALUE` before allocation. Recursion changes the first and
extends the second:

**RB5.** (Static, per body.) The cost of a `rec model` **body** is computed as for
any model, with each call to a group member counted as **one node**. It must be at
most `MAX_IR_COST` (a corpus error otherwise). The cost of the **whole recursion**
is dynamic and is charged to the fuel of RR7, one unit per node evaluation and per
frame, exactly as VER04k 2.7 charges a `fold` (one unit per element plus the callee's
own charge).

**RB6.** (Values.) Every value an evaluator produces is checked against
`MAX_VALUE_CELLS`, `MAX_NEST` and, for its sequences, `MAX_IR_VALUE = 65,536`
**before the allocation**, from sizes the operands already carry (each composite
caches its `cells` and `nest`, so the check is `O(1)` per constructor). A
constructor of a recursive value charges **one unit plus the cells of its fields
counted as if copied** (the rule of VER04k 2.7 and VER04o 3.6: a model that shares
structure and one that copies reach exhaustion at the same instruction). Sharing
inside an evaluator is an optimisation that MUST NOT change a charge or a cap, and
`cells` counts a shared subtree once per path (3.7).

**RB7.** (The prediction, and the three modes.) For a **subject-independent**
evaluation (a `rec model` applied to binders and models only: the writer `enc(v)`,
the admission `canonical(v)`) the generator runs every case, **refuses to emit a
corpus in which any case exhausts** (W0703 at generation, as VER04k 2.13 does
today), records the fuel per suite (`fuel`), and every lane and the oracle
reproduce the number exactly, as `wraps` and `expected_calls` are reproduced
today. The mode is **`measured`**: exhaustion cannot happen in a lane that
evaluates correctly, and a lane whose total differs fails. For a
**subject-dependent** evaluation (a `rec model` applied to a subject's result) no
measurement is possible before the run. If `fuel_bound` of 4.5 exists for the
largest value the call can see (`MAX_VALUE_CELLS` unless the op's result type
bounds it lower) and is at most `MAX_IR_FUEL`, the mode is **`proven`**. Otherwise
it is **`dynamic`**: exhaustion is a possible case failure (`ir-fuel`) and is
counted.

**RB7a.** (The run total.) The total fuel of a run is the sum, over the suites it
executes and their control re-runs, of the measured fuel of subject-independent
evaluations plus, for each subject-dependent one, `cases * min(MAX_IR_FUEL,
fuel_bound)` (`MAX_IR_FUEL` where no bound exists). It must not exceed
`MAX_TOTAL_FUEL_SLIM` for the slim set and `MAX_TOTAL_FUEL_FULL` for every chunk
(**W0706** otherwise, with the suite that contributes most named). The case
bounds `MAX_TOTAL_WORK_SLIM` and `MAX_TOTAL_WORK_FULL` of VER04r 14 apply as well;
neither replaces the other. The report prints the mode per suite as `fuel:
measured`, `fuel: proven sufficient` or `fuel: dynamic`.

**RB8.** (Slim and full totals.) The run totals are in **fuel**, because the case
counts of VER04r 14 (`MAX_TOTAL_WORK_SLIM = 32,000,000` cases) no longer bound work
when one case can cost a million units: `MAX_TOTAL_FUEL_SLIM = 2^30` (about 10^9
node evaluations; at the speed of an interpreted lane, minutes) and
`MAX_TOTAL_FUEL_FULL = 2^34`. The two constants are **provisional** and so is
`MAX_IR_FUEL`'s adequacy for the pilot: by hand, the pilot's three claims over
812,160 ordinals at a few hundred units each (the admission on every ordinal, the
writer on every admitted case) come to the order of `10^9`, which is **at** the
slim total, not comfortably under it. R1-S0 measures the Python and Lua
evaluators and the pilot's models and **fixes** the two totals, in either
direction, before any corpus that uses `ir.rec` is committed; after that a
`corpus.profile` may only lower them. If the measured pilot does not fit a total
that the interpreted lanes can run in PR CI, the pilot's alphabet is narrowed
(fewer `U64` boundaries per suite, the boundaries split across suites); the
constants are not raised to fit a document.

## 7. Corpus encoding (`warrant-corpus-v2`)

### 7.1 Tags: none taken

**RC1.** A value of a recursive type is encoded with the **existing** tags of
`warrant-stream-v2` (VER04i 2.6, VER04r 2.4, VER04o 8.2): a variant is `65`, u32-LE
length, UTF-8 constructor name, u32-LE field count, fields in order; a record or
tuple is `64`; a `List` is `30`; an `Option` is `40` or `41`; a `Set` is `66`; a
`Map` is `67`. **VER04s takes no stream tag.** The registry rows for `68` to `79`
stay free, and the line of VER04i 2.6.1 that says VER04s "if it needs one, takes
`68`" is rewritten to say VER04s needed none (15.1).

Why none: the stream is **injective**, one value one byte string (VER04i 2.6, the
reason VER04r 2.4 added counts and names so nested composites cannot collide). A
second tag for "a recursive variant" would give the same value two encodings, one
per tag, and every digest in the program would then depend on which a lane chose.
The nesting a recursive value needs is already expressed by the existing tags, each
self-delimiting: a count or a length says where a list, a name or a field list ends.
If R1-S0 finds a genuine need (for instance a back-reference tag, which only a DAG
story would need and which is out of scope, 18), it takes `68`, the lowest free
value, in the same PR as its use.

Worked example, `Tagged(24, Nil)` of the `Cbor` type of 14.2 (`Tagged` is 6 bytes,
`54 61 67 67 65 64`; `Nil` is 3, `4e 69 6c`; the `U64` `24` is `10`, then 8 bytes
little-endian):

```text
65 06000000 546167676564 02000000      -- variant Tagged, 2 fields
   10 1800000000000000                 -- field 1: U64 24
   65 03000000 4e696c 00000000         -- field 2: variant Nil, 0 fields
```

and `Not(Lit(true))` of the `Expr` type: `65 03000000 4e6f74 01000000 65 03000000
4c6974 01000000 01` (a `Bool` field is the single byte `00` or `01`).

**RC2.** (Decoder rules for nesting.) A decoder of a stream that may hold a recursive
value MUST be **iterative with an explicit stack** (it never recurses on the host
stack in proportion to input), and MUST, as a corpus error and never repaired: reject
a value whose `nest` exceeds `MAX_NEST` (`value-depth`), whose `cells` exceed
`MAX_VALUE_CELLS` (`value-cells`), a count above `MAX_COLLECTION` or one whose
elements cannot fit in the bytes that remain (**before** allocating: a decoder MUST
NOT size a buffer from a count beyond `min(count, 4,096)`, VER04o CT3), a variant name
that is not a constructor of the expected type, a field count that differs from the
constructor's, and any trailing byte. Nested counts multiply, so the **cell** cap is
what bounds a stream of `65 .. 02000000 65 .. 02000000 ...`: the decoder counts cells
cumulatively across all nesting before it allocates (the same cumulative counting as
VER04o CT2 does for leaves). The decoder re-encodes what it decoded and compares the
bytes, as VER04l 7.2 does.

### 7.2 The type table

**RC3.** VER04r 2.2 rule 1 says "a record or variant may not contain itself, directly
or through any chain (no recursion; the depth of a type expression is at most 16)".
In the `warrant-corpus-v2` profile with the level `ir.rec` it becomes: **a cycle in
`types` is legal iff every type in the cycle is inhabited (RM2)**, the group has at
most `MAX_SCC_TYPES` types, and the depth of a type *expression* is still at most 16
(a recursion edge is a name, not a nested expression). The reader computes groups
and inhabitation **iteratively** and, before using a type, checks that it
terminates in at most `MAX_SCC_TYPES * (types + 1)` rounds (a hostile table cannot
make it loop). A type row in a recursive group carries the field `"rec":
"<group>"` (**required** for a member of a group and **forbidden** for any other
type, so the canonical JSON of a table has one form), where `<group>` is the least member name of the group; the reader
**recomputes** it and a different value is the corpus error `type-cycle`, so a
corpus cannot assert a group it does not have. A cycle in a v1 profile, or one with
an uninhabited member, is `type-cycle` too.

```json
"types": {
  "Json": {"kind":"variant","rec":"Json","ctors":[["Null",[]],["Bool",["Bool"]],
            ["Num",["I64"]],["Str",["Text"]],["Arr",["List<Json>"]],["Obj",["Map<Text,Json>"]]]}
}
```

### 7.3 JSON and TFV

**RC4.** Values keep the forms of VER04r 2.3 and VER04o 8.3 (a variant is
`{"ctor":"Name","fields":[...]}`, a `Map` is an array of `[key, value]` pairs in
strict key order). Because the reader's limits bind materialised values (6.2), a
corpus MAY materialise only small recursive values; its sabotage probes and its
WX-E rows are such. Everything large or deep is produced by a descriptor or a
model, so the corpus stays small and the independent oracle recomputes it.

### 7.4 The IR level `ir.rec`

**RC5.** One new feature level (VER04r 4.1). A suite, op, model or fault that uses it
lists `requires: ["ir.core", "ir.data", "ir.rec"]` (and `ir.seq` when it applies a
WX-D form, RM0c, and `ir.coll` when its types hold a `Set` or a `Map`); a lane that
does not declare the level records the suite `not-live`, never skipped.

| Tags | Meaning |
|---|---|
| a model row with `"rec": {"group": G, "rank": k, "decreasing": "p"}` | a `rec model` (RR1); `model_call` and function references to it are the ordinary nodes |
| `depth_of nodes_of cells_of` | the built-ins of 3.4 |
| `least` with a type | the built-in of RM6c |
| `eq` and `same` extended | structural equality through recursion (RM5) |
| **fault-only:** `rec_drop_child rec_dup_child rec_swap_children rec_truncate rec_ctor_next rec_wrap rec_limit_low rec_accept` | the rewrites of 11.1, refused in a claim, a model, an `admit` and an `ensures` (as `recipe_is` is in VER04n OC5 and `coll_drop_first` in VER04o CC6) |

Lowering stays **syntax-directed and unoptimised** (VER04r 10.3): one IR node per
source construct, no inlining of a `rec model`, no unrolling, no sharing. A `rec`
annotation is part of the canonical JSON the oracle compares byte for byte; an
oracle that agrees on the lowering but disagrees on `rank` or `group` fails the
corpus (VER04r 10.3 check 1).

**RC6.** (The meter.) `ir.rec` is the first level whose evaluation is not bounded by
a static node count. It adds the **frame and fuel meter** of RR6 and RR7, charged by
the generator and by every evaluator the same way. C1-S0 of VER04o already
unifies the data meter of its CC7 with the `ir.seq` meter; R1-S0 adds the frame
counter and the fuel to that unified meter, and until `ir.seq` is defined `rec`
models over sequences are `not-live` (RM0c).

### 7.5 Manifest fields

**RC7.** The lane manifest (`warrant-lane-evidence-v2`, VER04r 9.1) gains, per suite
row that uses a recursive type: `fuel` (the generator-predicted fuel of the
subject-independent evaluation, reproduced), `fuel_mode` (`measured`, `proven` or
`dynamic`, RB7), `max_nest_seen` and `max_cells_seen` (the largest values the adapter built or
read, so a report can show the deepest value actually tried), `deep_cases`,
`branchy_cases`, `ctors_admitted` and, for a claim with a `match` on a subject
result, `ok_arm_cases` and `err_arm_cases` (counts, 12.3), and per binder of a `shape` domain the **scope**
(`depth<=3`, `width<=2`, the alphabet sizes) so the report can print the
quantifier. Per lane: `lane_max_nest` (the deepest value the lane's host form can
carry, default `MAX_NEST`, never above it); cases that need more are recorded
`not-live` with their count, never skipped (8.6). Per obligation (10): the
status.

## 8. Lane binding contract

### 8.1 The binding vocabulary

A per-package binding gains the directives below, closed like every binding
(unknown fields are errors, W0501), in the line form of VER04i section 5 and
VER04r 7.1. Every line is data; none is code.

```text
type    Cbor  host=CborValue  ctors=Uint,Nint,Blob,Str,Arr,Dict,Tagged,Flag,Nil
rec     Cbor  form=tagged-enum  box=Tagged.1                 -- the recursive host form, 8.3
rec     Json  form=sealed-hierarchy  free=none               -- C adds free=<export>, 8.4
own     decode result=owned      encode value=borrow
host_depth_limit  1024                                        -- the lane's own cap, <= MAX_NEST
```

* `type ... ctors=` gives the host constructor names in the corpus order, as for any
  variant (VER04r 7.3); positional matching is never used.
* `rec <Type> form=<name>` names a host form from the lane's **closed list** (8.3).
  A name outside it, or a form that cannot carry the type (a lane with no sum
  types binds a tagged struct), is `BindingInvalid` at start-up, before any case.
  The form name selects an adapter arm; no host type text is spliced into source
  (VER04o CT1).
* `own` states ownership for the lanes where it is a fact (C, C++, Rust returning
  references, Swift): `borrow`, `owned`, `take` (8.4). It is required wherever the
  lane table marks a form `ownership`; a missing one is **W0773**.
* `host_depth_limit` lowers the cap for a lane whose host cannot carry
  `MAX_NEST` (8.6); it cannot raise it.

### 8.2 The adapter, normative

**RL1.** (Host to canonical.) For every recursive value that crosses from the subject
to the runner the adapter MUST, **iteratively, with an explicit stack of frames each
holding (host node, expected type, state)**, and never by host recursion:

1. maintain two counters, `cells` and `nest`, and refuse **before** converting, copying
   or allocating more as soon as `cells` exceeds `MAX_VALUE_CELLS` (`value-cells`) or
   `nest` exceeds `min(MAX_NEST, host_depth_limit)` (`value-depth`). The counters
   count **per path**: a host node reached twice is converted twice and counted
   twice, so a DAG that unfolds to `2^40` nodes stops at 65,536 cells after about
   65,536 steps, not after `2^40`;
2. convert each constructor by name through the binding's closed `ctors` list (never
   by a field's host position) and each field by the exact-type rules of VER04r 7.2
   (a value of the wrong host type is `type-mismatch`, `True` is not `1`);
3. read host nodes **without running host code**: no `__getattr__`, no `__index`
   metamethod, no tied or overloaded value, no getter, no proxy trap (the adapter
   uses the raw access of the lane: `rawget` in Lua, `reftype` and no overload in
   Perl, own-property reads of a plain object in TypeScript), and it requires the
   host node to be **exactly** the bound class or shape, never a subclass (a
   Python subclass with a property, a Ruby subclass with an overridden reader).
   A host value whose read would run code is `type-mismatch`. Where a host cannot
   tell (plain JavaScript cannot detect a `Proxy`; the TypeScript lane uses the
   runtime's `util.types.isProxy` and says so), the lane table names the gap and
   the jail remains the defence (RT7);
4. never use the host's `==`, `equals`, `hashCode`, `toString`, `repr`, `deepcopy`,
   serialiser, `compare` or sort on a tree (they recurse on the host stack, loop on a
   cycle, or compare identity), only the work stack above and the `warrant-corpus`
   comparator (VER04o CL2) for any `Set` or `Map` inside;
5. check each list, set and map count against `MAX_COLLECTION` **before** converting
   it (VER04o CL2 step 0);
6. encode the canonical value.

The cycle rule needs no cycle detector: a cyclic host value has no finite `nest`
and no finite `cells`, so one of the two counters reaches its cap and the failure
is `value-depth` or `value-cells` (which one depends on the shape of the cycle and
on the walk order, so a control accepts either). The walk visits at most
`MAX_VALUE_CELLS + 1` host nodes and holds at most `min(MAX_NEST,
host_depth_limit) + 1` frames **whatever the host value is**: that is the step
bound RO2 checks. (A detector would be a hash set keyed by host identity, which
is exactly the kind of structure 13 forbids on attacker-chosen data.) Sharing
that fits the caps is **not** detected and not a failure: a small DAG is read as
the tree it unfolds to (3.7, RQ-7).

**RL1a.** (Letting go of a value the adapter refused.) A subject can hand back a
tree 100,000 levels deep. The adapter refuses it at the cap without a crash, and
then the **host** destroys it: a derived `Drop` in Rust, a `deinit` in Swift, a
destructor in C++, the reference-count cascade in Perl, each recursing once per
level on the host stack. The refusal would then end as a crash after all. So for
a host value that RL1 refused with `value-depth` or `value-cells`, or whose walk
it did not finish, the adapter MUST NOT let the host free it recursively: it
**dismantles** the value with a worklist where the host lets it move children
out (Rust `mem::take` on the child vector, C++ releasing each `unique_ptr`, C by
the rule of RL5), and otherwise **keeps it alive until the process ends** and
ends the run without running destructors. The case failure is recorded first in
either case. A lane that can do neither for a bound form cannot evidence a
depth-bomb claim for it and records the claim `not-live` (8.6). The kit's deep
fixture (RO2) is 10,000 levels of the **subject's** host type for exactly this.

**RL2.** (Canonical to host.) To build a host recursive argument the adapter
decodes the canonical value **iteratively, building bottom-up**: a stack of partially
filled nodes, popped when its field count is reached. It builds a **fresh** host
value for every call and every obligation build (VER04r 7.4): no node is shared
between an argument and an expected value, and no two nodes of one argument share a
child (so a subject that mutates a shared child cannot alias a second place). After
the call a non-`inout` argument is re-read by RL1 and compared with its canonical
form (`arg-mutated`).

**RL3.** (Equality of the check.) The comparison of a result with the expected value
is on the **canonical encoding** (digests, and decoded values), never the host's
equality (VER04o CL4, VER04p 4.3.1).

**RL4.** (Equal rigour.) Every lane implements RL1 to RL3, the obligations of
section 10 and every control of section 11. A lane that cannot records the
operation `not-live` (W0701); it is never counted as covered and never inherits
another lane's result (VER04n OL3).

### 8.3 Per-lane table

"From reading": each hazard is the known behaviour of the language and is
**confirmed by that lane's PR and its hazard-row tests** (the conformance kit of
section 16), not assumed. The closed list names the host forms a binding may select.
Each row also lists what the host does **to** a deep value on its own, because
that is what a lane must not trigger.

| Lane | Host forms (closed list) | Hazards for recursive values |
|---|---|---|
| rust | `enum` with `Vec`, `Box` (`tagged-enum`) | a recursive `Drop` overflows the stack on a deep tree (the adapter builds at most `MAX_NEST`, and drops through a worklist for anything deeper); a stack overflow **aborts** the process and cannot be caught; `derive(PartialEq, Debug, Clone)` recurse, never used by the adapter |
| python | `dataclass`, tagged `tuple` (`tagged-class`) | `RecursionError` at about 1,000 frames for `==`, `repr`, `deepcopy`, `json`, `pickle`; `sys.setrecursionlimit` is **forbidden** as a workaround (it turns a catchable error into a C-stack crash); a list can contain itself; `True == 1`, so a leaf is checked `type(x) is bool` |
| typescript | discriminated union objects (`tagged-object`) | a plain object keyed from data (`__proto__`, `constructor`) mis-parses: use a `Map` or a null-prototype object for any keyed collection; `RangeError` at roughly 10,000 frames; getters and `Proxy` run code on read, and a `Proxy` cannot be detected from plain JavaScript (RL1 step 3); a cycle makes `JSON.stringify` throw, never used |
| go | interface with a sealed method, struct pointers (`sealed-iface`) | a **goroutine stack overflow is fatal** and not recoverable by `recover`; a nil pointer is absent, not `Nil`; slices share backing arrays (`append` aliasing between two nodes); pointer cycles are expressible |
| java | `sealed interface` + `record` (`sealed-hierarchy`) | `StackOverflowError` leaves state unreliable; a record's `equals`, `hashCode`, `toString` recurse; `byte[]` in a record is compared by identity |
| kotlin | `sealed class` + `data class` (`sealed-hierarchy`) | as Java; `data class` `copy` shares children |
| csharp | abstract `record` hierarchy (`sealed-hierarchy`) | `StackOverflowException` **kills the process**; `record` equality recurses and compares `byte[]` by reference |
| fsharp | discriminated union (`tagged-union`) | structural `=` and `compare` recurse; arrays compare length first |
| haskell | algebraic data type (`tagged-union`) | **laziness**: a value may be an infinite or cyclic structure (`let x = Arr [x]`) or hold an undefined thunk; the adapter forces **incrementally under the caps**, never `deepseq` (8.5); `Show`, `Eq` instances recurse |
| ocaml | variant type (`tagged-union`) | `compare` and `=` on a cyclic value loop, and raise on closures; `let rec x = Arr [x]` builds a cycle; `Stack_overflow` is an exception only in some runtimes |
| swift | `indirect enum` (`tagged-enum`) | a deep `indirect enum` crashes in its own `deinit` at tens of thousands of levels; synthesised `==` recurses; value semantics hide sharing |
| dart | `sealed class` hierarchy (`sealed-hierarchy`) | `==` is identity unless overridden; compiled to the web, 64-bit integers are doubles (VER04r 7.2); deep recursion overflows the JS stack |
| ruby | `Struct` or class (`tagged-class`) | `SystemStackError`; `Hash` and `Array` can contain themselves; `==` and `inspect` recurse |
| elixir | tagged tuple `{:arr, [..]}` (`tagged-tuple`) | **no cycles exist** (immutable), but sharing does: `y = {:arr, [x, x]}` doubled forty times is `2^40` paths with forty lines, so RL1 step 1 counts per path; a tag is an atom, not a `Text` |
| lua | table `{tag=..., n=..., [1]=...}` (`tagged-table`) | a table can reference itself; `#t` has no value for holes, so the adapter stores `n` and reads `1..n` with `rawget`; a metatable's `__index` runs code (3 of RL1); no distinction between an array and a map |
| perl | blessed hash or array reference (`tagged-ref`) | a circular reference is expressible; `overload` and tie run code on read (RL1 step 3); a deep structure is **freed recursively** and can crash the interpreter on destruction (the cap protects the adapter's own values); "Deep recursion" is a warning only |
| c | tagged struct with pointers (`tagged-struct`), arena-owned | a pointer cycle; a shared child; a `size_t` count that overflows; ownership (8.4); no exceptions, so a failure is a status code |
| cpp | `std::variant` with `std::vector` of the incomplete type, or `unique_ptr` (`variant-box`) | recursive destructors overflow the stack on a deep tree; `operator==` synthesised recursion; `unique_ptr` forbids sharing (good) |

### 8.4 C structs, pointers and ownership

The C lane (and the C++ lane for `unique_ptr`) has no garbage collector, so who
owns the tree is a **fact the binding states**, not something the adapter guesses.

```text
struct json_value { int kind; union { int64_t num; struct { char *p; size_t n; } str;
                    struct { struct json_value **v; size_t n; } arr; ... } u; };
```

**RL5.** (Ownership.) For a recursive **argument** the adapter builds the tree in an
**arena it owns** and frees the arena after the call; the subject receives it
`const` and **borrows** it (`own ... value=borrow`). A subject that **takes
ownership** of an argument (`take`) is bound with the export that frees it
(`free=<export>`, a name resolved from the module at start-up under the
containment rule of VER04i 5.1), and the adapter does not free it afterwards; a
`take` without a `free` is **W0773**, because the lane could neither free nor
leak-check it. For a **result** the subject's tree is `owned` (the adapter calls
`free=<export>` once, after RL1 has converted it) or `borrowed` (the adapter never
frees). The adapter calls the subject's `free` **only on a result whose walk
finished**: a result refused as `value-depth` or `value-cells` may be cyclic or
deep, the subject's own recursive `free` would loop, double free or overflow on
it, and so it is left allocated and counted (`refused_unfreed`), never freed and
never reported as a `leak` of the subject's. A double free, a free of a borrowed result and a leak of an owned result are
distinct failures (`bad-free`, `leak`); `leak` is checked once per recursive result
the way VER04n OL7 checks it.

**RL6.** (Pointer discipline.) The adapter treats every pointer of a result as
hostile: it checks `kind` against the closed constructor list, a `NULL` child
against the field's optionality, `n` against `MAX_COLLECTION` **before** reading
`v[0..n]`, and counts `cells` and `nest` per path (RL1) so a pointer cycle, or sharing that
unfolds past the caps, is a named failure, not a hang (a small shared child is
read as two equal subtrees, RL1). The worklist and the arena are
allocated by the adapter with checked arithmetic (`n * sizeof` cannot wrap). In C++
a `unique_ptr` tree is built bottom-up (RL2) and destroyed by a **worklist**, not
by the implicit recursive destructor, for anything above 64 levels.

### 8.5 Lazy and cyclic hosts

**RL7.** (Haskell and other lazy hosts.) A value reachable from the subject's result
is **forced node by node** by the adapter under the caps of RL1; `deepseq` and
`force` on the whole value are forbidden, because they do not terminate on an
infinite or cyclic one. A thunk that raises is `unexpected-error`; an infinite value
is `value-depth` (nesting) or `value-cells` (width), as a cyclic one is anywhere.
Forcing one node **runs the subject's code** (the thunk), which the caps do not
bound: a thunk that never returns is stopped only by the suite's wall-clock
limit under the supervisor (8.6), and is then `subject-crash`, never a pass.
A lane with *strict* host forms is unaffected. This is the lane-specific shape of
the rule "values are finite trees" (3.1): the harness defends the model from a host
that can build what the model cannot contain.

### 8.6 Lane limits and the subject's own failure

A lane whose host cannot carry `MAX_NEST` (an interpreter with a small stack, a
runtime that recurses in its collector) declares `host_depth_limit` below it. The
cases whose values need more are recorded **`not-live`** with their count in the
manifest, never skipped silently, and a lane with a non-zero `not_live` count cannot
be `release-ready` for that adopter (VER04r 4.1). A claim about a depth that the
lane cannot carry (the depth-128 claims of the pilot need `nest` about 257) is
therefore `not-live` in that lane and reported so.

A **subject** that overflows its own stack on a deep input (a crash, not a clean
error) is the finding the depth-bomb claims exist to find. If the runner process
dies (a signal, an abort, a fatal runtime error) the run exits non-zero with **no
manifest** (VER04i 4.3: a failed run never writes one), and where the lane runs
the subject under a supervisor (the child-process wrapper of VER04i 12 and VER08b)
it is reported as `subject-crash` with the suite id and case address, never as a
pass and never as exit 2. A lane that cannot observe its own process dying cannot
evidence a depth-bomb claim and records the claim `not-live`.

## 9. Parser-style claims

A parser, a decoder or a codec is a pair of functions between a **tree** and a
**byte or text string**, and its defects fall into a few kinds that a few claim
forms catch. This section states the forms. The examples use the pilot (14) and
`bounded-json`; the forms are not specific to either.

### 9.1 The four forms

| Form | Statement | Binder domain | Independence |
|---|---|---|---|
| **P1 round trip** | `decode(enc(v)) == Ok(v)` and `enc(v) == try_encode(v)` for every `v` in a shape domain, where `enc` is a **`rec model`** (the writer), not the subject | `shape` | **reference**: the expected value `Ok(v)` is the binder itself and `enc` is independent of both subject functions, so the oracle supplies it |
| **P2 rejection** | `decode(b)` is the **expected** `Err` for `b` in a bounded family of malformed inputs | `within` over a byte alphabet, and **mutations** of canonical encodings produced by models | reference for mutations (the model says which error), predicate for `within` |
| **P3 canonical form** | every accepted input is its own canonical encoding: `match decode(b) with Ok(v) -> try_encode(v) == Ok(b) \| Err(_) -> true` | `within` over a byte alphabet | **predicate** (two subject calls): `relational` |
| **P4 limit boundary** | the structure at the subject's depth limit is accepted and one level past it is the closed error | a `Nat` of nesting levels around the limit | reference: the expected value is a model `Ok(nest(n))` or `Err(TOO_DEEP)` |

A round trip written **only** with two subject operations
(`decode(try_encode(v)) == Ok(v)`) is a **predicate** claim: it passes for a pair
that is wrong in the same way on both sides (an encoder that drops a field and a
decoder that restores the default). It is legal, it is printed `relational`, and a
document with both a `decode` and an `encode` operation SHOULD carry at least one
**reference** claim per operation (the advisory `no-independent-reference`, an
open exemption, no code allocated).

### 9.2 The independent writer

The model `enc` of P1 is the document's own statement of the format. For CBOR it is
`rec model enc(v: Cbor) -> Bytes decreasing v` (14.3), computed by structural
recursion, with the header encoder a non-recursive model. Its value is that a
**decoder** bug and an **encoder** bug cannot cancel: `decode(enc(v))` fails if the
decoder is wrong against the document, and `try_encode(v) == Ok(enc(v))` fails if
the encoder is.

### 9.3 Rejection of malformed input

Byte domains are **bounded**, so rejection claims are `bounded-set` and say so:
`Bytes` values have at most 4,096 bytes (VER04r 2.3), and `within(b, A, k)` has
`sum of |A|^i` members (VER04l 4.3). Two ways to build a malformed family, both
language neutral:

* **mutation of a canonical encoding**, by models: `concat(enc(v), b"00")`
  (trailing byte: `TRAILING_BYTES`), `take(enc(v), k)` for every `k < len(enc(v))`
  (every proper prefix is rejected: the format is **prefix-free**, a property
  worth a claim by itself), `non_minimal(v)` (the head of a number rewritten in a
  longer form: `NON_MINIMAL_INTEGER`), `swap_entries(v)` (a map's two entries
  exchanged: `NON_CANONICAL_MAP_ORDER`);
* **a token alphabet**: `within(b, [00, 17, 18, 19, 1a, 1b, 1f, 40, 60, 80, a0, c0,
  f4, f5, f6, f7, fb, ff], 4)`: the bytes that start every CBOR item kind, reserved
  value or boundary, all strings of length at most 4 over them.

A claim over a mutation family names the **expected error** (a `reference` claim
with a model that says which); a claim over a token alphabet is the predicate P3
(everything accepted is canonical) plus the totality claim "the result is `Ok` or a
closed `Err`, never a panic" (an unexpected panic is already `unexpected-error`,
VER04r 7.5).

### 9.4 Canonical form

P3 is the **non-malleability** claim: if two byte strings decode to the same tree
(or one does), the format has one canonical encoding. It is stated as "every
accepted input re-encodes to itself", which catches an over-liberal decoder (it
accepts `18 05` for `5`, an indefinite length, a duplicate map key, trailing data)
and an encoder that does not sort. It is the claim that matters for authentication
and for hashing, and it is the reason the pilot is a canonical codec.

### 9.5 Depth bombs

P4 builds the structure **by a model**, never by a literal: `nest_tag(n: Nat) -> Bytes
= fold(range(n), b"00", prepend_c0)` makes `n` tags in front of the byte `00`;
`nest_array(n)` makes `n` arrays. The claim quantifies `n` over a `nat_count`
domain around the limit:

```text
claim decode_depth_limit:
  forall n: Nat | n <= 140.
    decode(nest_tag(n)) == (if n <= 128 then Ok(tag_chain(n)) else Err(CborError.TOO_DEEP))
```

with `tag_chain(n)` built by `fold(range(n), Uint(0u64), wrap_tag)` (the byte `00`
is `Uint(0)`, so the two sides describe the same item). The subject counts
**wrappers** (CBR01: the root is depth zero, 128 nested tags are accepted), while
`depth_of` counts nodes, so `depth_of(tag_chain(n)) = n + 1`: a claim about a
subject's limit is written over the wrapper count `n`, as here, never by comparing
`depth_of` with the subject's number. The encoder side is the same claim on
`try_encode(tag_chain(n))`, with `Ok(nest_tag(n))` up to 128 and
`Err(CborError.ENCODE_TOO_DEEP)` past it; there the lane builds a host value of
`nest` up to 142, far inside `MAX_NEST`. The `nat_count`
descriptor (VER04r 5.1) has 141 values; the boundary `n = 127, 128, 129` is in every
slim run (it is one chunk). Because the bytes are at most 141 in length and the
expected value has `nest` at most 142, **neither side needs a materialised deep value**, which is the
rule of 6.2.

## 10. Evidence obligations

Three obligations are generated for every document that uses the feature. Each
has a failure id, a control (section 11) and a printed status; one that does not
apply prints `n/a (reason)`, never a pass.

**RO1.** (`adapter-roundtrip`.) Without calling the subject, the adapter builds a
host value from **every `least`/`witness` value of every recursive type the
document uses**, plus the values of its `shape` at depth 2 (or the first 2,048,
whichever is fewer), reads it back by RL1 and requires the bytes to equal the
canonical value's (VER04l 7.3's round trip, for trees). It catches a host form that
loses or merges information: a constructor that cannot carry its fields, a text
that is normalised, a `Dict` that a host map reorders, an integer that rounds. It
also builds the **hostile-key witnesses** (`__proto__`, `constructor`, `toString`,
`length`, `0`, the empty string, a NUL) as the keys of every `Map<Text, _>` or list
of `(Text, _)` pairs the type holds, because that is where a host that keys a plain
object by data breaks. Failure id `adapter-roundtrip`. Control `lossy-host` (11.2).

**RO2.** (`bounded-walk`.) The adapter is run on the **fixture hosts** of the
conformance kit (16): a deep host value (`nest` 10,000), a wide one (a million
empty children), a **shared** one (forty diamonds, `2^40` paths) and a **cyclic**
one. The adapter MUST finish each within the step bound of RL1 (at most
`MAX_VALUE_CELLS + 1` host nodes visited, counted by the fixture) and report
`value-depth` or `value-cells`, never crash and never hang, and MUST then release
the refused value without a crash (RL1a). Control: the four broken
adapters of 11.2 (`recursive-adapter`, `no-depth-cap`, `path-blind`, `cycle-blind`).

**RO3.** (`fresh-tree`.) For every recursive argument and every obligation build
(RL2), the adapter checks that no node is shared with any other argument or
expected value, and that after the call a non-`inout` argument equals its pre-call
canonical form (`arg-mutated`). Failure id `arg-aliased` for sharing, `arg-mutated`
for change. Control `share-child` (11.2).

### 10.1 What the obligations do not establish

| Establishes | Does not establish |
|---|---|
| for every tree of the stated shape, the subject agrees with the independent model, in this lane | the subject on a tree outside the shape (it is `bounded-set`); the subject at depth beyond the shape |
| the adapter converts the stated witnesses and fixture hosts without loss, crash or hang | that **every** host value is converted faithfully (only the witnesses and the fixtures) |
| the harness rejects a host value deeper or larger than its caps | that the **subject** survives its own depth bomb in a lane that cannot observe its death (8.6) |
| the stated depth-limit boundary holds at 127, 128, 129 | the limit at a different `d` (a claim per limit) |
| nothing about the subject's hash-flooding resistance (13, RT5) | CWE-407 |

## 11. Sabotage controls (faults)

A control passes only if a named claim or obligation **fails** on a deliberately
wrong subject, and an unrelated failure never credits one (VER04r 8.3). Two kinds,
because a defect lives in the subject (a corpus fault, expressed as IR) or in the
adapter (not an IR rewrite). Every control that applies MUST be in the `faults=`
row (else W0502); one that does not apply prints its condition and is never counted
as caught (W0701).

### 11.1 Corpus faults (data, VER04r 8.1)

The IR is built from the fault-only tags of RC5. A rewrite of a composite result
targets **the first node of the recursive group found in the result by a
left-to-right pre-order walk** (inside `Ok`, `Some`, a list element, a tuple
component); if there is none the control is `n/a (no recursive value in the
result)`. "Child" means a group-typed value in a field, a list element or a map
value of that node.

| # | Fault | Rewrite | Must be caught by | `n/a` when |
|---|---|---|---|---|
| RS1 | `drop-child` | `post`: `rec_drop_child(r)`, the **last** child of the first node that has one is removed | a reference claim whose expected value holds it (P1) | `branchy_cases = 0` for the claim (no case with a node that has a child) |
| RS2 | `dup-child` | `post`: `rec_dup_child(r)`, the first child is repeated after itself | a reference claim | the same |
| RS3 | `swap-children` | `post`: `rec_swap_children(r)`, the first two **different** children of one node exchanged | a reference claim over a case with two different siblings | `swappable_cases = 0` |
| RS4 | `truncate` | `post`: `rec_truncate(r, k)`, every node at `depth_of > k` replaced by `least<T>()` (a **silent depth truncation**: a parser that stops at its limit and returns what it has) | a claim whose domain has trees deeper than `k`, and **P4**: the boundary cases | no case has depth `> k` |
| RS5 | `ctor-confusion` | `post`: `rec_ctor_next(r)`, the first node's constructor replaced by the **next** constructor with the same field types, else the first node's first leaf by the next leaf value | a reference claim | no constructor has a same-typed neighbour |
| RS6 | `limit-low` | `post`: `rec_limit_low(r, L, E)`: a result `Ok(x)` with `depth_of(x) > L` becomes `Err(E)`, where `L` is the subject's limit counted in wrappers and `E` its error (a subject whose limit is one too low: the value **at** the limit, which has `depth_of = L + 1`, is refused) | **P4** at `n = limit` | the op has no depth-limit claim |
| RS15 | `accept-malformed` | `replace`: `rec_accept(r)`: a result `Err(_)` becomes `Ok(least<T>())` (the recursive form of the generic `missing-error` of VER04r 8.2, whose `Ok(first Bytes arg)` is not a tree). It is a decoder that accepts what it must reject, and with RS6 it is the other half of an off-by-one limit (one too high) | **P2** (every case expects an `Err`), **P3** (the accepted `least<T>()` does not re-encode to the input), **P4** at `n = limit + 1` | the op's result is not a `Result` with a recursive `Ok` |

A control is **observable** only if some case differs; the generator counts them
(`branchy_cases`, `swappable_cases`) and the witness-chunk rule of VER04r 5.6 puts
the first such case into the slim set.

### 11.2 Adapter controls (conformance fixture, VER04r 11)

Each is a deliberately broken adapter in the `v2-recursive` category of the shared
kit. A lane runner passes the kit before it may write a manifest (VER04r D7), so
each control runs in **every** lane. A broken adapter that would crash or hang is
run in a **supervised child with a timeout and an output cap**, and the expected
outcome is that it is **caught**, never that it survives.

| # | Fault | The broken part | Must be caught by |
|---|---|---|---|
| RS7 | `recursive-adapter` | RL1 implemented by host recursion | RO2: the 10,000-deep host value (stack exhaustion is observed as `subject-crash` of the fixture, or as an error, and counts as caught) |
| RS8 | `no-depth-cap` | RL1 without the `nest` and `cells` caps | RO2: the wide and deep hosts exceed memory or time, caught by the supervisor's limits |
| RS9 | `path-blind` | the counters count **distinct host nodes**, not paths | RO2: the forty-diamond host converts in forty steps and then produces a value over the cap or a huge stream |
| RS10 | `cycle-blind` | the adapter keeps a visited set of host nodes to "detect" cycles and so also hides sharing | RO2 (the shared host) and RO1 (a value whose two children are equal but separate) |
| RS11 | `lossy-host` | the host form drops or normalises a field (a text normalised, a map reordered, an integer rounded, `__proto__` swallowed) | RO1 `adapter-roundtrip` |
| RS12 | `share-child` | RL2 gives two arguments (or two fields) one child | RO3 `arg-aliased`; and a subject fixture that mutates one |
| RS13 | `silent-truncate` | the adapter cuts at `host_depth_limit` and returns what it has instead of `value-depth` | RO2 and the P4 claims |
| RS14 | `order-blind` | the adapter sorts the entries of a `List<(K, V)>` that the model holds in order (an object whose members' order is semantic) | RO1 and any reference claim over a case with two keys out of order |

### 11.3 Every claim form can fail

| Claim form | A control that makes it fail |
|---|---|
| P1 round trip (reference) | RS1, RS2, RS3, RS5 on the encoder; RS11 on the adapter |
| P1 `enc(v) == try_encode(v)` | RS1 to RS5 |
| P2 rejection by mutation, and by `is_err` | RS15 `accept-malformed` |
| P3 canonical form | RS15 on the decoder (an input it must reject comes back as `Ok(least<T>())`, whose encoding is not that input; the case exists whenever `err_arm_cases > 0`, RV11); RS1, RS2 or RS5 on the encoder (an accepted input no longer re-encodes to itself; needs `ok_arm_cases > 0`) |
| P4 limit boundary | RS6 at `n = limit`, RS15 at `n = limit + 1`, RS4 for a silent truncation |
| a `rec model` fold claim (R3) | RS1 to RS5 on the claim's subject; a model fault `rec_wrap` (the root wrapped in a constructor) on the expected side, refused outside a fault |
| the depth and size measures (`depth_of`, `nodes_of`) | `off-by-one` on the `Nat` result |

## 12. Honest labels and vacuity guards

### 12.1 Labels

The label rule is RD11. Three additions:

**RV1.** A suite whose binder is a recursive `shape` prints the **scope** on its
report line, always: `trees(depth<=3, width<=2; Bool 2, I64 2, Text 2)` or
`trees(nodes<=5, width<=2; Bool 2)`, with the size and the admitted count. A reader
cannot mistake a bounded domain for the type.

**RV2.** `exhaustive` is never printed for a claim with a recursive binder, in any
file a human or the report reads. The VER29 rung is at most `exhaustive-sampled`
(RD9, RD11).

**RV3.** A **demoted** default domain (RD3) prints `demoted from depth<=3 to
depth<=2` and is labelled `sample` (RD3, VER04r 9.3), never `bounded-set`.

### 12.2 The report block (sketch)

```text
claim decode_round_trip      reference  bounded-set  exhaustive-sampled
   trees(depth<=2, width<=2; U64 10, Bytes 2, Text 3, Bool 2)  812,160 ordinals  admitted 393,660  discarded 418,500
   deep_cases (depth>=2) 393,630   branchy_cases 393,300   max_nest_seen 5   max_cells_seen 12
   ctors_admitted 9 of 9   fuel: measured   obligations: adapter-roundtrip pass  bounded-walk pass  fresh-tree pass
   lanes ran 2 of 17 (rust, python); absent 15   ops adopted 3 of 8 public entries
   flood-resistance: not evidenced
```

(The ordinal and admitted counts are those of 14.4. `deep_cases` is the admitted
total less the 30 values of depth 1; `branchy_cases` counts `Arr` of two elements
(900), `Dict` of one entry (900, whose pair has two children) and `Dict` of two
entries (391,500); `max_nest_seen` is a `Dict` with an `Arr([])` key (`Dict`, `List`,
pair, `Arr`, `List`: 5); `max_cells_seen` is a `Dict` of two entries, each of a pair
and two two-cell nodes: `2 + 2 * 5 = 12`. All by hand.)

### 12.3 Vacuity guards

| # | Failure | Prevention |
|---|---|---|
| RV4 | **a claim over leaves only** (a recursive type whose every case is `Null`) | the report prints `deep_cases` (cases with `depth_of >= 2`) and `branchy_cases` (cases with a node of two or more children); zero is **W0601** for the claim |
| RV5 | **a recursive fold that visits nothing** | `nodes_visited` per claim, committed in the corpus and reproduced by every lane; zero is W0601 (the guard of VER04o CQ3) |
| RV6 | **a control that cannot bite** | `branchy_cases` and `swappable_cases` are counted and printed; a control with a zero count prints `n/a`, never `caught` (VER04o CB4) |
| RV7 | **a depth claim that never reaches the limit** | the P4 claim's domain MUST contain `limit - 1`, `limit` and `limit + 1`; the checker computes it from the literal (W0601 otherwise) |
| RV8 | **a lane that cannot carry the depth, counted as covered** | `lane_max_nest` and the `not-live` count (8.6, VER04o CB6) |
| RV9 | **a relational claim presented as independent** | the `relational` tag (VER04r 4.3, 9.3) and the advisory of 9.1 |
| RV10 | **a bounded domain read as the type** | the scope line (RV1) and the absence of `full` (RV2) |
| RV11 | **a canonical-form claim that accepts nothing** (P3 is `Err(_) -> true`, so a domain of rejected inputs only passes for any decoder), or one that rejects nothing | `ok_arm_cases` and `err_arm_cases` are counted by the generator from the model where one exists, and otherwise by the lane, and printed; `ok_arm_cases = 0` is **W0601** when predicted and a lane failure `vacuous-arm` when observed. A P3 claim SHOULD have both above zero, and a zero `err_arm_cases` makes RS15 `n/a` for it |
| RV12 | **a constructor never admitted** | `ctors_admitted n of m` per recursive binder (RD12b); below `m` is printed, and zero admitted cases for the whole suite is already a corpus error (VER04r 14) |
| RV13 | **a partial adoption read as the package** | the report prints `ops adopted n of m public entries` for the adopter (14.1), and the package is not counted as covered while `n < m` |

## 13. Security considerations

**RT1.** **Stack exhaustion (CWE-674) in host adapters and evaluators.** The attacks
are an input that nests deeply (a stream, a `corpus.json`, a host value returned by
the subject) and a host operation that recurses on it (equality, a printer, a
copier, a destructor, a garbage-collector mark phase). The defence is structural:
**no consumer recurses on the host stack in proportion to the data** (RM5a, RC2,
RL1), every cap is checked **before** allocation (6.1), every host value is read by
an explicit worklist, and `MAX_NEST` is small enough that the **adapter's own
fresh host values** can be dropped and collected by every lane on its smallest
stack (the per-lane table of 8.3 names the destructors that recurse; the adapter
drops anything deeper than 64 levels through a worklist where the lane allows, and
otherwise `host_depth_limit` is lowered, 8.6). A lane that cannot guarantee this
for `MAX_NEST` is `not-live` for the deeper cases, never silently unsafe.

**RT2.** **Hostile corpora.** A corpus is data a PR author controls (VER04r 14).
Beyond VER04o CT1 to CT3: a **type table with a cycle** is checked for
inhabitation and group size by iteration with a round bound (RC3); a **descriptor
whose declared `size` is wrong or whose table saturates** is a corpus error before
`nth` is called (RD5); a **stream of nested counts** (`65 .. 02000000 65 .. 02000000
...`) is bounded by the cumulative **cell** cap, not by per-level counts (RC2); a
`rec` annotation that **lies** (a model marked `rec` that is not structural) cannot
loop the evaluator, because frames and fuel are enforced by every consumer
independently of the annotation (RR8); a **materialised value** deeper than the
reader's limit is a corpus error at generation (6.2); an IR that uses `rec_*` fault
tags outside a fault is refused (RC5). The oracle recomputes the counting tables,
the unranking, the groups, the `rec` annotations and the fuel with code that shares
nothing with the generator (VER04r 10).

**RT3.** **Shared structure and cycles in host results.** A subject can return a
value with a shared subtree repeated 40 times (`2^40` paths, forty allocations) or a
pointer cycle. The adapter counts **per path**, so it stops at `MAX_VALUE_CELLS`
after about 65,536 steps, and a cycle is `value-depth` with no detector needed
(RL1). Detecting sharing by a visited set keyed by host identity is **forbidden**: it
is the structure a hostile value attacks, it hides legitimate sharing, and it is a
hash table over attacker-chosen identities (the control `cycle-blind`, RS10).

**RT4.** **Hash flooding (CWE-407), per VER04o CT4.** The model, the generator, the
oracle and every evaluator use **sorted arrays or balanced trees ordered by `ord`**
for every `Set` or `Map` inside a recursive value, never a hash table keyed by
corpus-chosen or subject-chosen data; the comparator is `warrant-corpus` code
(VER04o CT5), which is extended with the `ord` of RM6 and tested by the witness rows
of `least`. The host collections the adapter builds to feed the subject are bounded
by `MAX_COLLECTION` and by `MAX_VALUE_CELLS` and are counted by `entries_built`
(VER04o CB10). A `Map<Text, Json>` in a **subject** (a JSON object parsed into a
hash table) is **not** evidenced against flooding by any claim here:
`flood-resistance: not evidenced`, exactly as VER04o prints it; a `bound steps`
row and a hostile-key table (VER04j 6.3) are the document's to supply.

**RT5.** **Duplicate keys and key order are semantic.** `bounded-json` retains
duplicate object members and their order (`Object(Vec<(String, JsonValue)>)`);
`canonical-cbor` sorts map keys length-first and rejects duplicates. Whether an
object is a `Map` (extensional, last-wins or refused, sorted) or a `List` of pairs
(ordered, duplicates kept) is a **property of the subject** and the document types
it accordingly: `Obj(List<(Text, Json)>)` for `bounded-json`, `Dict(List<(Cbor,
Cbor)>)` for `canonical-cbor`, and `Obj(Map<Text, Json>)` only for a subject that
deduplicates. An adapter that re-sorts a list of pairs is the control `order-blind`.

**RT6.** **No secrets in recursive values** (W0401, the rule VER04o CK9 and VER04p
apply): a `labels ... secret` row on a recursive value, or on a field of one, is
refused, because comparison, traversal and encoding are variable-time and the
corpus is committed.

**RT7.** **The capability jail stays mandatory** (owner decision d, VER08b, VER04i
12). Nothing in this feature needs a host facility beyond the lane's standard
runtime (a thread for a large stack is the lane's own, not the binding's); the same
corpus, binding and wrapper run under the jail with identical digests, and no
manifest is written otherwise. The adapter's no-host-code read (RL1 step 3) is also a
jail property: reading a subject's value must not run the subject's code.

**RT8.** **No reproducing material in public output.** A failure prints the suite id,
the case address and the failure id (`value-depth`, `value-cells`, `rec-frames`,
`ir-fuel`, `adapter-roundtrip`, `arg-aliased`, `shape-size`, `type-cycle`,
`vacuous-arm`, `subject-crash`), never a key, a text or a byte of the value (VER00 7.3, VER04i
6.1): the tree may be a hostile document.

**RT9.** **Layered verification.** A lower layer's recursive-typed claim is a
**value claim** and is assumed above, never re-proved (decision i). A type shared by
two packages (the upper `json-value` uses the lower `bounded-json`'s `JsonValue`)
is **redeclared** in the upper document and the two declarations must have the same
**canonical type digest**: SHA-256 of the group's rows as written in `types`
(constructor order included, since it is semantic), compared by the generator when
both documents are in the repository, and a mismatch is a corpus error
`ContractTypeMismatch`, the type analogue of the VER04n OF6 rule. A published claim
about a recursive type resolves above the layer **with its scope** (`bounded-set`,
`trees(depth<=3, ...)`), never as an unbounded fact. Fail closed.

## 14. The pilot: `canonical-cbor`

### 14.1 Why this crate

The owner prefers pilots whose dependencies are already verified or verifiable
first, and the pilot is one crate from the inventory list, chosen by security
weight and dependency depth. The candidates, from the inventory CSV (2026-11-16) and
from reading the sources:

| Crate | Lanes (Rust plus ports) | Direct path deps | Dependents | LOC | Needs besides recursion | Verdict |
|---|---|---|---|---|---|---|
| **`canonical-cbor`** | **17** (16 ports) | **0** | 5 (12 transitive) | 952 | `Text` (T1); the pilot adopts `encode`, `try_encode`, `decode` (WX-G and generics only for `encode_into` and the `impl Into` constructors, left out) | **chosen** |
| `bounded-json` | 3 (c, cpp) | 0 | 10 (161) | 612 | floats (`JsonNumber::Float`), `Text`; float to text (FL2) for any claim about `Float` | second adopter, after FL2; few ports for a polyglot pilot |
| `json-serializer` | 12 | 1 in Rust; **8 in the Python port, `json-value` among them** | 21 | 633 | floats, `Text` | after `bounded-json` and `json-value` are verified |
| `json-value` | 12 | 4 (`bounded-json`, `json-parser`, `parser`, `lexer`) | 38 | 596 | floats, `Text` | deep dependency chain |
| `wasm-types` | 15 | 1 | 14 | 2,327 | WX-D2, stateful objects | large; stateful objects first |
| `json-parser`, `parser`, `grammar-tools` | 16, 15, 15 | 4, 2, 0 | 7, 137, 115 | 245, 2,344, 4,625 | stateful objects, collections, generics (collections and text only, for `grammar-tools`) | deeper chains, or collections first; later |
| `symbolic-ir` | 5 | 0 | 43 | 331 | floats, generics | weight 1, no hostile input; the expression-tree illustration of this document |

Reasons for `canonical-cbor`, in order of weight:

1. **Security weight and the failure that matters.** It is a codec under the whole
   Vault stack (records, authentication material, audit entries, FIDO2 and COSE
   framing); its security property **is** the canonical form: one value, one byte
   string, so a hash or a signature binds one meaning (CBR01, "Why this primitive
   exists"). The inventory weights it 2; reading it, it is the clearest case where
   a *parser-style claim* (P1 to P4) is the specification.
2. **Dependencies: none.** Its Cargo `[dependencies]` is empty, its required
   capabilities are `[]`, and CBR01 says every implementation is zero-dependency by
   design. There is nothing underneath that has to be verified first, which is the
   owner's preference, and nothing a failure could be blamed on.
3. **Seventeen lanes.** A pilot is also a test of the adapter design in section 8:
   with Rust plus sixteen ports (c, cpp, csharp, dart, elixir, fsharp, go, haskell,
   java, kotlin, lua, perl, python, ruby, swift, typescript) it exercises tagged
   unions, classes, tables, C structs with ownership and lazy hosts, which
   `bounded-json` (three lanes) cannot.
4. **No float and no float-to-text.** CBR01 forbids floats (`FloatNotSupported`), so
   the pilot does not wait for FL2 and does not touch the VER04p relation beyond
   what recursion lifts.
5. **An independent oracle already exists.** CBR01 names
   `code/specs/fixtures/canonical-cbor-v1/cases.json` as the **normative executable
   corpus** (header boundaries, ordering, rejection ids, depth 128 and 129), and the
   C and C++ implementations are independent reference oracles. The pilot imports
   those rows as WX-E tables (VER04j 6) next to the generated domains, so the
   evidence has two independent origins.
6. **Its limits are the ones this document must handle.** `max_nesting_depth = 128`
   and `max_encoded_bytes = 1,048,576` (CBR01) are exactly the depth bomb and the
   size bomb of section 6.

**What is adopted, and what is not (partial adoption, stated).** The inventory tags
the crate `WX-G|generics|recursive-types|text` because tags are computed over
**every** public entry, and it counts 8 public entries. The pilot adopts **3 of
the 8**. The report says `ops adopted 3 of 8 public entries` (RV13), the package
is **not** counted as covered, and nothing below is evidenced for the other five:

| Public entry (Rust) | Status | Why |
|---|---|---|
| `decode(&[u8]) -> Result<CborValue, CborError>` | **adopted** | P1, P2, P3, P4 |
| `try_encode(&CborValue) -> Result<Vec<u8>, CborError>` | **adopted** | P1 on the encoder, P3, P4 |
| `encode(&CborValue) -> Vec<u8>` | **adopted, on its precondition only** | it is `try_encode` that **panics** on a duplicate key, on depth 129 and on an oversized result; the claim is over values that are encodable, and the panic outside them is **not evidenced** |
| `encode_into`, `try_encode_into` (`&mut Vec<u8>`) | not adopted | an `inout` buffer: WX-G, which 3.7 keeps out of the first slice for a recursive value. The "destination unchanged on failure" property is unevidenced |
| `CborValue::text`, `CborValue::bytes` (`impl Into<...>`) | not adopted | generics (VER04o G1). Constructors only; the adapter builds values without them |
| `decode_map_spanned` | not adopted | returns byte spans into the input, a different result type; its agreement with `decode` on key order and depth is unevidenced |

Two further limits, both printed. **The ports do not share one surface** (from a
grep of the sources, not from running them): Go exports `Encode`, `EncodeChecked`
and `Decode`; Python exports `encode_checked` and `decode` and has **no**
panicking `encode`. An op is bound per lane by its Warrant name; an op with no
counterpart in a port is `not-live` in that lane and counted so, and R1-S5
produces the lane-by-op matrix before stage B (so `encode` is expected to be
`not-live` in the Python lane of stage A). **`max_encoded_bytes`** (1 MiB) is
above the harness's 4,096-byte value cap, so the `encode-too-large` boundary is
reached only by an **encode-only builder** whose result the lane does not return
as a value (the CBR01 fixture's shape); until that has a Warrant form it is
`not-live` and printed so. `bounded-json` follows as the second adopter (the
first with a float).

### 14.2 The document (sketch)

Illustrative; names are not checked. Constructors avoid the reserved `Map` and
`Bytes` (RM0a); `Dict` is a **list of pairs** because CBR01 keeps the on-wire order
and a key is itself a tree (a non-key type), so it is not VER04o's `Map`.

```text
warrant canonical_cbor v2

error CborError = UNEXPECTED_EOF | TRAILING_BYTES | RESERVED | INDEFINITE
                | NON_MINIMAL_INTEGER | INVALID_UTF8 | NON_CANONICAL_MAP_ORDER
                | UNSUPPORTED_SIMPLE | FLOAT_NOT_SUPPORTED | TOO_DEEP
                | LENGTH_TOO_LARGE | DUPLICATE_MAP_KEY | ENCODE_TOO_DEEP | ENCODE_TOO_LARGE

variant Cbor = Uint(U64) | Nint(U64) | Blob(Bytes) | Str(Text)
             | Arr(List<Cbor>) | Dict(List<(Cbor, Cbor)>) | Tagged(U64, Cbor)
             | Flag(Bool) | Nil

op decode(b: Bytes)     -> Result<Cbor, CborError>
op try_encode(v: Cbor)  -> Result<Bytes, CborError>
op encode(v: Cbor)      -> Bytes                      -- partial: defined where try_encode is Ok (a precondition, sketch)
```

### 14.3 Models and claims (sketch)

```text
model head(major: U8, n: U64) -> Bytes = ...                    -- the smallest-form header: inline, 1, 2, 4 or 8 bytes
rec model pair_step(acc: Bytes, e: (Cbor, Cbor)) -> Bytes decreasing e =
  concat(concat(acc, enc(e.0)), enc(e.1))                       -- strict: e.0 and e.1 are disjoint derived terms
rec model arr_step(acc: Bytes, x: Cbor) -> Bytes decreasing x = concat(acc, enc(x))   -- level, enc is later
rec model enc(v: Cbor) -> Bytes decreasing v =
  match v with
    Uint(n)   -> head(0u8, n)
  | Nint(n)   -> head(1u8, n)
  | Blob(b)   -> concat(head(2u8, len64(b)), b)
  | Str(t)    -> concat(head(3u8, len64(utf8(t))), utf8(t))
  | Arr(xs)   -> concat(head(4u8, len64(xs)), fold(xs, b"", arr_step))
  | Dict(es)  -> concat(head(5u8, len64(es)), fold(es, b"", pair_step))
  | Tagged(g, x) -> concat(head(6u8, g), enc(x))                 -- strict: x
  | Flag(b)   -> if b then b"f5" else b"f4"
  | Nil       -> b"f6"

rec model canonical(v: Cbor) -> Bool decreasing v = ...         -- every Dict's keys strictly ascending, length-first
model key_lt(a: Bytes, b: Bytes) -> Bool = ...                  -- length first, then unsigned bytewise

claim decode_round_trip:                                        -- P1, a reference claim
  forall v: Cbor | shape(v, depth <= 2, width <= 2,
                         U64 in [0u64, 23u64, 24u64, 255u64, 256u64, 65535u64, 65536u64,
                                 4294967295u64, 4294967296u64, 18446744073709551615u64],
                         Bytes in [b"", b"00"], Text in [t"", t"a", t"\u{E9}"])
         and canonical(v).
    decode(enc(v)) == Ok(v)

claim encode_is_model:                                          -- P1 on the encoder
  forall v: Cbor | shape(v, depth <= 2, ...) and canonical(v).
    try_encode(v) == Ok(enc(v))

claim encode_total_form:                                        -- the panicking wrapper, on its precondition
  forall v: Cbor | shape(v, depth <= 2, ...) and canonical(v).
    encode(v) == enc(v)

claim canonical_only:                                           -- P3, a predicate claim, relational
  forall b: Bytes | within(b, b"00171819 1a1b1f40 6080a0c0 f4f5f6f7 fbff", 4).   -- 18 bytes; spaces for reading
    match decode(b) with Ok(v) -> try_encode(v) == Ok(b) | Err(_) -> true

claim every_prefix_rejected:                                    -- P2, the format is prefix-free
  forall v: Cbor | shape(v, depth <= 1, width <= 2, <the same alphabets>),
         k: Nat | k <= 64 and k < len(enc(v)).
    is_err(decode(take(enc(v), k)))

(The `canonical` admission is itself a `rec model`; `utf8`, `len64` and `head` are
ordinary models; `fold(es, b"", pair_step)` passes a **list of pairs**, whose elements
`e` are derived terms, and `pair_step` calls `enc` on `e.0` and `e.1`, which are
disjoint: RR4 holds. Hex literals are written as the VER04j `b"..."` literal; the prefix claim uses depth 1
because a dependent binder `k < len(enc(v))` is a refinement charged the whole product,
VER04o CQ-2.)

### 14.4 Counts, by hand

`shape(v, depth <= 2, width <= 2, ...)` with the alphabets above (`Uint` 10, `Nint` 10,
`Blob` 2, `Str` 3, `Flag` 2, `Nil` 1: **28** leaf values):

| `k` | `V_k` | computation |
|---|---|---|
| 1 | 30 | 28 leaf values, plus `Arr([])`, plus `Dict([])` (`Tagged` needs a child: 0) |
| 2 | **812,160** | `28 + (1 + 30 + 900) + (1 + 900 + 810,000) + 10 * 30 = 28 + 931 + 810,901 + 300` |

(`Arr` over lists of at most two elements of the 30 values: `1 + 30 + 30^2 = 931`;
`Dict` over lists of at most two **pairs** of values, a pair being one of `30 * 30 =
900`: `1 + 900 + 900^2 = 810,901`; `Tagged` is the 10 tag alphabet members times the
30 children: 300.) The product ordinals are 812,160, so the suite has
`ceil(812,160 / 65,536) = 13` chunks (`12 * 65,536 = 786,432`, remainder 25,728); 13
is at most 16, so **`slim` is every chunk** and the label is `bounded-set` at both
depths.

Admission `canonical(v)`: every value of depth at most 2 is canonical except a
`Dict` of two entries whose keys are not strictly ascending. Of the 810,000 two-entry
lists, a pair of keys `(k1, k2)` from the 30 values with `k1 < k2` is `C(30, 2) = 435`
choices (the 30 values have 30 distinct encodings), each with `30 * 30 = 900` value
pairs: **391,500** admitted two-entry dicts; the discarded are the `30 * 900 =
27,000` with `k1 = k2` and the `391,500` with `k1 > k2`, 418,500 in all (`27,000 +
391,500 + 391,500 = 810,000`). So the admitted `Dict` count is `1 + 900 + 391,500 =
392,401`, the **admitted total is `28 + 931 + 392,401 + 300 = 393,660`**, and the
discarded total is `812,160 - 393,660 = 418,500`. The three claims
`decode_round_trip`, `encode_is_model` and `encode_total_form` make 393,660 subject
calls each, one call per admitted case.
`canonical_only` has, for an alphabet of 18 bytes and `k = 4`, `1 + 18 + 324 +
5,832 + 104,976 = 111,151` cases, a predicate claim with two subject calls on the
accepted ones. `every_prefix_rejected` has `30 * 65 = 1,950` product ordinals (30
values of depth at most 1, `k` over 0 to 64) and **94** admitted cases, because the
admitted `(v, k)` pairs are the proper prefixes and the encodings of the 30 values
have lengths `1 1 2 2 3 3 5 5 9 9` (`Uint`, 40 in all), the same for `Nint` (40),
`1 2` for `Blob` and `1 2 3` for `Str` (3 and 6), `1 1` for `Flag`, and 1 each for
`Nil`, `Arr([])` and `Dict([])`: `40 + 40 + 3 + 6 + 2 + 3 = 94`. In `canonical_only`
both arms are taken (RV11): the single bytes `00 17 40 60 80 a0 f4 f5 f6` are
accepted, and `18 00`, `1f`, `f7`, `fb` and `ff` are among the rejected; the exact
`ok_arm_cases` is the generator's to count, not this text's.

The **discarded** 418,500 are not wasted: a second pair of claims uses
`not canonical(v)` as its admission and says `decode(swap_or_dup_enc(v)) ==
Err(NON_CANONICAL_MAP_ORDER)`, so the rejection of out-of-order and duplicate keys
has the same domain. `try_encode` on those is `Err(DUPLICATE_MAP_KEY)` for the
27,000 with equal keys and, for the 391,500 out of order, `Ok(enc(swap_entries(v)))`
(CBR01: the encoder sorts; with two entries out of order, sorting is the swap),
which is a **reference** claim, since `swap_entries` is an ordinary model and the
expected bytes owe nothing to the subject.

### 14.5 Staging and acceptance

Stage A (Rust and Python, both exist as packages): P1 round trip and encoder
model, the prefix-free claim, the trailing byte and non-minimal mutations, the depth
claim P4 on both chains (`Tagged` and `Arr`) for `n` in 0 to 140, on the decoder
and on `try_encode` (9.5), the CBR01 `cases.json` rows as WX-E tables, the corpus
controls RS1 to RS6 and RS15 and the adapter controls RS7 to RS14. Stage B (TypeScript, Go, then the rest of R2a): the same,
with the adapter hazard rows (`__proto__`, a nil pointer, a shared slice). Stage C:
the remaining lanes of R2b and R2c, with the lanes that cannot carry the depth
recorded `not-live` (8.6).

**Acceptance for the pilot.** (1) P1 to P4 pass in Rust and Python at `slim` and at
`full`; (2) every control that applies is **caught** in both, and every one that does
not prints its condition; (3) the counts of 14.4 are reproduced by the oracle and by
both lanes; (4) the adapter obligations RO1 to RO3 pass in both; (5) the report
prints the scope, the label `bounded-set`, `fuel: measured`, `ops adopted 3 of 8
public entries`, the lane-by-op matrix with every `not-live` cell, and
`flood-resistance: not evidenced`; (6) the three crate-level findings, if any,
are filed under VER15 with their weakness id, never fixed inside this PR; (7) the
capability jail run (VER08b) gives identical digests. The pilot is **not** complete
until the Python lane passes, because one lane cannot test the polyglot design.

## 15. Diagnostics

Existing codes keep their meaning and are reused wherever the condition is one they
already name. Four codes are new.

| ID | Slice | Phase | Trigger |
|---|---|---|---|
| **W0770** | R1 | checking | recursive type declarations (RM2, RM3): a type with no finite value (no base case); a group of more than `MAX_SCC_TYPES` types; an opaque type inside a recursive type; a recursive value as an `inout` parameter or in stateful state; (R3) polymorphic recursion, a mention of a generic type at arguments other than its own parameters |
| **W0771** | R1 | checking | structural recursion (RR1 to RR5): a recursive call on a constructed or non-derived term; a level call to an earlier or the same member; a term used by two recursive calls; a `rec model` with no `decreasing` parameter or with one that does not exist; more than `MAX_REC_MODELS` members; a `rec model` that calls a later ordinary model or an operation; a `rec` call through a function reference whose element is not a derived term |
| **W0772** | R1 | checking, generation | recursive domains (RD1, RD11): a `shape` on a non-recursive binder; both or neither of `depth` and `nodes`; a bound over its maximum; a leaf alphabet over 16, not ground, with a duplicate or ill-typed member; an unknown leaf type; a float leaf with no alphabet; a second recursive group or an opaque type reachable from the binder's type; a `nodes` shape over a `Set` of group values or a `Map` keyed by them; an empty domain; a `shape` that is not a top-level conjunct; a `label full` asserted for a recursive binder |
| **W0773** | R1 | generation | recursive binding (RL5, 8.1): a `rec` form outside the lane's closed list; `own` missing where the lane table requires it; `take` without a `free`; a `host_depth_limit` above `MAX_NEST` |

Reused: **W0101** (a v2 construct in a v0 or v1 document, RM0), **W0201/W0203** (a
reserved name; a cycle through ordinary models, a cycle in a v0 or v1 document),
**W0301** (a type error, `==` on a type that contains a float), **W0401** (a
`secret` label on a recursive value, RT6), **W0501** (a malformed `rec`, `own` or
`host_depth_limit` entry), **W0502** (a `faults=` list that omits an applicable
control of section 11), **W0601** (a claim with no deep or branchy case, no nodes
visited, a depth claim that never reaches its limit, a canonical-form claim whose
`Ok` arm is never taken, an empty domain), **W0602**
(function-reference nesting over 4), **W0701** (a control that does not apply, a
lane without the level `ir.rec`), **W0703** (frame or fuel exhaustion at checking
and ground evaluation), **W0706** (a `shape` size, a product, or a total fuel over
budget), **W0712** (a WX-D function argument that is not a first-order model
reference; **a `rec` member is a model reference**, RR5).

### 15.1 Tags and codes, from the registry

The allocation registry is **VER04i section 2.6.1** (VER30 decision 19). Its rule: a
specification checks the registry, takes the lowest free value and adds its row **in
the same PR**; a later collision is resolved in favour of the earliest-merged
allocation. That registry edit is the one change this document's PR makes to
another specification (17.3).

* **Stream tags.** Held: `00 01 10 11 20 30 40 41 50 51`, `60 61`, `62 63`, `64 65`,
  `66 67` (VER04o, spec only), `80 81` with `82` to `8f` reserved. Free: `68` to `79`,
  `90` to `ff`. **Taken here: none.** `68` to `79` stay free (RC1: one value, one
  encoding). The registry's two sentences that said VER04s "if it needs one, takes
  `68`" are rewritten to say it needed none.
* **Diagnostic codes.** Held: the v0 set `W0001` to `W0602`; `W0701` to `W0713`;
  `W0730` to `W0733` (VER04m); `W0750` to `W0759` (VER04n); `W0760` to `W0769`
  (VER04o). **Taken here: the block `W0770` to `W0779`, with `W0770` to `W0773`
  assigned and `W0774` to `W0779` unassigned and reserved**, by the same
  block-of-ten rule as VER04m, VER04n and VER04o. The registry's row `W0770` and up
  becomes `W0780` and up.
* R1-S0 re-reads the registry before any code is written.

## 16. Conformance

A lane, the generator and the oracle each pass the **`v2-recursive` category** of
the shared conformance kit (VER04r 11) before they may write or accept a manifest.
The category is specified here and the fixture files are created by slice R1-S2; this
document creates none.

| Case | Checks |
|---|---|
| `rec/inhabited-ok`, `rec/inhabited-refused` | `Node`, `Expr` accepted; `Chain`, `Loop`, `Stuck` are W0770 |
| `rec/mutual-ok` | `Stmt` and `Expr` are one group; forward reference accepted |
| `rec/strict-ok`, `rec/strict-term`, `rec/level-order`, `rec/linear-use`, `rec/fold-step` | the accepted and refused forms of RR1 to RR5, with the diagnostic and the rule named |
| `rec/frames-1000` | a 1,000-deep value through the evaluator: no host recursion (a recursive evaluator fails) |
| `rec/fuel` | exhaustion at the same instruction in every implementation; `ir-fuel` split by subject dependence |
| `rec/nth-expr`, `rec/nth-json` | the unranker reproduces the ten values and `nth(7)` of 5.6, the sizes 9, 198, 79,011 and the refusal of 12,485,713,284 |
| `rec/nth-nodes-list` | the `nodes` order for a type with a `List` field and a three-field constructor (RD4): every value of `nodes <= 4`, listed, and `nth` at each boundary between lengths and between node counts |
| `rec/shape-refused` | W0772 for a second recursive group, a `nodes` shape over a `Set` of group values, a float leaf with no alphabet; W0706 for tables over `MAX_SHAPE_TABLE` |
| `rec/slim-ctors` | the constructor chunks of RD12b are in `slim_chunks`; a list without one is rejected by the oracle |
| `rec/key-group` | `A = X(B)`, `B = Y(A) \| Z(List<U8>)` has no key type (RM6); `Set<A>` is W0760 |
| `rec/refused-release` | a 10,000-deep value of the subject's host type is refused and then released without a crash (RL1a) |
| `rec/order` | `ord` on `Expr`: constructor index first, so `Lit(true) < Not(Lit(false))`; `Set<Expr>` canonical encoding; the strict-order decoder rejects a repeat |
| `rec/stream-roundtrip` | `Tagged(24, Nil)` and `Not(Lit(true))` encode as in RC1; the decoder re-encodes equal |
| `rec/decoder-nest`, `rec/decoder-cells`, `rec/decoder-count` | `value-depth` at `MAX_NEST + 1`, `value-cells`, a count bomb with nested counts |
| `rec/type-cycle` | a hostile table (uninhabited, over `MAX_SCC_TYPES`, a lying `rec` field) is `type-cycle` |
| `rec/adapter-fixtures` | the deep, wide, shared and cyclic host fixtures of RO2, each within the step bound of RL1 (`MAX_VALUE_CELLS + 1` nodes visited); the controls RS7 to RS14 are each caught |
| `rec/faults` | RS1 to RS6 and RS15 each change the value at the stated witness; `rec_accept` and `rec_limit_low` are refused outside a fault |
| `rec/hostile-keys` | the keys of RO1 survive the host form |
| `rec/same` | `same` lifted through a recursive type with an `F64` field |

## 17. Requirements on other specifications

This document edits no other specification **except the one registry edit of 15.1**.
Each requirement below is a follow-up to be filed.

### 17.1 VER04j, VER04k, VER04o

* **VER04j 4.2** (no recursion): replace "must be acyclic" by "must be well founded in
  a v2 document (VER04s RM2); a cycle in a v0 or v1 document is W0203".
* **VER04k 2.7 and 2.8**: add the per-frame depth counter and `MAX_REC_FRAMES` for the
  members of a `rec` group (RR6); lift W0203 for cycles through `rec` members only
  (RR5); state that the work counter is the fuel (RR7). `MAX_HO_NESTING` unchanged.
* **VER04o CM5, CK12, 3.2, CT2**: CM5's refusal of recursion through `Set` and `Map`
  is relaxed to well-foundedness; CK12 keeps refusing **polymorphic** recursion and
  accepts a recursive generic type at its own parameters (slice R3); the key-type
  table gains the row of RM6; CT2's "three levels in one type" is stated as a
  type-expression nesting (RM3a); section 19 is closed by this document.

### 17.2 VER04r

* **2.2 rule 1** (types structural, no recursion): the cycle rule of RC3, the
  `rec` field (required exactly for group members), the error `type-cycle`.
* **4.1** (levels): the level `ir.rec` and its tags (RC5, RC6).
* **4.2 rule 6** (static cost): the dynamic fuel and frame meter for `rec` groups
  (RR6, RR7, RB5 to RB8); the values bound of RB6.
* **5.1** (descriptors): `rec_depth` and `rec_nodes` with `size()` and `nth(i)` (RD6);
  the **list descriptor** keeps its reduced alphabet outside a `shape`.
* **8.1** (fault tags) and **11** (kit): the fault-only tags of RC5 and the category
  `v2-recursive` (16).
* **14** (security table): the constants of 6.1 and the totals of RB8.

### 17.3 VER04i, VER04l, VER04m, VER04n, VER29, VER30, backlog

* **VER04i 2.6.1**: the registry rows of 15.1 (this PR), and the lane table for the
  recursive host forms of 8.3.
* **VER04l**: `within` is unchanged; `shape` is its sibling and shares the domain
  rule of 4.3.
* **VER04m, VER04n**: a recursive value as `inout` or in object state, and an opaque
  inside a recursive type, are W0770 in the first slice; the observer of a recursive
  value is the form (3.7).
* **VER29**: a claim over a recursive type is published **with its scope**
  (`bounded-set`) and resolves above a layer as that, never as unbounded (RT9). The
  canonical type digest of RT9 is a new field of the published claim.
* **VER30 and the backlog**: section 19.2 lists the rows; this document edits neither.

## 18. Out of scope

* **Coinduction and infinite or lazy data**: streams, `let x = Arr [x]`, corecursion,
  productivity checking. Haskell and other lazy hosts are *defended against*
  (8.5), not modelled.
* **Cyclic graphs and observable sharing**: a graph, a DAG with identity,
  hash-consing as semantics, equality up to sharing, reachability. A cyclic host value
  is a named failure (RL1). A back-reference stream tag belongs to this story and is
  not taken (RC1).
* **General recursion**: recursion that is not structural (recursive descent on an
  input position, a loop with a counter, Ackermann). A parser's *model* is written as
  a `fold` over its input with an explicit stack, or its **inverse** is the writer
  `enc` (9.2); a document that needs more says so and waits.
* **Polymorphic recursion**, indexed families, type-level recursion, higher-kinded
  recursion (a recursive type through a type constructor parameter).
* **Recursion through function types**: a closure in a tree (VER04o 20, callbacks).
* **Recursive opaque types and handles**: a tree whose nodes are objects with
  identity (VER04n OQ-12).
* **`List`, `Set`, `Map` and float as keys**: unchanged from VER04o 20, except for
  the recursive key types of RM6.
* **Proofs**: every recursive claim is `not-live` for the proof rung (RD9). An
  induction principle for a `rec model` is a VER10 story.
* **Unbounded claims**: "for every tree" as evidence. The scope is always printed.
* **The proportion of a domain**: no sampling distribution over tree shapes is
  specified beyond uniform `nth(i)` (RD10).
* **Hash-flooding resistance of a subject** as a claim: printed `not evidenced`
  (VER04o CT4, RT4).
* **Generic recursive types** (`Tree<T>`) are specified as rules here (W0770) but
  **implemented in slice R3**, after VER04o G1.

## 19. Implementation slices and backlog rows

### 19.1 Slices

| Slice | Scope | Prerequisites |
|---|---|---|
| **R0** | this specification; the adversarial review recorded in section 21 | none |
| **R1-S0** | errata pass against the then-current VER04j, k, l, o, p, r; reconcile sections 6 and 7 with VER04r and file the requests of 17; re-confirm no tag and the block `W0770` to `W0779`; reserved names; accept the recursion constructs in the `v2` header parser; measure the evaluators and the pilot's models and fix `MAX_TOTAL_FUEL_*` (RB8); draw up the pilot's lane-by-op matrix (14.1) | R0 reviewed; T1-S0 (`v2` accepted) |
| **R1-S1** | checker and ground evaluator: groups, inhabitation, `rec model`, RR1 to RR5, `depth_of nodes_of cells_of least`, iterative equality and `ord`, `shape`, the frame and fuel meter, the labels | R1-S0; VER04o C1-S1 (the `witness<T>` call form; `Map` and `Set` for `Json`) |
| **R1-S2** | generator at `ir.rec`: the type-table cycles, the descriptors and the counting tables, `nth`, the stream (no tag), the fuel prediction, the faults RS1 to RS6, the independent oracle (an independent unranker), the kit category `v2-recursive` | R1-S1; VER04r slices S3 to S5 |
| **R1-S3** | Rust lane: binding reader (`rec`, `own`, `host_depth_limit`), RL1 and RL2, RO1 to RO3, the adapter controls RS7 to RS14, the shared fixture in Rust | R1-S2; the shared-crate queue of VER30 5.3 |
| **R1-S4** | Python lane, with the oracle cross-check | R1-S3 |
| **R1-S5** | pilot `canonical-cbor`, stages A (Rust, Python) and the CBR01 rows | R1-S4; the `ir.seq` amendment (VER04r 4.1); WX-D2; T1 |
| **R2a** | lanes typescript, go: adapter, fixture port, hazard-row tests, controls | R1-S5; the lane's v2 runner (VER04r S6, S7) |
| **R2b** | lanes ruby, c, cpp, java, kotlin, csharp, fsharp, elixir | R2a |
| **R2c** | lanes lua, perl, haskell, swift, dart, ocaml | R2b |
| **R3** | generic recursive types (`Tree<T>`): regular recursion accepted, polymorphic refused, monomorphised groups | R1-S1; VER04o G1-S2 |
| **later** | `List` keys, DAG and sharing, a back-reference tag, an induction story | demand |

Implementation order. VER30 puts WX-R1 **after the four features** (decision 15,
C15). By dependency, R1-S0 to R1-S4 need only VER04o C1-S1 and the `v2` header, and
the pilot needs T1, WX-D2 and the `ir.seq` meter, and none of floats (WX-FL1),
opaque types (WX-O1) or generics (WX-P1); the section 20.1 question asks whether
to keep the VER30 order or let the spec and checker slices start earlier.

### 19.2 Backlog rows needed (this document does not edit `VER-BACKLOG.md` or VER30)

* **WX-R0**: spec recursive types (this document); status: written; **adversarial
  review required before R1-S0**. (The row exists as P3 / WX-R1's predecessor in
  the backlog; this updates its status.)
* **WX-R-REVIEW**: the adversarial review of this document. One review ran on
  2026-11-16 (the type system, structural recursion, domains, the encoding,
  security and the pilot; section 21). **Still owed before R2:** a lane-specialist
  read of the per-lane table of 8.3 and of RL1a, which no reviewer has checked
  against a running host.
* **WX-R1** (R1-S0 to R1-S5): implementation, pilot `canonical-cbor`; depends on
  T1, WX-D2, the `ir.seq` amendment, VER04o C1-S1, and (VER30 order) WX-O1, WX-M1,
  WX-P1, WX-FL1.
* **WX-R2a, WX-R2b, WX-R2c**: the lane rollout, with the recursive fixture port per
  lane as its own package (BUILD, README, CHANGELOG, `required_capabilities.json`).
* **WX-R3**: generic recursive types, after WX-P1.
* **WX-R-PILOT2**: `bounded-json`, then `json-serializer` and `json-value`, after
  FL2.
* **VER04j, VER04k, VER04o, VER04r amendments** (17): each a PR of its own.
* **VER04i amendment** request (17.3): the registry rows (this PR) and the lane
  table.
* **VER13**: an assumption `A-REC-HOST`, "the lane's adapter converts the recursive
  host forms it binds faithfully and within its caps", discharged per lane by the
  `v2-recursive` kit (RO1, RO2).
* **VER29**: the published form of a recursive claim carries its scope and the
  canonical type digest (RT9).
* **VER30 and the inventory**: a change record (decision 15 is fulfilled by this
  specification; the pilot is `canonical-cbor`); re-measure the recursion rows once
  `Map`, `Set` and generics land, since the 74 "recursion alone" crates and the
  775 joint figure were computed before the key-type and list-of-pairs choices of
  RM6 and RT5.

## 20. Open questions

### 20.1 For the owner

| # | Question | Recommended default (encoded above) |
|---|---|---|
| OWN-R1 | Keep the VER30 order (WX-R1 after floats, opaque types, collections and generics), or let R1-S0 to R1-S4 start after VER04o C1-S1 alone? | **Open; the owner's.** Recommended: **keep the VER30 order.** Three reasons found in review. The `warrant-grammar` queue is serial (VER30 5.3), so starting R1-S1 early delays the four features rather than running beside them. R1 changes rules that the four are still landing on (VER04k's depth and work meter, VER04r 4.2 rule 6, VER04o CM5 and its key table), so an early start means amending moving text. And the pilot cannot finish early in any case: it needs T1, WX-D2 and the `ir.seq` meter. The one thing worth doing now is **R1-S0** (the errata pass and the measurement of the fuel totals, RB8), which writes no checker code and tells the owner whether the pilot fits PR CI |
| OWN-R2 | Confirm the pilot: `canonical-cbor` (17 lanes, no dependencies, canonical form is the property), `bounded-json` second; and accept that it is a **partial** adoption (3 of 8 public entries, 14.1) | **Open; the owner's.** Recommended: **confirm `canonical-cbor`, labelled partial.** Review re-checked the facts against the tree: zero dependencies in Rust and Python, 16 ports, depth 128 accepted and 129 refused, the encoder sorts and rejects duplicates, the decoder demands strict order. The alternative `bounded-json` has three lanes and needs FL2. What the owner is asked to accept is that the pilot's report says `3 of 8`, that `encode` is `not-live` in ports without it, and that the pilot is not called done on Rust alone |

### 20.2 Engineering questions

| # | Question | Default (encoded above) |
|---|---|---|
| RQ-1 | `List<K>` as a key type (so `Json`-like and `Node` types become keys)? | No (RM6); it needs a host key story for every lane; reopen with a customer |
| RQ-2 | May a document declare a constant multiplicity in RR4 (a term used by up to `k` recursive calls)? | No; linear use is the only rule that gives a polynomial bound without a depth parameter. A model that needs two folds uses one fold with a tuple accumulator |
| RQ-3 | Is `MAX_NEST = 1,024` right for the inventory (every crate checked caps nesting at 128 or less)? | Yes, a factor of four over the largest two-cell-per-level chain; R1-S0 confirms per lane, and a lane may lower it (8.6) |
| RQ-4 | Is `depth_of` counting group nodes only (a `List` between two nodes counts nothing) the right user-facing measure? | Yes, it matches "JSON nesting" and "CBOR depth"; `nest` is the cap measure and stays internal |
| RQ-5 | Does `rec model` need the explicit `decreasing` clause? | Yes for R1: it is documentation and makes the checker a simple pass; inference may follow |
| RQ-6 | Forward references among type declarations in a v2 document? | Yes (RM3): types are resolved in a first pass; only `rec model` is forward-referenceable among models |
| RQ-7 | Shared subtrees in host values: reject, or accept and count per path? | Accept and count per path (RL1): rejecting sharing would reject legitimate hosts (interned strings, shared constants) and needs a visited set |
| RQ-8 | Is `least<T>()` needed as a language built-in, or only inside `witness`? | A built-in: faults (RS4) and a model that needs a default tree use it |
| RQ-9 | A `shape` over `Set<Self>`? | Yes (RD2 table), but a recursive `Set` needs a key type (RM3a) |
| RQ-10 | The fuel and totals (`MAX_IR_FUEL`, `MAX_TOTAL_FUEL_*`)? | Provisional (RB8): the pilot is, by hand, at the slim total. R1-S0 measures and fixes them in either direction before any `ir.rec` corpus is committed; afterwards they only go down |
| RQ-11 | How is a cross-package recursive type kept identical (RT9)? | A canonical type digest compared by the generator; a mismatch is a corpus error. The VER29 field is requested (17.3) |
| RQ-12 | Does the encoder-side size cap (`max_encoded_bytes` 1 MiB) of the pilot have a Warrant form, given values are at most 4,096 bytes? | Not in the first slice: `not-live`, printed (14.1); an encode-only builder is a follow-up |
| RQ-13 | May `admit` call a `rec model`? | Yes (`canonical(v)` in 14.3): `admit` is a pure Bool IR and a `rec model` is pure; it is charged to the same fuel |
| RQ-14 | A `fuzz` engine that draws trees by a size-biased grammar instead of uniform `nth(i)`? | Not specified (RD10, 18); uniform by rank is the only distribution the oracle can reproduce |
| RQ-15 | A `shape` over a type that reaches a second recursive group? | Refused in R1 (RD1, W0772): one bound cannot bound two groups. A per-group bound is a later extension |
| RQ-16 | What label does a budget-demoted recursive binder carry? | `sample`, as VER04r 9.3 says for every demoted binder (RD3). The undemoted default `rec_d1` is `bounded-set` |
| RQ-17 | How does `slim` treat a tree domain of more than 16 chunks? | The VER04r selection plus one chunk per root constructor (RD12b), fail closed at `MAX_SLIM_CHUNKS` |
| RQ-18 | Is the work of a `rec model` linear? | Per group, in invocations (RR4). Across `s` groups it is degree `s` (RR4a), and a body with a size-charged form has no static bound; fuel is the limit and the report says `measured`, `proven` or `dynamic` (RB7) |
| RQ-19 | What happens to a subject's value that the adapter refused? | It is dismantled by a worklist or kept until exit, never handed to a recursive host destructor, and in C never passed to the subject's `free` (RL1a, RL5) |

## 21. Specification change record

* Initial VER04s (2026-11-16): normative WX-R0: recursive and mutually recursive
  algebraic types as inductive, finite trees; well-foundedness by inhabitation
  (W0770); the measures `depth_of`, `nodes_of`, `cells_of` and the caps `MAX_NEST`,
  `MAX_VALUE_CELLS`; structural equality, `same` and the canonical order extended
  to recursive key types, `least`; `rec model` with a decreasing parameter, the
  subterm, rank and linear-use rules (W0771), frames and fuel as the second layer;
  the `shape` predicate and the domains `rec_depth` and `rec_nodes` with counting,
  enumeration order, unranking and the default `rec_d1` (W0772), always
  `bounded-set`; resource bounds, depth bombs, exponential domains and the cost
  model under VER04r; the encoding (no new stream tag, `65` nests, the type-table
  cycle rule, the level `ir.rec`); the lane binding contract (an iterative adapter,
  per-path counting, a per-lane table for eighteen lanes, C ownership, lazy hosts,
  W0773); parser-style claims P1 to P4; three obligations; fourteen sabotage controls
  (RS1 to RS14); security (stack exhaustion, hostile corpora, sharing and cycles,
  hash flooding); the pilot `canonical-cbor`. Departures from VER04o 19 are D1 to D5
  of 1.2. New diagnostics `W0770` to `W0773` in the reserved block `W0770` to
  `W0779`; **no stream tag**. Edits no other specification but the VER04i 2.6.1
  registry rows; requirements on VER04j, VER04k, VER04o, VER04r, VER04i, VER29 are
  section 17.
* **Adversarial review, 2026-11-16** (one reviewer; the per-lane hazards of 8.3 and
  RL1a have still had no lane specialist). Defects found and fixed in the text:
  (1) the termination measure of RR3 was stated as rising, and is now the
  decreasing pair; RR4 did not say that a level call's argument counts, nor when
  two `at` terms are disjoint. (2) "Work is linear" was false across groups and
  for bodies with size-charged forms: RR4a, 4.5 and RB7 now say what is bounded,
  and the fuel mode gains `measured`. (3) RB7's total, taken with the old static
  bound, refused the pilot itself; RB7a restates it, and RB8 records that the
  pilot is at the slim total by hand and that R1-S0 fixes the constants. (4) A
  `shape` over a type that reaches a second recursive group was an infinite
  domain counted as finite (now W0772), and a non-recursive record inside a group
  was given its unbounded "ordinary" domain (now expanded structurally). (5) The
  `nodes` count and order were under-specified for `List`, `Option` and `Map`
  fields, so two implementations could disagree; RD2 and RD4 now give the
  recurrences and the order, `Set` of trees is refused there, and a conformance
  case pins it. (6) 5.9 claimed the first chunk holds the smallest trees, which is
  false in the `depth` order; RD12a to RD12c define what `slim` thins and add
  constructor chunks. (7) A demoted default was called `bounded-set` against
  VER04r 9.3; it is `sample`. (8) The key-type rule let one member of a mutual
  group be a key type while another was not; it is now a fixed point over the
  group. (9) The type-table `rec` field was optional, giving one table two JSON
  forms; it is required exactly for group members. (10) P3 could pass with no
  accepted input and had no control that could fail; RV11 and RS15
  (`accept-malformed`) are added, and RS6 is redefined as an IR rewrite
  (`limit-low`) because a corpus fault cannot edit the subject's comparison.
  (11) The depth-bomb sketch compared the bytes `c0..00` with a chain ending in
  `Nil` (`f6`); it is `Uint(0)`, and the off-by-one between the subject's wrapper
  count and `depth_of` is stated. (12) A subject's refused deep value was left to
  the host's recursive destructor, and in C to the subject's `free`; RL1a and
  RL5 forbid both. (13) The pilot said it adopted `encode` but declared no such
  op; it is declared on its precondition, the five entries not adopted are
  tabled, the ports' differing surfaces are recorded, and the report prints `3
  of 8`. (14) Smaller: four measures, not three; `within(..., 4)` in 9.3; the
  cyclic failure may be `value-cells`; the inventory path. **Confirmed, not
  changed:** no stream tag (the existing tags are self-delimiting and decoding is
  type-directed, so nesting is injective; VER04o expected the same); every value
  is finite and inductive; the counts of 5.6 and 14.4. **Checks the author
  made:** the stream tags and diagnostic codes of 15.1 against every specification
  on the base as it stood; the section references against the texts as read on
  2026-11-16; the counts of 5.6 (`N_1` to `N_5`, `V_1` to `V_4`) and 14.4 (`V_2`
  of 812,160, admitted 393,660, discarded 418,500, the 13 chunks, the 111,151
  `within` cases) by hand. **Not checked:** no fragment has been run through any
  checker; the behaviour of `canonical-cbor` and `bounded-json` beyond their public
  signatures and the code named in section 14 was not run; the interaction of RR4
  with the real `fold` forms of VER04k 2.3 is argued, not tested; the constants of
  6.1 are provisional until R1-S0 measures them.
