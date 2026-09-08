use mips_r2000_simulator::encoding::{assemble, encode_addiu, encode_lw, encode_syscall};
use mips_r2000_simulator::{MipsError, MipsR2000Simulator, MEMORY_SIZE};

#[test]
fn architectural_machine_is_exact_and_restore_is_atomic() {
    let mut cpu = MipsR2000Simulator::architectural();
    assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);
    let before = cpu.get_state();
    let mut invalid = before.clone();
    invalid.regs[0] = 1;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(MipsError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
    invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(MipsError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn checked_load_is_deterministic_typed_and_atomic() {
    let mut cpu = MipsR2000Simulator::new(64);
    cpu.write_byte_checked(60, 0xAA).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.load_at_checked(&[0; 8], 60),
        Err(MipsError::ProgramOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.load_at_checked(&[0; 4], 1),
        Err(MipsError::MisalignedProgram { origin: 1 })
    );
    cpu.load_checked(&assemble(&[encode_syscall()])).unwrap();
    assert_eq!(cpu.read_byte_checked(60).unwrap(), 0);
    assert_eq!(cpu.get_state().loaded_len, 4);
}

#[test]
fn checked_direct_register_and_big_endian_memory_access_is_typed() {
    let mut cpu = MipsR2000Simulator::new(64);
    assert_eq!(
        cpu.read_register_checked(32),
        Err(MipsError::InvalidRegister { index: 32 })
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
        Err(MipsError::MisalignedAccess { .. })
    ));
}

#[test]
fn checked_faults_and_halt_are_typed_and_atomic() {
    let mut cpu = MipsR2000Simulator::new(64);
    cpu.load_checked(&[0, 0, 0]).unwrap();
    assert_eq!(
        cpu.step_checked(),
        Err(MipsError::TruncatedInstruction { pc: 0 })
    );

    cpu.load_checked(&assemble(&[encode_lw(2, 1, 1)])).unwrap();
    cpu.write_register_checked(1, 0).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.step_checked(),
        Err(MipsError::MisalignedAccess { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&assemble(&[encode_syscall()])).unwrap();
    cpu.step_checked().unwrap();
    let halted = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(MipsError::Halted));
    assert_eq!(cpu.get_state(), halted);
}

#[test]
fn checked_run_returns_complete_traces_and_rolls_back_late_failure() {
    let mut cpu = MipsR2000Simulator::new(64);
    let program = assemble(&[encode_addiu(2, 0, 42), encode_syscall()]);
    let result = cpu.run_checked(&program, 10).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.final_state, cpu.get_state());
    assert_eq!(result.final_state.regs[2], 42);

    let bad = assemble(&[encode_addiu(2, 0, 7), encode_lw(3, 0, 1)]);
    cpu.load_checked(&bad).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.run_loaded_checked(10),
        Err(MipsError::MisalignedAccess { .. })
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn unaligned_merge_loads_and_stores_cover_the_gate_partner_surface() {
    use mips_r2000_simulator::encoding::{encode_lwl, encode_lwr, encode_swl, encode_swr};
    let mut cpu = MipsR2000Simulator::new(4096);
    cpu.load_checked(&assemble(&[
        encode_lwl(3, 1, 1),
        encode_lwr(4, 1, 1),
        encode_swl(5, 1, 1),
        encode_swr(6, 1, 1),
        encode_syscall(),
    ]))
    .unwrap();
    cpu.write_register_checked(1, 0x400).unwrap();
    cpu.write_register_checked(3, 0xAAAA_AAAA).unwrap();
    cpu.write_register_checked(4, 0xBBBB_BBBB).unwrap();
    cpu.write_register_checked(5, 0x1234_5678).unwrap();
    cpu.write_register_checked(6, 0x89AB_CDEF).unwrap();
    cpu.write_word_checked(0x400, 0xDEAD_BEEF).unwrap();
    cpu.run_loaded_checked(10).unwrap();
    assert_eq!(cpu.read_register_checked(3).unwrap(), 0xDEAD_AAAA);
    assert_eq!(cpu.read_register_checked(4).unwrap(), 0xBBAD_BEEF);
    assert_eq!(cpu.read_word_checked(0x400).unwrap(), 0x12AB_CDEF);
}
