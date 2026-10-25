use vax11_780_simulator::{
    MAX_RETAINED_TRACES, MEMORY_SIZE, Psl, REGISTER_PC, REGISTER_SP, VaxError, VaxSimulator,
    encoding,
};

fn execute_with_registers(program: Vec<u8>, registers: &[(usize, u32)]) -> String {
    let mut cpu = VaxSimulator::new();
    cpu.load_checked(&program).unwrap();
    for &(index, value) in registers {
        cpu.write_register_checked(index, value).unwrap();
    }
    cpu.step_checked().unwrap().mnemonic
}

#[test]
fn every_arithmetic_and_data_opcode_family_executes() {
    for (base, width_suffix) in [(0x80_u8, 'B'), (0xa0, 'W'), (0xc0, 'L')] {
        for offset in 0_u8..=14 {
            let operands = match offset {
                14 => vec![encoding::literal(2), encoding::register(1)],
                odd if odd & 1 == 1 => vec![
                    encoding::literal(2),
                    encoding::register(0),
                    encoding::register(1),
                ],
                _ => vec![encoding::literal(2), encoding::register(0)],
            };
            let program = encoding::assemble(&[
                encoding::instruction(base + offset, &operands),
                vec![encoding::HALT],
            ]);
            let name = execute_with_registers(program, &[(0, 9)]);
            assert!(
                name.ends_with(width_suffix)
                    || name.ends_with(&format!("{width_suffix}2"))
                    || name.ends_with(&format!("{width_suffix}3"))
            );
        }

        for offset in 0_u8..=7 {
            let operands = match offset {
                0 | 2 => vec![encoding::register(0), encoding::register(1)],
                1 | 3 => vec![encoding::literal(3), encoding::register(0)],
                4 | 6 | 7 => vec![encoding::register(0)],
                5 => vec![encoding::register(0)],
                _ => unreachable!(),
            };
            let program = encoding::assemble(&[
                encoding::instruction(base + 0x10 + offset, &operands),
                vec![encoding::HALT],
            ]);
            let name = execute_with_registers(program, &[(0, 9)]);
            assert!(name.ends_with(width_suffix));
        }

        let case_program = encoding::assemble(&[
            encoding::instruction(
                base + 15,
                &[
                    encoding::literal(5),
                    encoding::literal(0),
                    encoding::literal(2),
                ],
            ),
            vec![0; 6],
            vec![encoding::HALT],
        ]);
        let result = VaxSimulator::new().run_checked(&case_program, 3).unwrap();
        assert!(result.halted);
        assert_eq!(
            result.final_state.psl,
            Psl {
                n: false,
                z: false,
                v: false,
                c: false,
            }
        );
    }
}

#[test]
fn case_table_taken_path_and_all_branch_conditions_execute() {
    for opcode in [0x8f_u8, 0xaf, 0xcf] {
        let program = encoding::assemble(&[
            encoding::instruction(
                opcode,
                &[
                    encoding::literal(0),
                    encoding::literal(0),
                    encoding::literal(1),
                ],
            ),
            vec![2, 0],
            vec![encoding::HALT],
        ]);
        let result = VaxSimulator::new().run_checked(&program, 3).unwrap();
        assert!(result.halted);
        assert_eq!(
            result.final_state.psl,
            Psl {
                n: true,
                z: false,
                v: false,
                c: true,
            }
        );
    }

    for opcode in [
        0x12_u8, 0x13, 0x14, 0x15, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
    ] {
        let mut cpu = VaxSimulator::new();
        cpu.load_checked(&[opcode, 0, encoding::HALT]).unwrap();
        cpu.set_psl(Psl {
            n: false,
            z: false,
            v: false,
            c: false,
        });
        assert!(!cpu.step_checked().unwrap().mnemonic.is_empty());
    }

    let mut cpu = VaxSimulator::new();
    cpu.load_checked(&encoding::assemble(&[
        encoding::branch_word(0x30, 1),
        vec![encoding::HALT],
        vec![0x05],
        vec![encoding::HALT],
    ]))
    .unwrap();
    cpu.write_register_checked(REGISTER_SP, 0x400).unwrap();
    assert_eq!(cpu.step_checked().unwrap().mnemonic, "BSBW");
    assert_eq!(cpu.step_checked().unwrap().mnemonic, "RSB");

    assert_eq!(
        execute_with_registers(
            encoding::assemble(&[encoding::branch_word(0x31, 0), vec![encoding::HALT]]),
            &[],
        ),
        "BRW"
    );
}

