//! Conformance test suite for `avm-bytecode-lifter` (SWF04).

#![deny(unsafe_code)]

use avm_bytecode_lifter::{
    lift_avm1, lift_avm2, run_avm1, validate_constant_count, validate_instruction_count,
    validate_stack_depth, AvmEnvironment, AvmRunner, LifterError, SwfError, MAX_CONSTANTS,
    MAX_INSTRUCTIONS, MAX_OPERAND_STACK,
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

// ----------------------------------------------------------------------------
// Security Review & Soundness Tests (CWE-20, CWE-400, CWE-190, CWE-1321)
// ----------------------------------------------------------------------------

#[test]
fn test_nested_define_function_depth_limit() {
    // ActionPop (0x17) as innermost body
    let mut current_body = vec![0x17];
    // Nest 33 levels (> MAX_FUNCTION_DEPTH = 32)
    for _ in 0..33 {
        let mut outer = vec![0x9B, 5, 0, 0, 0, 0];
        let blen = current_body.len() as u16;
        outer.extend_from_slice(&blen.to_le_bytes());
        outer.extend_from_slice(&current_body);
        current_body = outer;
    }
    let res = lift_avm1(&current_body, "deep_fn");
    assert_eq!(res.err(), Some(LifterError::FunctionLimitExceeded));
}

#[test]
fn test_truncated_action_payload_len() {
    // ActionPush (0x96) claiming payload_len = 100, but only 4 bytes follow
    let bc = vec![0x96, 100, 0, 7, 1, 2, 3];
    let res = lift_avm1(&bc, "trunc_payload");
    assert_eq!(res.err(), Some(LifterError::UnexpectedEof));
}

#[test]
fn test_oversized_avm2_constant_pool_and_u30() {
    // 1. int_count > MAX_CONSTANTS
    let mut abc = vec![16, 0, 46, 0];
    // 70000 in u30 LEB128: 70000 = 0x11170 -> 0x70 | 0x80 = 0xF0, 0x22 | 0x80 = 0xA2, 0x04
    abc.extend_from_slice(&[0xF0, 0xA2, 0x04]);
    let res = lift_avm2(&abc, "huge_cpool");
    assert_eq!(res.err(), Some(LifterError::ConstantPoolLimitExceeded));

    // 2. u30 out of 30-bit range (> 0x3FFF_FFFF)
    let mut abc2 = vec![16, 0, 46, 0];
    abc2.extend_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]);
    let res2 = lift_avm2(&abc2, "huge_u30");
    assert!(matches!(res2.err(), Some(LifterError::InvalidBytecode(_))));

    // 3. Truncated read_bytes
    let mut abc3 = vec![16, 0, 46, 0, 0, 0, 0, 2]; // 1 string
    abc3.extend_from_slice(&[50]); // string length 50, but EOF immediately
    let res3 = lift_avm2(&abc3, "trunc_abc");
    assert_eq!(res3.err(), Some(LifterError::UnexpectedEof));
}

#[test]
fn test_cfg_stack_mismatch_at_merge_point() {
    // Offset 0: Push 1 (8 bytes) -> depth 1
    // Offset 8: BranchIfTrue to 34 (5 bytes) -> pops 1, depth 0. Target: 13 + 21 = 34.
    // Offset 13: Push 10 (8 bytes) -> depth 1
    // Offset 21: Push 20 (8 bytes) -> depth 2
    // Offset 29: BranchAlways to 34 (5 bytes) -> depth 2. Target: 34 + 0 = 34.
    // Offset 34: Play (1 byte) -> merge point with conflicting depths (0 vs 2)
    // Offset 35: ActionEnd (1 byte)
    let mut bc = Vec::new();
    bc.extend_from_slice(&[0x96, 5, 0, 7, 1, 0, 0, 0]);
    bc.extend_from_slice(&[0x9D, 2, 0, 21, 0]);
    bc.extend_from_slice(&[0x96, 5, 0, 7, 10, 0, 0, 0]);
    bc.extend_from_slice(&[0x96, 5, 0, 7, 20, 0, 0, 0]);
    bc.extend_from_slice(&[0x99, 2, 0, 0, 0]);
    bc.push(0x06); // Play (pops 0, pushes 0)
    bc.push(0x00); // End

    let res = lift_avm1(&bc, "cfg_mismatch");
    assert!(matches!(res.err(), Some(LifterError::InvalidBytecode(_))));
}

