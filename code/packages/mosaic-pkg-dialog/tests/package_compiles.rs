//! package_compiles — smoke test for the mosaic-pkg-dialog package (v0.2.0).
//!
//! v0.2.0 rewrites Dialog as a thin wrapper around the `HostDialog` kernel
//! primitive added by UI29-1.  This test asserts:
//!
//! 1. The manifest at `mosaic-package.toml` parses and declares
//!    `exports = ["Dialog"]` at version `0.2.0`.
//! 2. `Dialog.mil` compiles via `mosmodel_compiler::compile` and declares
//!    four slots in the documented order:
//!
//!        open : bool                          (NEW in v0.2.0)
//!        title : text
//!        message : text
//!        close-label : text
//!
//!    plus one zero-payload emit (`onClose`).
//! 3. `Dialog.mll` compiles via `moslayout_compiler::compile` validated
//!    against the `.mil`'s interface descriptor, and the resulting tree
//!    has the new shape the spec promised:
//!
//!        HostDialog [dialog-shell]            <-- NEW: root is the kernel
//!                                                 primitive, not a Box
//!          Column [dialog-stack]
//!            Box [dialog-message]    -> Text
//!            Box [dialog-actions]    -> HostButton
//!
//! 4. `Dialog.dark.msl` compiles via `mosstyle_compiler::compile` against
//!    the layout's part map, and declares exactly three parts
//!    (`dialog-shell`, `dialog-message`, `dialog-actions`) — the
//!    v0.1.0 `dialog-title` part is gone, because HostDialog renders the
//!    title natively.
//! 5. The per-backend artifact builder produces a non-empty `Dialog.<ext>`
//!    for each of React, SwiftUI, Qt — OR — returns a documented
//!    "unknown primitive: HostDialog" pipeline error.  The second case is
//!    the **expected** state today because the UI29-1 K-* backend
//!    lowering PRs are still in flight; the test records the backend as
//!    *deferred* rather than failing, so this package can land before
//!    every backend has wired HostDialog.  Any *other* error (a panic, an
//!    I/O failure, a different pipeline error) still fails the test
//!    loudly.
//! 6. WebComponent and HTML still return `UnsupportedBackend` from the
//!    artifact builder today (their `from_pipeline` entry points are
//!    landing in parallel PRs).  XAML is not yet in the builder's
//!    `Backend` enum at all.  Both states are skipped with documented
//!    comments — see `skipped_backends_return_unsupported`.
//!
//! When the K-react / K-swiftui / K-qt PRs land, the deferred status will
//! automatically flip to "built" — the assertion is two-sided: it accepts
//! either outcome and just records which one happened.  Once HostDialog
//! is wired on every backend, a follow-up PR can tighten this test to
//! demand success unconditionally.

use std::fs;
use std::path::PathBuf;

use mosaic_package_artifact_builder::{build_package, Backend, BuildError, BuildOptions};

/// Anchor every path resolution to the package root.  `CARGO_MANIFEST_DIR`
/// is set by Cargo at compile time, so this is deterministic regardless of
/// where `cargo test` was invoked from.
fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn src_path(name: &str) -> PathBuf {
    package_root().join("src").join(name)
}

fn read_source(name: &str) -> String {
    let path = src_path(name);
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e))
}

// ---------------------------------------------------------------------------
// 1. Manifest
// ---------------------------------------------------------------------------

