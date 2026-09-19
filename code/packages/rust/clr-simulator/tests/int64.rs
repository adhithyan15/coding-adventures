use clr_simulator::*;

fn wide(n: i64) -> Vec<u8> {
    let mut bytes = vec![0x21];
    bytes.extend(n.to_le_bytes());
    bytes
}
fn run(parts: &[Vec<u8>]) -> CLRSimulator {
    let mut sim = CLRSimulator::new();
    sim.load(&parts.concat(), 1);
    sim.run(100);
    assert!(sim.halted);
    sim
}

#[test]
fn literals_and_local_roundtrips_preserve_width() {
    for n in [i64::MIN, i64::MAX, -2147483649, 2147483648, 1, -7] {
        let sim = run(&[wide(n), vec![0x0a, 0x06, 0x2a]]);
        assert_eq!(sim.stack, vec![Some(Value::Int64(n))]);
        assert_eq!(sim.locals[0], Some(Value::Int64(n)));
    }
}

#[test]
fn arithmetic_keeps_width_and_signed_results() {
    for (a, b, op, expected) in [
        (i64::MAX, 1, OP_ADD, i64::MIN),
        (i64::MIN, 1, OP_SUB, i64::MAX),
        (4294967296, 3, OP_MUL, 12884901888),
        (4294967296, 1, OP_XOR, 4294967297),
        (-4294967297, 2, OP_DIV, -2147483648),
    ] {
        assert_eq!(run(&[wide(a), wide(b), vec![op, OP_RET]]).stack,
            vec![Some(Value::Int64(expected))]);
    }
    assert_eq!(run(&[wide(i64::MIN), vec![OP_NEG, OP_RET]]).stack,
        vec![Some(Value::Int64(i64::MIN))]);
    assert_eq!(run(&[encode_ldc_i4(i32::MAX), encode_ldc_i4(1), vec![OP_ADD, OP_RET]]).stack,
        vec![Some(Value::Int(i32::MIN))]);
}

#[test]
fn comparison_and_branches_use_full_width() {
    for (a, b, op, expected) in [(4294967296, 0, CEQ_BYTE, 0),
        (i64::MIN, 0, CLT_BYTE, 1), (i64::MAX, 0, CGT_BYTE, 1)] {
        assert_eq!(run(&[wide(a), wide(b), vec![0xfe, op, OP_RET]]).stack,
            vec![Some(Value::Int(expected))]);
    }
    // brfalse.s jumps over ldc.i4.1 and br.s to ldc.i4.0.
    for (value, expected) in [(0, 0), (4294967296, 1)] {
        assert_eq!(run(&[wide(value), vec![0x2c, 3, 0x17, 0x2b, 1, 0x16, OP_RET]]).stack,
            vec![Some(Value::Int(expected))]);
    }
}

#[test]
fn call_arguments_and_returns_preserve_int64() {
    let mut sim = CLRSimulator::new();
    sim.load_program(vec![
        MethodCode { body: [wide(i64::MAX), vec![OP_CALL, 2, 0, 0, 6, OP_RET]].concat(), num_locals: 0, num_args: 0 },
        MethodCode { body: vec![OP_LDARG_0, OP_RET], num_locals: 0, num_args: 1 },
    ], 0);
    sim.run(20);
    assert_eq!(sim.stack, vec![Some(Value::Int64(i64::MAX))]);
}

#[test]
fn loose_object_array_transport_keeps_all_bits() {
    let sim = run(&[
        encode_ldc_i4(1), vec![OP_NEWARR, 1, 0, 0, 1, 0x25],
        encode_ldc_i4(0), wide(i64::MIN), vec![OP_STELEM_REF],
        encode_ldc_i4(0), vec![OP_LDELEM_REF, OP_RET],
    ]);
    assert_eq!(sim.stack, vec![Some(Value::Int64(i64::MIN))]);
    assert_eq!(sim.heap[0][0], Value::Int64(i64::MIN));
}

#[test]
fn truncated_literal_refuses_without_state_change() {
    for length in 0..8 {
        let mut sim = CLRSimulator::new();
        sim.load(&[vec![0x21], vec![0; length]].concat(), 0);
        sim.stack.push(Some(Value::Int(7)));
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| sim.step())).is_err());
        assert_eq!(sim.pc, 0);
        assert_eq!(sim.stack, vec![Some(Value::Int(7))]);
    }
}

#[test]
fn invalid_arithmetic_and_indices_refuse() {
    for parts in [
        vec![wide(1), encode_ldc_i4(1), vec![OP_ADD]],
        vec![wide(1), encode_ldc_i4(1), vec![0xfe, CEQ_BYTE]],
        vec![wide(1), wide(0), vec![OP_DIV]],
        vec![wide(i64::MIN), wide(-1), vec![OP_DIV]],
        vec![encode_ldc_i4(i32::MIN), encode_ldc_i4(-1), vec![OP_DIV]],
        vec![wide(1), vec![OP_NEWARR, 1, 0, 0, 1]],
    ] {
        assert!(std::panic::catch_unwind(|| run(&parts)).is_err());
    }
}
