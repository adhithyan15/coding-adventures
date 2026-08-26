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
//! The toolkit is NOT degradation-clean today: 9 pre-existing capability
//! gaps exist across all five native backends, none introduced by this
//! test. Each is real native-UI feature work (native indeterminate
//! checkbox state, native radio-group mutual exclusion, a real Flutter
//! dialog), not something fixable as a side effect of wiring this gate —
//! see the linked issues. `ALLOWED_DEGRADATIONS` is the explicit,
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

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// (backend, component, degradation code) — every entry must reference
/// its tracking issue in the comment beside it.
const ALLOWED_DEGRADATIONS: &[(Backend, &str, &str)] = &[
    // #13006 — native indeterminate checkbox state not implemented.
    (Backend::SwiftUI, "Checkbox", "property.checkbox-indeterminate-ignored"),
    (Backend::Flutter, "Checkbox", "property.checkbox-indeterminate-ignored"),
    (Backend::Compose, "Checkbox", "property.checkbox-indeterminate-ignored"),
    // #13007 — native radio-group mutual exclusion not implemented.
    (Backend::SwiftUI, "Radio", "property.radio-group-ignored"),
    (Backend::Qt, "Radio", "property.radio-group-ignored"),
    (Backend::Flutter, "Radio", "property.radio-group-ignored"),
    (Backend::Compose, "Radio", "property.radio-group-ignored"),
    // #13008 — XAML Modal requires app code-behind to open. Confirmed
    // permanent, not a to-do: WinUI3's ContentDialog has no bindable
    // IsOpen-style property the way Popup/Flyout/TeachingTip do, so
    // there's no declarative show/hide surface to bind `open:` to.
    (Backend::Xaml, "Modal", "property.dialog-open-host-required"),
    // #13010 — Flutter Modal is a zero-size TODO placeholder.
    (Backend::Flutter, "Modal", "interaction.dialog-placeholder"),
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

        assert!(
            report.style_degradations.is_empty(),
            "{backend:?}: style properties were dropped that this test's allowlist \
             doesn't expect (the toolkit was clean here as of #12024): {:#?}",
            report.style_degradations
        );
    }
}
