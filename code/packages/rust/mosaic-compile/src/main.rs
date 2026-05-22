//! # mosaic-compile — CLI for compiling .mosaic files to platform-specific output.
//!
//! This binary wires together the Mosaic compiler's System A pipeline:
//!
//! ```text
//! .mosaic source
//!      │
//!      ▼
//! mosaic-analyzer  →  MosaicFile (IR)
//!      │
//!      ▼
//! MosaicVM  (drives MosaicRenderer callbacks)
//!      │
//!      ├── --backend webcomponent  →  MyComponent.js   (Custom Element)
//!      ├── --backend html          →  MyComponent.html (static snapshot)
//!      ├── --backend react         →  MyComponent.jsx  (React functional component)
//!      └── --backend paint         →  MyComponent.png  (raster PNG via Paint VM)
//! ```
//!
//! The CLI surface is driven by the spec at `code/specs/mosaic-compile.json`
//! and parsed by the `cli-builder` crate.  All help text, flag validation, and
//! version output are generated from that spec — there is no hand-rolled help
//! string in this file.

use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use cli_builder::types::ParserOutput;
use cli_builder::{load_spec_from_file, Parser};
use mosaic_analyzer::analyze;
use mosaic_emit_html::HtmlRenderer;
use mosaic_emit_react::ReactRenderer;
use mosaic_emit_webcomponent::WebComponentRenderer;
use mosaic_emit_paint;
use mosaic_package_artifact_builder::{build_package, Backend, BuildOptions};
use mosaic_vm::MosaicVM;

// ===========================================================================
// Repo-root discovery
// ===========================================================================

/// Walk up to find the repo root, identified by the sentinel file
/// `code/specs/mosaic-compile.json`.
///
/// Searches from two starting points in order:
/// 1. The current working directory — works when the user runs the binary from
///    inside the repo (the most common development workflow).
/// 2. The directory containing the binary itself — works when the binary is
///    invoked by an absolute path from an unrelated directory (e.g. `/tmp`),
///    because `target/debug/mosaic-compile` lives inside the repo tree.
///
/// Falls back to cwd if neither search finds the sentinel.
fn find_root() -> PathBuf {
    const SENTINEL: &str = "code/specs/mosaic-compile.json";

    let search_starts: Vec<PathBuf> = [
        std::env::current_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf())),
    ]
    .into_iter()
    .flatten()
    .collect();

    for start in search_starts {
        let mut curr = start;
        for _ in 0..20 {
            if curr.join(SENTINEL).exists() {
                return curr;
            }
            match curr.parent() {
                Some(p) => curr = p.to_path_buf(),
                None => break,
            }
        }
    }

    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

// ===========================================================================
// Main
// ===========================================================================

fn main() {
    // ---- Locate the CLI spec and parse arguments via cli-builder -------------

    let root = find_root();
    let spec_path = root.join("code/specs/mosaic-compile.json");
    let spec = load_spec_from_file(spec_path.to_str().unwrap_or("code/specs/mosaic-compile.json"))
        .unwrap_or_else(|e| {
            eprintln!("mosaic-compile: failed to load CLI spec: {e}");
            process::exit(1);
        });

    let parser = Parser::new(spec);
    let argv: Vec<String> = std::env::args().collect();

    match parser.parse(&argv) {
        // --help
        Ok(ParserOutput::Help(h)) => {
            print!("{}", h.text);
        }

        // --version
        Ok(ParserOutput::Version(v)) => {
            println!("{}", v.version);
        }

        // Normal invocation
        Ok(ParserOutput::Parse(result)) => {
            run(result);
        }

        // Bad flags / missing required args — cli-builder formats the error
        Err(e) => {
            eprintln!("{e}");
            eprintln!("Run 'mosaic-compile --help' for usage.");
            process::exit(1);
        }
    }
}

// ===========================================================================
// Core logic
// ===========================================================================

