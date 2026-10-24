use motorola6800_simulator::{
    instruction_width, Motorola6800Error, Motorola6800Simulator, CC_C, CC_H, CC_I, CC_N, CC_V,
    CC_Z, MEMORY_SIZE,
};

fn image(parts: &[&[u8]]) -> Vec<u8> {
    parts.iter().flat_map(|part| part.iter().copied()).collect()
}

#[test]
fn all_first_bytes_have_the_documented_classification_and_width() {
    let invalid = [
        0x00, 0x02, 0x03, 0x04, 0x05, 0x12, 0x13, 0x14, 0x15, 0x18, 0x1a, 0x1c, 0x1d, 0x1e, 0x1f,
        0x21, 0x38, 0x3a, 0x3c, 0x3d, 0x41, 0x42, 0x45, 0x4b, 0x4e, 0x51, 0x52, 0x55, 0x5b, 0x5e,
        0x61, 0x62, 0x65, 0x6b, 0x71, 0x72, 0x75, 0x7b, 0x83, 0x87, 0x8f, 0x93, 0xa3, 0xb3, 0xc3,
        0xc7, 0xcc, 0xcd, 0xcf, 0xd3, 0xdc, 0xdd, 0xe3, 0xec, 0xed, 0xf3, 0xfc, 0xfd,
    ];
    let mut valid_count = 0;
    for opcode in 0_u8..=u8::MAX {
        if invalid.contains(&opcode) {
            assert_eq!(instruction_width(opcode), None, "opcode {opcode:#04x}");
        } else {
            let width = instruction_width(opcode).expect("documented opcode");
            assert!((1..=3).contains(&width));
            valid_count += 1;
        }
    }
    assert_eq!(valid_count, 198);
}

#[test]
fn every_documented_encoding_executes_from_a_nontrivial_state() {
    for opcode in 0_u8..=u8::MAX {
        let Some(width) = instruction_width(opcode) else {
            continue;
        };
        let mut bytes = vec![opcode, 0, 0];
        bytes.truncate(width as usize);
        let mut cpu = Motorola6800Simulator::new();
        cpu.load(&bytes, 0).unwrap();
        cpu.write_a(0x12);
        cpu.write_b(0x34);
        cpu.write_x(0x0200);
        cpu.write_sp(0x0300);
        cpu.write_memory(0x0200, &[0x55, 0xaa]).unwrap();
        cpu.write_memory(0xfffa, &[0, 0]).unwrap();
        cpu.step()
            .unwrap_or_else(|error| panic!("opcode {opcode:#04x} failed: {error}"));
    }
}

#[test]
fn load_reset_restore_and_checked_access_cover_complete_state() {
    let mut cpu = Motorola6800Simulator::new();
    cpu.load(&[0x01, 0x3e], 0x1000).unwrap();
    cpu.write_a(0x12);
    cpu.write_b(0x34);
    cpu.write_x(0x4567);
    cpu.write_sp(0x89ab);
    cpu.write_cc(CC_H | CC_I | CC_N | CC_Z | CC_V | CC_C);
    cpu.write_memory(0x2000, &[1, 2, 3]).unwrap();
    let state = cpu.get_state();
    assert_eq!(state.pc, 0x1000);
    assert_eq!(state.loaded_origin, 0x1000);
    assert_eq!(state.loaded_len, 2);
    assert_eq!(state.memory.len(), MEMORY_SIZE);
    assert_eq!(cpu.read_memory(0x2000, 3).unwrap(), vec![1, 2, 3]);

    cpu.reset();
    assert_eq!(cpu.get_state().a, 0);
    assert_eq!(cpu.read_memory(0x2000, 3).unwrap(), vec![0, 0, 0]);
    cpu.restore(state.clone()).unwrap();
    assert_eq!(cpu.get_state(), state);

    let before = cpu.get_state();
    assert!(cpu.write_memory(0xffff, &[1, 2]).is_err());
    assert_eq!(cpu.get_state(), before);
    assert!(Motorola6800Simulator::new().load(&[], 0).is_err());
    assert!(Motorola6800Simulator::new().load(&[0; 2], 0xffff).is_err());
}

#[test]
fn immediate_direct_indexed_and_extended_data_paths_are_big_endian() {
    let program = image(&[
        &[0x86, 0x7f],       // LDAA #$7f
        &[0x97, 0x20],       // STAA $20
        &[0xd6, 0x20],       // LDAB $20
        &[0xce, 0x12, 0x34], // LDX #$1234
        &[0xff, 0x20, 0x00], // STX $2000
        &[0xa6, 0x10],       // LDAA $10,X
        &[0xb7, 0x20, 0x02], // STAA $2002
        &[0x3e],             // WAI
    ]);
    let mut cpu = Motorola6800Simulator::new();
    cpu.load(&program, 0).unwrap();
    cpu.write_memory(0x1244, &[0xa5]).unwrap();
    cpu.run(32).unwrap();
    let state = cpu.get_state();
    assert_eq!(state.b, 0x7f);
    assert_eq!(state.x, 0x1234);
    assert_eq!(cpu.read_memory(0x2000, 3).unwrap(), vec![0x12, 0x34, 0xa5]);
}

