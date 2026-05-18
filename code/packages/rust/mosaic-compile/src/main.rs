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

    if backend != "webcomponent" && backend != "html" && backend != "react" && backend != "paint" {
        eprintln!(
            "mosaic-compile: --backend must be 'webcomponent', 'html', 'react', or 'paint', got '{backend}'"
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
        run_pipeline(backend, interface, layout, style, output_path);
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

/// Run the three-file pipeline path: compile `.mil`, `.mll`, `.msl` to a
/// single output file using the new pipeline-aware backend emitter.
///
/// Currently only `--backend react` is wired here; the other backends will
/// follow when they gain their own pipeline entry points. The legacy
/// `--backend X SOURCE.mosaic` path continues to work unchanged for any of
/// the four backends.
fn run_pipeline(
    backend: &str,
    interface_path: &str,
    layout_path: &str,
    style_path: &str,
    output_path: Option<&str>,
) {
    if backend != "react" {
        eprintln!(
            "mosaic-compile: pipeline mode (--interface/--layout/--style) \
             currently supports only --backend react (got '{backend}'). \
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

    // -- 4. Lower the triple to a React TSX file ----------------------------
    let result = mosaic_emit_react::pipeline::from_pipeline(
        &mosmodel_out.component,
        &layout_out.def,
        &style_out.def,
    )
    .unwrap_or_else(|e| {
        eprintln!("mosaic-compile: react pipeline emit error: {e}");
        process::exit(1);
    });

    // -- 5. Write the output ------------------------------------------------
    let out = output_path
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}.tsx", result.component_name));
    write_file_or_die(&out, &result.output);
    eprintln!("Written: {out}");
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