/// Execute the compilation after cli-builder has parsed and validated argv.
fn run(result: cli_builder::types::ParseResult) {
    // ---- Subcommand dispatch ------------------------------------------------
    //
    // cli-builder reports the resolved command path as
    // `["mosaic-compile"]` for the root invocation and
    // `["mosaic-compile", "pkg"]` (etc.) for subcommands. We branch up front
    // because the package-build flow shares no logic with the single-file
    // compile path — different inputs, different outputs, different errors.
    if result.command_path.iter().any(|c| c == "pkg") {
        run_pkg(&result);
        return;
    }

    let flags = &result.flags;
    let args = &result.arguments;

    // Required: --backend
    let backend = flags
        .get("backend")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            eprintln!("mosaic-compile: --backend is required");
            process::exit(1);
        });

    if backend != "webcomponent"
        && backend != "html"
        && backend != "react"
        && backend != "paint"
        && backend != "xaml"
    {
        eprintln!(
            "mosaic-compile: --backend must be 'webcomponent', 'html', 'react', 'paint', or 'xaml', got '{backend}'"
        );
        process::exit(1);
    }

    // ---- Mode detection: legacy (.mosaic) vs. three-file pipeline -----------
    //
    // The CLI supports two mutually exclusive modes:
    //   * Legacy: a single positional SOURCE file (a `.mosaic` document).
    //   * Pipeline (UI23 / UI24): the three flags `--interface`, `--layout`,
    //     `--style` pointing at `.mil`, `.mll`, `.msl` files respectively.
    //
    // We detect the mode here. If both are present we reject the invocation;
    // if neither is present we reject as well. This keeps the user honest about
    // intent and produces a clearer error than "SOURCE file is required" when
    // they meant to use the pipeline flags.

    let interface_path = flags.get("interface").and_then(|v| v.as_str());
    let layout_path = flags.get("layout").and_then(|v| v.as_str());
    let style_path = flags.get("style").and_then(|v| v.as_str());
    let source_path = args.get("source").and_then(|v| v.as_str());
    let output_path = flags.get("output").and_then(|v| v.as_str());
    // --emit-project: when set on a pipeline xaml build, emit a full
    // WinUI 3 host shell (csproj + App + MainWindow + manifest +
    // build.ps1 + README) alongside the component triple. Fix B1.
    let emit_project = flags
        .get("emit-project")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // --package-manifest: when set, the .mll's component-reference
    // resolver auto-registers every name in [components].exports so
    // intra-package references work (e.g. Field → Input in
    // mosaic-pkg-toolkit). UI29-§4.4-style external dependencies
    // are NOT loaded yet — only the self-package's exports.
    let package_manifest_path = flags
        .get("package-manifest")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let pipeline_any =
        interface_path.is_some() || layout_path.is_some() || style_path.is_some();

    if pipeline_any && source_path.is_some() {
        eprintln!(
            "mosaic-compile: cannot mix SOURCE (legacy mode) with \
             --interface/--layout/--style (pipeline mode); use one or the other"
        );
        process::exit(1);
    }

    if pipeline_any {
        // Pipeline mode — all three flags are required together.
        let interface = require_pipeline_flag("interface", interface_path);
        let layout = require_pipeline_flag("layout", layout_path);
        let style = require_pipeline_flag("style", style_path);
        run_pipeline(
            backend,
            interface,
            layout,
            style,
            output_path,
            emit_project,
            package_manifest_path.as_deref(),
        );
        return;
    }

    // ---- Legacy mode -------------------------------------------------------
    let source_path = source_path.unwrap_or_else(|| {
        eprintln!(
            "mosaic-compile: provide either a SOURCE file (legacy single-file \
             mode) or all of --interface, --layout, --style (three-file \
             pipeline mode); see --help"
        );
        process::exit(1);
    });

    // Optional flags
    let fixtures_path = flags.get("fixtures").and_then(|v| v.as_str());
    let css_path = flags.get("css").and_then(|v| v.as_str());

    // ---- Read and analyze the source file ------------------------------------

    let source_text = read_file_or_die(source_path);

    let mosaic_file = analyze(&source_text).unwrap_or_else(|e| {
        eprintln!("mosaic-compile: error analyzing {source_path}: {e}");
        process::exit(1);
    });

    let component_name = mosaic_file.component.name.clone();
    let vm = MosaicVM::new(mosaic_file);

    // ---- Dispatch to the selected backend ------------------------------------

    match backend {
        "webcomponent" => {
            let out = output_path
                .map(str::to_string)
                .unwrap_or_else(|| format!("{component_name}.js"));

            let renderer = WebComponentRenderer::new();
            let result = vm.run(renderer).unwrap_or_else(|e| {
                eprintln!("mosaic-compile: webcomponent backend error: {e}");
                process::exit(1);
            });

            write_file_or_die(&out, &result.output);
            eprintln!("Written: {out}");
        }

        "html" => {
            let out = output_path
                .map(str::to_string)
                .unwrap_or_else(|| format!("{component_name}.html"));

            // Load optional fixture JSON.
            let fixtures = if let Some(path) = fixtures_path {
                let raw = read_file_or_die(path);
                let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_else(|e| {
                    eprintln!("mosaic-compile: error parsing fixtures file {path}: {e}");
                    process::exit(1);
                });
                val.as_object().cloned().unwrap_or_default()
            } else {
                serde_json::Map::new()
            };

            // Load optional CSS, rejecting content that would break out of <style>.
            let css = css_path.map(|path| {
                let raw = read_file_or_die(path);
                mosaic_emit_html::sanitize_css(&raw).unwrap_or_else(|e| {
                    eprintln!(
                        "mosaic-compile: CSS file '{path}' rejected for security reasons: {e}"
                    );
                    process::exit(1);
                })
            });

            let renderer = HtmlRenderer::new(fixtures, css);
            let result = vm.run(renderer).unwrap_or_else(|e| {
                eprintln!("mosaic-compile: html backend error: {e}");
                process::exit(1);
            });

            write_file_or_die(&out, &result.output);
            eprintln!("Written: {out}");
        }

        "react" => {
            let out = output_path
                .map(str::to_string)
                .unwrap_or_else(|| format!("{component_name}.jsx"));

            let renderer = ReactRenderer::new();
            let result = vm.run(renderer).unwrap_or_else(|e| {
                eprintln!("mosaic-compile: react backend error: {e}");
                process::exit(1);
            });

            write_file_or_die(&out, &result.output);
            eprintln!("Written: {out}");
        }

        "paint" => {
            // The paint backend bypasses MosaicVM and calls mosaic-emit-paint
            // directly: Mosaic source → PaintScene → raster PNG bytes.
            //
            // Unlike the text-based backends (html, react, webcomponent), this
            // path produces binary output, so we use write_bytes_or_die instead
            // of write_file_or_die.
            let out = output_path
                .map(str::to_string)
                .unwrap_or_else(|| format!("{component_name}.png"));

            let png_bytes = mosaic_emit_paint::render_png_with_defaults(&source_text)
                .unwrap_or_else(|e| {
                    eprintln!("mosaic-compile: paint backend error: {e}");
                    process::exit(1);
                });

            write_bytes_or_die(&out, &png_bytes);
            eprintln!("Written: {out}");
        }

        other => {
            // Should not reach here — caught above.
            eprintln!("mosaic-compile: unknown backend '{other}'");
            process::exit(1);
        }
    }
}

