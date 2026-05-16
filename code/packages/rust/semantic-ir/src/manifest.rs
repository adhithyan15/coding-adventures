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
}

impl Feature {
    /// The full list of v0 features, in declaration order.
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
    fn name_round_trips() {
        for f in Feature::ALL {
            assert_eq!(Feature::from_name(f.name()), Some(*f));
        }
    }

    #[test]
    fn unknown_name_is_none() {
        assert_eq!(Feature::from_name("not-a-real-feature"), None);
    }

    #[test]
    fn manifest_dedupes() {
        let m = FeatureManifest::from_features(&[
            Feature::Closures,
            Feature::Pairs,
            Feature::Closures,
        ]);
        assert_eq!(m.len(), 2);
        assert!(m.contains(Feature::Closures));
        assert!(m.contains(Feature::Pairs));
    }

    #[test]
    fn manifest_display_is_space_separated() {
        let m = FeatureManifest::from_features(&[
            Feature::Closures,
            Feature::Pairs,
            Feature::Globals,
        ]);
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
