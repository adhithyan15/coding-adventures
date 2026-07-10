//! # sir-conformance — cross-backend golden conformance harness
//!
//! This crate answers one question, by *running code*: **does the same Ruby
//! program produce the same output on every backend?**
//!
//! The Semantic IR (SIR) is a narrow waist — one Ruby frontend
//! ([`ruby_to_semantic_ir`]) lowers to a language-agnostic IR, and several
//! backends emit target source (Python, JavaScript, Go, Rust). The promise of
//! that design is *behavioural equivalence*: a program's observable result must
//! not depend on which backend emitted it. Nothing enforced that promise
//! end-to-end — each backend's own `compile_and_run_*` test hand-builds an SIR
//! module and checks one feature in isolation, so a construct the frontend
//! emits but a backend never implemented could pass every unit test and still
//! crash at runtime. (That is exactly what happened with the `case_eq` builtin,
//! which was missing from three of five runtimes; see the repo's `lessons.md`.)
//!
//! This harness closes that gap. It takes a corpus of **real Ruby source**
//! programs, each paired with **one expected stdout** (the reference answer,
//! written once), lowers each through the actual frontend, emits it through
//! *every* backend, runs the emitted program through that backend's *real*
//! toolchain (`python3`, `node`, `go`, `rustc`), and asserts the output equals
//! the reference — **byte for byte, on every backend**. A disagreement is a
//! faithfulness bug, localised to `(program, backend)`.
//!
//! This is the first, concrete slice of the conformance matrix specified in
//! [`SIR21` §Provability](../../../specs/SIR21-type-system-and-integer-semantics.md).
//!
//! ## Reference-oracle discipline
//!
//! The expected output is compared against **the reference**, never against
//! another backend — so two backends agreeing on a *wrong* answer cannot hide a
//! bug. The reference is the value the Ruby program would print under a real
//! Ruby interpreter; we encode it as a literal string in the corpus.
//!
//! ## Graceful degradation
//!
//! A backend whose toolchain is absent on the host is **skipped**, not failed
//! (mirroring the per-backend `compile_and_run_*` convention). The harness
//! proves what it can on the host it runs on; locally, with all toolchains
//! present, it proves the full matrix.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use ruby_to_semantic_ir::compile_source;
use semantic_ir::{BackendErrorKind, Module};

pub mod oracle;

/// One conformance program: a name, its Ruby source, and the **reference**
/// stdout it must produce on every backend.
#[derive(Debug, Clone)]
pub struct Program {
    /// Short identifier used in temp filenames and assertion messages.
    pub name: &'static str,
    /// The Ruby source to lower and run.
    pub ruby: &'static str,
    /// The exact stdout a real Ruby interpreter would produce (trailing
    /// whitespace is normalised away before comparison, so a single trailing
    /// newline need not be encoded).
    pub expected: &'static str,
}

/// A backend target and the toolchain that runs its emitted source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Python,
    JavaScript,
    Go,
    Rust,
    C,
}

