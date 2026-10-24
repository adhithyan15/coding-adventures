use edsac_simulator::*;

fn bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn program(orders: &[(Opcode, usize, Length)]) -> Vec<u32> {
    orders
        .iter()
        .map(|&(opcode, address, length)| encode_order(opcode, address, length))
        .collect()
}

#[test]
fn construction_transport_memory_and_reset_are_strict() {
    let mut machine = EdsacSimulator::new();
    assert_eq!(machine, EdsacSimulator::default());
    assert_eq!(machine.state().memory.as_ref(), &[0; MEMORY_CELLS]);

    let orders = program(&[(Opcode::X, 0, Length::Short), (Opcode::Z, 0, Length::Short)]);
    assert_eq!(machine.load(&bytes(&orders), 32).unwrap(), 2);
    assert_eq!(machine.state().scr, 32);
    assert_eq!(machine.state().loaded_origin, 32);
    assert_eq!(machine.state().loaded_words, 2);

    let before = machine.state();
    assert_eq!(
        machine.load(&[0, 1, 2], 0),
        Err(EdsacError::InvalidTransportLength { bytes: 3 })
    );
    assert_eq!(machine.state(), before);
    assert!(matches!(
        machine.load_words(&[EVEN_CELL_MASK + 1], 0),
        Err(EdsacError::InvalidMemoryWord { .. })
    ));
    assert!(matches!(
        machine.load_words(&[ODD_CELL_MASK + 1], 1),
        Err(EdsacError::InvalidMemoryWord { .. })
    ));
    assert!(matches!(
        machine.load_words(&[0, 0], MEMORY_CELLS - 1),
        Err(EdsacError::ProgramTooLarge { .. })
    ));
    assert_eq!(machine.state(), before);

    machine.reset();
    assert_eq!(machine, EdsacSimulator::new());
}

#[test]
fn short_and_long_add_subtract_store_and_multiplier_round_trip() {
    let mut machine = EdsacSimulator::new();
    let orders = program(&[
        (Opcode::A, 100, Length::Short),
        (Opcode::U, 101, Length::Short),
        (Opcode::S, 100, Length::Short),
        (Opcode::H, 104, Length::Long),
        (Opcode::A, 104, Length::Long),
        (Opcode::U, 106, Length::Long),
        (Opcode::T, 108, Length::Long),
        (Opcode::Z, 0, Length::Short),
    ]);
    machine.load_words(&orders, 0).unwrap();
    machine.write_memory(100, 0x12_345).unwrap();
    machine.write_long(104, 0x1_2345_6789).unwrap();

    machine.step().unwrap();
    machine.step().unwrap();
    assert_eq!(machine.read_memory(101).unwrap(), 0x12_345);
    machine.step().unwrap();
    assert_eq!(machine.state().accumulator, [0; 5]);

    machine.step().unwrap();
    let multiplier = machine.state().multiplier;
    machine.step().unwrap();
    machine.step().unwrap();
    assert_eq!(machine.read_long(106).unwrap(), 0x1_2345_6789);
    assert_eq!(machine.state().multiplier, multiplier);
    machine.step().unwrap();
    assert_eq!(machine.read_long(108).unwrap(), 0x1_2345_6789);
    assert_eq!(machine.state().accumulator, [0; 5]);
}

#[test]
fn collate_multiply_and_negative_multiply_use_serial_registers() {
    let mut machine = EdsacSimulator::new();
    let orders = program(&[
        (Opcode::H, 100, Length::Long),
        (Opcode::C, 102, Length::Long),
        (Opcode::V, 102, Length::Long),
        (Opcode::N, 102, Length::Long),
        (Opcode::Z, 0, Length::Short),
    ]);
    machine.load_words(&orders, 0).unwrap();
    machine.write_long(100, 0x0800_0000).unwrap();
    machine
        .write_long(102, (1_u64 << 35) - 0x0800_0000)
        .unwrap();

    machine.step().unwrap();
    machine.step().unwrap();
    let collated = machine.state().accumulator;
    assert_ne!(collated, [0; 5]);
    machine.step().unwrap();
    let with_product = machine.state().accumulator;
    assert_ne!(with_product, collated);
    machine.step().unwrap();
    assert_eq!(machine.state().accumulator, collated);
}

