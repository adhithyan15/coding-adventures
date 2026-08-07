//! # sir-bench — cross-backend performance benchmarks for the Semantic IR
//!
//! The [`sir-conformance`](../sir-conformance) harness answers *"does every
//! backend produce the **same answer**?"*.  This crate answers the sibling
//! question: *"how **fast** is the code each backend generates?"*.
//!
//! A single Ruby program is lowered **once** to the narrow-waist
//! [`semantic_ir::Module`], then emitted to every backend and run through its
//! real toolchain — exactly the pipeline `sir-conformance` uses — but here we
//! put a **stopwatch** on each phase:
//!
//! ```text
//!   Ruby ──frontend──▶ SIR ──emit──▶ source ──compile──▶ binary ──run──▶ stdout
//!                          └─ emit ms ┘        └─ compile ms ┘   └─ run ms ┘
//! ```
//!
//! - **emit ms** — how long the Rust backend takes to turn SIR into target
//!   source.  This is *our* code, and it is the same order of magnitude for
//!   every backend (string building), so it mostly measures the emitter.
//! - **compile ms** — how long the target's own compiler takes (`cc`, `rustc`,
//!   `go build`).  Interpreted targets (Python, JavaScript, Ruby) have **no**
//!   compile step, so this cell is blank.
//! - **run ms** — how long the *generated program* takes to execute.  This is
//!   the number the question is really about: a Ruby method compiled to C runs
//!   as native code; the same method left as Ruby runs on the Ruby VM.
//!
//! ## Methodology (why the numbers are trustworthy)
//!
//! - **Lower once.**  The frontend runs a single time per program; every backend
//!   shares that `Module`, so we never charge one backend for the parser.
//! - **Warm up, then take the median.**  The first execution of a freshly built
//!   binary can be dominated by one-time costs — the OS paging it in, and (on
//!   some managed macOS hosts) an endpoint-security scanner that stalls a new
//!   binary's *first* exec.  We therefore run a few discarded **warmup** passes
//!   and report the **median** of the timed passes, so a one-off outlier can
//!   never masquerade as the program's speed.
//! - **Optimise compiled targets.**  `cc -O2`, `rustc -O`, `go build` (release
//!   by default) — otherwise a debug binary would slander the backend.  The
//!   exact flags are recorded next to the table so a reader can reproduce them.
//! - **Skip, don't lie.**  A missing toolchain, or a program whose feature set a
//!   *v0* backend does not yet accept, is reported as a **skip** — never folded
//!   into a misleading "0 ms".

use semantic_ir::Module;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

pub use ruby_to_semantic_ir::compile_source;

/// A backend target and the toolchain that runs its emitted source.  Mirrors
/// `sir-conformance`'s `Target` so the two harnesses agree on what "a backend"
/// is, but adds the timing-relevant [`Target::is_compiled`] split.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Python,
    JavaScript,
    Go,
    Rust,
    C,
    Ruby,
}

impl Target {
    /// Every target the harness knows how to time.
    pub fn all() -> &'static [Target] {
        &[
            Target::Python,
            Target::JavaScript,
            Target::Go,
            Target::Rust,
            Target::C,
            Target::Ruby,
        ]
    }

    /// Human-readable tag (table row label, temp-file stem).
    pub fn tag(self) -> &'static str {
        match self {
            Target::Python => "python",
            Target::JavaScript => "javascript",
            Target::Go => "go",
            Target::Rust => "rust",
            Target::C => "c",
            Target::Ruby => "ruby",
        }
    }

    /// Does this target compile to a native binary before running?  Compiled
    /// targets (C, Rust, Go) pay a one-time **compile** cost and then run a
    /// binary; interpreted targets (Python, JavaScript, Ruby) run their source
    /// directly, so their compile cell is blank and the compiler *is* the
    /// runtime.
    pub fn is_compiled(self) -> bool {
        matches!(self, Target::C | Target::Rust | Target::Go)
    }

    /// The executable that must be on `PATH` (C discovers its compiler via
    /// [`c_compiler`], honouring `SIR_CC`, so this is only its fallback).
    fn toolchain(self) -> &'static str {
        match self {
            Target::Python => "python3",
            Target::JavaScript => "node",
            Target::Go => "go",
            Target::Rust => "rustc",
            Target::C => "cc",
            Target::Ruby => "ruby",
        }
    }

    /// The version-probe argument (`go version` takes no dashes, unlike the
    /// rest, and would otherwise make Go look unavailable).
    fn version_arg(self) -> &'static str {
        match self {
            Target::Go => "version",
            _ => "--version",
        }
    }

    /// Is this target's toolchain present?  A missing toolchain is a *skip*.
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

