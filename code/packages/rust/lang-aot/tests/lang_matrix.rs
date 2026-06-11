//! # Cross-language platform matrix — LANG-PLATFORM-MATRIX (LM0 foundation).
//!
//! The generalization of the McCarthy W16 capstone (`conformance.rs`) from one
//! reference language to **every language frontend in the repo**. Each language has
//! a small battery of programs with a known result; each backend is a runner gated
//! on its toolchain; every `(program, backend)` cell is asserted **by running**.
//!
//! The harness is a `Backend`-keyed grid: each `Prog` lists the backends a slice has
//! **proven** run it, and `matrix_every_proven_cell_agrees` runs every such cell on
//! its real toolchain and asserts the known result (skipping a cell whose tool is
//! absent, failing loudly when present-but-wrong). Columns so far:
//!
//! * **native-AOT** (LM0) — source → shared IIR → host object → system linker → run.
//!   Uniformly green: all six languages.
//! * **LLVM** (Phase L) — source → textual `.ll` (`iir-to-llvm`) → real `clang` → run.
//!   Green for the expression languages Twig / Oct / ALGOL 60 today; Nib pends an
//!   `iir-to-llvm` `u8`-widening fix and Brainfuck/BASIC pend the stdout I/O runner.
//!
//! Later slices add the WASM / JVM / CLR columns (also general code generators) and
//! tackle the two McCarthy-specialized backends — VM and JIT — which need
//! arithmetic/comparison op-coverage (see `code/specs/LANG-PLATFORM-MATRIX.md`).
//!
//! ## Two result kinds
//!
//! * **Expression languages** (Twig, Nib, Oct, ALGOL 60) return an integer — the
//!   process **exit code** (`& 0xFF` via the C runtime's `exit()`). Oct's `main` is
//!   void, so it exits `0`; the program still proves the whole chain runs.
//! * **I/O languages** (Brainfuck, Dartmouth BASIC) produce their result on
//!   **stdout** (`putchar` / `PRINT`), so the harness captures and compares stdout.

use lang_aot::Language;
use std::process::Command;

/// A non-BEAM backend the matrix proves languages on. Each new column the campaign
/// lands adds a variant here and a `run` arm; each `Prog` lists the backends it is
/// **proven** to run on (so a cell is only asserted once a slice has verified it).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Backend {
    /// Source → IIR → host object → system linker → run (LM0). General code gen.
    NativeAot,
    /// Source → textual `.ll` (`iir-to-llvm`) → real `clang` → run (Phase L).
    Llvm,
}

/// The known, backend-independent observable result of a conformance program.
enum Expect {
    /// The process exit code (an expression language's returned value, `& 0xFF`).
    Exit(i32),
    /// A trimmed stdout string (an I/O language's printed output).
    Stdout(&'static str),
}

/// One conformance program: a language, a source-file extension, the source, the
/// result it must produce, and the backends a slice has **proven** run it.
struct Prog {
    lang: Language,
    ext: &'static str,
    src: &'static str,
    expect: Expect,
    backends: &'static [Backend],
}

use Backend::{Llvm, NativeAot};

/// The cross-language battery. Each program is deliberately tiny but exercises real
/// computation (arithmetic, calls, comparisons, loops, I/O) — not just constants —
/// so a backend that merely emits a literal would not pass.
const PROGRAMS: &[Prog] = &[
    // Twig — the original AOT language; a bare expression is the whole program.
    Prog { lang: Language::Twig, ext: "twig", src: "42", expect: Expect::Exit(42), backends: &[NativeAot, Llvm] },
    // Nib — typed functions: define `double`, call it, return the result.
    // (LLVM column pending: `iir-to-llvm` mis-widens Nib's `u8` operands to `i64`
    // — `'%x' defined with type 'i8' but expected 'i64'`; fixed in the LM-L Nib slice.)
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn double(x: u8) -> u8 { return x + x; } fn main() -> u8 { return double(21); }",
        expect: Expect::Exit(42),
        backends: &[NativeAot],
    },
    // Oct — `let` + `if` + comparison; `main` is void so the process exits 0.
    Prog {
        lang: Language::Oct,
        ext: "oct",
        src: "fn main() { let x: u8 = 1; if x == 1 { let y: u8 = 2; } else { let z: u8 = 3; } }",
        expect: Expect::Exit(0),
        backends: &[NativeAot, Llvm],
    },
    // ALGOL 60 — a begin/end block with real integer arithmetic (`17 mod 5` = 2).
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer result; result := 17 mod 5 end",
        expect: Expect::Exit(2),
        backends: &[NativeAot, Llvm],
    },
    // Brainfuck — build 65 on the tape and `putchar` it: prints `A`.
    // (LLVM column pending: needs the stdout-capturing I/O runner — a later slice.)
    Prog {
        lang: Language::Brainfuck,
        ext: "bf",
        src: "++++++++[>++++++++<-]>+.",
        expect: Expect::Stdout("A"),
        backends: &[NativeAot],
    },
    // Dartmouth BASIC — `PRINT 42` writes `42` to stdout.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT 42\n20 END\n",
        expect: Expect::Stdout("42"),
        backends: &[NativeAot],
    },
];

