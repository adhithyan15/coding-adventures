use cdc6600_gatelevel::*;

fn short(program: &mut Vec<u8>, opcode: u8, i: u8, j: u8, k: u8) {
    program.extend_from_slice(&short_instr(opcode, i, j, k));
}

fn long(program: &mut Vec<u8>, opcode: u8, i: u8, j: u8, constant: u32) {
    program.extend_from_slice(&long_instr(opcode, i, j, constant));
}

fn halt(program: &mut Vec<u8>) {
    program.extend_from_slice(&HALT);
}

fn assert_lockstep(
    functional: &mut Cdc6600Simulator,
    gate: &mut Cdc6600GateLevel,
    max_steps: usize,
) {
    assert_eq!(gate.state(), functional.state());
    for _ in 0..max_steps {
        if functional.state().halted {
            assert!(gate.state().halted);
            return;
        }
        let functional_trace = functional.step().expect("functional step succeeds");
        let gate_trace = gate.step().expect("gate step succeeds");
        assert_eq!(gate_trace, functional_trace);
        assert_eq!(gate.state(), functional.state());
    }
    panic!("program did not halt within the test bound");
}

#[test]
fn topology_lifecycle_transport_and_mutators_match() {
    let gate = Cdc6600GateLevel::new();
    assert_eq!(gate.flip_flop_count(), 246_529);
    assert_eq!(gate.gate_count(), 1_519_174);
    assert_eq!(gate.state(), Cdc6600Simulator::new().state());

    let mut functional = Cdc6600Simulator::new();
    let mut gate = Cdc6600GateLevel::new();
    let mut program = Vec::new();
    long(&mut program, F_LDXI, 1, 0, 7);
    halt(&mut program);
    assert_eq!(gate.load(&program), functional.load(&program));
    functional.write_x(0, u64::MAX).unwrap();
    gate.write_x(0, u64::MAX).unwrap();
    functional.write_a(0, u32::MAX).unwrap();
    gate.write_a(0, u32::MAX).unwrap();
    functional.write_b(0, u32::MAX).unwrap();
    gate.write_b(0, u32::MAX).unwrap();
    functional.write_b(1, u32::MAX).unwrap();
    gate.write_b(1, u32::MAX).unwrap();
    functional.write_word(100, u64::MAX).unwrap();
    gate.write_word(100, u64::MAX).unwrap();
    assert_eq!(gate.read_word(100), functional.read_word(100));
    assert_eq!(gate.state(), functional.state());
    assert_lockstep(&mut functional, &mut gate, 3);
    assert_eq!(gate.step().unwrap(), functional.step().unwrap());
    gate.reset();
    assert_eq!(gate.state(), Cdc6600Simulator::new().state());
}

#[test]
fn all_twenty_two_short_opcodes_match_after_every_clock() {
    let mut program = Vec::new();
    for (opcode, i, j, k) in [
        (F_TXB, 1, 2, 0),
        (F_TBX, 3, 4, 0),
        (F_TAX, 5, 6, 0),
        (F_TXA, 7, 4, 0),
        (F_IXPB, 0, 1, 2),
        (F_IXMB, 3, 1, 2),
        (F_IXXP, 4, 1, 5),
        (F_IXXM, 6, 5, 1),
        (F_BXND, 3, 1, 2),
        (F_BXOR, 4, 1, 2),
        (F_BXXR, 5, 1, 2),
        (F_BXMR, 6, 1, 0),
        (F_LSHL, 7, 1, 3),
        (F_LSHR, 0, 2, 3),
        (F_IBBP, 1, 2, 3),
        (F_IBBM, 4, 2, 3),
        (F_IAAP, 1, 2, 3),
        (F_IAAM, 4, 2, 3),
        (F_CMPEQ, 5, 1, 1),
        (F_CMPLT, 6, 2, 1),
        (F_CMPGT, 7, 1, 2),
        (F_IXMUL, 2, 1, 2),
    ] {
        short(&mut program, opcode, i, j, k);
    }
    halt(&mut program);

    let mut functional = Cdc6600Simulator::new();
    let mut gate = Cdc6600GateLevel::new();
    functional.load(&program).unwrap();
    gate.load(&program).unwrap();
    for (index, value) in [0, 0x0123_4567_89ab, MASK_60, 3, 0x2aaaa, 20, SIGN_60, 7]
        .into_iter()
        .enumerate()
    {
        functional.write_x(index, value).unwrap();
        gate.write_x(index, value).unwrap();
    }
    for index in 0..8 {
        let a_value = (index as u32 * 17) & MASK_18;
        let b_value = (index as u32 * 5) & MASK_18;
        functional.write_a(index, a_value).unwrap();
        gate.write_a(index, a_value).unwrap();
        functional.write_b(index, b_value).unwrap();
        gate.write_b(index, b_value).unwrap();
    }
    assert_lockstep(&mut functional, &mut gate, 24);
}

