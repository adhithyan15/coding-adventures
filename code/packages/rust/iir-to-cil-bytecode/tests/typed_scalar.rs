use iir_to_cil_bytecode::{lower_typed_scalars_to_cil, IIRClrConfig};
use interpreter_ir::{IIRFunction, IIRInstr, IIRModule, Operand};
fn v(s: &str) -> Operand {
    Operand::Var(s.into())
}
fn base() -> IIRModule {
    let mut m = IIRModule::new("strict", "test");
    m.entry_point = Some("main".into());
    m.add_or_replace(IIRFunction::new(
        "main",
        vec![],
        "i64",
        vec![
            IIRInstr::new("const", Some("x".into()), vec![Operand::Int(1)], "i64"),
            IIRInstr::new("ret", None, vec![v("x")], "i64"),
        ],
    ));
    m
}
fn refuses(m: IIRModule, diagnostic: &str) {
    let error = lower_typed_scalars_to_cil(&m, &IIRClrConfig::default())
        .err()
        .expect("must refuse");
    assert!(format!("{error:?}").contains(diagnostic), "{error:?}");
}
#[test]
fn excluded_operations_and_types_never_fall_back() {
    for op in [
        "br",
        "label",
        "eq",
        "lt",
        "shl",
        "shr",
        "mod",
        "alloc",
        "load",
        "store",
        "make_closure",
        "call_closure",
        "input_i64",
        "input_str",
        "input_more",
        "cast",
        "print",
    ] {
        let mut m = base();
        m.functions[0].instructions[0].op = op.into();
        refuses(m, "UnsupportedOp");
    }
    for ty in [
        "any",
        "polymorphic",
        "void",
        "f64",
        "str",
        "object",
        "nativeint",
    ] {
        let mut m = base();
        m.functions[0].instructions[0].type_hint = ty.into();
        refuses(m.clone(), "UnsupportedType");
        m = base();
        m.functions[0].return_type = ty.into();
        refuses(m, "UnsupportedType");
        m = base();
        m.functions[0].params.push(("p".into(), ty.into()));
        refuses(m, "UnsupportedType");
    }
}
#[test]
fn malformed_definitions_returns_and_literals_are_refused() {
    type Mutation = (fn(&mut IIRModule), &'static str);
    let mutations: Vec<Mutation> = vec![
        (|m| m.functions[0].name.clear(), "function name"),
        (
            |m| m.functions.push(m.functions[0].clone()),
            "function name",
        ),
        (|m| m.entry_point = Some("missing".into()), "entry point"),
        (
            |m| m.functions[0].params = vec![("".into(), "i64".into())],
            "parameter",
        ),
        (
            |m| m.functions[0].params = vec![("p".into(), "i64".into()); 2],
            "parameter",
        ),
        (
            |m| m.functions[0].params = vec![("x".into(), "i64".into())],
            "destination",
        ),
        (
            |m| m.functions[0].instructions[0].dest = None,
            "destination",
        ),
        (
            |m| m.functions[0].instructions[0].dest = Some("".into()),
            "destination",
        ),
        (
            |m| {
                let i = m.functions[0].instructions[0].clone();
                m.functions[0].instructions.insert(1, i);
            },
            "destination",
        ),
        (
            |m| m.functions[0].instructions[0].srcs.clear(),
            "operand count",
        ),
        (
            |m| m.functions[0].instructions[0].srcs.push(Operand::Int(2)),
            "operand count",
        ),
        (
            |m| m.functions[0].instructions[0].srcs = vec![v("x")],
            "integer literal",
        ),
        (
            |m| m.functions[0].instructions[1].srcs = vec![Operand::Int(1)],
            "variable operand",
        ),
        (
            |m| m.functions[0].instructions[1].srcs = vec![v("later")],
            "forward variable",
        ),
        (
            |m| m.functions[0].instructions[0].type_hint = "i32".into(),
            "width mismatch",
        ),
        (
            |m| m.functions[0].instructions[1].type_hint = "i32".into(),
            "return shape",
        ),
        (
            |m| m.functions[0].instructions[1].dest = Some("r".into()),
            "return shape",
        ),
        (
            |m| {
                let i = m.functions[0].instructions[1].clone();
                m.functions[0].instructions.push(i);
            },
            "return shape",
        ),
        (
            |m| {
                m.functions[0].instructions.pop();
            },
            "end with ret",
        ),
    ];
    for (change, diagnostic) in mutations {
        let mut m = base();
        change(&mut m);
        refuses(m, diagnostic);
    }
    for n in [
        i64::MIN,
        i64::MAX,
        i64::from(i32::MIN) - 1,
        i64::from(i32::MAX) + 1,
    ] {
        let mut m = base();
        m.functions[0].instructions[0].srcs = vec![Operand::Int(n)];
        refuses(m, "CLR01 range");
    }
}
#[test]
fn direct_calls_check_callee_arity_argument_and_result_width() {
    for (srcs, hint, diagnostic) in [
        (vec![], "i64", "missing direct callee"),
        (vec![Operand::Int(1)], "i64", "missing direct callee"),
        (vec![v("absent")], "i64", "undefined direct callee"),
        (vec![v("callee")], "i64", "call signature"),
        (vec![v("callee"), v("x"), v("x")], "i64", "call signature"),
        (vec![v("callee"), v("x")], "i32", "call signature"),
        (
            vec![v("callee"), Operand::Int(1)],
            "i64",
            "variable operand",
        ),
        (vec![v("callee"), v("missing")], "i64", "forward variable"),
    ] {
        let mut m = base();
        m.functions.push(IIRFunction::new(
            "callee",
            vec![("p".into(), "i64".into())],
            "i64",
            vec![IIRInstr::new("ret", None, vec![v("p")], "i64")],
        ));
        m.functions[0]
            .instructions
            .insert(1, IIRInstr::new("call", Some("out".into()), srcs, hint));
        refuses(m, diagnostic);
    }
    let mut m = base();
    m.functions[0].instructions.insert(
        1,
        IIRInstr::new("mov", Some("out".into()), vec![v("x")], "i32"),
    );
    refuses(m, "width mismatch");
}
#[test]
fn indices_refuse_overflow_without_wrapping() {
    let mut m = base();
    m.functions[0].params = (0..257).map(|i| (format!("p{i}"), "i64".into())).collect();
    refuses(m, "too many parameters");
    let mut m = base();
    m.functions[0].instructions = (0..257)
        .map(|i| IIRInstr::new("const", Some(format!("v{i}")), vec![Operand::Int(0)], "i64"))
        .collect();
    m.functions[0]
        .instructions
        .push(IIRInstr::new("ret", None, vec![v("v256")], "i64"));
    refuses(m, "too many locals");
}

#[test]
fn all_callee_bodies_and_argument_widths_are_validated() {
    let mut m = base();
    m.functions.push(IIRFunction::new(
        "callee",
        vec![("p".into(), "i32".into())],
        "i64",
        vec![
            IIRInstr::new("const", Some("r".into()), vec![Operand::Int(0)], "i64"),
            IIRInstr::new("ret", None, vec![v("r")], "i64"),
        ],
    ));
    m.functions[0].instructions.insert(
        1,
        IIRInstr::new("call", Some("out".into()), vec![v("callee"), v("x")], "i64"),
    );
    refuses(m.clone(), "width mismatch");
    m.functions[0].instructions.remove(1);
    m.functions[1].instructions[0].op = "input_i64".into();
    refuses(m, "UnsupportedOp");
}

#[test]
fn comparisons_validate_logical_types_and_shapes() {
    for op in ["cmp_eq", "cmp_ne", "cmp_lt", "cmp_le", "cmp_gt", "cmp_ge"] {
        for (left, right, hint, diagnostic) in [
            ("i32", "i64", "bool", "width mismatch"),
            ("i64", "i32", "bool", "width mismatch"),
            ("bool", "bool", "bool", "integer operands"),
            ("i64", "i64", "i32", "result must be bool"),
            ("i64", "i64", "i64", "result must be bool"),
        ] {
            let mut m = base();
            m.functions[0].params = vec![("a".into(), left.into()), ("b".into(), right.into())];
            m.functions[0].instructions.insert(
                1,
                IIRInstr::new(op, Some("c".into()), vec![v("a"), v("b")], hint),
            );
            refuses(m, diagnostic);
        }
        for srcs in [vec![], vec![v("x")], vec![v("x"), v("x"), v("x")]] {
            let mut m = base();
            m.functions[0]
                .instructions
                .insert(1, IIRInstr::new(op, Some("c".into()), srcs, "bool"));
            refuses(m, "operand count");
        }
        for srcs in [vec![Operand::Int(1), v("x")], vec![v("x"), Operand::Int(1)]] {
            let mut m = base();
            m.functions[0]
                .instructions
                .insert(1, IIRInstr::new(op, Some("c".into()), srcs, "bool"));
            refuses(m, "variable operand");
        }
    }
}

#[test]
fn bool_is_not_an_alias_for_integer_arithmetic_or_transport() {
    for op in ["add", "sub", "mul", "div", "and", "or", "xor", "neg"] {
        let mut m = base();
        m.functions[0].params = vec![("b".into(), "bool".into())];
        let srcs = if op == "neg" {
            vec![v("b")]
        } else {
            vec![v("b"), v("b")]
        };
        m.functions[0]
            .instructions
            .insert(1, IIRInstr::new(op, Some("c".into()), srcs, "bool"));
        refuses(m, "boolean arithmetic");
    }
    for (source, target) in [
        ("bool", "i32"),
        ("i32", "bool"),
        ("bool", "i64"),
        ("i64", "bool"),
    ] {
        let mut m = base();
        m.functions[0].params = vec![("p".into(), source.into())];
        m.functions[0].instructions.insert(
            1,
            IIRInstr::new("mov", Some("c".into()), vec![v("p")], target),
        );
        refuses(m.clone(), "width mismatch");
        m.functions[0].instructions.remove(1);
        m.functions[0].return_type = target.into();
        m.functions[0].instructions[1] = IIRInstr::new("ret", None, vec![v("p")], target);
        refuses(m.clone(), "width mismatch");
        m.functions.push(IIRFunction::new(
            "callee",
            vec![("q".into(), target.into())],
            target,
            vec![IIRInstr::new("ret", None, vec![v("q")], target)],
        ));
        m.functions[0].instructions.insert(
            1,
            IIRInstr::new("call", Some("c".into()), vec![v("callee"), v("p")], target),
        );
        refuses(m, "width mismatch");
    }
    let mut m = base();
    m.functions[0].instructions[0].type_hint = "bool".into();
    refuses(m, "boolean literal");
    let mut m = base();
    m.functions[0].instructions[0].srcs = vec![Operand::Bool(true)];
    refuses(m, "integer literal");
}
