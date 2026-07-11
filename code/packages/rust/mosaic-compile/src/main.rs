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

// UI34 PR-3 — package-reference resolver.  Wired into the
// `run_pipeline` path between `moslayout_compiler::compile()` and the
// backend emitter so every `pkg::P::C` reference in the consumer's
// layout is substituted before any emitter sees it.
use cli_builder::types::ParserOutput;
use cli_builder::{load_spec_from_file, Parser};
use mosaic_analyzer::analyze;
use mosaic_emit_html::HtmlRenderer;
use mosaic_emit_react::ReactRenderer;
use mosaic_emit_webcomponent::WebComponentRenderer;
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
    let spec = load_spec_from_file(
        spec_path
            .to_str()
            .unwrap_or("code/specs/mosaic-compile.json"),
    )
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

    // UI30: pipeline mode now supports every emit-only backend
    // (react, html, webcomponent, swiftui, qt, xaml, flutter). The
    // legacy "paint" backend is single-file SOURCE mode only and is
    // also kept here for back-compat.
    let allowed_backends = [
        "webcomponent",
        "html",
        "react",
        "paint",
        "xaml",
        "swiftui",
        "qt",
        "flutter",
        "compose",
    ];
    if !allowed_backends.contains(&backend) {
        eprintln!(
            "mosaic-compile: --backend must be one of {allowed:?}, got '{backend}'",
            allowed = allowed_backends,
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
    // UI30 --variant: layout-variant selector. Only used when --layout
    // points at a directory; resolved via resolve_layout_path() below.
    let variant = flags.get("variant").and_then(|v| v.as_str());
    // --emit-project: when set on a pipeline xaml build, emit a full
    // WinUI 3 host shell (csproj + App + MainWindow + manifest +
    // build.ps1 + README) alongside the component triple. Fix B1.
    let emit_project = flags
        .get("emit-project")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    // --strict-style: escalate the "style part matches no layout part" warning
    // (emitted for every build) into a hard error — for CI that wants to fail on
    // stale stylesheets. Off by default.
    let strict_style = flags
        .get("strict-style")
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
    // UI34 --package-search-path: colon-separated list of directories
    // to search for `mosaic-package.toml` manifests.  Used by the
    // package-reference resolver (resolver.rs) to locate packages
    // named in `pkg::P::C` references inside the consumer's layout.
    let package_search_path = flags
        .get("package-search-path")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let pipeline_any = interface_path.is_some() || layout_path.is_some() || style_path.is_some();

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
            variant,
            output_path,
            emit_project,
            strict_style,
            package_manifest_path.as_deref(),
            package_search_path.as_deref(),
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
            eprintln!("mosaic-compile: warning: ignoring --package-manifest {path}: {e:?}");
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
/// UI30 multi-layout — resolve `--layout` to a concrete file path.
///
/// **Decision table:**
///
/// | `--layout` is | `--variant` is | Result                                                        |
/// |---|---|---|
/// | file path     | any / none     | unchanged (back-compat; warn if variant is set)               |
/// | directory     | None           | `<dir>/<Component>.mll`                                       |
/// | directory     | `Some("desktop")` | `<dir>/<Component>.desktop.mll`, fallback `<Component>.mll`|
/// | directory     | `Some("touch")`   | `<dir>/<Component>.touch.mll`, fallback `<Component>.mll`  |
///
/// **Fallback rationale.** Per UI30 §3.1, a missing variant file falls
/// back to the bare `<Component>.mll` (the default variant). This lets
/// a touch host gracefully degrade to the desktop layout when no
/// touch-specific layout was authored. If BOTH the variant file AND
/// the bare default are missing, this function `process::exit(1)`s
/// with a clear error listing both paths tried.
///
/// **Why not just always exit on missing variant.** The fallback rule
/// is the multi-layout equivalent of CSS's `@media` cascade — most
/// components don't need every form factor, and forcing authors to
/// duplicate the desktop layout into every variant would defeat the
/// purpose. Authors who DO want strict-mode behavior can omit the
/// bare `<Component>.mll` and ship only the variant files; the
/// fallback then provably can't fire.
fn resolve_layout_path(layout_arg: &str, component_name: &str, variant: Option<&str>) -> String {
    // Defense-in-depth: validate component_name + variant against a
    // strict identifier shape before interpolating into path joins.
    // The mosmodel grammar already enforces PascalCase on component
    // declarations (`NAME` token = `[A-Za-z_][A-Za-z0-9_]*`) and the
    // CLI spec describes variant as a kebab-case identifier. Re-
    // checking here costs essentially nothing and would catch:
    //   - a grammar regression that admitted slashes / `..`
    //   - a future codepath that passed a synthetic component name
    //   - hostile `.mil` files in a hypothetical multi-tenant build
    //     server (out of scope today but cheap to guard against)
    // The shape we accept: ASCII letters, digits, `_`, `-`. No `/`,
    // no `.`, no null bytes, no `..`, no shell metacharacters.
    fn is_safe_segment(s: &str) -> bool {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    }
    if !is_safe_segment(component_name) {
        eprintln!(
            "mosaic-compile: refusing to resolve layout for component name \
             '{component_name}' — must be ASCII alphanumeric / _ / -. This \
             usually means the .mil grammar admitted a name it shouldn't \
             have; please file a bug."
        );
        process::exit(1);
    }
    if let Some(v) = variant {
        if v != "default" && !is_safe_segment(v) {
            eprintln!(
                "mosaic-compile: --variant '{v}' contains characters outside \
                 the allowed ASCII alphanumeric / _ / - set"
            );
            process::exit(1);
        }
    }

    let path = Path::new(layout_arg);

    // File-path mode — unchanged behavior. If --variant is set we warn
    // (it's ignored when --layout points at a file) but proceed normally.
    if path.is_file() {
        if variant.is_some() {
            eprintln!(
                "mosaic-compile: warning: --variant is ignored when --layout \
                 points at a file (only meaningful with directory --layout); \
                 using {layout_arg} verbatim"
            );
        }
        return layout_arg.to_string();
    }

    // Directory mode — UI30 resolution.
    if !path.is_dir() {
        eprintln!(
            "mosaic-compile: --layout '{layout_arg}' is neither a file nor a \
             directory (does it exist?)"
        );
        process::exit(1);
    }

    // The reserved variant string `default` is the same as omitting
    // --variant entirely; per spec §3.2 it cannot appear in a filename
    // (`Grid.default.mll` would be redundant with `Grid.mll`).
    let want_variant = variant.filter(|v| *v != "default");

    // Try `<dir>/<Component>.<variant>.mll` first.
    if let Some(v) = want_variant {
        let candidate = path.join(format!("{component_name}.{v}.mll"));
        if candidate.is_file() {
            return candidate.to_string_lossy().into_owned();
        }
    }

    // Fallback: bare `<dir>/<Component>.mll`.
    let bare = path.join(format!("{component_name}.mll"));
    if bare.is_file() {
        return bare.to_string_lossy().into_owned();
    }

    // Neither found — error with a clear "looked for these paths" message.
    let tried: Vec<String> = match want_variant {
        Some(v) => vec![
            format!("{component_name}.{v}.mll"),
            format!("{component_name}.mll"),
        ],
        None => vec![format!("{component_name}.mll")],
    };
    eprintln!(
        "mosaic-compile: no layout file for component '{component_name}'\
         {variant_suffix} in {layout_arg}\n  looked for: {tried}",
        variant_suffix = match want_variant {
            Some(v) => format!(" with variant '{v}'"),
            None => String::new(),
        },
        tried = tried.join(", "),
    );
    process::exit(1);
}

fn run_pipeline(
    backend: &str,
    interface_path: &str,
    layout_path: &str,
    style_path: &str,
    variant: Option<&str>,
    output_path: Option<&str>,
    emit_project: bool,
    strict_style: bool,
    package_manifest_path: Option<&str>,
    package_search_path: Option<&str>,
) {
    // Pipeline mode supports every backend with a `pipeline::from_pipeline`
    // entry point. The mosaic-package-artifact-builder crate calls each
    // backend through the same surface, so they're all wire-compatible
    // here. The `match backend` below dispatches to each. Reject only
    // genuinely-unwired backends ("paint", which is legacy single-file
    // only) with a clear "use legacy SOURCE mode" error.
    if backend == "paint" {
        eprintln!(
            "mosaic-compile: pipeline mode does not support --backend paint \
             (raster output flows through the legacy single-file pipeline). \
             Use SOURCE mode with a .mosaic file."
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

    // -- 1b. Resolve the layout file (UI30 multi-layout) -------------------
    //
    // When `--layout` is a file path, this is a no-op (back-compat with
    // every existing build script). When it's a directory, we resolve
    // <Component>.<variant>.mll inside it with fallback to bare
    // <Component>.mll. See `resolve_layout_path` for the rules.
    let resolved_layout_path =
        resolve_layout_path(layout_path, &mosmodel_out.component.component, variant);

    // -- 2. Compile the moslayout file --------------------------------------
    //
    // We pass the descriptor JSON so the moslayout compiler can check that
    // every `@slot` and `emit onX` reference resolves correctly.
    let layout_path = resolved_layout_path.as_str();
    let layout_src = read_file_or_die(layout_path);
    let mut layout_out =
        moslayout_compiler::compile(&layout_src, Some(&mosmodel_out.descriptor_json))
            .unwrap_or_else(|errs| {
                eprintln!("mosaic-compile: moslayout error(s) in {layout_path}:");
                for e in errs {
                    eprintln!("  {e:?}");
                }
                process::exit(1);
            });

    // -- 2b. UI34 — resolve `pkg::P::C` qualified references -------------
    //
    // Walk the freshly-parsed `LayoutDef` and substitute every qualified
    // reference with the package's resolved sub-tree.  After this pass
    // the layout contains only kernel primitives and same-file local
    // references — exactly the surface every backend emitter already
    // handles.  The resolver is a no-op when the consumer's layout
    // contains zero `pkg::` references, so unqualified-only builds
    // (every pre-UI34 demo) are byte-identical to before.
    //
    // Search paths: explicit `--package-search-path` wins; otherwise
    // we default to `code/packages/` (and, if present, `code/packages/mosaic/`
    // — where the mosaic-pkg-* component family lives) relative to the cwd,
    // which makes monorepo builds work out of the box.  The default is empty
    // (no search) only when neither directory exists, so single-file
    // projects without any packages do not pay an I/O cost.
    let search_paths: Vec<PathBuf> = match package_search_path {
        Some(s) => s.split(':').map(PathBuf::from).collect(),
        None => {
            let base = PathBuf::from("code/packages");
            let mut paths = Vec::new();
            if base.is_dir() {
                paths.push(base.clone());
            }
            let mosaic = base.join("mosaic");
            if mosaic.is_dir() {
                paths.push(mosaic);
            }
            paths
        }
    };
    let resolver = mosaic_package_resolver::LayoutPackageResolver::new(search_paths);
    if let Err(e) = resolver.resolve(&mut layout_out.def) {
        eprintln!("mosaic-compile: package-resolver error in {layout_path}:");
        eprintln!("  {e:?}");
        process::exit(1);
    }
    if let Some(t) = mosaic_package_resolver::first_qualified_tag(&layout_out.def.root) {
        // Defensive — the resolver should leave no qualified tags
        // behind.  If one slips through we exit cleanly rather than
        // letting it confuse the backend emitter.
        eprintln!(
            "mosaic-compile: internal error: package-resolver left \
             qualified tag `{t}` in the layout"
        );
        process::exit(1);
    }
    // Re-run `validate()` on the resolved tree so the part-map JSON
    // reflects the package's inlined parts.  Without this, the
    // consumer's `.msl` would reject the package's part names as
    // unknown — they came from the package's `.mll`, not the
    // consumer's.  Resolution is also a chance for the validator to
    // catch any slot/emit mismatches that survived the package call,
    // surfacing them with the same UnknownSlot / UnknownEmit
    // diagnostics that pre-UI34 builds get.
    let resolved_parts =
        moslayout_compiler::validate(&layout_out.def, Some(&mosmodel_out.descriptor_json))
            .unwrap_or_else(|errs| {
                eprintln!(
                    "mosaic-compile: moslayout post-resolver validation error(s) in {layout_path}:"
                );
                for e in errs {
                    eprintln!("  {e:?}");
                }
                process::exit(1);
            });
    layout_out.parts = resolved_parts;
    layout_out.part_map_json =
        moslayout_compiler::emit_part_map_json(&layout_out.def.component_name, &layout_out.parts);

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

    // -- 3b. Warn on style parts that match no layout part ------------------
    //
    // `mosstyle_compiler::validate` (run inside `compile` above) is deliberately
    // lenient about sub-path part names: it only checks the top-level segment,
    // so a stylesheet that writes `sheet/cell` when the resolved composition
    // exports a flat `cell` compiles cleanly — yet the emitter styles `cell`, so
    // `sheet/cell` targets nothing and the element renders UNSTYLED. That is
    // exactly how the VisiCalc light-theme grid silently lost its gridlines
    // (Grid.light.msl used the legacy `sheet/cell` naming). Surface it here so
    // the typo can't hide: a warning by default, a hard error under
    // --strict-style for CI that wants to fail on stale stylesheets.
    let unmatched =
        mosstyle_compiler::unmatched_parts(&style_out.def, Some(&layout_out.part_map_json));
    if !unmatched.is_empty() {
        let component = &layout_out.def.component_name;
        for u in &unmatched {
            let hint = match &u.suggestion {
                Some(s) => format!(
                    " — did you mean `{s}`? (`{s}` is an exported part; the emitter targets flat part names)"
                ),
                None => String::new(),
            };
            eprintln!(
                "mosaic-compile: warning: style part `{}` in {style_path} matches no part \
                 exported by component `{component}` — it will not be styled{hint}",
                u.name
            );
        }
        if strict_style {
            eprintln!(
                "mosaic-compile: --strict-style: {} unmatched style part(s) — failing.",
                unmatched.len()
            );
            process::exit(1);
        }
    }

    // -- 4. Branch on backend. React emits one .tsx file; XAML emits a
    // triple (.xaml, .xaml.cs, .Event.cs) plus zero-or-more RowVm .cs
    // files (one per `For` block).
    match backend {
        "react" => {
            // UI32-K-react: route through `from_pipeline_with_options`
            // so the `--emit-project` flag activates the Vite shell
            // emission. Bare invocation (emit_project: false) is
            // behaviourally identical to the pre-UI32 single-file
            // path — same TSX bytes, same exit code.
            let mut react_opts = mosaic_emit_react::pipeline::EmitOptions::default();
            react_opts.emit_project = emit_project;
            let result = mosaic_emit_react::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_out.def,
                &react_opts,
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

            // UI32-K-react: when --emit-project is on, write the
            // five Vite shell side-files into the same directory
            // as the component TSX. Mirrors the XAML path below.
            if let Some(proj) = &result.project {
                let side_file_path = |relative: &str| -> String {
                    match std::path::Path::new(&out).parent() {
                        Some(p) if !p.as_os_str().is_empty() => {
                            p.join(relative).to_string_lossy().into_owned()
                        }
                        _ => relative.to_string(),
                    }
                };
                // Flat side-files: package.json / vite.config.ts /
                // index.html / README.md sit next to the .tsx.
                let flat: [(String, &str); 4] = [
                    (side_file_path("package.json"), &proj.package_json),
                    (side_file_path("vite.config.ts"), &proj.vite_config),
                    (side_file_path("index.html"), &proj.index_html),
                    (side_file_path("README.md"), &proj.readme),
                ];
                for (path, src) in &flat {
                    write_file_or_die(path, src);
                    eprintln!("Written: {path}");
                }
                // src/main.tsx is nested per Vite convention.
                let main_tsx_path = side_file_path("src/main.tsx");
                if let Some(parent) = std::path::Path::new(&main_tsx_path).parent() {
                    if !parent.as_os_str().is_empty() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            eprintln!("mosaic-compile: failed to create {}: {e}", parent.display());
                            process::exit(1);
                        }
                    }
                }
                write_file_or_die(&main_tsx_path, &proj.main_tsx);
                eprintln!("Written: {main_tsx_path}");
            }
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
                match base.rfind(['/', '\\']) {
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
        // -------- HTML / WebComponent / SwiftUI / Qt / Flutter -------------
        //
        // All five share the same "single-file output" shape: each
        // backend's `pipeline::from_pipeline` returns one string, we
        // write it to `output_path` (or `<Component>.<ext>` if omitted).
        // Output extensions per backend match the artifact-builder.
        "html" => {
            // UI32-K-html: route through `from_pipeline_with_options`
            // so the `--emit-project` flag activates the standalone-
            // HTML shell emission. Bare invocation (emit_project:
            // false) is byte-identical to pre-UI32 behaviour — same
            // .html fragment, same exit code.
            let mut html_opts = mosaic_emit_html::pipeline::EmitOptions::default();
            html_opts.emit_project = emit_project;
            let result = mosaic_emit_html::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_out.def,
                &html_opts,
            )
            .unwrap_or_else(|e| {
                eprintln!("mosaic-compile: html pipeline emit error: {e}");
                process::exit(1);
            });
            let out = output_path
                .map(str::to_string)
                .unwrap_or_else(|| format!("{}.html", result.component_name));
            write_file_or_die(&out, &result.output);
            eprintln!("Written: {out}");

            // UI32-K-html: when --emit-project is on, write the two
            // shell side-files (index.html + README.md) flat next to
            // the component .html fragment.
            if let Some(proj) = &result.project {
                let side_file_path = |relative: &str| -> String {
                    match std::path::Path::new(&out).parent() {
                        Some(p) if !p.as_os_str().is_empty() => {
                            p.join(relative).to_string_lossy().into_owned()
                        }
                        _ => relative.to_string(),
                    }
                };
                let flat: [(String, &str); 2] = [
                    (side_file_path("index.html"), &proj.index_html),
                    (side_file_path("README.md"), &proj.readme),
                ];
                for (path, src) in &flat {
                    write_file_or_die(path, src);
                    eprintln!("Written: {path}");
                }
            }
        }
        "webcomponent" => {
            // UI32-K-webcomp: route through `from_pipeline_with_options`
            // so the `--emit-project` flag activates the standalone-
            // HTML shell. Bare invocation (emit_project: false) is
            // byte-identical to pre-UI32 behaviour — same .js, no
            // new files.
            let mut wc_opts = mosaic_emit_webcomponent::pipeline::EmitOptions::default();
            wc_opts.emit_project = emit_project;
            let result = mosaic_emit_webcomponent::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_out.def,
                &wc_opts,
            )
            .unwrap_or_else(|e| {
                eprintln!("mosaic-compile: webcomponent pipeline emit error: {e}");
                process::exit(1);
            });
            let out = output_path
                .map(str::to_string)
                .unwrap_or_else(|| format!("{}.js", result.component_name));
            write_file_or_die(&out, &result.output);
            eprintln!("Written: {out}");

            // UI32-K-webcomp: when --emit-project is on, write the
            // two shell side-files (index.html + README.md) flat
            // next to the component .js.
            if let Some(proj) = &result.project {
                let side_file_path = |relative: &str| -> String {
                    match std::path::Path::new(&out).parent() {
                        Some(p) if !p.as_os_str().is_empty() => {
                            p.join(relative).to_string_lossy().into_owned()
                        }
                        _ => relative.to_string(),
                    }
                };
                let flat: [(String, &str); 2] = [
                    (side_file_path("index.html"), &proj.index_html),
                    (side_file_path("README.md"), &proj.readme),
                ];
                for (path, src) in &flat {
                    write_file_or_die(path, src);
                    eprintln!("Written: {path}");
                }
            }
        }
        "swiftui" => {
            // UI32-K-swiftui: route through from_pipeline_with_options
            // so --emit-project activates the SwiftPM macOS shell.
            // Bare invocation is byte-identical to pre-UI32.
            let mut sw_opts = mosaic_emit_swiftui::pipeline::EmitOptions::default();
            sw_opts.emit_project = emit_project;
            let result = mosaic_emit_swiftui::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_out.def,
                &sw_opts,
            )
            .unwrap_or_else(|e| {
                eprintln!("mosaic-compile: swiftui pipeline emit error: {e}");
                process::exit(1);
            });
            let out = output_path
                .map(str::to_string)
                .unwrap_or_else(|| format!("{}.swift", result.component_name));
            write_file_or_die(&out, &result.output);
            eprintln!("Written: {out}");

            // UI32-K-swiftui: emit Package.swift + README.md flat;
            // Sources/App/App.swift nested per SwiftPM convention.
            if let Some(proj) = &result.project {
                let side_file_path = |relative: &str| -> String {
                    match std::path::Path::new(&out).parent() {
                        Some(p) if !p.as_os_str().is_empty() => {
                            p.join(relative).to_string_lossy().into_owned()
                        }
                        _ => relative.to_string(),
                    }
                };
                let flat: [(String, &str); 2] = [
                    (side_file_path("Package.swift"), &proj.package_swift),
                    (side_file_path("README.md"), &proj.readme),
                ];
                for (path, src) in &flat {
                    write_file_or_die(path, src);
                    eprintln!("Written: {path}");
                }
                let app_swift_path = side_file_path("Sources/App/App.swift");
                if let Some(parent) = std::path::Path::new(&app_swift_path).parent() {
                    if !parent.as_os_str().is_empty() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            eprintln!("mosaic-compile: failed to create {}: {e}", parent.display());
                            process::exit(1);
                        }
                    }
                }
                write_file_or_die(&app_swift_path, &proj.app_swift);
                eprintln!("Written: {app_swift_path}");
            }
        }
        "qt" => {
            // UI32-K-qt: route through `from_pipeline_with_options`
            // so --emit-project activates the Qt6 + CMake shell.
            // Bare invocation is byte-identical to pre-UI32.
            let mut qt_opts = mosaic_emit_qt::pipeline::EmitOptions::default();
            qt_opts.emit_project = emit_project;
            let result = mosaic_emit_qt::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_out.def,
                &qt_opts,
            )
            .unwrap_or_else(|e| {
                eprintln!("mosaic-compile: qt pipeline emit error: {e}");
                process::exit(1);
            });
            let out = output_path
                .map(str::to_string)
                .unwrap_or_else(|| format!("{}.qml", result.component_name));
            write_file_or_die(&out, &result.output);
            eprintln!("Written: {out}");

            // UI32-K-qt: emit CMakeLists.txt + main.cpp + qmldir +
            // README.md flat next to the .qml.
            if let Some(proj) = &result.project {
                let side_file_path = |relative: &str| -> String {
                    match std::path::Path::new(&out).parent() {
                        Some(p) if !p.as_os_str().is_empty() => {
                            p.join(relative).to_string_lossy().into_owned()
                        }
                        _ => relative.to_string(),
                    }
                };
                let flat: [(String, &str); 4] = [
                    (side_file_path("CMakeLists.txt"), &proj.cmake_lists),
                    (side_file_path("main.cpp"), &proj.main_cpp),
                    (side_file_path("qmldir"), &proj.qmldir),
                    (side_file_path("README.md"), &proj.readme),
                ];
                for (path, src) in &flat {
                    write_file_or_die(path, src);
                    eprintln!("Written: {path}");
                }
            }
        }
        "flutter" => {
            // UI32-K-flutter: route through `from_pipeline_with_options`
            // so --emit-project activates the Flutter app shell.
            // Bare invocation is byte-identical to pre-UI32.
            let mut fl_opts = mosaic_emit_flutter::pipeline::EmitOptions::default();
            fl_opts.emit_project = emit_project;
            let result = mosaic_emit_flutter::pipeline::from_pipeline_with_options(
                &mosmodel_out.component,
                &layout_out.def,
                &style_out.def,
                &fl_opts,
            )
            .unwrap_or_else(|e| {
                eprintln!("mosaic-compile: flutter pipeline emit error: {e}");
                process::exit(1);
            });
            let out = output_path
                .map(str::to_string)
                .unwrap_or_else(|| format!("{}.dart", result.component_name));
            write_file_or_die(&out, &result.output);
            eprintln!("Written: {out}");

            // UI32-K-flutter: emit pubspec.yaml + README.md flat
            // next to .dart; lib/main.dart nested per Flutter
            // convention.
            if let Some(proj) = &result.project {
                let side_file_path = |relative: &str| -> String {
                    match std::path::Path::new(&out).parent() {
                        Some(p) if !p.as_os_str().is_empty() => {
                            p.join(relative).to_string_lossy().into_owned()
                        }
                        _ => relative.to_string(),
                    }
                };
                let flat: [(String, &str); 2] = [
                    (side_file_path("pubspec.yaml"), &proj.pubspec_yaml),
                    (side_file_path("README.md"), &proj.readme),
                ];
                for (path, src) in &flat {
                    write_file_or_die(path, src);
                    eprintln!("Written: {path}");
                }
                let main_dart_path = side_file_path("lib/main.dart");
                if let Some(parent) = std::path::Path::new(&main_dart_path).parent() {
                    if !parent.as_os_str().is_empty() {
                        if let Err(e) = std::fs::create_dir_all(parent) {
                            eprintln!("mosaic-compile: failed to create {}: {e}", parent.display());
                            process::exit(1);
                        }
                    }
                }
                write_file_or_die(&main_dart_path, &proj.main_dart);
                eprintln!("Written: {main_dart_path}");
            }
        }
        "compose" => {
            // mosaic-emit-compose v0.1.0 — Jetpack Compose /
            // Compose Multiplatform Kotlin codegen.  Targets both
            // Android (Jetpack Compose) and Desktop / iOS / Web
            // (Compose Multiplatform) from the same `.kt` output.
            let result = mosaic_emit_compose::from_pipeline(
                &mosmodel_out.component,
                &layout_out.def,
                &style_out.def,
            )
            .unwrap_or_else(|e| {
                eprintln!("mosaic-compile: compose pipeline emit error: {e}");
                process::exit(1);
            });
            let out = output_path
                .map(str::to_string)
                .unwrap_or_else(|| format!("{}.kt", result.component_name));
            write_file_or_die(&out, &result.output);
            eprintln!("Written: {out}");
        }
        _ => {
            eprintln!("mosaic-compile: unsupported pipeline backend '{backend}'");
            process::exit(1);
        }
    }
}

