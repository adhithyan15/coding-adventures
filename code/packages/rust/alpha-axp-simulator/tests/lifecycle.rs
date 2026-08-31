use alpha_axp_simulator::encoding::{
    assemble, branch, halt, memory, mov_literal, operate_register,
};
use alpha_axp_simulator::{AlphaError, AlphaSimulator, MEMORY_SIZE};

#[test]
fn exact_state_restore_and_zero_register_are_validated_atomically() {
    let mut cpu = AlphaSimulator::new();
    assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);
    assert_eq!(cpu.get_state().npc, 4);
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

    invalid = before.clone();
    invalid.pc = 1;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(AlphaError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn checked_loading_and_little_endian_direct_access_are_typed() {
    let mut cpu = AlphaSimulator::new();
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

    cpu.load_checked(&assemble(&[halt()])).unwrap();
    assert_eq!(cpu.read_byte_checked(60).unwrap(), 0);
    assert_eq!(cpu.get_state().loaded_len, 4);
    assert_eq!(
        cpu.read_register_checked(32),
        Err(AlphaError::InvalidRegister { index: 32 })
    );
    cpu.write_register_checked(31, 99).unwrap();
    assert_eq!(cpu.read_register_checked(31).unwrap(), 0);
    cpu.write_quad_checked(8, 0x0102_0304_0506_0708).unwrap();
    assert_eq!(cpu.read_quad_checked(8).unwrap(), 0x0102_0304_0506_0708);
    assert_eq!(cpu.read_byte_checked(8).unwrap(), 0x08);
    cpu.write_word_checked(2, 0x1234).unwrap();
    assert_eq!(cpu.read_word_checked(2).unwrap(), 0x1234);
    cpu.write_long_checked(4, 0x89ab_cdef).unwrap();
    assert_eq!(cpu.read_long_checked(4).unwrap(), 0x89ab_cdef);
    assert!(matches!(
        cpu.read_quad_checked(4),
        Err(AlphaError::MisalignedAccess { .. })
    ));
}

#[test]
fn faults_halt_and_bounded_runs_are_atomic() {
    let mut cpu = AlphaSimulator::new();
    cpu.load_checked(&[0, 0, 0]).unwrap();
    assert_eq!(
        cpu.step_checked(),
        Err(AlphaError::TruncatedInstruction { pc: 0 })
    );

    for raw in [1, 0x0400_0000, memory(0x29, 1, 31, 1)] {
        cpu.load_checked(&assemble(&[raw])).unwrap();
        let before = cpu.get_state();
        assert!(cpu.step_checked().is_err());
        assert_eq!(cpu.get_state(), before);
    }

    let program = assemble(&[mov_literal(1, 42), halt()]);
    let result = cpu.run_checked(&program, 4).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.final_state.regs[1], 42);

    cpu.load_checked(&assemble(&[mov_literal(1, 7), memory(0x29, 2, 31, 1)]))
        .unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.run_loaded_checked(4),
        Err(AlphaError::MisalignedAccess { .. })
    ));
    assert_eq!(cpu.get_state(), before);

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
fn arithmetic_control_flow_memory_and_full_traces_work_together() {
    let words = [
        mov_literal(1, 7),
        mov_literal(2, 6),
        operate_register(0x13, 0x20, 1, 2, 3),
        memory(0x2d, 3, 31, 0x100),
        memory(0x29, 4, 31, 0x100),
        halt(),
    ];
    let mut cpu = AlphaSimulator::new();
    let result = cpu.run_checked(&assemble(&words), 10).unwrap();
    assert_eq!(result.final_state.regs[3], 42);
    assert_eq!(result.final_state.regs[4], 42);
    assert_eq!(result.final_state.memory[0x100], 42);
    assert_eq!(result.traces[2].mnemonic, "MULQ");
    assert_eq!(result.traces[2].state_before.regs[3], 0);
    assert_eq!(result.traces[2].state_after.regs[3], 42);
}
