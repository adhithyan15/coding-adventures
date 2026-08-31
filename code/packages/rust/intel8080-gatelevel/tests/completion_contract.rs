use coding_adventures_intel8080_gatelevel::{GateLevelCpu, FLIP_FLOP_COUNT};
use intel8080_simulator::{Intel8080Error, Intel8080Simulator};

const UNDEFINED: [u8; 12] = [
    0x08, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38, 0xCB, 0xD9, 0xDD, 0xED, 0xFD,
];

fn initialized_functional() -> Intel8080Simulator {
    let mut simulator = Intel8080Simulator::new(65_536);
    simulator.regs.a = 0x91;
    simulator.regs.b = 0x12;
    simulator.regs.c = 0x34;
    simulator.regs.d = 0x20;
    simulator.regs.e = 0x00;
    simulator.regs.h = 0x21;
    simulator.regs.l = 0x00;
    simulator.regs.sp = 0x4000;
    simulator.flags.s = true;
    simulator.flags.ac = true;
    simulator.flags.cy = true;
    simulator.mem.write_byte(0x2000, 0x5A);
    simulator.mem.write_byte(0x2100, 0xC3);
    simulator.mem.write_byte(0x4000, 0x78);
    simulator.mem.write_byte(0x4001, 0x56);
    simulator.input_ports[0] = 0xA5;
    simulator
}

#[test]
fn exact_persistent_topology_is_dff_backed() {
    assert_eq!(
        FLIP_FLOP_COUNT,
        65_536 * 8 // memory
            + 7 * 8 // physical registers
            + 16 // PC
            + 16 // SP
            + 5 // flags
            + 1 // interrupt enable
            + 1 // halt
            + 256 * 8 // input latches
            + 256 * 8 // output latches
    );
    assert_eq!(FLIP_FLOP_COUNT, 528_479);
}

#[test]
fn every_defined_opcode_matches_the_functional_full_state() {
    let baseline = initialized_functional().snapshot();
    let mut baseline_gate = GateLevelCpu::new();
    baseline_gate.restore(&baseline).unwrap();

    for opcode in 0u8..=u8::MAX {
        let mut gate = baseline_gate.clone();
        let mut functional = initialized_functional();
        gate.load(&[opcode, 0x00, 0x20]).unwrap();
        functional.load_program(&[opcode, 0x00, 0x20]).unwrap();
        let gate_before = gate.snapshot();
        let functional_before = functional.snapshot();
        assert_eq!(gate_before, functional_before, "opcode {opcode:#04X}");

        if UNDEFINED.contains(&opcode) {
            let expected = Err(Intel8080Error::UnknownOpcode { address: 0, opcode });
            assert_eq!(gate.step(), expected, "gate opcode {opcode:#04X}");
            assert_eq!(
                functional.step(),
                expected,
                "functional opcode {opcode:#04X}"
            );
            assert_eq!(gate.snapshot(), gate_before, "gate opcode {opcode:#04X}");
            assert_eq!(
                functional.snapshot(),
                functional_before,
                "functional opcode {opcode:#04X}"
            );
            continue;
        }

        let gate_trace = gate
            .step()
            .unwrap_or_else(|error| panic!("gate rejected {opcode:#04X}: {error}"));
        let functional_trace = functional
            .step()
            .unwrap_or_else(|error| panic!("functional rejected {opcode:#04X}: {error}"));
        assert_eq!(gate_trace.address, functional_trace.address);
        assert_eq!(gate_trace.raw, functional_trace.raw, "opcode {opcode:#04X}");
        assert_eq!(
            gate_trace.state_before, functional_trace.state_before,
            "before opcode {opcode:#04X}"
        );
        assert_eq!(
            gate_trace.state_after, functional_trace.state_after,
            "after opcode {opcode:#04X}: gate={} functional={}",
            gate_trace.mnemonic, functional_trace.mnemonic
        );
        assert_eq!(
            gate.snapshot(),
            functional.snapshot(),
            "opcode {opcode:#04X}"
        );
    }
}

#[test]
fn lifecycle_failures_are_typed_transactional_and_atomic() {
    let mut gate = GateLevelCpu::new();
    gate.set_input_port(3, 0x5A);
    gate.load(&[0x76]).unwrap();
    let before = gate.snapshot();
    assert_eq!(
        gate.load(&vec![0; 65_537]),
        Err(Intel8080Error::ProgramOutOfRange {
            length: 65_537,
            memory_size: 65_536,
        })
    );
    assert_eq!(gate.snapshot(), before);

    let mut truncated_state = before.clone();
    truncated_state.pc = 0xFFFF;
    truncated_state.memory[0xFFFF] = 0x3E;
    gate.restore(&truncated_state).unwrap();
    let truncated_before = gate.snapshot();
    assert_eq!(
        gate.step(),
        Err(Intel8080Error::TruncatedInstruction {
            address: 0xFFFF,
            expected: 2,
            available: 1,
        })
    );
    assert_eq!(gate.snapshot(), truncated_before);

    let mut transactional = GateLevelCpu::new();
    transactional.set_input_port(3, 0x5A);
    let transaction_before = transactional.snapshot();
    assert_eq!(
        transactional.run(&[0x3E, 7, 0x08], 10),
        Err(Intel8080Error::UnknownOpcode {
            address: 2,
            opcode: 0x08,
        })
    );
    assert_eq!(transactional.snapshot(), transaction_before);

    gate.restore(&before).unwrap();
    gate.step().unwrap();
    let halted = gate.snapshot();
    assert_eq!(gate.step(), Err(Intel8080Error::Halted));
    assert_eq!(gate.snapshot(), halted);
}

#[test]
fn multi_instruction_workloads_match_trace_states() {
    let workloads: &[&[u8]] = &[
        &[0x3E, 0x25, 0x06, 0x38, 0x80, 0x27, 0xD3, 0x11, 0x76],
        &[
            0x31, 0x00, 0x40, 0x21, 0x34, 0x12, 0xE5, 0x21, 0x00, 0x00, 0xE1, 0x76,
        ],
        &[
            0x21, 0x00, 0x20, 0x36, 0x05, 0x3E, 0x03, 0x86, 0x32, 0x01, 0x20, 0x76,
        ],
    ];
    for program in workloads {
        let mut gate = GateLevelCpu::new();
        let mut functional = Intel8080Simulator::new(65_536);
        let gate_result = gate.run(program, 100).unwrap();
        let functional_result = functional.run(program, 100).unwrap();
        assert_eq!(gate_result.halted, functional_result.halted);
        assert_eq!(gate_result.steps, functional_result.steps);
        assert_eq!(gate_result.pc, functional_result.pc);
        assert_eq!(gate_result.final_state, functional_result.final_state);
        for (gate_trace, functional_trace) in
            gate_result.traces.iter().zip(&functional_result.traces)
        {
            assert_eq!(gate_trace.address, functional_trace.address);
            assert_eq!(gate_trace.raw, functional_trace.raw);
            assert_eq!(gate_trace.state_before, functional_trace.state_before);
            assert_eq!(gate_trace.state_after, functional_trace.state_after);
        }
    }
}
