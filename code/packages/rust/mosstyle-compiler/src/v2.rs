//! # mosstyle v2: the closed, typed property table (UI90 §3.1–§3.2)
//!
//! mosstyle v1 accepts any property name and passes every value through as a
//! string. In practice that made it CSS: an audit of all 186 `.msl` files
//! found 82 distinct properties against about 25 that the language described,
//! and every emitter grew its own interpreter for whatever CSS happened to
//! arrive (UI90 §2).
//!
//! v2 closes the language. The compiler knows every property, the type of its
//! value, and nothing else gets through:
//!
//! ```text
//!   .msl declaration          v2 check                          result
//!   ─────────────────────     ──────────────────────────────    ─────────────────
//!   gap: 12                   gap is a Length; 12 is one         ok
//!   gap: 12px                 units are not part of a Length     TypeMismatch
//!   corner: full              corner is Length | full            ok
//!   border-radius: 50%        not a property; use `corner`       UnknownProperty
//!   color: #1E1E1E / 0.6      Color, with alpha                  ok
//!   color: rgba(0,0,0,.6)     CSS function                       TypeMismatch
//! ```
//!
//! This module only *checks*. It does not change what any emitter receives,
//! so running it can never change generated output. Whether a finding is an
//! error or a warning is the caller's decision: a package that opts in with
//! `[style] version = 2` fails on these, and a v1 package reports them as
//! warnings so it can see what migrating would take (UI90 §4 step 1).
//!
//! It runs on a [`StyleDef`] after token resolution. A token's value is
//! checked where it is used, against the type of the property it lands in,
//! which is the only place the type is known. This means the built-in v1
//! palette (`4px`, `rgba(…)`) does not satisfy v2 until UI90 §6 step 3
//! replaces it with typed system tokens.
//!
//! Not checked here yet: whether a property belongs on the part it is set on
//! (text properties on text parts, UI90 §3.5). That needs the layout's part
//! kinds, which a `.msl` alone does not carry.

use crate::{CompileError, ErrorKind, StyleDef, StyleProp};

/// The type of a property's value (UI90 §3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    /// `#RRGGBB`, or `#RRGGBB / A` with an alpha `A` from 0 to 1.
    Color,
    /// A unitless, non-negative number in device-independent units.
    Length,
    /// A [`ValueType::Length`], or the keyword `fill`.
    LengthOrFill,
    /// A [`ValueType::Length`], or the keyword `full`.
    LengthOrFull,
    /// A number from 0 to 1.
    UnitInterval,
    /// A number of at least 0.
    NonNegativeNumber,
    /// A whole number of at least 1.
    PositiveInteger,
    /// A number of degrees, possibly negative, with no unit.
    Angle,
    /// One of a closed set of keywords.
    OneOf(&'static [&'static str]),
    /// A [`ValueType::Color`], or one of a closed set of keywords.
    ColorOr(&'static [&'static str]),
}

/// Material 3's type scale, which `type-role` names (UI90 §5.2).
pub const TYPE_ROLES: &[&str] = &[
    "display-large",
    "display-medium",
    "display-small",
    "headline-large",
    "headline-medium",
    "headline-small",
    "title-large",
    "title-medium",
    "title-small",
    "body-large",
    "body-medium",
    "body-small",
    "label-large",
    "label-medium",
    "label-small",
];

const EDGES: &[&str] = &["top", "right", "bottom", "left"];
const CORNERS: &[&str] = &["top-left", "top-right", "bottom-right", "bottom-left"];

