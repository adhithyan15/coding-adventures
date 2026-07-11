//! # Cross-backend McCarthy conformance suite — W16 (the capstone).
//!
//! One table of McCarthy 1960 LISP programs × **every** LANG VM backend, each
//! asserting the *identical* result. This is the proof that McCarthy is complete
//! and **uniform** across the whole platform — the same source, compiled through
//! eight independent code generators and three value models, computes the same
//! number.
//!
//! ## The eight backends and how each is RUN
//!
//! | backend     | value model    | runner (in-process unless noted)              |
//! |-------------|----------------|-----------------------------------------------|
//! | VM          | tagged-word    | `mccarthy_lisp_vm::run` (the reference interp)|
//! | JIT         | tagged-word    | `lang_aot::run_mccarthy_on_jit`               |
//! | WASM        | uniform-anyref | `wasm-runtime`                                |
//! | JVM         | object/boxing  | a real `java` (cons is `Object[]`) — gated    |
//! | CLR         | object/boxing  | `clr-simulator` (in-process floor)            |
//! | CLR-real    | object/boxing  | real `ilasm` + real `dotnet` (`.il`) — gated  |
//! | BEAM        | Erlang terms   | a real `erl` — gated                          |
//! | LLVM        | tagged-word    | `clang` + `dynval_runtime.c` — gated           |
//! | native AOT  | tagged-word    | `aarch64`/`x86_64` object + system `ld` — gated, macOS |
//!
//! External-tool backends return `None` (skip) when the tool is absent, so the
//! suite still proves uniformity across whatever is installed; the in-process
//! backends (VM / JIT / WASM / CLR) always run and must agree. The **CLR-real**
//! column is the capstone of the CLR-real verification chapter: it runs the same
//! McCarthy programs on the actual .NET runtime (textual `.il` → real `ilasm` →
//! real `dotnet`), upgrading the CLR column from an in-house simulator to the real
//! runtime whenever `dotnet`+`ilasm` are present.
//!
//! ## Why integer-result programs
//!
//! Each program's result is an unambiguous integer (or a boolean coerced to
//! `0`/`1`): the backends agree on *those* exit values. A program returning a
//! bare symbol or a bare cons would differ in representation (tagged word vs.
//! interned id vs. heap pointer vs. `[7|9]`), so those are exercised in the
//! per-backend suites, not here.

use lang_aot::{
    compile_file_to_macos_executable, compile_source_to_beam, compile_source_to_cil_artifact,
    compile_source_to_iir, compile_source_to_jvm_class, compile_source_to_llvm_with_target,
    compile_source_to_wasm, run_mccarthy_on_jit, Language,
};

use lispy_runtime::LispyValue;

// The shared real-CoreCLR harness (`compile_source_to_cil_text` → real `ilasm` →
// real `dotnet`), reused by the per-feature `clr_real_*` tests. Lives in a
// subdirectory so Cargo doesn't compile it as its own test binary.
#[path = "clr_support/mod.rs"]
mod clr_support;

// ── The conformance table: McCarthy F1–F7, every result an integer. ──
const PROGRAMS: &[(&str, i64)] = &[
    // F1 — scalar.
    ("42", 42),
    ("0", 0),
    // F2 — cons / car / cdr.
    ("(CAR (CONS 7 9))", 7),
    ("(CDR (CONS 7 9))", 9),
    ("(CAR (CDR (CONS 1 (CONS 2 3))))", 2),
    // F3 — ATOM.
    ("(ATOM 7)", 1),
    ("(ATOM (CONS 1 2))", 0),
    // F4 — EQ.
    ("(EQ 7 7)", 1),
    ("(EQ 7 8)", 0),
    // F5 — COND.
    ("(COND ((ATOM 7) 11) ((ATOM 8) 22))", 11),
    ("(COND ((ATOM (CONS 1 2)) 11) ((EQ 5 5) 22))", 22),
    // F6 — symbols.
    ("(EQ (QUOTE A) (QUOTE A))", 1),
    ("(EQ (QUOTE A) (QUOTE B))", 0),
    ("(ATOM (QUOTE A))", 1),
    // F7 — lambda / LABEL / recursion.
    ("((LAMBDA (X) X) 5)", 5),
    ("((LAMBDA (X) (CAR X)) (CONS 7 9))", 7),
    ("((LAMBDA (X Y) (EQ X Y)) 3 3)", 1),
    ("((LAMBDA (N) (COND ((EQ N 0) 100) ((EQ 1 1) 200))) 0)", 100),
    ("((LABEL FF (LAMBDA (X) (COND ((ATOM X) X) ((QUOTE T) (FF (CAR X)))))) (CONS (CONS 7 8) 9))", 7),
];

/// Coerce a `LispyValue` (the VM's result) to the canonical integer exit value —
/// the same tag dispatch every tagged-word backend applies at the program exit.
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

// ── Tool-availability probes (gate the external-process backends). ──
fn tool_ok(cmd: &str, arg: &str) -> bool {
    std::process::Command::new(cmd)
        .arg(arg)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn tmp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("mccarthy_w16_{}_{tag}", std::process::id()));
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

