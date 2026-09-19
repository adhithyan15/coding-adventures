use clr_simulator::{CLRSimulator, Value};
#[test]
fn signed_shifts_preserve_width() {
    for (value, count, left, right) in [
        (Value::Int(-3), 1, Value::Int(-6), Value::Int(-2)),
        (Value::Int(1), 31, Value::Int(i32::MIN), Value::Int(0)),
        (
            Value::Int(i32::MAX),
            1,
            Value::Int(-2),
            Value::Int(1073741823),
        ),
        (Value::Int(i32::MIN), 31, Value::Int(0), Value::Int(-1)),
        (Value::Int64(-3), 1, Value::Int64(-6), Value::Int64(-2)),
        (Value::Int64(1), 63, Value::Int64(i64::MIN), Value::Int64(0)),
        (
            Value::Int64(i64::MAX),
            1,
            Value::Int64(-2),
            Value::Int64(4611686018427387903),
        ),
        (
            Value::Int64(i64::MIN),
            63,
            Value::Int64(0),
            Value::Int64(-1),
        ),
        (
            Value::Int64(4294967296),
            1,
            Value::Int64(8589934592),
            Value::Int64(2147483648),
        ),
    ] {
        for (opcode, want) in [(0x62, left), (0x63, right)] {
            let mut s = CLRSimulator::new();
            s.load(&[opcode, 0x2a], 0);
            s.stack = vec![Some(Value::Int(42)), Some(value), Some(Value::Int(count))];
            s.run(10);
            assert!(s.halted);
            assert_eq!(s.stack, vec![Some(Value::Int(42)), Some(want)]);
        }
    }
    for value in [
        Value::Int(0),
        Value::Int(-1),
        Value::Int(i32::MIN),
        Value::Int64(0),
        Value::Int64(i64::MAX),
    ] {
        for opcode in [0x62, 0x63] {
            let mut s = CLRSimulator::new();
            s.load(&[opcode, 0x2a], 0);
            s.stack = vec![Some(value), Some(Value::Int(0))];
            s.run(10);
            assert_eq!(s.stack, vec![Some(value)]);
        }
    }
}
#[test]
fn refusal_preserves_operands_and_pc() {
    let mut cases = vec![
        vec![],
        vec![Some(Value::Int(1))],
        vec![None, Some(Value::Int(1))],
        vec![Some(Value::Int(1)), None],
        vec![Some(Value::Ref(None)), Some(Value::Int(1))],
        vec![Some(Value::Int(1)), Some(Value::Ref(Some(0)))],
        vec![Some(Value::Int64(1)), Some(Value::Int64(1))],
    ];
    for (value, width) in [(Value::Int(1), 32), (Value::Int64(1), 64)] {
        for count in [-1, width, width + 1, i32::MAX] {
            cases.push(vec![Some(value), Some(Value::Int(count))]);
        }
    }
    for opcode in [0x62, 0x63] {
        for stack in &cases {
            let mut s = CLRSimulator::new();
            s.load(&[opcode], 0);
            s.stack = stack.clone();
            let error =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.run(1))).unwrap_err();
            let msg = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap();
            assert!(
                msg.contains(if opcode == 0x62 { "shl" } else { "shr" }),
                "{msg}"
            );
            assert_eq!(&s.stack, stack);
            assert_eq!(s.pc, 0);
        }
    }
}

#[test]
fn literals_flow_through_shifts_to_return() {
    for (bytes, want) in [
        (vec![0x17, 0x1f, 31, 0x62, 0x2a], Value::Int(i32::MIN)),
        (vec![0x1f, 253, 0x17, 0x63, 0x2a], Value::Int(-2)),
        (
            vec![0x21, 0, 0, 0, 0, 1, 0, 0, 0, 0x17, 0x62, 0x2a],
            Value::Int64(8589934592),
        ),
        (
            vec![
                0x21, 253, 255, 255, 255, 255, 255, 255, 255, 0x17, 0x63, 0x2a,
            ],
            Value::Int64(-2),
        ),
    ] {
        let mut s = CLRSimulator::new();
        s.load(&bytes, 0);
        s.run(10);
        assert!(s.halted);
        assert_eq!(s.stack, vec![Some(want)]);
    }
}