impl Target {
    /// Every target the harness knows how to run.
    pub fn all() -> &'static [Target] {
        &[
            Target::Python,
            Target::JavaScript,
            Target::Go,
            Target::Rust,
            Target::C,
        ]
    }

    /// Human-readable tag for assertion messages.
    pub fn tag(self) -> &'static str {
        match self {
            Target::Python => "python",
            Target::JavaScript => "javascript",
            Target::Go => "go",
            Target::Rust => "rust",
            Target::C => "c",
        }
    }

    /// The executable that must be on `PATH` for this target to run.  (C is
    /// special — it discovers its compiler via [`c_compiler`], honouring the
    /// `SIR_CC` override — so this default is only its PATH fallback.)
    fn toolchain(self) -> &'static str {
        match self {
            Target::Python => "python3",
            Target::JavaScript => "node",
            Target::Go => "go",
            Target::Rust => "rustc",
            Target::C => "cc",
        }
    }

    /// The version-probe argument for this toolchain. Most accept `--version`,
    /// but the Go CLI uses the bare subcommand `go version` (dashes make it
    /// error), which would otherwise make the harness silently skip Go.
    fn version_arg(self) -> &'static str {
        match self {
            Target::Go => "version",
            _ => "--version",
        }
    }

    /// Is this target's toolchain available on the host? A missing toolchain
    /// means the caller should *skip* (not fail) that target.
    pub fn available(self) -> bool {
        if self == Target::C {
            return c_compiler().is_some();
        }
        Command::new(self.toolchain())
            .arg(self.version_arg())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

/// Discover a gcc/clang-style C compiler for the C backend: the `SIR_CC`
/// environment variable first (an absolute path works — handy on Windows),
/// then `cc` / `clang` / `gcc` on `PATH`. Returns `None` when none is present,
/// so the C cell *skips* rather than fails. MSVC `cl` uses a different CLI and
/// is verified by the repo's separate C harness, not here.
fn c_compiler() -> Option<String> {
    if let Ok(cc) = std::env::var("SIR_CC") {
        if !cc.trim().is_empty() {
            return Some(cc);
        }
    }
    for cand in ["cc", "clang", "gcc"] {
        let ok = Command::new(cand)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            return Some(cand.to_string());
        }
    }
    None
}

/// The outcome of running one program through one backend.
#[derive(Debug)]
pub enum RunOutcome {
    /// The program compiled and ran; carries its normalised stdout.
    Ran(String),
    /// The toolchain (or a usable linker) was absent — skip, do not fail.
    Skipped(String),
    /// A genuine failure: emit error, compile error, or non-zero exit.
    Failed(String),
}

/// Lower Ruby `source` (named `name`) to SIR. The source-level primitive used
/// by both the static [`Program`] corpus and the oracle-derived arithmetic
/// cases (whose source is generated at runtime, so it cannot be a `&'static`
/// [`Program`]).
pub fn lower_source(name: &str, source: &str) -> Result<Module, String> {
    compile_source(source, name)
        .map_err(|e| format!("frontend failed to lower `{name}`: {e:?}"))
}

/// Lower `program.ruby` to SIR (shared by every backend so the *frontend* runs
/// exactly once per program, as it would in production).
pub fn lower(program: &Program) -> Result<Module, String> {
    lower_source(program.name, program.ruby)
}

/// Run Ruby `source` (named `name`) through one `target`: lower, emit, compile
/// if needed, execute through the real toolchain, and return the normalised
/// stdout. This is the source-level primitive [`run`] delegates to; it accepts
/// a runtime-generated `&str` so oracle-derived programs can drive it.
pub fn run_source(name: &str, source: &str, target: Target) -> RunOutcome {
    if !target.available() {
        return RunOutcome::Skipped(format!("{} not on PATH", target.toolchain()));
    }
    let module = match lower_source(name, source) {
        Ok(m) => m,
        Err(e) => return RunOutcome::Failed(e),
    };
    match target {
        Target::Python => run_python(name, &module),
        Target::JavaScript => run_javascript(name, &module),
        Target::Go => run_go(name, &module),
        Target::Rust => run_rust(name, &module),
        Target::C => run_c(name, &module),
    }
}

/// Run one `program` through one `target`: emit, compile if needed, execute
/// through the real toolchain, and return the normalised stdout.
pub fn run(program: &Program, target: Target) -> RunOutcome {
    run_source(program.name, program.ruby, target)
}

/// Normalise stdout for comparison: unify CRLF→LF and strip trailing newlines
/// so the assertion tests *content*, not a platform's newline convention.
fn normalise(s: &str) -> String {
    s.replace("\r\n", "\n").trim_end_matches('\n').to_string()
}

fn temp_path(name: &str, target: Target, ext: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "sir_conf_{}_{}_{}{ext}",
        name,
        target.tag(),
        std::process::id()
    ));
    p
}

