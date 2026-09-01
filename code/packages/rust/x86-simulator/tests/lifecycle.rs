use x86_simulator::functional::{X86Error, X86FunctionalSimulator, MEMORY_SIZE};

#[test]
fn reset_state_is_exact_and_owned() {
    let mut cpu = X86FunctionalSimulator::new();
    cpu.write_register_checked(0, 42).unwrap();
    cpu.write_byte(0xffff, 0xaa);
    cpu.reset();
    let state = cpu.get_state();
    assert_eq!(state.rip, 0);
    assert!(state
        .gpr
        .iter()
        .enumerate()
        .all(|(index, value)| index == 4 || *value == 0));
    assert_eq!(state.gpr[4], 0xfff8);
    assert_eq!(state.rflags, 0);
    assert_eq!(state.memory, vec![0; MEMORY_SIZE]);
    assert!(!state.halted);
}

#[test]
fn checked_load_bounds_failure_is_atomic() {
    let mut cpu = X86FunctionalSimulator::new();
    cpu.load_checked(&[0x90, 0xf4]).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.load_at_checked(&[1, 2], MEMORY_SIZE as u64 - 1),
        Err(X86Error::ProgramOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn invalid_restore_and_register_access_are_atomic() {
    let mut cpu = X86FunctionalSimulator::new();
    let before = cpu.get_state();
    let mut invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(X86Error::InvalidState(_))
    ));
    assert!(matches!(
        cpu.write_register_checked(16, 1),
        Err(X86Error::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn checked_step_reports_complete_before_and_after_states() {
    let mut cpu = X86FunctionalSimulator::new();
    cpu.load_checked(&[0x90, 0xf4]).unwrap();
    let trace = cpu.step_checked().unwrap();
    assert_eq!(trace.rip_before, 0);
    assert_eq!(trace.rip_after, 1);
    assert_eq!(trace.raw, [0x90]);
    assert_eq!(trace.state_before.rip, 0);
    assert_eq!(trace.state_after.rip, 1);
    assert_eq!(trace.state_before.memory.len(), MEMORY_SIZE);
    assert_eq!(trace.state_after.memory.len(), MEMORY_SIZE);
}

#[test]
fn halted_and_unknown_instruction_faults_are_typed_and_atomic() {
    let mut halted = X86FunctionalSimulator::new();
    halted.load_checked(&[0xf4]).unwrap();
    halted.step_checked().unwrap();
    let before = halted.get_state();
    assert_eq!(halted.step_checked(), Err(X86Error::Halted));
    assert_eq!(halted.get_state(), before);

    let mut unknown = X86FunctionalSimulator::new();
    unknown.load_checked(&[0x0f, 0x01]).unwrap();
    let before = unknown.get_state();
    assert!(matches!(
        unknown.step_checked(),
        Err(X86Error::UnknownInstruction { .. })
    ));
    assert_eq!(unknown.get_state(), before);
}

#[test]
fn checked_run_rolls_back_every_prior_step_on_late_failure() {
    let mut cpu = X86FunctionalSimulator::new();
    cpu.write_register_checked(3, 0xfeed).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.run_checked(&[0x90, 0x0f, 0x01], 4),
        Err(X86Error::UnknownInstruction { rip: 1, .. })
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn checked_run_is_bounded_and_returns_full_traces() {
    let mut cpu = X86FunctionalSimulator::new();
    let result = cpu.run_checked(&[0x90, 0x90, 0xf4], 3).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 3);
    assert_eq!(result.traces.len(), 3);
    assert_eq!(result.final_state.rip, 3);

    let mut bounded = X86FunctionalSimulator::new();
    let before = bounded.get_state();
    assert_eq!(
        bounded.run_checked(&[0x90, 0xf4], 1),
        Err(X86Error::StepLimitExceeded { max_steps: 1 })
    );
    assert_eq!(bounded.get_state(), before);
}
