use clr_simulator::{CLRSimulator, Value};
fn literal(value: Value) -> Vec<u8> {
    match value {
        Value::Int(n) => [vec![0x20], n.to_le_bytes().to_vec()].concat(),
        Value::Int64(n) => [vec![0x21], n.to_le_bytes().to_vec()].concat(),
        _ => unreachable!(),
    }
}
fn execute(bytes: Vec<u8>) -> Vec<Option<Value>> {
    let mut sim = CLRSimulator::new();
    sim.load(&bytes, 0);
    sim.run(100);
    assert!(sim.halted);
    sim.stack
}
#[test]
fn signed_widening_and_identity() {
    for n in [i32::MIN, -1, 0, 1, i32::MAX] {
        assert_eq!(
            execute([literal(Value::Int(n)), vec![0x6a, 0x2a]].concat()),
            vec![Some(Value::Int64(i64::from(n)))]
        );
        assert_eq!(
            execute([literal(Value::Int(n)), vec![0x69, 0x2a]].concat()),
            vec![Some(Value::Int(n))]
        );
    }
    for n in [i64::MIN, -4294967296, -1, 0, 1, 4294967296, i64::MAX] {
        assert_eq!(
            execute([literal(Value::Int64(n)), vec![0x6a, 0x2a]].concat()),
            vec![Some(Value::Int64(n))]
        );
    }
}
#[test]
fn narrowing_uses_expected_low_bits() {
    for (n, want) in [
        (i64::MIN, 0),
        (i64::MAX, -1),
        (-2147483649, 2147483647),
        (-2147483648, -2147483648),
        (2147483647, 2147483647),
        (2147483648, -2147483648),
        (4294967296, 0),
        (-4294967296, 0),
    ] {
        assert_eq!(
            execute([literal(Value::Int64(n)), vec![0x69, 0x2a]].concat()),
            vec![Some(Value::Int(want))]
        );
        assert_eq!(
            execute([literal(Value::Int64(n)), vec![0x69, 0x6a, 0x2a]].concat()),
            vec![Some(Value::Int64(i64::from(want)))]
        );
    }
}
#[test]
fn comparison_boolean_requires_explicit_promotion_for_wide_add() {
    let prefix = [
        literal(Value::Int64(4294967296)),
        literal(Value::Int64(4294967296)),
        vec![0xfe, 0x01],
    ]
    .concat();
    assert_eq!(
        execute(
            [
                prefix.clone(),
                vec![0x6a],
                literal(Value::Int64(4294967296)),
                vec![0x58, 0x2a]
            ]
            .concat()
        ),
        vec![Some(Value::Int64(4294967297))]
    );
    let mut sim = CLRSimulator::new();
    sim.load(
        &[prefix, literal(Value::Int64(4294967296)), vec![0x58]].concat(),
        0,
    );
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sim.run(100))).is_err());
}
#[test]
fn malformed_conversion_preserves_stack_and_pc() {
    for (op, name) in [(0x69, "conv.i4"), (0x6a, "conv.i8")] {
        for stack in [
            vec![],
            vec![None],
            vec![Some(Value::Ref(None))],
            vec![Some(Value::Ref(Some(0)))],
        ] {
            let mut sim = CLRSimulator::new();
            sim.load(&[op], 0);
            sim.stack = stack.clone();
            let error =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sim.step())).unwrap_err();
            let message = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap();
            assert!(
                message.contains(name) && message.contains("integer operand"),
                "{message}"
            );
            assert_eq!(sim.stack, stack);
            assert_eq!(sim.pc, 0);
        }
    }
}
