//! Layout selection: which authored layout variant an app shows in which
//! host environment (UI48 §7.2, ENV3).
//!
//! A component may author several layouts — `TaskApp.mll`, and beside it
//! `TaskApp.compact.mll` for phones or `TaskApp.touch.mll` for touch screens
//! (UI30). Variant names are opaque to the compiler, so the package says which
//! environment selects which one:
//!
//! ```toml
//! # First match wins; no match is the default layout.
//! [[app.layouts]]
//! variant = "compact"
//! size-class = "compact"
//!
//! [[app.layouts]]
//! variant = "touch"
//! pointer = "coarse"
//! ```
//!
//! [`select_variant`] is the whole decision, as a pure function. Every
//! backend's generated shell carries the same rules, emitted from
//! [`LayoutRule`]s, and asks the same question of the environment it
//! observes, so a phone picks the same layout on SwiftUI as on Compose.
//!
//! ```text
//!   host observes ──▶ environment (size-class=compact, pointer=coarse, …)
//!                          │
//!                          ▼
//!   rules, in order:  compact? { size-class = compact }  ── match ──▶ "compact"
//!                     touch?   { pointer = coarse }
//!                     (none matched) ─────────────────────────────▶ default
//! ```

use std::collections::{BTreeMap, HashSet};

use serde::Deserialize;

/// The UI48 §4 axes a rule may test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EnvironmentAxis {
    SizeClass,
    Pointer,
    Hover,
    Orientation,
    ColorScheme,
    ReducedMotion,
}

impl EnvironmentAxis {
    pub const ALL: [EnvironmentAxis; 6] = [
        EnvironmentAxis::SizeClass,
        EnvironmentAxis::Pointer,
        EnvironmentAxis::Hover,
        EnvironmentAxis::Orientation,
        EnvironmentAxis::ColorScheme,
        EnvironmentAxis::ReducedMotion,
    ];

    /// The key a rule uses, the same kebab-case as UI48 §4.
    pub fn key(self) -> &'static str {
        match self {
            EnvironmentAxis::SizeClass => "size-class",
            EnvironmentAxis::Pointer => "pointer",
            EnvironmentAxis::Hover => "hover",
            EnvironmentAxis::Orientation => "orientation",
            EnvironmentAxis::ColorScheme => "color-scheme",
            EnvironmentAxis::ReducedMotion => "reduced-motion",
        }
    }

    /// Every value the axis can take (UI48 §4). Closed, so a rule that could
    /// never match is refused when the manifest is read.
    pub fn values(self) -> &'static [&'static str] {
        match self {
            EnvironmentAxis::SizeClass => &["compact", "regular", "expanded"],
            EnvironmentAxis::Pointer => &["coarse", "fine", "none"],
            EnvironmentAxis::Hover => &["hover", "none"],
            EnvironmentAxis::Orientation => &["portrait", "landscape"],
            EnvironmentAxis::ColorScheme => &["light", "dark"],
            EnvironmentAxis::ReducedMotion => &["reduce", "no-preference"],
        }
    }

    fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|axis| axis.key() == key)
    }
}

/// One rule: this variant, when every condition holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutRule {
    pub variant: String,
    /// Axis and required value, in axis order. Empty means "always".
    pub conditions: Vec<(EnvironmentAxis, String)>,
}

impl LayoutRule {
    fn matches(&self, observed: &impl Fn(EnvironmentAxis) -> String) -> bool {
        self.conditions
            .iter()
            .all(|(axis, value)| observed(*axis) == *value)
    }
}

/// The variant to show, or `None` for the default layout: the first rule
/// whose conditions all hold in the observed environment. `observed` answers
/// each axis with one of [`EnvironmentAxis::values`].
pub fn select_variant(
    rules: &[LayoutRule],
    observed: impl Fn(EnvironmentAxis) -> String,
) -> Option<&str> {
    rules
        .iter()
        .find(|rule| rule.matches(&observed))
        .map(|rule| rule.variant.as_str())
}

