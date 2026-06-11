//! # McCarthy on **real CoreCLR** — scalar (CLR-real C1).
//!
//! `compile_source_to_cil_text` emits textual CIL (`.il`); the shared `clr_support`
//! harness assembles it with real `ilasm` into a loadable PE and runs it on real
//! `dotnet`, asserting the printed result — the CLR analog of `llvm_*` (clang). The
//! CLR column is verified on its **real runtime**, not only the in-repo simulator.
//! Gated on `dotnet`+`ilasm` (skips gracefully when absent).

#[path = "clr_support/mod.rs"]
mod clr_support;
use clr_support::run_on_real_clr;

#[test]
fn mccarthy_scalar_runs_on_real_coreclr() {
    let Some(v) = run_on_real_clr("42", "s42") else {
        eprintln!("dotnet/ilasm absent — skipping real-CoreCLR scalar test");
        return;
    };
    assert_eq!(v, 42, "McCarthy `42` on real CoreCLR");
    assert_eq!(run_on_real_clr("0", "s0").unwrap(), 0);
    assert_eq!(run_on_real_clr("7", "s7").unwrap(), 7);
}
