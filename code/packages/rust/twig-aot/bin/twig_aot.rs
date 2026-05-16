//! `twig-aot` CLI — compile a Twig source file to a native executable.
//!
//! ## Usage
//!
//! ```text
//! twig-aot <FILE.twig> [-o <OUT>] [--target <TRIPLE>]
//! twig-aot --help
//! twig-aot --version
//! ```
//!
//! `--target` selects the output platform:
//! - `auto` (default) — picks the build host's platform.
//! - `macos-arm64` — Apple Silicon Mach-O executable (requires macOS host).
//! - `linux-x86_64` — Linux ELF64 executable (requires Linux host in V1).
//! - `windows-x86_64` — Windows PE executable (requires Windows host in V1).
//!
//! Argument parsing is driven by [`cli_builder`] — the JSON spec lives
//! in `twig_aot.cli.json` next to this binary's source.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cli_builder::parser::Parser;
use cli_builder::spec_loader::load_spec_from_str;
use cli_builder::types::ParserOutput;

/// CLI specification embedded at compile time.
static CLI_SPEC: &str = include_str!("../twig_aot.cli.json");

/// Output target the CLI dispatches to.
///
/// Resolution order:
/// 1. If `--target <triple>` is given, use it.
/// 2. Otherwise (or with `--target auto`), pick the build host's
///    triple via `cfg(target_os)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    MacosArm64,
    LinuxX86_64,
    WindowsX86_64,
}

impl Target {
    /// Parse a `--target` value.  `auto` (or unspecified) picks the host.
    fn parse(s: &str) -> Result<Self, String> {
        match s {
            "auto" => Ok(Self::host()),
            "macos-arm64"    | "aarch64-apple-darwin"     => Ok(Self::MacosArm64),
            "linux-x86_64"   | "x86_64-unknown-linux-gnu" => Ok(Self::LinuxX86_64),
            "windows-x86_64" | "x86_64-pc-windows-msvc"   => Ok(Self::WindowsX86_64),
            other => Err(format!(
                "unknown target {other:?}; expected one of: auto, \
                 macos-arm64, linux-x86_64, windows-x86_64")),
        }
    }

    /// Resolve to the build host's target.  Unsupported hosts fall
    /// through to `MacosArm64` so we don't fail to compile the CLI
    /// itself; the actual dispatch below errors with a clear message.
    fn host() -> Self {
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        { Self::MacosArm64 }
        #[cfg(target_os = "linux")]
        { Self::LinuxX86_64 }
        #[cfg(target_os = "windows")]
        { Self::WindowsX86_64 }
        #[cfg(not(any(
            all(target_os = "macos", target_arch = "aarch64"),
            target_os = "linux",
            target_os = "windows",
        )))]
        { Self::MacosArm64 }
    }
}

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

    let target_str = result.flags.get("target")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");
    let target = match Target::parse(target_str) {
        Ok(t) => t,
        Err(e) => { eprintln!("twig-aot: {e}"); return ExitCode::from(2); }
    };

    let result = dispatch(target, &input, &output);
    match result {
        Ok(())   => ExitCode::SUCCESS,
        Err(msg) => { eprintln!("twig-aot: {msg}"); ExitCode::from(1) }
    }
}

/// Route to the right `compile_file_*` entry point for `target`.
///
/// Each branch is cfg-gated to the matching host OS — `twig-aot`'s V1
/// supports only host-targets-host AOT.  On a non-matching host the
/// branch returns a clear error.  Cross-OS compilation is tracked as
/// a separate follow-up.
fn dispatch(target: Target, input: &Path, output: &Path) -> Result<(), String> {
    match target {
        Target::MacosArm64 => {
            #[cfg(unix)]
            { twig_aot::compile_file_macos_arm64(input, output)
                  .map_err(|e| format!("{e}")) }
            #[cfg(not(unix))]
            { let _ = (input, output);
              Err("--target=macos-arm64 requires a Unix host (macOS or \
                   Linux with cross-toolchain — cross-toolchain not yet \
                   supported)".into()) }
        }
        Target::LinuxX86_64 => {
            #[cfg(target_os = "linux")]
            { twig_aot::compile_file_linux_x86_64(input, output)
                  .map_err(|e| format!("{e}")) }
            #[cfg(not(target_os = "linux"))]
            { let _ = (input, output);
              Err("--target=linux-x86_64 requires a Linux x86-64 host \
                   in V1 (cross-OS compilation is a separate follow-up)".into()) }
        }
        Target::WindowsX86_64 => {
            #[cfg(target_os = "windows")]
            { twig_aot::compile_file_windows_x86_64(input, output)
                  .map_err(|e| format!("{e}")) }
            #[cfg(not(target_os = "windows"))]
            { let _ = (input, output);
              Err("--target=windows-x86_64 requires a Windows x86-64 host \
                   in V1 (cross-OS compilation is a separate follow-up)".into()) }
        }
    }
}