#[test]
fn control_transfer_and_address_instructions_execute() {
    let jump = encoding::assemble(&[
        encoding::instruction(0x17, &[encoding::absolute(7)]),
        vec![encoding::HALT],
        vec![encoding::HALT],
    ]);
    assert_eq!(VaxSimulator::new().run_checked(&jump, 3).unwrap().steps, 2);

    let subroutine = encoding::assemble(&[
        encoding::instruction(0x16, &[encoding::absolute(7)]),
        vec![encoding::HALT],
        vec![0x05],
    ]);
    let mut cpu = VaxSimulator::new();
    cpu.load_checked(&subroutine).unwrap();
    cpu.write_register_checked(REGISTER_SP, 0x400).unwrap();
    assert_eq!(cpu.run_loaded_checked(4).unwrap().steps, 3);

    for (opcode, expected) in [
        (0x9e_u8, "MOVAB"),
        (0x9f, "PUSHAB"),
        (0x3e, "MOVAW"),
        (0x3f, "PUSHAW"),
        (0xde, "MOVAL"),
        (0xdf, "PUSHAL"),
    ] {
        let push = opcode & 1 == 1;
        let mut operands = vec![encoding::absolute(0x200)];
        if !push {
            operands.push(encoding::register(0));
        }
        let program = encoding::assemble(&[
            encoding::instruction(opcode, &operands),
            vec![encoding::HALT],
        ]);
        let mut cpu = VaxSimulator::new();
        cpu.load_checked(&program).unwrap();
        cpu.write_register_checked(REGISTER_SP, 0x400).unwrap();
        assert_eq!(cpu.step_checked().unwrap().mnemonic, expected);
    }

    let movzwl = encoding::assemble(&[
        encoding::instruction(0x3c, &[encoding::literal(9), encoding::register(0)]),
        vec![encoding::HALT],
    ]);
    assert_eq!(execute_with_registers(movzwl, &[]), "MOVZWL");
}

#[test]
fn remaining_displacement_and_deferred_modes_execute() {
    let program = encoding::assemble(&[
        encoding::movl(encoding::word_displacement(0, 4), encoding::register(4)),
        encoding::movl(encoding::long_displacement(0, 8), encoding::register(5)),
        encoding::movl(vec![0xb0, 12], encoding::register(6)),
        encoding::movl(vec![0xd0, 16, 0], encoding::register(7)),
        encoding::movl(vec![0xf0, 20, 0, 0, 0], encoding::register(8)),
        vec![encoding::HALT],
    ]);
    let mut cpu = VaxSimulator::new();
    cpu.load_checked(&program).unwrap();
    cpu.write_register_checked(0, 0x200).unwrap();
    cpu.write_long_checked(0x204, 1).unwrap();
    cpu.write_long_checked(0x208, 2).unwrap();
    for (pointer, target, value) in [(0x20c, 0x300, 3), (0x210, 0x304, 4), (0x214, 0x308, 5)] {
        cpu.write_long_checked(pointer, target).unwrap();
        cpu.write_long_checked(target, value).unwrap();
    }
    let result = cpu.run_loaded_checked(16).unwrap();
    assert_eq!(&result.final_state.regs[4..=8], &[1, 2, 3, 4, 5]);

    let pc_relative = [
        0xd0, 0xaf, 3, 0x50, 0x00, 0x00, 0x78, 0x56, 0x34, 0x12, 0x00,
    ];
    let result = VaxSimulator::new().run_checked(&pc_relative, 3).unwrap();
    assert_eq!(result.final_state.regs[0], 0x1234_5678);

    assert_eq!(encoding::pc_relative_byte(-1), vec![0xaf, 0xff]);
}

