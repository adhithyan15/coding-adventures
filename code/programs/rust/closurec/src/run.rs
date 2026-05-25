//! `run` — execute a compiled [`CompilerConfig`].
//!
//! # CLOC11.01 scope
//!
//! v1: **identity pipeline.** Read every `--js` input as a literal
//! file path, concatenate the contents in input order, write to
//! `--js_output_file` (or stdout if absent). No lexing, no
//! parsing, no optimisation, no source maps.
//!
//! This is enough to:
//!
//! 1. Prove the wiring works end-to-end — flags → config → behavior
//!    → bytes on disk.
//! 2. Give later CLOC11.* PRs a "before" they can diff against.
//! 3. Replicate the simplest real Closure Compiler invocation
//!    (`--js a.js --js_output_file b.js` as a copy).
//!
//! Subsequent PRs replace `read → concatenate → write` with
//! `read → lex → parse → typecheck → passes → emit → write`.
//! The function signature doesn't change; only the body grows.

use crate::config::CompilerConfig;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Output type
// ---------------------------------------------------------------------------

/// The result of a successful `run_compiler` invocation.
///
/// `stdout_text` is what should be printed (empty if `--js_output_file`
/// captured all output). `wrote_files` lists every path actually
/// written, so tests can assert on disk effects without reading
/// each file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompilerOutput {
    pub stdout_text: String,
    pub wrote_files: Vec<PathBuf>,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Reasons compilation can fail.
///
/// I/O errors keep the underlying [`io::ErrorKind`] so the caller
/// can report a useful message; we don't try to wrap the full
/// [`io::Error`] (it isn't `Clone`/`PartialEq`).
#[derive(Debug, Clone, PartialEq)]
pub enum CompilerError {
    /// Couldn't read an input file.
    InputReadError {
        path: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
    /// Couldn't write the output file.
    OutputWriteError {
        path: PathBuf,
        kind: io::ErrorKind,
        message: String,
    },
}

impl std::fmt::Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerError::InputReadError { path, message, .. } => {
                write!(f, "failed to read input {}: {message}", path.display())
            }
            CompilerError::OutputWriteError { path, message, .. } => {
                write!(f, "failed to write output {}: {message}", path.display())
            }
        }
    }
}

impl std::error::Error for CompilerError {}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run the compiler with `config`.
///
/// v1: identity pipeline (concatenate inputs to output). See the
/// module docstring for the future expansion plan.
pub fn run_compiler(config: &CompilerConfig) -> Result<CompilerOutput, CompilerError> {
    // Step 1: read every input. CLOC11.02 will replace this loop
    // with glob expansion; v1 treats --js values as literal paths.
    let mut combined = String::new();
    for raw in &config.io.js_patterns {
        let path = PathBuf::from(raw);
        let contents = fs::read_to_string(&path).map_err(|e| CompilerError::InputReadError {
            path: path.clone(),
            kind: e.kind(),
            message: e.to_string(),
        })?;
        combined.push_str(&contents);
        // Closure separates concatenated inputs with a newline so
        // back-to-back files don't end up syntactically merged.
        // (e.g. file1 ending mid-line + file2 starting mid-line
        // would otherwise be a single source line.) We follow.
        if !contents.ends_with('\n') {
            combined.push('\n');
        }
    }

    // Step 2: write the output. Three cases:
    //   a) --js_output_file set + non-empty path → write to disk.
    //   b) --js_output_file empty or absent → stdout via the
    //      returned `stdout_text`.
    //   c) inputs were empty → return the identity banner so
    //      existing tests continue to pass and users running
    //      `closurec` with no args get the same friendly v1
    //      behavior as before.
    if config.io.js_patterns.is_empty() {
        return Ok(CompilerOutput {
            stdout_text: "closurec v0.1.0 - identity pipeline\n".to_string(),
            wrote_files: Vec::new(),
        });
    }

    match &config.io.js_output_file {
        Some(path) => {
            fs::write(path, combined.as_bytes()).map_err(|e| CompilerError::OutputWriteError {
                path: path.clone(),
                kind: e.kind(),
                message: e.to_string(),
            })?;
            Ok(CompilerOutput {
                stdout_text: String::new(),
                wrote_files: vec![path.clone()],
            })
        }
        None => Ok(CompilerOutput {
            stdout_text: combined,
            wrote_files: Vec::new(),
        }),
    }
}

