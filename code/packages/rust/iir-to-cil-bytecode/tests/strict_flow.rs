use iir_to_cil_bytecode::{lower_typed_scalars_to_cil, IIRClrConfig};
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
fn v(s: &str) -> Operand {
    Operand::Var(s.into())
}
fn control(op: &str, sources: &[&str]) -> IIRInstr {
    IIRInstr::new(op, None, sources.iter().map(|s| v(s)).collect(), "void")
}
fn base() -> IIRModule {
    let mut m = IIRModule::new("flow", "test");
    m.add_or_replace(IIRFunction::new(
        "main",
        vec![("c".into(), "bool".into())],
        "i32",
        vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(7)], "i32"),
            control("jmp_if_true", &["c", "join"]),
            control("label", &["arm"]),
            control("jmp", &["join"]),
            control("label", &["join"]),
            IIRInstr::new("ret", None, vec![v("x")], "i32"),
        ],
    ));
    m
}
fn refuses(m: &IIRModule, why: &str) {
    let e = lower_typed_scalars_to_cil(m, &IIRClrConfig::default())
        .err()
        .expect("refusal");
    assert!(format!("{e:?}").contains(why), "{e:?}");
}
#[test]
fn accepts_dominating_value_at_join_for_both_conditionals() {
    for op in ["jmp_if_true", "jmp_if_false"] {
        let mut m = base();
        m.functions[0].instructions[1].op = op.into();
        assert!(lower_typed_scalars_to_cil(&m, &IIRClrConfig::default()).is_ok());
    }
}
#[test]
fn malformed_controls_and_targets_are_refused() {
    for op in ["label", "jmp", "jmp_if_true", "jmp_if_false"] {
        for shape in 0..5 {
            let mut m = base();
            let i = &mut m.functions[0].instructions[1];
            *i = control(
                op,
                if op.starts_with("jmp_if") {
                    &["c", "join"]
                } else {
                    &["join"]
                },
            );
            match shape {
                0 => i.type_hint = "bool".into(),
                1 => i.dest = Some("bad".into()),
                2 => i.srcs.clear(),
                3 => i.srcs[0] = Operand::Int(1),
                _ => i.srcs[0] = v(""),
            }
            refuses(&m, "control shape");
        }
    }
    let mut m = base();
    m.functions[0].instructions[2] = control("label", &["join"]);
    refuses(&m, "duplicate label");
    let mut m = base();
    m.functions[0].instructions[1] = control("jmp_if_true", &["c", "missing"]);
    m.functions.push(IIRFunction::new(
        "other",
        vec![],
        "i32",
        vec![control("label", &["missing"])],
    ));
    refuses(&m, "undefined label");
    let mut m = base();
    m.functions[0].instructions[3] = control("jmp", &["arm"]);
    refuses(&m, "must be forward");
    for ty in ["i32", "i64"] {
        let mut m = base();
        m.functions[0].params[0].1 = ty.into();
        refuses(&m, "condition must be bool");
    }
}
#[test]
fn skipped_assignments_do_not_leak_through_join() {
    for reader in ["ret", "jmp_if_true", "call"] {
        let mut m = base();
        m.functions[0].instructions[2] = IIRInstr::new(
            "const",
            Some("skipped".into()),
            vec![Operand::Bool(true)],
            "bool",
        );
        m.functions[0].instructions[5] = match reader {
            "jmp_if_true" => control(reader, &["skipped", "later"]),
            "call" => IIRInstr::new(
                reader,
                Some("r".into()),
                vec![v("callee"), v("skipped")],
                "i32",
            ),
            _ => IIRInstr::new(reader, None, vec![v("skipped")], "bool"),
        };
        refuses(&m, "not definitely assigned");
    }
}
#[test]
fn unreachable_and_falloff_are_refused() {
    let mut m = base();
    m.functions[0].instructions[1] = control("jmp", &["join"]);
    refuses(&m, "unreachable");
    let mut m = base();
    m.functions[0].instructions.pop();
    refuses(&m, "falls off");
    let mut m = base();
    m.functions[0].instructions[5] = control("jmp_if_false", &["c", "join"]);
    refuses(&m, "must be forward");
}
