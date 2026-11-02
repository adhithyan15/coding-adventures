//! Repository guard: no layout in the repository relies on an unbound bare
//! name (UI29 §3.4, "A bare name must be bound"; backlog 6f1p).
//!
//! A bare NAME prop value (`Text ( content: hello )`) is a variable read on
//! every backend. The layout validator now rejects one that no enclosing
//! `For` binds and no slot declares, with `UnboundName`. This test runs that
//! check over the whole repository in the two places it applies:
//!
//! 1. **As authored.** Every `<C>.mll` / `<C>.<variant>.mll` beside a
//!    `<C>.mil`, compiled against that interface. This is where an
//!    unbound name in a component's own primitives is found.
//! 2. **As composed.** Every export of every `mosaic-package.toml`,
//!    composed the way a package build composes it (sibling exports
//!    qualified, `pkg::` references inlined, then validated again against
//!    the consumer's interface). This is where a bare value passed at a
//!    call site, which the authored check skips, is found: after inlining
//!    it sits in the callee's primitives in the consumer's scope.
//!
//! Only `UnboundName` failures count. A layout that fails for another reason
//! is its own build's business, and is skipped; the count assertions catch a
//! walk that skips everything.

use std::fs;
use std::path::{Path, PathBuf};

use mosaic_package_artifact_builder::{compose_component_with_model_in_package, Backend};

/// `code/` in this checkout: this crate is `code/packages/rust/<crate>`.
fn code_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("code/ exists")
}

/// Every file under `dir` named by `keep`, skipping build output and
/// installed dependencies (which may hold copies of the same files).
fn collect(dir: &Path, keep: &dyn Fn(&str) -> bool, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if name.starts_with('.')
                || name == "target"
                || name.starts_with("target-")
                || name == "node_modules"
                || name == "build"
                || name == "dist"
            {
                continue;
            }
            collect(&path, keep, out);
        } else if file_type.is_file() && keep(&name) {
            out.push(path);
        }
    }
}

/// The component a layout file belongs to: `Grid.mobile.mll` → `Grid`.
fn component_of(layout: &Path) -> String {
    let name = layout.file_name().unwrap().to_string_lossy();
    name.split('.').next().unwrap().to_string()
}

#[test]
fn every_authored_layout_binds_every_bare_name() {
    let mut layouts = Vec::new();
    collect(&code_root(), &|name| name.ends_with(".mll"), &mut layouts);
    layouts.sort();

    let mut checked = 0;
    let mut failures = Vec::new();
    for layout_path in &layouts {
        let component = component_of(layout_path);
        let mil_path = layout_path.with_file_name(format!("{component}.mil"));
        let Ok(mil) = fs::read_to_string(&mil_path) else {
            continue;
        };
        let Ok(model) = mosmodel_compiler::compile(&mil) else {
            continue;
        };
        let source = fs::read_to_string(layout_path).unwrap();
        checked += 1;
        if let Err(errors) = moslayout_compiler::compile(&source, Some(&model.descriptor_json)) {
            for error in errors
                .iter()
                .filter(|e| e.kind == moslayout_compiler::ErrorKind::UnboundName)
            {
                failures.push(format!("{}:\n  {error}", layout_path.display()));
            }
        }
    }

    assert!(
        checked >= 100,
        "expected the repository's ~120 layouts with an interface, checked {checked}"
    );
    assert!(
        failures.is_empty(),
        "layouts with an unbound bare name:\n{}",
        failures.join("\n")
    );
}

#[test]
fn every_composed_export_binds_every_bare_name() {
    let code = code_root();
    let packages = code.join("packages");
    let mut manifests = Vec::new();
    collect(&code, &|name| name == "mosaic-package.toml", &mut manifests);
    manifests.sort();

    let mut composed = 0;
    let mut failures = Vec::new();
    for manifest_path in &manifests {
        let Ok(manifest) = mosaic_package_manifest::parse_path(manifest_path) else {
            continue;
        };
        let package_root = manifest_path.parent().unwrap();
        let src = package_root.join("src");
        // The search paths a package build derives: the package's parent
        // directory, `code/packages` and `code/packages/mosaic`.
        let search_paths = vec![
            package_root.parent().unwrap().to_path_buf(),
            packages.clone(),
            packages.join("mosaic"),
        ];
        for component in &manifest.components.exports {
            let Ok(mil) = fs::read_to_string(src.join(format!("{component}.mil"))) else {
                continue;
            };
            let style = fs::read_to_string(src.join(format!("{component}.msl")))
                .unwrap_or_else(|_| format!("style {component} {{ }}"));
            let mut layouts: Vec<PathBuf> = fs::read_dir(&src)
                .into_iter()
                .flatten()
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    let name = path.file_name().unwrap().to_string_lossy();
                    name.ends_with(".mll") && component_of(path) == *component
                })
                .collect();
            layouts.sort();
            for layout_path in layouts {
                let Ok(model) = mosmodel_compiler::compile(&mil) else {
                    continue;
                };
                let source = fs::read_to_string(&layout_path).unwrap();
                match compose_component_with_model_in_package(
                    component,
                    model,
                    &source,
                    &style,
                    &search_paths,
                    None,
                    package_root,
                    &manifest,
                    Backend::React,
                ) {
                    Ok(_) => composed += 1,
                    Err(error) => {
                        let error = error.to_string();
                        if error.contains("UnboundName") {
                            failures.push(format!("{}:\n  {error}", layout_path.display()));
                        }
                    }
                }
            }
        }
    }

    assert!(
        composed >= 80,
        "expected the repository's ~90 exported layouts to compose, composed {composed}"
    );
    assert!(
        failures.is_empty(),
        "composed layouts with an unbound bare name:\n{}",
        failures.join("\n")
    );
}
