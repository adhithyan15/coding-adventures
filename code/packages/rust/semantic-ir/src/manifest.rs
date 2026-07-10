//! Module-level feature manifest.
//!
//! Every SIR module declares which IR features its body uses.  This
//! enables backends to reject incompatible modules in O(1) before
//! traversing the body.  See SIR10 §"Feature manifest" for the
//! design rationale.
//!
//! The feature set is small and stable within a major IR version;
//! adding a feature is a v.bump.

use std::fmt;

/// A SIR feature.  Frontends declare which features their module
/// uses; backends declare which features they accept.
///
/// | Variant                    | Used when the module contains...      |
/// |----------------------------|--------------------------------------|
/// | `Closures`                 | `MakeClosure` or `IndirectCall`       |
/// | `Pairs`                    | builtin call to `cons`/`car`/`cdr`    |
/// | `Symbols`                  | a `SymLit`                            |
/// | `Strings`                  | a `StrLit`                            |
/// | `DynamicTyping`            | a param or global with `sir_type=None`|
/// | `OptionalTypeAnnotations`  | a param or global with `Some(_)` type |
/// | `MutualRecursion`          | functions that reference each other   |
/// | `TailCalls`                | tail-call optimisation required       |
/// | `Globals`                  | any top-level value `define`          |
/// | `Intrinsics`               | any `Intrinsic` node                  |
/// | `StringInterpolation`      | an `Expr::StrConcat` node             |
/// | `DefaultParams`            | a param with `default = Some(_)`      |
/// | `KeywordParams`            | a `Keyword` param or `KeywordArg`     |
/// | `SizedIntegers`            | an `Int` of concrete (non-`Arbitrary`) width |
/// | `Unsigned`                 | an `Int` with `signed == false`       |
/// | `WrappingArithmetic`       | an int op with a non-`Arbitrary` overflow mode |
/// | `FixedArrays`              | a fixed-length `Seq` (C array)        |
/// | `Pointers`                 | a `SirType::Ptr`                      |
/// | `Structs`                  | a `SirType::Struct`                   |
/// | `Bignum`                   | an arbitrary-precision `Int`          |
/// | `NDArrays`                 | a `SirType::NDArray` or `Expr::ArrayLit`/`IndexGet`/`Range` |
/// | `MatrixOps`                | a `MatMul`, `ElementwiseOp`, or `Transpose` node |
/// | `Rationals`                | a `SirType::Rational`                 |
/// | `Complex`                  | a `SirType::Complex`                  |
/// | `ArrayColumnMajor`         | any `ArrayLit`/matrix op — column-major storage convention |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Feature {
    Closures,
    Pairs,
    Symbols,
    Strings,
    DynamicTyping,
    OptionalTypeAnnotations,
    MutualRecursion,
    TailCalls,
    Globals,
    Intrinsics,
    // ── SIR16 (Python / JavaScript interop) ──────────────────────────
    Floats,
    MutableBindings,
    Loops,
    Sequences,
    Maps,
    ShortCircuit,
    // ── SIR17 (object-oriented frontends) ────────────────────────────
    /// Module contains at least one `Stmt::ClassDef`.  Phase 14a
    /// (Ruby) introduces this feature with the empty-body form
    /// `class Foo; end`.  Future Ruby phases extend the body shape
    /// without renaming the feature.
    Classes,
    /// Module contains at least one `Stmt::ModuleDef`.  Phase 14d
    /// (Ruby) introduces this feature with `module M … end`.
    /// Distinct from `Classes`: a Ruby `module` is a namespace/mixin,
    /// not an instantiable class.
    Modules,
    /// Module references an object instance variable
    /// (`Scope::Instance`, Ruby `@x`).  Phase 15a (Ruby).  Instance
    /// vars need no prior declaration and are scoped to the receiver
    /// object, so they are distinct from `Local`/`Global` bindings.
    InstanceVars,
    /// Module references a class variable (`Scope::ClassVar`, Ruby
    /// `@@x`).  Phase 15b (Ruby).  Like instance vars, class vars need
    /// no prior declaration; they are shared across the class
    /// hierarchy rather than per-object.
    ClassVars,
    /// Module references a constant (`Scope::Const`, Ruby `FOO` /
    /// `MyClass` — any uppercase-initial name).  Phase 15c (Ruby).
    /// Like instance/class vars, a constant reference needs no prior
    /// `let` declaration; it resolves against the constant scope.
    Constants,
    /// Module uses structured exception handling (`Stmt::TryCatch`,
    /// Ruby `begin/rescue/ensure/end`).  Phase 16a (Ruby).  Replaces the
    /// earlier `__rescue_marker__`/`__ensure_marker__` placeholder
    /// builtins with a first-class node.
    Exceptions,
    // ── SIR18 (rich string handling) ─────────────────────────────────
    /// Module contains at least one `Expr::StrConcat` node — string
    /// concatenation / interpolation (Ruby `"a#{x}b"`).  Phase 20b
    /// (Ruby) introduces this feature, replacing the v0
    /// `BuiltinCall("string_concat", ...)` marker with a first-class
    /// node.  Distinct from `Strings` (a plain `StrLit`): a backend may
    /// support string literals yet not (yet) know how to build a
    /// concatenation natively, so the two capabilities are tracked
    /// separately.
    StringInterpolation,
    // ── SIR19 (parameter defaults) ───────────────────────────────────
    /// At least one function parameter carries a default-value
    /// expression (`Param::default = Some(_)`, e.g. Ruby `def f(a = 1)`
    /// and the Python / JS equivalents).  This is the core-IR
    /// representation only: it is observed by the validator when any
    /// param has a default, and is NOT yet accepted by any backend, so a
    /// default-using module is correctly rejected by the capability
    /// check until each backend gains support.  Emission (backends) and
    /// lowering (frontends) land in follow-up PRs.
    DefaultParams,
    // ── KW1 (keyword parameters & arguments) ─────────────────────────
    /// The module uses **named keyword parameters** (a `Param` with
    /// `kind == ParamKind::Keyword`, e.g. Ruby `def f(x:)` / `def f(x: 1)`)
    /// and/or **keyword arguments** at a call site (an `Expr::KeywordArg`,
    /// e.g. Ruby `f(x: 1)` / Python `f(x=1)`).  Distinct from
    /// `DefaultParams`: a keyword param is matched by *name*, not position,
    /// and its default (when present) marks it *optional* rather than
    /// *required* — a different axis from a positional trailing default.
    /// Like `DefaultParams`, this is the core-IR representation only: it is
    /// observed by the validator when a keyword param or argument appears
    /// and is NOT yet accepted by any backend, so a keyword-using module is
    /// correctly rejected by the capability check until each backend gains
    /// support.  Emission (backends) and lowering (frontends) land in
    /// follow-up PRs.
    KeywordParams,
    // ── SIR21 T1b (typed integers & source-fidelity types) ───────────
    /// The module uses at least one integer with a **concrete width**
    /// (an `IntSpec` whose width is not `Arbitrary`).  A backend that
    /// accepts this promises to honour the width's min/max/modulus per
    /// the SIR21 faithfulness contract.  A purely-dynamic (Ruby/Python)
    /// module never sets this; a C module does.
    SizedIntegers,
    /// The module uses at least one **unsigned** integer (`IntSpec`
    /// with `signed == false`).  Called out separately from
    /// `SizedIntegers` because some targets (Java) lack native unsigned
    /// types and must lower via the unsigned-op family.
    Unsigned,
    /// The module uses an integer op whose overflow mode is
    /// `Wrap`/`Saturate`/`Checked`/`Trap` (i.e. *not* `Arbitrary`).  A
    /// backend accepting this must realise the declared overflow
    /// behaviour, not silently promote to bignum.
    WrappingArithmetic,
    /// The module uses a fixed-length sequence (`Seq` with a known
    /// length — a C array).  Backends without fixed arrays lower to
    /// their growable list type; this flag lets a backend that *needs*
    /// the distinction (C) see it.
    FixedArrays,
    /// The module uses a pointer/reference type (`SirType::Ptr`).
    /// Dynamic targets lower it to a reference; targets without
    /// pointers reject via this flag.
    Pointers,
    /// The module uses a nominal record type (`SirType::Struct`).
    /// Dynamic targets lower it to a record/object.
    Structs,
    /// The module uses an **arbitrary-precision** integer
    /// (`IntWidth::Arbitrary`) — the Ruby/Python integer that grows.
    /// A backend without a bignum runtime must reject rather than
    /// silently truncate; this is the flag it checks.  (Existing
    /// dynamic backends implicitly support this; the flag makes the
    /// requirement explicit for the sized-integer cascade.)
    Bignum,
    // ── SIR22 (array/matrix numeric-language IR extension) ───────────
    /// The module uses a dense N-D numeric array — a `SirType::NDArray`
    /// type annotation, or any of `Expr::ArrayLit` / `Expr::IndexGet` /
    /// `Expr::Range` / `Stmt::IndexSet`.  A backend that doesn't declare
    /// this in `accepts_features()` cleanly rejects the module via the
    /// existing capability-check mechanism (SIR10) — no new mechanism
    /// needed.  See
    /// [SIR22](../../../../specs/SIR22-array-matrix-semantic-ir.md).
    NDArrays,
    /// The module uses a matrix operator: `Expr::MatMul`,
    /// `Expr::ElementwiseOp`, or `Expr::Transpose`.  Split out from
    /// `NDArrays` because a backend could in principle carry array
    /// *values* (e.g. via an opaque intrinsic) without supporting these
    /// specific operators, though the first-wave frontends declare both
    /// together.
    MatrixOps,
    /// The module uses an exact rational scalar (`SirType::Rational`).
    /// Shared with the future SIR23 symbolic extension.
    Rationals,
    /// The module uses a complex scalar (`SirType::Complex`).  Shared
    /// with the future SIR23 symbolic extension.  Named `Complex` to
    /// match the `SirType::Complex` variant it gates (not to be
    /// confused with `SirType`'s own naming — this is a `Feature`).
    Complex,
    /// Declared whenever any `ArrayLit`/matrix op appears in the
    /// module: states explicitly (per the SIR22 spec's "Storage
    /// convention") that array storage is column-major (Fortran/MATLAB
    /// order), matching `array_runtime::Array`'s internal layout. A JS
    /// backend's own array representation needs this fact stated
    /// explicitly rather than left implicit, since it owns its own
    /// buffer layout and must match the same `(r, c) → c * nrows + r`
    /// indexing formula.
    ArrayColumnMajor,
    // ── SIR26 (integer conversions) ──────────────────────────────────
    /// The module contains an `Expr::Convert` node — an integer
    /// narrowing / reinterpretation to a target width+signedness (a C cast
    /// or implicit conversion).  A backend accepting this renders the
    /// two's-complement reduction (mask + sign-fold); a `Convert`'s target
    /// type also implies `SizedIntegers`/`Unsigned` per SIR21.  See
    /// [SIR26](../../../../specs/SIR26-integer-conversions.md).
    Conversions,
}