/// Shared helper for backends whose pipeline emit produces a single
/// output file. Resolves the output path (override or default
/// `<Component>.<ext>`), writes the bytes, and logs to stderr.
///
/// Splitting this out keeps the per-backend match arms one-liner-like
/// — pre-UI30 the React arm was the only single-file path, but with
/// HTML/WebComponent/SwiftUI/Qt/Flutter all wired the same way, a
/// shared helper avoids five copies of the same write+log boilerplate.
fn emit_single_file<E: std::fmt::Display>(
    backend: &str,
    output_path: Option<&str>,
    component_name: &str,
    ext: &str,
    result: Result<String, E>,
) {
    let body = result.unwrap_or_else(|e| {
        eprintln!("mosaic-compile: {backend} pipeline emit error: {e}");
        process::exit(1);
    });
    let out = output_path
        .map(str::to_string)
        .unwrap_or_else(|| format!("{component_name}.{ext}"));
    write_file_or_die(&out, &body);
    eprintln!("Written: {out}");
}

// ===========================================================================
// `pkg` subcommand — package-artifact build (UI29 §4.3)
// ===========================================================================

const PKG_BACKENDS: &str = "react|electron|swiftui|qt|xaml|webcomponent|html|flutter|compose";

fn pkg_backend_from_str(value: &str) -> Option<Backend> {
    match value {
        "react" => Some(Backend::React),
        "electron" => Some(Backend::Electron),
        "swiftui" => Some(Backend::SwiftUI),
        "qt" => Some(Backend::Qt),
        "xaml" => Some(Backend::Xaml),
        "webcomponent" => Some(Backend::WebComponent),
        "html" => Some(Backend::Html),
        "flutter" => Some(Backend::Flutter),
        "compose" => Some(Backend::Compose),
        _ => None,
    }
}

