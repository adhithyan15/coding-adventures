use cdc6600_simulator::*;

fn short(program: &mut Vec<u8>, opcode: u8, i: u8, j: u8, k: u8) {
    program.extend_from_slice(&short_instr(opcode, i, j, k));
}

fn long(program: &mut Vec<u8>, opcode: u8, i: u8, j: u8, constant: u32) {
    program.extend_from_slice(&long_instr(opcode, i, j, constant));
}

fn halt(program: &mut Vec<u8>) {
    program.extend_from_slice(&HALT);
}

fn execute(program: &[u8]) -> ExecutionResult {
    Cdc6600Simulator::new()
        .execute(program, 10_000)
        .expect("test program executes")
}

#[test]
fn reset_load_pack_and_snapshot_are_deterministic() {
    let mut program = Vec::new();
    long(&mut program, F_LDXI, 1, 0, 7);
    halt(&mut program);

    let mut simulator = Cdc6600Simulator::new();
    assert_eq!(simulator.load(&program).unwrap(), 3);
    let before = simulator.state();
    assert_ne!(before.memory[0], 0);
    let trace = simulator.step().unwrap();
    assert_eq!((trace.pc_before, trace.pc_after, trace.parcels), (0, 2, 2));
    let first = u32::from(u16::from_be_bytes([program[0], program[1]]));
    let second = u32::from(u16::from_be_bytes([program[2], program[3]]));
    assert_eq!(trace.instruction, (first << 15) | second);
    assert!(trace.mnemonic.starts_with("LDXI"));
    assert_eq!(before.x[1], 0);
    assert_eq!(simulator.state().x[1], 7);
    simulator.reset();
    assert_eq!(simulator.state(), Cdc6600Simulator::new().state());
}

#[test]
fn load_validation_is_fail_closed() {
    let mut simulator = Cdc6600Simulator::new();
    simulator.write_x(1, 99).unwrap();
    let before = simulator.state();
    assert_eq!(
        simulator.load(&[1]),
        Err(Cdc6600Error::InvalidProgramLength { bytes: 1 })
    );
    assert_eq!(simulator.state(), before);
    assert_eq!(
        simulator.load(&[0x80, 0]),
        Err(Cdc6600Error::NonCanonicalParcel {
            index: 0,
            value: 0x8000,
        })
    );
    assert_eq!(simulator.state(), before);

    let oversized = vec![0_u16; MEMORY_PARCELS as usize + 1];
    assert!(matches!(
        simulator.load_parcels(&oversized),
        Err(Cdc6600Error::ProgramTooLarge { .. })
    ));
    assert_eq!(simulator.state(), before);

    let oversized_transport = vec![0_u8; (MEMORY_PARCELS as usize + 1) * 2];
    assert_eq!(
        simulator.load(&oversized_transport),
        Err(Cdc6600Error::ProgramTooLarge {
            parcels: MEMORY_PARCELS as usize + 1,
            capacity: MEMORY_PARCELS as usize,
        })
    );
    assert_eq!(simulator.state(), before);
}

#[test]
fn halt_and_already_halted_steps_are_stable() {
    let mut simulator = Cdc6600Simulator::new();
    let result = simulator.execute(&HALT, 1).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 1);
    assert_eq!(result.final_state.p, 0);
    let trace = simulator.step().unwrap();
    assert_eq!(trace.pc_before, trace.pc_after);
    assert_eq!(trace.mnemonic, "HALT");
}

#[test]
fn every_transfer_path_and_b0_invariant_match_the_oracle() {
    let mut program = Vec::new();
    short(&mut program, F_TXB, 1, 2, 0);
    short(&mut program, F_TBX, 3, 4, 0);
    short(&mut program, F_TAX, 5, 6, 0);
    short(&mut program, F_TXA, 7, 4, 0);
    short(&mut program, F_TBX, 0, 4, 0);
    halt(&mut program);
    let mut simulator = Cdc6600Simulator::new();
    simulator.load(&program).unwrap();
    simulator.write_b(2, MASK_18).unwrap();
    simulator.write_x(4, MASK_60).unwrap();
    simulator.write_a(6, 1234).unwrap();
    simulator.write_b(0, 999).unwrap();
    let state = simulator.run(20).unwrap().final_state;
    assert_eq!(state.x[1], u64::from(MASK_18));
    assert_eq!(state.b[3], MASK_18);
    assert_eq!(state.x[5], 1234);
    assert_eq!(state.a[7], MASK_18);
    assert_eq!(state.b[0], 0);
}

