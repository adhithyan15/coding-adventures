use vax11_780_gatelevel::{
    encoding, Psl, VaxError, VaxGateLevel, VaxSimulator, ESTIMATED_GATE_COUNT, FLIP_FLOP_COUNT,
    MAX_RETAINED_TRACES, MEMORY_SIZE, REGISTER_PC, REGISTER_SP,
};

fn compare_one(functional: &mut VaxSimulator) {
    let before = functional.get_state();
    let mut gate = VaxGateLevel::new();
    gate.restore(&before).unwrap();
    let gate_trace = gate.step_checked().unwrap();
    let functional_trace = functional.step_checked().unwrap();
    assert_eq!(gate_trace, functional_trace);
    assert_eq!(gate.get_state(), functional.get_state());
}

fn loaded(program: &[u8]) -> VaxSimulator {
    let mut functional = VaxSimulator::new();
    functional.load_checked(program).unwrap();
    functional
}

#[test]
fn exact_topology_and_public_state_are_dff_backed() {
    assert_eq!(FLIP_FLOP_COUNT, 524_854);
    let mut gate = VaxGateLevel::default();
    assert_eq!(gate.flip_flop_count(), FLIP_FLOP_COUNT);
    assert_eq!(gate.gate_count(), ESTIMATED_GATE_COUNT);
    gate.write_register_checked(0, 0x1234_5678).unwrap();
    gate.write_register_width_checked(0, 1, 0xaa).unwrap();
    gate.write_word_checked(3, 0xbbcc).unwrap();
    gate.write_long_checked(9, 0xfeed_face).unwrap();
    gate.set_psl(Psl {
        n: true,
        z: false,
        v: true,
        c: false,
    });
    assert_eq!(gate.read_register_checked(0).unwrap(), 0x1234_56aa);
    assert_eq!(gate.read_word_checked(3).unwrap(), 0xbbcc);
    assert_eq!(gate.read_long_checked(9).unwrap(), 0xfeed_face);
    assert!(gate.psl().n && gate.psl().v);
    assert_eq!(
        gate.read_register_checked(16),
        Err(VaxError::InvalidRegister { index: 16 })
    );
    gate.reset();
    assert_eq!(gate.get_state(), VaxGateLevel::new().get_state());
}

#[test]
fn every_integer_opcode_family_matches_complete_functional_transitions() {
    for (base, width) in [(0x80_u8, 1_usize), (0xa0, 2), (0xc0, 4)] {
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
            let mut functional = loaded(&program);
            functional.write_register_checked(0, 9).unwrap();
            functional.write_register_checked(1, 7).unwrap();
            compare_one(&mut functional);
        }

        for offset in 0_u8..=7 {
            let operands = match offset {
                0 | 2 => vec![encoding::register(0), encoding::register(1)],
                1 | 3 => vec![encoding::literal(3), encoding::register(0)],
                4..=7 => vec![encoding::register(0)],
                _ => unreachable!(),
            };
            let program = encoding::assemble(&[
                encoding::instruction(base + 0x10 + offset, &operands),
                vec![encoding::HALT],
            ]);
            let mut functional = loaded(&program);
            functional
                .write_register_checked(0, if width == 1 { 0xff } else { 9 })
                .unwrap();
            compare_one(&mut functional);
        }
    }
}

