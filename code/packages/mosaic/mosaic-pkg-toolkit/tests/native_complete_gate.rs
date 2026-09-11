//! native_complete_gate — per-component native-completeness for the
//! mosaic-pkg-toolkit atoms (issue #12024).
//!
//! `package_compiles.rs` proves every component round-trips through the
//! three IR compilers; it deliberately does not check backend lowering.
//! This test closes that gap: for each of the five native backends, it
//! runs the real `mosaic-package-artifact-builder` degradation analyzer
//! against the whole package (all 21 exported components in one pass —
//! the analyzer loops the manifest internally) and asserts nothing
//! *unexpected* is dropped.
//!
//! Before this test existed, the toolkit atoms had zero native
//! verification of their own — the only native check anywhere was the
//! whole-app TaskApp CI gate, which (per #12022/#12023) measures
//! capability coverage, not rendering, and mixes 21 components' worth of
//! failure surface into one signal. A failure here points at exactly one
//! component, one backend, one property.
//!
//! ## The allowlist
//!
//! The toolkit is NOT degradation-clean today: 5 pre-existing capability
//! gaps exist across the native backends, none introduced by this
//! test. Each is real native-UI feature work (native radio-group mutual
//! exclusion, XAML's inherent `ContentDialog` open-host requirement),
//! not something fixable as a side effect of wiring this gate — see the
//! linked issues. (#13006's native indeterminate checkbox state landed
//! on all three affected backends and no longer needs an entry here.)
//! `ALLOWED_DEGRADATIONS` is the explicit,
//! reviewed list of what's tolerated for now; anything else — on any
//! component, existing or new — fails this test immediately. Remove an
//! entry the moment its issue is fixed; do not add new entries without a
//! linked issue explaining why.
//!
//! `style_degradations` (issue #12022) gets NO allowlist: the toolkit is
//! already clean there, so any entry is a real regression.

use mosaic_package_artifact_builder::{
    analyze_package_degradations, Backend, BuildOptions, BuildProfile,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// (backend, property) — style properties a backend is known to discard.
///
/// Distinct from `ALLOWED_DEGRADATIONS` above: those are capability gaps the
/// analyzer reports as blocking, these are authored style properties the
/// emitter silently throws away. They do not gate `nativeComplete`, but they
/// are the difference between a control that looks right and one that merely
/// has the right accessibility tree.
///
/// Every entry must reference its tracking issue.
const ALLOWED_STYLE_DROPS: &[(Backend, &str)] = &[
    // #14810 — Compose has no `Modifier.clip(RoundedCornerShape(..))` in this
    // emitter, so every rounded surface in the toolkit renders square. 31 of
    // the toolkit's 42 drops, and 318 in TaskApp alone.
    (Backend::Compose, "border-radius"),
    // #14708 — `opacity` lowering for Compose and Flutter is still open. It is
    // what UI57's `state disabled` treatment depends on, so a disabled toolkit
    // control currently dims on some backends and not others.
    (Backend::Compose, "opacity"),
    // #14810 — `font-weight` is an argument to Text rather than a modifier, so
    // it has to be threaded through the text style instead of the box chain.
    // Every bold label in the toolkit renders at regular weight on Compose.
    (Backend::Compose, "font-weight"),
];

fn is_allowed_style_drop(backend: Backend, property: &str) -> bool {
    ALLOWED_STYLE_DROPS
        .iter()
        .any(|(b, p)| *b == backend && *p == property)
}

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// (backend, component, degradation code) — every entry must reference
/// its tracking issue in the comment beside it.
const ALLOWED_DEGRADATIONS: &[(Backend, &str, &str)] = &[
    // #13007 — Compose/Flutter/Qt now apply real native mutual-
    // exclusion wiring for a literal `group:` shared by 2+ resolvable
    // sibling HostRadios (e.g. mosaic-pkg-deck-options's leech-action
    // radios) — but the toolkit's own `Radio` component (`Radio.mll`)
    // is a 1:1 HostRadio wrapper with no sibling of its own, so it
    // never qualifies and stays degraded on all four backends here.
    // As of #14126 SwiftUI groups a real multi-radio run too, by
    // lowering it to a `Picker` whose selection is exclusive by
    // construction -- so all four backends are degraded HERE for the
    // same reason, and none of them for a SwiftUI-specific one.
    (Backend::SwiftUI, "Radio", "property.radio-group-ignored"),
    (Backend::Qt, "Radio", "property.radio-group-ignored"),
    (Backend::Flutter, "Radio", "property.radio-group-ignored"),
    (Backend::Compose, "Radio", "property.radio-group-ignored"),
    // #13008 — XAML Modal requires app code-behind to open. Confirmed
    // permanent, not a to-do: WinUI3's ContentDialog has no bindable
    // IsOpen-style property the way Popup/Flyout/TeachingTip do, so
    // there's no declarative show/hide surface to bind `open:` to.
    (Backend::Xaml, "Modal", "property.dialog-open-host-required"),
];

fn is_allowed(backend: Backend, component: &str, code: &str) -> bool {
    ALLOWED_DEGRADATIONS
        .iter()
        .any(|(b, c, code_)| *b == backend && *c == component && *code_ == code)
}

#[test]
fn toolkit_atoms_are_native_complete_or_explicitly_tracked() {
    for backend in [
        Backend::Xaml,
        Backend::SwiftUI,
        Backend::Qt,
        Backend::Flutter,
        Backend::Compose,
    ] {
        let out = TempDir::new().expect("temp dir");
        let report = analyze_package_degradations(
            &BuildOptions {
                package_root: package_root(),
                output_root: out.path().to_path_buf(),
                backend,
                emit_project: false,
                theme: None,
            },
            BuildProfile::NativeComplete,
        )
        .unwrap_or_else(|e| panic!("degradation analysis failed for {backend:?}: {e}"));

        let unexpected: Vec<_> = report
            .degradations
            .iter()
            .filter(|d| !is_allowed(backend, &d.component, &d.code))
            .collect();
        assert!(
            unexpected.is_empty(),
            "{backend:?}: untracked degradation(s) — either fix them, or add an \
             allowlist entry in this file pointing at a tracking issue: {unexpected:#?}"
        );

        // The toolkit was never "clean" here — until #12022 reporting reached a
        // backend, that backend said nothing, and silence read as cleanliness.
        // Compose started reporting its drops (#14810) and immediately named 42
        // drops across 3 properties. So this asserts what is TRACKED, not what is
        // absent, the same way `ALLOWED_DEGRADATIONS` does above.
        let unexpected_drops: Vec<_> = report
            .style_degradations
            .iter()
            // A drop that does not even name its property cannot be matched
            // against the allowlist, so it stays unexpected rather than
            // slipping through as "not in the list".
            .filter(|d| {
                d.primitive
                    .as_deref()
                    .is_none_or(|p| !is_allowed_style_drop(backend, p))
            })
            .collect();
        assert!(
            unexpected_drops.is_empty(),
            "{backend:?}: style properties were dropped that this test's allowlist \
             doesn't expect — either fix them, or add an entry pointing at a \
             tracking issue: {unexpected_drops:#?}"
        );
    }
}
