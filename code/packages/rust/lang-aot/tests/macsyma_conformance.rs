//! # Cross-backend Macsyma conformance suite — Wave 4 of `macsyma-iir-vm.md`.
//!
//! Macsyma's Wave 1 (`macsyma-iir-compiler` + `macsyma-vm`) proved the v0
//! integer-arithmetic/assignment/unevaluated-symbolic-`Apply` value model on the
//! VM interpreter only. This suite is the Wave 4 capstone: the **same** source,
//! compiled through five independent code generators, computes the same number.
//! Structurally this is `conformance.rs`'s McCarthy W16 capstone, retargeted —
//! `Language::McCarthyLisp` → `Language::Macsyma`, `mccarthy_lisp_vm::run` →
//! `macsyma_vm::run` — scoped to the five backends `macsyma-iir-vm.md` §6 Wave 4
//! names (NativeAOT arm64/x86_64, LLVM, WASM, JVM, CLR). BEAM and the universal
//! JIT are explicitly out of scope: JIT is McCarthy-hardcoded today
//! (`lang_aot::run_mccarthy_on_jit`, no generic `run_on_jit(language, ..)`), and
//! BEAM is scoped out repo-wide for non-McCarthy languages
//! (`code/specs/LANG-PLATFORM-MATRIX.md`).
//!
//! ## The one genuine risk this suite is built to catch
//!
//! Unlike Twig (whose literal arithmetic lowers straight to a raw `add`, never
//! touching `call_builtin`), Macsyma's lowerer **always** emits `call_builtin
//! "+"/"-"/"*"/"/"` — even for two already-concrete literal operands (see
//! `macsyma-iir-compiler/src/lower.rs`'s `combine`/`emit_builtin`). This hits
//! `iir-builtin-lowering::dynamic_arith.rs`'s "no unbox needed, box the result
//! anyway" path (`raw_operands_are_not_unboxed`). Every constituent pass is
//! independently proven (unit tests, and Twig's own dynamic-arithmetic matrix
//! cells where at least one operand IS already boxed), but the exact combination
//! here — a scalar program with zero cons/symbol ops, where `call_builtin` fires
//! over two already-concrete operands purely to produce a boxed, exit-unboxed
//! result — had never been run end-to-end on any backend before this suite.
//!
//! ## Native-AOT is cross-platform here (unlike the McCarthy W16 capstone)
//!
//! `conformance.rs`'s own `run_native` is `#[cfg(target_os = "macos")]`-only.
//! This suite instead mirrors `lang_matrix.rs`'s `compile_native` — Linux/macOS/
//! Windows all route through the matching `lang_aot::compile_file_to_*_executable`
//! — so native-AOT actually runs (and is asserted, not just attempted) on
//! whichever of the three this suite executes on, including this Windows box.

use lang_aot::{
    compile_source_to_cil_artifact, compile_source_to_iir, compile_source_to_jvm_class,
    compile_source_to_llvm_with_target, compile_source_to_wasm, Language,
};

// Shared `gc_link_args()` — see `conformance.rs`'s identical `mod common;` for why
// the LLVM column needs it (dynval_runtime.c references `__gc_alloc_kind`/
// `__gc_register_kind` from the gc-core-capi staticlib that replaced twig_gc.c).
#[path = "common/mod.rs"]
mod common;

/// A backend was present and the pipeline under test failed. Mirrors
/// `conformance.rs::backend_failed`'s policy exactly: the only legitimate reason
/// for a backend to produce no result is an absent host toolchain (probed
/// separately); anything else is a real bug and must fail loudly, not silently
/// vanish from the exercised-backend set.
fn backend_failed(backend: &str, src: &str, stage: &str, detail: impl std::fmt::Display) -> ! {
    panic!(
        "{backend}: {stage} failed for program {src:?} — this is a REAL failure, not a skip.\n\
         error:\n{detail}"
    )
}