/// Is a usable native linker present on this host? On Linux/macOS the AOT path uses
/// the always-present system linker; on Windows it needs a real MSVC/LLD/gcc linker.
fn native_linker_ok() -> bool {
    if cfg!(target_os = "windows") {
        // Mirror twig-aot's probe: confirm a genuine linker, not git-bash's `link`.
        let probes: &[(&str, &str, &[&str])] = &[
            ("link.exe", "", &["Microsoft", "Linker"]),
            ("lld-link.exe", "", &["LLD"]),
            ("gcc.exe", "--version", &["gcc"]),
        ];
        probes.iter().any(|(name, arg, markers)| {
            let mut cmd = Command::new(name);
            if !arg.is_empty() {
                cmd.arg(arg);
            }
            cmd.output()
                .map(|o| {
                    let banner = format!(
                        "{}{}",
                        String::from_utf8_lossy(&o.stdout),
                        String::from_utf8_lossy(&o.stderr)
                    );
                    markers.iter().all(|m| banner.contains(m))
                })
                .unwrap_or(false)
        })
    } else {
        cfg!(any(target_os = "linux", target_os = "macos"))
    }
}

/// Compile `p` to a native executable for the host OS. `None` when the host can't
/// produce a native exe (skip), so the suite degrades gracefully off Linux/macOS.
fn compile_native(src_path: &std::path::Path, exe: &std::path::Path, lang: Language) -> Option<()> {
    #[cfg(target_os = "linux")]
    {
        lang_aot::compile_file_to_linux_executable(src_path, exe, lang).ok()
    }
    #[cfg(target_os = "macos")]
    {
        lang_aot::compile_file_to_macos_executable(src_path, exe, lang).ok()
    }
    #[cfg(target_os = "windows")]
    {
        lang_aot::compile_file_to_windows_executable(src_path, exe, lang).ok()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = (src_path, exe, lang);
        None
    }
}

/// Native-AOT runner: write the source, compile to a host executable, run it, and
/// return `(exit_code, trimmed_stdout)`. `None` when native AOT is unavailable here.
///
/// The programs are fixed literals (no untrusted input), and each terminates by
/// construction — there is no unbounded loop or recursion in the harness itself.
/// The work happens in a fresh `tempfile::tempdir()` (a random, `0700`, auto-removed
/// directory) rather than a predictable `temp_dir()/<pid>` path, so a local attacker
/// cannot pre-create the directory or plant a symlink at `prog` and have the harness
/// execute substituted code in the compile→run window (CWE-377/367). The `_dir`
/// guard is held until after the executable runs so it is not removed early.
fn run_native(p: &Prog) -> Option<(Option<i32>, String)> {
    if !native_linker_ok() {
        return None;
    }
    let dir = tempfile::tempdir().ok()?;
    let src_path = dir.path().join(format!("prog.{}", p.ext));
    std::fs::write(&src_path, p.src).ok()?;
    let exe = dir.path().join("prog");
    compile_native(&src_path, &exe, p.lang)?;
    let out = Command::new(&exe).output().ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some((out.status.code(), stdout))
}

