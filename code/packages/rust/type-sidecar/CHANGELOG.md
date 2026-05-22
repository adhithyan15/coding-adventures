# Changelog

All notable changes to the `coding-adventures-type-sidecar` crate will be documented in this file.

## [0.1.0] - 2026-05-21

### Added
- New crate scaffolded per [CLOC04](../../../specs/CLOC04-type-sidecar-format.md) — the producer-agnostic carrier for JS/TS type information.
- `pub const FORMAT_VERSION: u32 = 1` — wire-format version.
- `CvId` type alias for `String` matching the current `correlation-vector` representation (with module docs explaining the upgrade path to a real newtype).
- `Sidecar` struct: `format_version: u32`, `records: HashMap<CvId, Record>`. `Sidecar::new()`, `default()`, `insert(record)`, plus `get`/`ty`/`attr`/`provenance` accessors.
- `Record` struct: `cv`, `ty: Option<Type>`, `attributes: Attributes`, `provenance: Provenance`. `ty = None` distinguishes "no opinion" from "record absent."
- `Type` enum (v1 scaffolding — primitives only):
  - `Never`, `Unknown`, `Any` (three distinct lattice points per CLOC04).
  - `Undefined`, `Null`, `Boolean`, `Number`, `BigInt`, `String`, `Symbol`.
  - `Opaque { raw: String }` escape hatch for types the producer can't lower yet (typechecker treats this as `Unknown`). Encoded as a struct variant rather than a tuple because the `tag = "kind"` serde representation can't carry a newtype around a raw string.
- `Attributes` struct: `TriState` fields (`nullable`, `readonly`, `pure`, `no_side_effects`, `idempotent`), `deprecated: Option<String>`, `extension: HashMap<String, serde_json::Value>` for keys that haven't been promoted to typed fields. `Default` impl returns all-`Unknown`.
- `TriState` enum: `Unknown` / `True` / `False`. Three-valued because "no claim" is distinct from `False`.
- `Provenance` struct: `producer: ProducerId`, `producer_version`, `source_file: Option<String>`, `source_location: Option<String>`, `generated_at: Option<String>`, `evidence: Vec<EvidenceStep>`.
- `ProducerId(pub String)` newtype + `ProducerId::new()` constructor.
- `EvidenceStep { stage, note, at }`.
- All types derive `Debug, Clone, PartialEq, Serialize, Deserialize`. `Type` uses `serde(tag = "kind")` so on-disk values look like `{"kind": "Number"}`.
- Module-level docs explain the role of this crate per CLOC01 + CLOC04 and document the dependency whitelist (no `javascript-ast`, no `closure-*`).
- 12 tests covering: empty/default sidecar, insert + get, accessor short paths, `ty = None` distinction, primitive type round-trip through JSON (all 10 primitives), `Opaque` round-trip, `kind` discriminator on wire format, `Attributes::default()` is all-`Unknown`, `TriState` round-trip, full Sidecar serde round-trip with multiple records, provenance evidence chain growth, `ProducerId` equality.

### Notes
- Zero runtime dependencies beyond `serde` + `serde_json`. No `coding_adventures_correlation_vector` dep yet — `CvId` is a `String` alias here; the dep returns once that crate exposes a `CvId` newtype.
- The full `Type` lattice (Object, Function, Class, Union, Intersection, generics, NamedRef, literals) ships in follow-up PRs to keep this scaffold small. Producers needing those today can encode them as `Type::Opaque` round-trips.
- `SidecarBuilder` and `type-sidecar-merger` are deferred to follow-up PRs.
