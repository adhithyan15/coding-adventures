use coding_adventures_mips_r2000_gatelevel::{CpuMipsR2000, MipsError, FLIP_FLOP_COUNT};

#[test]
fn topology_is_exact_and_reset_is_complete() {
    assert_eq!(FLIP_FLOP_COUNT, 525_409);
    let mut cpu = CpuMipsR2000::new();
    cpu.write_register_checked(7, 0xDEAD_BEEF).unwrap();
    cpu.write_byte_checked(0x1234, 0xA5).unwrap();
    cpu.load_checked(&0x0000_000Cu32.to_be_bytes()).unwrap();
    cpu.step_checked().unwrap();
    assert!(cpu.halted);
    cpu.reset();
    assert_eq!(cpu.get_state(), CpuMipsR2000::new().get_state());
}

#[test]
fn complete_state_round_trips_through_dff_storage() {
    let mut cpu = CpuMipsR2000::new();
    cpu.write_register_checked(5, 0x1234_5678).unwrap();
    cpu.write_word_checked(0x400, 0x89AB_CDEF).unwrap();
    let snapshot = cpu.get_state();
    cpu.reset();
    cpu.restore(&snapshot).unwrap();
    assert_eq!(cpu.get_state(), snapshot);
}

#[test]
fn checked_direct_access_is_typed_and_r0_is_hardwired() {
    let mut cpu = CpuMipsR2000::new();
    cpu.write_register_checked(0, u32::MAX).unwrap();
    assert_eq!(cpu.read_register_checked(0).unwrap(), 0);
    assert!(matches!(
        cpu.read_register_checked(32),
        Err(MipsError::InvalidRegister { index: 32 })
    ));
    assert!(matches!(
        cpu.read_word_checked(1),
        Err(MipsError::MisalignedAccess { .. })
    ));
    assert!(matches!(
        cpu.read_byte_checked(65_536),
        Err(MipsError::MemoryOutOfRange { .. })
    ));
}

#[test]
fn checked_step_traces_every_state_field_and_halt_clock() {
    let mut cpu = CpuMipsR2000::new();
    cpu.load_checked(&0x0000_000Cu32.to_be_bytes()).unwrap();
    let trace = cpu.step_checked().unwrap();
    assert_eq!(trace.pc_before, 0);
    assert_eq!(trace.pc_after, 4);
    assert_eq!(trace.raw, 0x0000_000C);
    assert!(!trace.state_before.halted);
    assert!(trace.state_after.halted);
    assert!(matches!(cpu.step_checked(), Err(MipsError::Halted)));
}

#[test]
fn invalid_load_and_restore_are_atomic() {
    let mut cpu = CpuMipsR2000::new();
    cpu.write_register_checked(3, 99).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.load_at_checked(&[0; 4], 2),
        Err(MipsError::MisalignedProgram { origin: 2 })
    ));
    assert_eq!(cpu.get_state(), before);
    let mut invalid = before.clone();
    invalid.regs[0] = 1;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(MipsError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn checked_run_rolls_back_the_complete_machine_on_fault() {
    let mut cpu = CpuMipsR2000::new();
    cpu.write_register_checked(4, 0xCAFE_BABE).unwrap();
    cpu.write_byte_checked(0x800, 0x5A).unwrap();
    let before = cpu.get_state();
    let unknown = 0xFC00_0000u32.to_be_bytes();
    assert!(matches!(
        cpu.run_checked(&unknown, 1),
        Err(MipsError::UnknownInstruction { .. })
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn bounded_checked_run_reports_complete_result() {
    let mut cpu = CpuMipsR2000::new();
    let program = [0x2408_002Au32.to_be_bytes(), 0x0000_000Cu32.to_be_bytes()].concat();
    let result = cpu.run_checked(&program, 8).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.final_state.regs[8], 42);
    assert_eq!(result.traces.len(), 2);
}
