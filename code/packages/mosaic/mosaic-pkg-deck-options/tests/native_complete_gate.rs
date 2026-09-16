//! native_complete_gate — this component package survives native lowering.
//!
//! Engram's UI is these eleven packages. Until now none of them had a native
//! gate of their own: `package_compiles.rs` proves the sources round-trip
//! through the three IR compilers and deliberately says nothing about what a
//! backend can express, so a capability this package asks for and a backend
//! cannot provide was invisible here.
//!
//! It was not invisible everywhere — `engram-app`'s gate asserts the composed
//! app emits with zero degradations. But that covers the parts Engram mounts,
//! in the configuration Engram mounts them, and reports one signal for eleven
//! packages at once. A failure here names one package, one backend, one
//! property, and catches a regression in a component before an app composes it.
//!
//! Follows `mosaic-pkg-toolkit`'s gate (#12024), including its lesson: an
//! allowlist that is never checked for staleness becomes a licence rather than
//! a record, so this one is asserted in both directions.
//!
//! ## Capability degradations: zero, with no allowlist
//!
//! Measured across all five native backends before this file was written. The
//! toolkit needs an allowlist because its `Radio` binds `group:` from a slot,
//! which no backend can resolve at compile time. This package has no such case,
//! so the assertion is zero and an allowlist would only be somewhere for a
//! regression to hide.
//!
//! ## Style drops: zero, on the three backends that report them
//!
//! Also measured rather than assumed. `style_degradations` sits outside
//! `native_complete` by design — some drops are accepted platform limits — and
//! this package currently has none, so the strictest assertion is the true one.
//!
//! **Narrower than it looks, and the first draft of this comment overstated
//! it.** Only XAML, SwiftUI and Compose populate `style_degradations`; Qt and
//! Flutter fall through the analyzer's `_ => {}`, whose own comment says an
//! empty list there means "nobody looked" rather than "nothing was lost"
//! (#12022). So this half covers three of the five. The capability assertion
//! above covers all five, because `collect_native_degradations` is
//! backend-generic.

