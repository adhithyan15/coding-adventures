# VER04o — Warrant Collections and Generics (WX-M, WX-P)

**Status:** Specification only. Nothing here is implemented and no code changes
with it. It is the normative text for backlog items WX-M0 (maps and sets) and
WX-P0 (the first slice of generics), which the design memo put in one document
because they share one definition (the key order `ord`) and one customer set.
The implementation slices are C1 and G1 (section 21). The owner decisions of
2026-11-16 are recorded in section 1 and applied throughout. The Warrant
fragments are **sketches**: none has been run through the checker, names are
illustrative until the C1-S0 errata pass fixes them, and every case count in the
text was computed by hand from the stated domains, so a reviewer can redo it.
Claims about what a package does are "from reading": the public signatures and
the code named in the text were read, not run. The freeze on collections and
generics is lifted **spec first, with adversarial review**. That review has **not
been done yet**: section 23 records only the author's own checks (tags, codes,
section references, counts). C1-S0 and G1-S0 do not start until the review has run
and the owner has answered section 22.1.

**Parent:** [VER04 §2–§8](VER04-warrant-language.md) (§3: values and equality;
§5: static parameters, `op name<N: Nat>`), [VER04d](VER04d-warrant-type-system.md),
[VER04h](VER04h-warrant-derived-evidence.md) (domains, budget `B`, controls),
[VER04i](VER04i-polyglot-warrant-evidence.md) (the eighteen lanes, the polyglot
rule that every feature runs in every lane with equal rigour),
[VER04j](VER04j-warrant-v1-extensions.md) (header, fixed-width types, tuples,
records, variants, the no-recursion rule, `Bytes<N>` and `corpus.json.widths`,
WX-E tables, WX-H bounds), [VER04k](VER04k-warrant-v1-wave4.md) (WX-D sequence
built-ins and the meter, on which every element quantification here is built),
[VER04l](VER04l-warrant-text.md) (header `v2`, `Text`, `within`, mandatory
boundary rows, style), [VER04n](VER04n-warrant-opaque-types.md) (opaque types:
generic opaque types are stated on top of it, section 6.9),
[VER04p](VER04p-warrant-floats.md) (style; the float relations that lift through
map values), [VER29](VER29-layer-contract-linking.md) (strength rungs), the draft
**VER04r** (`warrant-corpus-v2`, branch `claude/warrant-corpus-v2-spec`: the
encoding of section 9), [VER30](VER30-warrant-expansion-roadmap.md) (the frozen
features), [VER-BACKLOG](VER-BACKLOG.md) LP-012 (the honest kinds).

**Section references.** As in VER04n and VER04p, where this document needs a
corpus, binding or language rule from another specification it states a
**requirement** on that specification (section 18); it does not edit VER04, VER04d,
VER04i to VER04n, VER04p, VER04r, VER29, VER30 or `VER-BACKLOG.md`. The backlog rows
it needs are listed in section 21.2. Every reference to VER04n, VER04p and VER04r is
to the text as read on 2026-11-16; C1-S0 re-reads them.

**Design source:** an advisory design memo (not committed), sections 2 and 3
(collections WX-M, generics WX-P) and its sections on order, dependencies and
pilots. Where this document departs from it, section 1.2 says so and why.

## 0. Why this exists

Warrant has lists and bytes, which are ordered and carry duplicates, and nothing
for the two shapes most real libraries use to hold data: a **set** (membership,
no repeats) and a **map** (a key finds a value). The inventory counts about 654
crates that mention collections or generics somewhere in a signature (an upper
bound: ordering and other blockers are folded in). Counted alone, collections
unblock about 4 crates and generics about 36; the weight is in the pilots:

| Shape | Real API (from reading) | What a plain `op` cannot say |
|---|---|---|
| a map of untrusted data | `csv_parser::parse_csv(&str) -> Result<Vec<HashMap<String, String>>, CsvError>`: header names are keys, a duplicate header keeps the last column, a short row is padded with `""` | "each row is the map of the header columns to this row's fields, last duplicate wins" |
| a generic container | `hash_set::HashSet<T: Eq + Debug>`: `from_list`, `add`, `contains`, `size`, `to_list`, `union`, `intersection`, `difference`, `is_subset`, ... over any `T` | "for every element type, `add` then `contains` is true, and `to_list` is exactly the set, whatever order the hash table iterates in" |

Both are security-flagged targets in the memo: `csv_parser` reads untrusted
files, and a hash table is the textbook algorithmic-complexity target (CWE-407).

The design keeps the one principle of VER04k, VER04m, VER04n and VER04p: **the
document stays pure and never contains a host notion.** A host `HashMap` has an
iteration order, a seed, a capacity and a load factor. A Warrant `Map` has none of
them:

* a **set** is a mathematical finite set and a **map** a finite partial function;
  equality is extensional;
* there is **one canonical order**, the total order `ord` on the key types, and
  every list that comes out of a collection is in that order. A host whose order
  is unspecified cannot be bound to a `List` result (section 9.2);
* a **generic claim is a schema**. Evidence exists only for the instances the
  document lists, each instance is an ordinary monomorphic suite, and no lane ever
  sees a type variable. Parametricity ("one type suffices") is **not** evidence.

What is added:

| Added | Where |
|---|---|
| types `Set<K>`, `Map<K, V>`; the key types and the order `ord`; 20 built-ins, all prefixed `set_` or `map_`; extensional `==` | 3 |
| claims over collection values and over their elements; the element vacuity guard | 4 |
| the order-witness alphabet `witness<T>()`, the domains `set_d1`, `subset_of`, `maps_over`, `map_d1`, labelled by LP-012 kind | 5 |
| rank-1 type parameters, four built-in constraints (`Eq Ord Finite Bits`), the `instantiate` declaration, monomorphisation, instance coverage, `all-instances`, generic opaque types | 6 |
| surface syntax, typing rules and diagnostics `W0760` to `W0765` | 7, 16 |
| the corpus encoding: stream tags `70`, `71`, descriptors, the IR level `ir.coll`, instance suites | 8 |
| the lane binding contract: order classes, a per-lane table for eighteen lanes, four generic modes, the lane-by-instance matrix | 9 |
| four obligations and their controls | 10 |
| sabotage controls: corpus faults, adapter controls, generic faults | 11 |
| honest labels, counting and vacuity guards | 12 |
| security considerations: nested-collection bounds, count bombs, hash flooding | 13 |
| the pilots `csv-parser` and `hash-set` | 14, 15 |
| what VER04s (recursive types) will need from this design | 19 |

### 0.1 Slices, and how rules are marked

| Prefix | Content | Section |
|---|---|---|
| `CM1`, ... | the semantic model of collections | 3 |
| `CQ1`, ... | claims over elements | 4 |
| `CD1`, ... | domains | 5 |
| `GM1`, ... | the semantic model of generics | 6 |
| `CX1`, ... | surface syntax | 7 |
| `CK1`, ... | checker (type) rules | 7 |
| `CC1`, ... | corpus and encoding | 8 |
| `CL1`, ... | lane binding contract | 9 |
| `CO1`, ... | evidence obligations | 10 |
| `CS1`, ... | sabotage controls | 11 |
| `CB1`, ... | labels and vacuity guards | 12 |
| `CT1`, ... | security | 13 |

Slices: **C1** collections (sets and maps, the domains, `ir.coll`, the Rust and
Python lanes, pilot `csv-parser`), **C2** the lane rollout for collections, **G1**
generics (type parameters, constraints, `instantiate`, monomorphisation, generic
opaque types, the four lane modes, pilot `hash-set`), **G2** the generics lane
rollout. A rule binds the slices that implement what it names. G1 depends on C1
(the `Ord` constraint is the key order) and on VER04n's O1 (generic containers are
opaque types). Section 21 lists the slices and their prerequisites.

## 1. Decisions of 2026-11-16, and departures from the memo

### 1.1 Decisions

| # | Decision | Applied in |
|---|---|---|
| a | The freeze on collections and generics is lifted: specification first, with adversarial review before any implementation | the status paragraph, 21, 23 |
| b | **One canonical map order.** A host with unspecified iteration order cannot bind to a `List` result | 3.3, 9.2 |
| c | **Generic claims are never `exhaustive`.** Evidence covers the listed instances, except `all-instances(n/n)` over a closed universe. Coverage is counted per lane **and** per instance | 6.5, 6.6, 9.6, 12 |
| d | **Honest labels** (`full`, `bounded-set`, `sampled`, the LP-012 names), fail closed, layered verification (a lower contract is assumed above, never re-proved), and the runtime capability jail stays mandatory | 12, 13 |
| e | Pilots: `csv-parser` for collections, `hash-set` for generics | 14, 15 |
| f | **Recursive types are lifted too**, but get their own later specification, **VER04s**, written after this one. Collections and generics are designed so recursive algebraic types can be added cleanly; this document says what VER04s will need and specifies no recursion | 3.8, 19 |
| g | **Map iteration order is deterministic in the corpus**: every collection in a stream, a descriptor or a result is in canonical order | 3.3, 8 |
| h | **Per-lane mapping** for languages without generics (C, Lua, Perl) and with erased generics (Java, Kotlin) | 9.5 |
| i | The interaction with the `Bytes<N>` const parameters of VER04 §5 and VER04j is stated, not left to the lanes | 6.8 |

