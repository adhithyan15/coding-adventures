use arm_simulator::functional::{
    assemble_words, encode_branch, encode_data_immediate, encode_data_register, encode_transfer,
    ArmError, Armv7Simulator, Condition, DataOpcode, HALT_WORD, MEMORY_SIZE,
};

#[test]
fn exact_state_restore_and_register_access_are_checked_atomically() {
    let mut cpu = Armv7Simulator::new();
    assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);
    assert_eq!(cpu.get_state().registers[15], cpu.get_state().pc);
    let before = cpu.get_state();

    let mut invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(ArmError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    invalid = before.clone();
    invalid.registers[15] = 4;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(ArmError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    invalid = before.clone();
    invalid.loaded_origin = (MEMORY_SIZE - 2) as u32;
    invalid.loaded_len = 4;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(ArmError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    assert_eq!(
        cpu.read_register_checked(16),
        Err(ArmError::InvalidRegister { index: 16 })
    );
    cpu.write_register_checked(15, 0x100).unwrap();
    assert_eq!(cpu.get_state().pc, 0x100);
    assert_eq!(cpu.get_state().registers[15], 0x100);
}

#[test]
fn origin_loading_and_little_endian_direct_access_are_typed() {
    let mut cpu = Armv7Simulator::new();
    cpu.write_byte_checked(60, 0xaa).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.load_at_checked(&[0; 8], (MEMORY_SIZE - 4) as u32),
        Err(ArmError::ProgramOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.load_at_checked(&[0; 4], 1),
        Err(ArmError::MisalignedProgram { origin: 1 })
    );
    assert!(matches!(
        cpu.load_at_checked(&[], MEMORY_SIZE as u32),
        Err(ArmError::ProgramOutOfRange { .. })
    ));

    cpu.load_at_checked(&assemble_words(&[HALT_WORD]), 0x100)
        .unwrap();
    let state = cpu.get_state();
    assert_eq!(state.pc, 0x100);
    assert_eq!(state.loaded_origin, 0x100);
    assert_eq!(state.loaded_len, 4);
    assert_eq!(cpu.read_byte_checked(60).unwrap(), 0);

    cpu.write_word_checked(8, 0x0102_0304).unwrap();
    assert_eq!(cpu.read_word_checked(8).unwrap(), 0x0102_0304);
    assert_eq!(cpu.read_byte_checked(8).unwrap(), 4);
    assert!(matches!(
        cpu.read_word_checked(2),
        Err(ArmError::MisalignedAccess { .. })
    ));
    assert!(matches!(
        cpu.read_byte_checked(MEMORY_SIZE as u32),
        Err(ArmError::MemoryOutOfRange { .. })
    ));
}

#[test]
fn transition_faults_and_bounded_runs_roll_back() {
    let mut cpu = Armv7Simulator::new();
    cpu.load_checked(&[0, 0, 0]).unwrap();
    assert_eq!(
        cpu.step_checked(),
        Err(ArmError::TruncatedInstruction { pc: 0 })
    );

    for raw in [
        0xee00_0000,
        encode_transfer(Condition::Al, true, true, 0, 1, 2),
        encode_data_register(Condition::Al, DataOpcode::Add, false, 0, 1, 1) | (1 << 4),
    ] {
        cpu.load_checked(&assemble_words(&[raw])).unwrap();
        let before = cpu.get_state();
        assert!(cpu.step_checked().is_err());
        assert_eq!(cpu.get_state(), before);
    }

    cpu.load_at_checked(
        &assemble_words(&[encode_data_immediate(
            Condition::Al,
            DataOpcode::Mov,
            false,
            0,
            0,
            0,
            1,
        )]),
        (MEMORY_SIZE - 4) as u32,
    )
    .unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.step_checked(),
        Err(ArmError::MemoryOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&assemble_words(&[encode_branch(Condition::Al, false, -8)]))
        .unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_loaded_checked(3),
        Err(ArmError::StepLimitExceeded { max_steps: 3 })
    );
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn arithmetic_flags_drive_conditional_execution() {
    let words = [
        encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 0, 0, 1),
        encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 1, 0, 1),
        encode_data_register(Condition::Al, DataOpcode::Cmp, true, 0, 0, 1),
        encode_data_immediate(Condition::Eq, DataOpcode::Mov, false, 0, 2, 0, 42),
        encode_data_immediate(Condition::Ne, DataOpcode::Mov, false, 0, 3, 0, 99),
        HALT_WORD,
    ];
    let mut cpu = Armv7Simulator::new();
    let result = cpu.run_checked(&assemble_words(&words), 8).unwrap();
    assert_eq!(result.final_state.registers[2], 42);
    assert_eq!(result.final_state.registers[3], 0);
    assert!(result.final_state.flags.z);
    assert!(result.final_state.flags.c);
    assert!(!result.traces[4].condition_passed);
    assert_eq!(result.traces[4].mnemonic, "SKIP");
}

#[test]
fn full_traces_cover_data_memory_branch_and_halt() {
    let words = [
        encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 0, 0, 0x80),
        encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 1, 0, 42),
        encode_transfer(Condition::Al, false, true, 0, 1, 0),
        encode_transfer(Condition::Al, true, true, 0, 2, 0),
        encode_branch(Condition::Al, false, 0),
        encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 2, 0, 99),
        HALT_WORD,
    ];
    let mut cpu = Armv7Simulator::new();
    let result = cpu.run_checked(&assemble_words(&words), 8).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 6);
    assert_eq!(result.final_state.registers[2], 42);
    assert_eq!(&result.final_state.memory[0x80..0x84], &[42, 0, 0, 0]);
    assert_eq!(result.traces[2].mnemonic, "STR");
    assert_eq!(result.traces[2].state_before.memory[0x80], 0);
    assert_eq!(result.traces[2].state_after.memory[0x80], 42);
    assert_eq!(result.traces[4].pc_after, 24);
}