#[test]
fn byte_and_word_memory_destinations_cover_partial_memory_access() {
    let program = encoding::assemble(&[
        encoding::instruction(0x90, &[encoding::literal(0x2a), encoding::absolute(0x200)]),
        encoding::instruction(0xb0, &[encoding::literal(0x2b), encoding::absolute(0x201)]),
        vec![encoding::HALT],
    ]);
    let result = VaxSimulator::new().run_checked(&program, 4).unwrap();
    assert_eq!(result.final_state.memory[0x200], 0x2a);
    assert_eq!(&result.final_state.memory[0x201..0x203], &[0x2b, 0]);

    let mut cpu = VaxSimulator::new();
    cpu.write_byte_checked(9, 0xaa).unwrap();
    cpu.write_word_checked(10, 0xbbcc).unwrap();
    assert_eq!(cpu.read_byte_checked(9).unwrap(), 0xaa);
    assert_eq!(cpu.read_word_checked(10).unwrap(), 0xbbcc);
    cpu.reset();
    assert_eq!(cpu.read_byte_checked(9).unwrap(), 0);
    assert_eq!(cpu.psl(), Psl::default());
    assert_eq!(VaxSimulator::default().get_state(), cpu.get_state());
}

#[test]
fn increment_decrement_and_pushes_set_architectural_condition_codes() {
    let program = encoding::assemble(&[
        encoding::instruction(0x96, &[encoding::register(0)]),
        encoding::instruction(0xb7, &[encoding::register(1)]),
        encoding::instruction(0xdd, &[encoding::register(2)]),
        vec![encoding::HALT],
    ]);
    let mut cpu = VaxSimulator::new();
    cpu.load_checked(&program).unwrap();
    cpu.write_register_checked(0, 0xff).unwrap();
    cpu.write_register_checked(1, 0).unwrap();
    cpu.write_register_checked(2, 0x8000_0000).unwrap();
    cpu.write_register_checked(REGISTER_SP, 0x400).unwrap();

    let increment = cpu.step_checked().unwrap();
    assert!(increment.state_after.psl.c);
    assert!(increment.state_after.psl.z);
    let decrement = cpu.step_checked().unwrap();
    assert!(decrement.state_after.psl.c);
    assert!(decrement.state_after.psl.n);
    let push = cpu.step_checked().unwrap();
    assert!(push.state_after.psl.n);
    assert!(!push.state_after.psl.z);
    assert!(!push.state_after.psl.v);
    assert!(push.state_after.psl.c);

    let push_address = encoding::assemble(&[
        encoding::instruction(0x9f, &[encoding::absolute(0)]),
        vec![encoding::HALT],
    ]);
    let mut cpu = VaxSimulator::new();
    cpu.load_checked(&push_address).unwrap();
    cpu.write_register_checked(REGISTER_SP, 0x400).unwrap();
    cpu.set_psl(Psl {
        n: true,
        z: false,
        v: true,
        c: true,
    });
    let trace = cpu.step_checked().unwrap();
    assert_eq!(
        trace.state_after.psl,
        Psl {
            n: false,
            z: true,
            v: false,
            c: true,
        }
    );
}