#[test]
fn arithmetic_logic_shift_and_decimal_adjust_follow_condition_rules() {
    let program = image(&[
        &[0x86, 0x09], // LDAA #9
        &[0x8b, 0x09], // ADDA #9
        &[0x19],       // DAA -> 0x18
        &[0x16],       // TAB
        &[0x48],       // ASLA -> 0x30
        &[0x49],       // ROLA, carry participates
        &[0x43],       // COMA
        &[0x4a],       // DECA
        &[0x4c],       // INCA
        &[0x84, 0x0f], // ANDA #$0f
        &[0x88, 0x03], // EORA #3
        &[0x8a, 0x80], // ORAA #$80
        &[0x3e],
    ]);
    let mut cpu = Motorola6800Simulator::new();
    cpu.load(&program, 0).unwrap();
    cpu.run(32).unwrap();
    let state = cpu.get_state();
    assert_eq!(state.b, 0x18);
    assert_ne!(state.cc & CC_N, 0);
    assert_eq!(state.cc & CC_Z, 0);

    let mut memory = Motorola6800Simulator::new();
    memory
        .load(
            &image(&[&[0x60, 0], &[0x63, 0], &[0x6c, 0], &[0x6d, 0], &[0x3e]]),
            0,
        )
        .unwrap();
    memory.write_x(0x2000);
    memory.write_memory(0x2000, &[1]).unwrap();
    memory.run(16).unwrap();
    assert_eq!(memory.read_memory(0x2000, 1).unwrap(), vec![1]);
}

#[test]
fn every_conditional_branch_observes_the_documented_predicate() {
    let cases = [
        (0x22, 0, CC_C),
        (0x23, CC_C, 0),
        (0x24, 0, CC_C),
        (0x25, CC_C, 0),
        (0x26, 0, CC_Z),
        (0x27, CC_Z, 0),
        (0x28, 0, CC_V),
        (0x29, CC_V, 0),
        (0x2a, 0, CC_N),
        (0x2b, CC_N, 0),
        (0x2c, 0, CC_N),
        (0x2d, CC_N, 0),
        (0x2e, 0, CC_Z),
        (0x2f, CC_Z, 0),
    ];
    for (opcode, taken_cc, not_taken_cc) in cases {
        let mut cpu = Motorola6800Simulator::new();
        cpu.load(&[opcode, 2, 0x01, 0x01, 0x3e], 0).unwrap();
        cpu.write_cc(taken_cc);
        let trace = cpu.step().unwrap();
        assert_eq!(trace.pc_after, 4, "opcode {opcode:#04x}");

        let mut opposite = Motorola6800Simulator::new();
        opposite.load(&[opcode, 2, 0x01, 0x01, 0x3e], 0).unwrap();
        opposite.write_cc(not_taken_cc);
        assert_eq!(opposite.step().unwrap().pc_after, 2);
    }
}

#[test]
fn stack_subroutine_software_interrupt_and_return_paths_round_trip() {
    let mut call = Motorola6800Simulator::new();
    call.load(&[0xbd, 0, 4, 0x3e, 0x86, 42, 0x39], 0).unwrap();
    call.write_sp(0x0200);
    call.run(16).unwrap();
    assert_eq!(call.get_state().a, 42);

    let mut interrupt = Motorola6800Simulator::new();
    interrupt.load(&[0x3f, 0x3e, 0x01, 0x3b], 0).unwrap();
    interrupt.write_sp(0x0300);
    interrupt.write_a(0x12);
    interrupt.write_b(0x34);
    interrupt.write_x(0x5678);
    interrupt.write_memory(0xfffa, &[0, 3]).unwrap();
    interrupt.run(16).unwrap();
    let state = interrupt.get_state();
    assert_eq!((state.a, state.b, state.x), (0x12, 0x34, 0x5678));
    assert!(state.halted);
}

#[test]
fn transfer_stack_and_flag_orders_preserve_unrelated_state() {
    let mut cpu = Motorola6800Simulator::new();
    cpu.load(
        &[
            0x06, 0x07, 0x16, 0x17, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x36, 0x37, 0x32, 0x33,
            0x30, 0x35, 0x31, 0x34, 0x3e,
        ],
        0,
    )
    .unwrap();
    cpu.write_a(0x55);
    cpu.write_b(0xaa);
    cpu.write_x(0x1000);
    cpu.write_sp(0x0200);
    cpu.run(64).unwrap();
    assert!(cpu.get_state().halted);
}

