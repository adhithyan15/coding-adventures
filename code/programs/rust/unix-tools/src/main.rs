//! # pwd — Print Working Directory
//!
//! A reimplementation of the POSIX `pwd` utility, powered by
//! [CLI Builder](../../../packages/rust/cli-builder/).
//!
//! ## The Core Idea
//!
//! The entire command-line interface — flags, help text, version output,
//! error messages — is defined in `pwd.json`. This program never parses
//! a single argument by hand. Instead:
//!
//! 1. We hand `pwd.json` and the process's argv to CLI Builder's `Parser`.
//! 2. The parser validates the input, enforces mutual exclusivity of
//!    `-L` and `-P`, generates help text, and returns a typed result.
//! 3. We pattern-match on the result variant and run the business logic.
//!
//! The result is that **this file contains only business logic**. All parsing,
//! validation, and help generation happen inside CLI Builder, driven by the
//! JSON spec.
//!
//! ## Logical vs Physical Paths
//!
//! When you `cd` through a symbolic link, the shell updates the `$PWD`
//! environment variable to reflect the path *as you typed it* — including
//! the symlink name. This is the "logical" path.
//!
//! The "physical" path resolves all symlinks. For example, if `/home` is
//! a symlink to `/usr/home`:
//!
//! ```text
//!     Logical:  /home/user       (what $PWD says)
//!     Physical: /usr/home/user   (what the filesystem says)
//! ```
//!
//! By default (`-L`), we print the logical path. With `-P`, we resolve
//! symlinks and print the physical path.
//!
//! ## POSIX Compliance Note
//!
//! If `$PWD` is not set, or if it doesn't match the actual current
//! directory, even `-L` mode falls back to the physical path. This
//! matches POSIX behavior.

use std::path::PathBuf;
use std::process;

use cli_builder::{load_spec_from_file, Parser, ParserOutput};
use unix_tools::{get_logical_pwd, get_physical_pwd};

// ---------------------------------------------------------------------------
// Locating the spec file
// ---------------------------------------------------------------------------
// The `pwd.json` spec file lives alongside the compiled binary. We use
// `std::env::current_exe()` to find the binary's location at runtime, then
// look for `pwd.json` in the same directory. This mirrors how the Python
// version uses `__file__` to locate its spec.
//
// Why not embed the spec? Keeping it as an external file means:
//   - The spec is human-readable and editable without recompilation.
//   - Multiple programs can share the same spec format.
//   - The spec can be validated independently of the binary.

/// Find the `pwd.json` spec file relative to the current executable.
///
/// The spec file is expected to live in the same directory as the binary.
/// During development (`cargo run`), we also check the project root
/// (where Cargo.toml lives) as a fallback, since `cargo run` places the
/// binary in `target/debug/`.
fn find_spec_file() -> PathBuf {
    // --- Strategy 1: Next to the executable ---
    // This is the production case. When the binary is installed or copied
    // alongside pwd.json, this will find it immediately.
    if let Ok(exe_path) = std::env::current_exe() {
        let exe_dir = exe_path.parent().unwrap_or_else(|| std::path::Path::new("."));
        let spec_path = exe_dir.join("pwd.json");
        if spec_path.exists() {
            return spec_path;
        }
    }

    // --- Strategy 2: Project root (development fallback) ---
    // When running via `cargo run`, the binary lives in target/debug/ or
    // target/release/, but pwd.json lives in the project root. We walk up
    // from the executable looking for a directory containing both pwd.json
    // and Cargo.toml.
    if let Ok(exe_path) = std::env::current_exe() {
        let mut dir = exe_path.parent().map(|p| p.to_path_buf());
        while let Some(d) = dir {
            let candidate = d.join("pwd.json");
            if candidate.exists() && d.join("Cargo.toml").exists() {
                return candidate;
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
    }

    // --- Strategy 3: Current directory ---
    // Last resort: maybe the user is running from the project directory.
    let cwd_spec = PathBuf::from("pwd.json");
    if cwd_spec.exists() {
        return cwd_spec;
    }

    // --- Strategy 4: Relative to the source file (compile-time) ---
    // Use the CARGO_MANIFEST_DIR env var set during compilation.
    let manifest_spec = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("pwd.json");
    if manifest_spec.exists() {
        return manifest_spec;
    }

    // If we get here, we can't find the spec. Return the manifest path
    // anyway and let load_spec_from_file produce a clear error.
    manifest_spec
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    // --- Step 1: Locate and load the spec ---
    // The spec file defines the entire CLI: flags, help text, version,
    // mutual exclusivity rules, and error messages.
    let spec_path = find_spec_file();
    let spec = match load_spec_from_file(spec_path.to_string_lossy().as_ref()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("pwd: failed to load spec: {}", e);
            process::exit(1);
        }
    };

    // --- Step 2: Parse command-line arguments ---
    // CLI Builder's Parser takes the spec and argv, then returns one of
    // three result types:
    //   - Parse(result)   — normal invocation with flags/arguments
    //   - Help(result)    — user passed --help
    //   - Version(result) — user passed --version
    let parser = Parser::new(spec);
    let args: Vec<String> = std::env::args().collect();

    let output = match parser.parse(&args) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("pwd: {}", e);
            process::exit(1);
        }
    };

    // --- Step 3: Dispatch on result type ---
    // Pattern matching makes this clean and exhaustive. The compiler
    // ensures we handle every variant.
    match output {
        ParserOutput::Help(help) => {
            println!("{}", help.text);
            process::exit(0);
        }
        ParserOutput::Version(version) => {
            println!("{}", version.version);
            process::exit(0);
        }
        ParserOutput::Parse(result) => {
            // --- Step 4: Business logic ---
            // This is the *only* part specific to the pwd tool.
            // CLI Builder has already validated the flags and enforced
            // mutual exclusivity of -L and -P.
            //
            // We check whether the "physical" flag is set to true.
            // If it is, we resolve symlinks. Otherwise, we use the
            // logical path from $PWD.
            let use_physical = result.flags.get("physical")
                == Some(&serde_json::Value::Bool(true));

            let path = if use_physical {
                get_physical_pwd()
            } else {
                get_logical_pwd()
            };

            match path {
                Ok(p) => println!("{}", p),
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            }
        }
    }
}
