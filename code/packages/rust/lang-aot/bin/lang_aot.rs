//! `lang-aot` CLI — multi-language AOT driver.
//!
//! ## Usage
//!
//! ```text
//! lang-aot <FILE> [-o <OUT>] [--lang <LANG>]
//! lang-aot --help
//! ```
//!
//! `--lang` is optional: if omitted, the language is inferred from the
//! input's file extension (`.twig`, `.nib`, `.bf`, `.bas`, `.oct`).
//! When inference fails, the CLI prints the recognised extensions and
//! exits non-zero.
//!
//! The output target is **always** the build host's platform — same
//! V1 host-targets-host policy as `twig-aot`.  Use `twig-aot` (or
//! `lang-aot` once `--target`/`--emit-object` land here) for cross-OS
//! workflows.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use lang_aot::{detect_language_from_path, Language};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cmd = parse_args(&args);
    let cmd = match cmd {
        Ok(c) => c,
        Err(e) => { eprintln!("lang-aot: {e}"); return ExitCode::from(2); }
    };

    if cmd.help {
        print_help();
        return ExitCode::SUCCESS;
    }

    let input = match cmd.input {
        Some(p) => p,
        None => { eprintln!("lang-aot: missing input file"); print_help(); return ExitCode::from(2); }
    };
    // Default output extension depends on emit mode:
    //   * Native → strip extension (foo.bas → foo)
    //   * LlvmIr → .ll (foo.bas → foo.ll), matching downstream tooling
    let output = cmd.output.unwrap_or_else(|| match cmd.emit {
        EmitMode::Native => input.with_extension(""),
        EmitMode::LlvmIr => input.with_extension("ll"),
    });

    let language = match cmd.language {
        Some(l) => l,
        None => match detect_language_from_path(&input) {
            Some(l) => l,
            None => {
                eprintln!("lang-aot: could not infer language from {:?}; pass --lang explicitly", input);
                eprintln!("  recognised extensions: .twig .nib .bf .bas .oct");
                return ExitCode::from(2);
            }
        },
    };

    match dispatch(&input, &output, language, cmd.emit) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("lang-aot: {e}"); ExitCode::from(1) }
    }
}

/// Choice of emission target.
///
/// `Native` is the default and matches the pre-LLVM04 behaviour: produce a
/// native executable for the build host (Linux ELF / Windows PE / macOS
/// Mach-O).  `LlvmIr` produces a `.ll` textual LLVM IR file; the LLVM
/// toolchain (`llc` / `opt`) is the caller's job to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmitMode {
    Native,
    LlvmIr,
}

struct CliArgs {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    language: Option<Language>,
    emit: EmitMode,
    help: bool,
}

fn parse_args(args: &[String]) -> Result<CliArgs, String> {
    let mut input = None;
    let mut output = None;
    let mut language = None;
    let mut emit = EmitMode::Native;
    let mut help = false;
    let mut i = 1;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-h" | "--help" => help = true,
            "-o" | "--output" => {
                i += 1;
                let v = args.get(i).ok_or_else(|| "-o requires a value".to_string())?;
                output = Some(PathBuf::from(v));
            }
            s if s.starts_with("--output=") => {
                output = Some(PathBuf::from(&s["--output=".len()..]));
            }
            "-l" | "--lang" => {
                i += 1;
                let v = args.get(i).ok_or_else(|| "--lang requires a value".to_string())?;
                language = Some(Language::parse(v)?);
            }
            s if s.starts_with("--lang=") => {
                language = Some(Language::parse(&s["--lang=".len()..])?);
            }
            // --emit=<native|llvm-ir>  (LLVM04)
            //
            // We accept `llvm-ir` as the canonical spelling (matches the
            // file extension downstream — `foo.ll` files are "LLVM IR").
            // `native` is included for explicitness even though it's the
            // default.
            s if s.starts_with("--emit=") => {
                emit = match &s["--emit=".len()..] {
                    "native" => EmitMode::Native,
                    "llvm-ir" | "llvm" | "ll" => EmitMode::LlvmIr,
                    other => return Err(format!("unknown --emit value {other:?}; expected `native` or `llvm-ir`")),
                };
            }
            "--emit" => {
                i += 1;
                let v = args.get(i).ok_or_else(|| "--emit requires a value".to_string())?;
                emit = match v.as_str() {
                    "native" => EmitMode::Native,
                    "llvm-ir" | "llvm" | "ll" => EmitMode::LlvmIr,
                    other => return Err(format!("unknown --emit value {other:?}; expected `native` or `llvm-ir`")),
                };
            }
            s if s.starts_with('-') => {
                return Err(format!("unknown flag {s:?}"));
            }
            _ => {
                if input.is_some() {
                    return Err(format!("unexpected extra argument {arg:?}"));
                }
                input = Some(PathBuf::from(arg));
            }
        }
        i += 1;
    }
    Ok(CliArgs { input, output, language, emit, help })
}

fn print_help() {
    println!("\
Usage: lang-aot <FILE> [-o <OUT>] [--lang <LANG>]

Compile a source file in one of the supported LANG VM languages to a
native executable on the build host's platform.

Supported languages:
  twig            (.twig)        — full
  nib             (.nib)         — full
  brainfuck / bf  (.bf, .b)      — full
  dartmouth-basic (.bas, .basic) — TODO (no IIR frontend yet)
  oct             (.oct)         — TODO (no Rust frontend yet)

Options:
  -o, --output <PATH>      Output path. Default: input without extension
                           (native) or with .ll extension (--emit=llvm-ir).
  -l, --lang <LANG>        Override language detection (twig, nib, bf, basic, oct).
      --emit=<MODE>        What to emit. `native` (default) → host executable;
                           `llvm-ir` (alias `llvm`, `ll`) → textual LLVM IR (.ll)
                           via iir-to-llvm, cross-platform.  Downstream `opt`/`llc`
                           are the caller's responsibility.
  -h, --help               Show this help.\
");
}

fn dispatch(
    input: &Path,
    output: &Path,
    language: Language,
    emit: EmitMode,
) -> Result<(), String> {
    // LLVM IR emission is cross-platform — short-circuit before any host
    // cfg gating below.  Reading a file and writing a `.ll` string out
    // doesn't depend on the linker or platform binary format.
    if emit == EmitMode::LlvmIr {
        return lang_aot::compile_file_to_llvm_ir(input, output, language)
            .map_err(|e| format!("{e}"));
    }
    #[cfg(target_os = "linux")]
    { lang_aot::compile_file_to_linux_executable(input, output, language)
          .map_err(|e| format!("{e}")) }
    #[cfg(target_os = "windows")]
    { lang_aot::compile_file_to_windows_executable(input, output, language)
          .map_err(|e| format!("{e}")) }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    { lang_aot::compile_file_to_macos_executable(input, output, language)
          .map_err(|e| format!("{e}")) }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "windows",
        all(target_os = "macos", target_arch = "aarch64"),
    )))]
    { let _ = (input, output, language);
      Err("lang-aot: this host platform is not supported \
           (host-targets-host only; supported hosts: Linux x86-64, \
           Windows x86-64, macOS ARM64)".into()) }
}