/// Drive `mosaic_package_artifact_builder::build_package` from the CLI.
///
/// Spec (mosaic-compile.json):
///
/// ```text
/// mosaic-compile pkg <PACKAGE_ROOT> --backend <react|electron|swiftui|qt|xaml|webcomponent|html|flutter|compose> --output <DIR> [--emit-project]
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

    // UI32-M: read the same `--emit-project` flag that the single-
    // component path uses. When on, the artifact-builder will write
    // a per-backend project shell mounting the first component
    // alongside the per-component artifacts.
    let emit_project = flags
        .get("emit-project")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Theme selector for style (`.msl`) resolution — the style analogue of
    // the layout `--variant` flag. `--theme light` makes the builder read
    // each component's `<Component>.light.msl` (with fallback to the bare
    // `.msl`). Omitted → theme-agnostic resolution (historical dark default).
    //
    // The theme string is interpolated into a filename
    // (`<Component>.<theme>.msl`) that is then joined onto the package src
    // directory, so — exactly like `--variant` — it must be a single safe
    // path segment. Without this guard a value like `../../etc/passwd` could
    // escape `src/` and coax the compiler into reading an arbitrary file.
    let theme = flags.get("theme").and_then(|v| v.as_str());
    if let Some(t) = theme {
        // Same safe-segment rule the layout resolver applies to component
        // names and `--variant`: ASCII alphanumeric / `_` / `-`, non-empty.
        // No `/`, `.`, `..`, or null bytes — so the value cannot escape `src/`.
        let safe = !t.is_empty()
            && t.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !safe {
            eprintln!(
                "mosaic-compile pkg: --theme '{t}' contains characters outside \
                 the allowed ASCII alphanumeric / _ / - set"
            );
            process::exit(1);
        }
    }

    // Map the string to the typed `Backend`.
    let backend = pkg_backend_from_str(backend_str).unwrap_or_else(|| {
        eprintln!(
            "mosaic-compile pkg: --backend must be one of {PKG_BACKENDS}, got '{backend_str}'"
        );
        process::exit(1);
    });

    let opts = BuildOptions {
        package_root: PathBuf::from(package_root),
        output_root: PathBuf::from(output),
        backend,
        // UI32-M: forward the CLI's --emit-project flag into the
        // artifact-builder. When on, the builder writes a per-
        // backend project shell mounting the first component
        // alongside the per-component artifacts.
        emit_project,
        theme: theme.map(|s| s.to_string()),
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
                eprintln!(
                    "mosaic-compile: cannot create directory {}: {e}",
                    parent.display()
                );
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
                eprintln!(
                    "mosaic-compile: cannot create directory {}: {e}",
                    parent.display()
                );
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

    #[test]
    fn pkg_backend_mapping_exposes_native_and_web_package_backends() {
        assert_eq!(pkg_backend_from_str("react"), Some(Backend::React));
        assert_eq!(pkg_backend_from_str("electron"), Some(Backend::Electron));
        assert_eq!(pkg_backend_from_str("swiftui"), Some(Backend::SwiftUI));
        assert_eq!(pkg_backend_from_str("qt"), Some(Backend::Qt));
        assert_eq!(pkg_backend_from_str("xaml"), Some(Backend::Xaml));
        assert_eq!(
            pkg_backend_from_str("webcomponent"),
            Some(Backend::WebComponent)
        );
        assert_eq!(pkg_backend_from_str("html"), Some(Backend::Html));
        assert_eq!(pkg_backend_from_str("flutter"), Some(Backend::Flutter));
        assert_eq!(pkg_backend_from_str("compose"), Some(Backend::Compose));
        assert_eq!(pkg_backend_from_str("paint"), None);
    }

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

        let r =
            build_self_package_registry(Some(mpath.to_str().unwrap()), "Field", "Mosaic.Generated");
        let reg = r.expect("registry built from valid manifest");
        assert!(
            reg.lookup("Button").is_some(),
            "Button should be registered"
        );
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
        let r =
            build_self_package_registry(Some(mpath.to_str().unwrap()), "Card", "Mosaic.Generated");
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

    // -- UI30 multi-layout — resolve_layout_path tests ----------------------
    //
    // The error paths in resolve_layout_path() call process::exit(1)
    // which would terminate the test runner. We only cover the happy
    // paths here; error behaviour is verified in CHANGELOG via manual
    // smoke tests + would deserve an integration-test harness using
    // std::process::Command in a follow-up if we wanted full coverage.

    fn unique_tmpdir(label: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!(
            "ui30-resolve-{label}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// File-path mode: `resolve_layout_path` returns the path
    /// unchanged regardless of variant. This is the back-compat
    /// guarantee for every existing build script.
    #[test]
    fn resolve_file_path_returns_unchanged() {
        let dir = unique_tmpdir("file-mode");
        let file = dir.join("Grid.desktop.mll");
        std::fs::write(&file, "layout Grid { Box }").unwrap();
        let s = file.to_str().unwrap();

        // Without --variant.
        let r1 = resolve_layout_path(s, "Grid", None);
        assert_eq!(r1, s);

        // With --variant (warning logged; flag ignored).
        let r2 = resolve_layout_path(s, "Grid", Some("touch"));
        assert_eq!(r2, s);

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Directory mode with the requested variant file present: the
    /// variant file is preferred over the bare default.
    #[test]
    fn resolve_directory_picks_variant_file_when_present() {
        let dir = unique_tmpdir("variant-present");
        std::fs::write(dir.join("Grid.touch.mll"), "layout Grid { Box }").unwrap();
        std::fs::write(dir.join("Grid.mll"), "layout Grid { Box }").unwrap();

        let r = resolve_layout_path(dir.to_str().unwrap(), "Grid", Some("touch"));
        assert!(
            r.ends_with("Grid.touch.mll"),
            "expected variant-suffixed path, got {r}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Directory mode with no variant file: falls back to bare
    /// `<Component>.mll` (the default variant). Mirrors CSS's
    /// `@media` cascade — touch host gracefully degrades to desktop.
    #[test]
    fn resolve_directory_falls_back_to_bare_default() {
        let dir = unique_tmpdir("fallback");
        std::fs::write(dir.join("Grid.mll"), "layout Grid { Box }").unwrap();

        let r = resolve_layout_path(dir.to_str().unwrap(), "Grid", Some("touch"));
        // Path should end with `/Grid.mll` (not `Grid.touch.mll`).
        let pb = std::path::PathBuf::from(&r);
        assert_eq!(
            pb.file_name().unwrap().to_str().unwrap(),
            "Grid.mll",
            "expected fallback to bare Grid.mll, got {r}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Directory mode without `--variant` looks only at the bare
    /// default file. The string `default` is reserved per spec §3.2
    /// — supplying it explicitly is equivalent to omitting the flag.
    #[test]
    fn resolve_directory_no_variant_uses_bare_default() {
        let dir = unique_tmpdir("no-variant");
        std::fs::write(dir.join("Grid.mll"), "layout Grid { Box }").unwrap();
        // Decoy: a variant file exists but isn't requested.
        std::fs::write(dir.join("Grid.touch.mll"), "layout Grid { Box }").unwrap();

        let r1 = resolve_layout_path(dir.to_str().unwrap(), "Grid", None);
        let r2 = resolve_layout_path(dir.to_str().unwrap(), "Grid", Some("default"));
        assert!(r1.ends_with("Grid.mll") && !r1.ends_with("Grid.touch.mll"));
        assert_eq!(r1, r2, "--variant default must equal omitting --variant");

        std::fs::remove_dir_all(&dir).ok();
    }
}
