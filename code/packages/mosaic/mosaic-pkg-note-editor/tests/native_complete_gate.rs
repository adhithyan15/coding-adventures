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
//! ## Style drops: pinned exactly, in both directions
//!
//! Also measured rather than assumed. `style_degradations` sits outside
//! `native_complete` by design — some drops are accepted platform limits — so
//! each one this package has is pinned, an unpinned one fails, and a pin that
//! stops occurring fails too. When this gate was written the package had none
//! on the three backends that then reported drops; Qt and Flutter reporting
//! later found one (Flutter `font-weight`), and 0.2.0 added four more.
//!
//! The style-drop half covers all five native backends: XAML, SwiftUI, Compose,
//! Qt and Flutter each populate `style_degradations`. The capability assertion
//! above is independently backend-generic.
//!
//! ## 0.2.0: what mounting DraftEditor costs, measured (E4)
//!
//! Since 0.2.0 the focused field is a body-only
//! `pkg::mosaic-pkg-draft-editor::DraftEditor`. DraftEditor's own gate is
//! clean at its own root, and the editor convergence audit's R2 found that
//! that does NOT certify it when nested: mounted inside NoteEditor it loses
//! two things its own gate never sees. So this gate measures the composed
//! package, and its pins name the **part** as well as the backend and
//! property, as `mosaic-pkg-notes`' E2 gate does. A `(Flutter, "width")` pin
//! would license every part of the package to drop `width`.
//!
//! The measured set, identical in both themes (0.1.0 had only the first):
//!
//! ```text
//!                  font-weight    gap                    min-height  width
//!                  editor-title   draft-editor-          draft-editor-body
//!                                 title-group
//!   SwiftUI                         x                                         1
//!   Qt                                                    x                   1
//!   Xaml                                                                      0
//!   Flutter          x                                    x           x       3
//!   Compose                                                                   0
//!                                                                            --
//!                                                                             5
//! ```
//!
//! Why each is pinned rather than fixed (audit §8, D4):
//!
//! - **SwiftUI `gap` on `draft-editor-title-group`** is a reporting artifact.
//!   `title-label : ""` makes the title group's `If` a constant, so the
//!   package inliner folds it away and the part never appears in the emitted
//!   Swift. The SwiftUI reporter counts the style of a part with no
//!   occurrence as "not lowered". Nothing on screen is missing.
//! - **Flutter `width` on `draft-editor-body`** is a real constraint. The
//!   field sits under NoteEditor's `Row [ editor-body ]`, whose children get
//!   unbounded width, and Flutter can fill 100% only of a bounded parent, so
//!   the emitter declines rather than emit a layout that throws. The 0.1.0
//!   field asked for no width, so it is no narrower than before.
//! - **Qt and Flutter `min-height` on `draft-editor-body`** is the drop
//!   DraftEditor's own gate pins: neither lowers `min-height` on a text
//!   field yet.

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
const NATIVE_BACKENDS: &[Backend] = &Backend::NATIVE;

/// Style properties this package is currently known to lose, by backend,
/// property and part. Every entry is measured (both themes), and the
/// inverse-ratchet test rejects stale pins.
const ALLOWED_STYLE_DROPS: &[(Backend, &str, &str)] = &[
    // ---- NoteEditor's own parts (#12022) ----
    // This pre-existing loss became measurable when Flutter gained drop
    // reporting: the heading's weight (800) has no lowering on that text.
    (Backend::Flutter, "font-weight", "editor-title"),
    // ---- The focused field, mounted DraftEditor (0.2.0, E4) ----
    // A part that is folded away at compile time (title-label is ""),
    // reported as unlowered. See the header: nothing on screen is missing.
    (Backend::SwiftUI, "gap", "draft-editor-title-group"),
    // DraftEditor's own pinned gap: no `min-height` on a text field.
    (Backend::Qt, "min-height", "draft-editor-body"),
    (Backend::Flutter, "min-height", "draft-editor-body"),
    // `width: 100%` under NoteEditor's Row, which bounds no width.
    (Backend::Flutter, "width", "draft-editor-body"),
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

fn theme_name(theme: Option<&str>) -> &str {
    theme.unwrap_or("dark")
}

/// The part a style drop was reported on.
///
/// The analyzer names a style drop's location `$style.<part>`. Anything else
/// returns `None`, so it cannot match a pin and stays unexpected rather than
/// slipping through.
fn part_of(layout_path: &str) -> Option<&str> {
    layout_path.strip_prefix("$style.")
}

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
                    theme_name(theme),
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
                // A drop that does not name its property or its part cannot
                // be matched against the list, so it stays unexpected.
                let allowed = match (entry.primitive.as_deref(), part_of(&entry.layout_path)) {
                    (Some(property), Some(part)) => ALLOWED_STYLE_DROPS
                        .iter()
                        .any(|(b, p, q)| *b == backend && *p == property && *q == part),
                    _ => false,
                };
                if !allowed {
                    unexpected.push(format!(
                        "  {backend:?} ({}): {} at {} — {}",
                        theme_name(theme),
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

/// Every pinned drop is still a drop, in every theme.
///
/// The test above runs one way: it catches a NEW drop and cannot catch a pinned
/// one that stopped happening. A fixed-but-still-listed entry is not untidy —
/// it is a standing licence for that property to be dropped again with this
/// gate green. Engram's own copy of this pattern had 11 of 21 pins in exactly
/// that state before it was checked.
///
/// Checked per theme, not over the union of both: the five drops occur in each
/// theme today, so a pin that one theme stopped producing is already a change
/// worth seeing.
#[test]
fn no_pinned_style_drop_has_silently_been_fixed() {
    let mut stale = Vec::new();

    for &theme in THEMES {
        let mut observed: Vec<(Backend, String, String)> = Vec::new();
        for &backend in NATIVE_BACKENDS {
            for entry in &report_for(backend, theme).style_degradations {
                if let (Some(property), Some(part)) =
                    (entry.primitive.as_deref(), part_of(&entry.layout_path))
                {
                    observed.push((backend, property.to_string(), part.to_string()));
                }
            }
        }

        for (backend, property, part) in ALLOWED_STYLE_DROPS {
            let still_dropped = observed
                .iter()
                .any(|(b, p, q)| b == backend && p == property && q == part);
            if !still_dropped {
                stale.push(format!(
                    "  {backend:?} ({}): {property} on {part}",
                    theme_name(theme)
                ));
            }
        }
    }

    assert!(
        stale.is_empty(),
        "{} pinned style drop(s) no longer occur:\n{}\n\nThe emitter learned \
         the mapping (or the layout stopped asking for it), so delete these — \
         left in place they stop recording a known gap and start licensing its \
         return.",
        stale.len(),
        stale.join("\n")
    );
}

/// The pin list itself has no duplicates, so its length is the drop count.
///
/// Together with the two tests above this makes `ALLOWED_STYLE_DROPS.len()`
/// the package's exact per-theme drop count: 1 in 0.1.0, 5 since E4 mounted
/// DraftEditor. That is the number an emitter fix for any of them moves.
#[test]
fn the_pin_list_names_each_drop_once() {
    for (i, pin) in ALLOWED_STYLE_DROPS.iter().enumerate() {
        assert!(
            !ALLOWED_STYLE_DROPS[i + 1..].contains(pin),
            "{pin:?} is pinned twice"
        );
    }
}
