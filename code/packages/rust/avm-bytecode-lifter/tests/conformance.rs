//! Conformance test suite for `avm-bytecode-lifter` (SWF04).

#![deny(unsafe_code)]

use avm_bytecode_lifter::{
    lift_avm1, lift_avm2, run_avm1, validate_constant_count, validate_instruction_count,
    validate_stack_depth, AvmRunner, LifterError, SwfError, MAX_CONSTANTS, MAX_INSTRUCTIONS,
    MAX_OPERAND_STACK,
};

fn encode_avm1_push_int(val: i32) -> Vec<u8> {
    let mut bytes = vec![0x96, 5, 0, 7];
    bytes.extend_from_slice(&(val as u32).to_le_bytes());
    bytes
}

fn encode_avm1_push_str(s: &str) -> Vec<u8> {
    let s_bytes = s.as_bytes();
    let len = 1 + s_bytes.len() + 1; // type byte (0) + string + null terminator
    let mut bytes = vec![0x96, (len & 0xFF) as u8, ((len >> 8) & 0xFF) as u8, 0];
    bytes.extend_from_slice(s_bytes);
    bytes.push(0);
    bytes
}

#[test]
fn test_avm1_arithmetic_and_trace() {
    let mut bc = Vec::new();
    // push 15
    bc.extend_from_slice(&encode_avm1_push_int(15));
    // push 27
    bc.extend_from_slice(&encode_avm1_push_int(27));
    // add2 (0x47)
    bc.push(0x47);
    // trace (0x26)
    bc.push(0x26);

    let output = run_avm1(&bc).expect("execution should succeed");
    assert_eq!(output, "42\n");
}

#[test]
fn test_avm1_polymorphic_add_string() {
    let mut bc = Vec::new();
    // push "Score: "
    bc.extend_from_slice(&encode_avm1_push_str("Score: "));
    // push 100
    bc.extend_from_slice(&encode_avm1_push_int(100));
    // add2
    bc.push(0x47);
    // trace
    bc.push(0x26);

    let output = run_avm1(&bc).expect("execution should succeed");
    assert_eq!(output, "Score: 100\n");
}

#[test]
fn test_avm1_variables_get_and_set() {
    let mut bc = Vec::new();
    // push "player_health"
    bc.extend_from_slice(&encode_avm1_push_str("player_health"));
    // push 99
    bc.extend_from_slice(&encode_avm1_push_int(99));
    // setVariable (0x1D)
    bc.push(0x1D);

    // push "player_health"
    bc.extend_from_slice(&encode_avm1_push_str("player_health"));
    // getVariable (0x1C)
    bc.push(0x1C);
    // trace (0x26)
    bc.push(0x26);

    let output = run_avm1(&bc).expect("execution should succeed");
    assert_eq!(output, "99\n");
}

#[test]
fn test_avm1_objects_and_members() {
    let mut bc = Vec::new();
    // push "Player"
    bc.extend_from_slice(&encode_avm1_push_str("Player"));
    // push 0 (nargs)
    bc.extend_from_slice(&encode_avm1_push_int(0));
    // newObject (0x40)
    bc.push(0x40);
    // push "score"
    bc.extend_from_slice(&encode_avm1_push_str("score"));
    // push 500
    bc.extend_from_slice(&encode_avm1_push_int(500));
    // setMember (0x4F)
    bc.push(0x4F);

    let module = lift_avm1(&bc, "obj_test").expect("lifting should succeed");
    let mut runner = AvmRunner::runner_new();
    let res = runner.runner_execute(&module);
    assert!(res.is_ok());
}

#[test]
fn test_avm1_timeline_verbs() {
    let mut bc = Vec::new();
    // stop (0x07)
    bc.push(0x07);
    // play (0x06)
    bc.push(0x06);
    // gotoFrame 12 (0x81, len 2, 12, 0)
    bc.extend_from_slice(&[0x81, 0x02, 0x00, 12, 0]);

    let module = lift_avm1(&bc, "timeline_test").expect("lifting should succeed");
    let mut runner = AvmRunner::runner_new();
    runner.runner_execute(&module).expect("execution succeeded");
    let log = runner.runner_action_log();
    assert_eq!(log, vec!["stop", "play", "gotoFrame(12)"]);
}