#[test]
fn traces_are_exact_and_faults_are_atomic_and_transactional() {
    let mut cpu = Motorola6800Simulator::new();
    cpu.load(&[0x86, 0x2a, 0x3e], 0x100).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.pc_before, 0x100);
    assert_eq!(trace.pc_after, 0x102);
    assert_eq!(trace.raw, vec![0x86, 0x2a]);
    assert_eq!(trace.width, 2);
    assert_eq!(trace.mnemonic, "LDAA");
    assert_eq!(trace.state_after.a, 42);

    let mut unknown = Motorola6800Simulator::new();
    unknown.load(&[0x00], 0).unwrap();
    let before = unknown.get_state();
    assert_eq!(
        unknown.step(),
        Err(Motorola6800Error::UnknownOpcode { opcode: 0, pc: 0 })
    );
    assert_eq!(unknown.get_state(), before);

    let mut truncated = Motorola6800Simulator::new();
    truncated.load(&[0x86], 0).unwrap();
    let before = truncated.get_state();
    assert!(matches!(
        truncated.step(),
        Err(Motorola6800Error::TruncatedInstruction { .. })
    ));
    assert_eq!(truncated.get_state(), before);

    let mut bounded = Motorola6800Simulator::new();
    bounded.load(&[0x01, 0x01, 0x3e], 0).unwrap();
    let before = bounded.get_state();
    assert_eq!(
        bounded.run(1),
        Err(Motorola6800Error::StepLimitExceeded { limit: 1 })
    );
    assert_eq!(bounded.get_state(), before);
}

#[test]
fn error_messages_and_execute_result_are_stable() {
    let errors = [
        Motorola6800Error::EmptyProgram,
        Motorola6800Error::ProgramOutOfRange {
            origin: 0xffff,
            len: 2,
        },
        Motorola6800Error::FetchOutsideProgram { pc: 4 },
        Motorola6800Error::TruncatedInstruction { pc: 0, needed: 2 },
        Motorola6800Error::UnknownOpcode { opcode: 0, pc: 0 },
        Motorola6800Error::InvalidState {
            reason: "bad memory",
        },
        Motorola6800Error::Halted,
        Motorola6800Error::StepLimitExceeded { limit: 1 },
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }

    let result = Motorola6800Simulator::new()
        .execute(&[0x86, 42, 0x3e], 0x200, 8)
        .unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.final_state.a, 42);
}

#[test]
fn public_accessors_validation_and_failure_rollbacks_are_covered() {
    let mut cpu = Motorola6800Simulator::new();
    cpu.load(&[0x01, 0x3e], 0x100).unwrap();
    cpu.write_a(1);
    cpu.write_b(2);
    cpu.write_x(3);
    cpu.write_sp(4);
    cpu.write_cc(5);
    assert_eq!(
        (cpu.read_a(), cpu.read_b(), cpu.read_x(), cpu.read_sp()),
        (1, 2, 3, 4)
    );
    assert_eq!(cpu.read_cc() & 0x3f, 5);
    assert!(cpu.read_memory(0xffff, 2).is_err());

    let mut bad_memory = cpu.get_state();
    bad_memory.memory.pop();
    assert!(matches!(
        cpu.restore(bad_memory),
        Err(Motorola6800Error::InvalidState { .. })
    ));
    let mut bad_range = cpu.get_state();
    bad_range.loaded_len = 0;
    assert!(matches!(
        cpu.restore(bad_range),
        Err(Motorola6800Error::InvalidState { .. })
    ));

    cpu.write_cc(0);
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().pc, 0x101);
    let result = cpu.run(4).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 1);
}

#[test]
fn post_halt_fetch_and_transaction_failures_preserve_state() {
    let mut halted = Motorola6800Simulator::new();
    halted.load(&[0x3e], 0).unwrap();
    assert_eq!(halted.step().unwrap().mnemonic, "WAI");
    let before = halted.get_state();
    assert_eq!(halted.step(), Err(Motorola6800Error::Halted));
    assert_eq!(halted.get_state(), before);

    let mut outside = Motorola6800Simulator::new();
    outside.load(&[0x01], 0).unwrap();
    outside.step().unwrap();
    let before = outside.get_state();
    assert_eq!(
        outside.step(),
        Err(Motorola6800Error::FetchOutsideProgram { pc: 1 })
    );
    assert_eq!(outside.get_state(), before);
    assert_eq!(
        outside.run(2),
        Err(Motorola6800Error::FetchOutsideProgram { pc: 1 })
    );
    assert_eq!(outside.get_state(), before);

    let mut executing = Motorola6800Simulator::new();
    executing.load(&[0x3e], 0x200).unwrap();
    let original = executing.get_state();
    assert_eq!(
        executing.execute(&[0x01], 0, 1),
        Err(Motorola6800Error::StepLimitExceeded { limit: 1 })
    );
    assert_eq!(executing.get_state(), original);

    let mut memory = Motorola6800Simulator::new();
    memory.load(&[0x60, 0], 0).unwrap();
    memory.write_x(0x2000);
    memory.write_memory(0x2000, &[1]).unwrap();
    assert_eq!(memory.step().unwrap().mnemonic, "NEG");
}
