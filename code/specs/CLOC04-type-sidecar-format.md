# CLOC04 — Type Sidecar: The JSDoc / TypeScript Lingua Franca

## Why a sidecar

Per CLOC01, **types and annotations are separated**. The JavaScript AST is
type-blind. Types arrive from a parallel input — a *sidecar* — that says, for a
given AST node, "here is the type the world believes this node has, and here is
where that belief came from."

This separation is what lets JSDoc, TypeScript, and hand-written annotations
all feed the same downstream consumer (`closure-typechecker`, optimization
passes, the future V8 clone). The Closure Compiler does not care which producer
wrote the types. The sidecar is the **lingua franca** between any type
producer and any type consumer.

Concretely, the sidecar enables:

1. **JSDoc → JavaScript optimization.** Strip JSDoc comments out of the parser
   path entirely (CLOC05). A separate sub-pipeline reads the comments, extracts
   types, emits a sidecar. The JS AST stays clean.
2. **TypeScript → JavaScript optimization.** TS source goes through its own
   frontend, the type checker resolves types against the symbol table, a
   `typescript-types-extractor` emits a sidecar keyed to the *equivalent*
   JavaScript AST node CvIds. The Closure pipeline never sees TS syntax.
3. **`.d.ts`-style external annotations.** A user hand-writing types for an
   un-annotated JS library writes a sidecar file directly. No parsing of the
   source library required.
4. **Mixed sources.** A project might have JSDoc on its own code, a `.d.ts`
   from a dependency, and an inline override sidecar with stricter types for
   the public API. All three merge into one sidecar before typechecking.

## Where the sidecar lives in the pipeline

```text
                            ┌─► jsdoc-types-extractor ──► Sidecar A ─┐
                            │                                        │
                            ├─► typescript-types-extractor ──► Sidecar B ─┐
javascript-ast::Program ────┤                                            │
                            │                                            ├── type-sidecar-merger ──► merged Sidecar ──► closure-typechecker
                            ├─► external sidecar .json file ──► Sidecar C │
                            │                                            │
                            └─► (any future producer) ──► Sidecar X ─────┘
```

Sidecars are **immutable values**. Each producer emits a fresh sidecar;
merging is a pure function. The merger is its own crate (`type-sidecar-merger`)
so its policy is reviewable and testable in isolation.

## Crate location & layout

```text
code/packages/rust/type-sidecar/
  BUILD
  BUILD_windows
  CHANGELOG.md
  Cargo.toml
  README.md
  required_capabilities.json
  src/
    lib.rs
    sidecar.rs        # the Sidecar struct, builder, lookup API
    record.rs         # a single Record
    ty.rs             # the structural Type lattice
    producer.rs       # ProducerId + provenance
    serde_format.rs   # JSON envelope, version field, schema validation

code/packages/rust/type-sidecar-merger/
  ... standard layout ...
  src/
    lib.rs
    policy.rs         # conflict-resolution policies
```

Crate names: `coding-adventures-type-sidecar` and
`coding-adventures-type-sidecar-merger` (matching repo naming).

## Dependency whitelist

`type-sidecar` may depend only on:

- `coding-adventures-correlation-vector` — for `CvId`.
- `serde` + `serde_json` — for the JSON wire format.
- (Optionally) a small `jsonschema` crate for validating user-supplied sidecar
  files. If the dependency is heavy, we punt validation to a separate
  `type-sidecar-validator` crate.

It must **not** depend on:

- `javascript-ast` — the sidecar is AST-shape-agnostic; it just holds `CvId`
  keys. If consumers want to navigate from a sidecar record to an AST node,
  they do it themselves with their own map.
- Any `closure-*` or `jsdoc-*` or `typescript-*` crate. Those depend on
  `type-sidecar`, not the reverse.

This is what keeps the sidecar usable by every future producer.

## The top-level type: `Sidecar`

```rust
pub struct Sidecar {
    pub format_version: u32,           // bumped on breaking changes
    pub records: HashMap<CvId, Record>,
}
```

