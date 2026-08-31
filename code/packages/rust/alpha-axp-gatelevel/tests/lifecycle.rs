use alpha_axp_simulator::encoding::{
    assemble, branch, halt, memory, mov_literal, operate_register,
};
use alpha_axp_simulator::{AlphaError, MEMORY_SIZE};
use coding_adventures_alpha_axp_gatelevel::{AlphaGateSimulator, FLIP_FLOP_COUNT};

#[test]
fn topology_and_reset_are_exact() {
    assert_eq!(FLIP_FLOP_COUNT, 526_465);
    let mut cpu = AlphaGateSimulator::new();
    cpu.write_register_checked(1, u64::MAX).unwrap();
    cpu.write_quad_checked(8, u64::MAX).unwrap();
    cpu.reset();
    let state = cpu.get_state();
    assert_eq!(state.pc, 0);
    assert_eq!(state.npc, 4);
    assert_eq!(state.regs, [0; 32]);
    assert!(state.memory.iter().all(|byte| *byte == 0));
    assert!(!state.halted);
}

#[test]
fn dff_lifecycle_matches_functional_contract() {
    let program = assemble(&[mov_literal(1, 42), halt()]);
    let mut cpu = AlphaGateSimulator::new();
    let result = cpu.run_checked(&program, 4).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(cpu.read_register_checked(1).unwrap(), 42);
}

#[test]
fn complete_restore_and_direct_access_are_typed_and_atomic() {
    let mut cpu = AlphaGateSimulator::new();
    let before = cpu.get_state();
    let mut invalid = before.clone();
    invalid.regs[31] = 1;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(AlphaError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
    invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(AlphaError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    cpu.write_register_checked(31, 99).unwrap();
    assert_eq!(cpu.read_register_checked(31).unwrap(), 0);
    assert_eq!(
        cpu.read_register_checked(32),
        Err(AlphaError::InvalidRegister { index: 32 })
    );
    cpu.write_byte_checked(7, 0xaa).unwrap();
    assert_eq!(cpu.read_byte_checked(7).unwrap(), 0xaa);
    cpu.write_word_checked(8, 0x1234).unwrap();
    assert_eq!(cpu.read_word_checked(8).unwrap(), 0x1234);
    cpu.write_long_checked(12, 0x89ab_cdef).unwrap();
    assert_eq!(cpu.read_long_checked(12).unwrap(), 0x89ab_cdef);
    cpu.write_quad_checked(16, 0x0102_0304_0506_0708).unwrap();
    assert_eq!(cpu.read_quad_checked(16).unwrap(), 0x0102_0304_0506_0708);
    assert!(matches!(
        cpu.read_quad_checked(4),
        Err(AlphaError::MisalignedAccess { .. })
    ));
}

#[test]
fn checked_loading_faults_and_limits_roll_back_completely() {
    let mut cpu = AlphaGateSimulator::new();
    cpu.write_byte_checked(60, 0xaa).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.load_at_checked(&[0; 8], (MEMORY_SIZE - 4) as u64),
        Err(AlphaError::ProgramOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.load_at_checked(&[0; 4], 1),
        Err(AlphaError::MisalignedProgram { origin: 1 })
    );

    for raw in [1, 0x0400_0000, memory(0x29, 1, 31, 1)] {
        cpu.load_checked(&assemble(&[raw])).unwrap();
        let before = cpu.get_state();
        assert!(cpu.step_checked().is_err());
        assert_eq!(cpu.get_state(), before);
    }

    cpu.load_checked(&assemble(&[branch(0x30, 31, -1)]))
        .unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_loaded_checked(2),
        Err(AlphaError::StepLimitExceeded { max_steps: 2 })
    );
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn arithmetic_control_memory_and_full_traces_work_together() {
    let words = [
        mov_literal(1, 7),
        mov_literal(2, 6),
        operate_register(0x13, 0x20, 1, 2, 3),
        memory(0x2d, 3, 31, 0x100),
        memory(0x29, 4, 31, 0x100),
        halt(),
    ];
    let mut cpu = AlphaGateSimulator::new();
    let result = cpu.execute(&assemble(&words), 10).unwrap();
    assert_eq!(result.final_state.regs[3], 42);
    assert_eq!(result.final_state.regs[4], 42);
    assert_eq!(result.final_state.memory[0x100], 42);
    assert_eq!(result.traces[2].mnemonic, "MULQ");
    assert_eq!(result.traces[2].state_before.regs[3], 0);
    assert_eq!(result.traces[2].state_after.regs[3], 42);
    assert_eq!(cpu.step(), Err(AlphaError::Halted));
}
