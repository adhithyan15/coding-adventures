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
//! platform limits. Ignoring them would let a new one arrive unnoticed.
//!
//! (This paragraph used to say "Engram has twelve, all on XAML". That stopped
//! being true when Compose and SwiftUI began reporting their own drops, and the
//! count is exactly the kind of number that goes stale unread — which is the
//! argument for pinning properties rather than counts, made against itself.)
//!
//! So they are pinned by **property**, which is the part that carries meaning.
//! A count would pass if one drop were fixed and another introduced; a property
//! list says exactly what is being tolerated and fails on anything new.
//!
//! Pinning runs in **both** directions, and only one of them was here
//! originally. A pinned drop that stops happening has to be removed too:
//! left in place it is no longer a record of a known gap but a standing licence
//! for that property to be dropped again with the gate still green. The first
//! run of `no_pinned_style_drop_has_silently_been_fixed` found 11 of 21 entries
//! in exactly that state.
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
/// ## Eleven entries were deleted here, not fixed here
///
/// `no_pinned_style_drop_has_silently_been_fixed` is new, and the first time it
/// ran it found **11 of these 21 pins no longer occurred**. The emitters had
/// learned the mappings -- Compose's `max-width` is `Modifier.widthIn`, tagged
/// `#14833` in `mosaic-emit-compose`; XAML's per-side borders lower to
/// `BorderThickness` and have their own tests -- and nothing brought anyone
/// back to this list, because fixing an emitter does not touch this file.
///
/// A stale pin is worse than untidy. It stops being a record of a known gap and
/// becomes a standing licence: the property may be dropped again, on that
/// backend, and the gate stays green. More than half this list had quietly
/// turned into that.
///
/// What remains below is what is still genuinely dropped.
const ALLOWED_STYLE_DROPS: &[(Backend, &str)] = &[
    // ---- Compose (#14811) ----
    //
    // These became visible the moment Compose started reporting its drops in
    // #14811. They are pre-existing gaps, not regressions: Engram has been
    // rendering without them on Compose since the parts were authored.
    //
    // Of the nine originally pinned here, seven are gone -- `max-width`
    // (#14833), the three arrangement arguments (#14834), the two
    // `border-bottom` halves (#14835) and `flex-wrap` (#14836) all now map.
    // Only the one that needs no mapping is left.
    //
    // `border-bottom-style: solid` has no Compose equivalent and needs none --
    // solid is the only stroke it draws.
    (Backend::Compose, "border-bottom-style"),
    // WinUI 3 genuinely has no WrapPanel, so `flex-wrap` has nowhere to go.
    // An inherent platform limit rather than a mapping we have not written.
    (Backend::Xaml, "flex-wrap"),
    // `border-bottom-style: solid` has no XAML equivalent and needs none --
    // solid is the only kind of border WinUI draws. Its sibling
    // `border-bottom-width`/`-color` pins (#14132) are gone: the emitter
    // learned `BorderThickness="0,0,0,1"`.
    (Backend::Xaml, "border-bottom-style"),
    // ---- SwiftUI (#14728) ----
    //
    // These became visible when SwiftUI started reporting its drops (#12022);
    // they are pre-existing gaps, not regressions. The two that were a single
    // match arm each -- `border-radius` (254 occurrences here) and `max-width`
    // -- were mapped in the same change rather than listed.
    //
    // Layout properties: NOT missing mappings. SwiftUI expresses these through
    // the view shape chosen at construction (`HStack(spacing:alignment:)`,
    // `Spacer`, `.layoutPriority`), and this emitter appends modifiers to an
    // already-built view, so no match arm could apply them. Fixing them means
    // the container emitter reading the part's style before emitting children.
    //
    // `gap` was on this list and should not have been -- the comment above
    // describes the fix, and the container emitter had ALREADY been doing it:
    // `Column` opens `VStack(spacing:)` and `Row` an `HStack(spacing:)`, read
    // from the part's own style. What had not been fixed was the REPORT, which
    // scanned only the modifier chain and so called every applied gap a drop.
    // 22 of Engram's 40 reported SwiftUI drops were this, `$style.app-shell`
    // among them -- reported to drop `gap: 18` while its emitted Swift opened
    // `VStack(spacing: 18)`.
    //
    // A `gap` that a `Box`, `Stack` or `HostScroll` really does discard is
    // still reported, so this entry would come back if one appeared here.
    (Backend::SwiftUI, "align"),
    (Backend::SwiftUI, "align-items"),
    (Backend::SwiftUI, "justify-content"),
    (Backend::SwiftUI, "flex-grow"),
    // No SwiftUI equivalent before the `Layout` protocol; a wrapping stack has
    // to be written. The closest thing to a genuine platform limit here.
    (Backend::SwiftUI, "flex-wrap"),
    // Needs no mapping -- solid is the only stroke SwiftUI draws. The
    // `border-bottom-width`/`-color` pins beside it are gone: `.overlay`
    // draws the rule and the emitter learned it.
    (Backend::SwiftUI, "border-bottom-style"),
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

/// Every pinned drop is still a drop.
///
/// The test above is one-directional: it catches a NEW drop, and cannot catch a
/// pinned one that stopped happening. That asymmetry is not cosmetic. An entry
/// whose gap was fixed stays on the list, and from then on it is a permanently
/// open hole -- a later regression that re-drops the same property on the same
/// backend is allowlisted and passes silently, which is exactly the state this
/// file exists to prevent.
///
/// `seen_any` does not cover it either. It asks whether ANY drop was reported,
/// so it stays green while twenty of twenty-one entries go stale.
///
/// So each pair must still be observed. When one is not, the gap was fixed and
/// the entry should be deleted -- which is also the only moment anyone finds
/// out, since fixing an emitter mapping does not otherwise touch this file.
#[test]
fn no_pinned_style_drop_has_silently_been_fixed() {
    // A `Vec`, not a `HashSet`: `Backend` is not `Hash`, and deriving it to
    // suit a test would be the tail wagging the dog. Both collections are
    // dozens of entries, so the linear scan below costs nothing.
    let mut observed: Vec<(Backend, String)> = Vec::new();

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
            if let Some(primitive) = entry.primitive.as_deref() {
                observed.push((backend, primitive.to_string()));
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
        "{} pinned style drop(s) no longer occur:\n{}\n\n\
         The emitter learned the mapping, so delete these from \
         ALLOWED_STYLE_DROPS -- and close the issue each is pinned to. Left in \
         place they stop being a record of a known gap and become a licence for \
         that property to be dropped again with nothing noticing.",
        stale.len(),
        stale.join("\n")
    );
}