#[test]
fn x_arithmetic_wraps_at_sixty_bits() {
    let mut program = Vec::new();
    short(&mut program, F_IXPB, 0, 1, 2);
    short(&mut program, F_IXMB, 3, 1, 2);
    short(&mut program, F_IXXP, 4, 1, 5);
    short(&mut program, F_IXXM, 6, 5, 1);
    halt(&mut program);
    let mut simulator = Cdc6600Simulator::new();
    simulator.load(&program).unwrap();
    simulator.write_x(1, 10).unwrap();
    simulator.write_x(5, 20).unwrap();
    simulator.write_b(2, 3).unwrap();
    let state = simulator.run(20).unwrap().final_state;
    assert_eq!(
        (state.x[0], state.x[3], state.x[4], state.x[6]),
        (13, 7, 30, 10)
    );

    let mut wrap = Cdc6600Simulator::new();
    let mut wrap_program = Vec::new();
    short(&mut wrap_program, F_IXXP, 1, 1, 2);
    short(&mut wrap_program, F_IXXM, 3, 0, 2);
    halt(&mut wrap_program);
    wrap.load(&wrap_program).unwrap();
    wrap.write_x(1, MASK_60).unwrap();
    wrap.write_x(2, 1).unwrap();
    let state = wrap.run(10).unwrap().final_state;
    assert_eq!(state.x[1], 0);
    assert_eq!(state.x[3], MASK_60);
}

#[test]
fn boolean_shift_and_widened_multiply_cover_all_datapaths() {
    let mut program = Vec::new();
    short(&mut program, F_BXND, 3, 1, 2);
    short(&mut program, F_BXOR, 4, 1, 2);
    short(&mut program, F_BXXR, 5, 1, 2);
    short(&mut program, F_BXMR, 6, 1, 0);
    short(&mut program, F_LSHL, 7, 1, 3);
    short(&mut program, F_LSHR, 0, 2, 3);
    short(&mut program, F_IXMUL, 2, 1, 2);
    halt(&mut program);
    let mut simulator = Cdc6600Simulator::new();
    simulator.load(&program).unwrap();
    simulator.write_x(1, 0xf0f0).unwrap();
    simulator.write_x(2, MASK_60).unwrap();
    simulator.write_b(3, 4).unwrap();
    let state = simulator.run(20).unwrap().final_state;
    assert_eq!(state.x[3], 0xf0f0);
    assert_eq!(state.x[4], MASK_60);
    assert_eq!(state.x[5], MASK_60 ^ 0xf0f0);
    assert_eq!(state.x[6], !0xf0f0 & MASK_60);
    assert_eq!(state.x[7], 0xf0f0 << 4);
    assert_eq!(state.x[0], MASK_60 >> 4);
    assert_eq!(
        state.x[2],
        (u128::from(0xf0f0_u64) * u128::from(MASK_60)) as u64 & MASK_60
    );
}

#[test]
fn a_b_arithmetic_and_signed_compares_cover_all_register_rules() {
    let mut program = Vec::new();
    short(&mut program, F_IBBP, 1, 2, 3);
    short(&mut program, F_IBBM, 4, 2, 3);
    short(&mut program, F_IAAP, 1, 2, 3);
    short(&mut program, F_IAAM, 4, 2, 3);
    short(&mut program, F_CMPEQ, 5, 1, 1);
    short(&mut program, F_CMPLT, 6, 2, 1);
    short(&mut program, F_CMPGT, 7, 1, 2);
    halt(&mut program);
    let mut simulator = Cdc6600Simulator::new();
    simulator.load(&program).unwrap();
    simulator.write_b(2, MASK_18).unwrap();
    simulator.write_b(3, 1).unwrap();
    simulator.write_a(2, 10).unwrap();
    simulator.write_x(1, 7).unwrap();
    simulator.write_x(2, MASK_60).unwrap();
    let state = simulator.run(20).unwrap().final_state;
    assert_eq!(state.b[1], 0);
    assert_eq!(state.b[4], MASK_18 - 1);
    assert_eq!(state.a[1], 11);
    assert_eq!(state.a[4], 9);
    assert_eq!((state.b[5], state.b[6], state.b[7]), (1, 1, 1));
    assert_eq!(signed_60(MASK_60), -1);
}

#[test]
fn immediates_and_memory_round_trip_all_long_data_paths() {
    let mut program = Vec::new();
    long(&mut program, F_LDAI, 1, 0, 100);
    long(&mut program, F_LDXI, 2, 0, 777);
    long(&mut program, F_STX, 1, 2, 2);
    long(&mut program, F_LDX, 3, 1, 2);
    long(&mut program, F_LDBI, 4, 0, 55);
    long(&mut program, F_STB, 1, 4, 3);
    long(&mut program, F_LDB, 5, 1, 3);
    halt(&mut program);
    let state = execute(&program).final_state;
    assert_eq!(state.a[1], 100);
    assert_eq!((state.x[2], state.x[3]), (777, 777));
    assert_eq!((state.b[4], state.b[5]), (55, 55));
    assert_eq!(state.memory[102], 777);
    assert_eq!(state.memory[103], 55);
}

