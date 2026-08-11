//! # LLVM generational write barrier — prove `field_store` keeps an old→young
//! edge alive across a real minor collection (AOT00-T8 follow-up).
//!
//! `iir-to-llvm`'s `field_store` now emits an unconditional call to
//! `@__twig_gc_write_barrier` alongside the store itself (see `lower_field_store`
//! in `iir-to-llvm/src/lib.rs`). This test drives a real, hand-built IIR module —
//! compiled, linked, and *run* through `clang` and the actual `gc-core-capi`
//! collector, exactly like `llvm_gc_completion.rs` — through the exact scenario
//! the barrier exists for:
//!
//! 0. Attest via `gc_set_auto_minor(1)` — `gc_collect_minor_precise` is a safe
//!    no-op without this (a later security-review hardening of this same
//!    follow-up); this module's own `field_store` below is what makes the
//!    attestation genuinely true.
//! 1. Allocate `parent` and run one minor collection with it rooted on the
//!    stack. It survives, and — `gc_core::flat_heap::DEFAULT_TENURE_AGE` is `1`
//!    — tenures to **old** on that very first survived cycle.
//! 2. Allocate `child` (a **separate, later** allocation, so it starts this
//!    step genuinely young — never itself directly rooted during the first
//!    collection above).
//! 3. `field_store parent, 0, child` — the compiled store this PR's barrier
//!    call rides along with. `parent` is old, so the barrier records it in the
//!    remembered set.
//! 4. Run a **second** minor collection. `child` is reachable *only* through
//!    `parent`'s field now — `parent` itself, being old, is never itself
//!    scanned as a minor-cycle root, only rescanned via the remembered set the
//!    barrier populated. If the barrier fired, `child` survives; if it didn't
//!    (this PR's fix reverted), `child` is unreachable from any minor-cycle
//!    root and is swept as garbage.
//!
//! `gc_live_bytes()` is a fully deterministic survival signal here — unlike
//! `llvm_gc_completion.rs`'s garbage-loop scenario (documented there, and in
//! `array_ref_tracing.rs`, as unreliable under incidental conservative-stack
//! noise from many allocations), this module allocates *exactly* two objects,
//! total, ever. 16 live bytes afterward means only `parent` survived (barrier
//! broken); 32 means both did (barrier working) — no other object exists that
//! could make either reading ambiguous.

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

/// The scenario described in the module doc comment, as a single `main`. Both
/// `alloc`s use the default (kind-0, conservative) layout — `iir-to-llvm`'s
/// `alloc` op has no way to request a registered kind today — which is exactly
/// why `minor_finish`'s remembered-parent rescan must (and does, per
/// `FlatHeap::scan_payload`'s conservative fallback) scan every payload word of
/// an unregistered-kind old parent, not just declared reference fields.
fn write_barrier_module() -> IIRModule {
    let main = IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            // Attest before anything else: `gc_collect_minor_precise` is a safe
            // no-op without this (security-review finding -- see that builtin's
            // own doc). This module's own field_store is what makes the
            // attestation genuinely true, not just asserted.
            IIRInstr::new(
                "call_builtin",
                None,
                vec![Operand::Var("gc_set_auto_minor".into()), Operand::Int(1)],
                "void",
            ),
            IIRInstr::new("alloc", Some("parent".into()), vec![], "ref<LispyPair>"),
            // First minor collection: `parent` is referenced again below, so it
            // must be kept live (and therefore stack-discoverable) across this
            // call by any correct codegen. It survives and tenures to old.
            IIRInstr::new(
                "call_builtin",
                Some("freed1".into()),
                vec![Operand::Var("gc_collect_minor_precise".into())],
                "i64",
            ),
            // `child` is allocated only now -- it does not exist during the
            // first collection above, so it cannot be accidentally rooted (and
            // tenured) by it.
            IIRInstr::new("alloc", Some("child".into()), vec![], "ref<LispyPair>"),
            // The store this PR's barrier rides along with. `parent` (old) is
            // the barrier's `parent` argument; `child` (young) is its `value`.
            IIRInstr::new(
                "field_store",
                None,
                vec![Operand::Var("parent".into()), Operand::Int(0), Operand::Var("child".into())],
                "void",
            ),
            // Second minor collection: `child` is reachable only via `parent`'s
            // field now. Neither `parent` nor `child` is referenced by any Rust-
            // level local past this point, so nothing else roots either of them
            // going into this call -- `parent` survives regardless (old objects
            // are never swept by a minor cycle), and `child`'s survival is
            // exactly what this test is checking.
            IIRInstr::new(
                "call_builtin",
                Some("freed2".into()),
                vec![Operand::Var("gc_collect_minor_precise".into())],
                "i64",
            ),
            IIRInstr::new(
                "call_builtin",
                Some("live".into()),
                vec![Operand::Var("gc_live_bytes".into())],
                "i64",
            ),
            // 32 iff both `parent` (16 bytes) and `child` (16 bytes) are still
            // live -- the barrier kept the old->young edge alive. 16 would mean
            // `child` was wrongly swept.
            IIRInstr::new(
                "cmp_eq",
                Some("child_survived".into()),
                vec![Operand::Var("live".into()), Operand::Int(32)],
                "i64",
            ),
            IIRInstr::new("ret", None, vec![Operand::Var("child_survived".into())], "i64"),
        ],
    );
    let mut m = IIRModule::new("llvm_gc_write_barrier", "llvm_gc_write_barrier");
    m.add_or_replace(main);
    m
}

/// Compile `module` to LLVM IR, link only the `gc-core-capi` static archive (as
/// `llvm_gc_completion.rs` does -- `alloc`/`field_store`/`gc_collect_minor_precise`/
/// `gc_live_bytes` all resolve from `gc-core-capi`'s `twig_compat` symbols alone),
/// run it, and return its exit code.
fn run_llvm(module: &IIRModule) -> i32 {
    let cfg = IIRLlvmConfig::new("llvm_gc_write_barrier").with_target(host_triple());
    let ll = lower_iir_to_llvm(module, &cfg).expect("lower hand-built IIR to LLVM");
    let tmp = std::env::temp_dir().join(format!("llvm_gc_write_barrier_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let ll_path = tmp.join("llvm_gc_write_barrier.ll");
    std::fs::write(&ll_path, &ll).unwrap();
    let exe = tmp.join("llvm_gc_write_barrier");
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
fn field_store_write_barrier_keeps_young_child_alive_across_a_real_minor_collection() {
    if !clang_available() {
        eprintln!("clang absent — skipping");
        return;
    }
    let module = write_barrier_module();
    let child_survived = run_llvm(&module);
    assert_eq!(
        child_survived, 1,
        "expected the write barrier to keep `child` reachable via `parent`'s \
         remembered-set edge across the second minor collection \
         (gc_live_bytes() == 32 inside the module -- see the module doc comment \
         for why this is unambiguous here), got exit code {child_survived} -- \
         0 means `child` was wrongly swept as garbage"
    );
}