#[test]
fn test_avm1_branch_always() {
    let mut bc = Vec::new();
    // branchAlways (0x99, len 2, offset to skip the next push+trace)
    // push 999 (8 bytes) + trace (1 byte) = 9 bytes to skip
    // So jump offset = 9
    bc.extend_from_slice(&[0x99, 0x02, 0x00, 9, 0]);

    // Skipped instruction:
    bc.extend_from_slice(&encode_avm1_push_int(999));
    bc.push(0x26); // trace

    // Target instruction:
    bc.extend_from_slice(&encode_avm1_push_int(123));
    bc.push(0x26); // trace

    let output = run_avm1(&bc).expect("branch execution should succeed");
    assert_eq!(output, "123\n");
}

#[test]
fn test_avm1_invalid_branch_target_rejected() {
    let mut bc = Vec::new();
    // Branch jumping into the middle of an instruction (+3 bytes when instruction is 9 bytes)
    bc.extend_from_slice(&[0x99, 0x02, 0x00, 3, 0]);
    bc.extend_from_slice(&encode_avm1_push_int(42));
    bc.push(0x26);

    let res = lift_avm1(&bc, "invalid_branch");
    assert_eq!(res.err(), Some(SwfError::InvalidBranchTarget));
}

#[test]
fn test_avm1_branch_out_of_bounds_rejected() {
    let mut bc = Vec::new();
    // Branch jumping far past EOF
    bc.extend_from_slice(&[0x99, 0x02, 0x00, 100, 0]);

    let res = lift_avm1(&bc, "out_of_bounds_branch");
    assert_eq!(res.err(), Some(SwfError::InvalidBranchTarget));
}

#[test]
fn test_avm1_stack_underflow_rejected() {
    // Pop on empty stack
    let bc = vec![0x17];

    let res = lift_avm1(&bc, "stack_underflow");
    assert_eq!(res.err(), Some(SwfError::StackUnderflow));
}

#[test]
fn test_avm1_arithmetic_underflow_rejected() {
    let mut bc = Vec::new();
    // Push only 1 operand then add2 (requires 2 operands)
    bc.extend_from_slice(&encode_avm1_push_int(10));
    bc.push(0x47);

    let res = lift_avm1(&bc, "add2_underflow");
    assert_eq!(res.err(), Some(SwfError::StackUnderflow));
}

#[test]
fn test_avm1_cwe1321_prototype_pollution_blocked() {
    let mut bc = Vec::new();
    // Attempt to set variable on Object.prototype
    bc.extend_from_slice(&encode_avm1_push_str("Object.prototype"));
    bc.extend_from_slice(&encode_avm1_push_str("hacked"));
    bc.push(0x1D); // setVariable

    let module = lift_avm1(&bc, "proto_test").expect("lifting should succeed");
    let mut runner = AvmRunner::runner_new();
    let res = runner.runner_execute(&module);
    assert!(res.is_err());
    match res.unwrap_err() {
        LifterError::PrototypePollutionBlocked(_) => {}
        other => panic!("expected PrototypePollutionBlocked, got {:?}", other),
    }
}

// ----------------------------------------------------------------------------
// AVM2 ABC Tests
// ----------------------------------------------------------------------------

fn build_minimal_abc(code: &[u8], max_stack: u32, local_count: u32) -> Vec<u8> {
    let mut abc = Vec::new();
    // minor version 16, major version 46
    abc.extend_from_slice(&16u16.to_le_bytes());
    abc.extend_from_slice(&46u16.to_le_bytes());

    // CPool:
    // int_count: 0
    abc.push(0);
    // uint_count: 0
    abc.push(0);
    // double_count: 0
    abc.push(0);
    // string_count: 0
    abc.push(0);
    // namespace_count: 0
    abc.push(0);
    // ns_set_count: 0
    abc.push(0);
    // multiname_count: 0
    abc.push(0);

    // method_count: 1
    abc.push(1);
    // method 0: param_count: 0, return_type: 0, name: 0, flags: 0
    abc.extend_from_slice(&[0, 0, 0, 0]);

    // metadata_count: 0
    abc.push(0);

    // class_count: 0
    abc.push(0);

    // script_count: 1
    abc.push(1);
    // script 0: init_method: 0, trait_count: 0
    abc.extend_from_slice(&[0, 0]);

    // method_body_count: 1
    abc.push(1);
    // method_index: 0
    abc.push(0);
    // max_stack
    abc.push(max_stack as u8);
    // local_count
    abc.push(local_count as u8);
    // init_scope_depth: 0, max_scope_depth: 0
    abc.extend_from_slice(&[0, 0]);
    // code_length
    abc.push(code.len() as u8);
    // code
    abc.extend_from_slice(code);
    // exception_count: 0
    abc.push(0);
    // trait_count: 0
    abc.push(0);

    abc
}

