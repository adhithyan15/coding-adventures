//! Type sidecar — the producer-agnostic type-information carrier.
//!
//! # What this crate is for
//!
//! Per [CLOC01](../../../specs/CLOC01-closure-compiler-overview.md) and
//! [CLOC04](../../../specs/CLOC04-type-sidecar-format.md), JavaScript types
//! and the JS AST are kept **separate**. The AST is type-blind; types
//! arrive in a parallel input — a *sidecar* — keyed by the same `CvId`s
//! the AST uses.
//!
//! Multiple producers all emit the same sidecar shape:
//!
//! - **JSDoc extractor** (CLOC05): extracts types from `/** ... */`
//!   comments and emits Records keyed by the JS-side `CvId` the comment
//!   annotates.
//! - **TypeScript extractor** (deferred): parses `.ts` source, runs type
//!   inference, emits Records keyed by the equivalent JS-side `CvId`.
//! - **Hand-written `.d.ts`-style external sidecars**: users write the
//!   JSON directly.
//!
//! Downstream consumers (`closure-typechecker`, optimization passes, the
//! future V8 clone) read sidecars without knowing or caring which
//! producer wrote them.
//!
//! # Dependency whitelist (CLOC04 §"Dependency whitelist")
//!
//! - `coding_adventures_correlation_vector` — for the `CvId` representation.
//! - `serde` + `serde_json` — for the JSON wire format.
//!
//! Explicitly **not**:
//! - `javascript-ast` — the sidecar is AST-shape-agnostic; it just holds
//!   `CvId` keys.
//! - Any `closure-*`, `jsdoc-*`, or `typescript-*` crate — those depend
//!   *on* this crate, not the other way.
//!
//! # What's here in v1 (this scaffolding PR)
//!
//! - The top-level [`Sidecar`] and [`Record`] structs.
//! - The [`Type`] lattice, with primitive variants + the [`Type::Opaque`]
//!   escape hatch. The full lattice from CLOC04 (Object, Function, Class,
//!   Union, Intersection, generics, NamedRef, …) lands in follow-up PRs.
//! - [`Attributes`] (TriState fields), [`TriState`].
//! - [`Provenance`] with [`ProducerId`] and [`EvidenceStep`].
//!
//! Format version: `1`.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The wire-format version. Bumped on breaking shape changes per CLOC04.
pub const FORMAT_VERSION: u32 = 1;

/// A correlation-vector identifier.
///
/// Aliased to `String` here for v1, matching the current
/// `coding_adventures_correlation_vector` representation (where IDs are
/// returned and consumed as `String`). The dependency on
/// `correlation-vector` is kept in `Cargo.toml` so this alias can be
/// upgraded to a real newtype later without a downstream churn — see
/// CLOC02's `CvId` note for the migration plan.
pub type CvId = std::string::String;

// ============================================================================
// Top level: Sidecar
// ============================================================================

/// A type sidecar: a map from `CvId` to a single [`Record`], plus the
/// format-version tag downstream consumers use to reject incompatible
/// inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sidecar {
    /// The wire-format version. Always [`FORMAT_VERSION`] for sidecars
    /// produced by this crate.
    pub format_version: u32,
    /// One record per CvId. Exactly one — producers that hold multiple
    /// beliefs about the same node encode them inside the record's
    /// `evidence` chain, not as separate records.
    pub records: HashMap<CvId, Record>,
}

impl Default for Sidecar {
    fn default() -> Self {
        Self::new()
    }
}

impl Sidecar {
    /// Construct an empty sidecar at the current [`FORMAT_VERSION`].
    pub fn new() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            records: HashMap::new(),
        }
    }

    /// Look up the full record for a CvId, if any. Returns `None` if the
    /// producer has no record about that node.
    pub fn get(&self, cv: &CvId) -> Option<&Record> {
        self.records.get(cv)
    }

    /// Convenience: the resolved type for a CvId, if any.
    pub fn ty(&self, cv: &CvId) -> Option<&Type> {
        self.records.get(cv).and_then(|r| r.ty.as_ref())
    }

    /// Convenience: the attributes for a CvId, if any.
    pub fn attr(&self, cv: &CvId) -> Option<&Attributes> {
        self.records.get(cv).map(|r| &r.attributes)
    }

    /// Convenience: the provenance chain for a CvId, if any.
    pub fn provenance(&self, cv: &CvId) -> Option<&Provenance> {
        self.records.get(cv).map(|r| &r.provenance)
    }

    /// Insert (or overwrite) a record. Producers usually go through a
    /// builder rather than calling this directly; merger crates use it.
    pub fn insert(&mut self, record: Record) {
        self.records.insert(record.cv.clone(), record);
    }
}

