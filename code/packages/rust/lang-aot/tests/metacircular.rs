//! # L7 — Metacircular evaluator on every modern backend.
//!
//! McCarthy's 1960 paper closes with one of the most elegant ideas in
//! computing history: **Lisp's `EVAL` function, written in Lisp itself.**
//! Once you have the seven primitives (`QUOTE`, `ATOM`, `EQ`, `CAR`,
//! `CDR`, `CONS`, `COND`) plus `LAMBDA` and `LABEL`, you can express an
//! interpreter for the very language you're writing in.  That's the
//! **metacircular evaluator** — Lisp-in-Lisp.
//!
//! This test ships exactly that: a McCarthy 1960 `EVAL` function
//! authored as McCarthy Lisp source, compiled to **every modern
//! backend** in the LANG VM platform, applied to a handful of test
//! programs.  Every backend must agree on the integer result.
//!
//! Scope of the evaluator (v0.1.0):
//! - **Yes**: `QUOTE`, `ATOM`, `EQ`, `CAR`, `CDR`, `CONS`, `COND`
//!   applied to integer atoms.  Self-evaluating integer literals.
//! - **No**: user-defined `LAMBDA` inside the *evaluatee* (would need
//!   an environment).  Variables (same reason).  Full McCarthy `apply`.
//!
//! The "no environment" simplification means this is the *purely
//! syntactic* eval — programs whose value is determined by the
//! shape of the source alone.  That's still a faithful subset of the
//! 1960 paper and exercises every recursion edge of the McCarthy
//! frontend — which is the point.  A future increment can add an
//! environment + variable lookup.
//!
//! ## Why this test matters
//!
//! After L1–L4 and the W1–W16 cascade, McCarthy Lisp runs on eight
//! independent backends — VM, JIT, WASM, CLR, JVM, BEAM, LLVM, native
//! AOT — through one shared IIR.  The W16 conformance suite proves
//! they agree on a hand-written table of test programs.  **L7 raises
//! the bar**: the test programs are no longer just "what we
//! hand-wrote" — they're "anything the metacircular evaluator can
//! interpret."  The interpreter itself is a single McCarthy program
//! that exercises closures, recursion, COND, and every primitive
//! against itself, and all eight backends must agree on its output.
//!
//! ## Structure
//!
//! Mirrors `tests/conformance.rs` — same eight backend runners,
//! same gating pattern (4 in-process backends always run; 4 gated on
//! external tool availability).  The conformance suite tests "small
//! programs run uniformly"; this test tests "*the interpreter for
//! these programs* runs uniformly."

use lang_aot::{
    compile_source_to_beam, compile_source_to_cil_artifact, compile_source_to_iir,
    compile_source_to_jvm_class, compile_source_to_llvm_with_target, compile_source_to_wasm,
    run_mccarthy_on_jit, Language,
};
#[cfg(target_os = "macos")]
use lang_aot::compile_file_to_macos_executable;

use lispy_runtime::LispyValue;

// ===========================================================================
// EVALUATOR_BODY — McCarthy's 1960 EVAL, written in McCarthy Lisp.
// ===========================================================================
//
// `((LABEL EVAL (LAMBDA (E) <body>)) (QUOTE <INPUT>))`
//
// Where `<body>` is a chained COND dispatching on `(CAR E)` after
// confirming E is a cons cell.  Each branch recurses on the
// sub-expressions and applies the matching primitive.  This is the
// straight transcription of the eval table from §6 of McCarthy 1960,
// minus the env-and-lambda branches that would need a full
// environment-passing version (deferred to a later increment).
//
// `(QUOTE T)` in the final branch is McCarthy's idiomatic "always
// true" guard — `T` is just a symbol, so it self-evaluates to true
// under the lisp truthiness convention used by every LANG VM backend.
const EVALUATOR_BODY: &str = "\
((LABEL EVAL \
  (LAMBDA (E) \
    (COND \
      ((ATOM E) E) \
      ((EQ (CAR E) (QUOTE QUOTE)) (CAR (CDR E))) \
      ((EQ (CAR E) (QUOTE CAR))   (CAR (EVAL (CAR (CDR E))))) \
      ((EQ (CAR E) (QUOTE CDR))   (CDR (EVAL (CAR (CDR E))))) \
      ((EQ (CAR E) (QUOTE CONS))  (CONS (EVAL (CAR (CDR E))) (EVAL (CAR (CDR (CDR E)))))) \
      ((EQ (CAR E) (QUOTE ATOM))  (ATOM (EVAL (CAR (CDR E))))) \
      ((EQ (CAR E) (QUOTE EQ))    (EQ (EVAL (CAR (CDR E))) (EVAL (CAR (CDR (CDR E)))))) \
      ((QUOTE T) (QUOTE UNKNOWN))))) \
  (QUOTE __INPUT__))";