use mosaic_package_artifact_builder::{
    analyze_package_degradations, Backend, BuildOptions, BuildProfile,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// The five backends that produce a real native application.
///
/// React, HTML, WebComponent and Electron are excluded on purpose: they report
/// `profile.backend-not-native` by design, so including them would assert a
/// failure rather than guard against one.
const NATIVE_BACKENDS: &[Backend] = &[
    Backend::Qt,
    Backend::SwiftUI,
    Backend::Compose,
    Backend::Flutter,
    Backend::Xaml,
];

/// Deliberately empty, and that is an assertion rather than an omission.
///
/// This package drops nothing on the three backends that report style drops.
/// An entry here would mean a property was lost, so the empty list is the gate:
/// an empty allowlist tolerates nothing, rather than checking nothing.
const ALLOWED_STYLE_DROPS: &[(Backend, &str)] = &[    // ---- Qt (#15245) ----
    //
    // These became visible the moment Qt started reporting its drops in
    // #15245. They are PRE-EXISTING gaps, not regressions: this package has
    // been rendering without them on Qt since the parts were authored, and
    // nothing could see them because the backend reported no drops at all.
    //
    // Pinned so the gate stays honest, NOT to bless them -- see this list's
    // doc comment on how a stale pin turns into a standing licence.
    //
    // Qt lowers font size only where a primitive has native typography
    // (`has_native_font_size`); other parts drop it.
    (Backend::Qt, "font-size"),
    //
    // #15247 -- `qml_padding` reads ONE value and fans it to all four
    // edges, so any longhand beyond the one it picks is lost.
    (Backend::Qt, "padding-bottom"),
];

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Both themes, which is NOT what `theme: None` alone gives you.
///
/// `None` selects the historical dark-wins default, so a property dropped only
/// by the light stylesheet is invisible to it. Measured rather than assumed:
/// adding `flex-wrap` to this package's LIGHT `.msl` left the analyzer
/// reporting nothing at all, and the identical line in the DARK `.msl` produced
/// two drops.
///
/// The sibling gates in `mosaic-pkg-toolkit` and `engram-app` both pass `None`
/// and therefore cover one theme each. Noted separately rather than widened
/// here.
const THEMES: &[Option<&str>] = &[None, Some("light")];

fn report_for(
    backend: Backend,
    theme: Option<&str>,
) -> mosaic_package_artifact_builder::DegradationReport {
    let out = TempDir::new().expect("temp dir");
    analyze_package_degradations(
        &BuildOptions {
            package_root: package_root(),
            output_root: out.path().to_path_buf(),
            backend,
            emit_project: false,
            theme: theme.map(str::to_string),
        },
        BuildProfile::NativeComplete,
    )
    .unwrap_or_else(|error| {
        panic!("degradation analysis failed for {backend:?}/{theme:?}: {error}")
    })
}

#[test]
fn the_package_is_native_complete_on_every_backend() {
    let mut failures = Vec::new();

    for &backend in NATIVE_BACKENDS {
        for &theme in THEMES {
            for entry in &report_for(backend, theme).degradations {
                failures.push(format!(
                    "  {backend:?} ({}): {} — {} at {}",
                    theme.unwrap_or("dark"),
                    entry.component,
                    entry.code,
                    entry.layout_path
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} capability/capabilities lost when lowering this package:\n{}\n\n\
         Each line is a place a backend could not express something the layout \
         asks for — it still emits and still compiles, it just quietly does \
         less there. Fix the emitter, or author something expressible. Adding \
         an allowlist here would hide exactly the regression this catches.",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn no_style_property_is_dropped_beyond_the_pinned_set() {
    let mut unexpected = Vec::new();

    for &backend in NATIVE_BACKENDS {
        for &theme in THEMES {
            for entry in &report_for(backend, theme).style_degradations {
                // A drop that does not name its property cannot be matched
                // against the list, so it stays unexpected rather than
                // slipping through.
                let allowed = entry.primitive.as_deref().is_some_and(|property| {
                    ALLOWED_STYLE_DROPS
                        .iter()
                        .any(|(b, p)| *b == backend && *p == property)
                });
                if !allowed {
                    unexpected.push(format!(
                        "  {backend:?} ({}): {} at {} — {}",
                        theme.unwrap_or("dark"),
                        entry.primitive.as_deref().unwrap_or("<unnamed>"),
                        entry.layout_path,
                        entry.reason
                    ));
                }
            }
        }
    }

    assert!(
        unexpected.is_empty(),
        "{} style property/properties dropped that this package does not \
         pin:\n{}\n\nEither map the property in that backend's emitter, or \
         add it to ALLOWED_STYLE_DROPS with a reason.",
        unexpected.len(),
        unexpected.join("\n")
    );
}

/// Every pinned drop is still a drop.
///
/// The test above runs one way: it catches a NEW drop and cannot catch a pinned
/// one that stopped happening. A fixed-but-still-listed entry is not untidy —
/// it is a standing licence for that property to be dropped again with this
/// gate green. Engram's own copy of this pattern had 11 of 21 pins in exactly
/// that state before it was checked.
///
/// Vacuous when the list is empty, which is the correct behaviour: an empty
/// list licenses nothing, so there is nothing to go stale.
#[test]
fn no_pinned_style_drop_has_silently_been_fixed() {
    let mut observed: Vec<(Backend, String)> = Vec::new();
    for &backend in NATIVE_BACKENDS {
        for &theme in THEMES {
            for entry in &report_for(backend, theme).style_degradations {
                if let Some(property) = entry.primitive.as_deref() {
                    observed.push((backend, property.to_string()));
                }
            }
        }
    }

    let stale: Vec<String> = ALLOWED_STYLE_DROPS
        .iter()
        .filter(|(backend, property)| {
            !observed
                .iter()
                .any(|(seen, seen_property)| seen == backend && seen_property == property)
        })
        .map(|(backend, property)| format!("  {backend:?}: {property}"))
        .collect();

    assert!(
        stale.is_empty(),
        "{} pinned style drop(s) no longer occur:\n{}\n\nThe emitter learned \
         the mapping, so delete these — left in place they stop recording a \
         known gap and start licensing its return.",
        stale.len(),
        stale.join("\n")
    );
}
