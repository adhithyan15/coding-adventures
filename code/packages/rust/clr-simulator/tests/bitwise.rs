use clr_simulator::{CLRSimulator, Value};
fn literal(v: Value) -> Vec<u8> {
    match v {
        Value::Int(n) => [vec![0x20], n.to_le_bytes().to_vec()].concat(),
        Value::Int64(n) => [vec![0x21], n.to_le_bytes().to_vec()].concat(),
        _ => unreachable!(),
    }
}
#[test]
fn literal_bit_patterns_keep_their_width() {
    for (a, b, and, or) in [
        (Value::Int(0), Value::Int(-1), Value::Int(0), Value::Int(-1)),
        (
            Value::Int(0x55555555),
            Value::Int(-1431655766),
            Value::Int(0),
            Value::Int(-1),
        ),
        (
            Value::Int(i32::MIN),
            Value::Int(i32::MAX),
            Value::Int(0),
            Value::Int(-1),
        ),
        (
            Value::Int(0x12345678),
            Value::Int(-1),
            Value::Int(0x12345678),
            Value::Int(-1),
        ),
        (
            Value::Int64(0),
            Value::Int64(-1),
            Value::Int64(0),
            Value::Int64(-1),
        ),
        (
            Value::Int64(i64::MIN),
            Value::Int64(i64::MAX),
            Value::Int64(0),
            Value::Int64(-1),
        ),
        (
            Value::Int64(0x5555555555555555),
            Value::Int64(-6148914691236517206),
            Value::Int64(0),
            Value::Int64(-1),
        ),
        (
            Value::Int64(4294967297),
            Value::Int64(4294967298),
            Value::Int64(4294967296),
            Value::Int64(4294967299),
        ),
        (
            Value::Int64(4294967297),
            Value::Int64(-1),
            Value::Int64(4294967297),
            Value::Int64(-1),
        ),
    ] {
        for (x, y) in [(a, b), (b, a)] {
            for (op, want) in [(0x5f, and), (0x60, or)] {
                let bytes = [vec![0x19], literal(x), literal(y), vec![op, 0x2a]].concat();
                let mut s = CLRSimulator::new();
                s.load(&bytes, 0);
                s.run(10);
                assert!(s.halted);
                assert_eq!(s.stack, vec![Some(Value::Int(3)), Some(want)]);
            }
        }
    }
}
#[test]
fn malformed_operands_refuse_without_mutation() {
    let mut cases = vec![vec![], vec![Some(Value::Int(1))]];
    for bad in [
        None,
        Some(Value::Ref(None)),
        Some(Value::Ref(Some(0))),
        Some(Value::Int64(1)),
    ] {
        cases.push(vec![bad, Some(Value::Int(1))]);
        cases.push(vec![Some(Value::Int(1)), bad]);
    }
    for op in [0x5f, 0x60] {
        for stack in &cases {
            let mut s = CLRSimulator::new();
            s.load(&[op], 0);
            s.stack = stack.clone();
            let e =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.run(1))).unwrap_err();
            let msg = e
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap();
            assert!(
                msg.contains(if op == 0x5f {
                    "and requires"
                } else {
                    "or requires"
                }),
                "{msg}"
            );
            assert_eq!(&s.stack, stack);
            assert_eq!(s.pc, 0);
        }
    }
}
