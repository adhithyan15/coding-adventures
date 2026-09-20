use clr_simulator::{
    encode_ldc_i4, CLRSimulator, Value, OP_LDC_I4, OP_LDC_I4_M1, OP_LDC_I4_S, OP_RET,
};

#[test]
fn raw_compact_minus_one_executes_as_int32() {
    let mut simulator = CLRSimulator::new();
    simulator.load(&[0x15, OP_RET], 0);

    let trace = simulator.step();
    assert_eq!(trace.pc, 0);
    assert_eq!(trace.opcode, "ldc.i4.m1");
    assert_eq!(simulator.pc, 1);
    assert_eq!(simulator.stack, vec![Some(Value::Int(-1))]);

    simulator.step();
    assert!(simulator.halted);
    assert_eq!(simulator.stack, vec![Some(Value::Int(-1))]);
}

#[test]
fn integer_encoder_selects_every_constant_width_boundary() {
    assert_eq!(encode_ldc_i4(-1), vec![OP_LDC_I4_M1]);
    assert_eq!(encode_ldc_i4(-2), vec![OP_LDC_I4_S, 0xfe]);
    assert_eq!(encode_ldc_i4(-128), vec![OP_LDC_I4_S, 0x80]);
    assert_eq!(encode_ldc_i4(0), vec![0x16]);
    assert_eq!(encode_ldc_i4(8), vec![0x1e]);
    assert_eq!(encode_ldc_i4(127), vec![OP_LDC_I4_S, 0x7f]);
    assert_eq!(encode_ldc_i4(128), vec![OP_LDC_I4, 0x80, 0, 0, 0]);
}

#[test]
fn helper_produced_compact_minus_one_executes() {
    let mut bytecode = encode_ldc_i4(-1);
    bytecode.push(OP_RET);

    let mut simulator = CLRSimulator::new();
    simulator.load(&bytecode, 0);
    simulator.run(2);

    assert_eq!(simulator.stack, vec![Some(Value::Int(-1))]);
}
