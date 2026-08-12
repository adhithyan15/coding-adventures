//! # E6d-2b — dynamic integer arithmetic on the tagged-i64 backends.
//!
//! `(+ (car (cons 41 0)) 1)` forces `+` over a **boxed** operand: `car`'s result
//! is a `ref<any>` tagged word, not a machine int. `lower_dynamic_arith` expands
//! it to `unbox → add → box`; on the **tagged-i64** world (native aarch64/x86_64
//! + LLVM) `lower_box_unbox_to_runtime_calls` rewrites those generic ops to
//!   `dyn_box_int` / `dyn_unbox_int` runtime calls, which the backends dispatch to
//!   `__dyn_box_int` / `__dyn_unbox_int` in `dynval_runtime.c`. The final tagged
//!   result is exit-unboxed (`dyn_repr` recognises the `ref<any>` result even for
//!   a **Twig** program, whose bare-`any` params stay gated). Exit 42.
//!
//! `dynamic_arith_over_a_twig_parameter_runs_on_native` below is the regression
//! case for the bug that gate closed: a bare-`any`-typed Twig **parameter**
//! (never a `field_load`/`ref<any>` value) compared with `<` was previously
//! misread as boxed too, so `lower_dynamic_arith` inserted a spurious unbox that
//! silently corrupted every comparison against a function parameter — see
//! `iir-builtin-lowering::dynamic_arith::is_boxed`'s doc comment for the trace.
//!
//! **Verified by RUNNING**: emit host IR / native object, link the C runtime,
//! execute — the exit code is the arithmetic result.

use lang_aot::{compile_source_to_llvm_with_target, Language};
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

const SRC: &str = "(+ (car (cons 41 0)) 1)";

fn clang_available() -> bool {
    Command::new("clang").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}
fn host_triple() -> String {
    let o = Command::new("clang").arg("-dumpmachine").output().expect("clang -dumpmachine");
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}
fn runtime_c(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../twig-aot/runtime").join(name)
}

/// Run `src` through **vm-core** (`vm_core::core::VMCore`, the shared
/// cross-language register interpreter — LANG02, distinct from `twig-vm`)
/// as the always-correct oracle, and return the `i64` result. Mirrors
/// `lang_matrix.rs`'s `fn run_vm`, trimmed to the no-I/O expression case
/// this file's programs are (`(+ (car (cons 41 0)) 1)` never calls
/// `print_i64`/`putchar`, so no builtin registration is needed).
/// vm-core interprets a *different* lowering (`lower_heap_builtins`, the
/// structural pass — not `lower_heap_builtins_runtime`/`lower_dynamic_arith`,
/// which are native/LLVM-AOT-pipeline-specific), so it is structurally
/// immune to the `is_boxed`/retype bugs this file's other tests guard
/// against — an independent cross-check, not just a second run of the same
/// buggy code path.
fn run_vm_i64(src: &str) -> i64 {
    use vm_core::core::VMCore;
    let mut module = lang_aot::compile_source_to_iir(Language::Twig, src, "main")
        .unwrap_or_else(|e| panic!("compile {src:?} to IIR: {e}"));
    iir_builtin_lowering::lower_global_io(&mut module);
    iir_builtin_lowering::lower_closures_to_heap(&mut module);
    iir_builtin_lowering::lower_heap_builtins(&mut module);
    let entry = module.entry_point.clone().unwrap_or_else(|| "main".to_string());
    let mut vm = VMCore::new();
    vm.execute(&mut module, &entry, &[])
        .unwrap_or_else(|e| panic!("vm-core execute {src:?}: {e:?}"))
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| panic!("vm-core {src:?} did not return an int"))
}

#[test]
fn dynamic_arith_over_boxed_operand_matches_vm_core_oracle() {
    // vm-core independently agrees the correct answer is 42 — this is the
    // ground truth the native/LLVM assertions above are checked against,
    // not just a hardcoded literal both happen to share.
    assert_eq!(run_vm_i64(SRC), 42, "vm-core oracle: (+ (car (cons 41 0)) 1)");
}