/// Discover a gcc/clang-style C compiler: `SIR_CC` first (an absolute path
/// works), then `cc` / `clang` / `gcc` on `PATH`.  `None` ⇒ the C row skips.
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

/// A benchmark program: compute-heavy Ruby that lowers and runs on **every**
/// backend, so the whole row is comparable.  `iters`/`warmup` let a cheap
/// program ask for more repetitions to stay above the timer's noise floor.
#[derive(Debug, Clone, Copy)]
pub struct Bench {
    pub name: &'static str,
    pub ruby: &'static str,
    /// Timed passes whose median is reported.
    pub iters: u32,
    /// Discarded passes run first (page-in + first-exec-scan absorption).
    pub warmup: u32,
    /// A one-line note on what the program stresses.
    pub note: &'static str,
}

/// The measured cost of one (program, backend) cell.
#[derive(Debug, Clone)]
pub enum Sample {
    /// It emitted, compiled (if applicable), and ran.
    Ran {
        emit: Duration,
        /// `None` for interpreted targets.
        compile: Option<Duration>,
        /// Median of the timed run passes.
        run: Duration,
        /// Normalised stdout (so a caller can cross-check answers agree).
        stdout: String,
    },
    /// Toolchain absent, or a v0 backend does not yet accept the program.
    Skipped(String),
    /// A genuine failure (emit / compile / non-zero exit).
    Failed(String),
}

/// Normalise stdout: unify CRLF→LF and strip trailing newlines, so two backends
/// whose only difference is a trailing `\n` still compare equal.
fn normalise(s: &str) -> String {
    s.replace("\r\n", "\n").trim_end_matches('\n').to_string()
}

fn temp_path(name: &str, target: Target, ext: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "sir_bench_{}_{}_{}{ext}",
        name,
        target.tag(),
        std::process::id()
    ));
    p
}

/// The `PYTHONPATH` the Python backend's emitted code needs (it imports the
/// `sir-runtime-*` packages rather than inlining a runtime).  Discovered
/// relative to this crate, so no caller setup is required.
fn python_runtime_path() -> Option<String> {
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
    (!parts.is_empty()).then(|| parts.join(":"))
}

/// Lower Ruby `source` (named `name`) to SIR — the frontend runs **once** per
/// program and the resulting module is shared by every backend.
pub fn lower(name: &str, source: &str) -> Result<Module, String> {
    compile_source(source, name).map_err(|e| format!("frontend failed to lower `{name}`: {e:?}"))
}

/// Emit `module` to `target`'s source.  A `Skipped`/`Failed` here short-circuits
/// the cell (a v0 backend may cleanly reject a feature it does not yet accept).
fn emit(target: Target, module: &Module) -> Result<(String, String), Sample> {
    // Each backend exposes the same `compile(&Module) -> Result<Artifact, _>`.
    macro_rules! go {
        ($m:path, $ext:literal) => {{
            match $m(module) {
                Ok(a) => Ok((a.source, $ext.to_string())),
                Err(e) => {
                    // A "does not accept this feature yet" rejection is a skip
                    // (a declared v0 gap), not a benchmark failure.
                    let msg = format!("{e:?}");
                    if msg.contains("Unsupported") {
                        Err(Sample::Skipped(format!("{} backend rejects: {}", target.tag(), e.message)))
                    } else {
                        Err(Sample::Failed(format!("{} emit failed: {e:?}", target.tag())))
                    }
                }
            }
        }};
    }
    match target {
        Target::Python => go!(semantic_ir_to_python::compile, ".py"),
        Target::JavaScript => go!(semantic_ir_to_javascript::compile, ".js"),
        Target::Go => go!(semantic_ir_to_go::compile, ".go"),
        Target::Rust => go!(semantic_ir_to_rust::compile, ".rs"),
        Target::C => go!(semantic_ir_to_c::compile, ".c"),
        Target::Ruby => go!(semantic_ir_to_ruby::compile, ".rb"),
    }
}

/// The median of a set of run durations (sorted, middle element — for an even
/// count, the upper-middle, which is fine for a small `iters`).
fn median(mut ds: Vec<Duration>) -> Duration {
    ds.sort();
    ds[ds.len() / 2]
}