#[test]
fn every_barrel_stage_and_seeded_partial_product_match() {
    let mut shift_program = Vec::new();
    short(&mut shift_program, F_LSHL, 2, 1, 1);
    let mut shift_functional = Cdc6600Simulator::new();
    let mut shift_gate = Cdc6600GateLevel::new();
    shift_functional.load(&shift_program).unwrap();
    shift_gate.load(&shift_program).unwrap();
    for amount in 0..64_u32 {
        shift_functional.set_program_counter(0).unwrap();
        shift_gate.set_program_counter(0).unwrap();
        shift_functional.write_x(1, 0x0fed_cba9_8765_4321).unwrap();
        shift_gate.write_x(1, 0x0fed_cba9_8765_4321).unwrap();
        shift_functional.write_b(1, amount).unwrap();
        shift_gate.write_b(1, amount).unwrap();
        assert_eq!(shift_gate.step().unwrap(), shift_functional.step().unwrap());
        assert_eq!(shift_gate.state(), shift_functional.state());
    }

    let mut multiply_program = Vec::new();
    short(&mut multiply_program, F_IXMUL, 2, 0, 1);
    let mut multiply_functional = Cdc6600Simulator::new();
    let mut multiply_gate = Cdc6600GateLevel::new();
    multiply_functional.load(&multiply_program).unwrap();
    multiply_gate.load(&multiply_program).unwrap();
    let mut seed = 0x1234_5678_9abc_def0_u64;
    for _ in 0..32 {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let left = seed & MASK_60;
        seed = seed.rotate_left(17) ^ 0x0fed_cba9_8765_4321;
        let right = seed & MASK_60;
        multiply_functional.set_program_counter(0).unwrap();
        multiply_gate.set_program_counter(0).unwrap();
        multiply_functional.write_x(0, left).unwrap();
        multiply_gate.write_x(0, left).unwrap();
        multiply_functional.write_x(1, right).unwrap();
        multiply_gate.write_x(1, right).unwrap();
        assert_eq!(
            multiply_gate.step().unwrap(),
            multiply_functional.step().unwrap()
        );
        assert_eq!(multiply_gate.state(), multiply_functional.state());
    }
}

#[test]
fn long_data_memory_and_immediate_paths_match() {
    let mut program = Vec::new();
    long(&mut program, F_LDAI, 1, 0, 200);
    long(&mut program, F_LDXI, 2, 0, 777);
    long(&mut program, F_STX, 1, 2, 2);
    long(&mut program, F_LDX, 3, 1, 2);
    long(&mut program, F_LDBI, 4, 0, 55);
    long(&mut program, F_STB, 1, 4, 3);
    long(&mut program, F_LDB, 5, 1, 3);
    halt(&mut program);
    let mut functional = Cdc6600Simulator::new();
    let mut gate = Cdc6600GateLevel::new();
    functional.load(&program).unwrap();
    gate.load(&program).unwrap();
    assert_lockstep(&mut functional, &mut gate, 10);
}

#[test]
fn all_conditional_branch_paths_match() {
    let mut program = Vec::new();
    long(&mut program, F_JEQ, 0, 1, 4);
    long(&mut program, F_LDXI, 1, 0, 99);
    long(&mut program, F_LDBI, 2, 0, 1);
    long(&mut program, F_JNE, 0, 2, 10);
    long(&mut program, F_LDXI, 3, 0, 99);
    long(&mut program, F_JXZ, 0, 4, 14);
    long(&mut program, F_LDXI, 5, 0, 99);
    long(&mut program, F_LDXI, 4, 0, 7);
    long(&mut program, F_JXN, 0, 4, 20);
    long(&mut program, F_LDXI, 6, 0, 99);
    halt(&mut program);
    let mut functional = Cdc6600Simulator::new();
    let mut gate = Cdc6600GateLevel::new();
    functional.load(&program).unwrap();
    gate.load(&program).unwrap();
    assert_lockstep(&mut functional, &mut gate, 20);
}

#[test]
fn jump_call_return_and_program_workloads_match() {
    let mut calls = Vec::new();
    long(&mut calls, F_JMP, 0, 0, 4);
    long(&mut calls, F_LDXI, 1, 0, 99);
    long(&mut calls, F_LDXI, 1, 0, 21);
    long(&mut calls, F_JSR, 0, 0, 12);
    long(&mut calls, F_LDXI, 2, 0, 100);
    halt(&mut calls);
    halt(&mut calls);
    short(&mut calls, F_IXXP, 1, 1, 1);
    long(&mut calls, F_RET, 0, 7, 0);
    halt(&mut calls);
    let mut functional = Cdc6600Simulator::new();
    let mut gate = Cdc6600GateLevel::new();
    functional.load(&calls).unwrap();
    gate.load(&calls).unwrap();
    assert_lockstep(&mut functional, &mut gate, 20);

    let mut factorial = Vec::new();
    long(&mut factorial, F_LDXI, 1, 0, 1);
    long(&mut factorial, F_LDBI, 1, 0, 5);
    long(&mut factorial, F_LDBI, 2, 0, 1);
    short(&mut factorial, F_TXB, 2, 1, 0);
    short(&mut factorial, F_IXMUL, 1, 1, 2);
    short(&mut factorial, F_IBBM, 1, 1, 2);
    long(&mut factorial, F_JNE, 0, 1, 6);
    halt(&mut factorial);
    let functional_result = Cdc6600Simulator::new().execute(&factorial, 100).unwrap();
    let gate_result = Cdc6600GateLevel::new().execute(&factorial, 100).unwrap();
    assert_eq!(gate_result, functional_result);
    assert_eq!(gate_result.final_state.x[1], 120);
}