#[test]
fn test_avm2_abc_simple_arithmetic() {
    // pushbyte 20, pushbyte 22, add, returnvalue
    let code = [0x24, 20, 0x24, 22, 0xA0, 0x48];
    let abc = build_minimal_abc(&code, 8, 4);

    let module = lift_avm2(&abc, "abc_math").expect("lifting should succeed");
    let mut runner = AvmRunner::runner_new();
    let res = runner
        .runner_execute(&module)
        .expect("execution should succeed");
    assert_eq!(res, vm_core::value::Value::Int(42));
}

#[test]
fn test_avm2_abc_local_registers() {
    // pushbyte 77, setlocal_1, getlocal_1, returnvalue
    let code = [0x24, 77, 0xD5, 0xD1, 0x48];
    let abc = build_minimal_abc(&code, 8, 4);

    let module = lift_avm2(&abc, "abc_reg").expect("lifting should succeed");
    let mut runner = AvmRunner::runner_new();
    let res = runner
        .runner_execute(&module)
        .expect("execution should succeed");
    assert_eq!(res, vm_core::value::Value::Int(77));
}

#[test]
fn test_avm2_abc_invalid_branch_target() {
    // jump with offset that lands in the middle of pushbyte
    // jump is 4 bytes (0x10, b0, b1, b2). If offset is 1, target is 4 + 1 = 5.
    // Next instruction is pushbyte (0x24, 10) starting at 4, ending at 6.
    // Target 5 is inside pushbyte!
    let code = [0x10, 1, 0, 0, 0x24, 10, 0x48];
    let abc = build_minimal_abc(&code, 8, 4);

    let res = lift_avm2(&abc, "abc_invalid_branch");
    assert_eq!(res.err(), Some(SwfError::InvalidBranchTarget));
}

#[test]
fn test_avm2_abc_stack_underflow() {
    // add on empty stack
    let code = [0xA0];
    let abc = build_minimal_abc(&code, 8, 4);

    let res = lift_avm2(&abc, "abc_underflow");
    assert_eq!(res.err(), Some(SwfError::StackUnderflow));
}

// ----------------------------------------------------------------------------
// Warrant Agreement Tests
// ----------------------------------------------------------------------------

#[test]
fn test_warrant_stack_depth_soundness() {
    assert_eq!(validate_stack_depth(0), Ok(true));
    assert_eq!(validate_stack_depth(MAX_OPERAND_STACK), Ok(true));
    assert_eq!(
        validate_stack_depth(MAX_OPERAND_STACK + 1),
        Err(LifterError::StackOverflow)
    );
}

#[test]
fn test_warrant_instruction_count_soundness() {
    assert_eq!(validate_instruction_count(0), Ok(true));
    assert_eq!(validate_instruction_count(MAX_INSTRUCTIONS), Ok(true));
    assert_eq!(
        validate_instruction_count(MAX_INSTRUCTIONS + 1),
        Err(LifterError::InstructionLimitExceeded)
    );
}

#[test]
fn test_warrant_constant_count_soundness() {
    assert_eq!(validate_constant_count(0), Ok(true));
    assert_eq!(validate_constant_count(MAX_CONSTANTS), Ok(true));
    assert_eq!(
        validate_constant_count(MAX_CONSTANTS + 1),
        Err(LifterError::ConstantPoolLimitExceeded)
    );
}