#[test]
fn complete_addressing_grammar_matches_functional_state_and_trace() {
    let program = encoding::assemble(&[
        encoding::movl(encoding::literal(42), encoding::register(4)),
        encoding::movl(encoding::immediate_long(0x1234_5678), encoding::register(5)),
        encoding::movl(encoding::register_deferred(0), encoding::register(6)),
        encoding::movl(encoding::autoincrement(0), encoding::register(7)),
        encoding::movl(encoding::autodecrement(1), encoding::register(8)),
        encoding::movl(encoding::absolute(0x300), encoding::register(9)),
        encoding::movl(encoding::byte_displacement(2, 8), encoding::register(10)),
        encoding::movl(encoding::word_displacement(2, 12), encoding::register(11)),
        encoding::movl(encoding::long_displacement(2, 16), encoding::register(12)),
        encoding::movl(
            encoding::indexed(3, encoding::register_deferred(2)),
            encoding::register(13),
        ),
        encoding::movl(encoding::autoincrement_deferred(14), encoding::register(0)),
        encoding::movl(
            encoding::immediate_long(0xdead_beef),
            encoding::absolute(0x340),
        ),
        vec![encoding::HALT],
    ]);
    let mut functional = loaded(&program);
    functional.write_register_checked(0, 0x200).unwrap();
    functional.write_register_checked(1, 0x208).unwrap();
    functional.write_register_checked(2, 0x220).unwrap();
    functional.write_register_checked(3, 2).unwrap();
    functional.write_register_checked(14, 0x250).unwrap();
    for (address, value) in [
        (0x200, 11),
        (0x204, 12),
        (0x208, 13),
        (0x228, 14),
        (0x22c, 15),
        (0x230, 16),
        (0x300, 17),
        (0x250, 0x300),
    ] {
        functional.write_long_checked(address, value).unwrap();
    }
    let mut gate = VaxGateLevel::new();
    gate.restore(&functional.get_state()).unwrap();
    while !functional.get_state().halted {
        assert_eq!(
            gate.step_checked().unwrap(),
            functional.step_checked().unwrap()
        );
        assert_eq!(gate.get_state(), functional.get_state());
    }
}

#[test]
fn control_flow_case_address_and_stack_families_match() {
    for opcode in [
        0x12_u8, 0x13, 0x14, 0x15, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
    ] {
        let mut functional = loaded(&[opcode, 0, encoding::HALT]);
        functional.set_psl(Psl::default());
        compare_one(&mut functional);
    }

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
        let mut functional = loaded(&program);
        compare_one(&mut functional);
    }

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
        let mut functional = loaded(&program);
        functional
            .write_register_checked(REGISTER_SP, 0x400)
            .unwrap();
        let before = functional.get_state();
        let mut gate = VaxGateLevel::new();
        gate.restore(&before).unwrap();
        assert_eq!(gate.step_checked().unwrap().mnemonic, expected);
        functional.step_checked().unwrap();
        assert_eq!(gate.get_state(), functional.get_state());
    }
}

#[test]
fn lifecycle_faults_and_bounded_runs_are_typed_and_atomic() {
    let mut gate = VaxGateLevel::new();
    gate.load_checked(&[0x01, 0x02]).unwrap();
    let before = gate.get_state();
    assert_eq!(
        gate.run_loaded_checked(8),
        Err(VaxError::UnknownOpcode { opcode: 2, pc: 1 })
    );
    assert_eq!(gate.get_state(), before);

    for program in [&[0xfc][..], &[0xd0][..], &[0x02][..]] {
        gate.load_checked(program).unwrap();
        let before = gate.get_state();
        assert!(gate.step_checked().is_err());
        assert_eq!(gate.get_state(), before);
    }

    assert_eq!(
        gate.load_at_checked(&[0; 8], (MEMORY_SIZE - 4) as u32),
        Err(VaxError::ProgramOutOfRange {
            origin: (MEMORY_SIZE - 4) as u32,
            length: 8,
            memory_size: MEMORY_SIZE,
        })
    );
    assert_eq!(gate.read_register_checked(REGISTER_PC).unwrap(), 0);
}

