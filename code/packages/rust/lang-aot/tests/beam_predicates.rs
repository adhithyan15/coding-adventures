//! # BEAM predicates: ATOM / EQ / COND — F3–F5 (LANG77 / McCarthy W10).
//!
//! The native-Erlang twins of the JVM `instanceof`/`ixor`/`if_icmpeq` and the
//! CLR `isinst`/`xor`/`ceq`. The structural pass decomposes `ATOM x` →
//! `not (pair? x)`, `EQ a b` → `equal? a b`, `COND` → chained `jmp_if`; the
//! `call_builtin` predicates lower to BEAM type/equality guards via the same 0/1
//! synthesis the `cmp_*` ops use:
//!   `pair?`  → `is_nonempty_list` (a McCarthy cons IS a list cell `[H|T]`)
//!   `equal?` → `is_eq_exact` (Erlang `=:=`)
//!   `not`    → `is_eq_exact x 0` (logical `x == 0`)
//! **Verified by RUNNING** the emitted `.beam` on a real `erl` (skipped if absent).

use lang_aot::{compile_source_to_beam, Language};

fn erl_available() -> bool {
    std::process::Command::new("erl").arg("-version").output()
        .map(|o| o.status.success()).unwrap_or(false)
}

fn run(src: &str, module: &str) -> String {
    let bytes = compile_source_to_beam(Language::McCarthyLisp, src, module)
        .unwrap_or_else(|e| panic!("compile {src:?} to BEAM: {e}"));
    let tmp = std::env::temp_dir().join(format!("mccarthy_w10_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    std::fs::write(tmp.join(format!("{module}.beam")), &bytes).expect("write .beam");
    let out = std::process::Command::new("erl")
        .arg("-noshell").arg("-pa").arg(&tmp)
        .arg("-eval").arg(format!("io:format(\"~w~n\",[{module}:main()]),halt(0)."))
        .output().expect("spawn erl");
    assert!(out.status.success(), "erl non-zero; stderr: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn mccarthy_atom_eq_cond_run_on_beam() {
    if !erl_available() {
        eprintln!("erl absent — skipping BEAM predicates test");
        return;
    }
    assert_eq!(run("(ATOM 7)", "bp_a1"), "1", "an integer is an atom");
    assert_eq!(run("(ATOM (CONS 1 2))", "bp_a2"), "0", "a cons is not an atom");
    assert_eq!(run("(EQ 7 7)", "bp_e1"), "1", "equal atoms");
    assert_eq!(run("(EQ 7 8)", "bp_e2"), "0", "unequal atoms");
    assert_eq!(run("(COND ((ATOM 7) 100) ((ATOM 8) 200))", "bp_c1"), "100", "first clause true");
    assert_eq!(run("(COND ((EQ 1 2) 100) ((EQ 3 3) 200))", "bp_c2"), "200", "first false, second true");
    assert_eq!(run("(COND ((ATOM (CONS 1 2)) 100) ((EQ 5 5) 200))", "bp_c3"), "200", "cons not atom → fall through");
}