#[test]
fn every_dual_length_order_accepts_both_documented_forms() {
    let mut machine = EdsacSimulator::new();
    let orders = program(&[
        (Opcode::H, 100, Length::Short),
        (Opcode::V, 100, Length::Short),
        (Opcode::N, 100, Length::Short),
        (Opcode::C, 100, Length::Short),
        (Opcode::S, 102, Length::Long),
        (Opcode::A, 102, Length::Long),
        (Opcode::R, 0, Length::Short),
        (Opcode::L, 0, Length::Short),
        (Opcode::X, 0, Length::Long),
        (Opcode::Z, 0, Length::Short),
    ]);
    machine.load_words(&orders, 0).unwrap();
    machine.write_memory(100, 0x04000).unwrap();
    machine.write_long(102, 0x0400_0000).unwrap();

    let result = machine.run(orders.len()).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, orders.len());
    assert_eq!(result.traces[0].length, Length::Short);
    assert_eq!(result.traces[8].length, Length::Long);
}

#[test]
fn signed_branches_shift_pulses_round_noop_and_halt_execute() {
    let mut machine = EdsacSimulator::new();
    let orders = program(&[
        (Opcode::E, 2, Length::Short),
        (Opcode::Z, 0, Length::Short),
        (Opcode::L, 0, Length::Long),
        (Opcode::R, 0, Length::Long),
        (Opcode::Y, 0, Length::Short),
        (Opcode::X, 0, Length::Short),
        (Opcode::G, 8, Length::Short),
        (Opcode::Z, 0, Length::Short),
        (Opcode::Z, 0, Length::Short),
    ]);
    machine.load_words(&orders, 0).unwrap();
    machine.set_accumulator([1, 0, 0, 0, 0]).unwrap();

    assert_eq!(machine.step().unwrap().pc_after, 2);
    machine.step().unwrap();
    assert_eq!(machine.state().accumulator[0], 2);
    machine.step().unwrap();
    assert_eq!(machine.state().accumulator[0], 1);
    machine.step().unwrap();
    assert_eq!(machine.state().accumulator[2], 0x0200);
    let before_x = machine.state().accumulator;
    machine.step().unwrap();
    assert_eq!(machine.state().accumulator, before_x);

    machine.set_accumulator([0x4000, 0, 0, 0, 0]).unwrap();
    assert_eq!(machine.step().unwrap().pc_after, 8);
    let trace = machine.step().unwrap();
    assert_eq!(trace.mnemonic, "Z");
    assert!(machine.state().halted);
    assert_eq!(machine.step(), Err(EdsacError::Halted));
}

#[test]
fn paper_tape_teleprinter_and_verification_are_complete_state() {
    let mut machine = EdsacSimulator::new();
    let orders = program(&[
        (Opcode::I, 100, Length::Short),
        (Opcode::O, 101, Length::Short),
        (Opcode::F, 102, Length::Short),
        (Opcode::Z, 0, Length::Short),
    ]);
    machine.load_words(&orders, 0).unwrap();
    machine.set_input(&[0x1a]).unwrap();
    machine.write_memory(100, 0x1f000).unwrap();
    machine.write_memory(101, 0x1b000).unwrap();
    machine.write_memory(102, 0x0005).unwrap();

    assert_eq!(machine.step().unwrap().io_code, Some(0x1a));
    assert_eq!(machine.read_memory(100).unwrap(), 0x1f01a);
    assert_eq!(machine.step().unwrap().io_code, Some(0x1b));
    assert_eq!(machine.state().output, vec![0x1b]);
    machine.step().unwrap();
    assert_eq!(machine.read_memory(102).unwrap(), 0x1b005);
    assert_eq!(machine.state().input_cursor, 1);
    assert_eq!(machine.state().last_output, 0x1b);
}