/// The type of `name`'s value, or `None` when v2 has no such property.
///
/// This is UI90 §3.1's table, row for row. The per-edge and per-corner
/// longhands are matched by shape rather than listed one by one.
pub fn property_type(name: &str) -> Option<ValueType> {
    use ValueType::*;
    let t = match name {
        // Paint
        "color" | "background" => Color,
        "opacity" => UnitInterval,
        // Geometry and border
        "padding" => Length,
        "width" | "height" | "min-width" | "min-height" | "max-width" | "max-height" => {
            LengthOrFill
        }
        "corner" => LengthOrFull,
        "border-width" => Length,
        "border-color" => Color,
        "border-stroke" => OneOf(&["solid", "dashed", "dotted"]),
        // Text
        "type-role" => OneOf(TYPE_ROLES),
        "font-size" => Length,
        "font-weight" => OneOf(&["regular", "medium", "semibold", "bold"]),
        // Brand faces (UI90 §5.4) join this list with the token work of §6
        // step 3; until then only the three system families are accepted.
        "font" => OneOf(&["system", "mono", "serif"]),
        "tracking" => OneOf(&["tight", "normal", "wide"]),
        "text-case" => OneOf(&["none", "upper", "lower"]),
        "text-align" => OneOf(&["start", "center", "end"]),
        "max-lines" => PositiveInteger,
        "overflow-text" => OneOf(&["clip", "ellipsis"]),
        // Layout
        "gap" => Length,
        "align-main" => OneOf(&["start", "center", "end", "space-between"]),
        "align-cross" => OneOf(&["start", "center", "end", "stretch"]),
        "grow" => NonNegativeNumber,
        "shrink" => OneOf(&["yes", "no"]),
        // Effects and state
        "elevation" => OneOf(&["none", "raised", "overlay"]),
        "focus-ring" => ColorOr(&["system", "none"]),
        "rotation" => Angle,
        "motion" => OneOf(&["none", "subtle", "standard"]),
        _ => return longhand_type(name),
    };
    Some(t)
}

/// `padding-top`, `border-left-color`, `corner-top-right` and the like.
fn longhand_type(name: &str) -> Option<ValueType> {
    if let Some(edge) = name.strip_prefix("padding-") {
        return EDGES.contains(&edge).then_some(ValueType::Length);
    }
    if let Some(corner) = name.strip_prefix("corner-") {
        return CORNERS.contains(&corner).then_some(ValueType::LengthOrFull);
    }
    let rest = name.strip_prefix("border-")?;
    let (edge, what) = rest.split_once('-')?;
    if !EDGES.contains(&edge) {
        return None;
    }
    match what {
        "width" => Some(ValueType::Length),
        "color" => Some(ValueType::Color),
        _ => None,
    }
}

/// What to write instead of a v1 property that v2 does not have, when there
/// is a direct answer (UI90 §3.1's "Replaces" column and the "Moved out of
/// `.msl`" list).
pub fn replacement_hint(name: &str) -> Option<&'static str> {
    let hint = match name {
        "border-radius" => "`corner` (`50%` becomes `corner: full`)",
        "border" => "`border-width`, `border-color` and `border-stroke`",
        "border-style" => "`border-stroke`",
        "text-transform" => "`text-case`",
        "letter-spacing" => "`tracking`",
        "white-space" => "`max-lines`",
        "text-overflow" | "overflow" => "`overflow-text`",
        "justify-content" => "`align-main`",
        "align-items" => "`align-cross`",
        "align" => "`align-main` and `align-cross` (#16293)",
        "flex-grow" | "flex" => "`grow`",
        "flex-shrink" => "`shrink`",
        "box-shadow" => "`elevation`",
        "outline" | "outline-offset" => "`focus-ring`",
        "transform" => "`rotation`",
        "transition" => "`motion`",
        "font-family" => "`font`",
        "position" | "top" | "left" | "right" | "bottom" | "z-index" => {
            "a `Stack` placement or a sticky role in the `.mll`; this is structure, not style"
        }
        "display" | "flex-direction" | "flex-wrap" | "box-sizing" => {
            "a `Row`, `Column` or `Wrap` container in the `.mll`; this is structure, not style"
        }
        "cursor" => "nothing: the cursor follows from the part's role",
        "color-scheme" => "nothing: the theme mode sets it (UI90 §3.3)",
        _ if name.starts_with("border-") && name.ends_with("-style") => "`border-stroke`",
        _ if name.starts_with("border-") && name.ends_with("-radius") => {
            "a per-corner `corner-…` property"
        }
        _ => return None,
    };
    Some(hint)
}

/// Whether `value` is a well-formed value of type `t`.
pub fn value_matches(t: ValueType, value: &str) -> bool {
    let value = value.trim();
    match t {
        ValueType::Color => is_color(value),
        ValueType::Length => is_length(value),
        ValueType::LengthOrFill => value == "fill" || is_length(value),
        ValueType::LengthOrFull => value == "full" || is_length(value),
        ValueType::UnitInterval => number(value).is_some_and(|n| (0.0..=1.0).contains(&n)),
        ValueType::NonNegativeNumber => number(value).is_some_and(|n| n >= 0.0),
        ValueType::PositiveInteger => {
            !value.is_empty()
                && value.bytes().all(|b| b.is_ascii_digit())
                && value.parse::<u32>().is_ok_and(|n| n >= 1)
        }
        ValueType::Angle => number(value).is_some(),
        ValueType::OneOf(words) => words.contains(&value),
        ValueType::ColorOr(words) => words.contains(&value) || is_color(value),
    }
}