// ── Python ────────────────────────────────────────────────────────────────
//
// The Python backend emits code that imports the `sir-runtime-*` packages
// rather than inlining a runtime, so `python3` must see them on `PYTHONPATH`.
// We discover the package `src/` directories relative to this crate (they live
// under `code/packages/python/sir-runtime-*/src`) so the harness is
// self-locating and needs no environment setup by the caller.

fn python_runtime_path() -> Option<String> {
    // CARGO_MANIFEST_DIR = <repo>/code/packages/rust/sir-conformance
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let py_dir = manifest.parent()?.parent()?.join("python");
    let mut parts: Vec<String> = Vec::new();
    for entry in fs::read_dir(&py_dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("sir-runtime-") {
            let src = entry.path().join("src");
            if src.is_dir() {
                parts.push(src.to_string_lossy().into_owned());
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(":"))
    }
}

fn run_python(name: &str, module: &Module) -> RunOutcome {
    let artifact = match semantic_ir_to_python::compile(module) {
        Ok(a) => a,
        Err(e) => return RunOutcome::Failed(format!("python emit failed: {e:?}")),
    };
    let path = temp_path(name, Target::Python, ".py");
    if fs::write(&path, &artifact.source).is_err() {
        return RunOutcome::Failed("could not write temp .py".into());
    }
    let Some(pythonpath) = python_runtime_path() else {
        return RunOutcome::Skipped("sir-runtime-* python packages not found".into());
    };
    let out = Command::new("python3")
        .arg(&path)
        .env("PYTHONPATH", pythonpath)
        .output();
    let _ = fs::remove_file(&path);
    finish(out, &artifact.source, "python")
}

// ── JavaScript ──────────────────────────────────────────────────────────────

fn run_javascript(name: &str, module: &Module) -> RunOutcome {
    let artifact = match semantic_ir_to_javascript::compile(module) {
        Ok(a) => a,
        Err(e) => return RunOutcome::Failed(format!("javascript emit failed: {e:?}")),
    };
    let path = temp_path(name, Target::JavaScript, ".js");
    if fs::write(&path, &artifact.source).is_err() {
        return RunOutcome::Failed("could not write temp .js".into());
    }
    let out = Command::new("node").arg(&path).output();
    let _ = fs::remove_file(&path);
    finish(out, &artifact.source, "javascript")
}

// ── Go ──────────────────────────────────────────────────────────────────────

fn run_go(name: &str, module: &Module) -> RunOutcome {
    let artifact = match semantic_ir_to_go::compile(module) {
        Ok(a) => a,
        Err(e) => return RunOutcome::Failed(format!("go emit failed: {e:?}")),
    };
    let path = temp_path(name, Target::Go, ".go");
    if fs::write(&path, &artifact.source).is_err() {
        return RunOutcome::Failed("could not write temp .go".into());
    }
    let out = Command::new("go").arg("run").arg(&path).output();
    let _ = fs::remove_file(&path);
    finish(out, &artifact.source, "go")
}

// ── Rust ────────────────────────────────────────────────────────────────────
//
// The Rust backend emits a self-contained `.rs`; we compile with `rustc` (an
// optional `SIR_TEST_RUSTC_LINKER` override lets hosts without the default
// linker point at `rust-lld`), then run the binary. A missing linker is a
// *skip*, not a failure — matching the per-backend exec tests.

fn run_rust(name: &str, module: &Module) -> RunOutcome {
    let artifact = match semantic_ir_to_rust::compile(module) {
        Ok(a) => a,
        Err(e) => return RunOutcome::Failed(format!("rust emit failed: {e:?}")),
    };
    let src = temp_path(name, Target::Rust, ".rs");
    let bin = temp_path(name, Target::Rust, if cfg!(windows) { ".exe" } else { "" });
    if fs::write(&src, &artifact.source).is_err() {
        return RunOutcome::Failed("could not write temp .rs".into());
    }
    let mut cmd = Command::new("rustc");
    cmd.arg("--edition").arg("2021").arg("-O");
    if let Ok(linker) = std::env::var("SIR_TEST_RUSTC_LINKER") {
        if !linker.is_empty() {
            cmd.arg("-C").arg(format!("linker={linker}"));
        }
    }
    let compiled = cmd.arg(&src).arg("-o").arg(&bin).output();
    match compiled {
        Ok(o) if o.status.success() => {
            let run_out = Command::new(&bin).output();
            let _ = fs::remove_file(&src);
            let _ = fs::remove_file(&bin);
            finish(run_out, &artifact.source, "rust")
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let _ = fs::remove_file(&src);
            if stderr.contains("linker")
                && (stderr.contains("not found") || stderr.contains("No such file"))
            {
                RunOutcome::Skipped("no usable linker on host".into())
            } else {
                RunOutcome::Failed(format!(
                    "rustc failed:\n{stderr}\n--- source ---\n{}",
                    artifact.source
                ))
            }
        }
        Err(e) => {
            let _ = fs::remove_file(&src);
            RunOutcome::Skipped(format!("rustc unavailable: {e}"))
        }
    }
}

// ── C ─────────────────────────────────────────────────────────────────────
//
// The C backend emits a self-contained `.c`; we compile it with a discovered
// gcc/clang-style compiler (see `c_compiler`) and run the binary. Two skips
// (not failures) keep the matrix honest: no C compiler on the host, and a
// program whose feature set the *v0* C backend does not yet accept — the
// backend rejects it cleanly (a declared gap, not a faithfulness bug), so that
// `(program, C)` cell is skipped until the feature's batch lands.

fn run_c(name: &str, module: &Module) -> RunOutcome {
    let artifact = match semantic_ir_to_c::compile(module) {
        Ok(a) => a,
        Err(e)
            if matches!(
                e.kind,
                BackendErrorKind::UnsupportedFeature | BackendErrorKind::UnsupportedIntrinsic
            ) =>
        {
            return RunOutcome::Skipped(format!("c backend (v0) does not yet accept: {}", e.message))
        }
        Err(e) => return RunOutcome::Failed(format!("c emit failed: {e:?}")),
    };
    let Some(cc) = c_compiler() else {
        return RunOutcome::Skipped("no C compiler (set SIR_CC or install cc/clang/gcc)".into());
    };
    let src = temp_path(name, Target::C, ".c");
    let bin = temp_path(name, Target::C, if cfg!(windows) { ".exe" } else { "" });
    if fs::write(&src, &artifact.source).is_err() {
        return RunOutcome::Failed("could not write temp .c".into());
    }
    let compiled = Command::new(&cc)
        .arg("-std=c99")
        .arg("-o")
        .arg(&bin)
        .arg(&src)
        .output();
    match compiled {
        Ok(o) if o.status.success() => {
            let run_out = Command::new(&bin).output();
            let _ = fs::remove_file(&src);
            let _ = fs::remove_file(&bin);
            finish(run_out, &artifact.source, "c")
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let _ = fs::remove_file(&src);
            RunOutcome::Failed(format!(
                "C compiler failed:\n{stderr}\n--- source ---\n{}",
                artifact.source
            ))
        }
        Err(e) => {
            let _ = fs::remove_file(&src);
            RunOutcome::Skipped(format!("C compiler unavailable: {e}"))
        }
    }
}

/// Turn a process result into a [`RunOutcome`]: non-zero exit is a failure
/// (carrying stderr + the emitted source for debugging), success returns the
/// normalised stdout.
fn finish(
    out: std::io::Result<std::process::Output>,
    source: &str,
    tag: &str,
) -> RunOutcome {
    match out {
        Ok(o) if o.status.success() => {
            RunOutcome::Ran(normalise(&String::from_utf8_lossy(&o.stdout)))
        }
        Ok(o) => RunOutcome::Failed(format!(
            "{tag} program exited {} (should be 0):\n--- stderr ---\n{}\n--- source ---\n{}",
            o.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&o.stderr),
            source,
        )),
        Err(e) => RunOutcome::Skipped(format!("{tag} toolchain vanished mid-run: {e}")),
    }
}