// ===========================================================================
// Three-file pipeline (UI23 / UI24)
// ===========================================================================

/// Enforce that a pipeline flag is present; bail with a clear error otherwise.
///
/// The other two flags' presence is checked here too so the user gets one
/// message rather than three serial complaints.
fn require_pipeline_flag<'a>(name: &str, value: Option<&'a str>) -> &'a str {
    value.unwrap_or_else(|| {
        eprintln!(
            "mosaic-compile: --{name} is required in pipeline mode (with --interface --layout --style)"
        );
        process::exit(1);
    })
}

/// Build a `mosaic_emit_xaml::ComponentRegistry` that registers every
/// component exported by the active package's manifest — minus the
/// component currently being compiled (a component shouldn't reference
/// itself).
///
/// Returns `None` when `manifest_path` is `None` (the user didn't pass
/// `--package-manifest`), or when the manifest fails to parse (we
/// print a warning and continue without a registry — a single-file
/// compile shouldn't be blocked by a malformed sibling-manifest).
///
/// The xmlns prefix derives from the manifest's `package.name`
/// (lowercase, drops the `mosaic-pkg-` prefix). `mosaic-pkg-toolkit`
/// → `toolkit`. The xmlns value derives from the active emitter's
/// C# namespace, since all components in the same package share a
/// generated namespace.
///
/// Fix for the self-reference gap that blocked Field from
/// referencing Input cleanly in `mosaic-pkg-toolkit`.
fn build_self_package_registry(
    manifest_path: Option<&str>,
    current_component: &str,
    csharp_namespace: &str,
) -> Option<mosaic_emit_xaml::ComponentRegistry> {
    let path = manifest_path?;
    let manifest = match mosaic_package_manifest::parse_path(std::path::Path::new(path)) {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "mosaic-compile: warning: ignoring --package-manifest {path}: {e:?}"
            );
            return None;
        }
    };
    let xmlns_prefix = manifest
        .package
        .name
        .strip_prefix("mosaic-pkg-")
        .unwrap_or(&manifest.package.name)
        .to_lowercase();
    let xmlns_value = format!("using:{csharp_namespace}");

    let mut reg = mosaic_emit_xaml::ComponentRegistry::new();
    for export in &manifest.components.exports {
        if export == current_component {
            // Self-self-reference is meaningless and would mask
            // recursion bugs at parse time.
            continue;
        }
        reg.register(
            export.as_str(),
            xmlns_prefix.as_str(),
            xmlns_value.as_str(),
            manifest.package.name.as_str(),
        );
    }
    if reg.is_empty() {
        // A single-export package, or every export is the current
        // component — no point handing the emitter an empty registry.
        None
    } else {
        Some(reg)
    }
}

