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
    compile_source_to_beam, compile_source_to_cil_artifact,
    compile_source_to_iir, compile_source_to_jvm_class, compile_source_to_llvm_with_target,
    compile_source_to_wasm, run_mccarthy_on_jit, Language,
};

// `compile_file_to_macos_executable` is `#[cfg(unix)]` in `lang_aot` itself and only
// ever called from the `#[cfg(target_os = "macos")]` native-AOT arm below — gate the
// import to match, so a non-macOS build (which never calls it) still compiles.
#[cfg(target_os = "macos")]
use lang_aot::compile_file_to_macos_executable;

use dynval_runtime::LispyValue;

// The shared real-CoreCLR harness (`compile_source_to_cil_text` → real `ilasm` →
// real `dotnet`), reused by the per-feature `clr_real_*` tests. Lives in a
// subdirectory so Cargo doesn't compile it as its own test binary.
#[path = "clr_support/mod.rs"]
mod clr_support;

// `common::gc_link_args()` — the `gc-core-capi` staticlib that replaced the retired
// `twig_gc.c`. `dynval_runtime.c` references `__gc_alloc_kind`/`__gc_register_kind`
// from it, so the LLVM column cannot link without it (see `run_llvm`).
#[path = "common/mod.rs"]
mod common;

/// A backend was present and the pipeline under test failed. See the identical
/// policy note on `lang_matrix::cell_failed`: the ONLY legitimate reason for a
/// backend to produce no result is an absent host toolchain. Anything else is a
/// real failure and must be loud.
///
/// This suite is where the cost of getting that wrong was demonstrated. Every
/// runner below returned a bare `None` on failure, indistinguishable from "tool not
/// installed" — so when `dynval_runtime.c` grew a dependency on the `gc-core-capi`
/// archive and `run_llvm`'s link line was never updated, `clang` failed, `clang`
/// failing returned `None`, and the LLVM column vanished from the capstone while it
/// still reported `ok`. It stayed gone until someone read the exercised-backend set
/// and noticed LLVM missing from it.
fn backend_failed(backend: &str, src: &str, stage: &str, detail: impl std::fmt::Display) -> ! {
    panic!(
        "{backend}: {stage} failed for program {src:?} — this is a REAL failure, not a skip.\n\
         error:\n{detail}"
    )
}

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

/// A fresh, private scratch directory for one backend's artifacts.
///
/// This used to be `temp_dir()/mccarthy_w16_<pid>_<tag>` — a fully predictable
/// path that `create_dir_all` would happily adopt if it already existed. The
/// harness then writes a `.ll`/`.class`/`.beam` there, links an executable into
/// it, and runs it. A local attacker who pre-creates the directory (the PID
/// space is small enough to enumerate) owns it, and can redirect `clang -o`
/// through a symlink to clobber any file the developer can write, or swap the
/// binary between the link and the exec — CWE-377/367. `/tmp`'s sticky bit does
/// not help when the attacker owns the containing directory.
///
/// `tempfile::tempdir()` is `mkdtemp`: random name, mode `0700`, and it fails
/// rather than adopting an existing directory. The sibling `lang_matrix.rs`
/// already did it this way; this suite is where the habit had not reached.
///
/// The returned guard must outlive the run — dropping it deletes the directory,
/// so callers bind it, not just `.path()`.
fn tmp_dir(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("mccarthy_w16_{tag}_"))
        .tempdir()
        .expect("create a private temp dir")
}

// ── Backend 1: VM (the reference interpreter, tagged-word). Always runs. ──
fn run_vm(src: &str) -> Option<i64> {
    let module = match compile_source_to_iir(Language::McCarthyLisp, src, "vm") {
        Ok(m) => m,
        Err(e) => backend_failed("VM", src, "source → IIR", format!("{e:?}")),
    };
    match mccarthy_lisp_vm::run(&module) {
        Ok(v) => Some(exit_code(v)),
        Err(e) => backend_failed("VM", src, "mccarthy-lisp-vm execution", format!("{e:?}")),
    }
}

// ── Backend 2: the universal JIT. Always runs. ──
fn run_jit(src: &str) -> Option<i64> {
    match run_mccarthy_on_jit(src) {
        Ok(v) => Some(v.unwrap_or_else(|| {
            backend_failed("JIT", src, "reading the JIT result", "produced no value")
        })),
        Err(e) => backend_failed("JIT", src, "JIT execution", format!("{e:?}")),
    }
}

