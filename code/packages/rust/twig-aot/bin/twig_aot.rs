//! `twig-aot` CLI — compile a Twig source file to a native ARM64
//! Mach-O executable.
//!
//! ## Usage
//!
//! ```text
//! twig-aot <FILE.twig> [-o <OUT>]
//! twig-aot --help
//! twig-aot --version
//! ```
//!
//! Argument parsing is driven by [`cli_builder`] — the JSON spec lives
//! in `twig_aot.cli.json` next to this binary's source.  Keeping the
//! shape declarative means `--help` / `--version` / error messages are
//! generated for free, and we don't roll yet another argv parser.

use std::path::PathBuf;
use std::process::ExitCode;

use cli_builder::parser::Parser;
use cli_builder::spec_loader::load_spec_from_str;
use cli_builder::types::ParserOutput;

/// CLI specification embedded at compile time.  The same file ships
/// next to the source so editor tooling can pick it up.
static CLI_SPEC: &str = include_str!("../twig_aot.cli.json");

fn main() -> ExitCode {
    let spec = match load_spec_from_str(CLI_SPEC) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("twig-aot: invalid embedded CLI spec: {e:?}");
            return ExitCode::from(2);
        }
    };
    let parser = Parser::new(spec);

    let args: Vec<String> = std::env::args().collect();
    let outcome = match parser.parse(&args) {
        Ok(o)  => o,
        Err(e) => {
            eprintln!("twig-aot: {e:?}");
            return ExitCode::from(2);
        }
    };

    let result = match outcome {
        ParserOutput::Help(h)    => { print!("{}", h.text);    return ExitCode::SUCCESS; }
        ParserOutput::Version(v) => { println!("{}", v.version); return ExitCode::SUCCESS; }
        ParserOutput::Parse(r)   => r,
    };

    let input_str = match result.arguments.get("input").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None    => { eprintln!("twig-aot: missing input file"); return ExitCode::from(2); }
    };
    let input = PathBuf::from(&input_str);
    let output = result.flags.get("output").and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(|| input.with_extension(""));

    match twig_aot::compile_file_macos_arm64(&input, &output) {
        Ok(())   => ExitCode::SUCCESS,
        Err(e) => { eprintln!("twig-aot: {e}"); ExitCode::from(1) }
    }
}
