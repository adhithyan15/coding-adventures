use clr_simulator::{CLRSimulator, Value, OP_LDC_I8, OP_NOT, OP_REM, OP_RET};

fn literal(value: Value) -> Vec<u8> {
    match value {
        Value::Int(n) => [vec![0x20], n.to_le_bytes().to_vec()].concat(),
        Value::Int64(n) => [vec![OP_LDC_I8], n.to_le_bytes().to_vec()].concat(),
        Value::Ref(_) | Value::String(_) => {
            unreachable!("reference is not an integer literal")
        }
    }
}

fn run(parts: &[Vec<u8>]) -> CLRSimulator {
    let mut simulator = CLRSimulator::new();
    simulator.load(&parts.concat(), 0);
    simulator.run(20);
    assert!(simulator.halted);
    simulator
}

#[test]
fn signed_remainder_preserves_width_and_dividend_sign() {
    for (a, b, expected) in [
        (Value::Int(17), Value::Int(5), Value::Int(2)),
        (Value::Int(-17), Value::Int(5), Value::Int(-2)),
        (Value::Int(17), Value::Int(-5), Value::Int(2)),
        (
            Value::Int64(4_294_967_297),
            Value::Int64(4_294_967_296),
            Value::Int64(1),
        ),
        (
            Value::Int64(-4_294_967_297),
            Value::Int64(4_294_967_296),
            Value::Int64(-1),
        ),
    ] {
        let simulator = run(&[literal(a), literal(b), vec![OP_REM, OP_RET]]);
        assert_eq!(simulator.stack, vec![Some(expected)]);
    }
}

#[test]
fn bitwise_not_preserves_every_bit_and_width() {
    for (value, expected) in [
        (Value::Int(0), Value::Int(-1)),
        (Value::Int(-1), Value::Int(0)),
        (Value::Int(0x5555_5555), Value::Int(-1_431_655_766)),
        (Value::Int64(0), Value::Int64(-1)),
        (Value::Int64(4_294_967_296), Value::Int64(-4_294_967_297)),
        (Value::Int64(i64::MIN), Value::Int64(i64::MAX)),
    ] {
        let simulator = run(&[literal(value), vec![OP_NOT, OP_RET]]);
        assert_eq!(simulator.stack, vec![Some(expected)]);
    }
}

#[test]
fn remainder_failures_do_not_mutate_state() {
    let cases = [
        (vec![], "rem requires"),
        (vec![Some(Value::Int(1))], "rem requires"),
        (vec![Some(Value::Int(1)), None], "rem requires"),
        (
            vec![Some(Value::Ref(None)), Some(Value::Int(1))],
            "rem requires",
        ),
        (
            vec![Some(Value::Int64(1)), Some(Value::Int(1))],
            "matching widths",
        ),
        (
            vec![Some(Value::Int(1)), Some(Value::Int(0))],
            "remainder by zero",
        ),
        (
            vec![Some(Value::Int(i32::MIN)), Some(Value::Int(-1))],
            "remainder overflow",
        ),
        (
            vec![Some(Value::Int64(i64::MIN)), Some(Value::Int64(-1))],
            "remainder overflow",
        ),
    ];
    for (stack, diagnostic) in cases {
        let mut simulator = CLRSimulator::new();
        simulator.load(&[OP_REM], 0);
        simulator.stack = stack.clone();
        let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| simulator.run(1)))
            .expect_err("invalid remainder must refuse");
        assert!(panic_text(error).contains(diagnostic));
        assert_eq!(simulator.stack, stack);
        assert_eq!(simulator.pc, 0);
    }
}

#[test]
fn not_failures_do_not_mutate_state() {
    for stack in [vec![], vec![None], vec![Some(Value::Ref(None))]] {
        let mut simulator = CLRSimulator::new();
        simulator.load(&[OP_NOT], 0);
        simulator.stack = stack.clone();
        let error = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| simulator.run(1)))
            .expect_err("invalid not must refuse");
        assert!(panic_text(error).contains("not requires"));
        assert_eq!(simulator.stack, stack);
        assert_eq!(simulator.pc, 0);
    }
}

fn panic_text(error: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = error.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = error.downcast_ref::<&str>() {
        message.to_string()
    } else {
        panic!("unexpected panic payload")
    }
}
