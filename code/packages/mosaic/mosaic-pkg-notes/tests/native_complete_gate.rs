//! native_complete_gate — Notes on the five native backends.
//!
//! Follows `mosaic-pkg-draft-editor`'s gate: zero capability degradations on
//! all five native backends with no allowlist, and style drops pinned in both
//! directions (a pin that stops occurring fails too, so it cannot become a
//! licence). Both themes are measured.
//!
//! Why this gate exists (E2 of `mosaic-editor-convergence-audit-v1.md`): the
//! audit measured Notes 0.2.0 at 0 capability degradations but **12 style
//! drops per theme that nothing recorded**. E2 pinned those 12 so that E3 —
//! Notes 0.3.0, re-founded on `pkg::mosaic-pkg-toolkit::RecordList` and
//! `pkg::mosaic-pkg-draft-editor::DraftEditor` — would be *measured* rather
//! than claimed. E3 took the count to 10, as the audit's probe C predicted:
//!
//! - **removed (2 pins):** Qt `text-align` on `notes-row-on` and on
//!   `notes-row-off`. Those row buttons are gone; RecordList's rows ask for
//!   no `text-align`, so nothing replaces them.
//! - **moved (2 pins):** the body's `min-height` on Qt
//!   and Flutter. The body is now DraftEditor's `draft-editor-body`, which
//!   DraftEditor's own gate pins for the same reason, so the drop moves part
//!   rather than going away.
//! - **kept (8 pins):** Notes' own list/editor flex split, which E3 does
//!   not touch. PR-6 (`HostNavigationSplit`) removes it.
//!
//! The composed package adds no new drop: mounted here, neither RecordList
//! nor DraftEditor loses anything its own gate does not already record (the
//! audit's R2 found nested drops in `NoteEditor`'s context, not this one).
//!
//! One deliberate difference from DraftEditor's gate: a pin here names the
//! **part** as well as the backend and property. Eight of the drops are
//! `flex-grow`/`flex-shrink` on three different parts, so a
//! `(backend, property)` pin would let a fourth part start dropping
//! `flex-grow` with this gate green. So the pin set is the exact drop set:
//! 10 entries, and every one is required in each theme.
//!
//! The measured set, identical in both themes:
//!
//! ```text
//!              flex-grow       flex-shrink   min-height
//!              editor  empty   list          draft-editor-body
//!   SwiftUI      x       x      x                                3
//!   Qt           x       x      x            x                   4
//!   Xaml                        x                                1
//!   Flutter                     x            x                   2
//!   Compose                                                      0
//!                                                               --
//!                                                               10
//! ```

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
    // ---- The list/editor split: flex-grow and flex-shrink (3 parts) ----
    // `notes-body` is a Row: a fixed 220px list that must not shrink
    // (`flex-shrink: 0` on `notes-list`) beside an editor, or the empty-state
    // hint, that takes the rest (`flex-grow: 1` on `notes-editor` /
    // `notes-empty`). Native toolkits distribute space where the child is
    // built (SwiftUI `layoutPriority`/`Spacer`, Qt `Layout.fillWidth`,
    // Flutter's parent-owned `Expanded`), not as a modifier on a finished
    // element, so these flex hints are lost. Compose lowers all three. The
    // audit assigns their removal to PR-6 (`HostNavigationSplit`), not E3.
    (Backend::SwiftUI, "flex-grow", "notes-editor"),
    (Backend::SwiftUI, "flex-grow", "notes-empty"),
    (Backend::SwiftUI, "flex-shrink", "notes-list"),
    (Backend::Qt, "flex-grow", "notes-editor"),
    (Backend::Qt, "flex-grow", "notes-empty"),
    (Backend::Qt, "flex-shrink", "notes-list"),
    // XAML and Flutter report no `flex-grow` drop, but neither consumes
    // "never shrink" (XAML has no setter mapped; Flutter's child wrapper
    // does not read it on this occurrence).
    (Backend::Xaml, "flex-shrink", "notes-list"),
    (Backend::Flutter, "flex-shrink", "notes-list"),
    // ---- The body's usable height: min-height on the text area ----
    // `min-height: 160` (DraftEditor's theme) keeps the multi-line body from
    // collapsing to one line. Qt and Flutter do not lower `min-height` on a
    // text field yet. The body is DraftEditor's since 0.3.0, so this is the
    // drop DraftEditor's own gate pins, on DraftEditor's part; 0.2.0 pinned
    // the same gap on Notes' own `notes-body-input`.
    (Backend::Qt, "min-height", "draft-editor-body"),
    (Backend::Flutter, "min-height", "draft-editor-body"),
    // The two Qt `text-align` pins on `notes-row-on` / `notes-row-off` were
    // deleted in 0.3.0: the list is RecordList, whose rows ask for no
    // `text-align`, so there is nothing left to drop.
];

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Both themes, which is NOT what `theme: None` alone gives you.
///
/// `None` selects the historical dark-wins default, so a property dropped only
/// by the light stylesheet is invisible to it. DraftEditor's gate measured
/// this: a property added only to its LIGHT `.msl` produced no report under
/// `None`.
const THEMES: &[Option<&str>] = &[None, Some("light")];

fn theme_name(theme: Option<&str>) -> &str {
    theme.unwrap_or("dark")
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

/// The part a style drop was reported on.
///
/// The analyzer names a style drop's location `$style.<part>`. Anything else
/// returns `None`, so it cannot match a pin and stays unexpected rather than
/// slipping through.
fn part_of(layout_path: &str) -> Option<&str> {
    layout_path.strip_prefix("$style.")
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
/// The test above runs one way: it catches a NEW drop and cannot catch a
/// pinned one that stopped happening. A fixed-but-still-listed entry is not
/// untidy — it is a standing licence for that property to be dropped again
/// with this gate green. Engram's own copy of this pattern had 11 of 21 pins
/// in exactly that state before it was checked.
///
/// Checked per theme, not over the union of both: the ten drops occur in
/// each theme today, so a pin that one theme stopped producing is already a
/// change worth seeing.
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
/// the package's exact per-theme drop count: 12 on the audit base, 10 since
/// E3, and the number PR-6 is meant to move next.
#[test]
fn the_pin_list_names_each_drop_once() {
    for (i, pin) in ALLOWED_STYLE_DROPS.iter().enumerate() {
        assert!(
            !ALLOWED_STYLE_DROPS[i + 1..].contains(pin),
            "{pin:?} is pinned twice"
        );
    }
}
