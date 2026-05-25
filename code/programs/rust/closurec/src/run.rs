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

use crate::config::{CompilationLevel, CompilerConfig, IsolationMode};
use crate::defines;
use crate::globs;
use crate::whitespace_only;
use crate::wrapper;
use coding_adventures_javascript_tokens::EsVersion;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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
    /// `--js` pattern expansion failed (invalid glob, no matches,
    /// FS walk error). The inner [`globs::GlobError`] carries the
    /// specific reason and the offending pattern.
    GlobExpansion(globs::GlobError),
    /// `--compilation_level WHITESPACE_ONLY` minification failed.
    /// Carries the underlying [`whitespace_only::MinifyError`].
    Minify(whitespace_only::MinifyError),
    /// `--define / -D` substitution failed (tokenizer rejected the
    /// source). Inner [`defines::DefineError`] carries the message.
    Define(defines::DefineError),
    /// `--output_wrapper_file` couldn't be read.
    Wrapper(wrapper::WrapperError),
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
            CompilerError::GlobExpansion(e) => write!(f, "{e}"),
            CompilerError::Minify(e) => write!(f, "{e}"),
            CompilerError::Define(e) => write!(f, "{e}"),
            CompilerError::Wrapper(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CompilerError {}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Transform a single input source string per the config's
/// `--compilation_level`.
///
/// # Level matrix (CLOC11.06)
///
/// | Level             | Transform                                   |
/// |-------------------|---------------------------------------------|
/// | `WhitespaceOnly`  | strip comments + collapse whitespace        |
/// | `Simple`          | identity (CLOC11.07 lands real passes)      |
/// | `Advanced`        | identity (CLOC11.08)                        |
/// | `Bundle`          | identity (CLOC11.09)                        |
/// | `TranspileOnly`   | identity (CLOC11.10)                        |
///
/// Mapping `config.language.language_in` → `EsVersion`:
/// - `Stable` / `EcmascriptNext` / `Unstable` / anything missing →
///   `EsVersion::latest()`.
/// - A specific year → that EsVersion.
///
/// This mapping lives here (not in `wire.rs`) because it's
/// transform-policy, not flag-mapping.
pub fn transform_source(
    source: &str,
    config: &CompilerConfig,
) -> Result<String, CompilerError> {
    let es_version = map_language_in_to_es_version(config);

    // Step 1 — compilation-level transform.
    let after_level = match config.compilation.level {
        CompilationLevel::WhitespaceOnly => {
            whitespace_only::whitespace_only_minify(source, es_version)
                .map_err(CompilerError::Minify)?
        }
        // CLOC11.07+ will replace each of these with real passes.
        CompilationLevel::Simple
        | CompilationLevel::Advanced
        | CompilationLevel::Bundle
        | CompilationLevel::TranspileOnly => source.to_string(),
    };

    // Step 2 — `--define / -D` substitution (CLOC11.19). Runs
    // *after* the compilation-level transform so that
    // WHITESPACE_ONLY's output is the input to substitution. The
    // ordering matches CC's: defines are applied during
    // compilation, alongside the level's pass set. The
    // composition matters because some `@define`-tagged names
    // can survive comment-stripping (they shouldn't be in
    // comments) but the *whitespace* changes don't affect
    // identifier matching either way.
    //
    // Fast path: with no defines, this is a no-op string copy.
    defines::apply_defines(&after_level, &config.defines.defines, es_version)
        .map_err(CompilerError::Define)
}

/// Project the typed `LanguageVersion` enum into the
/// `EsVersion` the lexer understands. Defaults to the latest
/// known version for the shortcuts (`Stable`, `EcmascriptNext`,
/// `Unstable`) so user code targeting "modern JS" never gets
/// silently restricted.
fn map_language_in_to_es_version(config: &CompilerConfig) -> EsVersion {
    use crate::config::LanguageVersion as L;
    match config.language.language_in {
        L::Ecmascript3 => EsVersion::Es5, // closest available
        L::Ecmascript5 | L::Ecmascript5Strict => EsVersion::Es5,
        L::Ecmascript2015 => EsVersion::Es2015,
        L::Ecmascript2016 => EsVersion::Es2016,
        L::Ecmascript2017 => EsVersion::Es2017,
        L::Ecmascript2018 => EsVersion::Es2018,
        L::Ecmascript2019 => EsVersion::Es2019,
        L::Ecmascript2020 => EsVersion::Es2020,
        L::Ecmascript2021 => EsVersion::Es2021,
        L::Stable | L::EcmascriptNext | L::Unstable | L::NoTranspile => EsVersion::latest(),
    }
}

/// Resolve the list of input files from a `CompilerConfig`.
///
/// Extracted as its own function so glob behavior is unit-testable
/// without touching output writing. v0.2.0 treated `js_patterns`
/// as literal file paths; v0.3.0 (CLOC11.02) replaces that with
/// real glob expansion (see [`crate::globs`]).
pub fn resolve_inputs(config: &CompilerConfig) -> Result<Vec<PathBuf>, CompilerError> {
    globs::expand_js_patterns(&config.io.js_patterns).map_err(CompilerError::GlobExpansion)
}

/// Run the compiler with `config`.
///
/// CLOC11.02: glob-expanded inputs, identity pipeline body. See the
/// module docstring for the future expansion plan.
pub fn run_compiler(config: &CompilerConfig) -> Result<CompilerOutput, CompilerError> {
    // Step 0: identity-banner fallback. Empty argv → friendly
    // banner so users running `closurec` with no flags get a
    // useful response rather than a glob error.
    if config.io.js_patterns.is_empty() {
        return Ok(CompilerOutput {
            stdout_text: "closurec v0.1.0 - identity pipeline\n".to_string(),
            wrote_files: Vec::new(),
        });
    }

    // Step 1: glob-expand --js patterns into a concrete list of
    // files (CLOC11.02). Exclusion patterns (leading `!`) remove
    // from the accumulator. Errors out if any inclusion pattern
    // produces zero matches.
    let inputs = resolve_inputs(config)?;

    // Step 2: read every resolved input, then transform per
    // --compilation_level (CLOC11.06+).
    //
    // Today's level dispatch:
    //   - WHITESPACE_ONLY → strip comments + collapse whitespace
    //     via the token-level minifier (whitespace_only.rs).
    //   - All other levels → identity (concatenate verbatim).
    //
    // CLOC11.07+ replace the SIMPLE/ADVANCED arms with real
    // lex/parse/typecheck/passes/emit pipelines.
    let mut combined = String::new();
    for path in &inputs {
        let contents = fs::read_to_string(path).map_err(|e| CompilerError::InputReadError {
            path: path.clone(),
            kind: e.kind(),
            message: e.to_string(),
        })?;
        let transformed = transform_source(&contents, config)?;
        combined.push_str(&transformed);
        // Closure separates concatenated inputs with a newline so
        // back-to-back files don't end up syntactically merged.
        if !transformed.ends_with('\n') {
            combined.push('\n');
        }
    }

    // Step 3 (CLOC11.30): apply --output_wrapper / --output_wrapper_file.
    // When neither is set, `apply_output_wrapper` short-circuits to
    // a passthrough so the common no-wrapper case stays a simple
    // copy. When a wrapper is set, `%output%` is replaced by the
    // accumulated `combined` JS and `%n%` is replaced by a literal
    // newline. `--output_wrapper_file` (read at this point, not at
    // config-build time) overrides the inline `--output_wrapper`
    // string when both are present.
    let wrapped = wrapper::apply_output_wrapper(
        &combined,
        &config.formatting.output_wrapper,
        config.formatting.output_wrapper_file.as_deref(),
    )
    .map_err(CompilerError::Wrapper)?;

    // Step 3.5 (CLOC11.31): apply --isolation_mode IIFE if set.
    // Layered after the user wrapper, matching CC's pipeline:
    // user wrapper runs first, IIFE wraps the result. So the
    // user's banner sits *inside* the IIFE — same semantics as
    // CC, and the behavior users requesting IIFE expect.
    let isolated = match config.formatting.isolation_mode {
        IsolationMode::Iife => wrapper::apply_iife_wrap(&wrapped),
        IsolationMode::None => wrapped,
    };

    // Step 4: write the output. Two cases:
    //   a) --js_output_file set → write to disk via write_output_file.
    //   b) absent → stdout via the returned `stdout_text`.
    match &config.io.js_output_file {
        Some(path) => {
            write_output_file(path, &isolated)?;
            Ok(CompilerOutput {
                stdout_text: String::new(),
                wrote_files: vec![path.clone()],
            })
        }
        None => Ok(CompilerOutput {
            stdout_text: isolated,
            wrote_files: Vec::new(),
        }),
    }
}

/// Write `contents` to `path`, creating any missing parent
/// directories first.
///
/// # Why auto-create parents
///
/// The upstream Java Closure Compiler creates the
/// `--js_output_file`'s parent directory tree if it doesn't
/// exist. Mirroring that here means a script using
/// `closurec --js_output_file build/dist/app.min.js` works
/// without a preceding `mkdir -p build/dist`. Without this
/// behavior, `fs::write` would fail with
/// `io::ErrorKind::NotFound` on every fresh build.
///
/// # What we deliberately don't do
///
/// - We don't try to expand `~` or `$HOME` in the path; the
///   shell does that. A path the user passes through e.g.
///   `--js_output_file=~/out.js` reaches us as literally `~/out.js`
///   only if the shell didn't expand it (uncommon — usually
///   means the value was quoted), and CC has the same limitation.
/// - We don't `chmod` the created directories; default umask
///   applies (matches CC).
/// - We don't atomically write via a tempfile + rename. CC writes
///   directly too; the disk-half-full scenario is rare enough
///   and CC's behavior is already what users expect.
///
/// Extracted as its own function so the directory-creation
/// behavior is unit-testable independently of `run_compiler`'s
/// glob expansion + concatenation logic.
pub fn write_output_file(path: &Path, contents: &str) -> Result<(), CompilerError> {
    // Create the parent directory tree if needed. `parent()` of
    // a bare filename like `"out.js"` is `Some("")` (the empty
    // path), so we skip the create when the parent is empty —
    // `fs::create_dir_all("")` would error needlessly.
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| CompilerError::OutputWriteError {
                path: parent.to_path_buf(),
                kind: e.kind(),
                message: format!("failed to create parent directory: {e}"),
            })?;
        }
    }
    fs::write(path, contents.as_bytes()).map_err(|e| CompilerError::OutputWriteError {
        path: path.to_path_buf(),
        kind: e.kind(),
        message: e.to_string(),
    })
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
        // CLOC11.02: a missing literal --js path now flows through
        // glob expansion first, which produces a NoMatches error
        // before we ever try to read the file. This matches CC's
        // behavior (a literal `--js missing.js` errors with
        // JSC_NO_JS_FILES_FOUND_FOR_PATTERN, not a low-level "file
        // not found"). We assert the typed GlobExpansion wrapper.
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec!["/nonexistent/path/closurec/test/missing.js".to_string()],
                ..Default::default()
            },
            ..Default::default()
        };
        let err = run_compiler(&cfg).expect_err("missing input must error");
        match err {
            CompilerError::GlobExpansion(inner) => {
                // Inner GlobError must be NoMatches for the literal.
                assert!(format!("{inner}").contains("missing.js"));
            }
            other => panic!("expected GlobExpansion, got {other:?}"),
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

    // ------------------------------------------------------------------
    // CLOC11.03 — write_output_file behavior
    // ------------------------------------------------------------------

    #[test]
    fn write_output_file_creates_missing_parent_directories() {
        // The signature use case: a fresh build that targets
        // `build/dist/app.min.js` without a prior `mkdir -p`.
        let base = temp_path("autocreate");
        let nested = base.join("a").join("b").join("c");
        let out_path = nested.join("result.js");
        assert!(!nested.exists(), "parent dir must not exist pre-write");

        write_output_file(&out_path, "// generated\n").expect("write ok");

        assert!(nested.is_dir(), "parent dir should now exist");
        let written = fs::read_to_string(&out_path).expect("read written file");
        assert_eq!(written, "// generated\n");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn write_output_file_bare_filename_does_not_create_dot() {
        // `parent()` of a bare filename is `Some("")`. We must
        // skip the create_dir_all call rather than asking the OS
        // to create the empty path.
        let dir = temp_path("bare");
        fs::create_dir_all(&dir).expect("setup");
        let bare = dir.join("only.js");
        write_output_file(&bare, "// bare\n").expect("write ok");
        let written = fs::read_to_string(&bare).expect("read");
        assert_eq!(written, "// bare\n");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_output_file_reports_create_dir_failure_as_typed_error() {
        // Attempting to create a directory underneath a regular
        // file should surface as an OutputWriteError pointing at
        // the *parent* — not silently truncate the user's file or
        // panic.
        let blocker = temp_path("blocker.txt");
        fs::write(&blocker, "i am not a directory").expect("setup");
        // Try to write a child under a path that's actually a file.
        let bad = blocker.join("child").join("output.js");
        let err = write_output_file(&bad, "// won't land").expect_err("must error");
        match err {
            CompilerError::OutputWriteError { path, .. } => {
                // Either the create_dir_all hits the blocking
                // file (parent ends in /child) or the write itself
                // does. Either is a meaningful, typed error.
                assert!(
                    path.to_string_lossy().contains("blocker"),
                    "error path should mention the blocker: {path:?}"
                );
            }
            other => panic!("expected OutputWriteError, got {other:?}"),
        }
        let _ = fs::remove_file(&blocker);
    }

    #[test]
    fn run_compiler_autocreates_output_parent_dirs() {
        // End-to-end variant of the auto-create test: drive the
        // whole pipeline (config -> run_compiler -> write) and
        // verify the parent dir got created.
        let base = temp_path("e2e");
        let in_path = base.join("src").join("a.js");
        fs::create_dir_all(in_path.parent().unwrap()).expect("setup in");
        fs::write(&in_path, "var x = 1;\n").expect("write in");

        let out_path = base.join("build").join("dist").join("app.js");
        assert!(!out_path.parent().unwrap().exists());

        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: Some(out_path.clone()),
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert_eq!(out.wrote_files, vec![out_path.clone()]);
        let written = fs::read_to_string(&out_path).expect("read out");
        assert_eq!(written, "var x = 1;\n");
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn run_compiler_stdout_fallback_when_output_file_absent() {
        // CLOC11.01 already supported this; pin it as a CLOC11.03
        // regression test so we can't accidentally break stdout
        // fallback when the output-file path grows new behavior.
        let in_path = temp_path("stdout-fallback.js");
        fs::write(&in_path, "// content\n").expect("setup");
        let cfg = CompilerConfig {
            io: IoConfig {
                js_patterns: vec![in_path.to_string_lossy().to_string()],
                js_output_file: None,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = run_compiler(&cfg).expect("ok");
        assert!(out.wrote_files.is_empty());
        assert!(out.stdout_text.contains("// content"));
        let _ = fs::remove_file(&in_path);
    }
}
