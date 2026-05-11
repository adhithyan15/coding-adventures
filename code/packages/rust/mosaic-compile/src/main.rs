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
//! ## Usage
//!
//! ```text
//! mosaic-compile --backend webcomponent ProfileCard.mosaic
//!   Compile ProfileCard.mosaic to ProfileCard.js
//!
//! mosaic-compile --backend html --fixtures fixture.json ProfileCard.mosaic
//!   Compile with slot values from fixture.json, emit ProfileCard.html
//!
//! mosaic-compile --backend html --css styles.css -o out.html ProfileCard.mosaic
//!   Compile with inlined CSS, write to out.html
//!
//! mosaic-compile --help
//!   Show this usage information.
//!
//! mosaic-compile --version
//!   Print the version.
//! ```
//!
//! ## Options
//!
//! | Flag                  | Short | Description                                        |
//! |-----------------------|-------|----------------------------------------------------|
//! | `--backend <name>`    | `-b`  | Output backend: `webcomponent` or `html` (required) |
//! | `--output <path>`     | `-o`  | Output file path (default: component name + ext)   |
//! | `--fixtures <path>`   | `-f`  | JSON fixture file providing slot values (html only) |
//! | `--css <path>`        | `-c`  | CSS file to inline in HTML output (html only)      |
//! | `--help`              | `-h`  | Show usage information                             |
//! | `--version`           | `-V`  | Print version                                      |

use std::fs;
use std::path::Path;
use std::process;

use mosaic_analyzer::analyze;
use mosaic_emit_html::HtmlRenderer;
use mosaic_emit_webcomponent::WebComponentRenderer;
use mosaic_vm::MosaicVM;

// ===========================================================================
// Version
// ===========================================================================

const VERSION: &str = "0.1.0";

// ===========================================================================
// Usage / help text
// ===========================================================================

/// Print usage information and exit with code 0.
fn print_help_and_exit() -> ! {
    println!(
        r#"mosaic-compile {version}
Compile a .mosaic component file to a target output format.

Reads the unified .mosaic file format (interface + layout + style in one file)
and emits to the specified backend.

USAGE:
    mosaic-compile --backend <BACKEND> [OPTIONS] <SOURCE>

ARGUMENTS:
    SOURCE    Path to the .mosaic source file to compile

FLAGS:
    -b, --backend <name>     Output backend: 'webcomponent' (Custom Element JS)
                             or 'html' (static HTML file) [required]
    -o, --output <path>      Output file path [default: <ComponentName>.js/.html]
    -f, --fixtures <path>    JSON fixture file providing slot values (--backend html)
    -c, --css <path>         CSS file to inline in HTML output (--backend html)
    -h, --help               Show this help message
    -V, --version            Print version

EXAMPLES:
    mosaic-compile --backend webcomponent ProfileCard.mosaic
    mosaic-compile --backend html --fixtures data.json ProfileCard.mosaic
    mosaic-compile --backend html --css styles.css -o out.html ProfileCard.mosaic
"#,
        version = VERSION
    );
    process::exit(0);
}

/// Print the version string and exit.
fn print_version_and_exit() -> ! {
    println!("{VERSION}");
    process::exit(0);
}

// ===========================================================================
// Argument parsing
// ===========================================================================

/// Parsed and validated CLI arguments.
struct Args {
    source: String,
    backend: String,
    output: Option<String>,
    fixtures: Option<String>,
    css: Option<String>,
}

