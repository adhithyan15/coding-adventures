//! # BEAM symbols + lambda — F6/F7, COMPLETES the BEAM backend (LANG77 / W11).
//!
//! - **Symbols (F6):** the shared `intern_symbols_structural` pass interns each
//!   distinct symbol to a stable `i32` id (`SYMBOL_ID_BASE = 1<<29`) — the SAME
//!   id the wasm/JVM/CLR backends assign — which the BEAM carries as a native
//!   Erlang integer; `EQ` on symbols becomes `is_eq_exact`.
//! - **Lambda (F7):** needs NO BEAM-specific work — a `(LAMBDA …)` application is
//!   a method `call`, which `iir-to-beam` already lowers natively (a BEAM fun).
//!
//! **Verified by RUNNING** the emitted `.beam` on a real `erl` (skipped if absent).

use lang_aot::{compile_source_to_beam, Language};

fn erl_available() -> bool {
    std::process::Command::new("erl").arg("-version").output()
        .map(|o| o.status.success()).unwrap_or(false)
}

fn run(src: &str, module: &str) -> String {
    let bytes = compile_source_to_beam(Language::McCarthyLisp, src, module)
        .unwrap_or_else(|e| panic!("compile {src:?} to BEAM: {e}"));
    let tmp = std::env::temp_dir().join(format!("mccarthy_w11_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    std::fs::write(tmp.join(format!("{module}.beam")), &bytes).expect("write .beam");
    let out = std::process::Command::new("erl")
        .arg("-noshell").arg("-pa").arg(&tmp)
        .arg("-eval").arg(format!("io:format(\"~w~n\",[{module}:main()]),halt(0)."))
        .output().expect("spawn erl");
    assert!(out.status.success(), "erl non-zero; stderr: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

const SYMBOL_ID_BASE: i64 = 1 << 29;

#[test]
fn mccarthy_symbols_run_on_beam() {
    if !erl_available() { eprintln!("erl absent — skipping"); return; }
    // The first distinct symbol gets the base id — identical to the CLR/JVM/wasm id.
    assert_eq!(run("(QUOTE A)", "bs_q"), SYMBOL_ID_BASE.to_string(), "interned symbol id");
    assert_eq!(run("(EQ (QUOTE A) (QUOTE A))", "bs_e1"), "1", "a symbol equals itself");
    assert_eq!(run("(EQ (QUOTE A) (QUOTE B))", "bs_e2"), "0", "distinct symbols differ");
    assert_eq!(run("(ATOM (QUOTE A))", "bs_a"), "1", "a symbol is an atom");
}

#[test]
fn mccarthy_lambda_runs_on_beam() {
    if !erl_available() { eprintln!("erl absent — skipping"); return; }
    assert_eq!(run("((LAMBDA (X) X) 5)", "bl_id"), "5", "identity lambda");
    assert_eq!(run("((LAMBDA (X) (CAR X)) (CONS 7 9))", "bl_car"), "7", "lambda over a cons arg");
    assert_eq!(run("((LAMBDA (X Y) (EQ X Y)) 3 3)", "bl_eq"), "1", "2-arg lambda");
    assert_eq!(run("((LAMBDA (X) (EQ X (QUOTE A))) (QUOTE A))", "bl_sym"), "1", "lambda over a symbol arg");
}
