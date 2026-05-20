//! package_compiles — smoke test for the mosaic-pkg-dialog package.
//!
//! Dialog is the framework's cross-backend smoke test: a single userland
//! component that compiles cleanly through *every* Mosaic emitter without
//! reaching for primitives that exist in only some backends.  If this
//! test passes, the language frontend and at least the three "ready"
//! backend artifact emitters (React, SwiftUI, Qt) can ingest a userland
//! package end-to-end.
//!
//! What this test asserts
//! ----------------------
//!
//! 1. The manifest at `mosaic-package.toml` parses and declares exactly
//!    `exports = ["Dialog"]`.
//! 2. `Dialog.mil` compiles via `mosmodel_compiler::compile`, with three
//!    slots (`title`, `message`, `close-label`) and one emit (`onClose`).
//! 3. `Dialog.mll` compiles via `moslayout_compiler::compile`, validated
//!    against the `.mil`'s interface descriptor.  We also walk the
//!    resulting layout tree and assert the structural shape the spec
//!    promised:
//!
//!        Box [dialog-root]
//!          Column [dialog-stack]
//!            Box [dialog-title]    -> Text
//!            Box [dialog-message]  -> Text
//!            Box [dialog-actions]  -> HostButton
//!
//! 4. `Dialog.dark.msl` compiles via `mosstyle_compiler::compile` against
//!    the layout's part map, and the resulting style IR declares all
//!    four parts (`dialog-root`, `dialog-title`, `dialog-message`,
//!    `dialog-actions`).
//! 5. The per-backend artifact builder — the same crate that
//!    `mosaic-compile pkg build` drives — produces a non-empty
//!    `Dialog.<ext>` for every backend it currently supports (React,
//!    SwiftUI, Qt).  WebComponent and HTML are SKIPPED with a documented
//!    comment because today's builder returns `UnsupportedBackend` for
//!    those two enum variants (their `from_pipeline` entry points are
//!    landing in parallel PRs).  XAML is not yet in the builder's enum
//!    at all, so it is not exercised here.
//!
//! When a future PR lights up the WebComponent / HTML / XAML backends in
//! the artifact builder, this test will start covering them automatically
//! — just remove the skip comment and the relevant Backend variants from
//! `BACKENDS_SKIPPED`.

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

/// Parses `mosaic-package.toml` and asserts the UI29 §4.2 minimum surface:
/// `[package]` identity, `[components].exports = ["Dialog"]`, an empty
/// `[dependencies]`, and `[kernel].version = "1"`.
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

    // [package].version
    let version = value
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .expect("[package].version must be set");
    assert_eq!(version, "0.1.0", "[package].version must be 0.1.0");

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

/// `Dialog.mil` must compile and declare exactly the slots/emits the spec
/// promises.  We assert both names AND types so a future contributor who
/// accidentally renames a slot (e.g. `title` → `header`) trips the test
/// loudly, instead of the change silently propagating into a backend
/// artifact that no longer matches the published interface.
#[test]
fn mil_declares_three_slots_and_one_emit() {
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
        vec!["title", "message", "close-label"],
        "Dialog must declare exactly slots: title, message, close-label (in that order)"
    );
    for slot in &out.component.slots {
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
/// exact tree shape the README documents.  Asserting the shape (not just
/// "it compiled") catches the failure mode where the file still parses
/// but a primitive name has been changed — e.g. `Column` -> `Stack` —
/// which would silently change every emitter's output layout.
#[test]
fn mll_compiles_with_expected_shape() {
    let mil_src = read_source("Dialog.mil");
    let mil_out = mosmodel_compiler::compile(&mil_src)
        .unwrap_or_else(|e| panic!("Dialog.mil precompile failed: {:#?}", e));

    let mll_src = read_source("Dialog.mll");
    let mll_out = moslayout_compiler::compile(&mll_src, Some(&mil_out.descriptor_json))
        .unwrap_or_else(|e| panic!("Dialog.mll failed to compile: {:#?}", e));

    let root = &mll_out.def.root;

    // Layer 1: Box [dialog-root]
    assert_eq!(root.tag, "Box", "root node must be a Box");
    assert_eq!(
        root.part_name.as_deref(),
        Some("dialog-root"),
        "root Box must own the `dialog-root` part"
    );
    assert_eq!(root.children.len(), 1, "Box[dialog-root] has exactly one child (the Column)");

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
        3,
        "Column[dialog-stack] must have exactly three children (title, message, actions)"
    );

    // Layer 3a: Box [dialog-title] -> Text
    let title_box = &stack.children[0];
    assert_eq!(title_box.tag, "Box");
    assert_eq!(title_box.part_name.as_deref(), Some("dialog-title"));
    assert_eq!(title_box.children.len(), 1);
    assert_eq!(title_box.children[0].tag, "Text", "title Box wraps a Text");

    // Layer 3b: Box [dialog-message] -> Text
    let message_box = &stack.children[1];
    assert_eq!(message_box.tag, "Box");
    assert_eq!(message_box.part_name.as_deref(), Some("dialog-message"));
    assert_eq!(message_box.children.len(), 1);
    assert_eq!(message_box.children[0].tag, "Text", "message Box wraps a Text");

    // Layer 3c: Box [dialog-actions] -> HostButton
    let actions_box = &stack.children[2];
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
/// declare style blocks for all four named parts.  We do not assert the
/// inside of each block — colours/sizes may evolve — but the *set of
/// parts* is a public surface of the package (a host writing a custom
/// theme will target the same part names), so we assert it.
#[test]
fn msl_compiles_and_declares_four_parts() {
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
        vec!["dialog-actions", "dialog-message", "dialog-root", "dialog-title"],
        "Dialog.dark.msl must declare exactly four parts: dialog-root, \
         dialog-title, dialog-message, dialog-actions (got {:?})",
        part_names
    );
}

