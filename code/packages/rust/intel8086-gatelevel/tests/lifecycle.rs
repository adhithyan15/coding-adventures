use coding_adventures_intel8086_gatelevel::{Cpu8086, Intel8086Error, FLIP_FLOP_COUNT};

const MEMORY_SIZE: usize = 1 << 20;

#[test]
fn exact_topology_is_public_and_stable() {
    assert_eq!(FLIP_FLOP_COUNT, 8_392_922);
}

#[test]
fn checked_load_failures_are_atomic() {
    let mut cpu = Cpu8086::new();
    cpu.rf.ax = 0x1234;
    cpu.write_memory(MEMORY_SIZE - 1, 0xaa);
    let before = cpu.get_state();
    assert_eq!(
        cpu.load_checked(&[1, 2], MEMORY_SIZE - 1),
        Err(Intel8086Error::ProgramOutOfRange {
            origin: MEMORY_SIZE - 1,
            length: 2,
        })
    );
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn invalid_restore_is_atomic() {
    let mut cpu = Cpu8086::new();
    let before = cpu.get_state();
    let mut invalid = before.clone();
    invalid.memory = vec![0; 7].into_boxed_slice();
    assert_eq!(
        cpu.restore(&invalid),
        Err(Intel8086Error::InvalidStateMemory { length: 7 })
    );
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn unknown_instruction_is_rejected_without_mutation() {
    let mut cpu = Cpu8086::new();
    cpu.load_checked(&[0x0f], 0).unwrap();
    cpu.rf.ax = 0xbeef;
    let before = cpu.get_state();
    assert!(matches!(
        cpu.step_checked(),
        Err(Intel8086Error::UnknownOpcode { raw, .. }) if raw == [0x0f]
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn checked_trace_contains_prefix_operands_and_complete_states() {
    let mut cpu = Cpu8086::new();
    cpu.rf.es = 0x100;
    cpu.write_memory(0x1020, 0x34);
    cpu.write_memory(0x1021, 0x12);
    cpu.load_checked(&[0x26, 0xa1, 0x20, 0], 0).unwrap();
    let trace = cpu.step_checked().unwrap();
    assert_eq!(trace.raw, [0x26, 0xa1, 0x20, 0]);
    assert_eq!(trace.state_before.ax, 0);
    assert_eq!(trace.state_after.ax, 0x1234);
    assert_eq!(trace.state_after.memory.len(), MEMORY_SIZE);
}

#[test]
fn checked_ports_and_transactional_run_share_functional_contract() {
    let mut cpu = Cpu8086::new();
    cpu.set_input_port(0x20, 0x5a).unwrap();
    assert_eq!(
        cpu.set_input_port(256, 0),
        Err(Intel8086Error::InvalidPort { port: 256 })
    );
    let result = cpu.run_checked(&[0xe4, 0x20, 0xf4], 10).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.final_state.ax, 0x5a);
    assert_eq!(cpu.get_output_port(0).unwrap(), 0);
}