#[test]
fn fault_surface_and_error_messages_are_deterministic() {
    let errors = [
        VaxError::Halted,
        VaxError::InvalidRegister { index: 17 },
        VaxError::InvalidState("bad state".to_owned()),
        VaxError::ProgramOutOfRange {
            origin: 1,
            length: 2,
            memory_size: 3,
        },
        VaxError::FetchOutsideProgram { pc: 4 },
        VaxError::TruncatedInstruction { pc: 5 },
        VaxError::MemoryOutOfRange {
            address: 6,
            width: 4,
        },
        VaxError::StackOutOfRange { sp: 7 },
        VaxError::UnknownOpcode { opcode: 2, pc: 8 },
        VaxError::UnsupportedTwoByteOpcode {
            prefix: 0xfc,
            pc: 9,
        },
        VaxError::ReservedAddressingMode {
            specifier: 0x7f,
            pc: 10,
        },
        VaxError::RecursiveIndexMode { pc: 11 },
        VaxError::InvalidDestinationMode {
            specifier: 1,
            pc: 12,
        },
        VaxError::DivideByZero { pc: 13 },
        VaxError::StepLimitExceeded { max_steps: 14 },
        VaxError::TraceLimitTooLarge {
            requested: 16,
            maximum: 15,
        },
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }

    let mut cpu = VaxSimulator::new();
    cpu.load_checked(&[encoding::HALT]).unwrap();
    cpu.write_register_width_checked(0, 3, 0).unwrap_err();
    assert_eq!(
        cpu.read_byte_checked(MEMORY_SIZE as u32),
        Err(VaxError::MemoryOutOfRange {
            address: MEMORY_SIZE as u32,
            width: 1
        })
    );
    assert_eq!(
        cpu.write_word_checked((MEMORY_SIZE - 1) as u32, 0),
        Err(VaxError::MemoryOutOfRange {
            address: (MEMORY_SIZE - 1) as u32,
            width: 2
        })
    );
    assert_eq!(
        cpu.run_loaded_checked(MAX_RETAINED_TRACES + 1),
        Err(VaxError::TraceLimitTooLarge {
            requested: MAX_RETAINED_TRACES + 1,
            maximum: MAX_RETAINED_TRACES
        })
    );

    cpu.write_register_checked(REGISTER_PC, 2).unwrap();
    assert_eq!(
        cpu.step_checked(),
        Err(VaxError::FetchOutsideProgram { pc: 2 })
    );

    for (program, setup, expected) in [
        (
            vec![0x02],
            None,
            VaxError::UnknownOpcode { opcode: 2, pc: 0 },
        ),
        (
            vec![0xd0, 0x7f, 0x50],
            None,
            VaxError::ReservedAddressingMode {
                specifier: 0x7f,
                pc: 0,
            },
        ),
        (
            vec![0xd0, 0x5f, 0x50],
            None,
            VaxError::ReservedAddressingMode {
                specifier: 0x5f,
                pc: 0,
            },
        ),
        (
            vec![0xd0, 0x6f, 0x50],
            None,
            VaxError::ReservedAddressingMode {
                specifier: 0x6f,
                pc: 0,
            },
        ),
        (
            vec![0xd0, 0x40, 0x40, 0x60, 0x50],
            None,
            VaxError::RecursiveIndexMode { pc: 0 },
        ),
        (
            vec![0x17, 0x50],
            None,
            VaxError::InvalidDestinationMode {
                specifier: 0x50,
                pc: 0,
            },
        ),
        (
            vec![0xd0, 0x40, 0x70, 0x50],
            None,
            VaxError::ReservedAddressingMode {
                specifier: 0x70,
                pc: 0,
            },
        ),
        (
            vec![0xd0, 0x41, 0x81, 0x50],
            None,
            VaxError::ReservedAddressingMode {
                specifier: 0x81,
                pc: 0,
            },
        ),
        (
            vec![0xd0, 0x42, 0x92, 0x50],
            None,
            VaxError::ReservedAddressingMode {
                specifier: 0x92,
                pc: 0,
            },
        ),
        (
            vec![0x10, 0],
            Some((REGISTER_SP, 0)),
            VaxError::StackOutOfRange { sp: u32::MAX - 3 },
        ),
        (
            vec![0xd0, 0x60, 0x50],
            Some((0, u32::MAX)),
            VaxError::MemoryOutOfRange {
                address: u32::MAX,
                width: 4,
            },
        ),
    ] {
        let mut cpu = VaxSimulator::new();
        cpu.load_checked(&program).unwrap();
        if let Some((register, value)) = setup {
            cpu.write_register_checked(register, value).unwrap();
        }
        let before = cpu.get_state();
        assert_eq!(cpu.step_checked(), Err(expected));
        assert_eq!(cpu.get_state(), before);
    }
}

#[test]
fn successful_restore_and_transactional_run_error_paths_execute() {
    let mut cpu = VaxSimulator::new();
    cpu.write_register_checked(0, 7).unwrap();
    let state = cpu.get_state();
    cpu.reset();
    cpu.restore(&state).unwrap();
    assert_eq!(cpu.read_register_checked(0).unwrap(), 7);

    let mut invalid = state.clone();
    invalid.loaded_origin = u32::MAX;
    invalid.loaded_len = 2;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(VaxError::InvalidState(_))
    ));

    let before = cpu.get_state();
    assert!(matches!(
        cpu.run_checked(&[0x02], 1),
        Err(VaxError::UnknownOpcode { .. })
    ));
    assert_eq!(cpu.get_state(), before);
}