/// Run one process, returning `(elapsed, normalised_stdout)` or an error string.
fn timed_run(mut cmd: Command) -> Result<(Duration, String), String> {
    let start = Instant::now();
    let out = cmd
        .output()
        .map_err(|e| format!("spawn failed: {e}"))?;
    let elapsed = start.elapsed();
    if !out.status.success() {
        return Err(format!(
            "non-zero exit:\n{}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }
    Ok((elapsed, normalise(&String::from_utf8_lossy(&out.stdout))))
}

/// Build the `Command` that executes an already-prepared program: the compiled
/// binary for C/Rust/Go, or the interpreter over the source for Python/JS/Ruby.
fn run_command(target: Target, source_path: &PathBuf, bin_path: &PathBuf) -> Command {
    match target {
        Target::C | Target::Rust | Target::Go => Command::new(bin_path),
        Target::Python => {
            let mut c = Command::new("python3");
            c.arg(source_path);
            if let Some(pp) = python_runtime_path() {
                c.env("PYTHONPATH", pp);
            }
            c
        }
        Target::JavaScript => {
            let mut c = Command::new("node");
            c.arg(source_path);
            c
        }
        Target::Ruby => {
            let mut c = Command::new("ruby");
            c.arg(source_path);
            c
        }
    }
}

/// Compile `source_path` to `bin_path` for a compiled target, returning the
/// compile duration (or an error).  Never called for interpreted targets.
fn compile_target(
    target: Target,
    source_path: &PathBuf,
    bin_path: &PathBuf,
) -> Result<Duration, String> {
    let start = Instant::now();
    let out = match target {
        // `-O2`: a fair fight — a debug binary would slander the backend.
        Target::C => {
            let cc = c_compiler().ok_or("no C compiler")?;
            Command::new(cc)
                .args(["-std=c99", "-O2", "-o"])
                .arg(bin_path)
                .arg(source_path)
                .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
                .output()
        }
        Target::Rust => Command::new("rustc")
            .args(["--edition", "2021", "-O", "-o"])
            .arg(bin_path)
            .arg(source_path)
            .output(),
        Target::Go => Command::new("go")
            .arg("build")
            .arg("-o")
            .arg(bin_path)
            .arg(source_path)
            .output(),
        _ => return Err("not a compiled target".into()),
    };
    let elapsed = start.elapsed();
    match out {
        Ok(o) if o.status.success() => Ok(elapsed),
        Ok(o) => Err(format!(
            "{} compile failed:\n{}",
            target.tag(),
            String::from_utf8_lossy(&o.stderr)
        )),
        Err(e) => Err(format!("{} compiler unavailable: {e}", target.tag())),
    }
}

/// Measure one (program, backend) cell: lower is the caller's job (shared); this
/// emits, compiles if needed, warms up, and reports the median run.
pub fn measure(bench: &Bench, module: &Module, target: Target) -> Sample {
    if !target.available() {
        return Sample::Skipped(format!("{} not on PATH", target.toolchain()));
    }

    // 1. Emit (timed) — SIR → target source.
    let emit_start = Instant::now();
    let (source, ext) = match emit(target, module) {
        Ok(s) => s,
        Err(sample) => return sample,
    };
    let emit = emit_start.elapsed();

    let source_path = temp_path(bench.name, target, &ext);
    let bin_path = temp_path(bench.name, target, if cfg!(windows) { ".exe" } else { ".bin" });
    if fs::write(&source_path, &source).is_err() {
        return Sample::Failed(format!("could not write temp {ext}"));
    }

    // 2. Compile (timed) — only for native targets.
    let compile = if target.is_compiled() {
        match compile_target(target, &source_path, &bin_path) {
            Ok(d) => Some(d),
            Err(e) => {
                let _ = fs::remove_file(&source_path);
                // A missing linker/toolchain is a skip; a real error is a fail.
                return if e.contains("unavailable") || e.contains("linker") {
                    Sample::Skipped(e)
                } else {
                    Sample::Failed(e)
                };
            }
        }
    } else {
        None
    };

    // 3. Warm up (discarded) — pages the binary in and absorbs any first-exec
    //    scan, so it can never be mistaken for the program's speed.
    for _ in 0..bench.warmup {
        let _ = timed_run(run_command(target, &source_path, &bin_path));
    }

    // 4. Timed passes → median.
    let mut runs = Vec::with_capacity(bench.iters as usize);
    let mut stdout = String::new();
    for _ in 0..bench.iters.max(1) {
        match timed_run(run_command(target, &source_path, &bin_path)) {
            Ok((d, out)) => {
                runs.push(d);
                stdout = out;
            }
            Err(e) => {
                let _ = fs::remove_file(&source_path);
                let _ = fs::remove_file(&bin_path);
                return Sample::Failed(e);
            }
        }
    }
    let _ = fs::remove_file(&source_path);
    let _ = fs::remove_file(&bin_path);

    Sample::Ran {
        emit,
        compile,
        run: median(runs),
        stdout,
    }
}

/// Format a `Duration` as milliseconds with 2 decimals (`"12.34"`), or a blank
/// cell for `None` (an interpreted target's absent compile step).
fn ms(d: Duration) -> String {
    format!("{:.2}", d.as_secs_f64() * 1000.0)
}

/// Render the whole benchmark as a GitHub-flavoured Markdown report: one section
/// per program, a row per backend, sorted fastest-run first so the ranking reads
/// off the top.  `results` is `(target, sample)` in `Target::all()` order.
pub fn markdown_report(bench: &Bench, module_stdout: Option<&str>, results: &[(Target, Sample)]) -> String {
    let mut out = String::new();
    out.push_str(&format!("### `{}` — {}\n\n", bench.name, bench.note));
    out.push_str(&format!(
        "_Median of {} run(s) after {} warmup; compiled with `cc -O2` / `rustc -O` / `go build`._\n\n",
        bench.iters, bench.warmup
    ));
    if let Some(exp) = module_stdout {
        out.push_str(&format!("Output (all backends agree): `{}`\n\n", exp));
    }
    out.push_str("| backend | emit ms | compile ms | run ms | vs fastest |\n");
    out.push_str("|---|--:|--:|--:|--:|\n");

    // Rank by run time among the cells that actually ran.
    let mut ran: Vec<(&Target, &Duration)> = results
        .iter()
        .filter_map(|(t, s)| match s {
            Sample::Ran { run, .. } => Some((t, run)),
            _ => None,
        })
        .collect();
    ran.sort_by_key(|(_, r)| **r);
    let fastest = ran.first().map(|(_, r)| **r);

    // Emit rows in fastest-first order, then the skipped/failed rows.
    let mut ordered: Vec<&(Target, Sample)> = Vec::new();
    for (t, _) in &ran {
        if let Some(cell) = results.iter().find(|(rt, _)| rt == *t) {
            ordered.push(cell);
        }
    }
    for cell in results {
        if !matches!(cell.1, Sample::Ran { .. }) {
            ordered.push(cell);
        }
    }

    for (target, sample) in ordered {
        match sample {
            Sample::Ran {
                emit,
                compile,
                run,
                ..
            } => {
                let ratio = match fastest {
                    Some(f) if f.as_nanos() > 0 => {
                        format!("{:.1}×", run.as_secs_f64() / f.as_secs_f64())
                    }
                    _ => "—".to_string(),
                };
                out.push_str(&format!(
                    "| {} | {} | {} | **{}** | {} |\n",
                    target.tag(),
                    ms(*emit),
                    compile.map(ms).unwrap_or_else(|| "—".to_string()),
                    ms(*run),
                    ratio,
                ));
            }
            Sample::Skipped(why) => {
                out.push_str(&format!("| {} | — | — | _skip_ | {} |\n", target.tag(), why));
            }
            Sample::Failed(why) => {
                let first = why.lines().next().unwrap_or("failed");
                out.push_str(&format!("| {} | — | — | _fail_ | {} |\n", target.tag(), first));
            }
        }
    }
    out.push('\n');
    out
}

/// The default benchmark corpus: compute-heavy programs that lower and run on
/// **every** backend (functions, recursion, `if`, `while`, integer arithmetic,
/// comparison — the shared core), so a whole row is comparable.  Kept small and
/// honest; add programs as more of the shared surface is exercised.
pub fn corpus() -> Vec<Bench> {
    vec![
        Bench {
            name: "fib",
            // Naive recursive Fibonacci: ~2.7M calls at n=30 — a pure
            // function-call + integer-add stress test.  The two recursive calls
            // are bound to locals before the add: the frontend's tail-`if`
            // lowering does not yet accept a compound call-expression
            // (`fib(n-1) + fib(n-2)`) directly in a branch tail, so we spell it
            // with temps — behaviour-identical, and it keeps the whole row
            // comparable across backends.
            ruby: "def fib(n)\n  if n < 2\n    n\n  else\n    a = fib(n - 1)\n    b = fib(n - 2)\n    a + b\n  end\nend\nputs fib(30)\n",
            iters: 5,
            warmup: 2,
            note: "recursive fibonacci fib(30) — call + arithmetic overhead",
        },
        Bench {
            name: "loop_sum",
            // A tight counting loop: 5M iterations of read-add-write on two
            // locals — a mutable-binding + loop-dispatch stress test.
            ruby: "sum = 0\ni = 0\nwhile i < 5000000\n  sum = sum + i\n  i = i + 1\nend\nputs sum\n",
            iters: 5,
            warmup: 2,
            note: "5,000,000-iteration counting loop — loop + mutation overhead",
        },
    ]
}
