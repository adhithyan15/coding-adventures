use coding_adventures_x86_64_gatelevel::{X86Error, X86GateSimulator, FLIP_FLOP_COUNT};

#[test]
fn exact_topology_and_dff_state_are_stable() {
    assert_eq!(FLIP_FLOP_COUNT, 525_382);
    let mut cpu = X86GateSimulator::new();
    assert_eq!(cpu.get_state().gpr[4], 0xfff8);
    cpu.write_register_checked(7, 0x1234_5678_9abc_def0)
        .unwrap();
    cpu.write_byte(0x1_0100, 0xaa);
    let saved = cpu.get_state();
    cpu.reset();
    cpu.restore(&saved).unwrap();
    assert_eq!(cpu.read_register_checked(7).unwrap(), 0x1234_5678_9abc_def0);
    assert_eq!(cpu.read_byte(0x100), 0xaa);
}

#[test]
fn checked_load_step_and_trace_match_the_functional_machine() {
    let program = [
        0x48, 0xb8, 0x2a, 0, 0, 0, 0, 0, 0, 0, // mov rax,42
        0x48, 0x83, 0xc0, 0x01, // add rax,1
        0xf4,
    ];
    let mut gate = X86GateSimulator::new();
    let result = gate.run_checked(&program, 8).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 3);
    assert_eq!(result.final_state.gpr[0], 43);
    assert_eq!(result.traces[1].state_before.gpr[0], 42);
    assert_eq!(result.traces[1].state_after.gpr[0], 43);
}

#[test]
fn checked_failures_are_transition_atomic() {
    let mut cpu = X86GateSimulator::new();
    cpu.load_checked(&[0x0f]).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(X86Error::TruncatedInstruction { rip: 0 })
    );
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&[0xeb, 0xfe]).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_loaded_checked(3),
        Err(X86Error::StepLimitExceeded { max_steps: 3 })
    );
    assert_eq!(cpu.get_state(), before);
}
