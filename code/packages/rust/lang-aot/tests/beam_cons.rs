//! # BEAM cons — F2 (LANG77 / McCarthy W9b).
//!
//! Unlike the managed backends (wasm/JVM/CLR), BEAM uses the **native
//! Erlang-terms** value model — NOT the boxing structural pass. A McCarthy cons
//! cell is a native Erlang list cell `[H|T]`; `car`/`cdr` are `hd`/`tl`;
//! integers are native Erlang integers; nil is `[]`. `lang-aot`'s
//! `compile_source_to_beam` runs `lower_heap_builtins` (cons/car/cdr →
//! `alloc ref<LispyPair>` + `field_store`/`field_load`), and `iir-to-beam` maps
//! those to `put_list` / `get_hd` / `get_tl`. **Verified by RUNNING** the emitted
//! `.beam` on a real `erl` (skipped if `erl` is absent).

use lang_aot::{compile_source_to_beam, Language};

fn erl_available() -> bool {
    std::process::Command::new("erl").arg("-version").output()
        .map(|o| o.status.success()).unwrap_or(false)
}

/// Compile to `.beam`, run `module:main()` on a real `erl`, return its printout.
fn run(src: &str, module: &str) -> String {
    let bytes = compile_source_to_beam(Language::McCarthyLisp, src, module)
        .unwrap_or_else(|e| panic!("compile {src:?} to BEAM: {e}"));
    let tmp = std::env::temp_dir().join(format!("mccarthy_w9b_{}", std::process::id()));
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
fn mccarthy_cons_car_cdr_run_on_beam() {
    if !erl_available() {
        eprintln!("erl absent — skipping BEAM cons test");
        return;
    }
    assert_eq!(run("(CAR (CONS 7 9))", "bc_car"), "7", "car of a cons");
    assert_eq!(run("(CDR (CONS 7 9))", "bc_cdr"), "9", "cdr of a cons");
    assert_eq!(run("(CAR (CDR (CONS 1 (CONS 2 3))))", "bc_nest"), "2", "car of cdr of nested cons");
    // A McCarthy dotted pair is a *native Erlang* improper list cell `[H|T]`.
    assert_eq!(run("(CONS 7 9)", "bc_cell"), "[7|9]", "a cons IS an Erlang list cell");
    assert_eq!(run("42", "bc_scalar"), "42", "scalar still runs (backward compat)");
}
