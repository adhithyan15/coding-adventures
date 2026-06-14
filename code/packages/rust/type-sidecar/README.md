# coding-adventures-type-sidecar

The **producer-agnostic type-information carrier** for the JavaScript pipeline.
Per [CLOC04](../../../specs/CLOC04-type-sidecar-format.md), JSDoc, TypeScript,
and hand-written `.d.ts`-style annotations all emit the same sidecar shape;
downstream consumers (`closure-typechecker`, optimization passes, the future
V8 clone) read sidecars without knowing or caring which producer wrote them.

## Why a separate crate

The CLOC02 invariant is that the JS AST is **type-blind**. Types arrive as a
parallel input keyed by the same `CvId`s the AST uses. Splitting that input
into its own crate keeps:

- AST consumers free of type-system imports.
- Multiple type producers (JSDoc / TS / external) interchangeable behind one
  shared format.
- The merger between producers a separate, reviewable layer.

## Dependency whitelist

- `serde` + `serde_json` — for the JSON wire format.

Deliberately **not**:
- `javascript-ast` — the sidecar is AST-shape-agnostic.
- Any `closure-*`, `jsdoc-*`, or `typescript-*` crate — those depend *on* this
  crate, not the other way.

A future dep on `coding_adventures_correlation_vector` will land once `CvId`
becomes a true newtype there; for v1, `CvId` is a `String` alias defined
here.

## What's in v1

- `Sidecar` — top-level `HashMap<CvId, Record>` + `format_version: u32`.
- `Record` — `{ cv, ty: Option<Type>, attributes, provenance }`.
- `Type` — primitive variants (`Never`, `Unknown`, `Any`, `Undefined`, `Null`,
  `Boolean`, `Number`, `BigInt`, `String`, `Symbol`) plus an
  `Opaque { raw: String }` escape hatch.
- `Attributes` — `TriState` fields (`nullable`, `readonly`, `pure`,
  `no_side_effects`, `idempotent`), `deprecated: Option<String>`, plus an
  `extension: HashMap<String, serde_json::Value>` for keys that don't yet
  have a typed slot.
- `TriState` — `Unknown` / `True` / `False`.
- `Provenance` — `producer`, `producer_version`, `source_file`,
  `source_location`, `generated_at`, `evidence: Vec<EvidenceStep>`.
- `Sidecar::new()` constructor plus `get`/`ty`/`attr`/`provenance`
  accessors and `insert`.

## What's coming (follow-up PRs)

Per CLOC04 §"The `Type` lattice":
- Object / Function / Class / Constructor / Instance variants.
- Union / Intersection combinators.
- Generics with `TypeParam` / `Generic { base, args }`.
- `NamedRef` for nominal references.
- Literal types (`LiteralString`, `LiteralNumber`, …).
- `type-sidecar-merger` crate for combining sidecars from multiple producers
  with a pluggable conflict policy.
- `SidecarBuilder` for ergonomic producer-side construction.

## Wire format

JSON via serde. `Type` uses `serde(tag = "kind")` so on-disk values look
like:

```json
{ "kind": "Number" }
{ "kind": "Opaque", "raw": "future-syntax" }
```

The full envelope is at `format_version: 1` for everything this crate emits.
