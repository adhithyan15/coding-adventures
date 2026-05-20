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
    let output = cmd.output.unwrap_or_else(|| input.with_extension(""));

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

    match dispatch(&input, &output, language) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => { eprintln!("lang-aot: {e}"); ExitCode::from(1) }
    }
}

struct CliArgs {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    language: Option<Language>,
    help: bool,
}

fn parse_args(args: &[String]) -> Result<CliArgs, String> {
    let mut input = None;
    let mut output = None;
    let mut language = None;
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
    Ok(CliArgs { input, output, language, help })
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
  -o, --output <PATH>   Output executable path (default: input without extension).
  -l, --lang <LANG>     Override language detection (twig, nib, bf, basic, oct).
  -h, --help            Show this help.\
");
}

fn dispatch(input: &Path, output: &Path, language: Language) -> Result<(), String> {
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
