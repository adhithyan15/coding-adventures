//! Engram emits cleanly on every native backend — asserted, not measured once.
//!
//! As of #14126 the Engram app emits under `--profile native-complete` with
//! **zero degradations** on all five native backends. Nothing checked that,
//! which meant it was a fact about one afternoon rather than a property of the
//! app: any emitter change, or any new `.mll` using a capability a backend
//! lacks, would take it away silently and the suite would stay green.
//!
//! That is the gap this closes. It is the epic's (#13624) own line — *"the
//! Mosaic Engram app emits under `--profile native-complete` with zero
//! degradations, and that is asserted in CI"* — whose second half was missing.
//!
//! ## Capability degradations: zero, with no allowlist
//!
//! `mosaic-pkg-toolkit`'s equivalent gate carries an `ALLOWED_DEGRADATIONS`
//! list, because the toolkit genuinely is not clean: its `Radio` binds `group:`
//! from a *slot*, which no backend can resolve at compile time.
//!
//! Engram has no such entries and must not acquire any quietly. An allowlist is
//! a place for a regression to hide under a plausible comment, so there is none
//! — the assertion is zero.
//!
//! ## Style drops: pinned by property, not counted
//!
//! `style_degradations` is deliberately outside `native_complete` (see
//! `mosaic-package-artifact-builder`), because some entries are accepted
//! platform limits. Engram has twelve, all on XAML, and ignoring them would let
//! a thirteenth arrive unnoticed.
//!
//! So they are pinned by **property**, which is the part that carries meaning.
//! A count would pass if one drop were fixed and another introduced; a property
//! list says exactly what is being tolerated and fails on anything new.
//!
//! ## This is not the same "native-complete" as the Qt project gate
//!
//! `ci.yml` notes that Engram cannot be emitted with `--profile native-complete
//! --runtime-library`, because that rewrites `main.cpp` into the standard
//! binding shape and Engram replaces `MosaicHost` via `[host_assets]` with its
//! own `engram-capi` binding — the combination does not compile (#13728).
//!
//! That is about **project emission**. This analyses **capability lowering**
//! with `emit_project: false`, where no scaffolding is generated at all. Both
//! statements are true and they are about different things; conflating them
//! would make one of them look like a mistake.
//!
//! ## What "native-complete" adds over the default profile
//!
//! The permissive profile reports `runtime.library-not-bundled`, an artifact of
//! invoking the builder without `--runtime-library` rather than a defect. The
//! strict profile is what says whether the *UI* survives lowering, which is the
//! question worth gating.

use mosaic_package_artifact_builder::{
    analyze_package_degradations, Backend, BuildOptions, BuildProfile,
};
use std::path::PathBuf;
use tempfile::TempDir;

fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

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

/// Style properties a backend is currently allowed to drop, by name.
///
/// Pinned by property rather than by count: a count would still pass if one
/// drop were fixed and a different one appeared, which is precisely the
/// exchange worth noticing.
const ALLOWED_STYLE_DROPS: &[(Backend, &str)] = &[
    // WinUI 3 genuinely has no WrapPanel, so `flex-wrap` has nowhere to go.
    // An inherent platform limit rather than a mapping we have not written.
    (Backend::Xaml, "flex-wrap"),
    // #14132 — NOT a platform limit. WinUI expresses a bottom rule perfectly
    // well as `BorderThickness="0,0,0,1"`, and the emitter simply has no
    // mapping for the per-side properties. Introduced by the deck-list hairline
    // in #14115; the rows render on XAML without their separator until the
    // emitter learns the mapping.
    (Backend::Xaml, "border-bottom-width"),
    (Backend::Xaml, "border-bottom-color"),
    // `border-bottom-style: solid` has no XAML equivalent and needs none --
    // solid is the only kind of border WinUI draws.
    (Backend::Xaml, "border-bottom-style"),
];

#[test]
fn engram_emits_with_no_degradations_on_every_native_backend() {
    let mut failures = Vec::new();

    for &backend in NATIVE_BACKENDS {
        let output = TempDir::new().expect("temporary output");
        let report = analyze_package_degradations(
            &BuildOptions {
                package_root: package_root(),
                output_root: output.path().to_path_buf(),
                backend,
                emit_project: false,
                theme: None,
            },
            BuildProfile::NativeComplete,
        )
        .unwrap_or_else(|error| panic!("{backend:?} degradation analysis failed: {error}"));

        for entry in &report.degradations {
            failures.push(format!(
                "  {backend:?}: {} at {}",
                entry.code, entry.layout_path
            ));
        }

    }

    assert!(
        failures.is_empty(),
        "Engram lost {} native capability/capabilities.\n{}\n\n\
         Each line is a place a backend could not express something the layout \
         asks for -- the app still emits and still compiles, it just quietly \
         does less there. Fix the emitter, or change the layout to ask for \
         something expressible; adding an allowlist here would hide exactly \
         the regression this test exists to catch.",
        failures.len(),
        failures.join("\n")
    );
}

/// Nothing NEW is dropped from the stylesheets.
///
/// Style drops sit outside `native_complete` by design, so the gate above
/// cannot see them — and the deck-list hairline (#14115) added three of the
/// twelve without anyone noticing, which is the argument for this test rather
/// than against it.
#[test]
fn no_style_property_is_dropped_beyond_the_pinned_set() {
    let mut unexpected = Vec::new();
    let mut seen_any = false;

    for &backend in NATIVE_BACKENDS {
        let output = TempDir::new().expect("temporary output");
        let report = analyze_package_degradations(
            &BuildOptions {
                package_root: package_root(),
                output_root: output.path().to_path_buf(),
                backend,
                emit_project: false,
                theme: None,
            },
            BuildProfile::NativeComplete,
        )
        .unwrap_or_else(|error| panic!("{backend:?} degradation analysis failed: {error}"));

        for entry in &report.style_degradations {
            seen_any = true;
            let allowed = ALLOWED_STYLE_DROPS
                .iter()
                .any(|(b, property)| *b == backend && Some(*property) == entry.primitive.as_deref());
            if !allowed {
                unexpected.push(format!(
                    "  {backend:?}: {} at {} -- {}",
                    entry.primitive.as_deref().unwrap_or("<unnamed>"),
                    entry.layout_path,
                    entry.reason
                ));
            }
        }
    }

    assert!(
        unexpected.is_empty(),
        "{} style property/properties are dropped that were not pinned:\n{}\n\n\
         Either map the property in that backend's emitter, or add it to \
         ALLOWED_STYLE_DROPS with a reason and an issue.",
        unexpected.len(),
        unexpected.join("\n")
    );
    // If the analyzer stopped reporting style drops entirely, the pinning above
    // would pass by checking nothing.
    assert!(
        seen_any,
        "no style degradations reported at all -- expected XAML's known drops, \
         so either they were fixed (update ALLOWED_STYLE_DROPS) or the analyzer \
         stopped looking"
    );
}
