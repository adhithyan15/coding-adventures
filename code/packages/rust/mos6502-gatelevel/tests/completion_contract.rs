use coding_adventures_mos6502_gatelevel::{GateLevelCpu, FLIP_FLOP_COUNT};
use mos6502_simulator::{Mos6502Error, Mos6502Simulator};

fn initialized_functional() -> Mos6502Simulator {
    let mut sim = Mos6502Simulator::new(65_536);
    sim.a = 0x91;
    sim.x = 0x12;
    sim.y = 0x34;
    sim.s = 0xFD;
    sim.flag_v = true;
    sim.flag_d = true;
    sim.flag_i = false;
    sim.flag_c = true;
    sim.mem.write_byte(0x20, 0xEC);
    sim.mem.write_byte(0x21, 0x1F);
    sim.mem.write_byte(0x32, 0x20);
    sim.mem.write_byte(0x33, 0x20);
    sim.mem.write_byte(0x2020, 0x5A);
    sim.mem.write_byte(0x2021, 0x20);
    sim.mem.write_byte(0x01FE, 0x65);
    sim.mem.write_byte(0x01FF, 0x34);
    sim.mem.write_byte(0x0100, 0x12);
    sim
}

#[test]
fn exact_persistent_topology_is_dff_backed() {
    assert_eq!(
        FLIP_FLOP_COUNT,
        65_536 * 8 + 4 * 8 + 16 + 7 + 1 + 240 * 8 + 240 * 8
    );
    assert_eq!(FLIP_FLOP_COUNT, 528_184);
}

