use intel8080_simulator::{Intel8080Error, Intel8080Simulator};

const UNDEFINED: [u8; 12] = [
    0x08, 0x10, 0x18, 0x20, 0x28, 0x30, 0x38, 0xCB, 0xD9, 0xDD, 0xED, 0xFD,
];

fn encoded_length(opcode: u8) -> Option<usize> {
    if UNDEFINED.contains(&opcode) {
        None
    } else if opcode & 0xC7 == 0x06 || opcode & 0xC7 == 0xC6 || matches!(opcode, 0xD3 | 0xDB) {
        Some(2)
    } else if opcode & 0xCF == 0x01
        || opcode & 0xC7 == 0xC2
        || opcode & 0xC7 == 0xC4
        || matches!(opcode, 0x22 | 0x2A | 0x32 | 0x3A | 0xC3 | 0xCD)
    {
        Some(3)
    } else {
        Some(1)
    }
}

#[test]
fn every_encoding_has_the_expected_checked_boundary() {
    for opcode in 0u8..=u8::MAX {
        let mut simulator = Intel8080Simulator::new(65_536);
        simulator.load_program(&[opcode, 0, 0]).unwrap();
        simulator.regs.sp = 0x4000;
        let before = simulator.snapshot();
        match encoded_length(opcode) {
            Some(length) => {
                let trace = simulator.step().unwrap_or_else(|error| {
                    panic!("defined opcode {opcode:#04X} was rejected: {error}")
                });
                assert_eq!(trace.address, 0);
                assert_eq!(trace.raw.len(), length, "opcode {opcode:#04X}");
                assert_eq!(trace.state_before, before);
                assert_eq!(trace.state_after, simulator.snapshot());
            }
            None => {
                assert_eq!(
                    simulator.step(),
                    Err(Intel8080Error::UnknownOpcode { address: 0, opcode })
                );
                assert_eq!(simulator.snapshot(), before);
            }
        }
    }
}

#[test]
fn load_step_and_run_failures_are_typed_and_atomic() {
    let mut simulator = Intel8080Simulator::new(4);
    simulator.input_ports[7] = 0xA5;
    simulator.load_program(&[0x76]).unwrap();
    let before = simulator.snapshot();
    assert_eq!(
        simulator.load_program(&[0; 5]),
        Err(Intel8080Error::ProgramOutOfRange {
            length: 5,
            memory_size: 4,
        })
    );
    assert_eq!(simulator.snapshot(), before);

    simulator.pc = 3;
    simulator.mem.write_byte(3, 0x3E);
    let truncated = simulator.snapshot();
    assert_eq!(
        simulator.step(),
        Err(Intel8080Error::TruncatedInstruction {
            address: 3,
            expected: 2,
            available: 1,
        })
    );
    assert_eq!(simulator.snapshot(), truncated);

    let mut data_access = Intel8080Simulator::new(8);
    data_access.load_program(&[0x7E]).unwrap(); // MOV A,M
    data_access.regs.h = 0x01;
    let data_before = data_access.snapshot();
    assert_eq!(
        data_access.step(),
        Err(Intel8080Error::MemoryOutOfRange {
            address: 0x0100,
            memory_size: 8,
        })
    );
    assert_eq!(data_access.snapshot(), data_before);

    let mut transactional = Intel8080Simulator::new(65_536);
    transactional.regs.a = 99;
    transactional.input_ports[3] = 0x5A;
    let transaction_before = transactional.snapshot();
    assert_eq!(
        transactional.run(&[0x3E, 7, 0x08], 10),
        Err(Intel8080Error::UnknownOpcode {
            address: 2,
            opcode: 0x08,
        })
    );
    assert_eq!(transactional.snapshot(), transaction_before);
}

#[test]
fn run_is_bounded_deterministic_and_snapshots_are_owned() {
    let program = [0x3E, 9, 0xD3, 17, 0x76]; // MVI A,9; OUT 17; HLT
    let mut simulator = Intel8080Simulator::new(65_536);
    simulator.input_ports[2] = 0xCC;
    let first = simulator.run(&program, 10).unwrap();
    let first_state = simulator.snapshot();
    assert!(first.halted);
    assert_eq!(first.steps, 3);
    assert_eq!(first.traces.len(), 3);
    assert_eq!(first.final_state, first_state);
    assert_eq!(first_state.output_ports[17], 9);
    assert_eq!(first_state.input_ports[2], 0xCC);

    let second = simulator.run(&program, 10).unwrap();
    assert_eq!(second, first);
    assert_eq!(simulator.snapshot(), first_state);

    simulator.regs.a = 0;
    simulator.output_ports[17] = 0;
    assert_eq!(first_state.regs.a, 9, "prior snapshot remains owned");
    assert_eq!(first_state.output_ports[17], 9);

    simulator.reset();
    let reset = simulator.snapshot();
    assert_eq!(reset.memory.len(), 65_536);
    assert!(reset.memory.iter().all(|byte| *byte == 0));
    assert_eq!(reset.input_ports, [0; 256]);
    assert_eq!(reset.output_ports, [0; 256]);
}

fn hash_bytes(hash: &mut u64, bytes: impl IntoIterator<Item = u8>) {
    for byte in bytes {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(0x100000001B3);
    }
}

#[test]
fn all_244_defined_opcodes_match_the_python_oracle_full_state() {
    // Generated from the repository's Python reference simulator using the
    // same initialized registers, flags, memory, ports, and operand bytes.
    let mut hash = 0xCBF29CE484222325_u64;
    for opcode in 0u8..=u8::MAX {
        if UNDEFINED.contains(&opcode) {
            continue;
        }
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
        simulator.load_program(&[opcode, 0x00, 0x20]).unwrap();
        simulator.mem.write_byte(0x2000, 0x5A);
        simulator.mem.write_byte(0x2100, 0xC3);
        simulator.mem.write_byte(0x4000, 0x78);
        simulator.mem.write_byte(0x4001, 0x56);
        simulator.input_ports[0] = 0xA5;
        simulator.step().unwrap();
        let state = simulator.snapshot();

        hash_bytes(
            &mut hash,
            [
                opcode,
                state.regs.a,
                state.regs.b,
                state.regs.c,
                state.regs.d,
                state.regs.e,
                state.regs.h,
                state.regs.l,
            ],
        );
        hash_bytes(&mut hash, state.regs.sp.to_le_bytes());
        hash_bytes(&mut hash, state.pc.to_le_bytes());
        hash_bytes(
            &mut hash,
            [
                state.flags.s as u8,
                state.flags.z as u8,
                state.flags.ac as u8,
                state.flags.p as u8,
                state.flags.cy as u8,
                state.interrupts_enabled as u8,
                state.halted as u8,
            ],
        );
        hash_bytes(&mut hash, state.memory.iter().copied());
        hash_bytes(&mut hash, state.input_ports);
        hash_bytes(&mut hash, state.output_ports);
    }
    assert_eq!(hash, 0x4D1CEBC3531637D0);
}
