use sparc_v8_simulator::encoding::{
    assemble, encode_add_imm, encode_alu_imm, encode_ld, encode_save, encode_ta, encode_ticc,
    encode_udiv,
};
use sparc_v8_simulator::{SparcError, SparcState, SparcV8Simulator, MEMORY_SIZE};

#[test]
fn architectural_machine_is_exact_and_restore_is_atomic() {
    let mut cpu = SparcV8Simulator::architectural();
    assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);
    let before = cpu.get_state();

    let mut invalid = before.clone();
    invalid.regs[0] = 1;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(SparcError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(SparcError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    let mutations: [fn(&mut SparcState); 5] = [
        |state| state.cwp = 3,
        |state| state.save_depth = 3,
        |state| state.pc = 1,
        |state| state.npc = 8,
        |state| state.loaded_origin = 1,
    ];
    for mutate in mutations {
        invalid = before.clone();
        mutate(&mut invalid);
        assert!(matches!(
            cpu.restore(&invalid),
            Err(SparcError::InvalidState(_))
        ));
        assert_eq!(cpu.get_state(), before);
    }

    invalid = before.clone();
    invalid.loaded_origin = (MEMORY_SIZE - 4) as u32;
    invalid.loaded_len = 8;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(SparcError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn checked_load_is_deterministic_typed_and_atomic() {
    let mut cpu = SparcV8Simulator::new(64);
    cpu.write_byte_checked(60, 0xAA).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.load_at_checked(&[0; 8], 60),
        Err(SparcError::ProgramOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.load_at_checked(&[0; 4], 1),
        Err(SparcError::MisalignedProgram { origin: 1 })
    );
    cpu.load_checked(&assemble(&[encode_ta(0)])).unwrap();
    assert_eq!(cpu.read_byte_checked(60).unwrap(), 0);
    assert_eq!(cpu.get_state().loaded_len, 4);
}

#[test]
fn checked_direct_register_and_big_endian_memory_access_is_typed() {
    let mut cpu = SparcV8Simulator::new(64);
    assert_eq!(
        cpu.read_register_checked(32),
        Err(SparcError::InvalidRegister { index: 32 })
    );
    assert_eq!(
        cpu.write_register_checked(32, 1),
        Err(SparcError::InvalidRegister { index: 32 })
    );
    cpu.write_register_checked(2, 0xDEAD_BEEF).unwrap();
    assert_eq!(cpu.read_register_checked(2).unwrap(), 0xDEAD_BEEF);
    cpu.write_register_checked(0, 1).unwrap();
    assert_eq!(cpu.read_register_checked(0).unwrap(), 0);

    cpu.write_word_checked(4, 0x1020_3040).unwrap();
    assert_eq!(cpu.read_word_checked(4).unwrap(), 0x1020_3040);
    assert_eq!(cpu.read_byte_checked(4).unwrap(), 0x10);
    assert!(matches!(
        cpu.read_word_checked(5),
        Err(SparcError::MisalignedAccess { .. })
    ));
    assert!(matches!(
        cpu.write_word_checked(64, 0),
        Err(SparcError::MemoryOutOfRange { .. })
    ));
    assert!(matches!(
        cpu.read_byte_checked(64),
        Err(SparcError::MemoryOutOfRange { .. })
    ));
    assert!(matches!(
        cpu.write_byte_checked(64, 0),
        Err(SparcError::MemoryOutOfRange { .. })
    ));
}

#[test]
fn checked_faults_and_halt_are_typed_and_atomic() {
    let mut cpu = SparcV8Simulator::new(64);
    cpu.load_checked(&[0, 0, 0]).unwrap();
    assert_eq!(
        cpu.step_checked(),
        Err(SparcError::TruncatedInstruction { pc: 0 })
    );

    let fault_words = [
        encode_udiv(4, 2, 0),
        encode_ticc(1, 0, 0),
        encode_alu_imm(0x3F, 4, 2, 3),
        encode_ld(4, 1, 1),
    ];
    for word in fault_words {
        cpu.load_checked(&assemble(&[word])).unwrap();
        let before = cpu.get_state();
        assert!(cpu.step_checked().is_err(), "{word:#010x}");
        assert_eq!(cpu.get_state(), before, "{word:#010x} must fail atomically");
    }

    cpu.load_checked(&assemble(&[encode_save(4, 2, 3)]))
        .unwrap();
    let mut overflow = cpu.get_state();
    overflow.save_depth = 2;
    cpu.restore(&overflow).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.step_checked(),
        Err(SparcError::WindowOverflow { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&assemble(&[encode_ta(0)])).unwrap();
    cpu.step_checked().unwrap();
    let halted = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(SparcError::Halted));
    assert_eq!(cpu.get_state(), halted);
}

#[test]
fn checked_run_returns_complete_traces_and_rolls_back_late_failure() {
    let mut cpu = SparcV8Simulator::new(64);
    let program = assemble(&[encode_add_imm(2, 0, 42), encode_ta(0)]);
    let result = cpu.run_checked(&program, 10).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.final_state, cpu.get_state());
    assert_eq!(result.final_state.regs[2], 42);

    let bad = assemble(&[encode_add_imm(2, 0, 7), encode_ld(3, 0, 1)]);
    cpu.load_checked(&bad).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.run_loaded_checked(10),
        Err(SparcError::MisalignedAccess { .. })
    ));
    assert_eq!(cpu.get_state(), before);
}