// ============================================================================
// Record
// ============================================================================

/// One sidecar entry: the type and attributes a producer believes about
/// the CvId-keyed node, plus the audit chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    /// The CvId this record describes. Mirrors the map key.
    pub cv: CvId,
    /// The resolved type assertion. `None` means "this producer saw the
    /// node but explicitly has no opinion about its type" — distinct
    /// from the node being absent from the sidecar entirely.
    pub ty: Option<Type>,
    /// Auxiliary attributes that aren't part of the structural type but
    /// matter for optimization (nullability, purity, deprecation, …).
    pub attributes: Attributes,
    /// Where this record came from and how it got here.
    pub provenance: Provenance,
}

// ============================================================================
// Type lattice
// ============================================================================

/// The structural type of a node.
///
/// v1 scaffolds the primitive variants plus [`Type::Opaque`]. The full
/// lattice from CLOC04 §"The `Type` lattice" — Object, Function, Class,
/// Union, Intersection, generics, NamedRef, literal types — lands in
/// follow-up PRs to keep this scaffolding small and focused.
///
/// The `kind` discriminator on the wire is human-readable (`"Null"`,
/// `"Number"`, etc.) per the CLOC04 JSON format.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Type {
    /// The empty set — a node typed `Never` is unreachable.
    Never,
    /// The universal set with no assumed operations. The typechecker
    /// rejects any direct use; distinct from [`Type::Any`].
    Unknown,
    /// The universal set with all operations assumed valid. The opt-out
    /// type — JSDoc `@type {*}` and TypeScript `any` both lower to this.
    Any,
    /// The literal `undefined` type.
    Undefined,
    /// The literal `null` type.
    Null,
    /// The primitive `boolean` type.
    Boolean,
    /// The primitive `number` type.
    Number,
    /// The `bigint` type (ES2020+).
    BigInt,
    /// The primitive `string` type.
    String,
    /// The `symbol` type (ES2015+).
    Symbol,
    /// Escape hatch for types the producer cannot lower yet. The `raw`
    /// field is the producer-emitted raw form for debug purposes; the
    /// typechecker treats `Opaque` exactly like [`Type::Unknown`] (no
    /// claim) so the framework degrades gracefully.
    ///
    /// Encoded as a struct variant rather than a tuple variant because
    /// the `tag = "kind"` serde representation can't carry a tagged
    /// newtype around a raw string.
    Opaque { raw: std::string::String },
}

// ============================================================================
// Attributes
// ============================================================================

/// Per-node attributes that affect optimization but aren't part of the
/// structural type itself. Each field uses [`TriState`] so producers can
/// distinguish "we know it's true" from "we know it's false" from "we
/// have no opinion."
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attributes {
    /// Whether the node's value can be `null` / `undefined`.
    pub nullable: TriState,
    /// Whether the node is a read-only binding or property.
    pub readonly: TriState,
    /// Whether a function/expression is pure (no side effects, output
    /// determined by inputs only).
    pub pure: TriState,
    /// Weaker than `pure`: depends on inputs but does not mutate state.
    pub no_side_effects: TriState,
    /// Whether a function is idempotent (calling twice equals calling
    /// once).
    pub idempotent: TriState,
    /// Optional deprecation message. `Some(_)` means deprecated.
    pub deprecated: Option<std::string::String>,
    /// Free-form attribute extension space. Lets producers communicate
    /// attributes that don't yet have a typed slot here; once a key
    /// gains broad adoption, it gets promoted into a real field with a
    /// format-version bump.
    #[serde(default)]
    pub extension: HashMap<std::string::String, serde_json::Value>,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            nullable: TriState::Unknown,
            readonly: TriState::Unknown,
            pure: TriState::Unknown,
            no_side_effects: TriState::Unknown,
            idempotent: TriState::Unknown,
            deprecated: None,
            extension: HashMap::new(),
        }
    }
}

