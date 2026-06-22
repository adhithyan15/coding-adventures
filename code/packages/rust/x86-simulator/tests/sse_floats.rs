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
