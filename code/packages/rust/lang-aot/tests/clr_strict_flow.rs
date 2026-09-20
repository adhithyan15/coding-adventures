use clr_simulator::{CLRSimulator, MethodCode, Value};
use iir_to_cil_bytecode::{lower_typed_scalars_to_cil, IIRClrConfig};
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
fn var(s: &str) -> Operand {
    Operand::Var(s.into())
}
fn instr(op: &str, dest: &str, srcs: Vec<Operand>, ty: &str) -> IIRInstr {
    IIRInstr::new(
        op,
        if dest.is_empty() {
            None
        } else {
            Some(dest.into())
        },
        srcs,
        ty,
    )
}
fn execute(m: &IIRModule) -> Value {
    let a = lower_typed_scalars_to_cil(m, &IIRClrConfig::default()).unwrap();
    let entry = a
        .methods
        .iter()
        .position(|f| f.name == a.entry_label)
        .unwrap();
    let mut sim = CLRSimulator::new();
    sim.load_program(
        a.methods
            .iter()
            .map(|f| MethodCode {
                body: f.body.clone(),
                num_locals: f.local_types.len(),
                num_args: f.parameter_types.len(),
            })
            .collect(),
        entry,
    );
    sim.run(10000);
    assert!(sim.halted);
    assert_eq!(sim.stack.len(), 1);
    sim.stack[0].unwrap()
}

#[test]
fn both_conditional_kinds_execute_both_arms_and_promote_long_branches() {
    for op in ["jmp_if_true", "jmp_if_false"] {
        for condition in [false, true] {
            let mut m = IIRModule::new("flow", "test");
            m.entry_point = Some("main".into());
            let mut body = vec![
                instr("const", "c", vec![Operand::Bool(condition)], "bool"),
                instr(op, "", vec![var("c"), var("taken")], "void"),
            ];
            for i in 0..80 {
                body.push(instr(
                    "const",
                    &format!("pad{i}"),
                    vec![Operand::Int(17)],
                    "i32",
                ));
            }
            body.push(instr("ret", "", vec![var("pad79")], "i32"));
            body.push(instr("label", "", vec![var("taken")], "void"));
            body.push(instr("const", "yes", vec![Operand::Int(-1)], "i32"));
            body.push(instr("ret", "", vec![var("yes")], "i32"));
            m.add_or_replace(IIRFunction::new("main", vec![], "i32", body));
            let a = lower_typed_scalars_to_cil(&m, &IIRClrConfig::default()).unwrap();
            let bytes = &a.methods[0].body;
            // bool constant, stloc.0, ldloc.0 then the promoted branch.
            assert_eq!(bytes[3], if op == "jmp_if_true" { 0x3a } else { 0x39 });
            assert!(i32::from_le_bytes(bytes[4..8].try_into().unwrap()) > 127);
            let taken = condition == (op == "jmp_if_true");
            assert_eq!(execute(&m), Value::Int(if taken { -1 } else { 17 }));
        }
    }
}
#[test]
fn bool_parameter_nested_branches_and_wide_dominating_join_execute() {
    for condition in [false, true] {
        let mut m = IIRModule::new("flow", "test");
        m.entry_point = Some("main".into());
        m.add_or_replace(IIRFunction::new(
            "choose",
            vec![("c".into(), "bool".into())],
            "i64",
            vec![
                instr("const", "a", vec![Operand::Int(i32::MAX as i64)], "i64"),
                instr("const", "one", vec![Operand::Int(1)], "i64"),
                instr("add", "wide", vec![var("a"), var("one")], "i64"),
                instr("cmp_gt", "positive", vec![var("wide"), var("one")], "bool"),
                instr("jmp_if_false", "", vec![var("c"), var("join")], "void"),
                instr(
                    "jmp_if_true",
                    "",
                    vec![var("positive"), var("inner")],
                    "void",
                ),
                instr("ret", "", vec![var("one")], "i64"),
                instr("label", "", vec![var("inner")], "void"),
                instr("jmp", "", vec![var("join")], "void"),
                instr("label", "", vec![var("join")], "void"),
                instr("ret", "", vec![var("wide")], "i64"),
            ],
        ));
        m.add_or_replace(IIRFunction::new(
            "main",
            vec![],
            "i64",
            vec![
                instr("const", "condition", vec![Operand::Bool(condition)], "bool"),
                instr(
                    "call",
                    "result",
                    vec![var("choose"), var("condition")],
                    "i64",
                ),
                instr("ret", "", vec![var("result")], "i64"),
            ],
        ));
        assert_eq!(execute(&m), Value::Int64(2147483648));
    }
}
#[test]
fn encoded_api_refuses_a_textually_earlier_skipped_definition() {
    let mut m = IIRModule::new("flow", "test");
    m.add_or_replace(IIRFunction::new(
        "main",
        vec![],
        "i32",
        vec![
            instr("const", "c", vec![Operand::Bool(false)], "bool"),
            instr("jmp_if_false", "", vec![var("c"), var("join")], "void"),
            instr("const", "skipped", vec![Operand::Int(42)], "i32"),
            instr("label", "", vec![var("join")], "void"),
            instr("ret", "", vec![var("skipped")], "i32"),
        ],
    ));
    assert!(lower_typed_scalars_to_cil(&m, &IIRClrConfig::default()).is_err());
}