/// A plain decimal: optional `-`, digits, optional `.digits`. No exponents,
/// units, `+`, or leading `.`, so every backend can read it the same way.
fn number(value: &str) -> Option<f64> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    let (whole, fraction) = match digits.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (digits, None),
    };
    let all_digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !all_digits(whole) || fraction.is_some_and(|f| !all_digits(f)) {
        return None;
    }
    value.parse::<f64>().ok().filter(|n| n.is_finite())
}

fn is_length(value: &str) -> bool {
    number(value).is_some_and(|n| n >= 0.0)
}

/// `#RRGGBB` or `#RRGGBB / A` (UI90 §3.2). Named colours, `currentColor`,
/// 3- and 8-digit hex, and CSS functions such as `rgba()` are not colours.
fn is_color(value: &str) -> bool {
    let (hex, alpha) = match value.split_once('/') {
        Some((hex, alpha)) => (hex.trim_end(), Some(alpha.trim_start())),
        None => (value, None),
    };
    let rgb = hex
        .strip_prefix('#')
        .is_some_and(|digits| digits.len() == 6 && digits.bytes().all(|b| b.is_ascii_hexdigit()));
    rgb && alpha.is_none_or(|a| number(a).is_some_and(|n| (0.0..=1.0).contains(&n)))
}

/// Every v2 finding in `def`: `UnknownProperty` for a name the table does not
/// have, `TypeMismatch` for a value of the wrong type. Base styles and every
/// state override are checked; findings come back in source order.
pub fn check(def: &StyleDef) -> Vec<CompileError> {
    let mut findings = Vec::new();
    for part in &def.parts {
        let declarations =
            part.base
                .iter()
                .map(|prop| (None, prop))
                .chain(part.states.iter().flat_map(|state| {
                    state
                        .props
                        .iter()
                        .map(move |prop| (Some(&state.state), prop))
                }));
        for (state, prop) in declarations {
            if let Some(finding) = check_prop(&part.name, state.map(String::as_str), prop) {
                findings.push(finding);
            }
        }
    }
    findings
}

fn check_prop(part: &str, state: Option<&str>, prop: &StyleProp) -> Option<CompileError> {
    let place = match state {
        Some(state) => format!("part '{part}', state '{state}'"),
        None => format!("part '{part}'"),
    };
    let Some(t) = property_type(&prop.name) else {
        let hint = replacement_hint(&prop.name)
            .map(|h| format!("; use {h}"))
            .unwrap_or_default();
        return Some(CompileError {
            kind: ErrorKind::UnknownProperty,
            message: format!(
                "'{}' is not a mosstyle v2 property ({place}){hint}",
                prop.name
            ),
        });
    };
    if value_matches(t, &prop.value) {
        return None;
    }
    Some(CompileError {
        kind: ErrorKind::TypeMismatch,
        message: format!(
            "'{}: {}' ({place}) expects {}",
            prop.name,
            prop.value,
            describe(t)
        ),
    })
}