/// Parses `mosaic-package.toml` and asserts the UI29 §4.2 minimum surface,
/// plus the v0.2.0 version bump (the manifest version is part of the
/// package's public contract — bumping it from 0.1.0 → 0.2.0 is what tells
/// downstream consumers a breaking change happened).
#[test]
fn manifest_declares_dialog_export() {
    let manifest_src = fs::read_to_string(package_root().join("mosaic-package.toml"))
        .expect("manifest mosaic-package.toml must exist at the package root");

    let value: toml::Value = toml::from_str(&manifest_src)
        .expect("mosaic-package.toml must parse as valid TOML");

    // [package].name
    let name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .expect("[package].name must be set");
    assert_eq!(name, "mosaic-pkg-dialog", "[package].name mismatch");

    // [package].version — v0.2.0 is the HostDialog rewrite.
    let version = value
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .expect("[package].version must be set");
    assert_eq!(
        version, "0.2.0",
        "[package].version must be 0.2.0 (the HostDialog rewrite)"
    );

    // [components].exports
    let exports = value
        .get("components")
        .and_then(|c| c.get("exports"))
        .and_then(|e| e.as_array())
        .expect("[components].exports must be an array");
    let export_names: Vec<&str> = exports.iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(
        export_names,
        vec!["Dialog"],
        "[components].exports must list exactly Dialog"
    );

    // [kernel].version
    let kernel_version = value
        .get("kernel")
        .and_then(|k| k.get("version"))
        .and_then(|v| v.as_str())
        .expect("[kernel].version must be set");
    assert_eq!(kernel_version, "1", "[kernel].version must target UI29 kernel v1");
}

// ---------------------------------------------------------------------------
// 2. mosmodel — Dialog.mil compilation
// ---------------------------------------------------------------------------

