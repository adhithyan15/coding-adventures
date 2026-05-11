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
//!      ├── --backend webcomponent  →  MyComponent.js  (Custom Element)
//!      └── --backend html          →  MyComponent.html (static snapshot)
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
use mosaic_emit_webcomponent::WebComponentRenderer;
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

    if backend != "webcomponent" && backend != "html" {
        eprintln!(
            "mosaic-compile: --backend must be 'webcomponent' or 'html', got '{backend}'"
        );
        process::exit(1);
    }

    // Required positional: SOURCE
    let source_path = args
        .get("source")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| {
            eprintln!("mosaic-compile: SOURCE file is required");
            process::exit(1);
        });

    // Optional flags
    let output_path = flags.get("output").and_then(|v| v.as_str());
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

        other => {
            // Should not reach here — caught above.
            eprintln!("mosaic-compile: unknown backend '{other}'");
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