#[test]
fn checked_state_memory_and_loading_contracts_are_complete_and_atomic() {
    let mut gate = VaxGateLevel::new();
    gate.write_register_checked(0, 0x1234_5678).unwrap();
    gate.write_register_width_checked(0, 1, 0xaa).unwrap();
    gate.write_register_width_checked(0, 2, 0xbbcc).unwrap();
    assert_eq!(gate.read_register_checked(0).unwrap(), 0x1234_bbcc);
    assert!(matches!(
        gate.write_register_width_checked(0, 3, 0),
        Err(VaxError::InvalidState(_))
    ));

    gate.write_byte_checked(3, 0x11).unwrap();
    gate.write_word_checked(4, 0x2233).unwrap();
    gate.write_long_checked(7, 0x4455_6677).unwrap();
    assert_eq!(gate.read_byte_checked(3).unwrap(), 0x11);
    assert_eq!(gate.read_word_checked(4).unwrap(), 0x2233);
    assert_eq!(gate.read_long_checked(7).unwrap(), 0x4455_6677);
    assert!(matches!(
        gate.read_byte_checked(MEMORY_SIZE as u32),
        Err(VaxError::MemoryOutOfRange { .. })
    ));
    assert!(matches!(
        gate.write_word_checked((MEMORY_SIZE - 1) as u32, 0),
        Err(VaxError::MemoryOutOfRange { .. })
    ));
    assert!(matches!(
        gate.write_long_checked((MEMORY_SIZE - 3) as u32, 0),
        Err(VaxError::MemoryOutOfRange { .. })
    ));

    let valid = gate.get_state();
    let mut invalid = valid.clone();
    invalid.memory.pop();
    assert!(matches!(
        gate.restore(&invalid),
        Err(VaxError::InvalidState(_))
    ));
    assert_eq!(gate.get_state(), valid);

    gate.load_at_checked(&[encoding::HALT], 0x100).unwrap();
    assert_eq!(gate.read_register_checked(REGISTER_PC).unwrap(), 0x100);
    assert_eq!(gate.get_state().loaded_origin, 0x100);
    assert_eq!(gate.run_loaded_checked(2).unwrap().steps, 1);
    assert_eq!(
        gate.run_checked(&[encoding::NOP, encoding::HALT], 2)
            .unwrap()
            .steps,
        2
    );
    assert_eq!(gate.get_state().loaded_origin, 0);
}

#[test]
fn all_remaining_addressing_and_partial_memory_paths_match() {
    let program = encoding::assemble(&[
        encoding::movl(encoding::word_displacement(0, 4), encoding::register(4)),
        encoding::movl(encoding::long_displacement(0, 8), encoding::register(5)),
        encoding::movl(vec![0xb0, 12], encoding::register(6)),
        encoding::movl(vec![0xd0, 16, 0], encoding::register(7)),
        encoding::movl(vec![0xf0, 20, 0, 0, 0], encoding::register(8)),
        encoding::instruction(0x90, &[encoding::literal(0x2a), encoding::absolute(0x380)]),
        encoding::instruction(0xb0, &[encoding::literal(0x2b), encoding::absolute(0x381)]),
        vec![encoding::HALT],
    ]);
    let mut functional = loaded(&program);
    functional.write_register_checked(0, 0x200).unwrap();
    functional.write_long_checked(0x204, 1).unwrap();
    functional.write_long_checked(0x208, 2).unwrap();
    for (pointer, target, value) in [(0x20c, 0x300, 3), (0x210, 0x304, 4), (0x214, 0x308, 5)] {
        functional.write_long_checked(pointer, target).unwrap();
        functional.write_long_checked(target, value).unwrap();
    }
    let mut gate = VaxGateLevel::new();
    gate.restore(&functional.get_state()).unwrap();
    while !functional.get_state().halted {
        assert_eq!(
            gate.step_checked().unwrap(),
            functional.step_checked().unwrap()
        );
        assert_eq!(gate.get_state(), functional.get_state());
    }
    assert_eq!(&gate.get_state().memory[0x380..0x383], &[0x2a, 0x2b, 0]);

    let pc_relative = [
        0xd0, 0xaf, 3, 0x50, 0x00, 0x00, 0x78, 0x56, 0x34, 0x12, 0x00,
    ];
    let mut functional = loaded(&pc_relative);
    compare_one(&mut functional);
}

