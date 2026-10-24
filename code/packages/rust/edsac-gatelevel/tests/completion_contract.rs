use edsac_gatelevel::{EdsacGateLevel, ESTIMATED_GATE_COUNT, FLIP_FLOP_COUNT, MAX_RETAINED_STEPS};
use edsac_simulator::{
    encode_order, encode_raw_order, EdsacError, EdsacSimulator, Length, Opcode, EVEN_CELL_MASK,
    MEMORY_CELLS,
};

fn orders(values: &[(Opcode, usize, Length)]) -> Vec<u32> {
    values
        .iter()
        .map(|&(opcode, address, length)| encode_order(opcode, address, length))
        .collect()
}

fn assert_lockstep(functional: &mut EdsacSimulator, gate: &mut EdsacGateLevel, max_steps: usize) {
    for _ in 0..max_steps {
        assert_eq!(gate.state(), functional.state());
        if gate.state().halted {
            return;
        }
        let expected = functional.step();
        let actual = gate.step();
        assert_eq!(actual, expected);
        assert_eq!(gate.state(), functional.state());
        if actual.is_err() || gate.state().halted {
            return;
        }
    }
    panic!("lockstep machine did not halt or fault within {max_steps} steps");
}

#[test]
fn topology_and_power_on_state_are_exact() {
    let gate = EdsacGateLevel::new();
    assert_eq!(gate, EdsacGateLevel::default());
    assert_eq!(gate.state(), EdsacSimulator::new().state());
    assert_eq!(gate.flip_flop_count(), FLIP_FLOP_COUNT);
    assert_eq!(FLIP_FLOP_COUNT, 18_095);
    assert_eq!(gate.gate_count(), ESTIMATED_GATE_COUNT);
    assert_eq!(ESTIMATED_GATE_COUNT, FLIP_FLOP_COUNT * 6 + 36_000);
}

#[test]
fn checked_transport_memory_restore_and_reset_match_the_oracle() {
    let image = orders(&[(Opcode::X, 0, Length::Short), (Opcode::Z, 0, Length::Short)]);
    let bytes = image
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect::<Vec<_>>();
    let mut functional = EdsacSimulator::new();
    let mut gate = EdsacGateLevel::new();
    assert_eq!(gate.load(&bytes, 32), functional.load(&bytes, 32));
    assert_eq!(gate.state(), functional.state());
    assert_eq!(gate.write_memory(100, EVEN_CELL_MASK), Ok(()));
    assert_eq!(gate.read_memory(100), Ok(EVEN_CELL_MASK));
    assert_eq!(gate.write_long(102, 0x1_2345_6789), Ok(()));
    assert_eq!(gate.read_long(102), Ok(0x1_2345_6789));

    let snapshot = gate.state();
    gate.reset();
    assert_ne!(gate.state(), snapshot);
    gate.restore(&snapshot).unwrap();
    assert_eq!(gate.state(), snapshot);

    let mut invalid = snapshot;
    invalid.scr = MEMORY_CELLS;
    let before = gate.state();
    assert!(gate.restore(&invalid).is_err());
    assert_eq!(gate.state(), before);
}

#[test]
fn arithmetic_store_multiplier_collate_round_and_shift_paths_lockstep() {
    let image = orders(&[
        (Opcode::A, 100, Length::Short),
        (Opcode::U, 101, Length::Short),
        (Opcode::S, 100, Length::Short),
        (Opcode::H, 104, Length::Long),
        (Opcode::C, 106, Length::Long),
        (Opcode::V, 106, Length::Long),
        (Opcode::N, 106, Length::Long),
        (Opcode::Y, 0, Length::Short),
        (Opcode::L, 0, Length::Long),
        (Opcode::R, 0, Length::Long),
        (Opcode::T, 108, Length::Long),
        (Opcode::Z, 0, Length::Short),
    ]);
    let mut functional = EdsacSimulator::new();
    let mut gate = EdsacGateLevel::new();
    functional.load_words(&image, 0).unwrap();
    gate.load_words(&image, 0).unwrap();
    functional.write_memory(100, 0x12_345).unwrap();
    functional.write_long(104, 0x0800_0000).unwrap();
    functional.write_long(106, 0x0780_0000).unwrap();
    gate.write_memory(100, 0x12_345).unwrap();
    gate.write_long(104, 0x0800_0000).unwrap();
    gate.write_long(106, 0x0780_0000).unwrap();
    assert_lockstep(&mut functional, &mut gate, image.len());
}