/// Run the three-file pipeline path: compile `.mil`, `.mll`, `.msl` to a
/// single output file using the new pipeline-aware backend emitter.
///
/// Currently `--backend react` and `--backend xaml` are wired here; the
/// other backends (swiftui, qt) will follow when they're added. The legacy
/// `--backend X SOURCE.mosaic` path continues to work unchanged for any of
/// the four backends.
fn run_pipeline(
    backend: &str,
    interface_path: &str,
    layout_path: &str,
    style_path: &str,
    output_path: Option<&str>,
    emit_project: bool,
    package_manifest_path: Option<&str>,
) {
    if backend != "react" && backend != "xaml" {
        eprintln!(
            "mosaic-compile: pipeline mode (--interface/--layout/--style) \
             currently supports --backend react or --backend xaml (got '{backend}'). \
             Use legacy SOURCE mode for other backends."
        );
        process::exit(1);
    }

    // -- 1. Compile the mosmodel interface ----------------------------------
    //
    // The mosmodel compiler emits a JSON descriptor that the moslayout
    // compiler uses to validate slot/emit references.
    let interface_src = read_file_or_die(interface_path);
    let mosmodel_out = mosmodel_compiler::compile(&interface_src).unwrap_or_else(|errs| {
        eprintln!("mosaic-compile: mosmodel error(s) in {interface_path}:");
        for e in errs {
            eprintln!("  {e:?}");
        }
        process::exit(1);
    });

    // -- 2. Compile the moslayout file --------------------------------------
    //
    // We pass the descriptor JSON so the moslayout compiler can check that
    // every `@slot` and `emit onX` reference resolves correctly.
    let layout_src = read_file_or_die(layout_path);
    let layout_out =
        moslayout_compiler::compile(&layout_src, Some(&mosmodel_out.descriptor_json))
            .unwrap_or_else(|errs| {
                eprintln!("mosaic-compile: moslayout error(s) in {layout_path}:");
                for e in errs {
                    eprintln!("  {e:?}");
                }
                process::exit(1);
            });

    // -- 3. Compile the mosstyle file ---------------------------------------
    //
    // The part map JSON from moslayout tells mosstyle which part names are
    // legal targets for style blocks.
    let style_src = read_file_or_die(style_path);
    let style_out = mosstyle_compiler::compile(&style_src, Some(&layout_out.part_map_json))
        .unwrap_or_else(|errs| {
            eprintln!("mosaic-compile: mosstyle error(s) in {style_path}:");
            for e in errs {
                eprintln!("  {e:?}");
            }
            process::exit(1);
        });

    // -- 4. Branch on backend. React emits one .tsx file; XAML emits a
    // triple (.xaml, .xaml.cs, .Event.cs) plus zero-or-more RowVm .cs
    // files (one per `For` block).
    match backend {
        "react" => {
            let result = mosaic_emit_react::pipeline::from_pipeline(
                &mosmodel_out.component,
                &layout_out.def,
                &style_out.def,
            )
            .unwrap_or_else(|e| {
                eprintln!("mosaic-compile: react pipeline emit error: {e}");
                process::exit(1);
            });
            let out = output_path
                .map(str::to_string)
                .unwrap_or_else(|| format!("{}.tsx", result.component_name));
            write_file_or_die(&out, &result.output);
            eprintln!("Written: {out}");
        }
        "xaml" => {
            let mut opts = mosaic_emit_xaml::EmitOptions::default();
            opts.emit_project = emit_project;
            // Build the component registry: auto-register every name
            // in the active package's [components].exports so the .mll
            // can reference its siblings (UI29 §4.4 ish — but for the
            // SELF package, not external dependencies). Skips the
            // currently-being-compiled component to avoid a useless
            // self-self-reference.
            let registry = build_self_package_registry(
                package_manifest_path,
                &mosmodel_out.component.component,
                &opts.namespace,
            );
            let result = mosaic_emit_xaml::from_pipeline(
                &mosmodel_out.component,
                &layout_out.def,
                &style_out.def,
                registry.as_ref(),
                &opts,
            )
            .unwrap_or_else(|e| {
                eprintln!("mosaic-compile: xaml pipeline emit error: {e}");
                process::exit(1);
            });
            // `output_path` is treated as a *base* for the three (or
            // more) generated files. Default base = the component name.
            let base = output_path
                .map(str::to_string)
                .unwrap_or_else(|| result.component_name.clone());
            // Strip a trailing `.xaml` if the user passed e.g. `Grid.xaml`
            // so the C# files don't end up as `Grid.xaml.xaml.cs`.
            let base = base
                .strip_suffix(".xaml")
                .map(str::to_string)
                .unwrap_or(base);

            let xaml_path = format!("{base}.xaml");
            let cs_path = format!("{base}.xaml.cs");
            let evt_path = format!("{base}.Event.cs");
            write_file_or_die(&xaml_path, &result.xaml);
            write_file_or_die(&cs_path, &result.code_behind);
            write_file_or_die(&evt_path, &result.events);
            eprintln!("Written: {xaml_path}");
            eprintln!("Written: {cs_path}");
            eprintln!("Written: {evt_path}");
            // Per-For RowVm files (PR-2). One side-file per For block.
            //
            // Helper: place a side-file next to the .xaml file (same
            // directory as `base`).
            let side_file_path = |filename: &str| -> String {
                match base.rfind(|c| c == '/' || c == '\\') {
                    Some(idx) => format!("{}/{}", &base[..idx], filename),
                    None => filename.to_string(),
                }
            };
            for vm in &result.for_view_models {
                let rv_path = side_file_path(&vm.filename);
                write_file_or_die(&rv_path, &vm.source);
                eprintln!("Written: {rv_path}");
            }
            // PR-2 (Fix A5): if BoolToVisibilityConverter.cs was
            // emitted, write it alongside.
            for helper in &result.if_helpers {
                let path = side_file_path(&helper.filename);
                write_file_or_die(&path, &helper.source);
                eprintln!("Written: {path}");
            }
            // Fix B1: --emit-project — full WinUI 3 host shell.
            if let Some(proj) = &result.project {
                let component = &result.component_name;
                let writes: [(String, &str); 7] = [
                    (side_file_path(&format!("{component}.csproj")), &proj.csproj),
                    (side_file_path("App.xaml"), &proj.app_xaml),
                    (side_file_path("App.xaml.cs"), &proj.app_xaml_cs),
                    (side_file_path("MainWindow.xaml"), &proj.main_window_xaml),
                    (side_file_path("MainWindow.xaml.cs"), &proj.main_window_cs),
                    (side_file_path("app.manifest"), &proj.package_manifest),
                    (side_file_path("build.ps1"), &proj.build_script),
                ];
                for (path, src) in &writes {
                    write_file_or_die(path, src);
                    eprintln!("Written: {path}");
                }
                let readme_path = side_file_path("README.md");
                write_file_or_die(&readme_path, &proj.readme);
                eprintln!("Written: {readme_path}");
            }
        }
        _ => {
            // Already validated above; defensive.
            eprintln!("mosaic-compile: unsupported pipeline backend '{backend}'");
            process::exit(1);
        }
    }
}

