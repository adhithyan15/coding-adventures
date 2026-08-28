use coding_adventures_intel8008_gatelevel::{GateLevelCpu, FLIP_FLOP_COUNT};
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
fn exact_persistent_topology_is_documented() {
    // 16 KiB memory + 7 registers + stack and pointer + flags/halt + I/O.
    assert_eq!(
        FLIP_FLOP_COUNT,
        16_384 * 8 + 7 * 8 + (8 * 14 + 3) + 4 + 1 + 8 * 8 + 24 * 8
    );
    let cpu = GateLevelCpu::default();
    assert_eq!(cpu.flip_flop_count(), FLIP_FLOP_COUNT);
    assert_eq!(cpu.b(), 0);
    assert_eq!(cpu.c(), 0);
    assert_eq!(cpu.d(), 0);
    assert_eq!(cpu.e(), 0);
    assert_eq!(cpu.h(), 0);
    assert_eq!(cpu.l(), 0);
    assert_eq!(cpu.pc(), 0);
    assert_eq!(cpu.stack_depth(), 0);
}

#[test]
fn all_256_first_bytes_match_the_functional_oracle_completely() {
    let mut functional = Simulator::new();
    let mut gate = GateLevelCpu::new();
    for opcode in 0u8..=u8::MAX {
        let program = [opcode, 0, 0];
        functional.reset();
        gate.reset();
        functional.load_program(&program, 0).unwrap();
        gate.load_program(&program, 0).unwrap();
        let length = encoded_length(opcode);
        let before = length
            .is_none()
            .then(|| (functional.snapshot(), gate.snapshot()));

        let functional_result = functional.step().map(|trace| vec![trace]);
        let gate_result = gate.step().map(|trace| vec![trace]);
        match length {
            Some(length) => {
                let functional_trace = functional_result
                    .unwrap_or_else(|error| panic!("functional rejected {opcode:#04X}: {error}"));
                let gate_trace = gate_result
                    .unwrap_or_else(|error| panic!("gate rejected {opcode:#04X}: {error}"));
                assert_eq!(functional_trace, gate_trace, "trace for {opcode:#04X}");
                assert_eq!(functional_trace[0].raw.len(), length);
                assert_eq!(
                    functional.snapshot(),
                    gate.snapshot(),
                    "state for {opcode:#04X}"
                );
            }
            None => {
                let expected = Err(Intel8008Error::UnknownOpcode { address: 0, opcode });
                assert_eq!(functional_result, expected);
                assert_eq!(gate_result, expected);
                let (functional_before, gate_before) = before.unwrap();
                assert_eq!(functional.snapshot(), functional_before);
                assert_eq!(gate.snapshot(), gate_before);
            }
        }
    }
}

#[test]
fn checked_lifecycle_and_port_failures_match_and_are_atomic() {
    let mut functional = Simulator::new();
    let mut gate = GateLevelCpu::new();
    functional.load_program(&[0x76], 0).unwrap();
    gate.load_program(&[0x76], 0).unwrap();
    functional.set_input_port(7, 0xA5).unwrap();
    gate.set_input_port(7, 0xA5).unwrap();
    let functional_before = functional.snapshot();
    let gate_before = gate.snapshot();

    let expected_load = Err(Intel8008Error::ProgramOutOfRange {
        start: 16_383,
        length: 2,
    });
    assert_eq!(functional.load_program(&[1, 2], 16_383), expected_load);
    assert_eq!(gate.load_program(&[1, 2], 16_383), expected_load);
    assert_eq!(
        gate.load_program(&[], usize::MAX),
        Err(Intel8008Error::ProgramOutOfRange {
            start: usize::MAX,
            length: 0,
        })
    );
    assert_eq!(
        functional.set_input_port(8, 1),
        Err(Intel8008Error::InputPortOutOfRange { port: 8 })
    );
    assert_eq!(
        gate.set_input_port(8, 1),
        Err(Intel8008Error::InputPortOutOfRange { port: 8 })
    );
    assert_eq!(
        gate.get_output_port(24),
        Err(Intel8008Error::OutputPortOutOfRange { port: 24 })
    );
    assert_eq!(functional.snapshot(), functional_before);
    assert_eq!(gate.snapshot(), gate_before);
}