#[test]
fn branch_and_io_paths_lockstep_with_complete_peripheral_state() {
    let image = orders(&[
        (Opcode::E, 2, Length::Short),
        (Opcode::Z, 0, Length::Short),
        (Opcode::I, 100, Length::Short),
        (Opcode::O, 101, Length::Short),
        (Opcode::F, 102, Length::Short),
        (Opcode::G, 7, Length::Short),
        (Opcode::Z, 0, Length::Short),
        (Opcode::Z, 0, Length::Short),
    ]);
    let mut functional = EdsacSimulator::new();
    let mut gate = EdsacGateLevel::new();
    functional.load_words(&image, 0).unwrap();
    gate.load_words(&image, 0).unwrap();
    functional.set_input(&[0x1a]).unwrap();
    gate.set_input(&[0x1a]).unwrap();
    functional.write_memory(101, 0x1b000).unwrap();
    gate.write_memory(101, 0x1b000).unwrap();
    functional.write_memory(102, 5).unwrap();
    gate.write_memory(102, 5).unwrap();
    functional.set_accumulator([0x4000, 0, 0, 0, 0]).unwrap();
    gate.set_accumulator([0x4000, 0, 0, 0, 0]).unwrap();
    assert_lockstep(&mut functional, &mut gate, image.len());
}

#[test]
fn decode_fetch_input_halt_and_run_faults_are_atomic() {
    for bad in [
        encode_raw_order(0, 0, false),
        encode_order(Opcode::E, 0, Length::Long),
        encode_order(Opcode::Z, 0, Length::Long),
    ] {
        let mut gate = EdsacGateLevel::new();
        gate.load_words(&[bad], 0).unwrap();
        let before = gate.state();
        assert!(gate.step().is_err());
        assert_eq!(gate.state(), before);
    }

    let mut gate = EdsacGateLevel::new();
    gate.load_words(&[encode_order(Opcode::I, 10, Length::Short)], 0)
        .unwrap();
    let before = gate.state();
    assert_eq!(gate.step(), Err(EdsacError::InputExhausted));
    assert_eq!(gate.state(), before);

    gate.load_words(&[encode_order(Opcode::E, 0, Length::Short)], 0)
        .unwrap();
    let before = gate.state();
    assert_eq!(
        gate.run(3),
        Err(EdsacError::MaxStepsExceeded { max_steps: 3 })
    );
    assert_eq!(gate.state(), before);
}

#[test]
fn execute_variants_are_transactional_and_return_oracle_results() {
    let image = orders(&[(Opcode::X, 0, Length::Short), (Opcode::Z, 0, Length::Short)]);
    let bytes = image
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect::<Vec<_>>();
    let mut functional = EdsacSimulator::new();
    let mut gate = EdsacGateLevel::new();
    assert_eq!(
        gate.execute_words(&image, 20, 4),
        functional.execute_words(&image, 20, 4)
    );
    assert_eq!(gate.state(), functional.state());
    assert_eq!(
        gate.execute(&bytes, 40, 4),
        functional.execute(&bytes, 40, 4)
    );
    assert_eq!(gate.state(), functional.state());
}

#[test]
fn retained_execution_budget_is_bounded_and_preflighted() {
    let mut gate = EdsacGateLevel::new();
    let halt = [encode_order(Opcode::Z, 0, Length::Short)];
    gate.load_words(&halt, 0).unwrap();
    let before = gate.state();
    let excessive = MAX_RETAINED_STEPS + 1;
    assert_eq!(
        gate.run(excessive),
        Err(EdsacError::MaxStepsExceeded {
            max_steps: excessive
        })
    );
    assert_eq!(gate.state(), before);
    assert_eq!(
        gate.execute_words(&halt, 12, excessive),
        Err(EdsacError::MaxStepsExceeded {
            max_steps: excessive
        })
    );
    assert_eq!(gate.state(), before);
    assert_eq!(
        gate.run(0),
        Err(EdsacError::MaxStepsExceeded { max_steps: 0 })
    );
    assert_eq!(gate.state(), before);

    gate.step().unwrap();
    assert!(gate.run(0).unwrap().halted);
}