#[test]
fn subroutines_wide_branches_zero_extension_and_push_match() {
    let program = encoding::assemble(&[
        encoding::instruction(0x9a, &[encoding::literal(9), encoding::register(0)]),
        encoding::instruction(0x9b, &[encoding::literal(10), encoding::register(1)]),
        encoding::instruction(0x3c, &[encoding::literal(11), encoding::register(2)]),
        encoding::instruction(0xdd, &[encoding::register(2)]),
        vec![encoding::HALT],
    ]);
    let mut functional = loaded(&program);
    functional
        .write_register_checked(REGISTER_SP, 0x400)
        .unwrap();
    let mut gate = VaxGateLevel::new();
    gate.restore(&functional.get_state()).unwrap();
    while !functional.get_state().halted {
        assert_eq!(
            gate.step_checked().unwrap(),
            functional.step_checked().unwrap()
        );
    }
    assert_eq!(gate.get_state(), functional.get_state());

    let jump = encoding::assemble(&[
        encoding::instruction(0x17, &[encoding::absolute(7)]),
        vec![encoding::HALT],
        vec![encoding::HALT],
    ]);
    assert_eq!(VaxGateLevel::new().run_checked(&jump, 3).unwrap().steps, 2);

    let subroutine = encoding::assemble(&[
        encoding::instruction(0x16, &[encoding::absolute(7)]),
        vec![encoding::HALT],
        vec![0x05],
    ]);
    let mut gate = VaxGateLevel::new();
    gate.load_checked(&subroutine).unwrap();
    gate.write_register_checked(REGISTER_SP, 0x400).unwrap();
    assert_eq!(gate.run_loaded_checked(4).unwrap().steps, 3);

    let wide = encoding::assemble(&[
        encoding::branch_word(0x30, 1),
        vec![encoding::HALT],
        vec![0x05],
        vec![encoding::HALT],
    ]);
    let mut gate = VaxGateLevel::new();
    gate.load_checked(&wide).unwrap();
    gate.write_register_checked(REGISTER_SP, 0x400).unwrap();
    assert_eq!(gate.step_checked().unwrap().mnemonic, "BSBW");
    assert_eq!(gate.step_checked().unwrap().mnemonic, "RSB");
    assert_eq!(
        VaxGateLevel::new()
            .run_checked(
                &encoding::assemble(&[encoding::branch_word(0x31, 0), vec![encoding::HALT],]),
                3,
            )
            .unwrap()
            .traces[0]
            .mnemonic,
        "BRW"
    );
}

#[test]
fn complete_fault_surface_is_typed_and_rolls_back() {
    let mut gate = VaxGateLevel::new();
    gate.load_checked(&[encoding::HALT]).unwrap();
    assert_eq!(
        gate.run_loaded_checked(MAX_RETAINED_TRACES + 1),
        Err(VaxError::TraceLimitTooLarge {
            requested: MAX_RETAINED_TRACES + 1,
            maximum: MAX_RETAINED_TRACES,
        })
    );
    gate.write_register_checked(REGISTER_PC, 2).unwrap();
    assert_eq!(
        gate.step_checked(),
        Err(VaxError::FetchOutsideProgram { pc: 2 })
    );

    let cases = [
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
    ];
    for (program, setup, expected) in cases {
        gate.load_checked(&program).unwrap();
        if let Some((register, value)) = setup {
            gate.write_register_checked(register, value).unwrap();
        }
        let before = gate.get_state();
        assert_eq!(gate.step_checked(), Err(expected));
        assert_eq!(gate.get_state(), before);
    }

    gate.load_checked(&[encoding::NOP, 0x11, 0xfd]).unwrap();
    let before = gate.get_state();
    assert_eq!(
        gate.run_loaded_checked(3),
        Err(VaxError::StepLimitExceeded { max_steps: 3 })
    );
    assert_eq!(gate.get_state(), before);
}