// ── Backend 3: WASM via the in-repo runtime. Always runs. ──
fn run_wasm(src: &str) -> Option<i64> {
    let bytes = match compile_source_to_wasm(Language::McCarthyLisp, src, "main") {
        Ok(b) => b,
        Err(e) => backend_failed("WASM", src, "source → wasm bytes", format!("{e:?}")),
    };
    let rt = wasm_runtime::WasmRuntime::new();
    match rt.load_and_run(&bytes, "main", &[]) {
        Ok(vals) => Some(vals.first().copied().unwrap_or_else(|| {
            backend_failed("WASM", src, "reading `main`'s result", "returned no value")
        })),
        Err(e) => backend_failed("WASM", src, "wasm-runtime execution", format!("{e:?}")),
    }
}

// ── Backend 4: CLR via the in-repo simulator. Always runs. The whole-program
//    method table is loaded so a lambda's `call <MethodDef>` resolves by ordinal. ──
fn run_clr(src: &str) -> Option<i64> {
    use clr_simulator::{CLRSimulator, MethodCode, Value};
    let artifact = match compile_source_to_cil_artifact(Language::McCarthyLisp, src, "Main") {
        Ok(a) => a,
        Err(e) => backend_failed("CLR", src, "source → CIL artifact", format!("{e:?}")),
    };
    let methods: Vec<MethodCode> = artifact
        .methods
        .iter()
        .map(|m| MethodCode {
            body: m.body.clone(),
            num_locals: m.local_types.len(),
            num_args: m.parameter_types.len(),
        })
        .collect();
    let Some(entry) = artifact.methods.iter().position(|m| m.name == "main") else {
        backend_failed("CLR", src, "locating the entry method", "artifact has no `main`");
    };
    let mut sim = CLRSimulator::new();
    sim.load_program(methods, entry);
    sim.run(1_000_000);
    match sim.stack.last() {
        Some(Some(Value::Int(n))) => Some(*n as i64),
        other => backend_failed(
            "CLR",
            src,
            "reading the simulator's result",
            format!("top of stack was {other:?}, expected an Int"),
        ),
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
    let mut class = match compile_source_to_jvm_class(Language::McCarthyLisp, src, "Main") {
        Ok(c) => c,
        Err(e) => backend_failed("JVM", src, "source → JVM class file", format!("{e:?}")),
    };
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
    std::fs::write(dir.path().join("Main.class"), &bytes).expect("jvm: write Main.class");
    let out = std::process::Command::new("java")
        .arg("-Xverify:none").arg("-cp").arg(dir.path()).arg("Main")
        .output()
        .expect("jvm: spawn java");
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    match stdout.parse::<i64>() {
        Ok(v) => Some(v),
        Err(_) => backend_failed(
            "JVM",
            src,
            "`java` execution of the emitted class",
            format!(
                "exit {:?}, stdout {stdout:?}\n{}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr)
            ),
        ),
    }
}

// ── Backend 6: BEAM on a real `erl`. Gated on `erl`. ──
fn run_beam(src: &str) -> Option<i64> {
    if !tool_ok("erl", "-version") {
        return None;
    }
    let module = "conf";
    let bytes = match compile_source_to_beam(Language::McCarthyLisp, src, module) {
        Ok(b) => b,
        Err(e) => backend_failed("BEAM", src, "source → BEAM bytes", format!("{e:?}")),
    };
    let dir = tmp_dir("beam");
    std::fs::write(dir.path().join(format!("{module}.beam")), &bytes).expect("beam: write .beam");
    let out = std::process::Command::new("erl")
        .arg("-noshell").arg("-pa").arg(dir.path())
        .arg("-eval").arg(format!("io:format(\"~w~n\",[{module}:main()]),halt(0)."))
        .output()
        .expect("beam: spawn erl");
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    match stdout.parse::<i64>() {
        Ok(v) => Some(v),
        Err(_) => backend_failed(
            "BEAM",
            src,
            "`erl` execution of the emitted module",
            format!(
                "exit {:?}, stdout {stdout:?}\n{}",
                out.status.code(),
                String::from_utf8_lossy(&out.stderr)
            ),
        ),
    }
}

// ── Backend 7: LLVM via `clang` + the shared C runtime. Gated on `clang`. ──
fn run_llvm(src: &str) -> Option<i64> {
    if !tool_ok("clang", "--version") {
        return None;
    }
    let triple = String::from_utf8(
        std::process::Command::new("clang")
            .arg("-dumpmachine")
            .output()
            .expect("llvm: spawn clang -dumpmachine")
            .stdout,
    )
    .expect("llvm: clang -dumpmachine emitted valid UTF-8")
    .trim()
    .to_string();
    let runtime_c = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../twig-aot/runtime/dynval_runtime.c");
    let ll = match compile_source_to_llvm_with_target(Language::McCarthyLisp, src, "conf", &triple) {
        Ok(ll) => ll,
        Err(e) => backend_failed("LLVM", src, "source → LLVM IR", format!("{e:?}")),
    };
    let dir = tmp_dir("llvm");
    let ll_path = dir.path().join("conf.ll");
    std::fs::write(&ll_path, &ll).expect("llvm: write conf.ll");
    let exe = dir.path().join("conf");
    // `dynval_runtime.c`'s `__dyn_cons` calls `__gc_alloc_kind`/`__gc_register_kind`,
    // which moved into the `gc-core-capi` staticlib when `twig_gc.c` was retired
    // (#118b-2b). Without `gc_link_args()` the link fails with undefined symbols —
    // which, before `backend_failed`, silently removed the whole LLVM column from
    // this capstone. The per-feature `llvm_*.rs` suites have always linked it; this
    // runner is the one that was never updated.
    let build = std::process::Command::new("clang")
        .arg("-x").arg("ir").arg(&ll_path)
        .arg("-x").arg("none").arg(&runtime_c)
        .args(common::gc_link_args())
        .arg("-o").arg(&exe)
        .output()
        .expect("llvm: spawn clang");
    if !build.status.success() {
        backend_failed(
            "LLVM",
            src,
            "clang link of the emitted .ll",
            String::from_utf8_lossy(&build.stderr),
        );
    }
    let out = std::process::Command::new(&exe).output().expect("llvm: run linked executable");
    match out.status.code() {
        Some(c) => Some(i64::from(c)),
        None => backend_failed("LLVM", src, "reading the exit code", "process was signalled"),
    }
}

// ── Backend 8: native AOT — emit a host object, link with the system linker,
//    run. macOS only (the in-repo macho linker path); gated on `ld`. ──
#[cfg(target_os = "macos")]
fn run_native(src: &str) -> Option<i64> {
    if std::process::Command::new("ld").arg("-v").output().is_err() {
        return None;
    }
    let dir = tmp_dir("native");
    let s = dir.path().join("conf.mcl");
    std::fs::write(&s, src).expect("native: write source");
    let exe = dir.path().join("conf");
    if let Err(e) = compile_file_to_macos_executable(&s, &exe, Language::McCarthyLisp) {
        backend_failed("native-AOT", src, "source → macOS executable", format!("{e:?}"));
    }
    let out = std::process::Command::new(&exe).output().expect("native: run linked executable");
    match out.status.code() {
        Some(c) => Some(i64::from(c)),
        None => {
            backend_failed("native-AOT", src, "reading the exit code", "process was signalled")
        }
    }
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
    // A labelled table of backend runners; the `fn` pointer type is intentional
    // and reads clearly inline, so keep it rather than hoisting a type alias.
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

    // Every *gated* backend must also appear once its tool is installed. A backend
    // is allowed to be missing from `exercised` only when the host genuinely lacks
    // its toolchain — so pair each with its probe and assert the implication. This
    // is the check that was absent when the LLVM column silently disappeared: the
    // suite printed a set of exercised backends, LLVM was simply not in it, and
    // nothing compared that set against what the machine could actually run.
    let gated: &[(&str, bool)] = &[
        ("JVM", tool_ok("java", "-version")),
        ("BEAM", tool_ok("erl", "-version")),
        ("LLVM", tool_ok("clang", "--version")),
        ("native-AOT", cfg!(target_os = "macos")),
        ("CLR-real", tool_ok("dotnet", "--version") && clr_support::find_ilasm().is_some()),
    ];
    for (name, tool_present) in gated {
        assert!(
            !tool_present || exercised.contains(name),
            "{name}'s toolchain is installed on this host, so the {name} column must \
             have run — it did not, which means the column is silently disabled"
        );
    }
    eprintln!(
        "W16 conformance: {} programs × {} backends exercised → {:?}",
        PROGRAMS.len(),
        exercised.len(),
        exercised,
    );
}
