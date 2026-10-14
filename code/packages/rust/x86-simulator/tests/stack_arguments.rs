//! Execute the real backend's bounded word-call ABI, without host runtime
//! imports. This is ISA execution evidence for both ABIs, not OS-native proof.

use jit_core::backend::FunctionContext;
use jit_core::cir::{CIRInstr, CIROperand as Op};
use x86_64_backend::{compile_function_with_relocs, X86_64Abi};
use x86_simulator::{harness::{MachineCodeHarness, Reloc}, Reg, StepOutcome};

fn instruction(op: &str, dest: Option<String>, srcs: Vec<Op>) -> CIRInstr {
    CIRInstr { op: op.into(), dest, srcs, ty: "u64".into(), deopt_to: None }
}
fn var(name: &str) -> Op { Op::Var(name.into()) }
fn params(count: usize) -> Vec<(String, String)> { (0..count).map(|i| (format!("p{i}"), "u64".into())).collect() }
fn compile(name: &str, params: &[(String, String)], ir: &[CIRInstr], abi: X86_64Abi) -> (Vec<u8>, Vec<Reloc>) {
    let (bytes, relocs) = compile_function_with_relocs(&FunctionContext { name, params, return_type: "u64" }, ir, abi).unwrap();
    (bytes, relocs.into_iter().map(|r| Reloc { patch_offset: r.patch_offset, symbol: r.symbol }).collect())
}
fn weighted(count: usize) -> Vec<CIRInstr> {
    let mut ir = vec![instruction("const_u64", Some("sum0".into()), vec![Op::Int(0)])];
    for i in 0..count {
        ir.push(instruction("mul_u64", Some(format!("term{i}")), vec![var(&format!("p{i}")), Op::Int((i + 1) as i64)]));
        ir.push(instruction("add_u64", Some(format!("sum{}", i + 1)), vec![var(&format!("sum{i}")), var(&format!("term{i}"))]));
    }
    ir.push(instruction("ret_u64", None, vec![var(&format!("sum{count}"))])); ir
}
fn check_order(abi: X86_64Abi, count: usize) {
    let values = [3_i64, 5, 7, 11, 13, 17, 19, 23];
    let mut caller: Vec<_> = values[..count].iter().enumerate().map(|(i, v)| instruction("const_u64", Some(format!("p{i}")), vec![Op::Int(*v)])).collect();
    let mut arguments = vec![var("weighted")]; arguments.extend((0..count).map(|i| var(&format!("p{i}"))));
    caller.push(instruction("call", Some("answer".into()), arguments));
    caller.push(instruction("ret_u64", None, vec![var("answer")]));
    let (main, mr) = compile("main", &[], &caller, abi);
    let (callee, cr) = compile("weighted", &params(count), &weighted(count), abi);
    let mut sim = MachineCodeHarness::new().function("main", &main, &mr).function("weighted", &callee, &cr).build("main").unwrap();
    let callee_entry = sim.state.rip + main.len() as u64;
    let initial_rsp = sim.state.get(Reg::Rsp); let initial_rbp = sim.state.get(Reg::Rbp);
    let (registers, shadow) = if abi == X86_64Abi::MsX64 { (4, 32) } else { (6, 0) };
    let expected: u64 = values[..count].iter().enumerate().map(|(i, v)| (i as u64 + 1) * *v as u64).sum();
    let mut caller_frame = None; let mut result = None;
    for _ in 0..2000 {
        if sim.state.rip == callee_entry {
            assert!(caller_frame.is_none(), "one nonrecursive call");
            let sp = sim.state.get(Reg::Rsp); assert_eq!(sp % 16, 8, "caller call-site alignment");
            caller_frame = Some(sim.state.get(Reg::Rbp));
            for (i, expected) in values[..count].iter().enumerate().skip(registers) {
                assert_eq!(sim.mem.load(sp + 8 + shadow + 8 * (i - registers) as u64, 8).unwrap(), *expected as u64, "incoming argument {i}");
            }
            // A Windows callee may overwrite all32 bytes of caller shadow
            // space. Neither its stack arguments nor caller locals may alias it.
            for offset in (0..shadow).step_by(8) { sim.mem.store(sp + 8 + offset, 8, 0xbad0_0000 + offset).unwrap(); }
        }
        if let StepOutcome::Halt(value) = sim.step().unwrap() { result = Some(value); break; }
    }
    assert_eq!(result, Some(expected), "full64-bit weighted result for {abi:?}/{count}");
    let frame = caller_frame.expect("callee really executed");
    for (i, v) in values[..count].iter().enumerate() {
        assert_eq!(sim.mem.load(frame - 8 * (i as u64 + 1), 8).unwrap(), *v as u64, "caller local {i} survived shadow/argument writes");
    }
    assert_eq!(sim.state.get(Reg::Rsp), initial_rsp + 8); assert_eq!(sim.state.get(Reg::Rbp), initial_rbp);
}
macro_rules! order_test { ($name:ident, $abi:ident, $n:literal) => { #[test] fn $name() { check_order(X86_64Abi::$abi, $n); } }; }
order_test!(msx64_five_arguments, MsX64, 5);
order_test!(msx64_six_arguments, MsX64, 6);
order_test!(msx64_seven_arguments, MsX64, 7);
order_test!(msx64_eight_arguments, MsX64, 8);
order_test!(sysv_five_arguments, SysV, 5);
order_test!(sysv_six_arguments, SysV, 6);
order_test!(sysv_seven_arguments, SysV, 7);
order_test!(sysv_eight_arguments, SysV, 8);

#[test]
fn nine_arguments_and_formals_refuse_both_abis() {
    for abi in [X86_64Abi::MsX64, X86_64Abi::SysV] {
        let ps = params(9);
        let ctx = FunctionContext { name: "nine", params: &ps, return_type: "u64" };
        let err = compile_function_with_relocs(&ctx, &weighted(9), abi).unwrap_err();
        assert!(err.contains("TooManyParams") && err.contains("got: 9"), "{err}");
        let mut args = vec![var("nine")]; args.extend((0..9).map(Op::Int));
        let ir = vec![instruction("call", Some("answer".into()), args), instruction("ret_u64", None, vec![var("answer")])];
        let ctx = FunctionContext { name: "main", params: &[], return_type: "u64" };
        let err = compile_function_with_relocs(&ctx, &ir, abi).unwrap_err();
        assert!(err.contains("too many arguments (9)"), "{err}");
    }
}

#[test]
fn eight_arguments_survive_terminating_self_recursion_both_abis() {
    for abi in [X86_64Abi::MsX64, X86_64Abi::SysV] {
        let mut caller: Vec<_> = [3_i64, 5, 7, 11, 13, 17, 19, 23].iter().enumerate().map(|(i, v)| instruction("const_u64", Some(format!("p{i}")), vec![Op::Int(*v)])).collect();
        let mut args = vec![var("recur")]; args.extend((0..8).map(|i| var(&format!("p{i}"))));
        caller.push(instruction("call", Some("answer".into()), args)); caller.push(instruction("ret_u64", None, vec![var("answer")]));
        let mut recursive = vec![
            instruction("cmp_eq_u64", Some("done".into()), vec![var("p0"), Op::Int(0)]),
            instruction("jmp_if_true", None, vec![var("done"), var("base")]),
            instruction("sub_u64", Some("depth".into()), vec![var("p0"), Op::Int(1)]),
        ];
        let mut args = vec![var("recur"), var("depth")]; args.extend((1..8).map(|i| var(&format!("p{i}"))));
        recursive.push(instruction("call", Some("child".into()), args));
        recursive.push(instruction("add_u64", Some("answer".into()), vec![var("child"), var("p0")]));
        recursive.push(instruction("ret_u64", None, vec![var("answer")]));
        recursive.push(instruction("label", None, vec![var("base")])); recursive.extend(weighted(8));
        let (main, mr) = compile("main", &[], &caller, abi); let (rec, rr) = compile("recur", &params(8), &recursive, abi);
        let mut sim = MachineCodeHarness::new().function("main", &main, &mr).function("recur", &rec, &rr).build("main").unwrap();
        let entry = sim.state.rip + main.len() as u64; let mut entries = 0; let mut result = None;
        for _ in 0..5000 {
            if sim.state.rip == entry { entries += 1; assert_eq!(sim.state.get(Reg::Rsp) % 16, 8); }
            if let StepOutcome::Halt(value) = sim.step().unwrap() { result = Some(value); break; }
        }
        assert_eq!(entries, 4); assert_eq!(result, Some(565));
    }
}