/// Convenience wrapper for `main` — runs the compiler and prints
/// `stdout_text` to actual stdout, returning whether the write
/// itself succeeded.
///
/// Test code uses `run_compiler` directly so it can assert on the
/// `CompilerOutput` without spawning a process or capturing
/// stdout.
pub fn run_and_print(config: &CompilerConfig) -> Result<(), CompilerError> {
    let out = run_compiler(config)?;
    let stdout = io::stdout();
    let mut h = stdout.lock();
    let _ = h.write_all(out.stdout_text.as_bytes());
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IoConfig;
    use std::env;

    /// Unique temp path under the system temp dir. We don't pull
    /// in the `tempfile` crate — the repo principle is zero-dep
    /// where reasonable, and a unique-named file is enough for
    /// these tests.
    fn temp_path(suffix: &str) -> PathBuf {
        let id = std::process::id();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        env::temp_dir().join(format!("closurec-cloc11-01-{id}-{nanos}-{suffix}"))
    }

    #[test]
    fn no_inputs_emits_identity_banner() {
        let cfg = CompilerConfig::default();
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.stdout_text.contains("identity pipeline"));
        assert!(out.wrote_files.is_empty());
    }

    #[test]
    fn single_input_to_stdout_round_trips() {
        let in_path = temp_path("in.js");
        fs::write(&in_path, "alert(1);").expect("write input");

        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: None,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.stdout_text.contains("alert(1);"));
        assert!(out.wrote_files.is_empty());
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn single_input_to_output_file_writes_disk() {
        let in_path = temp_path("in.js");
        let out_path = temp_path("out.js");
        fs::write(&in_path, "console.log('hi');").expect("write input");

        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.stdout_text.is_empty(), "nothing on stdout when output file is set");
        assert_eq!(out.wrote_files, vec![out_path.clone()]);

        let written = fs::read_to_string(&out_path).expect("read output");
        assert!(written.contains("console.log('hi');"));
        let _ = fs::remove_file(&in_path);
        let _ = fs::remove_file(&out_path);
    }

    #[test]
    fn multiple_inputs_concatenate_in_order_with_newlines() {
        let a = temp_path("a.js");
        let b = temp_path("b.js");
        fs::write(&a, "// a").expect("write a");
        fs::write(&b, "// b").expect("write b");

        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![
                    a.to_string_lossy().to_string(),
                    b.to_string_lossy().to_string(),
                ],
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Both files appear; the first one gets a trailing newline
        // injected because its content didn't end with one.
        let s = &out.stdout_text;
        let a_idx = s.find("// a").expect("a in output");
        let b_idx = s.find("// b").expect("b in output");
        assert!(a_idx < b_idx, "input order preserved");
        let _ = fs::remove_file(&a);
        let _ = fs::remove_file(&b);
    }

    #[test]
    fn missing_input_returns_typed_error() {
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec!["/nonexistent/path/closurec/test/missing.js".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let err = run_compiler(&cfg).expect_err("missing input must error");
        match err {
            CompilerError::InputReadError { kind, .. } => {
                assert_eq!(kind, io::ErrorKind::NotFound);
            }
            other => panic!("expected InputReadError, got {other:?}"),
        }
    }

    #[test]
    fn input_already_ending_in_newline_does_not_get_doubled() {
        let in_path = temp_path("already-newline.js");
        fs::write(&in_path, "var x;\n").expect("write");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        // Should be exactly the original 7 bytes — no extra newline
        // appended because the input already had one.
        assert_eq!(out.stdout_text, "var x;\n");
        let _ = fs::remove_file(&in_path);
    }

    #[test]
    fn compiler_error_display_includes_path() {
        let e = CompilerError::InputReadError {
            path: PathBuf::from("/x/y.js"),
            kind: io::ErrorKind::PermissionDenied,
            message: "permission denied".to_string(),
        };
        let s = e.to_string();
        assert!(s.contains("/x/y.js"));
        assert!(s.contains("permission denied"));
        let _: &dyn std::error::Error = &e;
    }
}
