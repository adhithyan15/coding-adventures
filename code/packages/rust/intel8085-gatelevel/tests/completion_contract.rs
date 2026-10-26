use intel8085_gatelevel::{GateLevelCpu, FLIP_FLOP_COUNT, PERSISTENT_GATE_EQUIVALENT};
use intel8085_simulator::{Intel8085Error, Intel8085Simulator, InterruptRequest, RestartMasks};

const UNDOCUMENTED: [u8; 10] = [0x08, 0x10, 0x18, 0x28, 0x38, 0xCB, 0xD9, 0xDD, 0xED, 0xFD];

fn initialized_functional(program: &[u8]) -> Intel8085Simulator {
    let mut machine = Intel8085Simulator::new(65_536);
    machine.load_program(program).unwrap();
    machine.core.regs.a = 0x91;
    machine.core.regs.b = 0x12;
    machine.core.regs.c = 0x34;
    machine.core.regs.d = 0x20;
    machine.core.regs.e = 0x00;
    machine.core.regs.h = 0x21;
    machine.core.regs.l = 0x00;
    machine.core.regs.sp = 0x4000;
    machine.core.flags.s = true;
    machine.core.flags.ac = true;
    machine.core.flags.cy = true;
    machine.core.mem.write_byte(0x2000, 0x5A);
    machine.core.mem.write_byte(0x2100, 0xC3);
    machine.core.mem.write_byte(0x4000, 0x78);
    machine.core.mem.write_byte(0x4001, 0x56);
    machine.core.input_ports[0] = 0xA5;
    machine
}

#[test]
fn exact_persistent_topology_is_dff_backed() {
    assert_eq!(
        FLIP_FLOP_COUNT,
        528_479 // complete Intel 8080 core
            + 16 // loaded origin
            + 17 // loaded length, including 65,536
            + 3 // restart masks
            + 4 // pending fixed interrupt lines
            + 1 // INTR-present latch
            + 8 // INTR acknowledge byte
            + 2 // SID and SOD
            + 1 // delayed EI
    );
    assert_eq!(FLIP_FLOP_COUNT, 528_531);
    assert_eq!(PERSISTENT_GATE_EQUIVALENT, 3_171_186);
}

