use arm_simulator::functional::{
    assemble_words, encode_branch, encode_data_immediate, encode_transfer, ArmError,
    Armv7Simulator, Condition, DataOpcode, HALT_WORD, MEMORY_SIZE,
};
use armv7_gatelevel::{Armv7GateLevel, FLIP_FLOP_COUNT};

#[test]
fn topology_state_restore_and_access_cover_every_dff() {
    let mut cpu = Armv7GateLevel::new();
    assert_eq!(FLIP_FLOP_COUNT, 524_805);
    assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);
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
    assert_eq!(
        cpu.read_register_checked(16),
        Err(ArmError::InvalidRegister { index: 16 })
    );
    cpu.write_register_checked(0, 0xdead_beef).unwrap();
    assert_eq!(cpu.read_register_checked(0).unwrap(), 0xdead_beef);
    cpu.write_word_checked(8, 0x0102_0304).unwrap();
    assert_eq!(cpu.read_word_checked(8).unwrap(), 0x0102_0304);
    assert_eq!(cpu.read_byte_checked(8).unwrap(), 4);
}

#[test]
fn checked_loads_clear_stale_state_and_track_origin() {
    let mut cpu = Armv7GateLevel::new();
    cpu.write_byte_checked(900, 0xaa).unwrap();
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
    cpu.load_at_checked(&assemble_words(&[HALT_WORD]), 0x100)
        .unwrap();
    let state = cpu.get_state();
    assert_eq!(state.pc, 0x100);
    assert_eq!(state.registers[15], 0x100);
    assert_eq!(state.loaded_origin, 0x100);
    assert_eq!(state.loaded_len, 4);
    assert_eq!(state.memory[900], 0);
}

#[test]
fn step_and_run_faults_are_fully_transactional() {
    let mut cpu = Armv7GateLevel::new();
    cpu.load_checked(&[0, 0, 0]).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(ArmError::TruncatedInstruction { pc: 0 })
    );
    assert_eq!(cpu.get_state(), before);

    for raw in [
        0xee00_0000,
        encode_transfer(Condition::Al, true, true, 0, 1, 2),
        0xe080_1011,
    ] {
        cpu.load_checked(&assemble_words(&[raw])).unwrap();
        let before = cpu.get_state();
        assert!(cpu.step_checked().is_err());
        assert_eq!(cpu.get_state(), before);
    }

    cpu.load_checked(&assemble_words(&[encode_branch(Condition::Al, false, -8)]))
        .unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_loaded_checked(4),
        Err(ArmError::StepLimitExceeded { max_steps: 4 })
    );
    assert_eq!(cpu.get_state(), before);

    let late_fault = assemble_words(&[
        encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 0, 0, 7),
        0xee00_0000,
    ]);
    cpu.load_checked(&late_fault).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.run_loaded_checked(4),
        Err(ArmError::UnknownInstruction { .. })
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn traces_match_the_functional_lifecycle_contract() {
    let words = [
        encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 0, 0, 0x80),
        encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 1, 0, 42),
        encode_transfer(Condition::Al, false, true, 0, 1, 0),
        encode_transfer(Condition::Al, true, true, 0, 2, 0),
        HALT_WORD,
    ];
    let program = assemble_words(&words);
    let gate = Armv7GateLevel::new().run_checked(&program, 8).unwrap();
    let functional = Armv7Simulator::new().run_checked(&program, 8).unwrap();
    assert_eq!(gate, functional);
    assert_eq!(gate.traces[2].mnemonic, "STR");
    assert_eq!(gate.traces[2].state_before.memory[0x80], 0);
    assert_eq!(gate.traces[2].state_after.memory[0x80], 42);
}