/// A three-valued boolean for attributes. The three states have distinct
/// meanings: a producer that doesn't speak about an attribute emits
/// [`Unknown`](TriState::Unknown), which the merger then treats as "no
/// claim" and lets other producers fill in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TriState {
    /// No claim either way.
    Unknown,
    /// Known to be true.
    True,
    /// Known to be false.
    False,
}

// ============================================================================
// Provenance
// ============================================================================

/// The chain of where a record came from. Always present so a debugger
/// can answer "*why* does the typechecker believe this?"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    /// Who emitted this record.
    pub producer: ProducerId,
    /// Version string of the producer (free-form; SemVer-ish).
    pub producer_version: std::string::String,
    /// The source file the type came from, if any (e.g. the `.ts` file
    /// or the `.js` file containing the JSDoc comment).
    pub source_file: Option<std::string::String>,
    /// Free-form location string within `source_file`, e.g. `"42:1"`.
    pub source_location: Option<std::string::String>,
    /// Optional ISO 8601 timestamp the record was produced.
    pub generated_at: Option<std::string::String>,
    /// The audit chain. Grows as the record passes through stages
    /// (extractor → merger → typechecker contributions).
    #[serde(default)]
    pub evidence: Vec<EvidenceStep>,
}

/// An identifier for a sidecar producer. Conventional values:
/// `"jsdoc"`, `"tsc-5.8"`, `"manual"`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProducerId(pub std::string::String);

impl ProducerId {
    /// Convenience constructor.
    pub fn new(id: impl Into<std::string::String>) -> Self {
        Self(id.into())
    }
}