/// The rules an app uses: the ones it declared, or, when it declared none,
/// the conventional ones for the variants it has — `compact` for a compact
/// size class, `expanded` for an expanded one, `touch` for a coarse pointer,
/// in that order (UI48 §7.2).
pub fn effective_layout_rules(declared: &[LayoutRule], variants: &[String]) -> Vec<LayoutRule> {
    if !declared.is_empty() {
        return declared.to_vec();
    }
    let conventions = [
        ("compact", EnvironmentAxis::SizeClass, "compact"),
        ("expanded", EnvironmentAxis::SizeClass, "expanded"),
        ("touch", EnvironmentAxis::Pointer, "coarse"),
    ];
    conventions
        .into_iter()
        .filter(|(name, _, _)| variants.iter().any(|variant| variant == name))
        .map(|(name, axis, value)| LayoutRule {
            variant: name.to_string(),
            conditions: vec![(axis, value.to_string())],
        })
        .collect()
}

/// One `[[app.layouts]]` entry as written.
#[derive(Debug, Deserialize)]
pub(crate) struct RawLayoutRule {
    variant: String,
    #[serde(flatten)]
    axes: BTreeMap<String, String>,
}

/// Validate `[[app.layouts]]`: a variant name like UI30's filename infix,
/// known axes with known values, no variant twice, and an unconditional rule
/// only at the end (anything after it could never be chosen).
pub(crate) fn validate_layout_rules(raw: Vec<RawLayoutRule>) -> Result<Vec<LayoutRule>, String> {
    let mut rules = Vec::with_capacity(raw.len());
    let mut seen = HashSet::new();
    let count = raw.len();
    for (index, entry) in raw.into_iter().enumerate() {
        if !is_variant_name(&entry.variant) {
            return Err(format!(
                "`{}` is not a layout variant name (lowercase letters, digits and `-`, like the `<Component>.<variant>.mll` infix)",
                entry.variant
            ));
        }
        if !seen.insert(entry.variant.clone()) {
            return Err(format!("variant `{}` has two rules", entry.variant));
        }
        let mut conditions = Vec::with_capacity(entry.axes.len());
        for (key, value) in entry.axes {
            let axis = EnvironmentAxis::from_key(&key).ok_or_else(|| {
                format!(
                    "`{key}` is not an environment axis (one of {})",
                    EnvironmentAxis::ALL.map(EnvironmentAxis::key).join(", ")
                )
            })?;
            if !axis.values().contains(&value.as_str()) {
                return Err(format!(
                    "`{key} = \"{value}\"` can never match (one of {})",
                    axis.values().join(", ")
                ));
            }
            conditions.push((axis, value));
        }
        conditions.sort();
        if conditions.is_empty() && index + 1 != count {
            return Err(format!(
                "variant `{}` has no conditions, so the rules after it could never be chosen",
                entry.variant
            ));
        }
        rules.push(LayoutRule {
            variant: entry.variant,
            conditions,
        });
    }
    Ok(rules)
}