A sidecar is a map from `CvId` to a single `Record`. There is **one record per
CvId per sidecar**. If a producer has more than one belief about the same node
(e.g., "from JSDoc this is `string`, but from inference this is `string |
undefined`"), it must encode both inside the record's `evidence` chain, not as
multiple records.

`format_version = 1` for the MVP. The merger and consumers reject sidecars
with unrecognized versions rather than guessing.

## A single record

```rust
pub struct Record {
    /// What this record is about. Always present; this is the key.
    pub cv: CvId,

    /// The resolved type assertion. None means "this producer saw the node
    /// but explicitly says it has no opinion about the type" — distinct from
    /// the node simply being absent from the sidecar.
    pub ty: Option<Type>,

    /// Auxiliary attributes that aren't part of the structural type but
    /// affect optimization: nullability, readonly, deprecation, side-effect
    /// purity, etc. See § "Attributes" below.
    pub attributes: Attributes,

    /// Where this record came from. Always present; lets merger and
    /// debugger explain conflicts.
    pub provenance: Provenance,
}
```

The split between `ty` and `attributes` is deliberate: `ty` is the structural
shape (what set of values the node may hold), `attributes` is everything else
the optimizer cares about (purity, mutability, etc.). Keeping them separate
means `ty` can be merged structurally while `attributes` merges flag-by-flag.

## The `Type` lattice

```rust
pub enum Type {
    // Bottom and top
    Never,
    Unknown,
    Any,                              // distinct from Unknown — see below

    // Primitives
    Undefined,
    Null,
    Boolean,
    Number,
    BigInt,
    String,
    Symbol,

    // Literal types
    LiteralString(String),
    LiteralNumber(f64),
    LiteralBoolean(bool),
    LiteralBigInt(String),            // string-encoded for serde safety

    // Structural
    Array(Box<Type>),
    Tuple(Vec<Type>),
    Object(ObjectType),
    Function(FunctionType),
    Constructor(FunctionType),        // `new (...) => T`
    Class(ClassType),
    Instance(Box<Type>),              // an instance of a class type

    // Combinators
    Union(Vec<Type>),
    Intersection(Vec<Type>),

    // Generics
    Parameter(String),                // a type-parameter reference like `T`
    Generic { base: Box<Type>, args: Vec<Type> },  // List<Foo>

    // Reference to another sidecar record
    NamedRef(NamedRef),

    // Catch-all for things we can't represent yet
    Opaque(String),                   // raw producer-emitted type string
}
```

### `Never` vs `Unknown` vs `Any`

- `Never` is the empty set. A node typed `Never` is unreachable.
- `Unknown` is the universal set with no assumed operations. The typechecker
  rejects any direct use.
- `Any` is the universal set with all operations assumed valid. It's an
  opt-out — JSDoc `@type {*}` and TS `any` both emit `Type::Any`.

The distinction matters for the optimizer: passes can constant-fold or
eliminate `Never`-typed code but cannot assume anything about `Any`-typed
code.

### `Opaque`

The escape hatch. If a producer encounters a type it cannot lower (e.g., TS
mapped types in their full generality, JSDoc `@external` references), it emits
`Type::Opaque(raw)` with the raw string for debug purposes. The typechecker
treats `Opaque` exactly like `Unknown`: it's a non-claim. Passes that depend on
type info skip `Opaque`-typed nodes.

This is what lets early versions of producers ship without full coverage of
their source language. The framework degrades gracefully.

### `ObjectType` and `FunctionType`

```rust
pub struct ObjectType {
    pub fields: Vec<ObjectField>,
    pub index_signature: Option<IndexSignature>,
    pub extends: Vec<Type>,           // inheritance / mixins
}

pub struct ObjectField {
    pub name: String,
    pub ty: Type,
    pub optional: bool,
    pub readonly: bool,
}

pub struct IndexSignature {
    pub key: Type,                    // typically String or Number
    pub value: Type,
}

pub struct FunctionType {
    pub type_params: Vec<TypeParam>,
    pub params: Vec<FunctionParam>,
    pub returns: Type,
    pub this_ty: Option<Type>,
    pub is_async: bool,
    pub is_generator: bool,
}

pub struct FunctionParam {
    pub name: Option<String>,         // None for positional-only
    pub ty: Type,
    pub optional: bool,
    pub rest: bool,
}

pub struct TypeParam {
    pub name: String,
    pub constraint: Option<Type>,
    pub default: Option<Type>,
}
```

### `NamedRef`

Most type systems have nominal references — `interface Foo`, `class Bar`,
`@typedef Baz`. The sidecar represents these as references rather than
inlining the definition:

```rust
pub struct NamedRef {
    pub name: String,                 // "Foo"
    pub defined_at: Option<CvId>,     // the CV of the declaration site
    pub args: Vec<Type>,              // for generic instantiations: Foo<T, U>
}
```

`defined_at` is optional because external references (e.g., to a `.d.ts`
declaration outside the current compile) may not have a `CvId` in scope. When
`defined_at` is present, consumers can look up the *defining* record in the
sidecar and inline it on demand. When absent, the consumer treats the name as
an opaque token.

This is the same trick that makes `.d.ts` files work: references can resolve
later or not at all, without changing the record shape.

## Attributes

```rust
pub struct Attributes {
    pub nullable: TriState,
    pub readonly: TriState,
    pub deprecated: Option<String>,        // deprecation message
    pub pure: TriState,                    // function purity (no side effects)
    pub no_side_effects: TriState,         // weaker than pure: depends on inputs but doesn't mutate state
    pub idempotent: TriState,
    pub visibility: Option<Visibility>,    // public / protected / private (JSDoc + TS)
    pub abstract_: TriState,
    pub r#override: TriState,
    pub extension: HashMap<String, serde_json::Value>,
}

pub enum TriState {
    Unknown,
    True,
    False,
}

pub enum Visibility { Public, Protected, Private }
```

`TriState` instead of `Option<bool>` because the three states have distinct
meanings: *we know it's true*, *we know it's false*, *we don't know*. A
producer that doesn't speak about an attribute emits `Unknown`; the merger
treats `Unknown` as "no claim" and lets other producers fill in.

`extension` is an escape hatch for producers that want to communicate
something the format doesn't yet have a slot for. The closure-typechecker
ignores unknown keys; tooling can inspect them. Once a key gains broad
adoption, it gets promoted into a typed field with a format-version bump.

## Provenance

```rust
pub struct Provenance {
    pub producer: ProducerId,
    pub producer_version: String,
    pub source_file: Option<String>,       // e.g., the .ts file or .js file
    pub source_location: Option<String>,   // line:col of the annotation
    pub generated_at: Option<String>,      // ISO 8601
    pub evidence: Vec<EvidenceStep>,
}

pub struct ProducerId(pub String);  // e.g. "jsdoc", "tsc-5.8", "manual"

pub struct EvidenceStep {
    pub stage: String,                     // "parse" | "infer" | "merge" | ...
    pub note: String,                      // free text
    pub at: Option<String>,
}
```

`provenance` is the chain that lets a debugger answer "*why* does the
typechecker think `userId` is `number`?" — every record carries its full
producer history. The `evidence` chain accumulates as the record passes
through stages (extractor → merger → typechecker downstream contributions, if
any).

Provenance is **not** the same as the CV log. The CV log tracks which AST
nodes flowed where; provenance tracks which producer beliefs flowed where.
They cross-reference via `CvId` but live in separate stores.

## JSON wire format

The on-disk format is straightforward JSON:

```json
{
  "format_version": 1,
  "records": {
    "a3f1.1.4": {
      "cv": "a3f1.1.4",
      "ty": {
        "kind": "Function",
        "type_params": [],
        "params": [
          { "name": "id", "ty": { "kind": "Number" }, "optional": false, "rest": false }
        ],
        "returns": { "kind": "Promise", "args": [{ "kind": "NamedRef", "name": "User" }] },
        "this_ty": null,
        "is_async": true,
        "is_generator": false
      },
      "attributes": {
        "nullable": "Unknown",
        "readonly": "Unknown",
        "deprecated": null,
        "pure": "False",
        "no_side_effects": "False",
        "idempotent": "Unknown",
        "visibility": "Public",
        "abstract_": "Unknown",
        "override": "Unknown",
        "extension": {}
      },
      "provenance": {
        "producer": "jsdoc",
        "producer_version": "1.0.0",
        "source_file": "src/api.js",
        "source_location": "42:1",
        "generated_at": "2026-05-19T10:24:00Z",
        "evidence": [
          { "stage": "extract", "note": "from @param + @returns tags", "at": "42:1" }
        ]
      }
    }
  }
}
```

CvIds serialize as their canonical string form (e.g., `"a3f1.1.4"`).
Field-renaming on the `Type` enum uses serde's `tag: "kind"` discriminator for
human-readability.

## Merging policy

The merger consumes `Vec<Sidecar>` and produces one `Sidecar`. For each
`CvId`:

| Inputs | Output |
| --- | --- |
| Exactly one record exists | That record, unchanged. |
| All records agree on `ty` and `attributes` | One record; provenance becomes a list of all producers. |
| Records disagree | Run the conflict-resolution policy (below). |

The default conflict-resolution policy (`policy::Default`) merges structurally:

- **Types**: intersect. If both records say `string` and `string | undefined`,
  the merge is `string` (the more specific type wins via intersection). If
  `ty` values are structurally incompatible (e.g., `string` vs `number`), the
  merge produces a record with `ty = None` and an `EvidenceStep` noting the
  conflict. Downstream typechecker raises an error.
- **Attributes**: per-field, `True > False > Unknown` for `pure`-like
  attributes (more conservative wins); `False > True > Unknown` for negated
  attributes if any. Specific table in `policy.rs`.
- **`deprecated`**: any non-`None` value wins; if both are `Some` with different
  messages, both are kept (joined with `; `).
- **`extension`**: union of keys; on key collision, the policy says which
  producer wins (priority list configurable).

Alternative policies (`policy::Strict`, `policy::TsWins`, etc.) live alongside
`Default`. Consumers pick the policy when calling the merger. The MVP ships
`Default` and `Strict` (which errors on any disagreement).

Conflict records carry an `EvidenceStep { stage: "merge", note: "..." }` so
the chain stays auditable.

## Sidecar lifecycle in a compile

```rust
// closure-cli, conceptually
let sidecars = vec![
    jsdoc_extractor::run(&program, &mut cv)?,
    typescript_extractor::run(&program, &mut cv, &ts_source)?,
    read_external_sidecars(&opts.sidecar_paths)?,
];

let merged = type_sidecar_merger::merge(sidecars, MergePolicy::Default)?;

let judgments = closure_typechecker::check(&program, &merged, &mut cv)?;
```

Every producer also calls `cv.contribute(record.cv, Contribution{source:
"<producer>", tag: "sidecar-emitted", meta: {...}})` per the CLOC03 invariant.
The CV log is the audit trail; the sidecar is the data.

## Consumer API

The lookup surface a typechecker / pass uses:

```rust
impl Sidecar {
    pub fn get(&self, cv: CvId) -> Option<&Record>;
    pub fn ty(&self, cv: CvId) -> Option<&Type>;
    pub fn attr(&self, cv: CvId) -> Option<&Attributes>;
    pub fn provenance(&self, cv: CvId) -> Option<&Provenance>;
    pub fn resolve_named(&self, name: &str) -> Option<&Record>; // by NamedRef.defined_at
}
```

`Sidecar` is `Sync` and `Send`; consumers can read concurrently. Mutations
happen only at construction; we never expose `&mut Sidecar`.

A `SidecarBuilder` exists for producers:

```rust
let mut sb = SidecarBuilder::new("jsdoc", "1.0.0");
sb.record(node.cv)
  .ty(Type::Number)
  .attr_pure(TriState::True)
  .source_at("src/api.js", "42:1")
  .evidence("extract", "from @param tag");
let sidecar = sb.build();
```

## Versioning the format itself

The `format_version: u32` field on `Sidecar` is the single source of truth.
Bumping rules:

- **Minor extension** (new optional `attributes` field, new `Type` variant
  marked `#[serde(other)]`-friendly): no bump.
- **Breaking shape change** (renamed field, removed variant, changed
  semantics of existing field): bump by 1.

Consumers refuse to load sidecars with `format_version > 1` (or higher than
the consumer's compiled-in maximum). They warn (but accept) on
`format_version < 1` if a forward-compatible adapter exists.

Format version is **not** the same as `producer_version`. The producer can be
at version `9.3.1`; the sidecar format it emits is at version `1`.

## CV plumbing for the sidecar itself

Per CLOC03, every stage in the JS pipeline appends contributions. Sidecar
producers do likewise:

- `jsdoc-types-extractor` contributes `tag: "sidecar-emitted"` to each
  annotated JS-node CvId.
- `typescript-types-extractor` contributes `tag: "sidecar-emitted"` with meta
  pointing at the TS source location.
- `type-sidecar-merger` contributes `tag: "sidecar-merged"` with meta listing
  which producers contributed.
- `closure-typechecker` contributes `tag: "judged"` after consuming the
  sidecar.

The sidecar itself does not own a CV log; it references the shared compile
log via `CvId` keys.

## Testing strategy

| Layer | Tests |
| --- | --- |
| `Type` lattice | Round-trip serde for every variant; structural equality. |
| `Sidecar` builder | Builder calls produce expected records. |
| Merger | Golden tests per policy: inputs → expected merged sidecar. |
| Conflict resolution | A matrix of conflict pairs and expected outcomes. |
| Format version | Loading a v1 sidecar succeeds; loading a v999 sidecar errors with a clear message. |
| Schema validation | A JSON file rejected because it's missing required fields. |

Coverage target per `feedback_repo_standards`: 95%+ for the library crates.

## What this spec does **not** cover

- The JSDoc-to-sidecar producer's internals — that's CLOC05.
- The TypeScript-to-sidecar producer's internals — deferred.
- The typechecker's algorithm — that's the `closure-typechecker` crate's own
  README (no separate CLOC spec yet).
- Pass-by-pass usage of sidecar data — that's individual pass specs.
- A full structural-equality decision procedure for `Type`. The MVP uses
  syntactic equality; a richer equivalence (e.g., for `Union` ordering, mapped
  types) is a follow-up.

## Open questions

1. **`Type::Function` and overloads.** Closure-style `@type {function(string):
   number | function(number): string}` and TS function overloads need either
   `Union<Function, Function>` or a dedicated `Overload` variant. MVP uses the
   union form; we may revisit.
2. **Recursive types.** `type List = null | { head: number; tail: List }`
   requires either lazy resolution via `NamedRef` (current plan) or a
   reference-counted `Arc<Type>` graph. We start with `NamedRef`.
3. **Cross-file scope.** When a TS extractor emits a sidecar referencing types
   declared in another file, how does the merger find them? Answer: the
   `NamedRef.defined_at` field. Open: do we require all referenced files to
   contribute sidecars, or do we allow dangling references? MVP: allow
   dangling, treat as `Opaque` downstream.
4. **Performance.** A bundle with millions of nodes will produce a large
   `HashMap`. We may need an arena or a perfect-hash backing store. Not
   blocking; profile first.
5. **Source-map link.** Should the sidecar carry enough info to produce
   *type-aware* source maps (e.g., "the type of this byte was `User`")? Out
   of scope for MVP; the CV log can already answer this without sidecar help.