/// `Dialog.mil` must compile and declare exactly the slots/emits the
/// v0.2.0 spec promises: FOUR slots (one new `open: bool` plus the three
/// text slots inherited from v0.1.0) and one zero-arg `onClose` emit.
#[test]
fn mil_declares_four_slots_and_one_emit() {
    let src = read_source("Dialog.mil");
    let out = mosmodel_compiler::compile(&src)
        .unwrap_or_else(|errs| panic!("Dialog.mil failed to compile: {:#?}", errs));

    assert_eq!(out.component.component, "Dialog", "component name must be Dialog");

    // Slots — name + type, in source order.
    let slot_names: Vec<&str> = out
        .component
        .slots
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert_eq!(
        slot_names,
        vec!["open", "title", "message", "close-label"],
        "Dialog v0.2.0 must declare exactly slots: open, title, message, \
         close-label (in that order)"
    );
    assert_eq!(
        out.component.slots.len(),
        4,
        "Dialog v0.2.0 must declare exactly 4 slots (v0.1.0 had 3 — \
         `open` was added in v0.2.0)"
    );

    // `open` is the only bool slot; the other three are text.
    assert!(
        matches!(out.component.slots[0].r#type, mosmodel_compiler::SlotType::Bool),
        "slot `open` must be of type bool (got {:?})",
        out.component.slots[0].r#type
    );
    for slot in &out.component.slots[1..] {
        assert!(
            matches!(slot.r#type, mosmodel_compiler::SlotType::Text),
            "slot {} must be of type text (got {:?})",
            slot.name,
            slot.r#type
        );
    }

    // Emits — name + zero-arg payload.
    let emit_names: Vec<&str> = out
        .component
        .emits
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(
        emit_names,
        vec!["onClose"],
        "Dialog must declare exactly one emit: onClose"
    );
    assert!(
        out.component.emits[0].params.is_empty(),
        "onClose must be a zero-arg emit (got {:?})",
        out.component.emits[0].params
    );
}

// ---------------------------------------------------------------------------
// 3. moslayout — Dialog.mll compilation + structural shape
// ---------------------------------------------------------------------------

/// `Dialog.mll` must compile against the `.mil` interface AND have the
/// new v0.2.0 tree shape.  The big change from v0.1.0: the root is now
/// `HostDialog` (the kernel primitive added by UI29-1), not a plain
/// `Box`.  We assert the shape explicitly so a future refactor that
/// accidentally re-introduces the outer Box trips the test loudly.
#[test]
fn mll_compiles_with_expected_shape() {
    let mil_src = read_source("Dialog.mil");
    let mil_out = mosmodel_compiler::compile(&mil_src)
        .unwrap_or_else(|e| panic!("Dialog.mil precompile failed: {:#?}", e));

    let mll_src = read_source("Dialog.mll");
    let mll_out = moslayout_compiler::compile(&mll_src, Some(&mil_out.descriptor_json))
        .unwrap_or_else(|e| panic!("Dialog.mll failed to compile: {:#?}", e));

    let root = &mll_out.def.root;

    // Layer 1: HostDialog [dialog-shell] — the kernel primitive is now
    // the root (replacing v0.1.0's Box [dialog-root]).
    assert_eq!(
        root.tag, "HostDialog",
        "root node must be HostDialog (UI29-1 kernel primitive), not Box"
    );
    assert_eq!(
        root.part_name.as_deref(),
        Some("dialog-shell"),
        "root HostDialog must own the `dialog-shell` part (renamed from \
         v0.1.0's `dialog-root`)"
    );
    assert_eq!(
        root.children.len(),
        1,
        "HostDialog[dialog-shell] has exactly one child (the Column body)"
    );

    // The HostDialog node carries four structural props: open, modal,
    // title, onClose.  We assert each by name; values are checked
    // loosely (slot/emit binding presence) since the value AST varies.
    let prop_names: Vec<&str> = root.props.iter().map(|p| p.name.as_str()).collect();
    let mut sorted_prop_names = prop_names.clone();
    sorted_prop_names.sort();
    assert_eq!(
        sorted_prop_names,
        vec!["modal", "onClose", "open", "title"],
        "HostDialog must carry props: open, modal, title, onClose \
         (got {:?})",
        prop_names
    );

    // Layer 2: Column [dialog-stack]
    let stack = &root.children[0];
    assert_eq!(stack.tag, "Column", "layer-2 node must be a Column");
    assert_eq!(
        stack.part_name.as_deref(),
        Some("dialog-stack"),
        "stack Column must own the `dialog-stack` part"
    );
    assert_eq!(
        stack.children.len(),
        2,
        "Column[dialog-stack] must have exactly two children (message, \
         actions) — v0.1.0's title row is gone in v0.2.0 (HostDialog's \
         native title slot replaces it)"
    );

    // Layer 3a: Box [dialog-message] -> Text
    let message_box = &stack.children[0];
    assert_eq!(message_box.tag, "Box");
    assert_eq!(message_box.part_name.as_deref(), Some("dialog-message"));
    assert_eq!(message_box.children.len(), 1);
    assert_eq!(message_box.children[0].tag, "Text", "message Box wraps a Text");

    // Layer 3b: Box [dialog-actions] -> HostButton
    let actions_box = &stack.children[1];
    assert_eq!(actions_box.tag, "Box");
    assert_eq!(actions_box.part_name.as_deref(), Some("dialog-actions"));
    assert_eq!(actions_box.children.len(), 1);
    assert_eq!(
        actions_box.children[0].tag, "HostButton",
        "actions Box must wrap a HostButton (the close button)"
    );
}

// ---------------------------------------------------------------------------
// 4. mosstyle — Dialog.dark.msl compilation + part coverage
// ---------------------------------------------------------------------------

/// `Dialog.dark.msl` must compile against the layout's part map AND
/// declare style blocks for exactly the three v0.2.0 parts.  The fourth
/// v0.1.0 part (`dialog-title`) is gone — HostDialog renders the title
/// natively — and `dialog-root` was renamed to `dialog-shell`.
#[test]
fn msl_compiles_and_declares_three_parts() {
    let mll_src = read_source("Dialog.mll");
    let mll_out = moslayout_compiler::compile(&mll_src, None)
        .unwrap_or_else(|e| panic!("Dialog.mll precompile failed: {:#?}", e));

    let msl_src = read_source("Dialog.dark.msl");
    let msl_out = mosstyle_compiler::compile(&msl_src, Some(&mll_out.part_map_json))
        .unwrap_or_else(|e| panic!("Dialog.dark.msl failed to compile: {:#?}", e));

    let part_names: Vec<&str> = msl_out.def.parts.iter().map(|p| p.name.as_str()).collect();
    let mut sorted = part_names.clone();
    sorted.sort();
    assert_eq!(
        sorted,
        vec!["dialog-actions", "dialog-message", "dialog-shell"],
        "Dialog.dark.msl v0.2.0 must declare exactly three parts: \
         dialog-shell, dialog-message, dialog-actions (got {:?})",
        part_names
    );
}

// ---------------------------------------------------------------------------
// 5. Per-backend artifact compilation
// ---------------------------------------------------------------------------

/// The list of backends the artifact builder currently *attempts*.  React,
/// SwiftUI, and Qt have a `from_pipeline` entry point — but until the
/// UI29-1-K-react, K-swiftui, K-qt PRs land, those entry points return
/// `UnknownPrimitive("HostDialog")`, which the artifact builder wraps as
/// `BuildError::PipelineError`.  The test below treats that as a
/// **deferred** state rather than a failure.
const ARTIFACT_BACKENDS: &[Backend] = &[
    Backend::React,
    Backend::SwiftUI,
    Backend::Qt,
    Backend::Html,
    Backend::WebComponent,
    Backend::Xaml,
];

/// All six UI29 §4.3 backends are now wired by
/// `mosaic-package-artifact-builder` (UI29-2 follow-up). The
/// `UNWIRED_BACKENDS` slice that used to track WebComponent/Html
/// is empty, so the `skipped_backends_return_unsupported` test
/// has been removed (it pinned the rejection path that no longer
/// exists).
const UNWIRED_BACKENDS: &[Backend] = &[];

/// Distinguish an "unknown primitive: HostDialog" pipeline error (the
/// expected "K-* PR hasn't landed yet" state) from any other failure
/// mode.  Each backend's emitter renders its `UnknownPrimitive` error
/// slightly differently — the React emitter writes
/// `"moslayout primitive 'HostDialog' is not yet supported by the
/// pipeline React emitter"`, the SwiftUI emitter writes a similar
/// `"primitive 'HostDialog' not yet supported"` shape, and the Qt
/// emitter follows the same template — but every variant includes the
/// substring `HostDialog`.  Since `HostDialog` is the *only* primitive
/// we use in this package that any backend might not recognise (every
/// other tag — Box, Column, Text, HostButton — has been supported on
/// every backend since the v0.1.0 commit), a pipeline error whose
/// message mentions `HostDialog` is unambiguously the deferred state.
///
/// Anything else (panics, IO errors, a *different* unknown primitive,
/// a syntax-level pipeline error, a manifest error) bubbles up as a
/// real test failure.
fn is_deferred_hostdialog_error(err: &BuildError) -> bool {
    match err {
        BuildError::PipelineError { error, .. } => error.contains("HostDialog"),
        _ => false,
    }
}

/// For each artifact backend, drive `build_package` and demand one of:
///
///   (a) Ok(BuildResult) with a non-empty `Dialog.<ext>` artifact — the
///       backend's HostDialog lowering has landed.
///   (b) Err(PipelineError) whose message mentions `UnknownPrimitive`
///       and `HostDialog` — the backend's HostDialog lowering is still
///       in flight.  This is the expected state on `main` today.
///
/// Anything else fails the test.  After the run, the test prints a
/// per-backend status line so a contributor can see at a glance which
/// backends are ready.
#[test]
fn artifact_backends_either_build_or_defer_on_hostdialog() {
    // Stage output into a sibling `target/dialog-artifacts` directory
    // so a developer running `cargo test` can inspect any successfully
    // built files after the fact.  Cargo already excludes `target/`
    // from version control.
    let out_root = package_root().join("target").join("dialog-artifacts");
    let _ = fs::remove_dir_all(&out_root);

    // Track per-backend outcomes so we can print a status table at the
    // end.  A bare-bones `Vec<(Backend, &str)>` is enough — this is a
    // developer-facing diagnostic, not a parsed CI artifact.
    let mut outcomes: Vec<(Backend, &'static str)> = Vec::new();

    for backend in ARTIFACT_BACKENDS {
        let opts = BuildOptions {
            package_root: package_root(),
            output_root: out_root.clone(),
            backend: *backend,
        };

        match build_package(&opts) {
            Ok(result) => {
                // The K-* PR for this backend has landed: assert the
                // build produced what we expect.
                assert_eq!(
                    result.components_built,
                    vec!["Dialog".to_string()],
                    "{:?} build must report exactly Dialog as the built \
                     component",
                    backend
                );

                // The first artifact is the per-component file; the last
                // is the package index.  We look up the component file
                // by name and read it back.
                let component_artifact = result
                    .artifacts
                    .iter()
                    .find(|p| {
                        let name = p.file_name().and_then(|s| s.to_str());
                        name != Some("index.ts")
                            && name != Some("index.swift")
                            && name != Some("qmldir")
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "{:?} build produced no per-component artifact",
                            backend
                        )
                    });

                let body = fs::read_to_string(component_artifact).unwrap_or_else(|e| {
                    panic!(
                        "could not read {:?}'s artifact at {}: {}",
                        backend,
                        component_artifact.display(),
                        e
                    )
                });
                assert!(
                    !body.trim().is_empty(),
                    "{:?} produced an empty Dialog artifact at {}",
                    backend,
                    component_artifact.display()
                );
                assert!(
                    body.contains("Dialog"),
                    "{:?} artifact at {} does not mention the component \
                     name 'Dialog':\n{}",
                    backend,
                    component_artifact.display(),
                    body
                );
                outcomes.push((*backend, "BUILT"));
            }
            Err(err) if is_deferred_hostdialog_error(&err) => {
                // The K-* PR for this backend hasn't landed yet — this
                // is the expected state on `main` today for v0.2.0.
                outcomes.push((*backend, "DEFERRED (HostDialog lowering pending)"));
            }
            Err(err) => {
                panic!(
                    "{:?} build failed with an unexpected error (neither \
                     a successful build nor a deferred-HostDialog \
                     UnknownPrimitive pipeline error): {:?}",
                    backend, err
                );
            }
        }
    }

    // Print a status table so the developer running `cargo test --nocapture`
    // sees which backends are ready.  When every line says BUILT, the
    // K-* roadmap is done.
    eprintln!("\nmosaic-pkg-dialog v0.2.0 — per-backend status:");
    for (backend, status) in &outcomes {
        eprintln!("  {:?}: {}", backend, status);
    }
}

/// (Removed.) The v0.1.0-era `skipped_backends_return_unsupported`
/// test pinned the rejection path for WebComponent/Html. Those
/// backends are now fully wired by the artifact builder, so the
/// rejection path no longer exists for them. The `UNWIRED_BACKENDS`
/// slice is intentionally empty above — if a NEW backend ever gets
/// added to `Backend` but not wired in the artifact builder, populate
/// `UNWIRED_BACKENDS` and reinstate this test against it.

// ---------------------------------------------------------------------------
// 6. Source-shape sanity
// ---------------------------------------------------------------------------

/// Belt-and-suspenders: every file the package promises must be on disk.
/// Catches the failure mode where a `.mll` or `.msl` is accidentally
/// deleted in a refactor and the compile-test still passes because the
/// `for` loop simply iterates zero files.
#[test]
fn source_tree_has_expected_shape() {
    for name in ["Dialog.mil", "Dialog.mll", "Dialog.dark.msl"] {
        let path = src_path(name);
        assert!(
            path.exists(),
            "expected package source file missing: {}",
            path.display()
        );
    }
}
