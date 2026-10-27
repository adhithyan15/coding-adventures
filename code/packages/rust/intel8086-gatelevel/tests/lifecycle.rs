use coding_adventures_intel8086_gatelevel::{Cpu8086, Intel8086Error, FLIP_FLOP_COUNT};

const MEMORY_SIZE: usize = 1 << 20;

#[test]
fn exact_topology_is_public_and_stable() {
    let cpu = Cpu8086::new();
    let inventory = cpu.physical_storage_inventory();
    assert_eq!(inventory.memory_dffs, 8_388_608);
    assert_eq!(inventory.register_dffs, 208);
    assert_eq!(inventory.flag_dffs, 9);
    assert_eq!(inventory.halt_dffs, 1);
    assert_eq!(inventory.input_port_dffs, 2_048);
    assert_eq!(inventory.output_port_dffs, 2_048);
    assert_eq!(inventory.total(), FLIP_FLOP_COUNT);
    assert_eq!(inventory.total(), 8_392_922);
}

#[test]
fn halt_q_overrides_both_compatibility_mirror_divergences() {
    let mut running = Cpu8086::new();
    running.load_checked(&[0x90, 0xf4], 0).unwrap();
    running.halted = true;
    assert!(!running.halted());
    assert!(!running.get_state().halted);
    let trace = running.step_checked().unwrap();
    assert_eq!(trace.raw, [0x90]);
    assert_eq!(trace.state_after.ip, 1);
    assert!(!running.halted);

    let result = running.run_loaded_checked(2).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 1);
    assert!(result.final_state.halted);
    assert!(running.halted());

    running.halted = false;
    let before = running.get_state();
    assert!(before.halted);
    assert_eq!(running.step_checked(), Err(Intel8086Error::Halted));
    assert_eq!(running.get_state(), before);
    assert!(running.halted);
}

#[test]
fn restore_clocks_halt_q_and_faults_preserve_it() {
    let mut cpu = Cpu8086::new();
    let mut halted = cpu.get_state();
    halted.halted = true;
    cpu.restore(&halted).unwrap();
    assert!(cpu.halted());
    assert!(cpu.halted);

    cpu.load_checked(&[0xf4], 0).unwrap();
    cpu.step_checked().unwrap();
    assert!(cpu.halted());
    assert!(cpu.get_state().halted);

    cpu.load_checked(&[0x0f], 0).unwrap();
    cpu.halted = true;
    let before = cpu.get_state();
    assert!(matches!(
        cpu.step_checked(),
        Err(Intel8086Error::UnknownOpcode { .. })
    ));
    assert_eq!(cpu.get_state(), before);
    assert!(!cpu.halted());
    assert!(!cpu.halted);

    cpu.load_checked(&[0x90, 0x0f], 0).unwrap();
    cpu.halted = true;
    let before_run = cpu.get_state();
    assert!(matches!(
        cpu.run_loaded_checked(2),
        Err(Intel8086Error::UnknownOpcode { .. })
    ));
    assert_eq!(cpu.get_state(), before_run);
    assert!(!cpu.halted());
    assert!(!cpu.halted);
}

#[test]
fn counted_bank_q_overrides_observers_across_the_full_lifecycle() {
    let mut cpu = Cpu8086::new();
    let mut installed = cpu.get_state();
    installed.ax = 0x1234;
    installed.flags = 0x0203;
    installed.input_ports[0x20] = 0x5a;
    installed.output_ports[0x21] = 0xa5;
    installed.memory[0] = 0x15;
    installed.memory[1] = 0x01;
    installed.memory[2] = 0x00;
    installed.memory[3] = 0x0f;
    cpu.restore(&installed).unwrap();

    cpu.rf.ax = 0xdead;
    cpu.rf.flag_cf = 0;
    cpu.input_ports[0x20] = 0x11;
    cpu.output_ports[0x21] = 0x22;
    let snapshot = cpu.get_state();
    assert_eq!(snapshot.ax, 0x1234);
    assert_eq!(snapshot.flags, 0x0203);
    assert_eq!(snapshot.input_ports[0x20], 0x5a);
    assert_eq!(snapshot.output_ports[0x21], 0xa5);

    let trace = cpu.step_checked().unwrap();
    assert_eq!(trace.state_before.ax, 0x1234);
    assert_eq!(trace.state_before.flags, 0x0203);
    assert_eq!(trace.state_before.input_ports[0x20], 0x5a);
    assert_eq!(trace.state_before.output_ports[0x21], 0xa5);
    assert_eq!(trace.state_after.ip, 3);
    assert_eq!(trace.state_after.ax, 0x1236);
    assert_eq!(cpu.rf.ax, 0x1236);
    assert_eq!(cpu.rf.flag_cf, 0);
    assert_eq!(cpu.input_ports[0x20], 0x5a);
    assert_eq!(cpu.output_ports[0x21], 0xa5);

    let mut restored = cpu.get_state();
    restored.ax = 0x5678;
    restored.flags = 0x0042;
    restored.ip = 0;
    restored.input_ports[0x20] = 0x33;
    restored.output_ports[0x21] = 0xcc;
    restored.memory[0] = 0x90;
    restored.memory[1] = 0x0f;
    cpu.restore(&restored).unwrap();
    assert_eq!(cpu.get_state(), restored);
    assert_eq!(cpu.rf.ax, 0x5678);
    assert_eq!(cpu.rf.flag_zf, 1);
    assert_eq!(cpu.input_ports[0x20], 0x33);
    assert_eq!(cpu.output_ports[0x21], 0xcc);
    assert_eq!(cpu.get_output_port(0x21).unwrap(), 0xcc);

    cpu.rf.ax = 0xffff;
    cpu.rf.flag_zf = 0;
    cpu.input_ports[0x20] = 0xee;
    cpu.output_ports[0x21] = 0xee;
    let entry = cpu.get_state();
    assert!(matches!(
        cpu.run_loaded_checked(2),
        Err(Intel8086Error::UnknownOpcode { .. })
    ));
    assert_eq!(cpu.get_state(), entry);
    assert_eq!(cpu.rf.ax, 0x5678);
    assert_eq!(cpu.rf.flag_zf, 1);
    assert_eq!(cpu.input_ports[0x20], 0x33);
    assert_eq!(cpu.output_ports[0x21], 0xcc);
}

#[test]
fn port_instructions_sample_and_clock_q_not_public_arrays() {
    let mut cpu = Cpu8086::new();
    cpu.set_input_port(0x20, 0x5a).unwrap();
    cpu.load_checked(&[0xe4, 0x20, 0xe6, 0x21], 0).unwrap();

    cpu.input_ports[0x20] = 0x11;
    cpu.step_checked().unwrap();
    assert_eq!(cpu.get_state().ax & 0xff, 0x5a);
    assert_eq!(cpu.input_ports[0x20], 0x5a);

    cpu.output_ports[0x21] = 0x22;
    cpu.step_checked().unwrap();
    assert_eq!(cpu.get_output_port(0x21).unwrap(), 0x5a);
    assert_eq!(cpu.output_ports[0x21], 0x5a);
}

#[test]
fn checked_load_failures_are_atomic() {
    let mut cpu = Cpu8086::new();
    cpu.rf.write16(0, 0x1234);
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
    cpu.rf.write16(0, 0xbeef);
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
    cpu.rf.write_seg(0, 0x100);
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