// ===========================================================================
// `pkg` subcommand — package-artifact build (UI29 §4.3)
// ===========================================================================

/// Drive `mosaic_package_artifact_builder::build_package` from the CLI.
///
/// Spec (mosaic-compile.json):
///
/// ```text
/// mosaic-compile pkg <PACKAGE_ROOT> --backend <react|swiftui|qt> --output <DIR>
/// ```
///
/// Required: `package_root` positional, `--backend`, `--output`. cli-builder
/// already enforces presence; we re-check defensively and produce friendly
/// messages because cli-builder's errors are formatted before this function
/// is called.
fn run_pkg(result: &cli_builder::types::ParseResult) {
    let flags = &result.flags;
    let args = &result.arguments;

    let package_root = args
        .get("package_root")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            eprintln!("mosaic-compile pkg: PACKAGE_ROOT is required");
            process::exit(1);
        });

    let backend_str = flags
        .get("backend")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            eprintln!("mosaic-compile pkg: --backend is required");
            process::exit(1);
        });

    let output = flags
        .get("output")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            eprintln!("mosaic-compile pkg: --output is required");
            process::exit(1);
        });

    // Map the string to the typed `Backend`. The artifact builder accepts
    // the un-wired variants too (so callers can type the API surface
    // uniformly) but they return `UnsupportedBackend` immediately; we
    // forward that as-is below.
    let backend = match backend_str {
        "react" => Backend::React,
        "swiftui" => Backend::SwiftUI,
        "qt" => Backend::Qt,
        "webcomponent" => Backend::WebComponent,
        "html" => Backend::Html,
        other => {
            eprintln!(
                "mosaic-compile pkg: --backend must be one of \
                 react|swiftui|qt|webcomponent|html, got '{other}'"
            );
            process::exit(1);
        }
    };

    let opts = BuildOptions {
        package_root: PathBuf::from(package_root),
        output_root: PathBuf::from(output),
        backend,
    };

    match build_package(&opts) {
        Ok(result) => {
            for path in &result.artifacts {
                eprintln!("Written: {}", path.display());
            }
            eprintln!(
                "mosaic-compile pkg: built {} component(s)",
                result.components_built.len()
            );
        }
        Err(e) => {
            eprintln!("mosaic-compile pkg: {e}");
            process::exit(1);
        }
    }
}