/// Parse `argv` (the full argument list including `argv[0]`) into `Args`.
///
/// On error, prints a message to stderr and exits with code 1.
/// For `--help` or `--version`, prints and exits with code 0.
fn parse_args(argv: &[String]) -> Args {
    let mut backend: Option<String> = None;
    let mut output: Option<String> = None;
    let mut fixtures: Option<String> = None;
    let mut css: Option<String> = None;
    let mut source: Option<String> = None;

    let mut i = 1; // Skip argv[0] (the binary name).
    while i < argv.len() {
        let arg = &argv[i];
        match arg.as_str() {
            "--help" | "-h" => print_help_and_exit(),
            "--version" | "-V" => print_version_and_exit(),

            "--backend" | "-b" => {
                i += 1;
                backend = Some(require_value(&argv, i, "--backend"));
            }
            "--output" | "-o" => {
                i += 1;
                output = Some(require_value(&argv, i, "--output"));
            }
            "--fixtures" | "-f" => {
                i += 1;
                fixtures = Some(require_value(&argv, i, "--fixtures"));
            }
            "--css" | "-c" => {
                i += 1;
                css = Some(require_value(&argv, i, "--css"));
            }
            s if s.starts_with('-') => {
                eprintln!("Unknown flag: {s}");
                eprintln!("Run 'mosaic-compile --help' for usage.");
                process::exit(1);
            }
            _ => {
                if source.is_some() {
                    eprintln!("Unexpected positional argument: {arg}");
                    eprintln!("Only one SOURCE file is accepted.");
                    process::exit(1);
                }
                source = Some(arg.clone());
            }
        }
        i += 1;
    }

    let backend = backend.unwrap_or_else(|| {
        eprintln!("Error: --backend is required.");
        eprintln!("Run 'mosaic-compile --help' for usage.");
        process::exit(1);
    });

    let source = source.unwrap_or_else(|| {
        eprintln!("Error: SOURCE file is required.");
        eprintln!("Run 'mosaic-compile --help' for usage.");
        process::exit(1);
    });

    if backend != "webcomponent" && backend != "html" {
        eprintln!("Error: --backend must be 'webcomponent' or 'html', got '{backend}'.");
        process::exit(1);
    }

    Args {
        source,
        backend,
        output,
        fixtures,
        css,
    }
}

/// Return `argv[i]` or print an error about the missing argument and exit.
fn require_value(argv: &[String], i: usize, flag: &str) -> String {
    argv.get(i).cloned().unwrap_or_else(|| {
        eprintln!("Error: {flag} requires an argument.");
        process::exit(1);
    })
}

// ===========================================================================
// Main
// ===========================================================================

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let args = parse_args(&argv);

    // ---- Read and analyze the source file ------------------------------------

    let source_text = read_file_or_die(&args.source);

    let mosaic_file = analyze(&source_text).unwrap_or_else(|e| {
        eprintln!("Error analyzing {}: {e}", args.source);
        process::exit(1);
    });

    let component_name = mosaic_file.component.name.clone();
    let vm = MosaicVM::new(mosaic_file);

    // ---- Dispatch to the selected backend ------------------------------------

    match args.backend.as_str() {
        "webcomponent" => {
            // Determine default output file name: <ComponentName>.js
            let out_path = args
                .output
                .clone()
                .unwrap_or_else(|| format!("{component_name}.js"));

            let renderer = WebComponentRenderer::new();
            let result = vm.run(renderer).unwrap_or_else(|e| {
                eprintln!("Error during webcomponent compilation: {e}");
                process::exit(1);
            });

            write_file_or_die(&out_path, &result.output);
            eprintln!("Written: {out_path}");
        }

        "html" => {
            // Determine default output file name: <ComponentName>.html
            let out_path = args
                .output
                .clone()
                .unwrap_or_else(|| format!("{component_name}.html"));

            // Load optional fixture JSON.
            let fixtures = if let Some(path) = &args.fixtures {
                let raw = read_file_or_die(path);
                let val: serde_json::Value = serde_json::from_str(&raw).unwrap_or_else(|e| {
                    eprintln!("Error parsing fixtures file {path}: {e}");
                    process::exit(1);
                });
                val.as_object().cloned().unwrap_or_default()
            } else {
                serde_json::Map::new()
            };

            // Load optional CSS, rejecting content that would break out of <style>.
            let css = args.css.as_ref().map(|path| {
                let raw = read_file_or_die(path);
                mosaic_emit_html::sanitize_css(&raw).unwrap_or_else(|e| {
                    eprintln!("Error: CSS file '{path}' rejected for security reasons: {e}");
                    process::exit(1);
                })
            });

            let renderer = HtmlRenderer::new(fixtures, css);
            let result = vm.run(renderer).unwrap_or_else(|e| {
                eprintln!("Error during html compilation: {e}");
                process::exit(1);
            });

            write_file_or_die(&out_path, &result.output);
            eprintln!("Written: {out_path}");
        }

        other => {
            // Should not reach here — caught during arg parsing.
            eprintln!("Unknown backend: {other}");
            process::exit(1);
        }
    }
}

// ===========================================================================
// File I/O helpers
// ===========================================================================

/// Read a file to a String, or print an error and exit.
fn read_file_or_die(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| {
        eprintln!("Cannot read {path}: {e}");
        process::exit(1);
    })
}

/// Write a string to a file, creating parent directories as needed.
fn write_file_or_die(path: &str, content: &str) {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("Cannot create directory {}: {e}", parent.display());
                process::exit(1);
            });
        }
    }
    fs::write(path, content).unwrap_or_else(|e| {
        eprintln!("Cannot write {path}: {e}");
        process::exit(1);
    });
}
