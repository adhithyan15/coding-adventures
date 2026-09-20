use clr_simulator::{CLRSimulator, Value};
fn code(op: u8, offset: i32) -> Vec<u8> {
    [
        vec![op],
        offset.to_le_bytes().to_vec(),
        vec![0x17, 0x2a, 0x18, 0x2a],
    ]
    .concat()
}
#[test]
fn literal_long_branches_preserve_conditions_and_direction() {
    for op in [0x38, 0x39, 0x3a] {
        for (v, truth) in [
            (Value::Int(0), false),
            (Value::Int(-1), true),
            (Value::Int64(0), false),
            (Value::Int64(1 << 40), true),
            (Value::Ref(None), false),
            (Value::Ref(Some(7)), true),
        ] {
            let mut s = CLRSimulator::new();
            s.load(&code(op, 2), 0);
            s.stack = vec![Some(Value::Int64(99)), Some(v)];
            s.run(10);
            let take = op == 0x38 || if op == 0x39 { !truth } else { truth };
            let mut want = vec![Some(Value::Int64(99))];
            if op == 0x38 {
                want.push(Some(v));
            }
            want.push(Some(Value::Int(if take { 2 } else { 1 })));
            assert_eq!(s.stack, want);
        }
        for offset in [-5, 0] {
            let mut s = CLRSimulator::new();
            s.load(&code(op, offset), 0);
            s.stack = vec![Some(Value::Int(if op == 0x39 { 0 } else { 1 }))];
            s.step();
            assert_eq!(s.pc, if offset == -5 { 0 } else { 5 });
        }
        let mut s = CLRSimulator::new();
        s.load(&[vec![0; 200], code(op, -205)].concat(), 0);
        s.pc = 200;
        s.stack = vec![Some(Value::Int(if op == 0x39 { 0 } else { 1 }))];
        s.step();
        assert_eq!(s.pc, 0);
    }
}
#[test]
fn malformed_long_branches_refuse_without_mutation() {
    for op in [0x38, 0x39, 0x3a] {
        let mut cases = Vec::new();
        for len in 1..5 {
            cases.push((
                vec![op; len],
                vec![Some(Value::Int(0))],
                "Truncated operand",
            ));
        }
        for offset in [-6, 4, i32::MIN, i32::MAX] {
            cases.push((
                code(op, offset),
                vec![Some(Value::Int(0))],
                "Invalid target",
            ));
        }
        if op != 0x38 {
            for stack in [vec![], vec![None]] {
                cases.push((code(op, 0), stack, "Missing stack operand"));
            }
        }
        for (bytes, stack, message) in cases {
            let mut s = CLRSimulator::new();
            s.load(&bytes, 1);
            s.stack = stack.clone();
            let locals = s.locals.clone();
            let e = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| s.step()))
                .expect_err("must refuse");
            let text = e
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| e.downcast_ref::<&str>().copied())
                .unwrap();
            assert!(text.contains(message), "{text}");
            assert!(text.contains("PC=0"));
            assert_eq!(s.pc, 0);
            assert_eq!(s.stack, stack);
            assert_eq!(s.locals, locals);
        }
    }
}