#[test]
fn memory_errors_are_atomic() {
    let mut program = Vec::new();
    long(&mut program, F_LDAI, 1, 0, MEMORY_WORDS as u32);
    long(&mut program, F_STX, 1, 2, 0);
    halt(&mut program);
    let mut simulator = Cdc6600Simulator::new();
    simulator.load(&program).unwrap();
    simulator.step().unwrap();
    let before = simulator.state();
    assert_eq!(
        simulator.step(),
        Err(Cdc6600Error::MemoryAddressOutOfRange {
            address: MEMORY_WORDS as u32,
        })
    );
    assert_eq!(simulator.state(), before);
    assert_eq!(
        simulator.read_word(MEMORY_WORDS),
        Err(Cdc6600Error::MemoryAddressOutOfRange {
            address: MEMORY_WORDS as u32,
        })
    );
}

#[test]
fn b_and_x_conditional_branches_match_taken_and_fallthrough_paths() {
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
    let state = execute(&program).final_state;
    assert_eq!(state.x[1], 0);
    assert_eq!(state.b[2], 1);
    assert_eq!(state.x[3], 0);
    assert_eq!(state.x[5], 0);
    assert_eq!(state.x[4], 7);
    assert_eq!(state.x[6], 0);
}

#[test]
fn jump_call_and_return_preserve_parcel_addresses() {
    let mut program = Vec::new();
    long(&mut program, F_JMP, 0, 0, 4);
    long(&mut program, F_LDXI, 1, 0, 99);
    long(&mut program, F_LDXI, 1, 0, 21);
    long(&mut program, F_JSR, 0, 0, 12);
    long(&mut program, F_LDXI, 2, 0, 100);
    halt(&mut program);
    halt(&mut program);
    short(&mut program, F_IXXP, 1, 1, 1);
    long(&mut program, F_RET, 0, 7, 0);
    halt(&mut program);
    let result = execute(&program);
    assert_eq!(result.final_state.x[1], 42);
    assert_eq!(result.final_state.x[2], 100);
    assert_eq!(result.final_state.b[7], 8);
    assert!(result
        .traces
        .iter()
        .any(|trace| trace.mnemonic.starts_with("JSR")));
    assert!(result
        .traces
        .iter()
        .any(|trace| trace.mnemonic.starts_with("RET")));
}

#[test]
fn invalid_fetch_opcode_and_branch_paths_leave_state_unchanged() {
    let mut invalid_short = Cdc6600Simulator::new();
    let mut program = Vec::new();
    short(&mut program, 23, 0, 0, 0);
    halt(&mut program);
    invalid_short.load(&program).unwrap();
    let before = invalid_short.state();
    assert!(matches!(
        invalid_short.step(),
        Err(Cdc6600Error::UnknownShortOpcode { opcode: 23, pc: 0 })
    ));
    assert_eq!(invalid_short.state(), before);

    let mut invalid_long = Cdc6600Simulator::new();
    invalid_long.load(&long_instr(63, 0, 0, 0)).unwrap();
    let before = invalid_long.state();
    assert!(matches!(
        invalid_long.step(),
        Err(Cdc6600Error::UnknownLongOpcode { opcode: 63, pc: 0 })
    ));
    assert_eq!(invalid_long.state(), before);

    let mut invalid_branch = Cdc6600Simulator::new();
    invalid_branch
        .load(&long_instr(F_JMP, 0, 0, MEMORY_PARCELS))
        .unwrap();
    let before = invalid_branch.state();
    assert!(matches!(
        invalid_branch.step(),
        Err(Cdc6600Error::ProgramCounterOutOfRange { .. })
    ));
    assert_eq!(invalid_branch.state(), before);
}

#[test]
fn end_of_memory_fetch_and_fallthrough_are_checked_before_mutation() {
    let mut parcels = vec![0_u16; MEMORY_PARCELS as usize];
    parcels[MEMORY_PARCELS as usize - 1] = u16::from_be_bytes(short_instr(F_LDXI, 1, 0, 0));
    let mut missing = Cdc6600Simulator::new();
    missing.load_parcels(&parcels).unwrap();
    missing.set_program_counter(MEMORY_PARCELS - 1).unwrap();
    let before = missing.state();
    assert_eq!(
        missing.step(),
        Err(Cdc6600Error::MissingLongParcel {
            pc: MEMORY_PARCELS - 1,
        })
    );
    assert_eq!(missing.state(), before);

    parcels[MEMORY_PARCELS as usize - 1] = u16::from_be_bytes(short_instr(F_TXB, 1, 0, 0));
    let mut fallthrough = Cdc6600Simulator::new();
    fallthrough.load_parcels(&parcels).unwrap();
    fallthrough.set_program_counter(MEMORY_PARCELS - 1).unwrap();
    let before = fallthrough.state();
    assert!(matches!(
        fallthrough.step(),
        Err(Cdc6600Error::ProgramCounterOutOfRange { .. })
    ));
    assert_eq!(fallthrough.state(), before);
}