#[test]
fn truncated_halted_and_run_failures_match_atomically() {
    let mut functional = Simulator::new();
    let mut gate = GateLevelCpu::new();
    functional.load_program(&[0x7C, 0xFF, 0x3F], 0).unwrap();
    functional.load_program(&[0x3E], 0x3FFF).unwrap();
    functional.step().unwrap();
    gate.load_program(&[0x7C, 0xFF, 0x3F], 0).unwrap();
    gate.load_program(&[0x3E], 0x3FFF).unwrap();
    gate.step().unwrap();
    let functional_before = functional.snapshot();
    let gate_before = gate.snapshot();
    let expected = Err(Intel8008Error::TruncatedInstruction {
        address: 0x3FFF,
        expected: 2,
        available: 1,
    });
    assert_eq!(functional.step(), expected);
    assert_eq!(gate.step(), expected);
    assert_eq!(functional.snapshot(), functional_before);
    assert_eq!(gate.snapshot(), gate_before);

    functional.run(&[0x76], 1).unwrap();
    gate.run(&[0x76], 1).unwrap();
    let functional_halted = functional.snapshot();
    let gate_halted = gate.snapshot();
    assert_eq!(functional.step(), Err(Intel8008Error::Halted));
    assert_eq!(gate.step(), Err(Intel8008Error::Halted));
    assert_eq!(functional.snapshot(), functional_halted);
    assert_eq!(gate.snapshot(), gate_halted);

    let oversized = vec![0; 16_385];
    assert_eq!(
        gate.run(&oversized, 1),
        Err(Intel8008Error::ProgramOutOfRange {
            start: 0,
            length: 16_385,
        })
    );
    assert_eq!(gate.snapshot(), gate_halted);

    assert_eq!(
        gate.run(&[0x04], 1),
        Err(Intel8008Error::UnknownOpcode {
            address: 0,
            opcode: 0x04,
        })
    );
    assert_eq!(gate.snapshot(), gate_halted);
}

#[test]
fn multi_instruction_memory_control_and_io_workloads_match_completely() {
    let workloads: &[&[u8]] = &[
        // Repeated-addition loop: 4 × 5.
        &[0x06, 5, 0x0E, 4, 0x3E, 0, 0x80, 0x09, 0x48, 6, 0, 0x76],
        // H:L indirect write/read plus ALU-memory source.
        &[0x26, 0, 0x2E, 0x20, 0x36, 0x55, 0x7D, 0x86, 0x76],
        // Restart, return, then halt (vector 3 at address 0x18).
        &[
            0x1D, 0x76, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x3E, 77,
            0x3F,
        ],
    ];

    for program in workloads {
        let mut functional = Simulator::new();
        let mut gate = GateLevelCpu::new();
        let functional_traces = functional.run(program, 200).unwrap();
        let gate_traces = gate.run(program, 200).unwrap();
        assert_eq!(gate_traces, functional_traces);
        assert_eq!(gate.snapshot(), functional.snapshot());
    }

    let io_program = [0x59, 0x22, 0x76]; // IN 3; OUT 17; HLT
    let mut functional = Simulator::new();
    let mut gate = GateLevelCpu::new();
    functional.set_input_port(3, 0xA7).unwrap();
    gate.set_input_port(3, 0xA7).unwrap();
    assert_eq!(gate.run(&io_program, 10), functional.run(&io_program, 10));
    assert_eq!(gate.snapshot(), functional.snapshot());
    assert_eq!(gate.get_output_port(17).unwrap(), 0xA7);
}
