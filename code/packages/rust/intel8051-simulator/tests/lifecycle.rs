use intel8051_simulator::{Intel8051Error, Intel8051Simulator};

#[test]
fn state_owns_complete_harvard_memory() {
    let simulator = Intel8051Simulator::new();
    let state = simulator.get_state();
    assert_eq!(state.code.len(), 65_536);
    assert_eq!(state.xdata.len(), 65_536);
    assert_eq!(state.iram.len(), 256);
    assert_eq!(state.iram[0x81], 0x07);
    assert_eq!(state.iram[0x80], 0xff);
}

#[test]
fn checked_load_and_restore_failures_are_atomic() {
    let mut simulator = Intel8051Simulator::new();
    let before = simulator.get_state();
    assert!(matches!(
        simulator.load_at_checked(&[1, 2], 0xffff),
        Err(Intel8051Error::ProgramOutOfRange { .. })
    ));
    assert_eq!(simulator.get_state(), before);

    let mut invalid = before.clone();
    invalid.code.pop();
    assert!(matches!(
        simulator.restore(&invalid),
        Err(Intel8051Error::InvalidState(_))
    ));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn checked_load_is_deterministic() {
    let mut simulator = Intel8051Simulator::new();
    let mut state = simulator.get_state();
    state.code[100] = 0xaa;
    state.xdata[100] = 0xbb;
    simulator.restore(&state).unwrap();
    simulator.load_checked(&[0xa5]).unwrap();
    let loaded = simulator.get_state();
    assert_eq!(loaded.code[0], 0xa5);
    assert_eq!(loaded.code[100], 0);
    assert_eq!(loaded.xdata[100], 0);
}

#[test]
fn truncated_halted_and_execution_failures_are_atomic() {
    let mut simulator = Intel8051Simulator::new();
    simulator.load_checked(&[0x74]).unwrap();
    let before = simulator.get_state();
    assert!(matches!(
        simulator.step_checked(),
        Err(Intel8051Error::TruncatedInstruction { .. })
    ));
    assert_eq!(simulator.get_state(), before);

    simulator.load_checked(&[0xe6]).unwrap(); // MOV A,@R0
    let mut invalid_indirect = simulator.get_state();
    invalid_indirect.iram[0] = 0x80;
    simulator.restore(&invalid_indirect).unwrap();
    let before = simulator.get_state();
    assert!(matches!(
        simulator.step_checked(),
        Err(Intel8051Error::Execution(_))
    ));
    assert_eq!(simulator.get_state(), before);

    simulator.load_checked(&[0xa5]).unwrap();
    simulator.step_checked().unwrap();
    let before = simulator.get_state();
    assert_eq!(simulator.step_checked(), Err(Intel8051Error::Halted));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn checked_run_returns_complete_traces() {
    let mut simulator = Intel8051Simulator::new();
    let result = simulator
        .run_checked(&[0x74, 42, 0xa5], 8)
        .expect("MOV A,#42; HALT");
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.traces[0].raw, [0x74, 42]);
    assert_eq!(result.final_state.iram[0xe0], 42);
}
