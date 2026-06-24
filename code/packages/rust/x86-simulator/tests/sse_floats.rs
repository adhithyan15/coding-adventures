use jit_core::backend::FunctionContext;
use jit_core::cir::{CIRInstr, CIROperand};
use x86_64_backend::{compile_function_with_relocs, X86_64Abi};
use x86_simulator::harness::{MachineCodeHarness, Reloc};
fn run(cir: Vec<CIRInstr>) -> i32 {
    let ctx = FunctionContext { name: "main", params: &[], return_type: "u64" };
    let (bytes, relocs) = compile_function_with_relocs(&ctx, &cir, X86_64Abi::SysV).unwrap();
    let relocs: Vec<Reloc> = relocs.into_iter().map(|r| Reloc { patch_offset: r.patch_offset, symbol: r.symbol }).collect();
    MachineCodeHarness::new().function("main", &bytes, &relocs).build("main").unwrap().run().unwrap()
}
#[test]
fn f64_mul_and_compare_runs_locally() {
    let f = |op:&str,d:Option<&str>,s:Vec<CIROperand>,t:&str| CIRInstr{op:op.into(),dest:d.map(Into::into),srcs:s,ty:t.into(),deopt_to:None};
    let v=|n:&str| CIROperand::Var(n.into());
    // c = 2.5 * 2.0 (=5.0); d = (c == 5.0); ret d  → exit 1
    let cir = vec![
        f("const_f64",Some("a"),vec![CIROperand::Float(2.5)],"f64"),
        f("const_f64",Some("b"),vec![CIROperand::Float(2.0)],"f64"),
        f("mul_f64",Some("c"),vec![v("a"),v("b")],"f64"),
        f("const_f64",Some("five"),vec![CIROperand::Float(5.0)],"f64"),
        f("cmp_eq_f64",Some("d"),vec![v("c"),v("five")],"f64"),
        f("ret_u64",None,vec![v("d")],"u64"),
    ];
    assert_eq!(run(cir), 1, "2.5*2.0 == 5.0 → true; simulator runs real x86_64 SSE2 codegen");
}

#[test]
fn f64_div_runs_locally() {
    let f = |op:&str,d:Option<&str>,s:Vec<CIROperand>,t:&str| CIRInstr{op:op.into(),dest:d.map(Into::into),srcs:s,ty:t.into(),deopt_to:None};
    let v=|n:&str| CIROperand::Var(n.into());
    // (7.0 / 2.0 < 4.0) → true → exit 1
    let cir = vec![
        f("const_f64",Some("a"),vec![CIROperand::Float(7.0)],"f64"),
        f("const_f64",Some("b"),vec![CIROperand::Float(2.0)],"f64"),
        f("div_f64",Some("c"),vec![v("a"),v("b")],"f64"),
        f("const_f64",Some("four"),vec![CIROperand::Float(4.0)],"f64"),
        f("cmp_lt_f64",Some("d"),vec![v("c"),v("four")],"f64"),
        f("ret_u64",None,vec![v("d")],"u64"),
    ];
    assert_eq!(run(cir), 1, "7.0/2.0 = 3.5 < 4.0 → true");
}

// ── LANG-FULL E8: int ⇄ real conversions, end-to-end through real x86_64
// codegen (cvtsi2sd / roundsd / cvttsd2si) executed in the simulator. This is
// the x86_64 matrix cell — the same value (42) the LLVM/WASM/VM/JVM/CLR/aarch64
// cells assert.

#[test]
fn e8_floor_conversion_chain_runs_locally() {
    let f = |op:&str,d:Option<&str>,s:Vec<CIROperand>,t:&str| CIRInstr{op:op.into(),dest:d.map(Into::into),srcs:s,ty:t.into(),deopt_to:None};
    let v=|n:&str| CIROperand::Var(n.into());
    // floor(int_to_real(45) − 2.7) = floor(42.3) = 42
    let cir = vec![
        f("const_i64",Some("i"),vec![CIROperand::Int(45)],"i64"),
        f("int_to_real",Some("r"),vec![v("i")],"f64"),
        f("const_f64",Some("c"),vec![CIROperand::Float(2.7)],"f64"),
        f("sub_f64",Some("d"),vec![v("r"),v("c")],"f64"),
        f("real_to_int_floor",Some("o"),vec![v("d")],"i64"),
        f("ret_u64",None,vec![v("o")],"u64"),
    ];
    assert_eq!(run(cir), 42, "floor(45.0−2.7)=42 — cvtsi2sd→subsd→roundsd→cvttsd2si on real x86_64 codegen");
}

#[test]
fn e8_trunc_conversion_chain_runs_locally() {
    let f = |op:&str,d:Option<&str>,s:Vec<CIROperand>,t:&str| CIRInstr{op:op.into(),dest:d.map(Into::into),srcs:s,ty:t.into(),deopt_to:None};
    let v=|n:&str| CIROperand::Var(n.into());
    // trunc(int_to_real(45) − 2.7) = trunc(42.3) = 42 (toward zero drops the .3)
    let cir = vec![
        f("const_i64",Some("i"),vec![CIROperand::Int(45)],"i64"),
        f("int_to_real",Some("r"),vec![v("i")],"f64"),
        f("const_f64",Some("c"),vec![CIROperand::Float(2.7)],"f64"),
        f("sub_f64",Some("d"),vec![v("r"),v("c")],"f64"),
        f("real_to_int_trunc",Some("o"),vec![v("d")],"i64"),
        f("ret_u64",None,vec![v("o")],"u64"),
    ];
    assert_eq!(run(cir), 42, "trunc(45.0−2.7)=42 — cvtsi2sd→subsd→cvttsd2si on real x86_64 codegen");
}
