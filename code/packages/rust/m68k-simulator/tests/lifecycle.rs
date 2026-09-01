use m68k_simulator::{M68kError, M68kSimulator, LOAD_ADDRESS, MEMORY_SIZE};

#[test]
fn architectural_machine_has_exact_reset_state() {
    let simulator = M68kSimulator::architectural();
    let state = simulator.get_state();
    assert_eq!(state.memory.len(), MEMORY_SIZE);
    assert_eq!(state.pc, LOAD_ADDRESS);
    assert_eq!(state.a[7], 0x00f000);
    assert_eq!(state.sr, 0x2700);
    assert_eq!(state.d, [0; 8]);
    assert!(!state.halted);
}

#[test]
fn checked_load_bounds_failure_is_atomic() {
    let mut simulator = M68kSimulator::architectural();
    simulator.d[0] = 0x1234_5678;
    let before = simulator.get_state();
    let error = simulator.load_at_checked(&[1, 2], 0x00ff_ffff).unwrap_err();
    assert!(matches!(error, M68kError::ProgramTooLarge { .. }));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn invalid_restore_is_atomic() {
    let mut simulator = M68kSimulator::architectural();
    simulator.d[2] = 99;
    let before = simulator.get_state();
    let mut invalid = before.clone();
    invalid.pc = 0x0100_0000;
    assert!(matches!(
        simulator.restore(&invalid),
        Err(M68kError::InvalidState(_))
    ));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn unknown_instruction_is_rejected_without_mutation() {
    let mut simulator = M68kSimulator::architectural();
    simulator.load_checked(&[0xa0, 0x00]).unwrap();
    let before = simulator.get_state();
    assert!(matches!(
        simulator.step_checked(),
        Err(M68kError::Execution(_))
    ));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn checked_step_has_complete_before_and_after_states() {
    let mut simulator = M68kSimulator::architectural();
    simulator.load_checked(&[0x70, 0x80]).unwrap(); // MOVEQ #-128,D0
    let trace = simulator.step_checked().unwrap();
    assert_eq!(trace.pc_before, LOAD_ADDRESS);
    assert_eq!(trace.pc_after, LOAD_ADDRESS + 2);
    assert_eq!(trace.raw, 0x7080);
    assert_eq!(trace.mnemonic, "MOVEQ");
    assert_eq!(trace.state_before.memory.len(), MEMORY_SIZE);
    assert_eq!(trace.state_after.d[0], 0xffff_ff80);
    assert_eq!(trace.state_after, simulator.get_state());
}

#[test]
fn checked_lifecycle_rejects_legacy_sized_memory() {
    let mut simulator = M68kSimulator::new(65_536);
    assert_eq!(
        simulator.load_checked(&[0x4e, 0x4f]),
        Err(M68kError::NonArchitecturalMemory { actual: 65_536 })
    );
}

#[test]
fn checked_run_returns_bounded_full_traces() {
    let mut simulator = M68kSimulator::architectural();
    let result = simulator
        .run_checked(&[0x70, 0x2a, 0x4e, 0x4f], 10)
        .unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.final_state.d[0], 42);
    assert_eq!(result.final_state, simulator.get_state());
}

#[test]
fn checked_run_error_restores_whole_machine() {
    let mut simulator = M68kSimulator::architectural();
    simulator.d[4] = 0xfeed_face;
    let before = simulator.get_state();
    assert!(matches!(
        simulator.run_checked(&[0xa0, 0x00], 1),
        Err(M68kError::Execution(_))
    ));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn halted_checked_step_and_state_flags_are_typed() {
    let mut simulator = M68kSimulator::architectural();
    simulator.load_checked(&[0x4e, 0x4f]).unwrap();
    simulator.step_checked().unwrap();
    assert_eq!(simulator.step_checked(), Err(M68kError::Halted));
    let mut state = simulator.get_state();
    state.sr = 0x1f;
    assert!(state.x() && state.n() && state.z() && state.v() && state.c());
}
