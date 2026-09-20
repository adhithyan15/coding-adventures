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
fn module(ty: &str, tail: Vec<IIRInstr>) -> IIRModule {
    let mut m = IIRModule::new("typed", "test");
    m.entry_point = Some("main".into());
    let mut body = vec![
        instr("const", "a", vec![Operand::Int(2147483647)], ty),
        instr("const", "b", vec![Operand::Int(1)], ty),
        instr("add", "wide", vec![var("a"), var("b")], ty),
    ];
    body.extend(tail);
    m.add_or_replace(IIRFunction::new("main", vec![], ty, body));
    m
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
fn typed_artifacts_preserve_width_and_literal_encoding() {
    for (ty, expected, metadata, prefix) in [
        (
            "i64",
            Value::Int64(2147483648),
            "int64",
            vec![0x21, 0xff, 0xff, 0xff, 0x7f, 0, 0, 0, 0],
        ),
        (
            "i32",
            Value::Int(i32::MIN),
            "int32",
            vec![0x20, 0xff, 0xff, 0xff, 0x7f],
        ),
    ] {
        let m = module(ty, vec![instr("ret", "", vec![var("wide")], ty)]);
        let a = lower_typed_scalars_to_cil(&m, &IIRClrConfig::default()).unwrap();
        assert_eq!(a.methods[0].local_types, vec![metadata; 3]);
        assert_eq!(a.methods[0].return_type, metadata);
        assert!(a.methods[0].body.starts_with(&prefix));
        assert_eq!(execute(&m), expected);
    }
}
#[test]
fn typed_calls_transport_wide_values_and_entry_label_is_resolved() {
    let mut m = module(
        "i64",
        vec![
            instr("call", "out", vec![var("twice"), var("wide")], "i64"),
            instr("ret", "", vec![var("out")], "i64"),
        ],
    );
    m.functions.insert(
        0,
        IIRFunction::new(
            "twice",
            vec![("x".into(), "i64".into())],
            "i64",
            vec![
                instr("mov", "copy", vec![var("x")], "i64"),
                instr("add", "sum", vec![var("copy"), var("x")], "i64"),
                instr("ret", "", vec![var("sum")], "i64"),
            ],
        ),
    );
    let a = lower_typed_scalars_to_cil(&m, &IIRClrConfig::default()).unwrap();
    assert_eq!(a.methods[0].parameter_types, vec!["int64"]);
    assert!(a.methods[1]
        .body
        .windows(5)
        .any(|b| b == [0x28, 1, 0, 0, 6]));
    assert_eq!(execute(&m), Value::Int64(4294967296));
}
#[test]
fn typed_arithmetic_uses_high_bits_and_signed_division() {
    for (op, expected) in [
        ("mul", 4611686018427387904),
        ("sub", 0),
        ("and", 2147483648),
        ("or", 2147483648),
        ("xor", 0),
    ] {
        let m = module(
            "i64",
            vec![
                instr(op, "out", vec![var("wide"), var("wide")], "i64"),
                instr("ret", "", vec![var("out")], "i64"),
            ],
        );
        assert_eq!(execute(&m), Value::Int64(expected));
    }
    let m = module(
        "i64",
        vec![
            instr("neg", "negative", vec![var("wide")], "i64"),
            instr("div", "out", vec![var("negative"), var("a")], "i64"),
            instr("ret", "", vec![var("out")], "i64"),
        ],
    );
    assert_eq!(execute(&m), Value::Int64(-1));
}

#[test]
fn highest_short_local_and_argument_indices_execute() {
    let mut m = module("i64", vec![]);
    let f = &mut m.functions[0];
    f.instructions = (0..256)
        .map(|i| instr("const", &format!("v{i}"), vec![Operand::Int(i)], "i64"))
        .collect();
    m.functions[0]
        .instructions
        .push(instr("ret", "", vec![var("v255")], "i64"));
    assert_eq!(execute(&m), Value::Int64(255));
    let mut m = module("i64", vec![]);
    let mut args = vec![var("last")];
    args.extend((0..256).map(|_| var("wide")));
    m.functions[0].instructions.extend([
        instr("call", "out", args, "i64"),
        instr("ret", "", vec![var("out")], "i64"),
    ]);
    m.functions.push(IIRFunction::new(
        "last",
        (0..256).map(|i| (format!("p{i}"), "i64".into())).collect(),
        "i64",
        vec![instr("ret", "", vec![var("p255")], "i64")],
    ));
    assert_eq!(execute(&m), Value::Int64(2147483648));
}

#[test]
fn comparisons_preserve_integer_width_and_produce_normalized_booleans() {
    let comparisons: [(&str, [bool; 3], &[u8]); 6] = [
        ("cmp_eq", [false, true, false], &[0xfe, 1]),
        ("cmp_ne", [true, false, true], &[0xfe, 1, 0x16, 0xfe, 1]),
        ("cmp_lt", [true, false, false], &[0xfe, 4]),
        ("cmp_le", [true, true, false], &[0xfe, 2, 0x16, 0xfe, 1]),
        ("cmp_gt", [false, false, true], &[0xfe, 2]),
        ("cmp_ge", [false, true, true], &[0xfe, 4, 0x16, 0xfe, 1]),
    ];
    for ty in ["i32", "i64"] {
        for (op, truth, bytes) in comparisons {
            for (index, right) in [1, 0, -1].into_iter().enumerate() {
                let mut m = IIRModule::new("cmp", "test");
                m.add_or_replace(IIRFunction::new(
                    "main",
                    vec![],
                    "bool",
                    vec![
                        instr("const", "a", vec![Operand::Int(-10)], ty),
                        instr("const", "delta", vec![Operand::Int(right)], ty),
                        instr("add", "b", vec![var("a"), var("delta")], ty),
                        instr(op, "answer", vec![var("a"), var("b")], "bool"),
                        instr("ret", "", vec![var("answer")], "bool"),
                    ],
                ));
                let artifact = lower_typed_scalars_to_cil(&m, &IIRClrConfig::default()).unwrap();
                assert_eq!(artifact.methods[0].return_type, "int32");
                assert_eq!(artifact.methods[0].local_types.last().unwrap(), "int32");
                assert!(artifact.methods[0]
                    .body
                    .windows(bytes.len())
                    .any(|w| w == bytes));
                assert_eq!(
                    execute(&m),
                    Value::Int(i32::from(truth[index])),
                    "{op} {ty} {right}"
                );
            }
        }
    }
    // Compare an intermediate beyond i32 against the largest allowed literal.
    let mut m = module(
        "i64",
        vec![
            instr("cmp_gt", "answer", vec![var("wide"), var("a")], "bool"),
            instr("ret", "", vec![var("answer")], "bool"),
        ],
    );
    m.functions[0].return_type = "bool".into();
    assert_eq!(execute(&m), Value::Int(1));
}

#[test]
fn boolean_constants_moves_and_forward_calls_transport_both_truth_values() {
    for value in [false, true] {
        let mut m = IIRModule::new("bool", "test");
        m.entry_point = Some("main".into());
        m.functions.push(IIRFunction::new(
            "unused",
            vec![],
            "i32",
            vec![
                instr("const", "x", vec![Operand::Int(42)], "i32"),
                instr("ret", "", vec![var("x")], "i32"),
            ],
        ));
        m.functions.push(IIRFunction::new(
            "main",
            vec![],
            "bool",
            vec![
                instr("const", "x", vec![Operand::Bool(value)], "bool"),
                instr("call", "answer", vec![var("identity"), var("x")], "bool"),
                instr("ret", "", vec![var("answer")], "bool"),
            ],
        ));
        m.functions.push(IIRFunction::new(
            "identity",
            vec![("x".into(), "bool".into())],
            "bool",
            vec![
                instr("mov", "copy", vec![var("x")], "bool"),
                instr("ret", "", vec![var("copy")], "bool"),
            ],
        ));
        let a = lower_typed_scalars_to_cil(&m, &IIRClrConfig::default()).unwrap();
        assert_eq!(a.methods[2].parameter_types, vec!["int32"]);
        assert_eq!(a.methods[2].local_types, vec!["int32"]);
        assert_eq!(execute(&m), Value::Int(i32::from(value)));
    }
}