#[test]
fn bounded_execution_does_not_allocate_from_the_limit() {
    let mut simulator = Cdc6600Simulator::new();
    simulator.load(&long_instr(63, 0, 0, 0)).unwrap();
    assert!(matches!(
        simulator.run(usize::MAX),
        Err(Cdc6600Error::UnknownLongOpcode { .. })
    ));

    let mut loop_program = Cdc6600Simulator::new();
    loop_program.load(&long_instr(F_JMP, 0, 0, 0)).unwrap();
    assert_eq!(
        loop_program.run(7),
        Err(Cdc6600Error::MaxStepsExceeded { max_steps: 7 })
    );
    assert_eq!(loop_program.state().p, 0);
}

#[test]
fn sum_and_factorial_programs_match_python_oracle_vectors() {
    let mut sum = Vec::new();
    long(&mut sum, F_LDXI, 1, 0, 0);
    long(&mut sum, F_LDBI, 1, 0, 10);
    long(&mut sum, F_LDBI, 2, 0, 1);
    short(&mut sum, F_TXB, 3, 1, 0);
    short(&mut sum, F_IXXP, 1, 1, 3);
    short(&mut sum, F_IBBM, 1, 1, 2);
    long(&mut sum, F_JNE, 0, 1, 6);
    halt(&mut sum);
    assert_eq!(execute(&sum).final_state.x[1], 55);

    let mut factorial = Vec::new();
    long(&mut factorial, F_LDXI, 1, 0, 1);
    long(&mut factorial, F_LDBI, 1, 0, 5);
    long(&mut factorial, F_LDBI, 2, 0, 1);
    short(&mut factorial, F_TXB, 2, 1, 0);
    short(&mut factorial, F_IXMUL, 1, 1, 2);
    short(&mut factorial, F_IBBM, 1, 1, 2);
    long(&mut factorial, F_JNE, 0, 1, 6);
    halt(&mut factorial);
    assert_eq!(execute(&factorial).final_state.x[1], 120);
}

#[test]
fn seeded_register_trace_matches_independent_width_model() {
    let mut program = Vec::new();
    let mut expected_x1 = 0x0123_4567_89ab_u64;
    let mut expected_x2 = 0x00fe_dcba_9876_u64;
    let b3 = 7_u32;
    for index in 0..96 {
        match index % 4 {
            0 => {
                short(&mut program, F_IXXP, 1, 1, 2);
                expected_x1 = expected_x1.wrapping_add(expected_x2) & MASK_60;
            }
            1 => {
                short(&mut program, F_BXXR, 2, 2, 1);
                expected_x2 ^= expected_x1;
            }
            2 => {
                short(&mut program, F_IXPB, 1, 1, 3);
                expected_x1 = expected_x1.wrapping_add(u64::from(b3)) & MASK_60;
            }
            _ => {
                short(&mut program, F_LSHR, 2, 2, 3);
                expected_x2 >>= b3 & 63;
            }
        }
    }
    halt(&mut program);
    let mut simulator = Cdc6600Simulator::new();
    simulator.load(&program).unwrap();
    simulator.write_x(1, 0x0123_4567_89ab).unwrap();
    simulator.write_x(2, 0x00fe_dcba_9876).unwrap();
    simulator.write_b(3, b3).unwrap();
    let result = simulator.run(200).unwrap();
    assert_eq!(result.final_state.x[1], expected_x1);
    assert_eq!(result.final_state.x[2], expected_x2);
    assert_eq!(result.steps, 97);
}

#[test]
fn public_mutators_mask_widths_and_validate_indices() {
    let mut simulator = Cdc6600Simulator::new();
    simulator.write_x(0, u64::MAX).unwrap();
    simulator.write_a(0, u32::MAX).unwrap();
    simulator.write_b(1, u32::MAX).unwrap();
    simulator.write_word(0, u64::MAX).unwrap();
    let state = simulator.state();
    assert_eq!(state.x[0], MASK_60);
    assert_eq!(state.a[0], MASK_18);
    assert_eq!(state.b[1], MASK_18);
    assert_eq!(state.memory[0], MASK_60);
    assert!(matches!(
        simulator.write_x(8, 0),
        Err(Cdc6600Error::InvalidRegister {
            bank: 'X',
            index: 8
        })
    ));
    assert!(matches!(
        simulator.write_a(8, 0),
        Err(Cdc6600Error::InvalidRegister {
            bank: 'A',
            index: 8
        })
    ));
    assert!(matches!(
        simulator.write_b(8, 0),
        Err(Cdc6600Error::InvalidRegister {
            bank: 'B',
            index: 8
        })
    ));
}
