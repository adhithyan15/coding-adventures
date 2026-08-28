use intel8086_simulator::{Intel8086Error, Intel8086Simulator};

const MEMORY_SIZE: usize = 1 << 20;

#[test]
fn constructor_uses_the_architectural_one_megabyte() {
    let sim = Intel8086Simulator::new(1);
    assert_eq!(sim.mem.size(), MEMORY_SIZE);
}

#[test]
fn oversized_and_overflowing_loads_are_atomic() {
    let mut sim = Intel8086Simulator::new(MEMORY_SIZE);
    sim.ax = 0x1234;
    sim.mem.write_byte(MEMORY_SIZE - 1, 0xaa);
    let before = sim.snapshot();

    assert_eq!(
        sim.load_program_checked_at(&[1, 2], MEMORY_SIZE - 1),
        Err(Intel8086Error::ProgramOutOfRange {
            origin: MEMORY_SIZE - 1,
            length: 2,
        })
    );
    assert_eq!(sim.snapshot(), before);

    assert!(matches!(
        sim.load_program_checked_at(&[1], usize::MAX),
        Err(Intel8086Error::ProgramOutOfRange { .. })
    ));
    assert_eq!(sim.snapshot(), before);

    sim.run(&vec![0; MEMORY_SIZE + 1]);
    assert_eq!(sim.snapshot(), before);
}

#[test]
fn invalid_restore_is_atomic() {
    let mut sim = Intel8086Simulator::new(MEMORY_SIZE);
    sim.ax = 7;
    let before = sim.snapshot();
    let mut invalid = before.clone();
    invalid.memory = vec![0; 5].into_boxed_slice();
    assert_eq!(
        sim.restore(&invalid),
        Err(Intel8086Error::InvalidStateMemory { length: 5 })
    );
    assert_eq!(sim.snapshot(), before);
}

#[test]
fn checked_unknown_opcode_and_run_are_transactional() {
    let mut sim = Intel8086Simulator::new(MEMORY_SIZE);
    sim.ax = 0xbeef;
    sim.load_program_checked(&[0x0f]).unwrap();
    let before_step = sim.snapshot();
    assert!(matches!(
        sim.step_checked(),
        Err(Intel8086Error::UnknownOpcode {
            cs: 0,
            ip: 0,
            raw,
        }) if raw == [0x0f]
    ));
    assert_eq!(sim.snapshot(), before_step);

    sim.load_program_checked(&[0x90, 0x0f]).unwrap();
    let before_run = sim.snapshot();
    assert!(matches!(
        sim.run_loaded_checked(10),
        Err(Intel8086Error::UnknownOpcode { ip: 1, .. })
    ));
    assert_eq!(sim.snapshot(), before_run);
}

#[test]
fn complete_trace_contains_prefix_and_operands() {
    let mut sim = Intel8086Simulator::new(MEMORY_SIZE);
    sim.es = 0x0100;
    sim.mem.write_byte(0x1000 + 0x20, 0x34);
    sim.mem.write_byte(0x1000 + 0x21, 0x12);
    sim.load_program_checked(&[0x26, 0xa1, 0x20, 0x00]).unwrap();
    let trace = sim.step_checked().unwrap();
    assert_eq!(trace.raw, [0x26, 0xa1, 0x20, 0x00]);
    assert_eq!(trace.cs, 0);
    assert_eq!(trace.ip, 0);
    assert_eq!(trace.state_before.ax, 0);
    assert_eq!(trace.state_after.ax, 0x1234);
}

#[test]
fn checked_ports_reject_out_of_range_indices() {
    let mut sim = Intel8086Simulator::new(MEMORY_SIZE);
    sim.set_input_port(255, 0xa5).unwrap();
    assert_eq!(sim.input_ports[255], 0xa5);
    assert_eq!(
        sim.set_input_port(256, 1),
        Err(Intel8086Error::InvalidPort { port: 256 })
    );
    assert_eq!(
        sim.get_output_port(999),
        Err(Intel8086Error::InvalidPort { port: 999 })
    );
}

#[test]
fn run_checked_preserves_inputs_and_returns_complete_final_state() {
    let mut sim = Intel8086Simulator::new(MEMORY_SIZE);
    sim.set_input_port(0x20, 0x5a).unwrap();
    let result = sim.run_checked(&[0xe4, 0x20, 0xf4], 10).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.final_state.ax, 0x5a);
    assert_eq!(result.final_state.memory.len(), MEMORY_SIZE);
}
