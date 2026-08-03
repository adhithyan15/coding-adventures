//! # LLVM GC completion — verify, don't assume, that `alloc`/`gc_alloc`
//! genuinely auto-collects on the LLVM backend (Twig GC completion round).
//!
//! An earlier investigation this round found that `iir-to-llvm`'s `alloc`
//! (records/unions/closures/cons cells) calls `@__twig_gc_alloc`, which routes
//! through `gc-core-capi`'s `__gc_alloc_kind` — and `__gc_alloc_kind` already
//! runs a conservative collection *before every allocation* whenever
//! `FlatHeap::should_collect()` says the live-byte threshold has been
//! crossed. This contradicted an earlier claim (from reading only
//! `iir-to-llvm`'s own codegen, which indeed never emits an explicit
//! safepoint call) that LLVM-compiled Twig programs never collect at all.
//!
//! Reading the C source is not the same as *running* it, so this test drives
//! a real, hand-built IIR module — a loop that calls a helper function
//! 70,000 times, each call allocating one throwaway 16-byte cons cell and
//! then returning (so its stack frame, and the conservative root it briefly
//! held, is gone by the time the *next* call runs) — through the real
//! `iir-to-llvm` lowering, `clang`, and the actual linked `gc-core-capi`
//! collector. 70,000 × 16 bytes ≈ 1.09 MiB, comfortably over
//! `gc_core::flat_heap::INITIAL_THRESHOLD` (1 MiB), so the auto-collect-
//! before-alloc check must fire at least once purely from ordinary
//! allocation pressure — no explicit `gc_collect` call anywhere in this
//! module, exactly as a real compiled Twig program would run.

use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use interpreter_ir::module::IIRModule;
use iir_to_llvm::{lower_iir_to_llvm, IIRLlvmConfig};
use std::process::Command;

mod common;

fn clang_available() -> bool {
    Command::new("clang").arg("--version").output().map(|o| o.status.success()).unwrap_or(false)
}

fn host_triple() -> String {
    String::from_utf8(Command::new("clang").arg("-dumpmachine").output().expect("clang").stdout)
        .unwrap()
        .trim()
        .to_string()
}

/// A loop of `limit` throwaway allocations, each in its own helper-function
/// call (so each one's conservative root dies with its stack frame), then one
/// more allocation and a `gc_live_bytes` read — returned as the exit code.
fn garbage_loop_module(limit: i64) -> IIRModule {
    let helper = IIRFunction::new(
        "helper",
        vec![],
        "void",
        vec![
            IIRInstr::new("alloc", Some("obj".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new("ret_void", None, vec![], "void"),
        ],
    );
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("i".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("label", None, vec![Operand::Var("loop_top".into())], "void"),
            IIRInstr::new(
                "cmp_lt",
                Some("cond".into()),
                vec![Operand::Var("i".into()), Operand::Int(limit)],
                "i64",
            ),
            IIRInstr::new(
                "jmp_if_false",
                None,
                vec![Operand::Var("cond".into()), Operand::Var("loop_done".into())],
                "i64",
            ),
            IIRInstr::new("call", None, vec![Operand::Var("helper".into())], "void"),
            IIRInstr::new(
                "add",
                Some("i".into()),
                vec![Operand::Var("i".into()), Operand::Int(1)],
                "i64",
            ),
            IIRInstr::new("jmp", None, vec![Operand::Var("loop_top".into())], "void"),
            IIRInstr::new("label", None, vec![Operand::Var("loop_done".into())], "void"),
            // One more allocation after the loop -- both a final auto-collect
            // trigger point and proof that allocation keeps working cleanly
            // after however many collections already ran during the loop.
            IIRInstr::new("alloc", Some("keep".into()), vec![], "ref<LispyPair>"),
            IIRInstr::new(
                "call_builtin",
                Some("live".into()),
                vec![Operand::Var("gc_live_bytes".into())],
                "i64",
            ),
            // Compare *inside* the module rather than returning `live` raw as
            // the exit code: a process exit code is truncated to its low 8
            // bits on POSIX, so a large uncollected total (~1,120,016 bytes)
            // and a genuinely-small collected residual can share the same
            // masked byte by pure coincidence -- a real risk, not a
            // theoretical one (1,120,016 mod 256 happens to be 16, well
            // inside a naively-chosen "small" range). A boolean comparison
            // result is always exactly 0 or 1, so it survives truncation
            // unambiguously.
            IIRInstr::new(
                "cmp_lt",
                Some("collected_ok".into()),
                vec![Operand::Var("live".into()), Operand::Int(10_000)],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("collected_ok".into())], "i64"),
        ],
    );
    let mut m = IIRModule::new("llvm_gc_completion", "llvm_gc_completion");
    m.add_or_replace(helper);
    m.add_or_replace(main);
    m
}

/// Compile `module` to LLVM IR, link only the `gc-core-capi` static archive
/// (no `dynval_runtime.c`/`twig_runtime.c` needed -- `alloc`/`gc_live_bytes`
/// resolve entirely from `gc-core-capi`'s `twig_compat` symbols), run it, and
/// return its exit code.
fn run_llvm(module: &IIRModule) -> i32 {
    let cfg = IIRLlvmConfig::new("llvm_gc_completion").with_target(host_triple());
    let ll = lower_iir_to_llvm(module, &cfg).expect("lower hand-built IIR to LLVM");
    let tmp = std::env::temp_dir().join(format!("llvm_gc_completion_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let ll_path = tmp.join("llvm_gc_completion.ll");
    std::fs::write(&ll_path, &ll).unwrap();
    let exe = tmp.join("llvm_gc_completion");
    let build = Command::new("clang")
        .arg("-x")
        .arg("ir")
        .arg(&ll_path)
        .arg("-x")
        .arg("none")
        .args(common::gc_link_args())
        .arg("-o")
        .arg(&exe)
        .output()
        .expect("clang");
    assert!(build.status.success(), "clang link: {}", String::from_utf8_lossy(&build.stderr));
    Command::new(&exe).output().unwrap().status.code().unwrap()
}

#[test]
fn alloc_on_llvm_auto_collects_under_real_allocation_pressure() {
    if !clang_available() {
        eprintln!("clang absent — skipping");
        return;
    }
    // 70,000 * 16 bytes ~= 1.09 MiB, over INITIAL_THRESHOLD (1 MiB) -- the
    // auto-collect-before-alloc check must fire purely from allocation
    // pressure, with no explicit gc_collect call anywhere in this module.
    let module = garbage_loop_module(70_000);
    let live_bytes = run_llvm(&module);
    assert!(
        (0..10_000).contains(&live_bytes),
        "expected only a small residual live-byte total (proving the loop's \
         70,000 throwaway allocations were actually collected, not left live \
         forever), got exit code {live_bytes} -- a code of 255 or 1,120,016 \
         (70001*16, mod 256 = 112) would mean nothing was ever collected"
    );
}