fn is_variant_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 32
        && name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        && !name.starts_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(variant: &str, conditions: &[(EnvironmentAxis, &str)]) -> LayoutRule {
        LayoutRule {
            variant: variant.into(),
            conditions: conditions
                .iter()
                .map(|(axis, value)| (*axis, value.to_string()))
                .collect(),
        }
    }

    /// An environment as a closure: the given axes, UI48's defaults elsewhere.
    fn environment(set: &[(EnvironmentAxis, &str)]) -> impl Fn(EnvironmentAxis) -> String {
        let set: Vec<(EnvironmentAxis, String)> = set
            .iter()
            .map(|(axis, value)| (*axis, value.to_string()))
            .collect();
        move |axis| {
            set.iter()
                .find(|(candidate, _)| *candidate == axis)
                .map(|(_, value)| value.clone())
                .unwrap_or_else(|| {
                    match axis {
                        EnvironmentAxis::SizeClass => "regular",
                        EnvironmentAxis::Pointer => "fine",
                        EnvironmentAxis::Hover => "hover",
                        EnvironmentAxis::Orientation => "landscape",
                        EnvironmentAxis::ColorScheme => "light",
                        EnvironmentAxis::ReducedMotion => "no-preference",
                    }
                    .to_string()
                })
        }
    }

    #[test]
    fn first_matching_rule_wins_and_no_match_is_the_default() {
        use EnvironmentAxis::*;
        let rules = [
            rule("compact", &[(SizeClass, "compact")]),
            rule("touch", &[(Pointer, "coarse")]),
        ];
        let phone = environment(&[(SizeClass, "compact"), (Pointer, "coarse")]);
        assert_eq!(select_variant(&rules, phone), Some("compact"));
        let tablet = environment(&[(Pointer, "coarse")]);
        assert_eq!(select_variant(&rules, tablet), Some("touch"));
        assert_eq!(select_variant(&rules, environment(&[])), None);
    }

    #[test]
    fn every_condition_must_hold() {
        use EnvironmentAxis::*;
        let rules = [rule(
            "phone-portrait",
            &[(Orientation, "portrait"), (SizeClass, "compact")],
        )];
        assert_eq!(
            select_variant(&rules, environment(&[(SizeClass, "compact")])),
            None
        );
        assert_eq!(
            select_variant(
                &rules,
                environment(&[(SizeClass, "compact"), (Orientation, "portrait")])
            ),
            Some("phone-portrait")
        );
    }

    #[test]
    fn conventional_rules_cover_the_variants_that_exist() {
        let variants = vec![
            "touch".to_string(),
            "compact".to_string(),
            "print".to_string(),
        ];
        let rules = effective_layout_rules(&[], &variants);
        assert_eq!(
            rules,
            vec![
                rule("compact", &[(EnvironmentAxis::SizeClass, "compact")]),
                rule("touch", &[(EnvironmentAxis::Pointer, "coarse")]),
            ]
        );
        let declared = [rule("print", &[])];
        assert_eq!(
            effective_layout_rules(&declared, &variants),
            declared.to_vec()
        );
        assert!(effective_layout_rules(&[], &[]).is_empty());
    }

    fn raw(variant: &str, axes: &[(&str, &str)]) -> RawLayoutRule {
        RawLayoutRule {
            variant: variant.into(),
            axes: axes
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }

    #[test]
    fn validation_accepts_well_formed_rules_in_order() {
        let rules = validate_layout_rules(vec![
            raw("compact", &[("size-class", "compact")]),
            raw("touch", &[("pointer", "coarse"), ("hover", "none")]),
            raw("fallback", &[]),
        ])
        .unwrap();
        assert_eq!(rules[0].variant, "compact");
        assert_eq!(
            rules[1].conditions,
            vec![
                (EnvironmentAxis::Pointer, "coarse".to_string()),
                (EnvironmentAxis::Hover, "none".to_string()),
            ]
        );
        assert!(rules[2].conditions.is_empty());
    }

    #[test]
    fn validation_refuses_rules_that_cannot_work() {
        let refused = |rules: Vec<RawLayoutRule>| validate_layout_rules(rules).is_err();
        assert!(refused(vec![raw("Compact", &[("size-class", "compact")])]));
        assert!(refused(vec![raw("compact/../x", &[])]));
        assert!(refused(vec![raw("", &[])]));
        assert!(refused(vec![raw("compact", &[("screen", "small")])]));
        assert!(refused(vec![raw("compact", &[("size-class", "tiny")])]));
        assert!(refused(vec![
            raw("compact", &[("size-class", "compact")]),
            raw("compact", &[("pointer", "coarse")]),
        ]));
        assert!(refused(vec![
            raw("always", &[]),
            raw("touch", &[("pointer", "coarse")])
        ]));
    }
}
