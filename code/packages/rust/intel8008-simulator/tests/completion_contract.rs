use coding_adventures_intel8008_simulator::{Intel8008Error, Simulator};

fn encoded_length(opcode: u8) -> Option<usize> {
    let group = opcode >> 6;
    let ddd = (opcode >> 3) & 0x07;
    let sss = opcode & 0x07;
    match group {
        0 if sss == 4 => None,
        0 if sss == 6 => Some(2),
        0 => Some(1),
        1 if opcode == 0x7C || opcode == 0x7E => Some(3),
        1 if ddd <= 3 && matches!(sss, 0 | 2 | 4 | 6) => Some(3),
        1 | 2 => Some(1),
        3 if opcode == 0xFF => Some(1),
        3 if sss == 4 => Some(2),
        _ => None,
    }
}

#[test]
fn every_encoding_has_the_expected_checked_boundary() {
    for opcode in 0u8..=u8::MAX {
        let mut simulator = Simulator::new();
        simulator
            .load_program(&[opcode, 0, 0], 0)
            .expect("three bytes fit");
        let before = simulator.snapshot();
        match encoded_length(opcode) {
            Some(length) => {
                let trace = simulator.step().unwrap_or_else(|error| {
                    panic!("defined opcode {opcode:#04X} was rejected: {error}")
                });
                assert_eq!(trace.address, 0);
                assert_eq!(trace.raw.len(), length, "opcode {opcode:#04X}");
            }
            None => {
                assert_eq!(
                    simulator.step(),
                    Err(Intel8008Error::UnknownOpcode { address: 0, opcode })
                );
                assert_eq!(simulator.snapshot(), before);
            }
        }
    }
}

#[test]
fn load_and_port_failures_are_typed_and_atomic() {
    let mut simulator = Simulator::new();
    simulator.load_program(&[0x76], 0).unwrap();
    simulator.set_input_port(7, 0xA5).unwrap();
    let before = simulator.snapshot();

    assert_eq!(
        simulator.load_program(&[1, 2], 16_383),
        Err(Intel8008Error::ProgramOutOfRange {
            start: 16_383,
            length: 2,
        })
    );
    assert_eq!(
        simulator.load_program(&[], usize::MAX),
        Err(Intel8008Error::ProgramOutOfRange {
            start: usize::MAX,
            length: 0,
        })
    );
    assert_eq!(
        simulator.set_input_port(8, 1),
        Err(Intel8008Error::InputPortOutOfRange { port: 8 })
    );
    assert_eq!(
        simulator.get_output_port(24),
        Err(Intel8008Error::OutputPortOutOfRange { port: 24 })
    );
    assert_eq!(simulator.snapshot(), before);
}

#[test]
fn step_rejects_truncated_and_halted_execution_atomically() {
    let mut simulator = Simulator::new();
    // Jump to the final byte, which contains a two-byte MVI instruction.
    simulator.load_program(&[0x7C, 0xFF, 0x3F], 0).unwrap();
    simulator.load_program(&[0x3E], 0x3FFF).unwrap();
    simulator.step().unwrap();
    let before = simulator.snapshot();
    assert_eq!(
        simulator.step(),
        Err(Intel8008Error::TruncatedInstruction {
            address: 0x3FFF,
            expected: 2,
            available: 1,
        })
    );
    assert_eq!(simulator.snapshot(), before);

    simulator.run(&[0x76], 1).unwrap();
    let halted = simulator.snapshot();
    assert_eq!(simulator.step(), Err(Intel8008Error::Halted));
    assert_eq!(simulator.snapshot(), halted);
}

#[test]
fn run_is_bounded_deterministic_and_rejects_oversized_images() {
    let mut simulator = Simulator::new();
    simulator.load_program(&[0xAA], 100).unwrap();
    simulator.set_input_port(3, 0x5A).unwrap();
    let before = simulator.snapshot();
    let oversized = vec![0; 16_385];
    assert_eq!(
        simulator.run(&oversized, 1),
        Err(Intel8008Error::ProgramOutOfRange {
            start: 0,
            length: 16_385,
        })
    );
    assert_eq!(simulator.snapshot(), before);

    assert_eq!(
        simulator.run(&[0x04], 1),
        Err(Intel8008Error::UnknownOpcode {
            address: 0,
            opcode: 0x04,
        })
    );
    assert_eq!(simulator.snapshot(), before);

    let traces = simulator.run(&[0x41], 1).unwrap();
    assert_eq!(traces.len(), 1);
    assert_eq!(simulator.snapshot().memory[100], 0);

    let first = simulator.run(&[0x3E, 7, 0x76], 10).unwrap();
    let first_state = simulator.snapshot();
    let second = simulator.run(&[0x3E, 7, 0x76], 10).unwrap();
    assert_eq!(second, first);
    assert_eq!(simulator.snapshot(), first_state);
}

#[test]
fn snapshot_is_owned_and_reset_preserves_memory_and_external_inputs() {
    let mut simulator = Simulator::new();
    simulator.load_program(&[0x3E, 9, 0x22], 0).unwrap();
    simulator.set_input_port(2, 0xCC).unwrap();
    simulator.step().unwrap();
    simulator.step().unwrap();
    let snapshot = simulator.snapshot();
    assert_eq!(snapshot.registers[7], 9);
    assert_eq!(snapshot.output_ports[17], 9);

    simulator.reset();
    let reset = simulator.snapshot();
    assert_eq!(reset.registers, [0; 8]);
    assert_eq!(reset.stack, [0; 8]);
    assert_eq!(reset.input_ports[2], 0xCC);
    assert_eq!(reset.output_ports, [0; 24]);
    assert_eq!(reset.memory[0], 0x3E);
    assert_eq!(snapshot.registers[7], 9, "prior snapshot remains owned");
}