// ===========================================================================
// File I/O helpers
// ===========================================================================

/// Read a file to a String, or print an error and exit with code 1.
fn read_file_or_die(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("mosaic-compile: cannot read {path}: {e}");
        process::exit(1);
    })
}

/// Write a string to a file, creating parent directories as needed.
fn write_file_or_die(path: &str, content: &str) {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("mosaic-compile: cannot create directory {}: {e}", parent.display());
                process::exit(1);
            });
        }
    }
    fs::write(path, content).unwrap_or_else(|e| {
        eprintln!("mosaic-compile: cannot write {path}: {e}");
        process::exit(1);
    });
}

/// Write raw bytes to a file, creating parent directories as needed.
///
/// Used for binary backends (e.g. `--backend paint`) that produce PNG output
/// rather than UTF-8 text.  Mirrors `write_file_or_die` but accepts `&[u8]`
/// so the caller doesn't need to round-trip bytes through a String.
fn write_bytes_or_die(path: &str, content: &[u8]) {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("mosaic-compile: cannot create directory {}: {e}", parent.display());
                process::exit(1);
            });
        }
    }
    fs::write(path, content).unwrap_or_else(|e| {
        eprintln!("mosaic-compile: cannot write {path}: {e}");
        process::exit(1);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `build_self_package_registry` returns None when no manifest
    /// path is provided.
    #[test]
    fn registry_is_none_without_manifest() {
        let r = build_self_package_registry(None, "Field", "Mosaic.Generated");
        assert!(r.is_none());
    }

    /// Given a manifest path that points at a valid manifest, the
    /// registry contains every export except the current component.
    #[test]
    fn registry_registers_sibling_exports() {
        let tmp = std::env::temp_dir().join(format!(
            "self-ref-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let mpath = tmp.join("mosaic-package.toml");
        std::fs::write(
            &mpath,
            r#"
[package]
name = "mosaic-pkg-toolkit"
version = "0.1.0"
description = "test"
license = "MIT"
[components]
exports = ["Button", "Field", "Input"]
[dependencies]
[kernel]
version = "1"
"#,
        )
        .unwrap();

        let r = build_self_package_registry(
            Some(mpath.to_str().unwrap()),
            "Field",
            "Mosaic.Generated",
        );
        let reg = r.expect("registry built from valid manifest");
        assert!(reg.lookup("Button").is_some(), "Button should be registered");
        assert!(reg.lookup("Input").is_some(), "Input should be registered");
        assert!(
            reg.lookup("Field").is_none(),
            "current component should NOT self-reference"
        );

        let entry = reg.lookup("Button").unwrap();
        assert_eq!(entry.xmlns_prefix, "toolkit");
        assert_eq!(entry.xmlns_value, "using:Mosaic.Generated");
        assert_eq!(entry.package_name, "mosaic-pkg-toolkit");

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// A package that exports only the current component returns None
    /// — no point handing the emitter an empty registry.
    #[test]
    fn registry_is_none_when_only_export_is_current_component() {
        let tmp = std::env::temp_dir().join(format!(
            "self-ref-test2-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let mpath = tmp.join("mosaic-package.toml");
        std::fs::write(
            &mpath,
            r#"
[package]
name = "mosaic-pkg-card"
version = "0.1.0"
description = "test"
license = "MIT"
[components]
exports = ["Card"]
[dependencies]
[kernel]
version = "1"
"#,
        )
        .unwrap();
        let r = build_self_package_registry(
            Some(mpath.to_str().unwrap()),
            "Card",
            "Mosaic.Generated",
        );
        assert!(r.is_none());
        std::fs::remove_dir_all(&tmp).ok();
    }

    /// A missing manifest path produces a warning + None.
    #[test]
    fn registry_is_none_when_manifest_missing() {
        let r = build_self_package_registry(
            Some("/this/path/definitely/does/not/exist.toml"),
            "Field",
            "Mosaic.Generated",
        );
        assert!(r.is_none());
    }
}