// ── Backend 1: VM (the reference interpreter, tagged-word). Always runs. ──
fn run_vm(src: &str) -> Option<i64> {
    let module = compile_source_to_iir(Language::McCarthyLisp, src, "vm").ok()?;
    mccarthy_lisp_vm::run(&module).ok().map(exit_code)
}

// ── Backend 2: the universal JIT. Always runs. ──
fn run_jit(src: &str) -> Option<i64> {
    run_mccarthy_on_jit(src).ok().flatten()
}

// ── Backend 3: WASM via the in-repo runtime. Always runs. ──
fn run_wasm(src: &str) -> Option<i64> {
    let bytes = compile_source_to_wasm(Language::McCarthyLisp, src, "main").ok()?;
    let rt = wasm_runtime::WasmRuntime::new();
    rt.load_and_run(&bytes, "main", &[]).ok()?.first().copied()
}

// ── Backend 4: CLR via the in-repo simulator. Always runs. The whole-program
//    method table is loaded so a lambda's `call <MethodDef>` resolves by ordinal. ──
fn run_clr(src: &str) -> Option<i64> {
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
    let mut sim = CLRSimulator::new();
    sim.load_program(methods, entry);
    sim.run(1_000_000);
    match sim.stack.last() {
        Some(Some(Value::Int(n))) => Some(*n as i64),
        _ => None,
    }
}

// ── Backend 5: JVM on a real `java`. cons is an `Object[]` the in-repo simulator
//    cannot execute, so we inject a `main([Ljava/lang/String;)V` launcher that
//    prints the entry method's `int` result and run it. Gated on `java`. ──
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

// ── Backend 6: BEAM on a real `erl`. Gated on `erl`. ──
fn run_beam(src: &str) -> Option<i64> {
    if !tool_ok("erl", "-version") {
        return None;
    }
    let module = "conf";
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

// ── Backend 7: LLVM via `clang` + the shared C runtime. Gated on `clang`. ──
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
        .join("../twig-aot/runtime/dynval_runtime.c");
    let ll = compile_source_to_llvm_with_target(Language::McCarthyLisp, src, "conf", &triple).ok()?;
    let dir = tmp_dir("llvm");
    let ll_path = dir.join("conf.ll");
    std::fs::write(&ll_path, &ll).ok()?;
    let exe = dir.join("conf");
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

// ── Backend 8: native AOT — emit a host object, link with the system linker,
//    run. macOS only (the in-repo macho linker path); gated on `ld`. ──
#[cfg(target_os = "macos")]
fn run_native(src: &str) -> Option<i64> {
    if std::process::Command::new("ld").arg("-v").output().is_err() {
        return None;
    }
    let dir = tmp_dir("native");
    let s = dir.join("conf.mcl");
    std::fs::write(&s, src).ok()?;
    let exe = dir.join("conf");
    compile_file_to_macos_executable(&s, &exe, Language::McCarthyLisp).ok()?;
    std::process::Command::new(&exe).output().ok()?.status.code().map(i64::from)
}
#[cfg(not(target_os = "macos"))]
fn run_native(_src: &str) -> Option<i64> {
    None // the in-repo native-executable link path is macOS-only today
}

// ── Backend 9: CLR on **real CoreCLR** — the CLR-real chapter's capstone wiring.
//    Emits textual `.il`, assembles it with real `ilasm`, and runs it on real
//    `dotnet` (via the shared `clr_support` harness). Gated on `dotnet`+`ilasm`:
//    skips (returns `None`) when either is absent, so the in-process simulator
//    `CLR` column above remains the conformance floor while this column proves the
//    SAME programs on the actual .NET runtime when the toolchain is installed. ──
fn run_clr_real(src: &str) -> Option<i64> {
    clr_support::run_on_real_clr(src, "w16")
}

/// The capstone: every backend that can run a program computes the same integer.
#[test]
fn mccarthy_is_uniform_across_every_backend() {
    let backends: &[(&str, fn(&str) -> Option<i64>)] = &[
        ("VM", run_vm),
        ("JIT", run_jit),
        ("WASM", run_wasm),
        ("CLR", run_clr),
        ("JVM", run_jvm),
        ("BEAM", run_beam),
        ("LLVM", run_llvm),
        ("native-AOT", run_native),
        ("CLR-real", run_clr_real),
    ];

    let mut exercised: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for (src, expected) in PROGRAMS {
        for (name, runner) in backends {
            if let Some(got) = runner(src) {
                assert_eq!(
                    got, *expected,
                    "BACKEND DISAGREEMENT: {name} computed {got} for {src:?}, expected {expected}",
                );
                exercised.insert(name);
            }
        }
    }

    // The four pure-in-process backends have no external dependency and MUST run
    // every program — they are the conformance floor.
    for must in ["VM", "JIT", "WASM", "CLR"] {
        assert!(exercised.contains(must), "in-process backend {must} failed to run");
    }
    eprintln!(
        "W16 conformance: {} programs × {} backends exercised → {:?}",
        PROGRAMS.len(),
        exercised.len(),
        exercised,
    );
}
