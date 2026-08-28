use intel4004_gatelevel::{Intel4004GateLevel, FLIP_FLOP_COUNT};
use intel4004_simulator::{Intel4004Error, Intel4004Simulator};

fn assert_state_matches(gate: &Intel4004GateLevel, functional: &Intel4004Simulator) {
    let gate = gate.snapshot();
    let functional = functional.snapshot();
    assert_eq!(gate.accumulator, functional.accumulator);
    assert_eq!(gate.registers, functional.registers);
    assert_eq!(gate.carry, functional.carry);
    assert_eq!(gate.rom, functional.memory);
    assert_eq!(usize::from(gate.pc), functional.pc);
    assert_eq!(gate.halted, functional.halted);
    assert_eq!(gate.hw_stack, functional.hw_stack);
    assert_eq!(gate.stack_pointer, functional.stack_pointer);
    assert_eq!(gate.ram_bank, functional.ram_bank);
    assert_eq!(gate.ram_register, functional.ram_register);
    assert_eq!(gate.ram_character, functional.ram_character);
    assert_eq!(gate.rom_port, functional.rom_port);
    assert_eq!(gate.ram_output, functional.ram_output);
    for bank in 0..4 {
        for register in 0..4 {
            assert_eq!(gate.ram[bank][register], functional.ram[bank][register]);
            assert_eq!(
                gate.ram_status[bank][register],
                functional.ram_status[bank][register]
            );
        }
    }
}

fn assert_run_matches(program: &[u8], max_steps: usize) {
    let mut gate = Intel4004GateLevel::new();
    let mut functional = Intel4004Simulator::new(4096);
    let gate_traces = gate.run(program, max_steps).unwrap();
    let functional_traces = functional.run(program, max_steps).unwrap();
    assert_eq!(gate_traces.len(), functional_traces.len());
    for (gate_trace, functional_trace) in gate_traces.iter().zip(&functional_traces) {
        assert_eq!(usize::from(gate_trace.address), functional_trace.address);
        assert_eq!(gate_trace.raw, functional_trace.raw);
        assert_eq!(gate_trace.raw2, functional_trace.raw2);
        assert_eq!(
            gate_trace.accumulator_after,
            functional_trace.accumulator_after
        );
        assert_eq!(gate_trace.carry_after, functional_trace.carry_after);
    }
    assert_state_matches(&gate, &functional);
}

fn is_two_byte(raw: u8) -> bool {
    matches!(raw >> 4, 0x1 | 0x4 | 0x5 | 0x7) || raw >> 4 == 0x2 && raw & 1 == 0
}

#[test]
fn every_specified_encoding_matches_the_functional_oracle() {
    for raw in [0x00, 0x01].into_iter().chain(0x10..=0xFD) {
        let program = if is_two_byte(raw) {
            vec![raw, 0x22]
        } else {
            vec![raw]
        };
        assert_run_matches(&program, 1);
    }
}

#[test]
fn memory_ports_branches_and_stack_workloads_match() {
    assert_run_matches(
        &[
            0xD2, 0xFD, // LDM 2; DCL
            0x20, 0x3A, 0x21, // FIM P0,3A; SRC P0
            0xD9, 0xE0, 0xE4, 0xE5, 0xE6, 0xE7, // writes
            0xE1, 0xE2, // RAM/ROM ports
            0xD0, 0xE9, 0xEC, 0xED, 0xEE, 0xEF, // reads
            0xEA, 0xEB, 0xE8, 0x01,
        ],
        64,
    );

    let mut calls = vec![0; 0x30];
    calls[0..3].copy_from_slice(&[0x50, 0x20, 0x01]);
    calls[0x20..0x23].copy_from_slice(&[0xD7, 0xC3, 0x01]);
    assert_run_matches(&calls, 32);

    assert_run_matches(
        &[
            0x20, 0xFE, // FIM P0,FE
            0x30, // FIN P0
            0x31, // JIN P0
            0x00, 0x00, 0x00, 0x00, 0x01,
        ],
        16,
    );

    assert_run_matches(
        &[
            0xD0, 0x14, 0x08, // JCN A==0 taken
            0xD1, 0x40, 0x0A, // skipped, then JUN
            0x00, 0x00, 0xD5, // target
            0x20, 0xE0, 0x70, 0x0B, // FIM, ISZ loop
            0x01,
        ],
        64,
    );
}

#[test]
fn checked_failures_are_identical_and_atomic() {
    let mut gate = Intel4004GateLevel::new();
    let mut functional = Intel4004Simulator::new(4096);
    let oversized = vec![0; 4097];
    let gate_before = gate.snapshot();
    let functional_before = functional.snapshot();
    let expected = Intel4004Error::ProgramTooLarge {
        bytes: 4097,
        capacity: 4096,
    };
    assert_eq!(gate.load_program(&oversized), Err(expected.clone()));
    assert_eq!(functional.load_program(&oversized), Err(expected));
    assert_eq!(gate.snapshot(), gate_before);
    assert_eq!(functional.snapshot(), functional_before);

    gate.load_program(&[0xFE]).unwrap();
    functional.load_program(&[0xFE]).unwrap();
    let gate_before = gate.snapshot();
    let functional_before = functional.snapshot();
    assert_eq!(gate.step().unwrap_err(), functional.step().unwrap_err());
    assert_eq!(gate.snapshot(), gate_before);
    assert_eq!(functional.snapshot(), functional_before);

    let mut boundary = vec![0; 4096];
    boundary[0..2].copy_from_slice(&[0x4F, 0xFF]);
    boundary[4095] = 0x40;
    gate.load_program(&boundary).unwrap();
    functional.load_program(&boundary).unwrap();
    gate.step().unwrap();
    functional.step().unwrap();
    let gate_before = gate.snapshot();
    let functional_before = functional.snapshot();
    assert_eq!(
        gate.step(),
        Err(Intel4004Error::TruncatedInstruction { address: 4095 })
    );
    assert_eq!(
        functional.step(),
        Err(Intel4004Error::TruncatedInstruction { address: 4095 })
    );
    assert_eq!(gate.snapshot(), gate_before);
    assert_eq!(functional.snapshot(), functional_before);

    gate.reset();
    functional.reset();
    assert_eq!(
        gate.run(&[0xFE], usize::MAX),
        Err(Intel4004Error::UnknownOpcode {
            address: 0,
            opcode: 0xFE,
        })
    );
    assert_eq!(
        functional.run(&[0xFE], usize::MAX),
        Err(Intel4004Error::UnknownOpcode {
            address: 0,
            opcode: 0xFE,
        })
    );
}

#[test]
fn reset_and_exact_topology_are_stable() {
    let mut gate = Intel4004GateLevel::new();
    gate.run(&[0xD7, 0xFA, 0x01], 8).unwrap();
    gate.reset();
    let reset = gate.snapshot();
    gate.reset();
    assert_eq!(gate.snapshot(), reset);
    assert_eq!(FLIP_FLOP_COUNT, 1_428);
    assert!(gate.gate_count() > FLIP_FLOP_COUNT * 6);
}