fn assert_opcode_range(start: u8, end: u8) {
    for opcode in start..=end {
        let program = [opcode, 0x00, 0x20];
        let mut functional = initialized_functional(&program);
        let before = functional.snapshot();
        let mut gate = GateLevelCpu::new();
        gate.restore(&before).unwrap();
        assert_eq!(gate.snapshot(), before, "restore opcode {opcode:#04X}");

        if UNDOCUMENTED.contains(&opcode) {
            let expected = Err(Intel8085Error::UnknownOpcode { address: 0, opcode });
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
            .unwrap_or_else(|error| panic!("gate rejected {opcode:#04X}: {error}"));
        let functional_trace = functional
            .step()
            .unwrap_or_else(|error| panic!("functional rejected {opcode:#04X}: {error}"));
        assert_eq!(gate_trace.address, functional_trace.address);
        assert_eq!(gate_trace.raw, functional_trace.raw, "opcode {opcode:#04X}");
        assert_eq!(
            gate_trace.mnemonic, functional_trace.mnemonic,
            "opcode {opcode:#04X}"
        );
        assert_eq!(
            gate_trace.state_before, functional_trace.state_before,
            "before opcode {opcode:#04X}"
        );
        assert_eq!(
            gate_trace.state_after, functional_trace.state_after,
            "after opcode {opcode:#04X}"
        );
        assert_eq!(
            gate.snapshot(),
            functional.snapshot(),
            "opcode {opcode:#04X}"
        );
    }
}

#[test]
fn first_bytes_00_through_3f_match_functional_trace_and_complete_state() {
    assert_opcode_range(0x00, 0x3F);
}

#[test]
fn first_bytes_40_through_7f_match_functional_trace_and_complete_state() {
    assert_opcode_range(0x40, 0x7F);
}

#[test]
fn first_bytes_80_through_bf_match_functional_trace_and_complete_state() {
    assert_opcode_range(0x80, 0xBF);
}

#[test]
fn first_bytes_c0_through_ff_match_functional_trace_and_complete_state() {
    assert_opcode_range(0xC0, 0xFF);
}

#[test]
fn rim_sim_and_delayed_ei_match_the_control_plane_oracle() {
    let program = [0x20, 0x30, 0xFB, 0x00, 0x76];
    let mut functional = initialized_functional(&program);
    functional.serial_input = true;
    functional.core.regs.a = 0xD9;
    functional.request_interrupt(InterruptRequest::Rst75);
    functional.request_interrupt(InterruptRequest::Rst65);
    functional.request_interrupt(InterruptRequest::Rst55);

    let mut gate = GateLevelCpu::new();
    gate.restore(&functional.snapshot()).unwrap();
    for step in 0..program.len() {
        let gate_trace = gate.step().unwrap();
        let functional_trace = functional.step().unwrap();
        assert_eq!(
            gate_trace.mnemonic, functional_trace.mnemonic,
            "step {step}"
        );
        assert_eq!(gate_trace.raw, functional_trace.raw, "step {step}");
        assert_eq!(gate.snapshot(), functional.snapshot(), "step {step}");
    }
}

#[test]
fn interrupt_priority_vectors_stack_order_halt_release_and_intr_match() {
    let mut functional = initialized_functional(&[0x76]);
    functional.core.regs.sp = 0x0100;
    functional.restart_masks = RestartMasks::unmasked();
    functional.core.interrupts_enabled = true;
    functional.request_interrupt(InterruptRequest::Rst55);
    functional.request_interrupt(InterruptRequest::Rst65);
    functional.request_interrupt(InterruptRequest::Rst75);
    functional.request_interrupt(InterruptRequest::Trap);

    let mut gate = GateLevelCpu::new();
    gate.restore(&functional.snapshot()).unwrap();
    for expected in ["INT TRAP", "INT RST7.5"] {
        let gate_trace = gate.step().unwrap();
        let functional_trace = functional.step().unwrap();
        assert_eq!(gate_trace.mnemonic, expected);
        assert_eq!(gate_trace, functional_trace);
        assert_eq!(gate.snapshot(), functional.snapshot());
        if expected == "INT TRAP" {
            let mut state = gate.snapshot();
            state.core.interrupts_enabled = true;
            gate.restore(&state).unwrap();
            functional.core.interrupts_enabled = true;
        }
    }

    let mut halted = initialized_functional(&[0x76]);
    halted.core.regs.sp = 0x0200;
    halted.step().unwrap();
    halted.request_interrupt(InterruptRequest::Intr(0xDF));
    halted.core.interrupts_enabled = true;
    gate.restore(&halted.snapshot()).unwrap();
    let functional_trace = halted.step().unwrap();
    let gate_trace = gate.step().unwrap();
    assert_eq!(gate_trace, functional_trace);
    assert_eq!(gate.snapshot(), halted.snapshot());
    assert!(!gate.snapshot().core.halted);
    assert_eq!(gate.snapshot().core.pc, 0x18);
}

#[test]
fn checked_lifecycle_faults_are_typed_and_atomic() {
    let mut gate = GateLevelCpu::new();
    gate.set_input_port(3, 0x5A);
    gate.load(&[0x76]).unwrap();
    let before = gate.snapshot();
    assert_eq!(
        gate.load(&vec![0; 65_537]),
        Err(Intel8085Error::ProgramOutOfRange {
            origin: 0,
            length: 65_537,
            memory_size: 65_536,
        })
    );
    assert_eq!(gate.snapshot(), before);

    gate.load_program_at(&[0x3E], 0xFFFF).unwrap();
    let truncated = gate.snapshot();
    assert_eq!(
        gate.step(),
        Err(Intel8085Error::TruncatedInstruction {
            address: 0xFFFF,
            expected: 2,
            available: 1,
        })
    );
    assert_eq!(gate.snapshot(), truncated);

    let mut state = gate.snapshot();
    state.core.pc = 0;
    gate.restore(&state).unwrap();
    let outside = gate.snapshot();
    assert_eq!(
        gate.step(),
        Err(Intel8085Error::FetchOutsideProgram {
            address: 0,
            origin: 0xFFFF,
            length: 1,
        })
    );
    assert_eq!(gate.snapshot(), outside);

    let mut invalid_intr = GateLevelCpu::new();
    invalid_intr.load(&[0]).unwrap();
    let mut state = invalid_intr.snapshot();
    state.core.interrupts_enabled = true;
    invalid_intr.restore(&state).unwrap();
    invalid_intr.request_interrupt(InterruptRequest::Intr(0));
    let invalid_before = invalid_intr.snapshot();
    assert_eq!(
        invalid_intr.step(),
        Err(Intel8085Error::UnsupportedInterruptOpcode { opcode: 0 })
    );
    assert_eq!(invalid_intr.snapshot(), invalid_before);
}

#[test]
fn bounded_runs_are_transactional_and_match_multi_instruction_workloads() {
    let workloads: &[&[u8]] = &[
        &[0x3E, 0x25, 0x06, 0x38, 0x80, 0x27, 0xD3, 0x11, 0x76],
        &[0x3E, 0xC0, 0x30, 0x20, 0x76],
        &[0xFB, 0xF3, 0x00, 0x76],
        &[0x31, 0x00, 0x40, 0x21, 0x34, 0x12, 0xE5, 0xE1, 0x76],
    ];
    for program in workloads {
        let mut gate = GateLevelCpu::new();
        let mut functional = Intel8085Simulator::new(65_536);
        gate.set_serial_input(true);
        functional.serial_input = true;
        let gate_result = gate.run(program, 100).unwrap();
        let functional_result = functional.run(program, 100).unwrap();
        assert_eq!(gate_result, functional_result);
        assert_eq!(gate.snapshot(), functional.snapshot());
    }

    let mut gate = GateLevelCpu::new();
    gate.set_serial_input(true);
    let before = gate.snapshot();
    assert_eq!(
        gate.run(&[0x3E, 7, 0x08], 10),
        Err(Intel8085Error::UnknownOpcode {
            address: 2,
            opcode: 0x08,
        })
    );
    assert_eq!(gate.snapshot(), before);

    let mut loaded = GateLevelCpu::new();
    loaded.load(&[0x3E, 7, 0x08]).unwrap();
    let loaded_before = loaded.snapshot();
    assert_eq!(
        loaded.run_loaded_with_limit(10),
        Err(Intel8085Error::UnknownOpcode {
            address: 2,
            opcode: 0x08,
        })
    );
    assert_eq!(loaded.snapshot(), loaded_before);
    assert_eq!(
        loaded.run_loaded(),
        Err(Intel8085Error::UnknownOpcode {
            address: 2,
            opcode: 0x08,
        })
    );
    assert_eq!(loaded.snapshot(), loaded_before);
}

#[test]
fn sixteen_bit_memory_and_stack_cycles_wrap_at_the_bus_boundary() {
    for (program, configure) in [
        (&[0x2A, 0xFF, 0xFF][..], 0_u8),    // LHLD 0xffff
        (&[0x22, 0xFF, 0xFF][..], 0),       // SHLD 0xffff
        (&[0xC1][..], 1),                   // POP B with SP=0xffff
        (&[0xC9][..], 1),                   // RET with SP=0xffff
        (&[0xE3][..], 2),                   // XTHL with SP=0xffff
        (&[0xC5][..], 3),                   // PUSH B with SP=1
        (&[0xCF][..], 3),                   // RST 1 with SP=1
        (&[0xCD, 0x03, 0x00, 0x76][..], 3), // CALL with SP=1
    ] {
        let mut functional = initialized_functional(program);
        functional.core.mem.write_byte(0xFFFF, 0x34);
        match configure {
            0 => {}
            1 | 2 => functional.core.regs.sp = 0xFFFF,
            3 => functional.core.regs.sp = 1,
            _ => unreachable!(),
        }
        let before = functional.snapshot();
        let mut gate = GateLevelCpu::new();
        gate.restore(&before).unwrap();

        let gate_trace = gate.step().unwrap();
        let functional_trace = functional.step().unwrap();
        assert_eq!(gate_trace, functional_trace, "program {program:02X?}");
        assert_eq!(
            gate.snapshot(),
            functional.snapshot(),
            "program {program:02X?}"
        );
    }
}

#[test]
fn reset_restore_ports_and_trace_limits_cover_complete_state() {
    let mut gate = GateLevelCpu::new();
    gate.set_input_port(9, 0xA5);
    gate.set_serial_input(true);
    gate.load(&[0xDB, 9, 0xD3, 7, 0x76]).unwrap();
    let result = gate.run_loaded().unwrap();
    assert!(result.halted);
    assert_eq!(gate.output_port(7), 0xA5);
    let owned = gate.snapshot();
    assert!(owned.serial_input);
    assert_eq!(owned.core.output_ports[7], 0xA5);

    let maximum = gate.max_retained_trace_steps();
    assert!(maximum > 0);
    let before = gate.snapshot();
    assert_eq!(
        gate.run_loaded_with_limit(maximum + 1),
        Err(Intel8085Error::TraceLimitTooLarge {
            requested: maximum + 1,
            maximum,
        })
    );
    assert_eq!(gate.snapshot(), before);

    gate.reset();
    let reset = gate.snapshot();
    assert!(reset.core.memory.iter().all(|byte| *byte == 0));
    assert_eq!(reset.core.input_ports, [0; 256]);
    assert_eq!(reset.restart_masks, RestartMasks::masked());
    assert_eq!(reset.loaded_len, 0);
    assert!(!reset.serial_input);
}

#[test]
fn remaining_public_control_paths_are_typed_and_complete() {
    let mut gate = GateLevelCpu::default();
    gate.load_program(&[0x30, 0x76]).unwrap();
    let mut state = gate.snapshot();
    state.core.regs.a = 0b1100_1111;
    gate.restore(&state).unwrap();
    gate.step().unwrap();
    let sim = gate.snapshot();
    assert_eq!(sim.restart_masks, RestartMasks::masked());
    assert!(sim.serial_output);
    gate.step().unwrap();
    let halted = gate.snapshot();
    assert_eq!(gate.step(), Err(Intel8085Error::Halted));
    assert_eq!(gate.snapshot(), halted);

    let mut pending = GateLevelCpu::new();
    pending.request_interrupt(InterruptRequest::Trap);
    pending.request_interrupt(InterruptRequest::Rst75);
    pending.request_interrupt(InterruptRequest::Rst65);
    pending.request_interrupt(InterruptRequest::Rst55);
    let requests = pending.pending();
    assert!(requests.trap && requests.rst75 && requests.rst65 && requests.rst55);

    for (request, vector) in [
        (InterruptRequest::Rst65, 0x0034),
        (InterruptRequest::Rst55, 0x002C),
    ] {
        let mut machine = GateLevelCpu::new();
        machine.load(&[0]).unwrap();
        let mut state = machine.snapshot();
        state.core.regs.sp = 0x2000;
        state.core.interrupts_enabled = true;
        state.restart_masks = RestartMasks::unmasked();
        machine.restore(&state).unwrap();
        machine.request_interrupt(request);
        machine.step().unwrap();
        assert_eq!(machine.snapshot().core.pc, vector);
    }

    for number in 0u8..8 {
        let mut machine = GateLevelCpu::new();
        machine.load(&[0]).unwrap();
        let mut state = machine.snapshot();
        state.core.regs.sp = 0x3000;
        state.core.interrupts_enabled = true;
        machine.restore(&state).unwrap();
        machine.request_interrupt(InterruptRequest::Intr(0xC7 | (number << 3)));
        let trace = machine.step().unwrap();
        assert_eq!(trace.mnemonic, format!("INT INTR/RST{number}"));
        assert_eq!(machine.snapshot().core.pc, u16::from(number) * 8);
    }

    let mut malformed = gate.snapshot();
    malformed.loaded_origin = 0xFFFF;
    malformed.loaded_len = 2;
    let before = gate.snapshot();
    assert_eq!(
        gate.restore(&malformed),
        Err(Intel8085Error::ProgramOutOfRange {
            origin: 0xFFFF,
            length: 2,
            memory_size: 65_536,
        })
    );
    assert_eq!(gate.snapshot(), before);

    assert_eq!(
        gate.run(&vec![0; 65_537], 1),
        Err(Intel8085Error::ProgramOutOfRange {
            origin: 0,
            length: 65_537,
            memory_size: 65_536,
        })
    );
    assert_eq!(gate.snapshot(), before);
}