/// One step in a [`Provenance::evidence`] chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceStep {
    /// The stage that recorded this step (`"parse"`, `"infer"`,
    /// `"merge"`, …).
    pub stage: std::string::String,
    /// Free-text note about what happened at this stage.
    pub note: std::string::String,
    /// Optional location string anchoring the step (e.g. `"42:1"`).
    pub at: Option<std::string::String>,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_provenance() -> Provenance {
        Provenance {
            producer: ProducerId::new("jsdoc"),
            producer_version: "1.0.0".into(),
            source_file: Some("src/api.js".into()),
            source_location: Some("42:1".into()),
            generated_at: None,
            evidence: vec![EvidenceStep {
                stage: "extract".into(),
                note: "from @type tag".into(),
                at: Some("42:1".into()),
            }],
        }
    }

    fn sample_record(cv: &str) -> Record {
        Record {
            cv: cv.to_string(),
            ty: Some(Type::Number),
            attributes: Attributes {
                nullable: TriState::False,
                pure: TriState::True,
                ..Attributes::default()
            },
            provenance: sample_provenance(),
        }
    }

    #[test]
    fn sidecar_new_is_empty_at_format_version_1() {
        let s = Sidecar::new();
        assert_eq!(s.format_version, FORMAT_VERSION);
        assert_eq!(s.format_version, 1);
        assert!(s.records.is_empty());
    }

    #[test]
    fn sidecar_default_matches_new() {
        let a = Sidecar::default();
        let b = Sidecar::new();
        assert_eq!(a, b);
    }

    #[test]
    fn sidecar_insert_and_get() {
        let mut s = Sidecar::new();
        let rec = sample_record("a3f1.1");
        s.insert(rec.clone());

        assert_eq!(s.get(&"a3f1.1".to_string()), Some(&rec));
        assert_eq!(s.get(&"nope.1".to_string()), None);
    }

    #[test]
    fn sidecar_accessors_short_paths() {
        let mut s = Sidecar::new();
        s.insert(sample_record("a3f1.1"));
        let id = "a3f1.1".to_string();

        assert_eq!(s.ty(&id), Some(&Type::Number));
        assert!(matches!(s.attr(&id).unwrap().pure, TriState::True));
        assert_eq!(s.provenance(&id).unwrap().producer, ProducerId::new("jsdoc"));

        let missing = "missing.1".to_string();
        assert!(s.ty(&missing).is_none());
        assert!(s.attr(&missing).is_none());
        assert!(s.provenance(&missing).is_none());
    }

    #[test]
    fn record_with_no_opinion_has_ty_none() {
        // CLOC04: ty = None means "producer saw the node but has no
        // opinion" — distinct from the node being absent entirely.
        let rec = Record {
            cv: "x.1".into(),
            ty: None,
            attributes: Attributes::default(),
            provenance: sample_provenance(),
        };

        let mut s = Sidecar::new();
        s.insert(rec);
        let id = "x.1".to_string();

        assert!(s.get(&id).is_some());
        assert!(s.ty(&id).is_none());
    }

    #[test]
    fn type_primitives_round_trip_through_json() {
        // The full primitive set should serialize/deserialize cleanly via
        // the `tag = "kind"` discriminator.
        let primitives = [
            Type::Never,
            Type::Unknown,
            Type::Any,
            Type::Undefined,
            Type::Null,
            Type::Boolean,
            Type::Number,
            Type::BigInt,
            Type::String,
            Type::Symbol,
        ];
        for t in primitives {
            let j = serde_json::to_string(&t).unwrap();
            let back: Type = serde_json::from_str(&j).unwrap();
            assert_eq!(t, back, "round-trip failed for {:?}", t);
        }
    }

    #[test]
    fn type_opaque_round_trips_with_raw_string() {
        let t = Type::Opaque { raw: "MappedType<keyof Foo>".into() };
        let j = serde_json::to_string(&t).unwrap();
        let back: Type = serde_json::from_str(&j).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn type_serializes_with_kind_discriminator() {
        let json = serde_json::to_value(Type::Number).unwrap();
        // Per CLOC04 wire format, the kind field is the discriminator.
        assert_eq!(json["kind"], "Number");
    }

    #[test]
    fn attributes_default_is_all_unknown() {
        let a = Attributes::default();
        assert_eq!(a.nullable, TriState::Unknown);
        assert_eq!(a.readonly, TriState::Unknown);
        assert_eq!(a.pure, TriState::Unknown);
        assert_eq!(a.no_side_effects, TriState::Unknown);
        assert_eq!(a.idempotent, TriState::Unknown);
        assert_eq!(a.deprecated, None);
        assert!(a.extension.is_empty());
    }

    #[test]
    fn tristate_round_trips() {
        for t in [TriState::Unknown, TriState::True, TriState::False] {
            let j = serde_json::to_string(&t).unwrap();
            let back: TriState = serde_json::from_str(&j).unwrap();
            assert_eq!(t, back);
        }
    }

    #[test]
    fn sidecar_serde_round_trip() {
        // End-to-end: insert a record, serialize the whole sidecar, parse
        // it back, get identical contents.
        let mut s = Sidecar::new();
        s.insert(sample_record("a3f1.1"));
        s.insert(Record {
            cv: "b2c4.7".into(),
            ty: Some(Type::Opaque { raw: "future-syntax".into() }),
            attributes: Attributes::default(),
            provenance: sample_provenance(),
        });

        let json = serde_json::to_string(&s).unwrap();
        let back: Sidecar = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
        assert_eq!(back.format_version, 1);
        assert_eq!(back.records.len(), 2);
    }

    #[test]
    fn provenance_evidence_grows_as_vec() {
        let mut p = sample_provenance();
        let before = p.evidence.len();
        p.evidence.push(EvidenceStep {
            stage: "merge".into(),
            note: "combined with tsc-5.8 record".into(),
            at: None,
        });
        assert_eq!(p.evidence.len(), before + 1);
    }

    #[test]
    fn producer_id_equality_by_inner_string() {
        assert_eq!(ProducerId::new("jsdoc"), ProducerId::new("jsdoc"));
        assert_ne!(ProducerId::new("jsdoc"), ProducerId::new("tsc-5.8"));
    }
}