#[test]
fn malformed_transport_and_public_bounds_are_atomic() {
    let mut functional = Cdc6600Simulator::new();
    let mut gate = Cdc6600GateLevel::new();
    functional.write_x(1, 99).unwrap();
    gate.write_x(1, 99).unwrap();
    let before = gate.state();
    assert_eq!(gate.load(&[1]), functional.load(&[1]));
    assert_eq!(gate.state(), before);
    assert_eq!(gate.load(&[0x80, 0]), functional.load(&[0x80, 0]));
    assert_eq!(gate.state(), before);
    let too_many = vec![0_u16; MEMORY_PARCELS as usize + 1];
    assert_eq!(
        gate.load_parcels(&too_many),
        functional.load_parcels(&too_many)
    );
    assert_eq!(gate.state(), before);
    assert_eq!(
        gate.write_x(8, 0),
        Err(Cdc6600Error::InvalidRegister {
            bank: 'X',
            index: 8
        })
    );
    assert_eq!(
        gate.write_a(8, 0),
        Err(Cdc6600Error::InvalidRegister {
            bank: 'A',
            index: 8
        })
    );
    assert_eq!(
        gate.write_b(8, 0),
        Err(Cdc6600Error::InvalidRegister {
            bank: 'B',
            index: 8
        })
    );
    assert!(matches!(
        gate.read_word(MEMORY_WORDS),
        Err(Cdc6600Error::MemoryAddressOutOfRange { .. })
    ));
}

#[test]
fn decode_fetch_branch_and_memory_errors_preserve_complete_state() {
    for instruction in [
        short_instr(23, 0, 0, 0).to_vec(),
        long_instr(63, 0, 0, 0).to_vec(),
    ] {
        let mut functional = Cdc6600Simulator::new();
        let mut gate = Cdc6600GateLevel::new();
        functional.load(&instruction).unwrap();
        gate.load(&instruction).unwrap();
        let before = gate.state();
        assert_eq!(gate.step(), functional.step());
        assert_eq!(gate.state(), before);
        assert_eq!(gate.state(), functional.state());
    }

    let invalid_branch = long_instr(F_JMP, 0, 0, MEMORY_PARCELS);
    let mut functional = Cdc6600Simulator::new();
    let mut gate = Cdc6600GateLevel::new();
    functional.load(&invalid_branch).unwrap();
    gate.load(&invalid_branch).unwrap();
    let before = gate.state();
    assert_eq!(gate.step(), functional.step());
    assert_eq!(gate.state(), before);

    let mut memory_program = Vec::new();
    long(&mut memory_program, F_LDAI, 1, 0, MEMORY_WORDS as u32);
    long(&mut memory_program, F_STX, 1, 2, 0);
    let mut functional = Cdc6600Simulator::new();
    let mut gate = Cdc6600GateLevel::new();
    functional.load(&memory_program).unwrap();
    gate.load(&memory_program).unwrap();
    assert_eq!(gate.step().unwrap(), functional.step().unwrap());
    let before = gate.state();
    assert_eq!(gate.step(), functional.step());
    assert_eq!(gate.state(), before);
}

#[test]
fn final_parcel_preflight_and_step_bounds_match() {
    let mut parcels = vec![0_u16; MEMORY_PARCELS as usize];
    for instruction in [short_instr(F_LDXI, 1, 0, 0), short_instr(F_TXB, 1, 0, 0)] {
        parcels[MEMORY_PARCELS as usize - 1] = u16::from_be_bytes(instruction);
        let mut functional = Cdc6600Simulator::new();
        let mut gate = Cdc6600GateLevel::new();
        functional.load_parcels(&parcels).unwrap();
        gate.load_parcels(&parcels).unwrap();
        functional.set_program_counter(MEMORY_PARCELS - 1).unwrap();
        gate.set_program_counter(MEMORY_PARCELS - 1).unwrap();
        let before = gate.state();
        assert_eq!(gate.step(), functional.step());
        assert_eq!(gate.state(), before);
    }

    let loop_program = long_instr(F_JMP, 0, 0, 0);
    let mut functional = Cdc6600Simulator::new();
    let mut gate = Cdc6600GateLevel::new();
    functional.load(&loop_program).unwrap();
    gate.load(&loop_program).unwrap();
    assert_eq!(gate.run(7), functional.run(7));
    assert_eq!(gate.state(), functional.state());
}