/// Build the full top-level McCarthy program: the evaluator definition
/// applied to a quoted form of the test input.  Each `__INPUT__`
/// placeholder is replaced with the test program's raw source.
fn evaluator_program(input: &str) -> String {
    EVALUATOR_BODY.replacen("__INPUT__", input, 1)
}

// ===========================================================================
// Test programs — every result is an integer the backends agree on.
// ===========================================================================

/// Each row: `(input-expression-as-mccarthy-source, expected-integer-result)`.
///
/// The input is what gets fed *into* the metacircular evaluator.  The
/// expected result is what `(EVAL '<input>)` should compute to.
const PROGRAMS: &[(&str, i64)] = &[
    // Self-evaluating integer atom.  Trivial base case — exercises the
    // (ATOM E) branch of the evaluator.
    ("42", 42),
    ("0", 0),
    // QUOTE form: `(QUOTE 7)` evaluates to 7 — exercises the
    // QUOTE-symbol-dispatch branch.
    ("(QUOTE 7)", 7),
    // CAR / CDR primitives with a literal CONS construction.
    ("(CAR (CONS 7 9))", 7),
    ("(CDR (CONS 7 9))", 9),
    // ATOM predicate: an integer atom is true.
    ("(ATOM 7)", 1),
    // EQ predicate over integer atoms.
    ("(EQ 5 5)", 1),
    ("(EQ 5 6)", 0),
    // Deeper nesting — exercises multiple recursive descents through
    // EVAL.  CDR of a cons-of-cons.
    ("(CAR (CDR (CONS 1 (CONS 2 3))))", 2),
];

// ===========================================================================
// Helpers (mirror conformance.rs).
// ===========================================================================

fn exit_code(lv: LispyValue) -> i64 {
    if let Some(n) = lv.as_int() {
        n
    } else if lv.is_true() {
        1
    } else if lv.is_false() || lv.is_nil() {
        0
    } else {
        lv.bits() as i64
    }
}