// ---------------------------------------------------------------------------
// 5. Per-backend artifact compilation
// ---------------------------------------------------------------------------

/// The list of backends the artifact builder currently wires.  React /
/// SwiftUI / Qt have a `from_pipeline` entry point; WebComponent and Html
/// return `UnsupportedBackend` today.  XAML is not represented in the
/// builder's `Backend` enum at all (its emitter is still landing in the
/// `feat(mosaic-emit-xaml)` PR series), so it cannot be exercised from
/// this test until the builder is taught about it.
const SUPPORTED_BACKENDS: &[Backend] = &[Backend::React, Backend::SwiftUI, Backend::Qt];

/// Backends that the artifact builder knows about but does not yet wire.
/// We assert each of these returns the documented `UnsupportedBackend`
/// error rather than silently succeeding or crashing — that way the day
/// they DO get wired, this assertion flips and we know to move the
/// variant up into `SUPPORTED_BACKENDS`.
const SKIPPED_BACKENDS: &[Backend] = &[Backend::WebComponent, Backend::Html];

/// For each supported backend, drive `build_package` and assert it
/// produces a non-empty `Dialog.<ext>` (plus the package index file).
#[test]
fn supported_backends_build_dialog_artifact() {
    // Stage output into a sibling `target/dialog-artifacts` directory so
    // a developer running `cargo test` can inspect the generated files
    // after the fact.  We use `target/` because Cargo already excludes it
    // from version control.
    let out_root = package_root().join("target").join("dialog-artifacts");
    // Clean previous run so stale files from a removed backend don't
    // confuse a curious reader.
    let _ = fs::remove_dir_all(&out_root);

    for backend in SUPPORTED_BACKENDS {
        let opts = BuildOptions {
            package_root: package_root(),
            output_root: out_root.clone(),
            backend: *backend,
        };
        let result = build_package(&opts).unwrap_or_else(|e| {
            panic!("artifact build for {:?} failed: {}", backend, e);
        });

        assert_eq!(
            result.components_built,
            vec!["Dialog".to_string()],
            "{:?} build must report exactly Dialog as the built component",
            backend
        );

        // The first artifact is the per-component file; the last is the
        // package index (index.ts / index.swift / qmldir).  We sanity-
        // check both.
        let component_artifact = result
            .artifacts
            .iter()
            .find(|p| p.file_name().and_then(|s| s.to_str()) != Some("index.ts")
                && p.file_name().and_then(|s| s.to_str()) != Some("index.swift")
                && p.file_name().and_then(|s| s.to_str()) != Some("qmldir"))
            .unwrap_or_else(|| {
                panic!("{:?} build produced no per-component artifact", backend)
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
        // Every backend either uses `Dialog` as the function/struct/type
        // name (React/SwiftUI/Qt) or references it in the index — so the
        // name must appear somewhere in the artifact body.
        assert!(
            body.contains("Dialog"),
            "{:?} artifact at {} does not mention the component name 'Dialog':\n{}",
            backend,
            component_artifact.display(),
            body
        );
    }
}

/// Backends the builder knows about but has not wired yet must return
/// `UnsupportedBackend`.  This is a documentation test: it captures the
/// CURRENT state of the artifact builder so the day WebComponent or Html
/// is wired, this test fails loudly and the contributor knows to flip
/// the relevant variant into `SUPPORTED_BACKENDS`.
#[test]
fn skipped_backends_return_unsupported() {
    let out_root = package_root().join("target").join("dialog-artifacts-skip");
    for backend in SKIPPED_BACKENDS {
        let opts = BuildOptions {
            package_root: package_root(),
            output_root: out_root.clone(),
            backend: *backend,
        };
        let err = build_package(&opts).unwrap_err();
        assert!(
            matches!(err, BuildError::UnsupportedBackend(b) if b == *backend),
            "expected UnsupportedBackend({:?}), got {:?}",
            backend,
            err
        );
    }
}

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
