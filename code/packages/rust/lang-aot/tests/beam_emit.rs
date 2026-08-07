//! # BEAM emit + run tests (LANG77 / McCarthy W9a).
//!
//! The fourth *managed* `--emit` target, and the first on the Erlang VM. These
//! RUN the emitted `.beam` on a real `erl` (OTP) and assert the result — the
//! same verify-by-running discipline as the wasm/jvm/clr paths. W9a scope:
//! **scalar** McCarthy programs; the cons/symbol/lambda Erlang-terms model is W9+.

use lang_aot::{compile_source_to_beam, Language};

fn erl_available() -> bool {
    std::process::Command::new("erl")
        .args(["-noshell", "-eval", "halt(0)."])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Compile a scalar program to a `.beam`, load it on a real `erl`, call
/// `module:main()`, and return its printed result (or None if `erl` is absent).
fn compile_and_run(language: Language, source: &str, module: &str) -> Option<String> {
    if !erl_available() {
        return None;
    }
    let bytes = compile_source_to_beam(language, source, module)
        .unwrap_or_else(|e| panic!("compile {source:?} to BEAM: {e}"));
    let tmp = std::env::temp_dir().join(format!("mccarthy_w9_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    std::fs::write(tmp.join(format!("{module}.beam")), &bytes).expect("write .beam");
    let out = std::process::Command::new("erl")
        .arg("-noshell")
        .arg("-pa").arg(&tmp)
        .arg("-eval")
        .arg(format!("io:format(\"~w~n\",[{module}:main()]),halt(0)."))
        .output()
        .expect("spawn erl");
    assert!(out.status.success(), "erl non-zero; stderr: {}", String::from_utf8_lossy(&out.stderr));
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[test]
fn mccarthy_scalar_emits_and_runs_on_beam() {
    let Some(_) = compile_and_run(Language::McCarthyLisp, "42", "mc_probe") else {
        eprintln!("erl absent — skipped");
        return;
    };
    assert_eq!(compile_and_run(Language::McCarthyLisp, "42", "mc_a").unwrap(), "42", "McCarthy 42");
    assert_eq!(compile_and_run(Language::McCarthyLisp, "0", "mc_b").unwrap(), "0", "McCarthy 0");
    assert_eq!(compile_and_run(Language::McCarthyLisp, "7", "mc_c").unwrap(), "7", "McCarthy 7");
}

#[test]
fn twig_scalar_emits_and_runs_on_beam() {
    if !erl_available() {
        return;
    }
    assert_eq!(compile_and_run(Language::Twig, "42", "tw_a").unwrap(), "42", "Twig 42");
}

#[test]
fn algol_runtime_string_local_emits_and_runs_on_beam() {
    if !erl_available() {
        return;
    }
    let source = "begin string s; integer result; \
                  string procedure pick(n); value n; integer n; \
                    if n > 0 then pick := 'HI' else pick := 'LO'; \
                  s := pick(1); \
                  if s = 'HI' then result := 42 else result := 0; \
                  print(s) end";
    assert_eq!(
        compile_and_run(Language::Algol60, source, "algol_runtime_string_beam").unwrap(),
        "HI42",
    );
}

#[test]
fn algol_runtime_string_ordering_emits_and_runs_on_beam() {
    if !erl_available() {
        return;
    }
    let source = "begin string s; integer result; \
                  string procedure pick(n); value n; integer n; \
                    if n > 0 then pick := 'HI' else pick := 'LO'; \
                  s := pick(1); \
                  if s < 'LO' then result := 42 else result := 0; \
                  print(s) end";
    assert_eq!(
        compile_and_run(Language::Algol60, source, "algol_runtime_string_ordering_beam").unwrap(),
        "HI42",
    );
}