impl Feature {
    /// The full list of features, in declaration order.
    pub const ALL: &'static [Feature] = &[
        Feature::Closures,
        Feature::Pairs,
        Feature::Symbols,
        Feature::Strings,
        Feature::DynamicTyping,
        Feature::OptionalTypeAnnotations,
        Feature::MutualRecursion,
        Feature::TailCalls,
        Feature::Globals,
        Feature::Intrinsics,
        Feature::Floats,
        Feature::MutableBindings,
        Feature::Loops,
        Feature::Sequences,
        Feature::Maps,
        Feature::ShortCircuit,
        Feature::Classes,
        Feature::Modules,
        Feature::InstanceVars,
        Feature::ClassVars,
        Feature::Constants,
        Feature::Exceptions,
        Feature::StringInterpolation,
        Feature::DefaultParams,
        Feature::KeywordParams,
        Feature::SizedIntegers,
        Feature::Unsigned,
        Feature::WrappingArithmetic,
        Feature::FixedArrays,
        Feature::Pointers,
        Feature::Structs,
        Feature::Bignum,
        Feature::NDArrays,
        Feature::MatrixOps,
        Feature::Rationals,
        Feature::Complex,
        Feature::ArrayColumnMajor,
        Feature::Conversions,
    ];

    /// Kebab-case name for the SIR text format.
    pub fn name(&self) -> &'static str {
        match self {
            Feature::Closures => "closures",
            Feature::Pairs => "pairs",
            Feature::Symbols => "symbols",
            Feature::Strings => "strings",
            Feature::DynamicTyping => "dynamic-typing",
            Feature::OptionalTypeAnnotations => "optional-type-annotations",
            Feature::MutualRecursion => "mutual-recursion",
            Feature::TailCalls => "tail-calls",
            Feature::Globals => "globals",
            Feature::Intrinsics => "intrinsics",
            Feature::Floats => "floats",
            Feature::MutableBindings => "mutable-bindings",
            Feature::Loops => "loops",
            Feature::Sequences => "sequences",
            Feature::Maps => "maps",
            Feature::ShortCircuit => "short-circuit",
            Feature::Classes => "classes",
            Feature::Modules => "modules",
            Feature::InstanceVars => "instance-vars",
            Feature::ClassVars => "class-vars",
            Feature::Constants => "constants",
            Feature::Exceptions => "exceptions",
            Feature::StringInterpolation => "string-interpolation",
            Feature::DefaultParams => "default-params",
            Feature::KeywordParams => "keyword-params",
            Feature::SizedIntegers => "sized-integers",
            Feature::Unsigned => "unsigned",
            Feature::WrappingArithmetic => "wrapping-arithmetic",
            Feature::FixedArrays => "fixed-arrays",
            Feature::Pointers => "pointers",
            Feature::Structs => "structs",
            Feature::Bignum => "bignum",
            Feature::NDArrays => "nd-arrays",
            Feature::MatrixOps => "matrix-ops",
            Feature::Rationals => "rationals",
            Feature::Complex => "complex",
            Feature::ArrayColumnMajor => "array-column-major",
            Feature::Conversions => "conversions",
        }
    }

    /// Inverse of [`name`].  Returns `None` for unknown names.
    pub fn from_name(name: &str) -> Option<Feature> {
        Feature::ALL.iter().find(|f| f.name() == name).copied()
    }
}