#[test]
fn all_256_encodings_match_the_functional_full_state() {
    let baseline = initialized_functional().snapshot();
    let mut baseline_gate = GateLevelCpu::new();
    baseline_gate.restore(&baseline).unwrap();

    for opcode in 0u8..=u8::MAX {
        let mut gate = baseline_gate.clone();
        let mut functional = initialized_functional();
        gate.load(&[opcode, 0x20, 0x20], 0).unwrap();
        functional.load_program(&[opcode, 0x20, 0x20]).unwrap();
        let before = functional.snapshot();
        assert_eq!(gate.snapshot(), before, "before opcode {opcode:#04X}");

        if mos6502_simulator::opcodes::lookup(opcode).is_none() {
            let expected = Err(Mos6502Error::UnknownOpcode { address: 0, opcode });
            assert_eq!(gate.step(), expected, "gate opcode {opcode:#04X}");
            assert_eq!(
                functional.step(),
                expected,
                "functional opcode {opcode:#04X}"
            );
            assert_eq!(gate.snapshot(), before, "gate opcode {opcode:#04X}");
            assert_eq!(
                functional.snapshot(),
                before,
                "functional opcode {opcode:#04X}"
            );
            continue;
        }

        let gate_trace = gate
            .step()
            .unwrap_or_else(|error| panic!("gate opcode {opcode:#04X}: {error}"));
        let functional_trace = functional
            .step()
            .unwrap_or_else(|error| panic!("functional opcode {opcode:#04X}: {error}"));
        assert_eq!(gate_trace.address, functional_trace.address);
        assert_eq!(gate_trace.raw, functional_trace.raw, "opcode {opcode:#04X}");
        assert_eq!(gate_trace.state_before, functional_trace.state_before);
        assert!(
            gate_trace.state_after == functional_trace.state_after,
            "opcode {opcode:#04X}: gate={} A={:#04X} X={:#04X} Y={:#04X} S={:#04X} PC={:#06X} flags={:?}; functional={} A={:#04X} X={:#04X} Y={:#04X} S={:#04X} PC={:#06X} flags={:?}",
            gate_trace.mnemonic,
            gate_trace.state_after.a,
            gate_trace.state_after.x,
            gate_trace.state_after.y,
            gate_trace.state_after.s,
            gate_trace.state_after.pc,
            [gate_trace.state_after.flag_n, gate_trace.state_after.flag_v, gate_trace.state_after.flag_b, gate_trace.state_after.flag_d, gate_trace.state_after.flag_i, gate_trace.state_after.flag_z, gate_trace.state_after.flag_c],
            functional_trace.mnemonic,
            functional_trace.state_after.a,
            functional_trace.state_after.x,
            functional_trace.state_after.y,
            functional_trace.state_after.s,
            functional_trace.state_after.pc,
            [functional_trace.state_after.flag_n, functional_trace.state_after.flag_v, functional_trace.state_after.flag_b, functional_trace.state_after.flag_d, functional_trace.state_after.flag_i, functional_trace.state_after.flag_z, functional_trace.state_after.flag_c],
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
    gate.set_input_port(4, 0xA5).unwrap();
    let before = gate.snapshot();
    assert_eq!(
        gate.load(&vec![0; 65_537], 0),
        Err(Mos6502Error::ProgramTooLarge {
            length: 65_537,
            capacity: 65_536,
        })
    );
    assert_eq!(gate.snapshot(), before);
    assert_eq!(
        gate.run(&[0xA9, 0x42, 0x02], 10),
        Err(Mos6502Error::UnknownOpcode {
            address: 2,
            opcode: 0x02,
        })
    );
    assert_eq!(gate.snapshot(), before);

    gate.load(&[0x00], 0).unwrap();
    gate.step().unwrap();
    let halted = gate.snapshot();
    assert_eq!(gate.step(), Err(Mos6502Error::Halted));
    assert_eq!(gate.snapshot(), halted);
    assert_eq!(
        gate.set_input_port(240, 0),
        Err(Mos6502Error::InvalidPort { port: 240 })
    );
    assert_eq!(
        gate.get_output_port(255),
        Err(Mos6502Error::InvalidPort { port: 255 })
    );
}

#[test]
fn irq_masking_and_nmi_vector_entry_are_persistent() {
    let mut gate = GateLevelCpu::new();
    let mut initial = gate.snapshot();
    initial.pc = 0x1234;
    initial.s = 0xFD;
    initial.flag_c = true;
    initial.flag_i = true;
    initial.memory[0xFFFA] = 0xBC;
    initial.memory[0xFFFB] = 0x9A;
    initial.memory[0xFFFE] = 0x78;
    initial.memory[0xFFFF] = 0x56;
    gate.restore(&initial).unwrap();

    gate.interrupt();
    assert_eq!(gate.snapshot(), initial, "a masked IRQ must be atomic");

    initial.flag_i = false;
    gate.restore(&initial).unwrap();
    gate.interrupt();
    let irq = gate.snapshot();
    assert_eq!(irq.pc, 0x5678);
    assert_eq!(irq.s, 0xFA);
    assert!(irq.flag_i);
    assert_eq!(&irq.memory[0x01FB..=0x01FD], &[0x21, 0x34, 0x12]);

    gate.restore(&initial).unwrap();
    gate.nmi();
    let nmi = gate.snapshot();
    assert_eq!(nmi.pc, 0x9ABC);
    assert_eq!(nmi.s, 0xFA);
    assert!(nmi.flag_i);
    assert_eq!(&nmi.memory[0x01FB..=0x01FD], &[0x21, 0x34, 0x12]);
}

#[test]
fn workloads_match_functional_traces_and_memory_mapped_io() {
    let workloads: &[&[u8]] = &[
        &[0xA9, 0x25, 0x69, 0x38, 0x85, 0x20, 0x00],
        &[0xA2, 3, 0xA9, 0, 0x18, 0x69, 1, 0xCA, 0xD0, 0xFA, 0x00],
        &[0x20, 0x07, 0x00, 0xA9, 0x42, 0x00, 0xEA, 0xA9, 7, 0x60],
    ];
    for program in workloads {
        let mut gate = GateLevelCpu::new();
        let mut functional = Mos6502Simulator::new(65_536);
        let gate_result = gate.run(program, 100).unwrap();
        let functional_result = functional.run(program).unwrap();
        assert_eq!(gate_result, functional_result);
    }

    let mut gate = GateLevelCpu::new();
    let mut functional = Mos6502Simulator::new(65_536);
    gate.set_input_port(5, 0xCD).unwrap();
    functional.set_input_port(5, 0xCD).unwrap();
    let program = [0xAD, 0x05, 0xFF, 0x8D, 0x0A, 0xFF, 0x00];
    assert_eq!(
        gate.run(&program, 100).unwrap(),
        functional.run(&program).unwrap()
    );
}