#[test]
fn invalid_orders_input_and_fetch_fail_atomically() {
    for bad in [
        encode_raw_order(0, 0, false),
        encode_order(Opcode::E, 0, Length::Long),
        encode_order(Opcode::Z, 0, Length::Long),
    ] {
        let mut machine = EdsacSimulator::new();
        machine.load_words(&[bad], 0).unwrap();
        let before = machine.state();
        assert!(machine.step().is_err());
        assert_eq!(machine.state(), before);
    }

    let mut machine = EdsacSimulator::new();
    machine
        .load_words(&[encode_order(Opcode::I, 10, Length::Short)], 0)
        .unwrap();
    let before = machine.state();
    assert_eq!(machine.step(), Err(EdsacError::InputExhausted));
    assert_eq!(machine.state(), before);

    machine
        .load_words(&[encode_order(Opcode::X, 0, Length::Short)], 0)
        .unwrap();
    machine.step().unwrap();
    let before = machine.state();
    assert!(matches!(
        machine.step(),
        Err(EdsacError::FetchOutsideProgram { scr: 1, .. })
    ));
    assert_eq!(machine.state(), before);
}

#[test]
fn restore_and_direct_boundaries_are_checked() {
    let mut machine = EdsacSimulator::new();
    assert_eq!(
        machine.read_memory(MEMORY_CELLS),
        Err(EdsacError::InvalidAddress {
            address: MEMORY_CELLS
        })
    );
    assert!(machine.write_memory(1, ODD_CELL_MASK + 1).is_err());
    assert!(machine.write_long(3, 0).is_err());

    let mut state = machine.state();
    state.accumulator = [1, 2, 3, 4, 0x10];
    state.scr = 17;
    state.last_output = 31;
    machine.restore(&state).unwrap();
    assert_eq!(machine.state(), state);

    let mut invalid = state;
    invalid.accumulator[2] = 0x8000;
    let before = machine.state();
    assert!(machine.restore(&invalid).is_err());
    assert_eq!(machine.state(), before);
}

#[test]
fn run_and_execute_are_transactional_and_bounded() {
    let loop_order = encode_order(Opcode::E, 0, Length::Short);
    let mut machine = EdsacSimulator::new();
    machine.load_words(&[loop_order], 0).unwrap();
    let before = machine.state();
    assert_eq!(
        machine.run(3),
        Err(EdsacError::MaxStepsExceeded { max_steps: 3 })
    );
    assert_eq!(machine.state(), before);

    let orders = program(&[(Opcode::X, 0, Length::Short), (Opcode::Z, 0, Length::Short)]);
    let result = machine.execute_words(&orders, 20, 4).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(machine.state().scr, 22);

    let result = machine.execute(&bytes(&orders), 40, 4).unwrap();
    assert!(result.halted);
    assert_eq!(machine.state().scr, 42);

    let already_halted = machine.run(0).unwrap();
    assert!(already_halted.halted);
    assert_eq!(already_halted.steps, 0);
}