/// How a type reads in a message.
pub fn describe(t: ValueType) -> String {
    match t {
        ValueType::Color => "a colour: #RRGGBB or #RRGGBB / A, with A from 0 to 1".into(),
        ValueType::Length => "a length: a plain number with no unit".into(),
        ValueType::LengthOrFill => "a length (a plain number with no unit) or `fill`".into(),
        ValueType::LengthOrFull => "a length (a plain number with no unit) or `full`".into(),
        ValueType::UnitInterval => "a number from 0 to 1".into(),
        ValueType::NonNegativeNumber => "a number of at least 0".into(),
        ValueType::PositiveInteger => "a whole number of at least 1".into(),
        ValueType::Angle => "an angle: a plain number of degrees".into(),
        ValueType::OneOf(words) => format!("one of: {}", words.join(", ")),
        ValueType::ColorOr(words) => format!("a colour, or one of: {}", words.join(", ")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PartStyle, StateStyle};

    fn prop(name: &str, value: &str) -> StyleProp {
        StyleProp {
            name: name.into(),
            value: value.into(),
        }
    }

    fn def(base: Vec<StyleProp>) -> StyleDef {
        StyleDef {
            component_name: "Probe".into(),
            parts: vec![PartStyle {
                name: "root".into(),
                base,
                transitions: vec![],
                states: vec![],
            }],
        }
    }

    #[test]
    fn every_table_row_accepts_a_well_typed_value() {
        let good = def(vec![
            prop("color", "#1E1E1E"),
            prop("background", "#ffffff / 0.5"),
            prop("opacity", "0.25"),
            prop("padding", "12"),
            prop("padding-top", "4.5"),
            prop("width", "fill"),
            prop("max-height", "640"),
            prop("corner", "full"),
            prop("corner-top-left", "6"),
            prop("border-width", "1"),
            prop("border-left-width", "2"),
            prop("border-color", "#000000"),
            prop("border-bottom-color", "#000000 / 1"),
            prop("border-stroke", "dashed"),
            prop("type-role", "title-medium"),
            prop("font-size", "15"),
            prop("font-weight", "semibold"),
            prop("font", "mono"),
            prop("tracking", "wide"),
            prop("text-case", "upper"),
            prop("text-align", "start"),
            prop("max-lines", "2"),
            prop("overflow-text", "ellipsis"),
            prop("gap", "0"),
            prop("align-main", "space-between"),
            prop("align-cross", "stretch"),
            prop("grow", "1"),
            prop("shrink", "no"),
            prop("elevation", "raised"),
            prop("focus-ring", "system"),
            prop("rotation", "-90"),
            prop("motion", "subtle"),
        ]);
        assert_eq!(check(&good), vec![]);
        assert_eq!(TYPE_ROLES.len(), 15);
    }

    #[test]
    fn css_values_are_type_mismatches() {
        for (name, value) in [
            ("gap", "12px"),
            ("gap", "-4"),
            ("gap", ".5"),
            ("gap", "1e3"),
            ("width", "100%"),
            ("height", "100vh"),
            ("corner", "50%"),
            ("color", "rgba(0,0,0,0.6)"),
            ("color", "red"),
            ("color", "currentColor"),
            ("color", "#fff"),
            ("color", "#11223344"),
            ("color", "#112233 / 1.5"),
            ("color", "#112233 /"),
            ("opacity", "1.2"),
            ("font-weight", "600"),
            ("font-weight", "SemiBold"),
            ("font", "\"Inter\", system-ui"),
            ("text-align", "left"),
            ("max-lines", "0"),
            ("max-lines", "1.5"),
            ("grow", "-1"),
            ("shrink", "0"),
            ("rotation", "45deg"),
            ("focus-ring", "2px solid blue"),
            ("type-role", "body"),
        ] {
            let findings = check(&def(vec![prop(name, value)]));
            assert_eq!(findings.len(), 1, "{name}: {value}");
            assert_eq!(findings[0].kind, ErrorKind::TypeMismatch, "{name}: {value}");
            assert!(
                findings[0].message.contains(value),
                "{}",
                findings[0].message
            );
        }
    }

    #[test]
    fn css_properties_are_unknown_and_say_what_replaces_them() {
        for (name, hint) in [
            ("border-radius", "corner"),
            ("box-shadow", "elevation"),
            ("letter-spacing", "tracking"),
            ("text-transform", "text-case"),
            ("flex-grow", "grow"),
            ("align", "align-main"),
            ("border-top-style", "border-stroke"),
            ("position", ".mll"),
            ("display", ".mll"),
            ("cursor", "role"),
        ] {
            let findings = check(&def(vec![prop(name, "x")]));
            assert_eq!(findings.len(), 1, "{name}");
            assert_eq!(findings[0].kind, ErrorKind::UnknownProperty, "{name}");
            assert!(
                findings[0].message.contains(hint),
                "{}",
                findings[0].message
            );
        }
        let plain = check(&def(vec![prop("made-up", "1")]));
        assert_eq!(plain[0].kind, ErrorKind::UnknownProperty);
        assert!(!plain[0].message.contains("; use"), "{}", plain[0].message);
        for name in [
            "padding-middle",
            "border-middle-width",
            "border-top-radius-x",
            "corner-center",
        ] {
            assert!(property_type(name).is_none(), "{name}");
        }
    }

    #[test]
    fn state_overrides_are_checked_and_named() {
        let mut d = def(vec![prop("color", "#000000")]);
        d.parts[0].states.push(StateStyle {
            state: "hover".into(),
            slot: None,
            slot_is_bool: false,
            props: vec![prop("background", "rgba(0,0,0,.1)")],
            transitions: vec![],
        });
        let findings = check(&d);
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0].message.contains("state 'hover'"),
            "{}",
            findings[0].message
        );
    }
}