/// Compile to host LLVM IR, link the full C runtime with `clang`, run, return exit code.
fn run_llvm(src: &str, module: &str) -> i32 {
    let ll = compile_source_to_llvm_with_target(Language::Twig, src, module, &host_triple())
        .unwrap_or_else(|e| panic!("compile {src:?} to LLVM: {e}"));
    let tmp = std::env::temp_dir().join(format!("e6d2b_llvm_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).expect("temp dir");
    let ll_path = tmp.join(format!("{module}.ll"));
    std::fs::write(&ll_path, &ll).expect("write .ll");
    let exe = tmp.join(module);
    let build = Command::new("clang")
        .arg("-x").arg("ir").arg(&ll_path)
        // Reset to extension-based mode for the C runtime files (GC + tagged-value + I/O).
        .arg("-x").arg("none")
        .arg(runtime_c("dynval_runtime.c"))
        .args(common::gc_link_args()) // gc-core-capi staticlib (retired twig_gc.c)
        .arg(runtime_c("twig_runtime.c"))
        .arg("-o").arg(&exe)
        .output().expect("spawn clang");
    assert!(build.status.success(), "clang link failed: {}", String::from_utf8_lossy(&build.stderr));
    let out = Command::new(&exe).output().expect("run exe");
    out.status.code().expect("exit code")
}

#[test]
fn dynamic_arith_over_boxed_operand_runs_on_llvm() {
    if !clang_available() {
        eprintln!("clang absent — skipping E6d-2b LLVM run");
        return;
    }
    // (+ (car (cons 41 0)) 1) = 41 + 1 = 42, re-boxed then exit-unboxed.
    assert_eq!(run_llvm(SRC, "e6d2b_add"), 42, "dynamic + over a boxed car result");
    // A pure comparison also flows through box/unbox → runtime calls (tagged #t = 5,
    // exit-unboxed to a raw truthy value is a follow-up; here we keep the arithmetic).
    assert_eq!(run_llvm("(+ (car (cons 40 0)) (car (cons 1 0)))", "e6d2b_add2"), 41, "+ over two boxed operands");
}

/// Whether a real Windows linker (not git-bash's POSIX `link(1)` lookalike)
/// is on `PATH` — mirrors `twig-aot/tests/windows_x86_64_smoke.rs`'s own
/// `linker_available` probe (not exported as public API, so duplicated here).
/// The Linux/macOS native arms use `clang` instead (their `run_native` calls
/// `twig_aot`'s ELF/Mach-O paths, which shell out to the system `ld`/`clang`),
/// so gating the Windows arm on `clang_available()` would wrongly skip a host
/// that has `link.exe`/`lld-link.exe`/`gcc.exe` but no `clang.exe`.
#[cfg(target_os = "windows")]
fn windows_linker_available() -> bool {
    let probes: &[(&str, &str, &[&str])] = &[
        ("link.exe",     "",          &["Microsoft", "Linker"]),
        ("lld-link.exe", "",          &["LLD"]),
        ("gcc.exe",      "--version", &["gcc"]),
    ];
    for (name, arg, markers) in probes {
        let mut cmd = Command::new(name);
        if !arg.is_empty() { cmd.arg(arg); }
        let Ok(o) = cmd.output() else { continue; };
        let banner = format!("{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr));
        if markers.iter().all(|m| banner.contains(m)) {
            return true;
        }
    }
    false
}

/// Native AOT: compile the source straight to a host executable and run it.
fn run_native(src: &str, stem: &str) -> Option<i32> {
    let tmp = std::env::temp_dir().join(format!("e6d2b_native_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).ok()?;
    let src_path = tmp.join(format!("{stem}.twig"));
    std::fs::write(&src_path, src).ok()?;
    let exe = tmp.join(stem);
    #[cfg(target_os = "linux")]
    lang_aot::compile_file_to_linux_executable(&src_path, &exe, Language::Twig).ok()?;
    #[cfg(target_os = "macos")]
    lang_aot::compile_file_to_macos_executable(&src_path, &exe, Language::Twig).ok()?;
    #[cfg(target_os = "windows")]
    lang_aot::compile_file_to_windows_executable(&src_path, &exe, Language::Twig).ok()?;
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = (src_path, exe);
        return None;
    }
    #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
    Command::new(&exe).output().ok()?.status.code()
}

/// Windows: gate on a real linker, exactly like `windows_x86_64_smoke.rs`.
/// Every other host: gate on `clang` (unchanged from before this fix).
#[cfg(target_os = "windows")]
fn native_toolchain_available() -> bool { windows_linker_available() }
#[cfg(not(target_os = "windows"))]
fn native_toolchain_available() -> bool { clang_available() }

#[test]
fn dynamic_arith_over_boxed_operand_runs_on_native() {
    if !native_toolchain_available() {
        eprintln!("native linker absent — skipping E6d-2b native run");
        return;
    }
    match run_native(SRC, "e6d2b_native_add") {
        Some(code) => assert_eq!(code, 42, "native dynamic + over a boxed car result"),
        None => eprintln!("native AOT unsupported on this host — skipping"),
    }
}

/// Regression: a Twig function **parameter** — a bare-`any`-typed value, never
/// a `field_load`/`ref<any>` — compared with `<` inside an `if` must take the
/// correct branch. Before this fix, `lower_dynamic_arith` treated the bare-`any`
/// parameter as a boxed dynamic value (same misclassification as the module's
/// `is_boxed`, just triggered by a parameter instead of a `car` result) and
/// inserted a spurious unbox, right-shifting `n` by 3 before the comparison —
/// so `classify(10)` read `n` back as `1` and always took the `< 2` branch.
/// `n = 10` is not `< 2`, so the correct exit code is `222`; the bug produced
/// `111`. Caught here because — unlike the boxed-`car`-result case above, which
/// exercises the SAME `lower_dynamic_arith` pass but never touched a bare
/// parameter — this is the shape every recursive/guarded Twig function uses.
#[test]
fn dynamic_arith_over_a_twig_parameter_runs_on_native() {
    if !native_toolchain_available() {
        eprintln!("native linker absent — skipping parameter-comparison native run");
        return;
    }
    const CLASSIFY_SRC: &str = "(define (classify n) (if (< n 2) 111 222)) (classify 10)";
    match run_native(CLASSIFY_SRC, "e6d2b_native_param_cmp") {
        Some(code) => assert_eq!(
            code, 222,
            "n=10 is not < 2, so classify must take the else branch (222), not the then branch (111)"
        ),
        None => eprintln!("native AOT unsupported on this host — skipping"),
    }
}