#[test]
fn test_prototype_pollution_defense() {
    let env = AvmEnvironment::env_new();

    // 1. Member access prototype pollution blocked
    let res1 = env.env_set_member(1, "prototype", vm_core::value::Value::Int(1));
    assert!(matches!(
        res1.err(),
        Some(LifterError::PrototypePollutionBlocked(_))
    ));

    let res2 = env.env_set_member(1, "__proto__", vm_core::value::Value::Int(1));
    assert!(matches!(
        res2.err(),
        Some(LifterError::PrototypePollutionBlocked(_))
    ));

    let res3 = env.env_set_member(1, "constructor", vm_core::value::Value::Int(1));
    assert!(matches!(
        res3.err(),
        Some(LifterError::PrototypePollutionBlocked(_))
    ));

    // 2. Variable path prototype pollution blocked
    let res4 = env.env_set_variable(
        "Object.prototype.isAdmin",
        vm_core::value::Value::Bool(true),
    );
    assert!(matches!(
        res4.err(),
        Some(LifterError::PrototypePollutionBlocked(_))
    ));

    let res5 = env.env_set_variable("_global.Object.prototype", vm_core::value::Value::Int(123));
    assert!(matches!(
        res5.err(),
        Some(LifterError::PrototypePollutionBlocked(_))
    ));

    let res6 = env.env_set_variable("myObj.__proto__.polluted", vm_core::value::Value::Int(99));
    assert!(matches!(
        res6.err(),
        Some(LifterError::PrototypePollutionBlocked(_))
    ));

    let res7 = env.env_set_variable("Function.prototype.hook", vm_core::value::Value::Int(1));
    assert!(matches!(
        res7.err(),
        Some(LifterError::PrototypePollutionBlocked(_))
    ));
}

#[test]
fn test_avm2_extended_opcodes_execution() {
    // 1. Modulo: pushbyte 10, pushbyte 3, modulo, returnvalue
    let code_mod = [0x24, 10, 0x24, 3, 0xA4, 0x48];
    let abc_mod = build_minimal_abc(&code_mod, 8, 4);
    let mod_module = lift_avm2(&abc_mod, "abc_mod").expect("lifting should succeed");
    let mut runner = AvmRunner::runner_new();
    let res = runner
        .runner_execute(&mod_module)
        .expect("execution should succeed");
    assert_eq!(res, vm_core::value::Value::Int(1));

    // 2. Negate: pushbyte 42, negate, returnvalue
    let code_neg = [0x24, 42, 0x90, 0x48];
    let abc_neg = build_minimal_abc(&code_neg, 8, 4);
    let neg_module = lift_avm2(&abc_neg, "abc_neg").expect("lifting should succeed");
    let res_neg = runner
        .runner_execute(&neg_module)
        .expect("execution should succeed");
    assert_eq!(res_neg, vm_core::value::Value::Int(-42));

    // 3. Swap: pushbyte 10, pushbyte 25, swap, subtract, returnvalue
    // Without swap: 10 - 25 = -15. With swap: 25 - 10 = 15!
    let code_swap = [0x24, 10, 0x24, 25, 0x2B, 0xA1, 0x48];
    let abc_swap = build_minimal_abc(&code_swap, 8, 4);
    let swap_module = lift_avm2(&abc_swap, "abc_swap").expect("lifting should succeed");
    let res_swap = runner
        .runner_execute(&swap_module)
        .expect("execution should succeed");
    assert_eq!(res_swap, vm_core::value::Value::Int(15));
}
