//! `md2docx` — CLI: convert a Markdown file to a real Word `.docx`, natively.
//!
//! ```text
//!   md2docx <in.md> [out.docx]   convert a Markdown file (default out: in.docx)
//!   md2docx --gfm <in.md> [out]  use GitHub-Flavored Markdown (tables, tasks, …)
//!   md2docx --demo [out.docx]    convert the built-in sample (default: md2docx-demo.docx)
//!   md2docx --help               show this help
//! ```
//!
//! The whole path is native, zero-dependency Rust:
//! Markdown → `commonmark-parser`/`gfm-parser` → `document_ast::DocumentNode` →
//! `document-ast-to-docx` → `docx-writer` → `.docx` bytes.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use md2docx::{convert, Dialect, SAMPLE_MARKDOWN};

const HELP: &str = "\
md2docx — convert Markdown to a real Word .docx, natively

USAGE:
  md2docx <in.md> [out.docx]    convert a Markdown file (default out: <in>.docx)
  md2docx --gfm <in.md> [out]   parse as GitHub-Flavored Markdown (tables, task lists)
  md2docx --demo [out.docx]     convert the built-in sample (default: md2docx-demo.docx)
  md2docx --help                show this help";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut rest: Vec<&str> = Vec::new();
    let mut dialect = Dialect::CommonMark;
    let mut demo = false;

    for arg in &args {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return ExitCode::SUCCESS;
            }
            "--gfm" => dialect = Dialect::Gfm,
            "--demo" => demo = true,
            other if other.starts_with('-') => {
                eprintln!("md2docx: unknown option '{other}'\n\n{HELP}");
                return ExitCode::FAILURE;
            }
            other => rest.push(other),
        }
    }

    // --demo: convert the bundled sample; optional first positional = output path.
    if demo {
        let out = rest
            .first()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("md2docx-demo.docx"));
        return write_docx(convert(SAMPLE_MARKDOWN, dialect), &out);
    }

    // Otherwise: <in.md> [out.docx].
    let input = match rest.first() {
        Some(p) => *p,
        None => {
            eprintln!("md2docx: no input file\n\n{HELP}");
            return ExitCode::FAILURE;
        }
    };
    let source = match std::fs::read_to_string(input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("md2docx: cannot read '{input}': {e}");
            return ExitCode::FAILURE;
        }
    };
    let out = match rest.get(1) {
        Some(o) => PathBuf::from(o),
        None => {
            // No explicit output: derive `<in>.docx`. Refuse if that equals the
            // input (i.e. the input is already named `.docx`), so we never
            // silently overwrite the source with its own conversion — the caller
            // must name an output path in that case.
            let derived = default_output(input);
            if derived == Path::new(input) {
                eprintln!(
                    "md2docx: input '{input}' is already .docx — specify an output path \
                     (`md2docx {input} out.docx`) so the input isn't overwritten"
                );
                return ExitCode::FAILURE;
            }
            derived
        }
    };
    write_docx(convert(&source, dialect), &out)
}

/// The default output path: the input with its extension swapped to `.docx`.
fn default_output(input: &str) -> PathBuf {
    Path::new(input).with_extension("docx")
}

/// Write the `.docx` bytes, reporting the result.
fn write_docx(bytes: Vec<u8>, out: &Path) -> ExitCode {
    match std::fs::write(out, &bytes) {
        Ok(()) => {
            println!("md2docx: wrote {} ({} bytes)", out.display(), bytes.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("md2docx: cannot write '{}': {e}", out.display());
            ExitCode::FAILURE
        }
    }
}