Decision (b) in plain words: Rust's `HashMap`, Go's `map` and Swift's `Dictionary`
iterate in an order that differs between two runs of one program. A claim that
compares the iteration order of such a host with a model list would pass or fail
by chance. So that host's result may bind only to `Set` or `Map`, where the
adapter sorts it (with a comparator of ours, never the host's), or to a `List`
only when the host's order is a stated fact (`sorted` or `insertion`) that the
claim itself then tests.

Decision (c) in plain words: "it works for `U8`" and "it works for `Text`" are two
facts, and the document lists exactly which of them it has evidence for. A schema
says `instances(3)`, never "for all types". The one exception is a parameter
constrained to a type universe the language itself closes and finite, such as the
eight fixed-width integers, where "all eight, each run" is a complete statement.

### 1.2 Where this document departs from the memo

| # | Memo says | Here | Why |
|---|---|---|---|
| D1 | built-ins `member get insert remove put del size union inter diff keys to_list entries` | every built-in carries a prefix: `set_member`, `map_get`, ... | the pilot `hash-set` declares operations named `union`, `intersection`, `difference`, `contains`, `size` and `to_list`, and `hash-map` names `get`, `set`, `delete`, `has`, `merge`. A bare reserved built-in would force a rename of the document's own operations (W0201/W0203 in v2), and `to_list` is already a WX-D built-in on `Bytes` (VER04k 2.2, which has no overloading) |
| D2 | "no new binder form" and a default `set_d1` of "empty, singletons, all subsets of the first four members" | the multi-element members of `set_d1` are pairs of the **order-witness alphabet**, not of the first four values (5.2) | the first four values of `U8` are `0 1 2 3`; no host comparator bug (signed bytes, UTF-16 order, length-first array order) shows on them. The hazards sit at specific pairs |
| D3 | schema suites keyed `claim@T=U8` | corpus claim field `claim__i<8 hex>`, display `claim[T=U8]` (8.5) | `@` already separates a suite id from a case ordinal (VER04r 5.2); VER04r 5.5 allows a claim field only `[A-Za-z_][A-Za-z0-9_]{0,63}`, so `<>=,()` cannot appear in a chunk file name |
| D4 | Dart is an `erased` lane with Java, Kotlin and TypeScript | Dart is `static` (reified generics); TypeScript, Java and Kotlin are `erased` (9.4) | Dart keeps type arguments at run time, so an adapter arm per instance is possible and an erased row would hide that |
| D5 | `Finite`: "a complete domain within `B`" | `Finite` means at most **16** values (3.2) | the only use of `Finite` is `all_values<T>()` feeding `subset_of`, whose cap is 16 (5.3). A larger bound would promise `full` for a domain the generator refuses |
| D6 | the sketch `op add<T>(s: HashSet<T>, x: T) -> HashSet<T>` | an operation that returns a handle is refused by VER04n OK3; `add`, `union` and the other transformers are **makers whose plan applies them** (15.2) | VER04n section 18 defers transformers and answers builder-style construction with a plan (OR5, OL1) |
| D7 | `insertion` order may bind a `List<(K, V)>`; "the model must compute that order" | kept, but only with a document model that computes it from the construction list (3.3, 9.2); there is no ordered-map type | a type whose value includes an order would put a second meaning of equality in the language |
| D8 | stream tags `70` Set and `71` Map; `W07xx` for generic diagnostics | tags `70`, `71`, and the code block `W0760` to `W0769`, rechecked on 2026-11-16 against the tree and both draft branches (16.1) | VER04n holds `80`, `81`, `W0750` to `W0759` |
| D9 | the generic claim minimum is "two instances, one scalar and one of `Bytes` or `Text`" | per type parameter, one scalar and one **by-reference-hazard** type (`Bytes`, `Bytes<N>`, `Text` or a tuple, record, variant, `Option`, `Set` or `Map` over one) (6.5) | the hazard is reference equality and identity hashing, which composites of bytes and text share |
| D10 | `corpus.json.widths` becomes sugar for `instantiate` rows | sugar in the **checker and report** only. A document whose only static parameters are `Nat` keeps the `widths` encoding and ids byte for byte (8.5) | the acceptance gate of VER04r 1.2: `ct-compare` under v2 reproduces nine v1 digests |
| D11 | the `Ord` constraint on a generic container "needs `Ord`" | `Ord` is a constraint on the **model's** key, not on the subject: `hash-set` needs only `Eq` and a hash (6.2) | the document needs a canonical order to state `elements(s)` as a `Set`; the subject never sorts |

Alternatives considered and rejected: **sets and maps with host order visible**
(rejected: a claim over an order no lane can reproduce is a coin flip);
**user-defined type classes now** (rejected: four closed constraints cover the
two pilots, and a class with laws is a separate theory; section 20);
**inferring type arguments** (rejected: an inferred instance hides which
instance a claim is about; every type argument is written); **a type
parameter with evidence by parametricity** (rejected, 6.7).

## 2. Versioning and compatibility

**CM0.** (Header.) Collections and generics are **v2** features (VER04l 2.1): a
document is v2 exactly when its header is `warrant name v2`. A collection or
generic construct in a v0 or v1 document is **W0101** ("v2 construct in a v1
document"), never ignored. C1-S0 depends on T1-S0 having made the checker accept
`v2`; if it has not, C1-S0 makes that change itself, as VER04n O1-S0 does.

**CM0a.** (Reserved names.) In a v2 document the following become reserved
(W0201/W0203 as a declaration or binder name): the type names `Set` and `Map`; the
built-ins of 3.4 and 5 (`set_of set_member set_insert set_remove set_size
set_union set_inter set_diff set_subset set_elems map_of map_of_strict map_get
map_has map_put map_del map_size map_keys map_values map_entries witness
all_values`), the domain predicates `subset_of`, `maps_over` and `one_of`; the declaration word
`instantiate`; the constraint names `Eq Ord Finite Bits`. `all-instances` is a
label, not a token. In a v0 or v1 document all of these are **ordinary
identifiers** (an undeclared call is W0202, a declared one keeps its meaning),
which is what keeps the byte-identity gate of VER04j 1.3 and VER04l 2.3 true.
C1-S0 checks every in-repo v1 document for a collision, as VER04l 2.2 does.

**CM0b.** (Prelude parts.) `CheckedWarrantModule` gains a `collections` table and a
`generics` table, empty for every document without the features. Every new
prelude part (the canonical order, the collection built-ins, the instance
dispatch) is emitted only for a document that uses it, so the output of every
existing document stays byte-identical.

**CM0c.** (Dependencies on other drafts.) The element quantification of section 4
uses the WX-D built-ins (`all`, `any`, `count`, `fold`), which the draft VER04r
keeps in a **reserved** level `ir.seq` ("not-live until an amendment defines the
v2 meter", VER04r 4.1). So a claim that quantifies over elements is `not-live`
in every lane until that amendment lands; a claim that uses only the built-ins of
3.4, `==` and `match` needs only the new level `ir.coll` (8.4) and `ir.data`. C1
therefore has two stages (section 21): **C1-S2** needs no `ir.seq`; **C1-S5**
(element quantification, the `csv-parser` pilot) does.

## 3. The semantic model of collections

### 3.1 Values

**CM1.** A value of `Set<K>` is a finite set of values of `K`. A value of
`Map<K, V>` is a finite partial function from `K` to `V`. There is no insertion
order, no capacity, no hash and no identity in a value. Two collections are equal
iff they have the same elements (the same entries): `==` is **extensional**.

```text
set_of([3u8, 1u8, 3u8]) == set_of([1u8, 3u8])                 -- true: order and repeats are gone
map_of([(1u8, 10u8), (1u8, 20u8)]) == map_of([(1u8, 20u8)])   -- true: the last entry for a key wins
map_of_strict([(1u8, 10u8), (1u8, 20u8)]) == None              -- the duplicate is reported
```

Analogy: a set is a **guest list**, not a queue. Asking "who is on the list" has
an answer; asking "who is first" has none until the document says how the list is
read out. Section 3.3 is that rule.

### 3.2 Key types and the order `ord`

**CM2.** A **key type** is a type with the built-in total order `ord`. Key types
are the element type of a `Set` and the first argument of a `Map`. The order is:

| Type | `ord` | Note |
|---|---|---|
| `Bool` | `false < true` | |
| `U8 U16 U32 U64 I8 I16 I32 I64 Nat` | numeric; signed types in signed order | not the host's order in languages with only signed bytes |
| `Bytes`, `Bytes<N>` | unsigned lexicographic; a proper prefix is smaller (`00 < 0000`) | |
| `Char` | scalar value | |
| `Text` | scalar order, which equals the order of the UTF-8 bytes | **not** UTF-16 code-unit order: `U+FFFF < U+10000`, while as UTF-16 `D800 DC00 < FFFF` |
| `Option<K>` | `None < Some(x)`; `Some` by `ord` of the payload | |
| tuple, record | lexicographic over components or fields in declaration order | |
| variant | constructor **declaration index** first, then the fields lexicographically | reordering constructor declarations changes the canonical order, and so the corpus digests |

Not key types: `F32 F64` (VER04p: no equality), opaque types, `Result` (its error
cases are compared by name), `List<T>`, `Set<K>`, `Map<K, V>`. `List` and nested
collections as keys are deferred (section 20): they need the recursive machinery
of VER04s or a host story the first slice does not have. The `Text` order is the one VER04l 3.4 records so that a later slice cannot choose
differently. `ord` is consistent with
`==`: `cmp(a, b) = 0` iff `a == b`, and the canonical stream (8.2) is injective, so
two distinct values never have one encoding.

The built-in constraints (6.2) are defined on the same table: `Eq` is every type
with `==`, `Ord` is every key type, `Finite` is a key type with at most 16 values
(`Bool`, `Bytes<0>`, `Option<Bool>`, a tuple of `Bool`s, a variant of at most 16
payload-free constructors), `Bits` is the eight fixed-width integers
(`U8 U16 U32 U64 I8 I16 I32 I64`, the family `F` of VER04j 3.2).

### 3.3 One canonical order

**CM3.** Every list a collection produces is in **ascending `ord`, without
repeats**. That is the whole rule, and it is what makes the corpus deterministic:

* `set_elems(s)`, `map_keys(m)`, `map_values(m)` (in key order) and `map_entries(m)`
  are the only ways to turn a collection into a sequence, and each is canonical;
* a collection in a stream, a descriptor, a `corpus.json` value or a TFV row is in
  canonical order (8.2), and a decoder MUST reject any other order;
* a host collection is **converted to canonical form by the adapter** before it is
  compared or hashed: convert each key, **sort with the `warrant-corpus`
  comparator** for that key type, and check that no two neighbours are equal (a
  repeat is the failure `duplicate-element`). The host's own sort, comparator and
  iteration order are never used (9.3).

**CM3a.** (Binding the result.) The order class of a host collection is a fact
declared in the binding (9.2): `sorted` (the host guarantees ascending `ord`),
`insertion` (the host keeps construction order) or `unspecified`. The rules:

| Model result type | `sorted` | `insertion` | `unspecified` |
|---|---|---|---|
| `Set<K>`, `Map<K, V>` | accepted; the adapter still sorts | accepted; the adapter sorts | accepted; the adapter sorts and runs the order-independence obligation (10.1) |
| `List<K>`, `List<(K, V)>` (the host sequence is the answer) | accepted; the claim compares with the canonical list | accepted **only** if the claim's model computes the insertion order from the construction list (D7) | **refused: W0762** |

So `to_list` of a hash set binds as `Set<T>` (the adapter sorts the host `Vec` and
rejects a repeated element), and a function that really returns keys in
insertion order (Python `list(d)`) binds as `List<K>` with a model that says what
insertion order means. A claim over a host list never depends on a hash seed.

### 3.4 Built-ins

Every form is **total**: there is no error and no partial application. Names carry
the prefix because of D1.

| Form | Operands | Result | Semantics |
|---|---|---|---|
| `set_of(xs)` | `xs: List<K>` | `Set<K>` | the set of the elements; repeats collapse |
| `set_member(s, x)` | `Set<K>, K` | `Bool` | `x` is in `s` |
| `set_insert(s, x)` | `Set<K>, K` | `Set<K>` | `s` with `x` added |
| `set_remove(s, x)` | `Set<K>, K` | `Set<K>` | `s` without `x`; `s` itself when `x` is absent |
| `set_size(s)` | `Set<K>` | `Nat` | the number of elements |
| `set_union(a, b)`, `set_inter(a, b)`, `set_diff(a, b)` | `Set<K>, Set<K>` | `Set<K>` | the usual; `set_diff(a, b)` keeps the elements of `a` not in `b` |
| `set_subset(a, b)` | `Set<K>, Set<K>` | `Bool` | every element of `a` is in `b` |
| `set_elems(s)` | `Set<K>` | `List<K>` | the elements, ascending `ord` |
| `map_of(es)` | `es: List<(K, V)>` | `Map<K, V>` | the entries; for a repeated key the **last** entry wins |
| `map_of_strict(es)` | `List<(K, V)>` | `Option<Map<K, V>>` | `None` iff two entries share a key |
| `map_get(m, k)` | `Map<K, V>, K` | `Option<V>` | `Some(v)` iff `k` is a key |
| `map_has(m, k)` | `Map<K, V>, K` | `Bool` | `k` is a key |
| `map_put(m, k, v)` | `Map<K, V>, K, V` | `Map<K, V>` | `m` with `k` mapped to `v` (replacing) |
| `map_del(m, k)` | `Map<K, V>, K` | `Map<K, V>` | `m` without `k`; `m` itself when `k` is absent |
| `map_size(m)` | `Map<K, V>` | `Nat` | the number of keys |
| `map_keys(m)` | `Map<K, V>` | `List<K>` | the keys, ascending `ord` |
| `map_values(m)` | `Map<K, V>` | `List<V>` | the values, in ascending key order |
| `map_entries(m)` | `Map<K, V>` | `List<(K, V)>` | the entries, ascending key order |

`len` stays a sequence built-in: `len(s)` on a `Set` or `Map` is W0760 (use
`set_size` or `map_size`, so the unit is always written; the `len(t)` precedent of
VER04l decision b). There is no literal syntax: `set_of([])` and `map_of([])`
take their type from the expected type, as `[]` does in v0. `set_empty` and
`map_empty` are not added. `set_min`, `map_merge` and friends are open question
CQ-15.

**Worked example** (keys `U8`, hand-computed):

```text
s = set_of([200u8, 5u8, 200u8, 128u8])
set_elems(s)               == [5u8, 128u8, 200u8]        -- ascending, no repeat
set_size(s)                == 3
set_elems(set_remove(s, 7u8)) == [5u8, 128u8, 200u8]     -- absent: unchanged
m = map_of([(2u8, 9u8), (1u8, 8u8), (2u8, 7u8)])
map_entries(m)             == [(1u8, 8u8), (2u8, 7u8)]   -- last entry for key 2 wins
map_get(m, 3u8)           == None
```

### 3.5 Equality, `same` and floats

`==` on `Set<K>` is defined for every key type (all have `==`). `==` on
`Map<K, V>` is defined iff it is defined on `V`; a `V` that contains a float is
W0301 for `==`, as VER04p 4.3 says for every type containing a float, and the
relation `same` of VER04p **lifts through the values of a map** in key order
(keys are never floats). Equality of two collections never compares allocation
identity or a host representation (VER04r 4.2 rule 5).

### 3.6 Size limits

**CM4.** (Limits, all checked before allocation, in checked arithmetic.)

| Limit | Value | Where it comes from |
|---|---|---|
| entries in a collection in a corpus value, a stream, a result | `MAX_COLLECTION = 4,096` | the list cap of VER04r 2.3 |
| entries in a collection an evaluator produces mid-expression | `MAX_IR_VALUE = 65,536` | VER04r 4.2 rule 6, extended to collections |
| leaves of any one value (keys, values and every nested component) | `MAX_VALUE_LEAVES = 65,536` | the meter of VER04k 2.7 |
| nesting of `Set`/`Map` in one type (`Map<K, Set<V>>` is 2) | 3 (W0760 above) | CT2 |
| entries in a value generated by a domain | 16 (the `subset_of` universe) | 5 |
| work for one built-in | `1 + (sizes of the collection operands) + (size of the result)` units, charged as if every result were copied | VER04k 2.7 ("charges as if it copied") |

The work rule is deliberately simple and implementation independent: a model that
uses a balanced tree and one that uses a sorted array reach exhaustion at the
same instruction. C1-S0 checks the constants against the meter text of VER04k 2.7.

### 3.7 What a collection is not

* **Not a handle.** A `Set<K>` that a host library hands out as a mutable object
  with identity is an opaque type with a `Set<K>` observer (section 6.9, 15), or a
  VER04m object. The value type `Set<K>` is the *observation*.
* **Not a priority queue or heap.** A heap is a stateful object whose model may
  use a sorted list (VER04m).
* **Not a multiset.** A bag is a `Map<K, Nat>`.

### 3.8 Room for recursive types (VER04s)

The owner lifted recursive types and put them in their own specification. This
document therefore does two things only: it **forbids** recursion cleanly, and it
**leaves** the places a later spec will need open. The forbidding is one rule,
`CM5`: the acyclicity check of VER04j 4.2 ("the relation 'type mentions type' is
acyclic through tuples, records, variants, `List`, `Option` and `Result`") MUST
also traverse `Set` and `Map`, with the same diagnostic (W0203), so
`variant Json = Obj(Map<Text, Json>) | ...` is refused today by the rule that
VER04s will relax. Section 19 lists what VER04s will need.

## 4. Claims over collections and their elements

### 4.1 Three forms

A claim speaks about collections in three ways. Each has a different strength and
a different cost, so the text states which is which.

| Form | Example | Quantifies over | Strength |
|---|---|---|---|
| **Q1 binder over collection values** | `forall s: Set<Bool>, x: Bool. set_member(set_insert(s, x), x)` | the values of the binder's domain (section 5) | the label of the weakest binder |
| **Q2 elements of a given value** | `all(set_elems(s), ok)`; `count(map_entries(m), p)`; `fold(...)` | exactly every element of that value, whatever its size (a WX-D form over the canonical list, which is total) | no domain is involved: `all` over a value is exact; the strength still comes from the binders that produced the value |
| **Q3 whole-value equality** | `set_of(to_list(add(h, x))) == set_insert(model_set(h), x)` | every key at once, because `==` is extensional | the label of the binders |

**CQ1.** (No new binder form.) There is no `forall x in s` binder. A refinement
`set_member(s, x)` on a binder `x: U8` is allowed, filters the product, and is charged the
**whole** product `|D(s)| * |D(x)|` against `B` (VER04h 3). Authors should prefer
Q2: `all(set_elems(s), p)` visits only the elements. A dependent binder over the
elements of an earlier binder, with an exact case count in the manner of the
`split(m)` domain of VER04k 2.9, is open question CQ-2.

**CQ2.** (Prefer Q3 to a per-key sweep.) The claim
`forall s, k: K. set_member(subject(s), k) == set_member(model(s), k)` quantifies
over the **domain of `K`**, so a spurious extra key outside that domain is
invisible to it. `subject(s) == model(s)` compares the whole value, including
every key the domain does not enumerate. When a claim is written as a per-key
sweep and a whole-value form exists, the report prints the advisory
`per-key sweep: extra keys outside D(K) are not seen` as an open exemption (no
code is allocated; the form is legal).

### 4.2 Worked example

```text
op insert_into(s: Set<U8>, x: U8) -> Set<U8>                      -- a subject operation
claim insert_contains:
  forall s: Set<U8> | subset_of(s, [0u8, 127u8, 128u8, 255u8]), x: U8.
    set_member(insert_into(s, x), x)
claim insert_exact:
  forall s: Set<U8> | subset_of(s, [0u8, 127u8, 128u8, 255u8]), x: U8.
    insert_into(s, x) == set_insert(s, x)
```

`subset_of` over 4 members gives 2^4 = 16 sets; `x: U8` is 256 values: 16 * 256 =
4,096 cases, label `bounded-set` (the universe lists 4 of 256 `U8` values). The
second claim is a **reference** claim in the sense of VER04r 4.3 (one subject
call compared with a pure expected value), so the oracle supplies independent
expected values; the first is a **predicate** claim (`relational`) and the report
says so. Both exercise the order-witness pair `127, 128`, so a host that sorts
bytes as signed values fails the canonicalisation of `insert_into`'s result.

### 4.3 The element vacuity guard

**CQ3.** `all(xs, p)` over an empty sequence is true, and a claim whose every
collection is empty proves nothing. For every claim that applies `all`, `any`,
`count` or `fold` to a collection expression the generator computes, from the
model alone, **`elements_visited`**: the total number of element evaluations over
the suite's admitted cases (an `all` that short-circuits counts the elements it
reached). Zero is **W0601** (a claim that cannot fail), a value below 1 per case
on average is printed, and the number is committed in the corpus and reproduced
by every lane (it does not depend on the subject). The report also prints, per
collection binder, `nonempty` and `size>=2` case counts, so "the sets were all
empty" cannot hide behind a large case count.

## 5. Domains and labels

### 5.1 The order-witness alphabet

**CD1.** For every key type `T` there is a short, fixed list `witness<T>()`: the
values whose **pairs** make a wrong comparator or a lossy host key visible. It is
a ground built-in (no free binder), so it can be the alphabet of `within`
(VER04l 4.3) and the universe of `subset_of` and `maps_over`. It is not
ascending; the order below is the order of the list.

| Key type | `witness<T>()` | What a pair catches |
|---|---|---|
| `Bool` | `false, true` | |
| unsigned `w`-bit, `Nat` | `0, 2^(w-1)-1, 2^(w-1), MAX` (for `Nat`, `w = 64`) | signed compare of an unsigned key: `127` vs `128`, `2^63-1` vs `2^63` |
| signed `w`-bit | `MIN, -1, 1, MAX` | unsigned compare of a signed key (`-1` sorts after `1`) |
| `Char` | `U+7F, U+80, U+FFFF, U+10000` | signed `char`; UTF-16 code-unit order |
| `Bytes` | `b"", 00, 0000, 7f, 80, 00ff, 0100` | prefix order, signed byte, carry or endianness mistakes |
| `Bytes<N>` | `N = 0`: `b""`. `N = 1`: `00, 7f, 80, ff`. `N >= 2`: the zero string, the strings that are zero but for a last byte `7f`, `80` or `ff`, and the carry string `0...0100` (last two bytes `01 00`): 5 members | the same |
| `Text` | `t"\u{FFFF}", t"\u{10000}", t"\u{E9}", t"e\u{301}", t"", t"\0", t"a", t"ab", t"\u{7F}", t"\u{80}"` | UTF-16 order; canonical-equivalence collapse (Swift `String ==`); NUL and prefix order; signed `char` |
| `Option<K>` | `None`, then `Some(w)` for the first two members `w` of `witness<K>()` | `None` placement |
| tuple, record | the lexicographic grid of the first two members of each component's list (at most 2^n elements for `n` components, at most 8 elements in all) | a last-component-first comparator |
| variant | for each constructor in declaration order, the value built from the first member of each field's list (at most 8 constructors, then the first 8) | declaration order against name order |

`witness<T>()` is **checked** as a total function of `T` (CK6): a key type with no
row is W0761. `all_values<T>()` for a `Finite` type is the complete list of its
values in `ord`.

### 5.2 `set_d1`: the default domain of a `Set` binder

**CD2.** A `Set<K>` binder with no refinement has the domain `set_d1`. Let `D` be
the ordinary domain of `K` (VER04h 3, VER04l 6) and `U` be `D` followed by those
members of `witness<K>()` not already in `D` (so `U` contains the witnesses),
`n' = |U|` and `w = |witness<K>()|`.

* If `n' <= 4`, `set_d1` is **all `2^n'` subsets of `U`**, ordered by ascending
  bitmask (bit `i` is the membership of `U[i]`, so mask 1 is `{U[0]}`, mask 2 is
  `{U[1]}`, mask 3 is `{U[0], U[1]}`).
* Otherwise `set_d1` is: the empty set, then the singleton `{u}` for each `u` in
  `U` in order, then the pair `{witness[i], witness[j]}` for `i < j` in
  lexicographic index order. Its size is `1 + n' + C(w, 2)`.

| `K` | `n'` | `w` | `set_d1` size | Label |
|---|---|---|---|---|
| `Bool` | 2 | 2 | 4 (all subsets) | **`full`** |
| `U8`, `I8` | 256 | 4 | `1 + 256 + 6 = 263` | `bounded-set` |
| `Bytes` | 347 (VER04h 3; every witness is already in it) | 7 | `1 + 347 + 21 = 369` | `bounded-set` |
| `Option<Bool>` | 3 | 3 | 8 (all subsets) | `full` |

Why not the memo's "all subsets of the first four members" (D2): for `U8` that
would be subsets of `{0, 1, 2, 3}`, which no signedness bug can see. The pairs of
`witness` make every known wrong comparator fail a case in the default domain
without the author having to know it.

**CD3.** The label is `full` exactly when `n' <= 4` **and** `D` is the whole type
(so `Bool`, `Option<Bool>`, a variant with at most four values); otherwise
`bounded-set`. It is never `sampled` for a generated domain; a property or fuzz
suite draws sets of 0 to 16 members from `U` and is `sampled`, as VER04h says of
every random suite.

### 5.3 `subset_of` and `maps_over`

**CD4.** (`subset_of`.) A binder `s: Set<K>` whose refinement has a top-level
conjunct `subset_of(s, A)`, with `A` a ground list of distinct members of `K` and
`|A| <= 16`, has the domain **all `2^|A|` subsets of `A`**, ordered by ascending
bitmask over the positional order of `A` (as `within` orders by the positional
order of its alphabet, VER04l 4.3). A duplicate in `A`, a member not of type `K`,
or `|A| > 16` is **W0761**. The label is **`full` iff `A` lists every value of
`K`** (so `|K| <= 16`: `Set<Bool>`, a four-constructor variant; the checker
computes `|K|` from the type), else `bounded-set`.

**CD5.** (`maps_over`.) A binder `m: Map<K, V>` with a conjunct
`maps_over(m, KA, VA)`, ground distinct lists, `|KA| <= 16`, has the domain of
**all partial functions from `KA` to `VA`**: `(|VA| + 1)^|KA|` maps. Ordinal `p`
is written in base `|VA| + 1` with `|KA|` digits, **`KA[0]` the most significant**
(the last key varies fastest, as in the product order of VER04r 5.1); digit 0 means
"key absent", digit `d >= 1` maps the key to `VA[d - 1]`. The size is computed in
checked arithmetic and counted against `B` before anything is built (W0706 over
budget). The label is `full` iff `KA` lists all of `K` and `VA` all of `V`.

Worked example. `m: Map<Bool, Bool>` with `maps_over(m, [false, true], [false,
true])` has `(2 + 1)^2 = 9` maps, all the partial functions from `Bool` to
`Bool`, label `full`. In ordinal order:

| p (base 3) | map | p | map |
|---|---|---|---|
| 0 (00) | `{}` | 5 (12) | `{false: false, true: true}` |
| 1 (01) | `{true: false}` | 6 (20) | `{false: true}` |
| 2 (02) | `{true: true}` | 7 (21) | `{false: true, true: false}` |
| 3 (10) | `{false: false}` | 8 (22) | `{false: true, true: true}` |
| 4 (11) | `{false: false, true: false}` | | |

### 5.4 `map_d1`: the default domain of a `Map` binder

**CD6.** A `Map<K, V>` binder with no refinement has the domain
`maps_over(KA, VA)` where `KA` is the first `min(4, w)` members of
`witness<K>()` and `VA` the first `min(2, |D(V)|)` members of `D(V)` (VER04h 3).
The two values let a map exchange two values (the `swap-values` control, 11.1) and
the four keys put the order-witness pairs in every map. For `Map<U8, U8>`:
`KA = [0, 127, 128, 255]`, `VA = [0, 1]`, size `3^4 = 81`, label `bounded-set`;
for `Map<Bool, Bool>`: `3^2 = 9`, `full`.

### 5.5 Lists of pairs: `within` with tuple alphabets

A claim about `map_of` over a list with a **repeated key** needs a list domain in
which keys repeat with different values. The default domain of `List<(U8, U8)>`
(VER04j 4.7) takes the first 16 tuples, whose first component is always `0`, so it
has repeats but only for one key. The refinement
`within(es, [(0u8, 0u8), (0u8, 1u8), (1u8, 0u8)], 3)` (VER04l 4.2, a list
alphabet) has `1 + 3 + 9 + 27 = 40` lists, all of which have repeated keys in many
cases; label `bounded-set`. The check of `first-wins` (11.1) needs at least one
admitted case with a repeated key and different values; the generator counts them
(`dup_key_cases`).

### 5.6 `one_of`: one element from a list

**CD7.** A scalar or composite binder `x: T` (or a maker parameter, 18.1) whose
refinement has a top-level conjunct `one_of(x, A)`, with `A` a ground list of
distinct members of `T` and `|A| <= 16`, has the domain `A` in its **positional
order**; a duplicate, a member of the wrong type or a longer list is W0761. The label
is `full` iff `A` lists every value of `T` (so `T` has at most 16 values), else
`bounded-set`. It is the scalar counterpart of `within` (which restricts sequences)
and is needed wherever a claim wants one element of the order-witness alphabet:
`x: T | one_of(x, witness<T>())` has `|witness<T>()|` values (4 for `U8`) instead of
256.

### 5.7 Budget

The product of binder domains is counted before refinement and before
enumeration (VER04h 3). A collection binder contributes its domain size, a
`subset_of` binder `2^|A|`, a `maps_over` binder `(|VA|+1)^|KA|`. Over
`B = 400,000` is **W0706**. Descriptors (8.3) record `size`, so a lane never
re-runs the demotion.

## 6. Generics: the model

### 6.1 Rank-1 type parameters

**GM1.** A declaration may carry **type parameters** next to the existing static
`Nat` parameters of VER04 §5: `op`, `model`, `record`, `variant`, `opaque` and
`claim`. Parameters are rank 1 (no parameter of a parameter, no higher-kinded
type), and **type arguments are always written**: `elements<U8>(s)`, never
`elements(s)`. There is no inference, so a reader (and a lane) always sees which
instance a call is about.

```text
op add_all<T: Ord, N: Nat>(xs: List<T>, tag: Bytes<N>) -> Set<T>
```

One parameter list holds both kinds: `Nat` for a width (`N`), a constraint list for
a type (`T`). This is the **single static-parameter mechanism**; widths are not a
second system (6.8).

### 6.2 Constraints: a closed set of four

**GM2.** A type parameter carries one or more of the constraints `Eq`, `Ord`,
`Finite`, `Bits`, joined by `+`. They are built in, not user-extensible (section
20). Each is defined by the table of 3.2 and licenses exactly these operations
inside a generic body:

| Constraint | Instances | Licenses |
|---|---|---|
| `Eq` | every type with `==` | `==`, `!=` on `T` |
| `Ord` | every key type | `Eq`, and `Set<T>`, `Map<T, V>`, the `set_` and `map_` built-ins, `witness<T>()` |
| `Finite` | key types with at most 16 values | `Ord`, and `all_values<T>()` |
| `Bits` | the eight fixed-width integers | `Eq`, and the WX-B and WX-C built-ins of VER04j 2 and 3 that take and return `T` (`bit_and`, `shl`, `rotl`, `popcount`, ...) |

A type parameter with **no** constraint is legal and licenses nothing but being
passed along (a record field, a list element): the body cannot compare it.

A constraint is on the **model**. `HashSet<T: Ord>` in a document does not say the
library sorts its elements or needs `Ord`; it says the document states
`elements(s)` as a `Set<T>`, which needs a canonical order, so every instance must
be a Warrant key type. The Rust `HashSet<T: Eq + Debug>` needs only equality and
a hash (D11).

### 6.3 A schema has no evidence; an instance does

**GM3.** A generic declaration is a **schema**. It is type-checked **once**,
abstractly, with every type parameter known only through its constraints. A claim
is evidenced only at the **instances** an `instantiate` declaration lists
(section 7). An instance is the schema with every parameter replaced by a ground
type or number; nothing else about the schema is tested.

```text
claim add_inserts<T: Ord>:   -- the schema: no suite exists for it
instantiate add_inserts:
  T = U8                     -- three suites exist, one per row
  T = Bytes
  T = (I8, Bool)
```

### 6.4 Monomorphisation

**GM4.** The compiler **monomorphises before it emits any IR**. For each instance
row it substitutes the ground types, re-checks the result as an ordinary
monomorphic claim (an instance that fails a constraint is **W0764**), resolves
domains, enumerates and emits one suite. **No corpus value, descriptor, stream
context, IR tag or lane evaluator ever contains a type variable.** Each instance
has its own suite id, case ids, chunk files, digests and kind (8.5). This is what
makes the polyglot rule hold without effort: the eighteen lanes implement
`Set<U8>` and `Set<Text>`, never "for all `T`". It also fixes the cost model: the
work of a schema is the **sum** over its instances, charged to the run totals of
VER04r 14 at generation (W0706).

### 6.5 Instance coverage and the minimum set

**GM5.** For every type parameter that carries `Eq`, `Ord` or `Finite` and is not
covered by `all-instances` (6.6), the rows of the schema's `instantiate` block
MUST contain, in that parameter's column:

1. at least two distinct types, and
2. at least one **scalar** (`Bool`, an integer type, `Char`) and at least one
   **by-reference-hazard** type: `Bytes`, `Bytes<N>` with `N >= 1`, `Text`, or a
   tuple, record, variant, `Option`, `Set` or `Map` that contains one.

Fewer is **W0764**. The second item is there because identity equality passes on
every scalar and fails on the first value that has a heap representation:
`byte[]` in Java, `Uint8Array` in JavaScript, `[]byte` in Go, `Vec<u8>` behind a
pointer comparison, a Python `str` interned at one address. A schema evidenced
only at `U8` and `Bool` would never see the identity bug the generic container
exists to have.

A parameter constrained by `Bits` alone is exempt from item 2 (its universe has
no heap-represented member) and needs at least two rows or `T = *`.

**GM5a.** (Rows are explicit.) A row binds every parameter of the schema. There
is no implicit product of columns: two parameters with three candidate types each
need as many rows as the author wants tested (at most 64 per schema, 256 per
document; W0764 above). A type argument may mention a `Nat` parameter of the same
row (`T = Bytes<N>, N = 4`); substitution is simultaneous and the printed,
hashed, resolved row is ground (`T=Bytes<4>,N=4`).

### 6.6 Labels: `instances(n)` and `all-instances(n/n)`

**GM6.** The schema's printed form is its listed instances, never "for all":

```text
add_inserts<T: Ord>   instances(3): U8, Bytes, (I8,Bool)
```

Its **kind** is the weakest kind among its instances' suites **capped at
`bounded-set`**: a schema is never `full` through `instances(...)`, even when
every listed instance is `full`. Its **VER29 rung** is at most
`exhaustive-sampled`, never `exhaustive`. A suite with kind `sampled` stays
`sampled`.

**GM6a.** (The one exception.) A schema is **`all-instances(n/n)`** when every
type parameter is constrained to a **closed universe** and the `instantiate` block
lists **every member of the product of the universes**. A universe is closed iff
the language itself fixes and bounds it. In this document exactly one is:

| Constraint | Closed universe | Members |
|---|---|---|
| `Bits` | the family `F` | `U8 U16 U32 U64 I8 I16 I32 I64` (8) |

(`Finite` is not closed: tuples nest, so the family of finite types is unbounded.
A `Nat` parameter's range `0..=4096` is closed but its 4,097 members exceed the
64-row cap, so a `Nat` parameter never makes a schema `all-instances`.) For
`T: Bits` the schema prints `all-instances(8/8)` and takes the **weakest instance
kind uncapped**, so a claim whose every instance is `full` is `full`, and its
rung may be `exhaustive`. The shorthand `T = *` (7.1) writes every member, so none can
be forgotten; a block that lists fewer is `instances(k)`, and a block that claims
`all-instances` with a member missing is W0764.

### 6.7 Why parametricity is not evidence

A function that is polymorphic in `T` cannot, in a language with parametricity,
look at a `T`; so one might argue that one instance suffices. That argument is
**not used**, for three reasons, each of which is a fact about the lanes:

* only Rust, Haskell and a few others forbid a generic function from inspecting
  its argument; Java, C#, TypeScript, Python and Go have `instanceof`, `is`,
  `typeof`, `isinstance` and type switches, so a generic container may branch on
  the element type;
* C++ has explicit and partial specialisation (and Rust has an unstable one), so a
  subject can be wrong at exactly one `T`;
* the subject may be generic in the source but not in behaviour: a hash set whose
  hash is `format!("{:?}", key)` (the Rust `hash-map` does exactly that) is
  correct or incorrect per `Debug` implementation, which is per type.

So the evidence is what ran, at the instances that ran, in the lanes that ran
them (9.6).

### 6.8 Interaction with `Bytes<N>` const parameters

The language already has one static parameter kind: `op eq_fixed<N: Nat>(a:
Bytes<N>, b: Bytes<N>)` (VER04 §5), bound in a lane by `width_params` and a
`width: runtime | static` choice (VER04i 5), enumerated from
`corpus.json.widths`. This document does not add a second mechanism:

1. **One list, one mechanism.** `N: Nat` and `T: Ord` live in one static-parameter
   list, and an `instantiate` row binds both (`T = Bytes<N>, N = 4`). The width
   list `widths` of an operation that has only `Nat` parameters is **sugar for
   `instantiate` rows in the checker and the report**: the diagnostics
   (W0764: duplicate, over the cap), the budget accounting and the `instances(...)`
   label treat it as one. Its **encoding does not change**: `ct-compare` keeps
   `widths`, its suite ids and its nine v1 digests byte for byte (D10, 8.5). The new
   encoding applies as soon as any type parameter is present.
2. **`Bytes<N>` as a type argument.** `T = Bytes<4>` is a legal row and a key type
   (`ord` is the `Bytes` order; every key of `Bytes<4>` has length 4). In a
   signature `Map<Bytes<N>, V>` the two occurrences of `N` are one parameter, so
   `map_put(m: Map<Bytes<N>, V>, k: Bytes<N>, v: V)` cannot be applied to a key of
   another width: the type checker, not the lane, rejects it. At the lane boundary
   a key of the wrong length from a host result is the existing
   `type-mismatch` (VER04r 7.3: a wrong-length result is a failure, never padded).
3. **Labels follow the width.** The domain of a `Bytes<N>` binder is `full` for
   `N <= 1` and `sampled` for `N >= 2` (VER04r 9.3, following VER04j 14.4); an
   instance `T = Bytes<4>` is therefore a `sampled` suite and the schema's kind is
   at most `sampled`. Nothing here raises it.
4. **`Finite` and `Bits` interplay.** `Bytes<0>` (1 value) and `Bytes<1>` (256 values)
   are key types; only `Bytes<0>` is `Finite` (at most 16 values). `Bytes<N>` is
   never in `Bits`.
5. **The static lanes' dispatch table** (one arm per width in VER04i 5) becomes a
   table of rows: for `op put<K: Ord, N: Nat>` the table has one arm per
   `(K, N)` row, and a lane with no row for a pair has an absent cell (9.6), not a
   fallback arm.
6. **No arithmetic in types.** `Bytes<N + 1>` is not accepted: a static parameter
   is a name, as in VER04 §5. The checker refuses an expression in a width
   position (W0763); no row can express `Bytes<N>` against `Bytes<2 * N>`.
7. **Cap.** `N <= MAX_FIXED_WIDTH = 4096` (VER04h 3, W0707 above) per row; the
   64-row cap bounds the number of `(T, N)` pairs.

### 6.9 Generic opaque types (an amendment to VER04n)

VER04n 4.5 makes a type parameter on an opaque type W0750 "until VER04o amends
this", and its section 18 defers generic opaque types. This document supplies the
amendment, which is also what the pilot needs (`HashSet<T>`).

**GM7.** An `opaque` declaration may carry type parameters
(`opaque HashSet<T: Ord>`). Makers and observers carry the same parameters. After
monomorphisation (GM4) **each instance is a separate, ordinary opaque type**
(`HashSet[T=U8]`) with its own makers, observers, recipes, handle domain,
observation classes and the whole of the five VER04n obligations (adequacy,
immutability, unforgeability, freshness, grounding). VER04n's machinery is
unchanged; only the declaration site is generic. Consequences, each stated so no
lane has to guess:

* **Domains and counts are per instance** (`handles(via from_list: 21 of 21
  admitted)` for `U8`); labels follow VER04n OB1 (a handle binder is never `full`).
* **Unforgeability** (VER04n OL2) is probed **once per distinct host type**: once
  for an erased or dynamic lane (one class), once per instance for a static lane
  (distinct host types). The report prints the instance probed.
* **Elements are never handles.** `T` must be a key type or unconstrained and not
  opaque (a key type cannot be opaque, 3.2); a handle inside a `Set`, `Map` or
  `List` stays out of scope (VER04n OQ-12 is **not** reopened by this document).
* **Foreign generic handles** (`from contract...`) are out of scope: a VER29
  contract id has no instance segment, so a generic type cannot be cited
  (section 18, section 20).

## 7. Surface syntax and type rules

### 7.1 Grammar

```text
type             += "Set" "<" type ">" | "Map" "<" type "," type ">"
                  | NAME "<" type { "," type } ">"      -- an instance of a generic record, variant or opaque type
static_parameter  = NAME ":" ( "Nat" | constraint { "+" constraint } ) ;   -- replaces VER04 section 5's NAME ":" "Nat"
constraint        = "Eq" | "Ord" | "Finite" | "Bits" ;
operation         = "op" NAME [ "<" list(static_parameter) ">" ] "(" ... ;  -- unchanged shape
model_fn          = "model" NAME [ "<" list(static_parameter) ">" ] "(" ... ;
claim             = "claim" NAME [ "<" list(static_parameter) ">" ] ":" ... ;
record_decl       = "record" NAME [ "<" list(static_parameter) ">" ] "(" ... ;
variant_decl      = "variant" NAME [ "<" list(static_parameter) ">" ] "=" ... ;
opaque_decl       = "opaque" NAME [ "<" list(static_parameter) ">" ] ... ;   -- VER04n 4.1, amended by 6.9
instantiate_decl  = "instantiate" NAME ":" NEWLINE INDENT instance_row { instance_row } DEDENT ;
instance_row      = binding { "," binding } NEWLINE ;
binding           = NAME "=" ( type | NAT | "*" ) ;      -- "*" only for a Bits parameter: every member
call              += NAME "<" type_arg { "," type_arg } ">" [ "(" args ")" ]   -- explicit type arguments
```

`Set` and `Map` are type names, not keywords; the new words are the reserved names
of CM0a. A type argument list uses the same `<` `>` of `Bytes<N>` and `Result<T,
E>`. The call form extends the v0 `reference = qualified_name [ "<" widths ">" ] [
"(" args ")" ]` of VER04 §5: the angle part now takes types as well as widths, and
a call of a generic operation MUST write all of them. An `instantiate` declaration
follows the schema it names (a forward reference is a name-resolution error) and
there is at most one per schema (a second is W0764).

### 7.2 A small complete example (illustrative)

```text
warrant tagged_map v2

record Pair<A: Eq, B: Eq>(first: A, second: B)

model swap<A: Eq, B: Eq>(p: Pair<A, B>) -> Pair<B, A> = Pair<B, A>(p.second, p.first)

op put_all<K: Ord, V: Eq>(m: Map<K, V>, es: List<(K, V)>) -> Map<K, V>

claim put_all_is_last_wins<K: Ord, V: Eq>:
  forall m: Map<K, V>, es: List<(K, V)> | within(es, pairs<K, V>(), 2).
    put_all<K, V>(m, es) == map_of(concat(map_entries(m), es))

instantiate put_all_is_last_wins:
  K = U8, V = Bool
  K = Bytes, V = U8
  K = Text, V = Bool
```

(`pairs<K, V>()` stands for a ground model that builds the tuples of two
witnesses; a real document writes it as a `model`. `concat` of two `List<(K, V)>`
is the existing sequence built-in.) Three suites exist, one per row. Reading the
claim: applying the entries of `es` in order to `m` is the same as building a map
from `m`'s entries followed by `es`, because `map_of` lets a later entry win. That
is the specification of a subject `put_all`.

### 7.3 Checker rules

**CK1.** (Key types, W0760.) The first argument of `Map`, and the argument of
`Set`, MUST be a key type (3.2): a float, an opaque type, a `Result`, a `List`, a
`Set` or a `Map` there is W0760. A type parameter in key position MUST carry
`Ord` (or `Finite`), else W0760.

**CK2.** (Nesting and recursion.) At most three `Set`/`Map` levels in one type
(W0760). The acyclicity rule of VER04j 4.2 traverses `Set` and `Map` (CM5, W0203):
a collection never closes a cycle in the type-mentions relation.

**CK3.** (Collection typing, W0301 or W0760.) The built-ins have exactly the
signatures of 3.4; an argument of the wrong type is W0301; `len`, `<`, `<=` or any
ordering on a `Set` or `Map` is W0760; `==` and `!=` follow 3.5.

**CK4.** (Explicit arguments, W0763.) A call of a generic operation or model, a
use of a generic nominal type, and a call of `witness`, `all_values` or a generic
constructor MUST write every type argument. A missing or surplus argument, a
`Nat` where a type is expected (or the reverse), and an expression in a width
position (6.8 item 6) are W0763.

**CK5.** (The body is checked under the constraints, W0763.) Inside a generic
declaration a value of type parameter `T` supports exactly the operations its
constraints license (6.2). A `==` on an unconstrained `T`, a `set_insert` on a
`Set<T>` with `T: Eq` only, or a literal of type `T` is W0763 (a generic body has
no literal of an unknown type: constants are passed as parameters or come from
`witness<T>()`).

**CK6.** (`witness` and `all_values`.) `witness<T>()` requires `T: Ord` and a row
in 5.1 (a key type always has one: W0761 if a later slice adds a key type without
its row). `all_values<T>()` requires `T: Finite` (W0763 otherwise). Both are
ground (no free binder), so both may appear as the alphabet of `within` and the
universe of `subset_of` and `maps_over`.

**CK7.** (`instantiate`, W0764.) It names a schema declared earlier; each row
binds every parameter of that schema exactly once (a parameter the schema lacks,
or a missing one, is W0764); each bound type satisfies its constraints (an
instance that violates `Ord` is W0764, with the constraint named); rows are
distinct after resolution (6.5 `GM5a`); the minimum set of GM5 holds; at most 64
rows per schema and 256 per document. A `*` is accepted only for a `Bits`
parameter and expands to its eight members.

**CK8.** (Schema names.) A generic **claim** name is at most 52 characters, so the
mangled claim field `<name>__i<8 hex>` (8.5) fits the 64-character limit of VER04r
5.5 (W0763 above).

**CK9.** (Secrets, W0401.) A `labels` row that classes a `Set` or `Map` parameter,
or its keys, as `secret` is refused. Collection operations are variable-time
by design (lookup, sort, compare); a secret-dependent branch or access pattern
is outside what this feature can say, and the constant-time machinery of VER20 does
not cover it. The same refusal as VER04p uses for floats.

**CK10.** (No stateful collections.) A `Set` or `Map` as an `inout` parameter is
not in the first slice (W0760); VER04m's explicit state may hold one only through
a codec that VER04m defines.

**CK11.** (Opaque and collections.) An opaque type MUST NOT appear in a `Set`, a
`Map` or as a key or value (VER04n OK2, unchanged; the key rule CK1 would refuse
it anyway for a key).

**CK12.** (No recursion through instantiation, W0203.) A generic record, variant or
opaque type that mentions itself, at any instance, is refused with the diagnostic of
CM5. `Nested<T>` mentioning `Nested<List<T>>` (polymorphic recursion) would make the
set of instances infinite, and monomorphisation (GM4) terminates only because it is
finite.

## 8. Corpus and encoding (`warrant-corpus-v2`)

Collections and generics are encoded in the `warrant-corpus-v2` profile of the
draft VER04r, never in the v1 corpus: a v1 reader rejects the profile (VER04r
1.1), a suite with a `Set` or `Map` binder is outside the v1 type set and so uses
the `warrant-stream-v2` header (VER04r 1.2). Because VER04r is itself a draft,
section 18 lists what it must gain; C1-S0 reconciles the two.

### 8.1 Canonical type names

The grammar of VER04r 2.1 gains `"Set<" type ">"` and `"Map<" type "," type ">"`,
written exactly so (no space after the comma, as in `Result<Bytes,E>`):
`Set<U8>`, `Map<Text,Set<U8>>`. A **generic nominal type** never appears with
arguments in the corpus: each instance is a declared type with a mangled name
`<Name>__i<8 hex>` (8.5) in `types`, whose row carries two optional fields,
`"instance_of": "Pair"` and `"args": ["U8","Bool"]`, so a reader can print it.

### 8.2 Streams

**CC1.** (Tags.) A `Set<K>` is tag `70`, a u32 little-endian **count**, then the
elements in ascending `ord`, each in the ordinary encoding of `K`. A `Map<K, V>` is
tag `71`, a u32 little-endian count, then for each entry **in ascending key order**
the key and then the value. The count makes each value self-delimiting
(VER04r 2.4). The type name in the context prefix is the canonical name of 8.1.
**No host representation ever enters a stream**: the corpus is language independent
and a host's iteration order is irrelevant to every digest.

Worked example, `Set<U8>` holding `{5, 128}`:

```text
70 02000000 10 0500000000000000 10 8000000000000000
```

and `Map<Bool, U8>` holding `{false: 7, true: 0}`:

```text
71 02000000 00 10 0700000000000000 01 10 0000000000000000
```

**CC2.** (Decoder rules; corpus error `collection-order`.) A decoder MUST reject,
as a corpus error and never repair: an element or key that is not **strictly**
greater than the previous one under `ord` (this rejects both an unsorted
collection and a duplicate); a count above `MAX_COLLECTION = 4,096`; a count whose
elements cannot fit in the bytes that remain (**check before allocating**; a decoder
MUST NOT size a buffer from the count beyond `min(count, 4,096)`, CT3). The
decoder re-encodes what it decoded and compares the bytes, as VER04l 7.2 does.

**CC3.** (Tags, checked.) `70` and `71` are free. Checked on 2026-11-16 against
every specification that allocates stream tags, merged or draft (16.1). `72` to
`79` are reserved for follow-ups of this document (an unreserved use by another
specification is a conflict to settle at that merge).

### 8.3 JSON, TFV and descriptors

**CC4.** (JSON.) In `corpus.json` a `Set<K>` is the JSON array of its elements, and
a `Map<K, V>` the array of `[key, value]` pairs, **both in strictly ascending
key order** (JSON objects are not used: their keys are strings and their order is
a tool's choice). Elements use the JSON form of their type (VER04r 2.3: integers
are decimal strings). In `cases.tfv` a `Set` is an `L` of its elements and a `Map`
an `L` of two-element `L` rows (TFV `M` keys are strings, TF01 section 2). The
strict-order check applies to both; the order is part of the corpus.

**CC5.** (Descriptors.) The resolved descriptor tree of VER04r 5.1 gains four
nodes, each with `size()` and `nth(i)`, so chunk jumping works and a lane never
evaluates a refinement to enumerate:

| Node | JSON | `nth(i)` |
|---|---|---|
| `subsets` | `{"d":"subsets","t":K,"universe":[v0,v1,...],"size":2^n}` | the set of `universe[j]` for each set bit `j` of `i` |
| `maps_over` | `{"d":"maps_over","kt":K,"vt":V,"keys":[...],"vals":[...],"size":(m+1)^n}` | digits of `i` in base `m+1`, most significant first, one per key; 0 is absent, `d` maps to `vals[d-1]` |
| `set_d1` | `{"d":"set_d1","t":K,"u":[...],"w":[...],"size":S}` | `i = 0`: empty; `1 <= i <= n'`: `{u[i-1]}`; then the pairs of `w` in index order (the `n' <= 4` case resolves to `subsets` at generation) |
| `list_over` | VER04l 4.3, with a tuple alphabet | unchanged |

`map_d1` and every `witness<T>()` or `all_values<T>()` call **resolve at
generation** to a `maps_over` or `subsets` node with literal lists: the committed
corpus carries the values, and the independent oracle (VER04r 10) recomputes them
from 5.1 and fails the corpus if they differ. The universe lists are at most 16
members (a descriptor of larger size is a corpus error).

### 8.4 The IR level `ir.coll`

**CC6.** Collections add one feature level (VER04r 4.1). A suite, op or fault that
uses it lists `requires: ["ir.core", "ir.data", "ir.coll"]` (and `ir.seq` when it
applies a WX-D form, CM0c); a lane that does not declare the level records the
suite `not-live`, never skipped.

| Tags | Meaning |
|---|---|
| `set_of set_member set_insert set_remove set_size set_union set_inter set_diff set_subset set_elems` | the built-ins of 3.4, same names |
| `map_of map_of_strict map_get map_has map_put map_del map_size map_keys map_values map_entries` | the built-ins of 3.4, same names |
| `eq` extended | extensional equality on `Set`/`Map` (3.5) |
| **fault-only:** `coll_drop_first coll_drop_last coll_add_least coll_swap_values map_of_first` | the five rewrites of 11.1; refused in a claim, a model and an `admit` (as `recipe_is` is in VER04n OC5) |

**CC7.** (A run-time meter.) Every earlier level is bounded by its static node count
(VER04r 4.2 rule 6). `ir.coll` is the first level where one node's cost depends on
the size of its operands, so it adds a **run-time data meter**: each built-in
charges the work of 3.6, a case may charge at most `MAX_IR_DATA_WORK = 4,194,304`
units, and exhaustion is `ir-overflow` (a corpus error when subject-independent, a
case failure otherwise, VER04r 3.2). The generator meters the same way. C1-S0
unifies this meter with the `ir.seq` meter that VER04r reserves; until then
`ir.seq` stays `not-live`.

### 8.5 Instances in `corpus.json`

**CC8.** (Instance suites.) A schema is not a suite. Each instance is a suite whose
claim field is `<claim>__i<8 hex>`, where the hex is the first 8 digits of the
SHA-256 of the **canonical instance text** (parameters in declaration order,
`T=Bytes<4>,N=4`, canonical type names, no spaces). The generator fails if two
instances of one document collide in those 8 digits. The suite row gains
`"schema": "<claim>"` and `"instance": {"T": "Bytes<4>", "N": "4"}`, and the file
`chunks/<claim>__i<8 hex>.txt` follows VER04r 5.5 unchanged. The display form in
reports is `claim[T=Bytes<4>,N=4]`; the `@` of VER04r 5.2 keeps meaning "case
ordinal" (`...exhaustive/add_inserts__i3f9a21c4@17`), which is D3. A top-level
`schemas` section lists each schema, its parameters and constraints, its rows in
document order, and its label (`instances(k)` or `all-instances(n/n)`):

```json
"schemas": { "add_inserts": { "params": [["T","Ord"]],
   "rows": [{"T":"U8"},{"T":"Bytes"},{"T":"(I8,Bool)"}],
   "label": "instances(3)", "universe": null } }
```

**CC9.** (Nat-only generics keep `widths`.) A document whose static parameters are
all `Nat` keeps `width_params` and `corpus.json.widths` exactly as VER04i 2.5
and VER04r 6 write them, with the **v1 claim ids**; no `schemas` entry and no
`__i` suffix. This is what keeps `ct-compare` reproducing its nine v1 digests
(VER04r 1.2 gate). A document with at least one type parameter uses `schemas`
for every generic claim, including the `Nat` parameters of that claim.

### 8.6 Manifest fields

The lane manifest (`warrant-lane-evidence-v2`, VER04r 9.1) gains, per suite row:
`schema` and `instance` (copied from the corpus, so a lane cannot rename them);
for a suite with a collection input, `m1_cases` and `m1_calls` (10.1) and
`entries_built` (13, the number of collection entries the adapter built, which the
corpus predicts without the subject); for a suite that uses `all`, `any`, `count`
or `fold` over a collection, `elements_visited` (CQ3); for a `Map` or `Set`
result with an `unspecified` host, `order_class`. Per generic schema: the
**lane-by-instance matrix** of 9.6, each cell `ran`, `absent` or `not-live` with
a reason. Per lane: `host_hash` (`randomised`, `fixed:<seed>` or `n/a`, 13).

## 9. Lane binding contract

### 9.1 The binding vocabulary

A per-package binding gains three directives, closed like every binding (unknown
fields are errors, W0501). In the line form of VER04i section 5 and VER04r 7.1,
every line is data and none is code:

```text
values  ... Set=hashset Map=hashmap                         -- lane closed list, 9.4
coll    Map<Text,Text>  order=unspecified                   -- per collection type
coll    Set<U8>         order=sorted host=btreeset
generic HashSet mode=static                                 -- 9.5
instance HashSet T=U8      key=u8
instance HashSet T=Bytes   key=vec-u8
instance HashSet T=Text    absent=no-value-equal-host-form
```

* `values Set=` and `Map=` name a host form from the lane's **closed list** (the
  table of 9.4); a name outside it is `BindingInvalid` at start-up, before any
  case. `coll` states the **order class** (9.2) of a collection type and, where a
  type has more than one host form in the lane, which one.
* `instance` rows carry **no host text**. The host type is derived from the
  Warrant type through the lane's closed `values` list (`U8=u8`); a `key=` names a
  host key form from the lane's closed list (9.4). A free-text type name would be
  untrusted input spliced into generated source (VER04n OT1).
* An instance the lane does not run is `absent=<reason>` (9.6); a missing
  instance row with no `absent` is **W0765**.

### 9.2 Order classes

**CL1.** The `order=` of a host collection is one of three facts. The binding
MUST state it for every collection type in a **result** position, and for every
`List` result of an operation whose package declares collection types (a missing
one is W0762: fail closed).

| Class | Meaning | The lane table (9.4) lets it claim it for |
|---|---|---|
| `sorted` | the host yields ascending `ord` (the comparator equals ours for this key form) | only the forms the table marks `canonical` |
| `insertion` | the host yields construction order | `dict`, `Map`, `LinkedHashMap`, Ruby `Hash`, Dart `LinkedHashMap` |
| `unspecified` | anything else, including every hash-seeded iteration | every other form |

An `order=` **stronger** than the lane table allows for that host form is W0762
(declaring `sorted` for a Java `TreeMap<Byte, V>` of unsigned keys, whose natural
order is signed, is the same mistake as 3.3 describes). A host `List` result with
`order=unspecified` is W0762 (3.3). Because a lane could under-declare in the
other direction (`unspecified` for a host that is in fact sorted), that costs
nothing: the adapter sorts anyway.

### 9.3 The adapter, normative

**CL2.** (Host to canonical.) For every collection value that crosses from the
subject to the runner, the adapter MUST: (1) convert each key and value by the
exact-type rules of VER04r 7.2 (a value of the wrong host type is
`type-mismatch`); (2) **sort with the `warrant-corpus` comparator for the key
type**, never the host's sort with a host comparator, never `sorted()` on mixed
types, never a default sort that stringifies (JavaScript `Array.prototype.sort`
without a comparator sorts `10` before `9`); (3) check that no two neighbours are
equal (`duplicate-element`); (4) encode. The comparator and the sort are written
once per lane in `warrant-corpus` and covered by the conformance category
`v2-collections` of VER04r 11.

**CL3.** (Canonical to host, and the order-independence run.) To build a host
collection argument the adapter inserts the canonical entries into a fresh host
collection **twice per case that has a collection argument of 2 or more entries**:
once in canonical order and once in reverse (obligation `order-independence`,
10.1). The two subject results, converted by CL2, MUST be equal; a difference is
the failure `order-dependent`. A host collection is never reused across cases
(freshness, VER04r 7.4), and a non-`inout` collection argument is re-read after
the call: a change is `arg-mutated`.

**CL4.** (Never the host's equality for the check.) The comparison of a result with
the expected value is on the canonical encoding (digests and decoded values), never
the host's `==`, `equals` or `deepEqual` (VER04p 4.3.1 makes the same rule for
floats).

### 9.4 Per-lane table for collections

"From reading": each hazard is the known behaviour of the language and is
**confirmed by that lane's PR and its hazard-row test** (the order witnesses of 5.1
fail a wrong comparator), not assumed. The closed lists name host forms; `canonical`
marks a form whose natural order the lane can certify equals `ord` for the key
forms listed.

| Lane | Host set and map forms | Order | Key and order hazards |
|---|---|---|---|
| rust | `HashSet HashMap` (SipHash, random per map); `BTreeSet BTreeMap` (**canonical** for integers, `String`, `Vec<u8>`, tuples, derived-`Ord` enums in declaration order) | `unspecified`; `sorted` | `HashMap` iteration differs between two maps in one process; `f64` has no `Ord` (never a key); `String` keys need valid UTF-8 (the `Text` policy of VER04l) |
| python | `set frozenset dict`, `list` of pairs | `unspecified` (set); `insertion` (dict) | `True == 1 == 1.0` is one key (keys are checked `type(k) is int`, VER04r 7.2); `str` hash is seeded per process; `bytearray` is unhashable; `sorted()` on mixed types raises |
| typescript | `Set`, `Map` (SameValueZero) | `insertion` | `Uint8Array` and arrays are keyed **by identity**; `-0` and `0` are one key; `bigint` and `number` are distinct; `string` comparison is by **UTF-16 code unit**, not scalar; default `sort()` stringifies |
| go | `map[K]V` (no set type: `map[K]struct{}`) | `unspecified` (randomised on **every** range, even over one map) | `[]byte` is not comparable (converted to `string` or `[N]byte`); `string` order is bytewise (equals scalar order for valid UTF-8); a struct key compares by fields |
| ruby | `Hash`, `Set` (on a Hash) | `insertion` | `eql?` and `hash` define a key; a `String` key is duplicated and frozen; `"é"` in UTF-8 and the same bytes in `ASCII-8BIT` are **unequal**, so `Bytes` and `Text` keys need the right encoding; `<=>` on strings of different encodings |
| c | no library type; the package supplies its own (an opaque struct); **no host form is bound**: a C collection is an opaque type (VER04n) with a `Set` observer | `unspecified` | keys are byte ranges: signed `char` compare; `memcmp` is unsigned (canonical for `Bytes`); `strcmp` stops at NUL |
| cpp | `unordered_set unordered_map` (**unspecified**), `set map` (`std::less`) | `unspecified`; `sorted` | `std::string` and `std::vector<uint8_t>` compare lexicographically unsigned (canonical); a `const char *` key compares **addresses**; `signed char` and `std::less<char>` |
| java | `HashSet HashMap` (bucket order, deterministic per JVM version), `LinkedHashMap` (`insertion`), `TreeMap` | `unspecified`; `insertion`; not canonical | `byte[]` keys hash by identity; boxed `Integer ==` is identity (cache -128..127); `Byte` and `Long` natural order is **signed** (use `Byte.compareUnsigned`); `String.compareTo` is by UTF-16 code unit; `"Aa"` and `"BB"` collide in `String.hashCode` |
| kotlin | as java; `UByte`/`ULong` are boxed in generic collections | as java | `ByteArray` identity; a `data class` key's `copy`; unsigned types box in `Set<UInt>` |
| csharp | `HashSet Dictionary` (**unspecified**; per-process string hashing), `SortedDictionary` | `unspecified`; not canonical | `byte[]` keys by reference; `string.Compare` is **culture sensitive** by default and `CompareOrdinal` is by UTF-16 unit; `sbyte`/`byte`; structs compare field-wise only if `IEquatable` |
| fsharp | `Map`, `Set` (structural order), `Dictionary`, `HashSet` | `sorted` only if the comparer matches `ord`; else `unspecified` | structural comparison of **arrays compares length first**, not lexicographically; strings by `String.CompareOrdinal` (UTF-16 unit); `Map` and `Set` are always ordered by the structural comparison |
| elixir | maps (`%{}`), `MapSet` | `unspecified` (a map with at most 32 keys iterates in key order; **above 32 it does not**) | `1` and `1.0` are distinct keys (`===`), equal under `==`; binaries compare bytewise; atoms are not `Text` |
| lua | tables | `unspecified` (`pairs`) | keys cannot be `nil` or NaN; `t[1.0]` and `t[1]` are one key; a table key is by **identity** (a composite key is serialised to its canonical stream bytes by the adapter, which owns that); `#t` has no value with holes |
| perl | hashes | `unspecified` (random per process, `PERL_HASH_SEED`) | keys stringify (`1`, `1.0`, `"1"` agree); a `utf8`-flagged `"\xe9"` and a byte string `"\xe9"` are **one** key; a NUL in a key is legal; composites are serialised by the adapter |
| haskell | `Data.Map Data.Set` (balanced tree, by `Ord`), `Data.HashMap` | `sorted` (when `Ord` matches); `unspecified` | `Ord` on `ByteString` is lexicographic unsigned (canonical); `Data.Text` `Ord` is by UTF-16 unit in `text-1.x` and by UTF-8 bytes in `text-2.0`; derived `Ord` for sums is by constructor order; laziness (forced before compare) |
| swift | `Set`, `Dictionary` (seeded per process) | `unspecified` | `String ==` is **canonical equivalence**: `"\u{E9}"` and `"e\u{301}"` are one key (the adapter keys by `Array(s.unicodeScalars)` or `utf8`); `Character`; `[UInt8]` is hashable |
| dart | `LinkedHashSet/Map` (default), `HashMap`, `SplayTreeMap` | `insertion`; `unspecified`; `sorted` only with the corpus comparator | `String.compareTo` is by UTF-16 unit; `Uint8List` keys are by identity; on the web ints are doubles (VER04r 7.2) |
| ocaml | `Hashtbl`, `Map.Make`, `Set.Make` | `unspecified`; `sorted` with a corpus-supplied `compare` | `Hashtbl.hash` inspects a bounded number of nodes of a structured key (collisions for long keys); `Int64.compare` is **signed** (use `Int64.unsigned_compare`); polymorphic `compare` raises on functions |

**CL5.** (Equal rigour.) Every lane MUST implement the canonicalisation of CL2,
the construction and order-independence of CL3 and every control of section 11. A
lane that cannot records the operation `not-live` (W0701); it is never counted as
covered and never inherits another lane's result (the rule of VER04n OL3).

### 9.5 Generic modes and the per-lane mapping

**CL6.** A `generic` row declares one **mode** per package. The mode says how a
generated adapter reaches one export at one instance. The eighteen lanes of VER04i
fall into four classes, with equal rigour in each:

| Mode | Lanes | Mapping | Hazards the mode adds |
|---|---|---|---|
| `static` | rust cpp csharp fsharp swift haskell ocaml go dart | one adapter arm per instance, each calling the export at a concrete host type (`HashSet::<u8>`, `HashSet<byte>`, `Set<int>`); an instance with no arm is `absent` | Haskell passes a dictionary (`Ord a`) the adapter must supply: it is the lane's `Ord` for that type, which must be the one the package documents; OCaml's polymorphism is uniform, so one function serves all instances and each arm differs only in its conversion and comparison; Go 1.18+ type parameters are monomorphised or shape-stenciled, either way an arm per instance |
| `erased` | java kotlin typescript | **one export**; the element type is invisible at run time; the adapter supplies the boxed or runtime representation of each instance from the closed list, and checks result elements by runtime class | see below |
| `dynamic` | python ruby elixir lua perl | one export, no static types; an instance differs **only in the value conversion** the adapter applies | the hazards of 9.4: `True == 1`, `1 == 1.0`, stringifying keys |
| `c` | c | `c-void-ptr` (the export takes an element size and a comparator pointer) or `c-per-type` (one named export per instance, such as `hash_set_add_u8`); an instance with no export is `absent` | see below |

**Erased lanes (Java, Kotlin, TypeScript).** The type argument does not exist at
run time, which makes three things true that the evidence must not hide:

* a wrong element is **accepted silently** (heap pollution, a cast the compiler
  allows); the instance `T = U8` and `T = I8` are the same export. The adapter
  therefore checks every element it *receives* by exact runtime class
  (`instanceof Integer`, `typeof x === "bigint"`), and the boxed form is chosen per
  instance from the closed list (`U8` as `Integer` range-checked, `U64` as `Long`
  carrying the bit pattern or `BigInteger`, `Text` as `String`);
* natural order is **not** `ord` (`Byte` and `Long` compare signed, `String`
  compares by UTF-16 unit), so a subject that sorts or tree-hashes by
  `Comparable` is tested against the corpus's `ord`, and the witnesses of 5.1 are
  what make it fail honestly;
* the element representation is the package's own: where the host's natural form
  of `Bytes` has identity equality (`byte[]`, `Uint8Array`, `ByteArray`), the
  instance `T = Bytes` is bound to **the form the package documents or its tests
  use**, or it is `absent=no-value-equal-host-form`. The adapter MUST NOT wrap the
  array in a value class of its own to make the claim pass: that would test the
  wrapper, not the package (VER04i 5 rule 5: no logic in the binding).

**Languages without generics (C, Lua, Perl).** C has none, Lua and Perl have no
types to parameterise. The mapping is stated so a reader can tell what "ran
instance X" means in each:

* **C, `c-per-type`**: a package that exports one function per element type is
  bound by `instance HashSet T=U8 export=hash_set_add_u8`; every instance is then a
  separate, fully typed export and the cell is as strong as any. An instance with no
  export is `absent`.
* **C, `c-void-ptr`**: the package takes the element size and a comparison
  function. The adapter supplies the instance's comparator from `warrant-corpus`
  (an implementation of `ord` for that key form). Then the claim tests the
  *container* with our comparator; the package's own notion of element equality is
  **not exercised**, and the cell prints `eq: supplied by adapter`. A claim about
  the element equality of the subject is not made in this mode.
* **Lua and Perl**: one function; the instance is a *conversion*. `U8` is a Lua
  integer (a float result is `type-mismatch`, VER04r 7.2) or a Perl `IV` that is not
  an `NV`; `Bytes` a byte string; `Text` a character string (Perl: `utf8::decode`d).
  Each instance runs in a **fresh container** that never sees a value of another
  instance, because both languages would happily hold `1` and `"1"` as one key. The
  conversion for an instance is part of the binding's `values` line, never code.
  "Lane L ran instance T" means: values of `T`, converted that way, passed through
  the single export, with the results canonicalised by CL2.

### 9.6 The lane-by-instance matrix

**CL7.** (Coverage is per lane and per instance.) A schema is reported as a matrix,
one row per lane, one column per instance, each cell `ran`, `absent=<reason>` or
`not-live=<reason>`; the count printed is **cells run of cells listed**, never a
total of cases across instances and never a lane count. A lane covers a schema
**only for the instances it ran**: an absent cell is printed and is not inherited
from a neighbouring lane or instance, and a schema is `lane-complete` only when
every cell of every lane that ships the package is `ran`. The headline per-kind
counts are computed per instance and per lane (`full: ...; bounded-set: ...`), and
the word "exhaustive" is never a total (LP-012).

```text
add_inserts<T: Ord>  instances(3): U8, Bytes, (I8,Bool)
        U8   Bytes                              (I8,Bool)
rust    ran  ran                                ran
python  ran  ran                                ran
java    ran  absent=no-value-equal-host-form    ran
c       ran  absent=no-export                   absent=no-export
ruby    absent=no-port  absent=no-port          absent=no-port
cells (the five lanes shown): 9 ran of 15 listed, 6 absent
```

The rows are a toy (five lanes, three instances); the pilot's real matrix is in 15.4.

## 10. Evidence obligations

Four obligations are generated for every document that uses the features. Each
has a failure id, a control (section 11) and a printed status; an obligation that
does not apply prints `n/a (reason)`, never a pass.

### 10.1 The four obligations

**CO1.** (`order-independence`.) For every case that has a collection argument of
two or more entries, the adapter builds the host collection in canonical order and
in reverse (CL3), runs the subject on both, and the two results, canonicalised by
CL2, MUST be equal. Failure id `order-dependent`. The two builds and the second
subject call are counted apart from the claim's own calls: the corpus predicts
`m1_cases` (the cases that qualify, computed from the inputs alone) and
`m1_calls`, and the manifest must reproduce both, so a lane cannot skip the run
and still pass (CB7). A case whose collection arguments are all empty or
singletons has only one insertion order and is not counted. Control: the
fixture subject `insertion-bias` (11.2).

**CO2.** (`canonical-result`.) Every collection result is converted by CL2, which
sorts with the corpus comparator and rejects a repeated element
(`duplicate-element`). The strict-order decoder (CC2) is the second check: a result
that reaches the stream in any other order is a corpus error in the lane's own
re-decoding. Controls: `leak-host-order` and `host-ord` (11.2).

**CO3.** (`key-fidelity`.) Without calling the subject, the adapter builds a host
collection from **every witness set** of every key type the document uses
(`witness<K>()` and its pairs, 5.1) and reads it back by CL2; size and elements
MUST equal the model's. This is the round-trip obligation of VER04l 7.3, for keys:
a host collection that merges `"\u{E9}"` with `"e\u{301}"` (Swift), `True` with `1`
(Python), `-0` with `0`, or a `utf8`-flagged byte string with a byte string
(Perl) fails here, before any operation is blamed. Failure id `key-collapsed`.
Control: `lossy-key`.

**CO4.** (`instance-isolation`.) Each instance suite runs with that instance's
conversion and its own fresh container, and the lane's manifest binds each suite
row to the corpus's `schema` and `instance` fields. Running the `T=U8` suite with
the `T=I8` arm, a suite whose `instance` differs from the corpus, or a container
that survives into the next instance is the failure `instance-mixed`. Controls:
`wrong-instance`, `specialise`, `drop-instance`.

### 10.2 What the obligations do not establish

| Establishes | Does not establish |
|---|---|
| the host's result, after canonicalisation by our comparator, equals the model, for every case of the stated domains | the host's own iteration order, sort or hash (they are never used) |
| the subject's results do not depend on the order in which a container was filled, for the cases where the order can differ | that the subject is correct for a key the domain does not enumerate (it is `bounded-set`) |
| the adapter's host collections do not merge keys the model keeps apart, for the witness sets of the key types used | that every host key form is faithful (only the witnesses are tried) |
| the instances listed ran, in the lanes marked `ran` | the schema at an unlisted type, or the same lane at another instance (parametricity, 6.7) |
| nothing about resource use or flooding resistance of the subject | CWE-407 (section 13, CT4) |

## 11. Sabotage controls (faults)

A control passes only if a named claim or obligation **fails** on a deliberately
wrong subject, and an unrelated failure never credits one (VER04r 8.3). Two kinds
exist because collections have two places a defect can live: in the subject
(a corpus fault, expressed as IR) and in the adapter (which cannot be an IR
rewrite, because a canonical value has no wrong order to rewrite). Every control
that applies MUST be listed in the `faults=` row (else W0502); one that does not
apply prints its condition and is never counted as caught (W0701).

### 11.1 Corpus faults (data, VER04r 8.1)

The IR is built from the fault-only tags of CC6. A rewrite of a composite result
targets **the first `Set` or `Map` found in the result by a left-to-right pre-order
walk** (inside `Ok`, `Some`, a list element, a tuple component); if there is none,
the control is `n/a (no collection in the result)`.

| # | Fault | Kind and rewrite | Must be caught by | `n/a` when |
|---|---|---|---|---|
| CS1 | `drop-element` | `post`: `coll_drop_first(r)`, the least element or entry removed | a claim whose expected value holds it (any reference claim) | every expected collection is empty |
| CS2 | `extra-element` | `post`: `coll_add_least(r)`, the least key of the key type added (for a map, mapped to the value of the first entry) | a reference claim | the map result is always empty (no value to borrow) |
| CS3 | `first-wins` | `replace`: `map_of_first(es)`, the result built from the op's first `List<(K, V)>` parameter with the **first** entry per key winning | a claim over a case with a repeated key and different values | the op has no such parameter, or `dup_key_cases = 0` |
| CS4 | `swap-values` | `post`: `coll_swap_values(r)`, the values of the two least entries exchanged | a reference claim over a map with two entries of different values | `swappable_cases = 0` |
| CS5 | `stale-key` | `post`: the result is the first parameter of the result's type (an `insert` or `remove` that did nothing) | a reference claim over a case where the operation changes the collection | the op has no parameter of the result type |
| CS6 | `count-off-by-one` | `post`: the generic `off-by-one` (VER04r 8.2) on a `Nat` result | the claim over `set_size` or `map_size` | the op returns no `Nat` |

A control is **observable** only if some case differs; the generator counts them
(`dup_key_cases`, `swappable_cases`) and the VER04r 5.6 witness-chunk rule puts
the first such case into the slim set.

### 11.2 Adapter controls (conformance fixture, VER04r 11)

Each is a deliberately broken adapter or subject in the `v2-collections` category
of the shared conformance kit. A lane runner passes the kit before it may write
a manifest (VER04r D7), so each control runs in **every** lane.

| # | Fault | The broken part | Must be caught by |
|---|---|---|---|
| CS7 | `leak-host-order` | the adapter skips the sort of CL2 for an `unspecified` or `insertion` host | the strict-order decoder (CC2), and a digest mismatch on any case with two or more elements |
| CS8 | `host-ord` | the adapter sorts with the host comparator (signed bytes, UTF-16 order, length-first arrays, culture compare) | the witness rows of 5.1: `127` against `128`, `U+FFFF` against `U+10000`, `00` against `0000`. It MUST fail in every lane whose table row (9.4) names a divergent host order, and may pass only where the host order is `canonical`, which the lane prints |
| CS9 | `lossy-key` | the adapter builds the host collection through a key function that merges two distinct keys | `key-fidelity` (CO3), on the witness sets |
| CS10 | `insertion-bias` | a fixture subject whose result depends on the order in which it was filled | `order-independence` (CO1) |

### 11.3 Generic controls

| # | Fault | Mechanism | Must be caught by |
|---|---|---|---|
| CS11 | `wrong-instance` | in the fixture binding, the arm of one instance calls the export of another | `instance-isolation`, and the claims of that instance |
| CS12 | `identity-eq` | a fixture subject that compares elements by reference or identity | **the by-reference-hazard instance** (`Bytes` or `Text`) MUST fail while the scalar instances pass. This is the control that justifies GM5: a schema with scalar instances only could never catch it |
| CS13 | `specialise` | a fixture subject that is wrong at exactly one instance | **only** that instance's suite fails, proving the suites are not aliased |
| CS14 | `drop-instance` | a row removed from the lane manifest | `--check` fails against the document's `schemas` (CO4) |
| CS15 | `forge-universe` | a document or binding that claims `all-instances` with a member missing, or `T = *` with seven rows | refused at load (W0764), before any case |

## 12. Honest labels, counting and vacuity guards

### 12.1 Labels

**CB1.** The kinds are the three of LP-012: **`full`** (every binder enumerates its
whole type), **`bounded-set`** (a fixed strict subset the generator chose or the
author listed) and **`sampled`** (a draw from a space too large). Owner decision d
uses LP-012's spelling `sampled`; the v1 manifest and the drafts VER04n and VER04r
print `sample` for the same kind. The spelling is settled by LP-012's count split,
and this document changes nothing about it (owner question OWN-1). A claim's kind
is the weakest among its binders; a refinement never improves it; "exhaustive" is
never a total.

| Claim form | Example | Kind | VER29 rung |
|---|---|---|---|
| a `Set<Bool>` binder, `set_d1`, over all of `Bool` | `insert_contains` | `full` (4 sets x 2 = 8 cases) | `exhaustive`, if VER29 6.2 holds for the operation |
| `subset_of` over 4 of the 256 `U8` | `insert_exact` | `bounded-set` (16 x 256 = 4,096) | `exhaustive-sampled` |
| `maps_over` listing all of `K` and all of `V` | a `Map<Bool, Bool>` claim (9 maps) | `full` | `exhaustive` |
| `set_d1` or `map_d1` for a large `K` | `Map<U8, U8>`: 81 maps | `bounded-set` | `exhaustive-sampled` |
| `property` or `fuzz` over collections | sets of 0 to 16 members | `sampled` | `randomized` |
| a schema, `instances(k)` | `add_inserts` over three types | the weakest instance kind **capped at `bounded-set`** | at most `exhaustive-sampled` |
| a schema, `all-instances(8/8)` over `Bits` | a bit operation over the eight integers | the weakest instance kind, uncapped | up to `exhaustive` |
| an instance with a `Bytes<N>`, `N >= 2` binder | `T = Bytes<4>` | `sampled` (VER04r 9.3) | `exhaustive-sampled` |
| a `reference` claim, a `predicate` claim | | independent expected values versus `relational` | a `relational` suite never counts as an independent reference |
| a generic opaque instance | `HashSet[T=U8]` | handle binder `bounded-set` (VER04n OB1) | at most `exhaustive-sampled` |

### 12.2 The report block (sketch)

```text
schema add_inserts<T: Ord>    instances(3): U8, Bytes, (I8,Bool)       bounded-set  exhaustive-sampled
   T=U8        bounded-set  sets(set_d1: 263) x x(256) = 67,328 cases   nonempty 67,072  size>=2 1,536
   T=Bytes     bounded-set  ...
   obligations: order-independence pass (m1_cases=..., predicted)  key-fidelity pass (witness sets: 21)
                canonical-result pass (control ok)  instance-isolation pass (control ok)
   matrix (lanes shipping the package 14 of 18):  cells ran 31 of 42, absent 11 (see below)
   flood-resistance: not evidenced
```

(Only the `T=U8` line is computed, from 5.2: `263 x 256 = 67,328`; the non-empty
sets are `262 x 256 = 67,072` and the sets of two members, the 6 pairs of the
witness list, give `6 x 256 = 1,536`. The matrix line and the other numbers are a
toy.)

### 12.3 Vacuity guards

| # | Failure | Prevention |
|---|---|---|
| CB2 | **a claim over empty or one-element collections only** | the report prints `nonempty` and `size>=2` case counts per collection binder (CQ3); the order-witness pairs are in the default domains so ordering is always exercised |
| CB3 | **`all` over nothing** | `elements_visited`, committed in the corpus and reproduced by every lane; zero is W0601 (CQ3) |
| CB4 | **a map control that cannot bite** | `dup_key_cases` and `swappable_cases` are counted and printed; a control with a zero count prints `n/a`, never `caught` (11.1) |
| CB5 | **a schema that cannot see identity bugs** | GM5: every `Eq`, `Ord` or `Finite` parameter needs a by-reference-hazard instance, and `identity-eq` (CS12) is run in every lane |
| CB6 | **an absent or not-live cell counted as covered** | the matrix prints every cell; coverage is cells run of cells listed, never inherited (CL7) |
| CB7 | **the order-independence run skipped** | `m1_cases` and `m1_calls` are predicted by the corpus and reproduced (CO1) |
| CB8 | **`all-instances` claimed for a partial universe** | the label is derived from the rows, only `Bits` is closed, `T = *` expands mechanically, a short block is `instances(k)` (GM6a) |
| CB9 | **a per-key sweep standing in for equality** | the advisory of CQ2 |
| CB10 | **the adapter's build counts hidden** | `entries_built` is predicted by the corpus and reproduced; a lane that builds less skipped a build (13) |
| CB11 | **a relational claim presented as independent** | the `relational` tag of VER04r 4.3 and 9.3 |

## 13. Security considerations

**CT1.** **Bindings, corpora and documents are untrusted input.** The new strings
are host form names (`hashset`), key forms, `instance` rows, the `schemas` section
and the mangled ids. Every one is looked up in a closed list or matched against a
fixed grammar; none is spliced into generated source as text (an `instance` row
carries no host type, 9.1; the mangled claim field is `[A-Za-z_][A-Za-z0-9_]*`
by construction, CK8). A `schemas` entry whose rows differ from the document's is the
corpus error `SchemaDigest`; unknown fields are errors; two instances that collide
in 8 hex digits fail generation (CC8).

**CT2.** **Nested collections: the bound is on leaves, not on entries.**
`Map<K, List<Map<K2, Set<V>>>>` has four sizes that multiply: 4,096 entries per level
is `4,096^4` values. So the bound that matters is the **total number of leaves of one
value**, and every consumer enforces it cumulatively while decoding, before it
allocates, across all nesting levels:

| Bound | Value | Where enforced |
|---|---|---|
| `Set`/`Map` levels in one type | 3 | checker (W0760), corpus type table |
| entries in one collection (corpus, stream, result) | 4,096 | decoder, adapter (`collection-size`) |
| leaves of one value, all nesting | 65,536 | decoder, adapter, evaluator (`value-leaves`) |
| leaves of one case (inputs, results, intermediates together) | `MAX_CASE_LEAVES = 262,144` | generator and evaluator |
| data work of one case | `MAX_IR_DATA_WORK = 4,194,304` | evaluator (CC7) |
| entries in a generated domain value | 16 per collection | descriptors (CD4, CD5) |
| entries built by the adapter per run | `2^28` at `slim`, `2^32` at `full` (counted per entry of every host collection built, doubled where CO1 applies) | generator (W0706), and `entries_built` is reproduced by the lane (CB10) |
| instances per schema, per document | 64, 256 | checker (W0764) |
| total cases | the run totals of VER04r 14, **summed over instances** | generator (W0706) |

**CT3.** **Count bombs.** A stream `71 ffffffff ...` is an allocation attack
(the shape of the CWE-789 findings in VER04j 6.1: a capacity taken from a
header). A decoder checks the count against `MAX_COLLECTION` and against the bytes
that remain **before** it allocates, never pre-sizes from the count beyond
`min(count, 4,096)`, and counts leaves across nesting (CT2). The corpus reader
caps the file sizes already fixed by VER04r 14 before parsing.

**CT4.** **Hash flooding (CWE-407): three surfaces, three rules.**

* *The model, the generator, the oracle and every lane's evaluator.* Collections
  are **sorted arrays or balanced trees ordered by `ord`**; no hash table keyed by
  corpus-chosen or subject-chosen data is used to canonicalise, deduplicate or
  compare. The comparator and the merge or heap sort are `warrant-corpus` code, so
  the worst case is `O(n log n)` with `n <= 4,096` and cannot be driven by a
  chosen hash. (A host sort is allowed only as a stable `O(n log n)` primitive taking
  the corpus comparator; C's `qsort` is not guaranteed to be, so the C lane vendors
  a merge sort.)
* *Host collections the adapter builds to feed the subject (CL3).* They hold at most
  4,096 entries per collection and are built from committed corpus keys, which a
  reviewer and the independent oracle see. A corpus tuned to collide in one host's
  string hash (Java `"Aa"`/`"BB"`) costs at most about 8.4 million comparisons per
  build (4,096 colliding keys), and the build is counted by `entries_built` and
  bounded by the lane's runtime ceiling (`max_wall_seconds`, VER04i 3). A lane MUST NOT switch off the
  host's hash randomisation to make iteration order reproducible: reproduction of a
  failure comes from the case address and the canonical forms, which is what CL2
  is for. Where a lane fixes a seed for another reason (Python `PYTHONHASHSEED`,
  Perl `PERL_HASH_SEED`), the manifest records `host_hash: fixed:<seed>`; the
  default is `randomised`; a lane with no control prints `n/a`.
* *The subject.* Resistance of the library under test to hostile keys is **not
  evidenced** by any claim in this document, and the report always says
  `flood-resistance: not evidenced`. The harness supplies only what it already has:
  the runtime ceiling of VER04i 3, the allocation-capped observer and `bound steps|heap` rows of
  VER04j 6.3 which a document MAY state for an operation, and **document-supplied**
  hostile keys as `example` or WX-E rows (the generator cannot invent keys that
  collide in a subject's hash; VER04j 6.3 says the same of format bombs). When an
  operation has a `bound steps` row and a hostile-key table of at least one row,
  the report prints `flood-resistance: bounded-steps(<op>, <k> keys)` for it, still
  per lane (a lane without an observer prints `not-live`). The `hash-set` pilot is
  exactly this case: from reading, `hash-map` offers unkeyed FNV-1a, djb2 and
  Murmur3 as hash functions, and a SipHash used with a **constant** key, so
  colliding keys can be computed offline (15.5, stage C). Whether a
  security-flagged hash container must carry such evidence before it ships is the
  owner's decision (OWN-2).

**CT5.** **The comparator is part of the trusted base and is tested as such.** Every
`ord` implementation in every lane passes the witness rows of 5.1 and the
conformance vectors of `v2-collections` (including `host-ord`); a lane's
`warrant-corpus` comparator is never replaced by a binding.

**CT6.** **No secrets in collections** (CK9, W0401). Lookup, sort and compare are
variable-time and the corpus is committed in the repository, so a `labels ...
secret` row on a `Set` or `Map` parameter, or a secret-classed key, is refused
(the same rule VER04p applies to floats, VER04n OT2 to handles).

**CT7.** **Aliasing and mutation.** Every collection argument is a fresh host
copy per call and per CO1 build (VER04r 7.4); after the call a non-`inout`
collection equals its pre-call canonical form (`arg-mutated`); a host collection
returned from the subject may be the subject's own storage, so the adapter
canonicalises it into a copy before comparing and, in lanes where host values are
mutable, runs the leak check of VER04n OL7 on it once per collection result.

**CT8.** **The capability jail stays mandatory** (owner decision d, VER08b, VER04i
12). Nothing in collections or generics needs a host facility beyond the lane's
standard runtime; the `c-void-ptr` comparator pointer is adapter code, never a
function the binding names (CT1); the same corpus, binding and wrapper run under the
jail with identical digests, and no manifest is written otherwise.

**CT9.** **No reproducing material in public output.** A failure prints the suite
id, the case address and the failure id (`duplicate-element`, `order-dependent`,
`key-collapsed`, `instance-mixed`, `collection-order`), never a key or a value
(VER00 7.3, VER04i 6.1): keys may be attacker-chosen text.

**CT10.** **Layered verification.** Collections are values, not contracts, so a
lower layer's collection-typed claim is assumed above and never re-proved
(owner decision d). A generic claim is **not published through VER29 in G1**: a
contract id has no instance segment (VER29 3.1), so citing `contract.<pkg>.<claim>`
for a generic claim resolves as open with the reason `generic-not-published`
(section 18, OWN-5). Fail closed.

## 14. The pilot for collections: `csv-parser`

### 14.1 Why this crate

`csv-parser` parses untrusted files, has 14 ports among the eighteen lanes (c cpp
csharp dart fsharp go haskell java kotlin perl python rust swift typescript), and
its result is `List<Map<Text, Text>>`: the Rust port returns
`Vec<HashMap<String, String>>`, whose iteration order is **unspecified**, which is
exactly the owner's rule (decision b). It also has a built-in oracle for last-wins
semantics: from reading `build_row_map`, the map is filled by inserting
`header[i] -> data[i]` (or `""` when the row is short) for each header column in
order, so a duplicate header keeps the **last** column, and a long row is
truncated: that is `map_of` over the padded pairs. The memo adds the caution that
it needs text T1 and that the four crates unblocked by collections alone carry no
security weight; the pilot is chosen for its shape, not for a count.

### 14.2 Staging

| Stage | Content | Prerequisites |
|---|---|---|
| **P-a** | `parse_csv` claim A (below) in the Rust and Python lanes, with the adapter obligations and controls of sections 10 and 11 | C1-S0 to C1-S4, T1 (`Text`, `within` over `Text`) |
| **P-b** | the relational claim C, which needs `ir.seq` for the `map` in `rows_to_maps`; then typescript and go | C1-S5, `ir.seq` meter |
| **P-c** | the remaining ten ports as C2 waves; the hostile-header table and `bound steps` row (CT4), where a lane has the observer | C2a to C2c |

### 14.3 The document (sketch)

Illustrative; names are not checked. `render` is a **writer model** (every field
quoted, a `"` doubled, fields joined by `,`, each record ended by `\n`), so the
expected value of claim A is independent of any parser, the shape VER04l 8
recommends for parsers.

```text
warrant csv_parser v2

error CsvError = UNCLOSED_QUOTE

op parse_records(source: Text, delimiter: Char) -> Result<List<List<Text>>, CsvError>
op parse_csv(source: Text) -> Result<List<Map<Text, Text>>, CsvError>

model cols()  -> List<Text> = [t"a", t"b"]
model cells() -> List<Text> = [t"", t"x", t"a,b", t"q\"q", t"l\nl"]
model render(rows: List<List<Text>>) -> Text = ...       -- a fold over rows and fields
model row_map(h: List<Text>, r: List<Text>) -> Map<Text, Text> =
  map_of(map(range(len(h)), pair_at(h, r)))              -- pad with t"" and truncate to len(h)

claim parse_csv_is_header_map:                           -- claim A, a reference claim
  forall h: List<Text> | within(h, cols(), 2) and len(h) >= 1,
         r: List<Text> | within(r, cells(), 2) and len(r) >= 1.
    parse_csv(render([h, r])) == Ok([row_map(h, r)])

claim parse_csv_agrees_with_records:                     -- claim C, relational
  forall src: Text | within(src, t",\"a\n", 4).
    parse_csv(src) == rows_to_maps(parse_records(src, c","))
```

Counts, by hand. Claim A: `within(h, cols(), 2)` has `1 + 2 + 4 = 7` lists, of which
`len(h) >= 1` admits 6 (`[a] [b] [a,a] [a,b] [b,a] [b,b]`, so **duplicate headers
occur**); `within(r, cells(), 2)` has `1 + 5 + 25 = 31`, of which 30 are
non-empty. Tried `7 x 31 = 217`, admitted `6 x 30 = 180`; kind `bounded-set`; a
`reference` claim, 180 subject calls. For `h = [a, a]` and `r = [x, y]` the model
is `map_of([(a, x), (a, y)]) = {a: y}`. `parse_csv` takes no collection argument, so
`m1_cases = 0` (CO1 does not apply and prints `n/a (no collection argument)`).
Claim C: `within(src, A, 4)` with `|A| = 4` has `1 + 4 + 16 + 64 + 256 = 341`
sources, a `predicate` claim, label `relational`.

The Rust adapter binds `Map=hashmap`, `coll Map<Text,Text> order=unspecified`,
and the result is canonicalised by CL2 before it is compared; the Python adapter
binds `Map=dict`, `order=insertion`. A row with all-empty fields, a blank line, and
`\r\n` are **not** claimed here: they are pinned in P-a by reading the 14 ports
(the S0 task), and claims about them are added only for behaviour that all ports
share or whose difference the owner accepts.

### 14.4 Acceptance

1. Claim A passes in the Rust and Python lanes at `slim` and at `full`, with the
   expected digests reproduced across the two lanes (the digests are independent of
   host order by CC1).
2. A second Rust run with a different `HashMap` seed gives the same manifest
   digests (the order never entered a stream).
3. The corpus faults that apply are caught: `drop-element`, `extra-element` and
   `swap-values` on the first map in the result (CS1, CS2, CS4: `swappable_cases`
   is positive because `[a, b]` with two different cells occurs); an adopter fault
   `first-column-wins`, a `replace` fault using a fault-only model
   (VER04r 8.2) that builds the row map with the **first** duplicate winning, is
   caught by the `h = [a, a]` cases; `first-wins` (CS3) prints `n/a (no List of
   pairs parameter)`.
4. The adapter controls (CS7 to CS9) fail the lane's fixture, and `host-ord` fails
   in the lane tables that name a divergent order (`typescript` and the JVM lanes
   when they arrive).
5. `key-fidelity` passes on the witness sets of `Text` in the lanes that ran, or the
   lane's port **fails honestly**: a Swift port whose `Dictionary<String, String>`
   merges two headers that differ only by canonical equivalence is a finding about
   the port (the owner chooses between fixing it and a declared exemption), not an
   adapter problem.
6. The report prints `flood-resistance: not evidenced` for every op.

## 15. The pilot for generics: `hash-set`

### 15.1 Why this crate

`hash-set` (DT19) is security-flagged in the memo, has ports in 14 of the eighteen
lanes (c cpp csharp fsharp go haskell java kotlin lua perl python rust swift
typescript; a `wasm` port is outside the lanes), is blocked by collections and
generics only, and lands in every lane class of 9.5: static (rust cpp csharp fsharp
go haskell swift), erased (java kotlin typescript), dynamic (python lua perl) and c.
From reading `hash-set` and `hash-map`: the Rust `HashSet<T: Eq + Debug>` is a
wrapper of `HashMap<T, ()>` whose hash is computed over the **`Debug` text** of the
key (`format!("{:?}", key)`), with a chosen function (SipHash 2-4 with a constant
key, FNV-1a 32, Murmur3 32, djb2) and a chosen collision strategy (chaining or
open addressing); the operations take `self` and return a new set (persistent
style). The model needs only `Ord` for its canonical order (D11); the subject
needs `Eq` and a hash.

### 15.2 Restating the operations (D6)

An operation that returns a handle is refused by VER04n OK3, and `add`, `remove`,
`discard`, `union`, `intersection`, `difference` and `symmetric_difference` all do.
VER04n's answer is the **maker plan** (OL1, OR5): the transformer becomes a maker
whose plan applies it, and the derived grounding claim (OV6) is the claim the memo
wrote by hand. Two-handle operations become makers of two lists, because a maker
parameter cannot be a handle (OM3). Operations that return a value stay
operations. For example:

| Library operation | In the document |
|---|---|
| `from_list(xs)` | maker `from_list(xs)` |
| `from_list(xs).add(x)` | maker `from_list_add(xs, x)`, plan `[from_list(xs), step.0.add(x)]` |
| `from_list(xs).union(from_list(ys))` | maker `from_lists_union(xs, ys)`, plan `[from_list(xs), from_list(ys), step.0.union(step.1)]` |
| `contains(&s, x)`, `size(&s)` | operation `contains`, observer `size` |
| `to_list(&s)` | observer `elements` with result `Set<T>` (the host `Vec` is canonicalised, CL2) |
| `is_subset(&a, &b)` | operation over two handles returning `Bool` |
| `with_options(capacity, strategy, hash_fn)` | **not modelled** in G1: the `impl AsRef<str>` parameters are erased by the binding to fixed defaults (memo P-3); strategies and hash functions are the callbacks slice (section 20) |

The plan of a maker is checked by the grounding claim over the **whole** plan
(VER04n OL1), so `add` and `union` are evidenced exactly as well as a handle
operation is.

### 15.3 The document (sketch)

```text
warrant hash_set v2

opaque HashSet<T: Ord>
  make from_list(xs: List<T> | within(xs, witness<T>(), 2)) -> HashSet<T>
    admits true
    means elements = set_of(xs)
    means size     = set_size(set_of(xs))
  make from_list_add(xs: List<T> | within(xs, witness<T>(), 1), x: T | one_of(x, witness<T>())) -> HashSet<T>
    admits true
    means elements = set_insert(set_of(xs), x)
    means size     = set_size(set_insert(set_of(xs), x))
  make from_lists_union(xs: List<T> | within(xs, witness<T>(), 1), ys: List<T> | within(ys, witness<T>(), 1)) -> HashSet<T>
    admits true
    means elements = set_union(set_of(xs), set_of(ys))
    means size     = set_size(set_union(set_of(xs), set_of(ys)))
  observe elements(s: HashSet<T>) -> Set<T>
  observe size(s: HashSet<T>) -> Nat

op contains<T: Ord>(s: HashSet<T>, x: T) -> Bool
op is_subset<T: Ord>(a: HashSet<T>, b: HashSet<T>) -> Bool

claim contains_agrees<T: Ord>:
  forall s: HashSet<T>, x: T. contains<T>(s, x) == set_member(elements<T>(s), x)
claim is_subset_agrees<T: Ord>:
  forall a: HashSet<T>, b: HashSet<T>. is_subset<T>(a, b) == set_subset(elements<T>(a), elements<T>(b))

instantiate contains_agrees:
  T = Bool
  T = U8
  T = I8
  T = Bytes
  T = Text
  T = (I8, Bool)
instantiate is_subset_agrees:
  T = Bool
  T = U8
  T = I8
  T = Bytes
  T = Text
  T = (I8, Bool)
```

The six instances satisfy GM5: scalars `Bool`, `U8`, `I8` and by-reference-hazard
types `Bytes` and `Text`; the tuple `(I8, Bool)` is neither (it holds no heap
representation) but exercises the lexicographic order of 3.2. `U8` and `I8` put
`127`/`128` and `-1`/`1` in every set (`host-ord`); `Text` carries the NFC/NFD pair
(`key-fidelity`). The makers use `within` (VER04l 4.3) over the ground alphabet
`witness<T>()` (CD1) and `one_of(x, A)` (CD7). Both need VER04n's maker rule to
read a parameter refinement as the domain, which section 18.1 requests.

### 15.4 Counts

Hand-computed. For each instance, `w = |witness<T>()|` (4, 2, 4, 7, 10, 4 for `U8`,
`Bool`, `I8`, `Bytes`, `Text`, `(I8, Bool)`; the tuple's witness is the grid of the
first two members of each component, 2 x 2 = 4). The recipes per maker are
`sum_{i<=2} w^i`, `(1 + w) * w` and `(1 + w)^2`:

| `T` | `from_list` | `from_list_add` | `from_lists_union` | handles | `D(T)` | `contains_agrees` | `is_subset_agrees` |
|---|---|---|---|---|---|---|---|
| `Bool` | 7 | 6 | 9 | 22 | 2 | 44 | 484 |
| `U8` | 21 | 20 | 25 | 66 | 256 | 16,896 | 4,356 |
| `I8` | 21 | 20 | 25 | 66 | 256 | 16,896 | 4,356 |
| `Bytes` | 57 | 56 | 64 | 177 | 347 | 61,419 | 31,329 |
| `Text` | 111 | 110 | 121 | 342 | 418 | 142,956 | 116,964 |
| `(I8, Bool)` | 21 | 20 | 25 | 66 | 512 | 33,792 | 4,356 |
| **total** | | | | 739 | | **272,003** | **161,845** |

Every maker list is at most 256 and every handle domain at most 2,048 recipes
(VER04n OV1), so each handle binder is `bounded-set` and none is cut.
`contains_agrees` over `Text` is `342 x 418 = 142,956`, within `B = 400,000`. The
derived grounding suites add the 739 recipes. A schema line reads
`contains_agrees<T: Ord> instances(6)`, kind `bounded-set` (the weakest instance
kind is `bounded-set`, which is also the cap), rung at most
`exhaustive-sampled`. The cases of one lane over all six instances are
`272,003 + 161,845 + 739 = 434,587`, far under the slim total of 32,000,000
(VER04r 14).

The matrix has `18 x 6 = 108` cells. Four lanes (ruby, elixir, dart, ocaml) ship no
`hash-set` port: `4 x 6 = 24` cells are `absent=no-port`. The other 14 lanes have 84
cells, of which the lane PRs decide how many are `ran`; the expected `absent`
ones are `T = Bytes` in the erased lanes and in C where the natural host form is
identity-compared (9.5).

### 15.5 Staging and acceptance

| Stage | Content | Prerequisites |
|---|---|---|
| **P-a** | Rust (static) and Python (dynamic), instances `Bool` and `U8` only; the grounding claims, `contains_agrees` | G1-S0 to G1-S3, VER04n O1-S3 |
| **P-b** | all six instances; Rust, Python, TypeScript (erased) and Go (static); the controls `host-ord`, `identity-eq`, `specialise` in their fixtures | G1-S4 |
| **P-c** | the remaining lanes (G2 waves); the **flood bound**: for the stage's hash functions, a hostile-key table of keys that collide under FNV-1a and under djb2 (computed offline by the document author) and a `bound steps` row on `contains`, in the lanes that have the observer; the report then prints `flood-resistance: bounded-steps(contains, k keys)` | G2a to G2c; OWN-2 decides whether it is required |

Acceptance for P-b:

1. Every claim passes in every `ran` cell, and the report prints the **matrix**
   (cells run of cells listed, per lane and per instance) and the label
   `instances(6)`, `bounded-set`, at most `exhaustive-sampled`; never `exhaustive`.
2. `host-ord` fails the `U8` and `I8` instances (and `Text`) in every lane whose
   table row names a divergent order; it passes only where the host order is
   `canonical`, and the lane prints which.
3. `identity-eq` fails the `Bytes` and `Text` instances and passes `Bool`, `U8` and
   `I8`; the same fixture subject with scalar instances only passes the whole
   schema, which is why GM5 requires a hazard instance.
4. `specialise` fails exactly one instance suite and no other.
5. `drop-instance`, `forge-universe`, `wrong-instance` and the adapter controls
   CS7 to CS10 are caught.
6. VER04n's five obligations hold per instance (the sealing probe once per host
   type, 6.9), and the printed trust base lists the crates of each maker plan.
7. `flood-resistance: not evidenced` is printed (stage P-c changes it per lane
   and per operation only where the bound exists).

## 16. Diagnostics

Existing codes keep their meaning and are reused wherever the condition is one they
already name. Six codes are new.

| ID | Slice | Phase | Trigger |
|---|---|---|---|
| **W0760** | C1 | checking | collection typing (CK1 to CK3, CK10): a type without `ord` in key position, a type parameter in key position without `Ord`, more than three `Set`/`Map` levels, a literal collection, `len`, `<` or any ordering on a `Set` or `Map`, a built-in applied to the wrong arity, a collection as an `inout` parameter |
| **W0761** | C1 | checking, generation | collection domains (CD1 to CD7): a `subset_of`, `maps_over` or `one_of` universe that is not ground, has a duplicate, has a member of the wrong type, or exceeds 16 members; a key type with no `witness` row; a collection binder whose universe makes the product over `B` is W0706 instead |
| **W0762** | C1 | generation | collection binding (CL1, 3.3): `order=` missing where required; `unspecified` bound to a `List` result; `order=` stronger than the lane table allows for the host form; a host form outside the lane's closed list |
| **W0763** | G1 | checking | generics (CK4, CK5, CK8, CK12): a missing, surplus or ill-kinded type argument; an operation used on a type parameter that its constraints do not license; an expression in a width position; recursion through instantiation; a generic claim name over 52 characters |
| **W0764** | G1 | checking, generation | `instantiate` (CK7, GM5, GM6a): an unknown schema or parameter; a missing parameter in a row; an instance that violates its constraint; a duplicate row; fewer than two instances or no by-reference-hazard instance for a parameter; over 64 rows or 256 per document; `*` for a parameter that is not `Bits`; `all-instances` with a member missing; a second `instantiate` for one schema |
| **W0765** | G1 | generation | generic binding (CL6, 9.1): a `generic` mode missing or not allowed for the lane class; an `instance` row with no `absent=` and no host mapping; a free-text host type in an `instance` row; an adapter-supplied wrapper where `absent` is required |

Reused: **W0101** (a v2 construct in a v0 or v1 document, CM0), **W0201/W0203**
(a reserved name; a recursive type through a collection or an instantiation),
**W0301** (a type error), **W0401** (a `secret` label on a collection, CK9),
**W0501** (a malformed `coll`, `generic` or `instance` binding entry), **W0502**
(a `faults=` list that omits an applicable control of section 11), **W0601** (a
claim with no collection element visited, a domain with zero admitted cases),
**W0701** (a control that does not apply, a lane without the level `ir.coll`),
**W0706** (a product, a schema's summed work, the entries-built bound over budget),
**W0707** (a `Bytes<N>` width above 4,096).

### 16.1 Tags and codes, checked

Checked on 2026-11-16 against the tree at the base of this branch and the two draft
branches it is merged with (VER04n, VER04r).

* **Stream tags in use:** `00 01 10 20 30 40 41 50 51` (VER04i 2.6); `11 60 62`
  (draft VER04r 2.4: signed integers, tuple and record, variant); `60 61` (VER04l
  7.2: `Text`, `Char`); `62 63` (draft VER04p, floats); `80 81` (draft VER04n 9.1).
  **`60` is claimed by VER04l and VER04r, and `62` by VER04r and VER04p**; that is
  theirs to settle and does not touch this document. **Chosen: `70` (`Set`) and
  `71` (`Map`); `72` to `79` reserved here.** No other document uses a tag in `70`
  to `79` or `90` and above.
* **Diagnostic codes in use:** `W0001`; `W0101 W0102`; `W0201 W0202 W0203`; `W0301`;
  `W0401 W0402`; `W0501 W0502`; `W0601 W0602`; `W0701` to `W0713`; `W0730` to `W0733`
  (VER04m); `W0750` to `W0753`, block `W0750` to `W0759` reserved (draft VER04n).
  **Chosen: `W0760` to `W0765`; the block `W0760` to `W0769` is reserved here, the
  last four unassigned.** VER04p and VER04r allocate none. If another document merges
  the same numbers first, the later one renumbers at merge; C1-S0 and G1-S0 repeat
  the check.

## 17. Conformance

An implementation of a slice conforms when every item for that slice holds. A
"control" is the negative control named; a rule without a passing control is not
implemented.

**C1-S1 (checker, ground evaluator).**
1. The key table of 3.2 is total: every key type has `ord`, every non-key type is
   W0760 in key position, and `ord` agrees with `==` on the witness lists.
2. Every built-in of 3.4 is total, with the worked examples of 3.4, and `set_remove`
   of an absent element and `map_del` of an absent key return their argument.
3. `len`, `<` on a collection are W0760; `==` on a map with a float value is W0301;
   `same` lifts through map values.
4. `CM5`: a variant that mentions itself through `Map` is W0203.
5. `witness<T>()` for every key class; the sizes of 5.2 (`Bool` 4 and `full`; `U8`
   263; `Bytes` 369) and of 5.3 (the nine-map table) are reproduced.
6. Budget: a `subset_of` of 17 members is W0761; a `maps_over` product over `B` is
   W0706 and an overflowing product is over budget, never zero.
7. v0 and v1 byte-identity tests pass with the reserved names of CM0a absent.

**C1-S2 (generator, oracle, fixture).**
8. The strict-order decoder rejects an unsorted `Set`, a duplicate key and an
   over-count (`collection-order`, `collection-size`); a count of `ffffffff`
   allocates nothing (CT3); leaf counting crosses nesting levels.
9. The independent oracle reads the `.warrant` source with its own reader and
   recomputes every descriptor list (`subsets`, `maps_over`, `set_d1`), the witness
   lists, `dup_key_cases`, `swappable_cases`, `elements_visited` and `m1_cases`; a
   seeded-wrong corpus fails each check.
10. Corpus faults CS1 to CS6: each is caught by its claim and each prints its `n/a`
    condition where it has one.
11. The `v2-collections` conformance category has vectors for `ord` (every row of
    5.1), CL2, the decoder and the faults CS7 to CS10 as broken adapters.

**C1-S3, C1-S4 (lanes).**
12. A lane passes the kit of item 11 before it writes a manifest; `host-ord`
    fails in every lane whose table row names a divergent order and the lane prints
    when it does not.
13. `order-independence`, `canonical-result` and `key-fidelity` run with their
    controls (`insertion-bias`, `leak-host-order`, `lossy-key`); `m1_cases`,
    `m1_calls` and `entries_built` equal the corpus's predictions.
14. The sealed values of 9.2: an `order=sorted` declaration for a form the table
    does not certify is W0762.

**G1-S1, G1-S2 (checker, generator).**
15. A generic body is checked under its constraints only (CK5); a call without type
    arguments is W0763; a width expression is W0763.
16. `instantiate`: each W0764 case of section 16 has a test; the minimum set (GM5)
    is enforced; `T = *` expands to the eight members of `Bits` and yields
    `all-instances(8/8)`; a short block is `instances(k)`.
17. Monomorphisation: no type variable survives into the corpus; instance suites
    carry `schema` and `instance`, ids `<claim>__i<8 hex>`; two instances never
    share chunk files or digests.
18. **The gate:** `ct-compare` under v2 with its `Nat` parameters keeps the
    `widths` encoding and reproduces its nine v1 digests (CC9, VER04r 1.2).
19. The label rules of 12.1: `instances(k)` is never `full` and at most
    `exhaustive-sampled`; `all-instances(8/8)` takes the weakest kind uncapped.

**G1-S3, G1-S4 (lanes, pilot).**
20. The four modes of 9.5 are implemented; a lane without a mode for the package
    records `not-live` (W0701).
21. The matrix is printed with `ran`, `absent=` and `not-live=` cells; counts are
    cells, per lane and per instance; a missing `instance` row without `absent=` is
    W0765.
22. Controls CS11 to CS15 are caught; `identity-eq` fails `Bytes`/`Text` and passes
    the scalar instances.
23. Generic opaque instances: VER04n's five obligations hold per instance; sealing
    is probed per host type (6.9).
24. The pilots' acceptance lists (14.4, 15.5).

## 18. Requirements on other specifications

This document edits no other specification. It **requires** the following; the
slices that make them true apply them (as VER04n section 17 does for VER29).

### 18.1 VER04n (opaque types)

* **4.5 and section 18.** Generic opaque types are accepted as 6.9 states: a type
  parameter on an `opaque` declaration is no longer W0750 in a v2 document that
  uses `instantiate`; each instance is an ordinary opaque type. The grammar of
  VER04n 4.1 gains `NAME "<" static_parameter ... ">"` after `opaque`, and
  `maker_result` accepts the instance type.
* **OV1.** A maker parameter refinement `within(p, A, K)` or `one_of(p, A)` with
  `A` ground is read as the **domain** of that parameter, as VER04l 4.3 reads it
  for a binder; otherwise the maker domain of a list parameter is the ordinary one
  (which for `List<U8>` has 337 members and cuts the recipe list). This is
  needed by 15.3.
* **OQ-12** (nested handles, a handle in a `List`, `Set` or `Map`) is **not** reopened:
  CK11 refuses it, and the reasons of VER04n OQ-12 (recursive recipe descriptors,
  aliasing) stand.
* **OWN-1** (`eq=observational`): still not added. Handles are not key types, so
  the trigger VER04n names ("two adopters need handles as map keys") has not
  occurred.

### 18.2 VER04r (`warrant-corpus-v2`)

1. 2.1 type grammar: `Set<T>`, `Map<K,V>`; 2.2: the optional `instance_of` and `args`
   fields of a `types` row, and a restatement that rule 1 (no recursion) traverses
   `Set` and `Map` (CM5).
2. 2.4: stream rows `70` and `71`, the decoder rules CC2, the leaf cap, the JSON and
   TFV forms of CC4.
3. 4.1: the level `ir.coll`, its tags, the five fault-only tags, and the run-time
   data meter of CC7, to be unified with the `ir.seq` meter when that is defined.
4. 5.1: the descriptor nodes `subsets`, `maps_over`, `set_d1` and the tuple-alphabet
   `list_over`.
5. 6, 9: the `schemas` section, the suite fields `schema` and `instance`, the manifest
   fields of 8.6, the claim field `<claim>__i<8 hex>` (CC8), and the explicit
   statement that a `Nat`-only generic keeps `widths` (CC9).
6. 11: conformance categories `v2-collections` and `v2-generics` (vectors for every
   row of 5.1, CL2, CO1 to CO4 and the controls of sections 11.2 and 11.3).
7. 15.2 item 5: `ir.seq` is needed by C1-S5; its meter should be the data meter of CC7
   generalised, so there is one run-time meter in the profile.

### 18.3 VER04i (lanes and bindings)

The binding directives `coll`, `generic`, `instance` and the `values Set=`/`Map=`
entries with each lane's closed list (9.1, 9.4); the four modes of 9.5; the
statement that `corpus.json.widths` is an instance list for the checker and the
report (6.8); the per-lane table of 9.4 as the hazard rows of the lane's PR; a
pointer paragraph to this document.

### 18.4 VER04j, VER04k, VER04l

VER04j 4.2: the acyclicity sentence names `Set` and `Map` (CM5); VER04j 3.2 is cited,
not changed, for the family `F`. VER04k: no change; element quantification uses 2.2
unchanged (CM0c). VER04l 3.4 records the canonical `Text` order; this document
**uses** it (3.2) and adds nothing to `Text`; `within` (VER04l 4) is used unchanged,
with the generic `witness<T>()` as its alphabet.

### 18.5 VER29, VER30, backlog

VER29: a contract id naming a generic claim resolves as open with the reason
`generic-not-published` until a later slice gives the id an instance segment
(CT10, OWN-5). VER30: a change record (the freeze on collections and generics is
lifted; the order sentence of the memo's section 5). Backlog: section 21.2.

## 19. What VER04s (recursive types) will need

The owner decided that recursive algebraic types get **their own specification,
VER04s, written after this one** (backlog name WX-I, recursive sums). It is not
specified here. This section is a list of the places where this design is already
shaped so recursion can be added, and what VER04s must supply. The point is that
nothing in the encoding or the model of this document has to change.

| # | VER04s must | What this document already does |
|---|---|---|
| 1 | lift the acyclicity rule (VER04j 4.2, VER04r 2.2 rule 1) to **well-founded** recursion: a recursive type needs a non-recursive constructor | CM5 and CK2 make every collection traverse the one rule; VER04s changes it in one place |
| 2 | define `ord` on a recursive type (constructor index, then fields, depth first) and decide whether a recursive type may be a key | `ord` is structural and lexicographic already; a recursive variant fits the same table, and a termination argument is the only new text |
| 3 | give recursive types a **depth-bounded domain** (a `tree_over(depth <= d, alphabet)`), with `size()`/`nth(i)` and a label | descriptors are built from element descriptors, not from the type, so a recursive node is one more descriptor; `within` and `subsets` are the models |
| 4 | bound **decoder recursion** (CWE-674) and total leaves | the stream is already self-delimiting (counts, constructor names), CT2 already counts leaves across nesting, CT3 already forbids pre-sizing; only a depth cap is new |
| 5 | add **structural recursion** to the IR, with a meter | the IR has no recursion (VER04r 4.2 rule 1); `ir.seq` and the data meter of CC7 are the model; a `fold_rec` over a value is bounded by its leaf count |
| 6 | define the host form of a recursive value per lane (boxed trees, C structs and ownership) and **sharing and cycles** in host results | CL2 is driven by the declared type and a leaf counter, so a cyclic host value cannot loop the adapter; sharing and ownership rules (VER04r 7.4) need extending |
| 7 | state `Eq`, `Ord` and `Finite` for recursive types | the constraints are closed (GM2); VER04s extends the table of 3.2 |
| 8 | forbid **polymorphic recursion** (`Nested<T>` mentioning `Nested<List<T>>`), which would make the instance set infinite | CK12 already refuses recursion through instantiation, so monomorphisation terminates |
| 9 | take a fresh diagnostic block and check tags again | this block ends at `W0765` and reserves `W0760` to `W0769`; tags `72` to `79` stay reserved here but VER04s needs none (the existing tags suffice) |

VER04s's pilots (`json-value` with 38 dependents, `cbor`, the tree crates,
`wasm-module-parser`) are not chosen here. A `Map<Text, JsonValue>` is exactly the
shape this document types today **except** for the recursion, which is the
statement that the two documents fit.

## 20. Out of scope

* **Recursive types** (VER04s, section 19): a type that mentions itself, directly,
  through a collection, or through an instantiation.
* **User-defined type classes, interfaces and laws** (memo P2); **higher-kinded and
  associated types**; **variance**; **inference of type arguments**;
  **specialisation**; **type-level arithmetic** (`Bytes<N + 1>`).
* **Callbacks as values**: a host `Fn(T) -> U`, a stored hasher or comparator, the
  `impl AsRef<str>` strategy and hash-function parameters of `with_options` (memo
  P3). The binding fixes them to defaults.
* **Collection keys of kind `List`, `Set`, `Map`, float or opaque**; **handles
  inside collections** (VER04n OQ-12); **multisets** (use `Map<K, Nat>`);
  **ordered maps** as a type (D7); **sorted-by-custom-comparator** containers;
  **custom hashers**.
* **Collections as `inout` or as stateful objects** (CK10); **heaps and priority
  queues** (VER04m).
* **Iterator invalidation, concurrent collections, capacity and load factor** as
  properties of a value; **hash-flooding resistance** as a claim (CT4: bounded where
  the document supplies a bound and a hostile table, otherwise not evidenced).
* **Publication of generic claims through VER29** (CT10, OWN-5); **foreign generic
  handles** (6.9).
* **Proofs**: all collection and generic rows are `not-live` for the proof rung at
  first, acknowledged in the binding, as in VER04m and VER04n.
* **Float keys and float elements in a set** (VER04p: no equality).

## 21. Implementation slices and backlog rows

### 21.1 Slices

| Slice | Scope | Prerequisites |
|---|---|---|
| **C0, G0** | this specification; adversarial review recorded in section 23 | none |
| **C1-S0** | errata pass against the then-current VER04j, k, l, n, p, r; reconcile section 8 with VER04r and file the requests of 18; re-confirm tags `70`, `71` and the block `W0760` to `W0769`; reserved names; accept the collection constructs in the `v2` header parser; the data meter against VER04k 2.7 | C0 reviewed; T1-S0 (`v2` accepted) |
| **C1-S1** | checker and ground evaluator: `Set`/`Map`, `ord`, the 20 built-ins, CK1 to CK3, CK9 to CK11, CM5, `witness`, `subset_of`, `maps_over`, `one_of`, `set_d1`, `map_d1`, the labels | C1-S0; T1 for `Text` keys |
| **C1-S2** | generator at `ir.coll` (no `ir.seq`): tags `70`/`71`, descriptors, the data meter, reference and predicate claims, faults CS1 to CS6, `m1_cases`/`entries_built` predictions, the independent oracle at the level, the `v2-collections` kit | C1-S1; VER04r slices S3 to S5 |
| **C1-S3** | Rust lane: binding reader (`coll`, closed list), CL2, CL3, CO1 to CO3, the adapter controls CS7 to CS10, the shared fixture `collections-fixture` in Rust | C1-S2; the shared-crate queue of VER30 5.3 |
| **C1-S4** | Python lane, with the oracle cross-check | C1-S3 |
| **C1-S5** | element quantification (`all`, `any`, `count`, `fold` over collections), `elements_visited`; pilot `csv-parser` stages P-a and P-b in Rust and Python | C1-S4; the `ir.seq` amendment (VER04r 4.1); WX-D2 |
| **C2a** | lanes typescript, go: adapter, fixture port, hazard-row tests (the order witnesses), controls | C1-S5; the lane's v2 runner (VER04r S6, S7) |
| **C2b** | lanes ruby, c, cpp, java, kotlin, csharp, fsharp, elixir | C2a |
| **C2c** | lanes lua, perl, haskell, swift, dart, ocaml; toolchains in CI first | C2b |
| **G1-S0** | errata for generics; the VER04n amendment of 18.1; re-confirm `W0763` to `W0765`; the `widths` equivalence test plan | C1-S0; VER04n O1-S0 |
| **G1-S1** | checker: parameters, constraints, explicit arguments, `instantiate`, the minimum set, `witness<T>`, `all_values<T>`, generic opaque declarations | C1-S1; VER04n O1-S1 |
| **G1-S2** | generator: monomorphisation, the `schemas` section, instance ids, the `widths` compatibility gate (item 18), the matrix, faults CS11 to CS15, the oracle's independent monomorphiser | G1-S1; C1-S2; VER04n O1-S2 |
| **G1-S3** | Rust (static) and Python (dynamic) lanes: modes, the instance bindings, CO4, the fixture with `specialise` and `identity-eq` | G1-S2; C1-S4; VER04n O1-S3 |
| **G1-S4** | pilot `hash-set`, stages P-a and P-b (Rust, Python, TypeScript, Go) | G1-S3 |
| **G2a** | lanes typescript (erased), go (static), then ruby, java, kotlin (erased), c (`c-per-type`, `c-void-ptr`) | G1-S4; C2a |
| **G2b, G2c** | the remaining lanes; P-c of the pilot | G2a |
| **later** | VER04s, callbacks (memo P3), interfaces (P2), `eq=observational` | demand |

Implementation order (memo section 5): after WX-G, WX-D2, WX-F, LP-012, S1 and T1:
VER04n O1, then **C1**, then **G1**, then FL1. C1-S0 to C1-S4 need neither `ir.seq`
nor generics, so collection work can begin while the `ir.seq` amendment is written.

### 21.2 Backlog rows needed (this document does not edit `VER-BACKLOG.md` or VER30)

* **WX-M0**: spec collections (this document, sections 3 to 5, 7 to 14); status:
  written; **adversarial review required before C1-S0**.
* **WX-M1** (C1-S0 to C1-S5): implementation slices of section 21.1, pilot
  `csv-parser`; depends on WX-G, WX-D2, WX-F, LP-012, S1, T1; C1-S5 on the `ir.seq`
  amendment.
* **WX-M2a, WX-M2b, WX-M2c**: lane rollout, with the `collections-fixture` port per
  lane as its own package (BUILD, README, CHANGELOG, `required_capabilities.json`).
* **WX-P0**: spec generics, the first slice (sections 6, 7, 9.5, 9.6, 11.3, 15).
* **WX-P1** (G1-S0 to G1-S4): the generics slices, pilot `hash-set`; depends on
  WX-M1 and on VER04n O1.
* **WX-P2a, WX-P2b, WX-P2c**: the generics lane rollout, per lane class.
* **WX-MP-REVIEW**: the adversarial review of owner decision (a): three reviewers
  (polyglot lane and host hazards; type system and generics; security, resource
  bounds and hash flooding).
* **WX-I** (recursive sums) re-scoped to VER04s: spec after this one (decision f).
* **LP-012 amendment**: the kinds of the new domains (`set_d1`, `subset_of`,
  `maps_over`, `map_d1`, `one_of`, instance suites), and the settlement of the spelling
  `sample`/`sampled` (OWN-1).
* **VER04n amendment** (18.1) and **VER04r amendment** (18.2), as the owners of those
  drafts see fit; **VER04i amendment** request (18.3).
* **VER29**: the open state `generic-not-published` (CT10).
* **VER13**: an assumption `A-COLLECTION-HOST`, "the lane's canonicalisation and key
  conversion are faithful for the key forms the lane binds", discharged per lane
  by the `v2-collections` kit (CT5).
* **VER30 / inventory**: a change record (the freeze is lifted for collections and
  generics); re-measure the collections and generics rows with a `recursive-types`
  tag before scheduling adopters beyond the pilots.

## 22. Open questions

### 22.1 For the owner

Five questions are choices of policy or scope. Each has a recommendation, and the
text above already encodes it, so a "yes" needs no edit.

| # | Question | Recommendation |
|---|---|---|
| OWN-1 | **Spelling of the third kind.** Your decision d says `sampled` (LP-012). The v1 manifest, VER04n and VER04r print `sample`. Confirm `sampled`, and settle it once at LP-012's count split? | **`sampled`, settled at the count split.** This document uses LP-012's word; the three specifications that print `sample` change together, not one by one |
| OWN-2 | **Flooding evidence for security-flagged hash containers.** Must `hash-set` carry a `bound steps` row and a hostile-key table before it ships (CT4)? | **No in C1 and G1, printed as `not evidenced`.** The harness can bound its own work, and a document can supply a bound, but a collision table is per hash function and per subject; make it a **shipping-gate** decision per crate (VER00), not a language rule |
| OWN-3 | **More closed universes.** Only `Bits` is closed (GM6a). Allow a `Finite` type list, or `Nat` ranges, to be declared closed by the document? | **No.** A document that declares its own universe closed asserts the thing the label exists to check |
| OWN-4 | **The dependency chain.** `csv-parser` needs T1, C1 and the `ir.seq` amendment; `hash-set` needs C1, VER04n O1 and G1. Accept this order (21.1), with collection claims that use no WX-D form first? | **Yes.** The chain is real; C1-S0 to C1-S4 need no `ir.seq` |
| OWN-5 | **Generic claims across layers.** Fail closed: a contract id naming a generic claim is `generic-not-published` (CT10). Accept until a package needs to cite one? | **Yes.** An id with an instance segment is a VER29 design question with no adopter yet |

### 22.2 Engineering questions

| # | Question | Default (encoded above) |
|---|---|---|
| CQ-1 | Should `Set` and `Map` as `inout` host forms exist? | No (CK10); VER04m's codec is the place |
| CQ-2 | A binder over the elements of an earlier binder (`x in s`) with a dependent domain, as `split(m)` does for `Nat`? | No in C1; `all(set_elems(s), p)` and a refinement are the forms (CQ1). Reopen with a customer whose product is over `B` |
| CQ-3 | `List`, `Set` and `Map` as keys? | No; they need VER04s's machinery or a host story (3.2) |
| CQ-4 | Is the `Finite` bound of 16 right? | Yes while it feeds `subset_of` (D5); raise both together |
| CQ-5 | May C1 ship without `ir.seq`? | Yes: C1-S2 is claims without WX-D forms; C1-S5 waits |
| CQ-6 | One data meter or two? | One: CC7 generalised into the `ir.seq` meter at C1-S0 |
| CQ-7 | Does `witness<T>()` for `Text` hold ten members, and are those the right ones? | Yes, from VER04l 7.3 rows 3, 6, 14, 16 and the prefix/NUL rows; C1-S0 re-derives it against `text_d1` |
| CQ-8 | Should `order-independence` (CO1) run every case or a sample at `full`? | Every qualifying case at `slim`; at `full` too, because `m1_cases` is predicted exactly; revisit if interpreted lanes cannot afford the doubling |
| CQ-9 | A lane whose `unspecified` host is seeded: run each case under two seeds? | A SHOULD at nightly `full` in the lanes that can set a seed (`host_hash`, CT4); not a gate |
| CQ-10 | Is `all-instances` worth keeping for one constraint? | Yes: it is the only way a generic claim reaches `exhaustive`, and the eight-integer family is the common case (bit operations) |
| CQ-11 | Dart classification (D4)? | `static` (reified); revisit if a Dart package takes `dynamic` |
| CQ-12 | Suite id mangling: 8 hex digits, collision checked? | Yes; the generator fails on a collision (CC8) |
| CQ-13 | Do instance suites each get their own slim set? | Yes (VER04r 5.6 per suite); the report prints it per instance |
| CQ-14 | `T = *` for a `Finite` parameter? | No (GM6a); only `Bits` |
| CQ-15 | More built-ins (`set_min`, `set_max`, `map_merge`, `map_fold`, `map_filter`)? | Not until an adopter needs one; each is another IR tag in eighteen evaluators |
| CQ-16 | The unkeyed hash functions of `hash-map`: report them? | The report prints, per hash container, `flood-resistance: not evidenced` and nothing else; whether to name the unkeyed functions is the owner's (OWN-2) |

## 23. Specification change record

* Initial VER04o (2026-11-16): normative WX-M and WX-P first slice: collections
  as mathematical sets and maps with one canonical order and the key order `ord`;
  twenty prefixed built-ins; claims over values and over elements with a vacuity
  guard; the order-witness alphabet and the domains `set_d1`, `subset_of`,
  `maps_over`, `map_d1`, `one_of`; rank-1 generics with four constraints,
  `instantiate`, monomorphisation before IR, the minimum instance set, honest
  labels (`instances(n)`, `all-instances(n/n)` over `Bits` only); generic opaque
  types as an amendment to VER04n; the interaction with `Bytes<N>` parameters;
  the corpus encoding (tags `70`, `71`, descriptors, `ir.coll`, instance suites,
  `widths` compatibility); the lane binding contract with four generic modes and a
  per-lane table for eighteen lanes; four obligations; fifteen sabotage controls;
  security (leaf bounds, count bombs, hash flooding on three surfaces); pilots
  `csv-parser` and `hash-set`; what VER04s will need. Departures from the memo
  are D1 to D11 of section 1.2. New diagnostics `W0760` to `W0765`. Edits no other
  specification; requirements on VER04n, VER04r, VER04i, VER29 are section 18.
* **Adversarial review: not yet done.** The freeze is lifted spec first, with
  adversarial review (decision a); the review has not run. Findings will be recorded
  here as R1, R2, ... each with the place it is closed. Checks the author did make:
  the stream tags and diagnostic codes of 16.1 against every specification on the
  base and both draft branches; the section references against the texts as read on
  2026-11-16; the counts of 4.2, 5.2, 5.3, 14.3 and 15.4 by hand. Not checked: no
  fragment has been run through any checker; the per-lane hazards of 9.4 are "from
  reading" and are confirmed only by each lane's hazard-row test; the behaviour of
  the `csv-parser` and `hash-set` ports beyond their public signatures, the code
  named in sections 14 and 15 and the Rust packages was not run.
* Known cross-document conflicts reported, not fixed here: VER04l, the draft VER04r
  and the draft VER04p allocate overlapping stream tags `60` to `62`; VER04n holds
  `80`, `81`, `W0750` to `W0759`.