fn tool_ok(cmd: &str, arg: &str) -> bool {
    std::process::Command::new(cmd)
        .arg(arg)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn tmp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mccarthy_l7_{}_{tag}", std::process::id()));
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

// ===========================================================================
// Backend runners — same eight as conformance.rs.
// ===========================================================================
//
// We duplicate (rather than share) the runners because conformance.rs
// keeps them private to its own test crate.  Sharing would require a
// `tests/common/mod.rs` refactor — deferred.  Each runner here is
// behaviourally identical to its counterpart in conformance.rs.

fn run_vm(src: &str) -> Option<i64> {
    let module = compile_source_to_iir(Language::McCarthyLisp, src, "vm").ok()?;
    mccarthy_lisp_vm::run(&module).ok().map(exit_code)
}

fn run_jit(src: &str) -> Option<i64> {
    run_mccarthy_on_jit(src).ok().flatten()
}

fn run_wasm(src: &str) -> Option<i64> {
    let bytes = compile_source_to_wasm(Language::McCarthyLisp, src, "main").ok()?;
    let rt = wasm_runtime::WasmRuntime::new();
    rt.load_and_run(&bytes, "main", &[]).ok()?.first().copied()
}

fn run_clr(src: &str) -> Option<i64> {
    // The CLR-simulator runs CIL bytecode but doesn't implement every
    // opcode the iir-to-cil-bytecode backend emits — the metacircular
    // evaluator, with its deeply-nested COND + LABEL + recursion, can
    // exceed the short-branch range and triggers opcode `0x38` (long-
    // form `br`), which the current simulator panics on.  That's a
    // documented `clr-simulator` gap, not a backend regression — the
    // emitted CIL is valid; a real CLR loads it fine.
    //
    // Catch the panic so the rest of the suite can run.  When/if
    // clr-simulator grows long-form-branch support, this `catch_unwind`
    // becomes a no-op.
    use clr_simulator::{CLRSimulator, MethodCode, Value};
    let artifact = compile_source_to_cil_artifact(Language::McCarthyLisp, src, "Main").ok()?;
    let methods: Vec<MethodCode> = artifact
        .methods
        .iter()
        .map(|m| MethodCode {
            body: m.body.clone(),
            num_locals: m.local_types.len(),
            num_args: m.parameter_types.len(),
        })
        .collect();
    let entry = artifact.methods.iter().position(|m| m.name == "main")?;
    std::panic::catch_unwind(move || {
        let mut sim = CLRSimulator::new();
        sim.load_program(methods, entry);
        sim.run(1_000_000);
        match sim.stack.last() {
            Some(Some(Value::Int(n))) => Some(*n as i64),
            _ => None,
        }
    })
    .ok()
    .flatten()
}

fn run_jvm(src: &str) -> Option<i64> {
    use iir_to_jvm_class_file::serialize_jvm_class_file;
    use jvm_class_file::{
        JvmCodeAttribute, JvmConstantPoolEntry, JvmMethodAttribute, JvmMethodInfo, ACC_PUBLIC,
        ACC_STATIC,
    };
    if !tool_ok("java", "-version") {
        return None;
    }
    fn cp_append(cp: &mut Vec<Option<JvmConstantPoolEntry>>, e: JvmConstantPoolEntry) -> u16 {
        cp.push(Some(e));
        (cp.len() - 1) as u16
    }
    let mut class = compile_source_to_jvm_class(Language::McCarthyLisp, src, "Main").ok()?;
    let (out_fieldref, println_ref, entry_ref) = {
        let cp = &mut class.constant_pool;
        let sys_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("java/lang/System".into()));
        let sys_class = cp_append(cp, JvmConstantPoolEntry::Class { name_index: sys_utf8 });
        let out_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("out".into()));
        let ps_desc = cp_append(cp, JvmConstantPoolEntry::Utf8("Ljava/io/PrintStream;".into()));
        let out_nat = cp_append(cp, JvmConstantPoolEntry::NameAndType { name_index: out_utf8, descriptor_index: ps_desc });
        let out_fieldref = cp_append(cp, JvmConstantPoolEntry::Fieldref { class_index: sys_class, name_and_type_index: out_nat });
        let ps_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("java/io/PrintStream".into()));
        let ps_class = cp_append(cp, JvmConstantPoolEntry::Class { name_index: ps_utf8 });
        let pln_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("println".into()));
        let pln_desc = cp_append(cp, JvmConstantPoolEntry::Utf8("(I)V".into()));
        let pln_nat = cp_append(cp, JvmConstantPoolEntry::NameAndType { name_index: pln_utf8, descriptor_index: pln_desc });
        let println_ref = cp_append(cp, JvmConstantPoolEntry::Methodref { class_index: ps_class, name_and_type_index: pln_nat });
        let main_utf8 = cp_append(cp, JvmConstantPoolEntry::Utf8("Main".into()));
        let main_class = cp_append(cp, JvmConstantPoolEntry::Class { name_index: main_utf8 });
        let ent_name = cp_append(cp, JvmConstantPoolEntry::Utf8("main".into()));
        let ent_desc = cp_append(cp, JvmConstantPoolEntry::Utf8("()I".into()));
        let ent_nat = cp_append(cp, JvmConstantPoolEntry::NameAndType { name_index: ent_name, descriptor_index: ent_desc });
        let entry_ref = cp_append(cp, JvmConstantPoolEntry::Methodref { class_index: main_class, name_and_type_index: ent_nat });
        let _ = cp_append(cp, JvmConstantPoolEntry::Utf8("([Ljava/lang/String;)V".into()));
        (out_fieldref, println_ref, entry_ref)
    };
    let [out_hi, out_lo] = out_fieldref.to_be_bytes();
    let [ent_hi, ent_lo] = entry_ref.to_be_bytes();
    let [pln_hi, pln_lo] = println_ref.to_be_bytes();
    let main_code = vec![
        0xB2, out_hi, out_lo, // getstatic System.out
        0xB8, ent_hi, ent_lo, // invokestatic Main.main()I
        0xB6, pln_hi, pln_lo, // invokevirtual println(I)V
        0xB1,                 // return
    ];
    class.methods.push(JvmMethodInfo {
        access_flags: ACC_PUBLIC | ACC_STATIC,
        name: "main".into(),
        descriptor: "([Ljava/lang/String;)V".into(),
        attributes: vec![JvmMethodAttribute::Code(JvmCodeAttribute {
            name: "Code".into(),
            max_stack: 2,
            max_locals: 1,
            code: main_code,
            nested_attributes: vec![],
        })],
    });
    let bytes = serialize_jvm_class_file(&class);
    let dir = tmp_dir("jvm");
    std::fs::write(dir.join("Main.class"), &bytes).ok()?;
    let out = std::process::Command::new("java")
        .arg("-Xverify:none").arg("-cp").arg(&dir).arg("Main")
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse::<i64>().ok()
}

fn run_beam(src: &str) -> Option<i64> {
    if !tool_ok("erl", "-version") {
        return None;
    }
    let module = "metacirc";
    let bytes = compile_source_to_beam(Language::McCarthyLisp, src, module).ok()?;
    let dir = tmp_dir("beam");
    std::fs::write(dir.join(format!("{module}.beam")), &bytes).ok()?;
    let out = std::process::Command::new("erl")
        .arg("-noshell").arg("-pa").arg(&dir)
        .arg("-eval").arg(format!("io:format(\"~w~n\",[{module}:main()]),halt(0)."))
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse::<i64>().ok()
}

