//! # Cross-language platform matrix — LANG-PLATFORM-MATRIX (LM0 foundation).
//!
//! The generalization of the McCarthy W16 capstone (`conformance.rs`) from one
//! reference language to **every language frontend in the repo**. Each language has
//! a small battery of programs with a known result; each backend is a runner gated
//! on its toolchain; every `(program, backend)` cell is asserted **by running**.
//!
//! LM0 establishes the harness and the **native-AOT** column — the genuinely
//! already-green path: source → shared IIR → host object (`aarch64`/`x86_64-backend`)
//! → system linker → run. Native AOT is a *general* IIR code generator, so it runs
//! all six languages today (verified below). Later slices add the LLVM / WASM / JVM /
//! CLR columns (also general code generators) and tackle the two McCarthy-specialized
//! backends — VM and JIT — which need arithmetic/comparison op-coverage to run the
//! non-McCarthy languages (see `code/specs/LANG-PLATFORM-MATRIX.md`).
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

/// The known, backend-independent observable result of a conformance program.
enum Expect {
    /// The process exit code (an expression language's returned value, `& 0xFF`).
    Exit(i32),
    /// A trimmed stdout string (an I/O language's printed output).
    Stdout(&'static str),
}

/// One conformance program: a language, a source-file extension, the source, and
/// the result it must produce on any backend that can run it.
struct Prog {
    lang: Language,
    ext: &'static str,
    src: &'static str,
    expect: Expect,
}

/// The cross-language battery. Each program is deliberately tiny but exercises real
/// computation (arithmetic, calls, comparisons, loops, I/O) — not just constants —
/// so a backend that merely emits a literal would not pass.
const PROGRAMS: &[Prog] = &[
    // Twig — the original AOT language; a bare expression is the whole program.
    Prog { lang: Language::Twig, ext: "twig", src: "42", expect: Expect::Exit(42) },
    // Nib — typed functions: define `double`, call it, return the result.
    Prog {
        lang: Language::Nib,
        ext: "nib",
        src: "fn double(x: u8) -> u8 { return x + x; } fn main() -> u8 { return double(21); }",
        expect: Expect::Exit(42),
    },
    // Oct — `let` + `if` + comparison; `main` is void so the process exits 0.
    Prog {
        lang: Language::Oct,
        ext: "oct",
        src: "fn main() { let x: u8 = 1; if x == 1 { let y: u8 = 2; } else { let z: u8 = 3; } }",
        expect: Expect::Exit(0),
    },
    // ALGOL 60 — a begin/end block with real integer arithmetic (`17 mod 5` = 2).
    Prog {
        lang: Language::Algol60,
        ext: "alg",
        src: "begin integer result; result := 17 mod 5 end",
        expect: Expect::Exit(2),
    },
    // Brainfuck — build 65 on the tape and `putchar` it: prints `A`.
    Prog {
        lang: Language::Brainfuck,
        ext: "bf",
        src: "++++++++[>++++++++<-]>+.",
        expect: Expect::Stdout("A"),
    },
    // Dartmouth BASIC — `PRINT 42` writes `42` to stdout.
    Prog {
        lang: Language::DartmouthBasic,
        ext: "bas",
        src: "10 PRINT 42\n20 END\n",
        expect: Expect::Stdout("42"),
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

/// LM0: every language runs on **native AOT** with its known result, on any host
/// that can link. Native AOT is a general IIR code generator, so this is the
/// conformance floor the later (LLVM/WASM/JVM/CLR) columns are measured against.
#[test]
fn native_aot_runs_every_language() {
    let mut ran = 0usize;
    for p in PROGRAMS {
        let Some((code, stdout)) = run_native(p) else {
            continue; // host can't link — skip gracefully
        };
        match &p.expect {
            Expect::Exit(n) => assert_eq!(
                code,
                Some(*n),
                "native-AOT {:?}: expected exit {n}, got {code:?} (stdout {stdout:?})",
                p.lang
            ),
            Expect::Stdout(s) => assert_eq!(
                stdout, *s,
                "native-AOT {:?}: expected stdout {s:?}, got {stdout:?}",
                p.lang
            ),
        }
        ran += 1;
    }

    // On a host with a system linker (Linux/macOS CI runners), native AOT MUST run
    // every language — it is the general code-gen floor for the whole matrix.
    if cfg!(any(target_os = "linux", target_os = "macos")) {
        assert_eq!(
            ran,
            PROGRAMS.len(),
            "every language must run on native AOT on a Linux/macOS host"
        );
    } else {
        eprintln!("native-AOT lang-matrix: ran {ran}/{} (non-Linux/macOS host)", PROGRAMS.len());
    }
}