// ── The conformance table: v0 integer arithmetic/assignment, every result an
//    unambiguous integer. Bare symbol/inert-cons results are NOT exercised here —
//    their representation differs by backend (tagged word vs. interned id vs.
//    heap pointer vs. boxed anyref/Object/object), matching McCarthy's own W16
//    "why integer-result programs" scoping; those already have per-backend VM
//    oracle coverage from Wave 1 (`macsyma-iir-compiler/tests/oracle.rs`). ──
const PROGRAMS: &[(&str, i64)] = &[
    // literals
    ("42$", 42),
    ("0$", 0),
    ("-7$", -7),
    // all 4 binary ops
    ("2 + 3$", 5),
    ("10 - 4$", 6),
    ("6 * 7$", 42),
    ("20 / 4$", 5),
    // precedence / chains
    ("2 + 3 * 4$", 14),
    ("1 + 2 + 3 + 4$", 10),
    ("(2 + 3) * 4$", 20),
    // exact division only (the `/` exactness rule — macsyma-iir-vm.md §3/§6)
    ("-4 / 2$", -2),
    ("100 / 25$", 4),
    // unary
    ("-5 + 3$", -2),
    ("-(5 + 3)$", -8),
    ("+5$", 5),
    ("-(-5)$", 5),
    // assignment + later reference
    ("x: 3$\nx + 1$", 4),
    ("x: 3$\nx: x + 1$\nx$", 4),
    ("a: 2$\nb: 3$\na * b$", 6),
    // multi-statement chains, mixed `;` and `$` terminators
    ("x: 5;\ny: 2$\nx - y$", 3),
    ("a: 1$\nb: 2$\nc: 3$\na + b + c$", 6),
];

