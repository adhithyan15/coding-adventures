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
//! "Remove an entry the moment its issue is fixed" is now **enforced**, not
//! merely asked for. `no_allowed_degradation_has_silently_been_fixed` requires
//! every entry to still be observed, because the gate above only ever ran one
//! way — it catches an untracked degradation and cannot catch a tracked one
//! that stopped happening. A fixed-but-still-listed entry is not untidy; it is
//! a standing licence for that exact degradation to come back with this test
//! green. Engram's copy of this pattern had 11 of 21 style pins in that state.
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
    // Compose was the first backend to report here: it immediately named 42
    // drops across `border-radius`, `opacity` and `font-weight`; #14817,
    // #14821 and #14818 fixed all three, and each removed its own entry
    // rather than leaving it to rot. Compose has no entry below, and that
    // emptiness is now a measurement rather than an absence of looking.
    //
    // Qt gained reporting in #15245 and named 60 drops that had always been
    // there. 16 are fixed: `border-color`, `border-width` and `border-radius`
    // were read from the base part only, so an Alert or a Toast wore the base
    // variant's border whatever its `variant` — the background beside them was
    // already conditional, and the border was not. The 44 below are real, and
    // each points at the issue that will retire it.
    //
    // #15276 — a `Text` part cannot paint. `background`, `border-color`,
    // `border-width` and `padding` on `$style.tabs-panel`,
    // `$style.input-group-prefix`/`-suffix` and `$style.accordion-body` reach
    // a bare QML `Text`, which has no fill, no `border.*` and no padding.
    // Its comment thread also covers Badge's `font-weight`, dropped because a
    // `Rectangle` has no font group and nothing carries the value to the
    // `Text` inside it.
    (Backend::Qt, "background"),
    (Backend::Qt, "border-color"),
    (Backend::Qt, "border-width"),
    (Backend::Qt, "font-weight"),
    // #15276 and #15277 both — `padding` is dropped on Text parts (Accordion,
    // InputGroup, Tabs) AND on host controls (Button, Field, Input, Navbar).
    // It takes both fixes to retire this one entry, which is an argument for
    // the finer-grained pin this list does not yet have.
    (Backend::Qt, "padding"),
    // #15277 — state-layer overrides are ignored outside the Rectangle
    // builders. Spinner authors `width`/`height` per size variant and renders
    // all three at the base's 24px. Extending the Rectangle path to
    // `width`/`height` was tried and changed zero bytes of output, which is
    // how we know those parts never traverse it -- Spinner is a `Stack` and
    // lowers to a QML `Item`.
    //
    // `border-radius` was pinned here and is NOT any more: the fourth site
    // that assembles a Rectangle (a host control's `background: Rectangle`)
    // had a conditional `border.color` beside a base-only `radius`, so Button
    // and Input rendered every size with the base corner. The inverse ratchet
    // below is what caught it -- the pin was written, the site was fixed, and
    // the assertion refused to let the stale entry stand.
    (Backend::Qt, "font-size"),
    (Backend::Qt, "height"),
    (Backend::Qt, "width"),
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

/// Every allowed degradation is still a degradation.
///
/// The gate above runs one way: it catches an UNTRACKED degradation, and cannot
/// catch a tracked one that stopped happening. That gap is not cosmetic. An
/// entry whose defect was fixed stays on the list, and from then on it is a
/// standing licence — the same degradation can return, on the same component
/// and backend, with the gate still green.
///
/// The comment on `ALLOWED_STYLE_DROPS` records that #14817, #14821 and #14818
/// each removed their own entry by hand. That is the right instinct, and it is
/// exactly the kind of discipline that holds until the one time nobody
/// remembers — fixing an emitter does not otherwise bring anyone back to this
/// file. This makes it a gate rather than a habit.
///
/// Found by the same check on Engram's copy of this pattern, where 11 of 21
/// pinned style drops had silently been fixed.
///
/// `ALLOWED_STYLE_DROPS` deliberately gets no equivalent: it is empty, so the
/// check would compare two empty sets and prove nothing. It is already as
/// strict as it can be — every style drop fails.
/// Every allowed style drop is still a drop.
///
/// The twin of `no_allowed_degradation_has_silently_been_fixed`, which this
/// list did not have. That asymmetry is the one this file's own header warns
/// about: Engram's copy of the pattern reached 11 stale pins out of 21,
/// because a pin that is fixed and left in place stops recording a known gap
/// and starts licensing its return. Nothing was watching these.
#[test]
fn no_allowed_style_drop_has_silently_been_fixed() {
    let mut observed: Vec<(Backend, String)> = Vec::new();

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

        for entry in &report.style_degradations {
            if let Some(property) = entry.primitive.as_deref() {
                observed.push((backend, property.to_string()));
            }
        }
    }

    let stale: Vec<String> = ALLOWED_STYLE_DROPS
        .iter()
        .filter(|(backend, property)| {
            !observed
                .iter()
                .any(|(seen_backend, seen_property)| {
                    seen_backend == backend && seen_property == property
                })
        })
        .map(|(backend, property)| format!("  {backend:?}: {property}"))
        .collect();

    assert!(
        stale.is_empty(),
        "{} allowed style drop(s) no longer occur:\n{}\n\n\
         The emitter learned to lower these, so delete them from \
         ALLOWED_STYLE_DROPS and close the issue each points at. Left in \
         place they license the drop coming back.",
        stale.len(),
        stale.join("\n")
    );
}

#[test]
fn no_allowed_degradation_has_silently_been_fixed() {
    let mut observed: Vec<(Backend, String, String)> = Vec::new();

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

        for entry in &report.degradations {
            observed.push((backend, entry.component.clone(), entry.code.clone()));
        }
    }

    let stale: Vec<String> = ALLOWED_DEGRADATIONS
        .iter()
        .filter(|(backend, component, code)| {
            !observed
                .iter()
                .any(|(seen_backend, seen_component, seen_code)| {
                    seen_backend == backend && seen_component == component && seen_code == code
                })
        })
        .map(|(backend, component, code)| format!("  {backend:?}: {component} — {code}"))
        .collect();

    assert!(
        stale.is_empty(),
        "{} allowed degradation(s) no longer occur:\n{}\n\n\
         The defect was fixed, so delete these from ALLOWED_DEGRADATIONS and \
         close the issue each points at. Left in place they stop recording a \
         known gap and start licensing its return.",
        stale.len(),
        stale.join("\n")
    );
}
