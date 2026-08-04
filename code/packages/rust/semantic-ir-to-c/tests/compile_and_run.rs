//! Execution proof — lower a source snippet, emit C, **compile it with a real
//! C compiler and run it**, and assert the stdout.
//!
//! Requires a gcc/clang-style C compiler.  It is discovered from the `SIR_CC`
//! environment variable first (point it at any `clang`/`gcc`, e.g. an absolute
//! path on Windows), then `cc` / `clang` / `gcc` on `PATH`.  When none is
//! found every case **skips** (prints a notice) rather than failing — the same
//! graceful-degradation convention the LLVM/clang and JVM/java conformance
//! cells use — so this file never breaks a toolchain-less CI box or sandbox.
//!
//! MSVC (`cl`) uses a different CLI (`/Fe`, vcvars) and is verified separately
//! (the repo's C-conformance harness / a developer's `ccheck`), not here.

use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

fn find_cc() -> Option<String> {
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

static COUNTER: AtomicU32 = AtomicU32::new(0);

fn unique_stem(name: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("sirc_{}_{}_{}", name, std::process::id(), n)
}

/// Emit C for `module`, compile it with `cc`, run it, and return stdout.
fn compile_and_run(cc: &str, module: &semantic_ir::Module, name: &str) -> String {
    let artifact = semantic_ir_to_c::compile(module).expect("C backend compile");

    let dir = std::env::temp_dir();
    let stem = unique_stem(name);
    let cpath: PathBuf = dir.join(format!("{stem}.c"));
    let exe: PathBuf = dir.join(format!("{stem}{}", std::env::consts::EXE_SUFFIX));

    std::fs::File::create(&cpath)
        .and_then(|mut f| f.write_all(artifact.source.as_bytes()))
        .expect("write .c");

    let out = Command::new(cc)
        .arg("-std=c99")
        .arg("-o")
        .arg(&exe)
        .arg(&cpath)
        .arg("-lm")  // Linux needs -lm to link floor/ceil/fabs (macOS libSystem folds it in)
        .output()
        .expect("spawn C compiler");
    assert!(
        out.status.success(),
        "compile failed for `{name}`:\n{}\n--- source ---\n{}",
        String::from_utf8_lossy(&out.stderr),
        artifact.source
    );

    let run = Command::new(&exe).output().expect("run emitted program");
    assert!(
        run.status.success(),
        "run failed for `{name}` (exit {:?}): {}",
        run.status.code(),
        String::from_utf8_lossy(&run.stderr)
    );

    // Cleanup is best-effort (arena/leak in the C program is fine; temp files
    // are removed to stay tidy).
    let _ = std::fs::remove_file(&cpath);
    let _ = std::fs::remove_file(&exe);

    String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n")
}

struct Case {
    name: &'static str,
    lang: &'static str,
    source: &'static str,
    expected: &'static str,
}

const CORPUS: &[Case] = &[
    Case {
        name: "arithmetic",
        lang: "ruby",
        source: "puts 2 + 3 * 4",
        expected: "14\n",
    },
    Case {
        name: "def_params",
        lang: "ruby",
        source: "def add(a, b)\n  a + b\nend\nputs add(2, 3)",
        expected: "5\n",
    },
    Case {
        name: "tail_if",
        lang: "ruby",
        source: "def f(x)\n  if x > 0\n    10\n  else\n    0\n  end\nend\nputs f(3)",
        expected: "10\n",
    },
    Case {
        name: "seq_assign",
        lang: "ruby",
        source: "a = 5\nb = a + 1\nputs a\nputs b\nputs a + b",
        expected: "5\n6\n11\n",
    },
    Case {
        name: "string_concat",
        lang: "ruby",
        source: "puts \"ab\" + \"cd\"",
        expected: "abcd\n",
    },
    Case {
        // Trigraph-safety: `??/` in a source string must reach stdout verbatim,
        // not be mangled by C trigraph translation under `-std=c99`.
        name: "trigraph",
        lang: "ruby",
        source: "puts \"why?? ok??/\"",
        expected: "why?? ok??/\n",
    },
    Case {
        name: "twig_arith",
        lang: "twig",
        source: "(print (+ 2 (* 3 4)))",
        expected: "14",
    },
    Case {
        name: "twig_closure",
        lang: "twig",
        source: "(define (adder n) (lambda (x) (+ x n))) (define a (adder 5)) (print (a 3))",
        expected: "8",
    },
    Case {
        // Unary minus (`-x`) lowers to the `neg` builtin, which the C backend
        // now lowers (SIR21 §E3). Before, ANY negative literal made the C
        // backend report an unsupported builtin and skip. `neg` reuses the
        // single-argument `_sir_minus` path (which negates tag-preservingly),
        // so this also confirms floored `Integer#/` on a negative dividend
        // (`-7 / 2 == -4`, not the truncating `-3`).
        name: "unary_minus",
        lang: "ruby",
        source: "puts(-7)\nputs(-7 / 2)\nputs(-(3 * 2))",
        expected: "-7\n-4\n-6\n",
    },
];

fn lower(case: &Case) -> semantic_ir::Module {
    match case.lang {
        "twig" => twig_to_semantic_ir::compile_source(case.source, "prog").expect("twig lowering"),
        _ => ruby_to_semantic_ir::compile_source(case.source, "prog").expect("ruby lowering"),
    }
}

#[test]
fn corpus_compiles_and_runs() {
    let Some(cc) = find_cc() else {
        eprintln!("SKIP: no C compiler found (set SIR_CC or install cc/clang/gcc)");
        return;
    };
    eprintln!("using C compiler: {cc}");
    let mut ran = 0;
    for case in CORPUS {
        let m = lower(case);
        let got = compile_and_run(&cc, &m, case.name);
        assert_eq!(got, case.expected, "output mismatch for `{}`", case.name);
        ran += 1;
    }
    assert!(ran > 0);
    eprintln!("compile_and_run: {ran} programs OK on {cc}");
}