// ── Tool-availability probes (gate the external-process backends). ──
fn tool_ok(cmd: &str, arg: &str) -> bool {
    std::process::Command::new(cmd)
        .arg(arg)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Windows has no single canonical linker probe (`link.exe` may be MSVC's
/// linker, or a git-bash/coreutils shim of the same name) — mirrors
/// `lang_matrix.rs::native_linker_ok`'s three-probe disambiguation exactly.
fn native_linker_ok() -> bool {
    if cfg!(target_os = "windows") {
        let probes: &[(&str, &str, &[&str])] = &[
            ("link.exe", "", &["Microsoft", "Linker"]),
            ("lld-link.exe", "", &["LLD"]),
            ("gcc.exe", "--version", &["gcc"]),
        ];
        probes.iter().any(|(name, arg, markers)| {
            let mut cmd = std::process::Command::new(name);
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

fn tmp_dir(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("macsyma_w4_{tag}_"))
        .tempdir()
        .expect("create a private temp dir")
}

/// Decode the scalar-result ABI used by executable backends in this suite.
///
/// A POSIX process status preserves only the low byte, so a compiled result of
/// `-7` is observed as 249. Windows may instead expose the sign-extended value
/// directly. Every executable result in `PROGRAMS` is deliberately within the
/// signed-byte range; interpreting the low byte as two's-complement makes both
/// host representations agree with the in-process backends.
fn decode_process_result(code: i32) -> i64 {
    let low_byte = u8::try_from(code.rem_euclid(256))
        .expect("an i32 remainder modulo 256 always fits in a byte");
    i64::from(i8::from_ne_bytes([low_byte]))
}

#[test]
fn executable_result_decoder_handles_unix_and_windows_statuses() {
    assert_eq!(decode_process_result(0), 0);
    assert_eq!(decode_process_result(42), 42);
    assert_eq!(decode_process_result(127), 127);
    assert_eq!(decode_process_result(249), -7); // POSIX low-byte status.
    assert_eq!(decode_process_result(-7), -7); // Sign-extended Windows status.
    assert_eq!(decode_process_result(255), -1);
}

// ── Backend 1: VM (the reference interpreter, tagged-word). Always runs. ──
fn run_vm(src: &str) -> Option<i64> {
    let module = match compile_source_to_iir(Language::Macsyma, src, "vm") {
        Ok(m) => m,
        Err(e) => backend_failed("VM", src, "source → IIR", format!("{e:?}")),
    };
    match macsyma_vm::run(&module) {
        Ok(v) => match v.as_int() {
            Some(n) => Some(n),
            None => backend_failed("VM", src, "reading the result as an int", format!("{v:?}")),
        },
        Err(e) => backend_failed("VM", src, "macsyma-vm execution", format!("{e:?}")),
    }
}

// ── Backend 2: WASM via the in-repo runtime. Always runs. ──
fn run_wasm(src: &str) -> Option<i64> {
    let bytes = match compile_source_to_wasm(Language::Macsyma, src, "main") {
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

// ── Backend 3: CLR via the in-repo simulator. Always runs. ──
fn run_clr(src: &str) -> Option<i64> {
    use clr_simulator::{CLRSimulator, MethodCode, Value};
    let artifact = match compile_source_to_cil_artifact(Language::Macsyma, src, "Main") {
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

// ── Backend 4: JVM on a real `java`. Gated on `java`. ──
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
    let mut class = match compile_source_to_jvm_class(Language::Macsyma, src, "Main") {
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

// ── Backend 5: LLVM via `clang` + the shared C runtime. Gated on `clang`. ──
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
    let ll = match compile_source_to_llvm_with_target(Language::Macsyma, src, "conf", &triple) {
        Ok(ll) => ll,
        Err(e) => backend_failed("LLVM", src, "source → LLVM IR", format!("{e:?}")),
    };
    let dir = tmp_dir("llvm");
    let ll_path = dir.path().join("conf.ll");
    std::fs::write(&ll_path, &ll).expect("llvm: write conf.ll");
    let exe = dir.path().join("conf");
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
        Some(c) => Some(decode_process_result(c)),
        None => backend_failed("LLVM", src, "reading the exit code", "process was signalled"),
    }
}

// ── Backend 6: native AOT — cross-platform (Linux/macOS/Windows), unlike the
//    McCarthy W16 capstone's macOS-only `run_native`. Mirrors
//    `lang_matrix.rs::compile_native`'s per-OS dispatch. ──
#[cfg(target_os = "linux")]
fn compile_native(src_path: &std::path::Path, exe: &std::path::Path) -> Result<(), String> {
    lang_aot::compile_file_to_linux_executable(src_path, exe, Language::Macsyma)
        .map_err(|e| format!("{e:?}"))
}
#[cfg(target_os = "macos")]
fn compile_native(src_path: &std::path::Path, exe: &std::path::Path) -> Result<(), String> {
    lang_aot::compile_file_to_macos_executable(src_path, exe, Language::Macsyma)
        .map_err(|e| format!("{e:?}"))
}
#[cfg(target_os = "windows")]
fn compile_native(src_path: &std::path::Path, exe: &std::path::Path) -> Result<(), String> {
    lang_aot::compile_file_to_windows_executable(src_path, exe, Language::Macsyma)
        .map_err(|e| format!("{e:?}"))
}
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn compile_native(_src_path: &std::path::Path, _exe: &std::path::Path) -> Result<(), String> {
    Err("no native-executable path for this host OS".to_string())
}

fn run_native(src: &str) -> Option<i64> {
    if !native_linker_ok() {
        return None;
    }
    let dir = tmp_dir("native");
    let src_path = dir.path().join("conf.mac");
    std::fs::write(&src_path, src).expect("native: write source");
    let exe = dir.path().join(if cfg!(target_os = "windows") { "conf.exe" } else { "conf" });
    if let Err(e) = compile_native(&src_path, &exe) {
        backend_failed("native-AOT", src, "source → executable", e);
    }
    let out = std::process::Command::new(&exe).output().expect("native: run linked executable");
    match out.status.code() {
        Some(c) => Some(decode_process_result(c)),
        None => backend_failed("native-AOT", src, "reading the exit code", "process was signalled"),
    }
}

/// The Wave 4 capstone: every backend that can run a Macsyma program computes
/// the same integer.
#[test]
fn macsyma_is_uniform_across_every_backend() {
    #[allow(clippy::type_complexity)]
    let backends: &[(&str, fn(&str) -> Option<i64>)] = &[
        ("VM", run_vm),
        ("WASM", run_wasm),
        ("CLR", run_clr),
        ("JVM", run_jvm),
        ("LLVM", run_llvm),
        ("native-AOT", run_native),
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

    // The three pure-in-process backends have no external dependency and MUST
    // run every program — they are the conformance floor.
    for must in ["VM", "WASM", "CLR"] {
        assert!(exercised.contains(must), "in-process backend {must} failed to run");
    }

    // Every *gated* backend must also appear once its tool is installed —
    // mirrors `conformance.rs`'s own policy, which exists precisely because a
    // backend silently vanishing from `exercised` (rather than failing loudly)
    // let a real LLVM-link regression hide for a while.
    let gated: &[(&str, bool)] = &[
        ("JVM", tool_ok("java", "-version")),
        ("LLVM", tool_ok("clang", "--version")),
        ("native-AOT", native_linker_ok()),
    ];
    for (name, tool_present) in gated {
        assert!(
            !tool_present || exercised.contains(name),
            "{name}'s toolchain is installed on this host, so the {name} column must \
             have run — it did not, which means the column is silently disabled"
        );
    }
    eprintln!(
        "Macsyma Wave 4 conformance: {} programs × {} backends exercised → {:?}",
        PROGRAMS.len(),
        exercised.len(),
        exercised,
    );
}