/// Is a usable `clang` present? Gates the LLVM column (skip when absent).
fn clang_ok() -> bool {
    Command::new("clang")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// LLVM runner: source → textual `.ll` (`iir-to-llvm`) → real `clang` → run, the
/// exact CLR-real/McCarthy strategy of handing symbolic code to the real toolchain.
/// `None` when `clang` is absent or the build fails (skip). Exit-code programs only
/// (the expression languages); the I/O languages get their stdout-capturing LLVM
/// runner — which links the C runtime — in a later slice. The expression languages
/// need no runtime (verified: Twig/Oct/ALGOL link a bare `.ll` standalone).
///
/// Same temp-file hardening as `run_native`: a fresh `tempfile::tempdir()` whose
/// guard outlives the run, so the executed `prog` cannot be substituted (CWE-377/367).
fn run_llvm(p: &Prog) -> Option<(Option<i32>, String)> {
    if !clang_ok() {
        return None;
    }
    let triple = String::from_utf8(
        Command::new("clang").arg("-dumpmachine").output().ok()?.stdout,
    )
    .ok()?
    .trim()
    .to_string();
    let ll = lang_aot::compile_source_to_llvm_with_target(p.lang, p.src, "lm", &triple).ok()?;
    let dir = tempfile::tempdir().ok()?;
    let ll_path = dir.path().join("prog.ll");
    std::fs::write(&ll_path, &ll).ok()?;
    let exe = dir.path().join("prog");
    let built = Command::new("clang")
        .arg("-x")
        .arg("ir")
        .arg(&ll_path)
        .arg("-o")
        .arg(&exe)
        .output()
        .ok()?;
    if !built.status.success() {
        return None;
    }
    let out = Command::new(&exe).output().ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Some((out.status.code(), stdout))
}

/// Dispatch a program to a backend runner. `None` = the backend's toolchain is
/// unavailable on this host (skip, like the W16 external-tool backends).
fn run(backend: Backend, p: &Prog) -> Option<(Option<i32>, String)> {
    match backend {
        Backend::NativeAot => run_native(p),
        Backend::Llvm => run_llvm(p),
    }
}

/// Assert a single matrix cell agrees with the program's known result.
fn assert_cell(backend: Backend, p: &Prog, code: Option<i32>, stdout: &str) {
    match &p.expect {
        Expect::Exit(n) => assert_eq!(
            code,
            Some(*n),
            "{backend:?} {:?}: expected exit {n}, got {code:?} (stdout {stdout:?})",
            p.lang
        ),
        Expect::Stdout(s) => assert_eq!(
            stdout, *s,
            "{backend:?} {:?}: expected stdout {s:?}, got {stdout:?}",
            p.lang
        ),
    }
}

/// The capstone: every `(program, backend)` cell the campaign has **proven** runs
/// and agrees with the known result. A cell whose toolchain is absent skips
/// gracefully; a cell whose toolchain is present but disagrees fails loudly.
#[test]
fn matrix_every_proven_cell_agrees() {
    let mut ran = 0usize;
    for p in PROGRAMS {
        for &backend in p.backends {
            let Some((code, stdout)) = run(backend, p) else {
                continue;
            };
            assert_cell(backend, p, code, &stdout);
            ran += 1;
        }
    }
    eprintln!("lang-matrix: {ran} proven cells exercised");
}

/// Per-column floor: when a backend's toolchain IS present, every program tagged
/// for that backend MUST actually run — a proven cell silently skipping is a
/// regression, not a graceful absence.
#[test]
fn proven_columns_do_not_silently_skip() {
    // native-AOT: on a Linux/macOS host every native-tagged program must run.
    if cfg!(any(target_os = "linux", target_os = "macos")) {
        for p in PROGRAMS.iter().filter(|p| p.backends.contains(&NativeAot)) {
            assert!(
                run_native(p).is_some(),
                "native-AOT present but failed to run {:?}",
                p.lang
            );
        }
    }
    // LLVM: when clang is present every LLVM-tagged program must run.
    if clang_ok() {
        for p in PROGRAMS.iter().filter(|p| p.backends.contains(&Llvm)) {
            assert!(
                run_llvm(p).is_some(),
                "clang present but LLVM failed to run {:?}",
                p.lang
            );
        }
    }
}