impl fmt::Display for Feature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// A set of features.  Order is preserved for display purposes
/// (deterministic printing → reliable golden tests).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FeatureManifest {
    features: Vec<Feature>,
}

impl FeatureManifest {
    /// An empty manifest.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from a slice, deduplicating.
    pub fn from_features(features: &[Feature]) -> Self {
        let mut m = Self::new();
        for f in features {
            m.add(*f);
        }
        m
    }

    /// Add a feature.  No-op if already present.
    pub fn add(&mut self, feature: Feature) {
        if !self.contains(feature) {
            self.features.push(feature);
        }
    }

    /// `true` iff the feature is declared.
    pub fn contains(&self, feature: Feature) -> bool {
        self.features.iter().any(|f| *f == feature)
    }

    /// Iterate declared features in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = Feature> + '_ {
        self.features.iter().copied()
    }

    /// Number of declared features.
    pub fn len(&self) -> usize {
        self.features.len()
    }

    /// `true` iff no features declared.
    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    /// Features in `self` not in `other`.  Used for validator
    /// reporting (manifest declared X but body doesn't use it →
    /// warning).
    pub fn difference<'a>(&'a self, other: &'a FeatureManifest) -> Vec<Feature> {
        self.iter().filter(|f| !other.contains(*f)).collect()
    }
}