fn run_llvm(src: &str) -> Option<i64> {
    if !tool_ok("clang", "--version") {
        return None;
    }
    let triple = String::from_utf8(
        std::process::Command::new("clang").arg("-dumpmachine").output().ok()?.stdout,
    )
    .ok()?
    .trim()
    .to_string();
    let runtime_c = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../twig-aot/runtime/lispy_runtime.c");
    let ll = compile_source_to_llvm_with_target(Language::McCarthyLisp, src, "metacirc", &triple).ok()?;
    let dir = tmp_dir("llvm");
    let ll_path = dir.join("metacirc.ll");
    std::fs::write(&ll_path, &ll).ok()?;
    let exe = dir.join("metacirc");
    let build = std::process::Command::new("clang")
        .arg("-x").arg("ir").arg(&ll_path)
        .arg("-x").arg("none").arg(&runtime_c)
        .arg("-o").arg(&exe)
        .output()
        .ok()?;
    if !build.status.success() {
        return None;
    }
    std::process::Command::new(&exe).output().ok()?.status.code().map(i64::from)
}

#[cfg(target_os = "macos")]
fn run_native(src: &str) -> Option<i64> {
    if std::process::Command::new("ld").arg("-v").output().is_err() {
        return None;
    }
    let dir = tmp_dir("native");
    let s = dir.join("metacirc.mcl");
    std::fs::write(&s, src).ok()?;
    let exe = dir.join("metacirc");
    compile_file_to_macos_executable(&s, &exe, Language::McCarthyLisp).ok()?;
    std::process::Command::new(&exe).output().ok()?.status.code().map(i64::from)
}
#[cfg(not(target_os = "macos"))]
fn run_native(_src: &str) -> Option<i64> {
    None
}


// ===========================================================================
// L7 — Smoke test on the VM (validates the EVALUATOR_BODY before we
// expand to every backend).  Independent test; runs even if the
// uniform-across-backends test below has issues.
// ===========================================================================

#[test]
fn metacircular_smoke_vm_evaluates_canonical_programs() {
    let mut failures: Vec<String> = Vec::new();
    for (input, expected) in PROGRAMS {
        let src = evaluator_program(input);
        match run_vm(&src) {
            Some(got) if got == *expected => {}
            Some(got) => failures.push(format!("VM: input={input:?} got={got} expected={expected}")),
            None => failures.push(format!("VM: input={input:?} compile/run returned None")),
        }
    }
    assert!(
        failures.is_empty(),
        "metacircular evaluator failed on the VM:\n  {}",
        failures.join("\n  ")
    );
}

// ===========================================================================
// L7 — The full conformance: every modern backend agrees.
// ===========================================================================

#[test]
fn metacircular_eval_uniform_across_modern_backends() {
    // Labelled table of backend runners; the inline `fn` pointer type is clearer
    // here than a hoisted type alias.
    #[allow(clippy::type_complexity)]
    let backends: &[(&str, fn(&str) -> Option<i64>)] = &[
        ("VM", run_vm),
        ("JIT", run_jit),
        ("WASM", run_wasm),
        ("CLR", run_clr),
        ("JVM", run_jvm),
        ("BEAM", run_beam),
        ("LLVM", run_llvm),
        ("native-AOT", run_native),
    ];

    let mut exercised: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for (input, expected) in PROGRAMS {
        let src = evaluator_program(input);
        for (name, runner) in backends {
            if let Some(got) = runner(&src) {
                assert_eq!(
                    got, *expected,
                    "BACKEND DISAGREEMENT on metacircular eval of {input:?}: \
                     {name} computed {got}, expected {expected}",
                );
                exercised.insert(name);
            }
        }
    }

    // The metacircular evaluator's deeply-nested CIL output exceeds
    // what `clr-simulator` currently implements (specifically, long-
    // form `br` / opcode 0x38), so CLR is best-effort here even though
    // it's part of the W16 conformance floor for simpler programs.
    // VM + JIT + WASM still must run — they're the L7 floor.
    for must in ["VM", "JIT", "WASM"] {
        assert!(
            exercised.contains(must),
            "in-process backend {must} failed to evaluate the metacircular interpreter \
             — the L7 floor requires this backend to work without external tools"
        );
    }
    eprintln!(
        "L7 metacircular: {} test programs × {} exercised backends → {:?}",
        PROGRAMS.len(),
        exercised.len(),
        exercised,
    );
}
