//! native_complete_gate — the Journal app survives native lowering.
//!
//! Follows `mosaic-pkg-note-editor`'s gate: zero capability degradations on
//! all five native backends with no allowlist, and style drops pinned in both
//! directions (a pin that stops occurring fails too, so it cannot become a
//! licence). Both themes are measured.
//!
//! The app composes RecordList and DraftEditor, so their pinned drops show
//! up here too; the list is measured, and each pin is checked both ways.

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

/// Style properties this package is currently known to lose, by backend.
/// Every entry is measured and the inverse-ratchet test rejects stale pins.
const ALLOWED_STYLE_DROPS: &[(Backend, &str)] = &[
    // ---- Sizing the editor (#14416, found by rendering on Compose) ----
    // The title and body fields fill the editor (`width: 100%`) and the body
    // has a usable height (`min-height`); rendering Journal on Compose showed
    // an empty entry as a 120px-wide box. The web, SwiftUI and XAML lower
    // both. Compose lowers `min-height` but silently ignores a percentage
    // `width` (not reported, so not pinned; lowering it safely in a Row is a
    // separate emitter change). Qt and Flutter lower neither on a text field
    // yet. Checked both ways below.
    (Backend::Qt, "width"),
    (Backend::Qt, "min-height"),
    (Backend::Flutter, "width"),
    (Backend::Flutter, "min-height"),
    // ---- Flutter (#12022) ----
    // Measured on first build, in both themes. None is on this package's own
    // parts (its styles use only properties Flutter lowers); all come from the
    // components it composes, which pin the same drops themselves:
    //
    // - `border-radius` on DraftEditor's two text fields;
    // - `color` on DraftEditor's labels, and on EmptyState's and RecordList's
    //   secondary `Text`s;
    // - `font-size` on DraftEditor's action buttons and RecordList's row
    //   titles.
    (Backend::Flutter, "border-radius"),
    (Backend::Flutter, "color"),
    (Backend::Flutter, "font-size"),
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