impl fmt::Display for FeatureManifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, feat) in self.features.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{}", feat)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_features_have_unique_names() {
        let mut seen = std::collections::HashSet::new();
        for f in Feature::ALL {
            assert!(seen.insert(f.name()), "duplicate name {}", f.name());
        }
    }

    #[test]
    fn sir21_t1b_features_present_and_named() {
        // The seven SIR21 T1b flags round-trip through name()/from_name
        // and are members of ALL.
        let new = [
            (Feature::SizedIntegers, "sized-integers"),
            (Feature::Unsigned, "unsigned"),
            (Feature::WrappingArithmetic, "wrapping-arithmetic"),
            (Feature::FixedArrays, "fixed-arrays"),
            (Feature::Pointers, "pointers"),
            (Feature::Structs, "structs"),
            (Feature::Bignum, "bignum"),
        ];
        for (feat, name) in new {
            assert_eq!(feat.name(), name);
            assert_eq!(Feature::from_name(name), Some(feat));
            assert!(Feature::ALL.contains(&feat), "{} missing from ALL", name);
        }
    }

    #[test]
    fn name_round_trips() {
        for f in Feature::ALL {
            assert_eq!(Feature::from_name(f.name()), Some(*f));
        }
    }

    #[test]
    fn keyword_params_feature_name_and_round_trip() {
        assert_eq!(Feature::KeywordParams.name(), "keyword-params");
        assert_eq!(
            Feature::from_name("keyword-params"),
            Some(Feature::KeywordParams)
        );
        assert_eq!(format!("{}", Feature::KeywordParams), "keyword-params");
        assert!(Feature::ALL.contains(&Feature::KeywordParams));
    }

    #[test]
    fn unknown_name_is_none() {
        assert_eq!(Feature::from_name("not-a-real-feature"), None);
    }

    #[test]
    fn manifest_dedupes() {
        let m =
            FeatureManifest::from_features(&[Feature::Closures, Feature::Pairs, Feature::Closures]);
        assert_eq!(m.len(), 2);
        assert!(m.contains(Feature::Closures));
        assert!(m.contains(Feature::Pairs));
    }

    #[test]
    fn manifest_display_is_space_separated() {
        let m =
            FeatureManifest::from_features(&[Feature::Closures, Feature::Pairs, Feature::Globals]);
        assert_eq!(format!("{}", m), "closures pairs globals");
    }

    #[test]
    fn manifest_iter_preserves_insertion_order() {
        let mut m = FeatureManifest::new();
        m.add(Feature::Pairs);
        m.add(Feature::Closures);
        m.add(Feature::Globals);
        let v: Vec<_> = m.iter().collect();
        assert_eq!(v, vec![Feature::Pairs, Feature::Closures, Feature::Globals]);
    }

    #[test]
    fn manifest_difference() {
        let a = FeatureManifest::from_features(&[Feature::Closures, Feature::Pairs]);
        let b = FeatureManifest::from_features(&[Feature::Pairs]);
        assert_eq!(a.difference(&b), vec![Feature::Closures]);
        assert_eq!(b.difference(&a), Vec::<Feature>::new());
    }

    #[test]
    fn empty_manifest_renders_empty() {
        assert_eq!(format!("{}", FeatureManifest::new()), "");
    }
}