#[test]
fn all_state_validation_failures_and_error_messages_are_typed() {
    let mut machine = EdsacSimulator::new();
    assert!(matches!(
        machine.load_words(&[], MEMORY_CELLS),
        Err(EdsacError::InvalidOrigin { .. })
    ));
    assert!(matches!(
        machine.set_input(&[32]),
        Err(EdsacError::InvalidInputCode { .. })
    ));
    assert!(matches!(
        machine.write_long(0, LONG_WORD_MASK + 1),
        Err(EdsacError::InvalidLongWord { .. })
    ));
    assert!(machine.read_long(MEMORY_CELLS - 1).is_err());
    assert!(matches!(
        machine.set_accumulator([0, 0, 0, 0, 1]),
        Err(EdsacError::InvalidRegisterPadding { .. })
    ));

    let base = machine.state();
    let mut invalid_states = Vec::new();
    let mut state = base.clone();
    state.scr = MEMORY_CELLS;
    invalid_states.push(state);
    let mut state = base.clone();
    state.order_tank = ODD_CELL_MASK + 1;
    invalid_states.push(state);
    let mut state = base.clone();
    state.multiplier[0] = 0x8000;
    invalid_states.push(state);
    let mut state = base.clone();
    state.multiplier[2] = 1;
    invalid_states.push(state);
    let mut state = base.clone();
    state.input = vec![32];
    invalid_states.push(state);
    let mut state = base.clone();
    state.input_cursor = 1;
    invalid_states.push(state);
    let mut state = base.clone();
    state.output = vec![32];
    invalid_states.push(state);
    let mut state = base.clone();
    state.last_output = 32;
    invalid_states.push(state);
    let mut state = base;
    state.loaded_origin = MEMORY_CELLS;
    invalid_states.push(state);
    for state in invalid_states {
        assert!(machine.restore(&state).is_err());
    }

    let errors = [
        EdsacError::InvalidTransportLength { bytes: 1 },
        EdsacError::InvalidOrigin { origin: 1024 },
        EdsacError::ProgramTooLarge {
            words: 2,
            capacity: 1,
        },
        EdsacError::InvalidMemoryWord {
            address: 1,
            word: 0x20000,
            maximum: ODD_CELL_MASK,
        },
        EdsacError::InvalidAddress { address: 1024 },
        EdsacError::MisalignedLongAddress { address: 3 },
        EdsacError::InvalidLongWord {
            word: LONG_WORD_MASK + 1,
        },
        EdsacError::InvalidRegisterSlice {
            register: "accumulator",
            index: 0,
            value: 0x8000,
        },
        EdsacError::InvalidRegisterPadding {
            register: "accumulator",
            value: 1,
            required_zero_mask: 0xf,
        },
        EdsacError::InvalidSequenceControl { scr: 1024 },
        EdsacError::InvalidOrderTank { order: 0x20000 },
        EdsacError::InvalidInputCode { index: 0, code: 32 },
        EdsacError::InvalidInputCursor {
            cursor: 1,
            length: 0,
        },
        EdsacError::InvalidOutputCode { index: 0, code: 32 },
        EdsacError::InvalidLastOutput { code: 32 },
        EdsacError::InvalidLoadedRange {
            origin: 1024,
            words: 1,
        },
        EdsacError::Halted,
        EdsacError::FetchOutsideProgram {
            scr: 1,
            origin: 0,
            words: 1,
        },
        EdsacError::UnknownOpcode {
            opcode: 0,
            instruction: 0,
        },
        EdsacError::InvalidLength {
            opcode: Opcode::Z,
            instruction: encode_order(Opcode::Z, 0, Length::Long),
        },
        EdsacError::InputExhausted,
        EdsacError::SequenceControlOverflow,
        EdsacError::MaxStepsExceeded { max_steps: 1 },
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn terminal_conditional_fallthrough_never_wraps_sequence_control() {
    for (opcode, accumulator) in [
        (Opcode::E, [0x4000, 0, 0, 0, 0]),
        (Opcode::G, [0, 0, 0, 0, 0]),
    ] {
        let mut machine = EdsacSimulator::new();
        machine
            .load_words(&[encode_order(opcode, 0, Length::Short)], MEMORY_CELLS - 1)
            .unwrap();
        machine.set_accumulator(accumulator).unwrap();
        let before = machine.state();
        assert_eq!(machine.step(), Err(EdsacError::SequenceControlOverflow));
        assert_eq!(machine.state(), before);
    }

    for (opcode, accumulator) in [
        (Opcode::E, [0, 0, 0, 0, 0]),
        (Opcode::G, [0x4000, 0, 0, 0, 0]),
    ] {
        let mut machine = EdsacSimulator::new();
        machine
            .load_words(&[encode_order(opcode, 0, Length::Short)], MEMORY_CELLS - 1)
            .unwrap();
        machine.set_accumulator(accumulator).unwrap();
        assert_eq!(machine.step().unwrap().pc_after, 0);
    }
}

#[test]
fn short_operands_ignore_the_even_cell_sandwich_digit() {
    let mut machine = EdsacSimulator::new();
    machine
        .load_words(
            &[
                encode_order(Opcode::H, 100, Length::Short),
                encode_order(Opcode::Z, 0, Length::Short),
            ],
            0,
        )
        .unwrap();
    machine.write_memory(100, 0x2_0000).unwrap();

    machine.step().unwrap();
    assert_eq!(machine.state().multiplier, [0; 3]);
    let state = machine.state();
    machine.restore(&state).unwrap();
    assert_eq!(machine.state(), state);
}
