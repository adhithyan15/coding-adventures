use manchester_baby_gatelevel::{
    BabyError, BabyState, ManchesterBabyGateLevel, ESTIMATED_GATE_COUNT, FLIP_FLOP_COUNT,
    STORE_WORDS,
};
use manchester_baby_simulator::{encode_instruction, BabySimulator as FunctionalBaby, Function};

const STP: u32 = encode_instruction(Function::Stop, 0);

fn word(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

fn image(store: [u32; STORE_WORDS]) -> Vec<u8> {
    store.iter().flat_map(|value| value.to_le_bytes()).collect()
}

fn instruction(function: Function, operand: u8) -> u32 {
    encode_instruction(function, operand)
}

fn assert_matches_functional(program: &[u8], max_steps: usize) {
    let gate = ManchesterBabyGateLevel::new()
        .execute(program, max_steps)
        .unwrap();
    let functional = FunctionalBaby::new().execute(program, max_steps).unwrap();

    assert_eq!(gate.final_state.store, functional.final_state.store);
    assert_eq!(
        gate.final_state.accumulator,
        functional.final_state.accumulator
    );
    assert_eq!(gate.final_state.ci, functional.final_state.ci);
    assert_eq!(gate.final_state.halted, functional.final_state.halted);
    assert_eq!(gate.steps, functional.steps);
    for (gate_trace, functional_trace) in gate.traces.iter().zip(&functional.traces) {
        assert_eq!(gate_trace.pc_before, functional_trace.pc_before);
        assert_eq!(gate_trace.pc_after, functional_trace.pc_after);
        assert_eq!(gate_trace.instruction, functional_trace.instruction);
        assert_eq!(gate_trace.mnemonic, functional_trace.mnemonic);
        assert_eq!(gate_trace.description, functional_trace.description);
    }
}

fn assert_lockstep_matches_functional(program: &[u8], max_steps: usize) {
    let mut gate = ManchesterBabyGateLevel::new();
    let mut functional = FunctionalBaby::new();
    gate.load(program, 0).unwrap();
    functional.load(program, 0).unwrap();

    for _ in 0..max_steps {
        let gate_before = gate.get_state();
        let functional_before = functional.get_state();
        assert_eq!(gate_before.store, functional_before.store);
        assert_eq!(gate_before.accumulator, functional_before.accumulator);
        assert_eq!(gate_before.ci, functional_before.ci);
        assert_eq!(gate_before.halted, functional_before.halted);
        if gate_before.halted {
            return;
        }

        let gate_trace = gate.step().unwrap();
        let functional_trace = functional.step().unwrap();
        assert_eq!(gate_trace.pc_before, functional_trace.pc_before);
        assert_eq!(gate_trace.pc_after, functional_trace.pc_after);
        assert_eq!(gate_trace.instruction, functional_trace.instruction);
        assert_eq!(gate_trace.mnemonic, functional_trace.mnemonic);
        assert_eq!(gate_trace.description, functional_trace.description);

        let gate_after = gate.get_state();
        let functional_after = functional.get_state();
        assert_eq!(gate_after.store, functional_after.store);
        assert_eq!(gate_after.accumulator, functional_after.accumulator);
        assert_eq!(gate_after.ci, functional_after.ci);
        assert_eq!(gate_after.halted, functional_after.halted);
        if gate_after.halted {
            return;
        }
    }

    panic!("lockstep program did not halt within {max_steps} steps");
}

#[test]
fn construction_uses_flip_flops_in_the_power_on_state() {
    let simulator = ManchesterBabyGateLevel::new();
    let state = simulator.get_state();

    assert_eq!(ManchesterBabyGateLevel::default(), simulator);
    assert_eq!(state.store, [0; STORE_WORDS]);
    assert_eq!(state.accumulator, 0);
    assert_eq!(state.ci, 31);
    assert!(!state.halted);
    assert_eq!(simulator.flip_flop_count(), FLIP_FLOP_COUNT);
    assert_eq!(FLIP_FLOP_COUNT, 1_062);
    assert_eq!(simulator.gate_count(), ESTIMATED_GATE_COUNT);
}

#[test]
fn load_clocks_complete_little_endian_words_only() {
    let mut simulator = ManchesterBabyGateLevel::new();
    let loaded = simulator.load(&[0x78, 0x56, 0x34, 0x12, 0xaa], 4).unwrap();

    assert_eq!(loaded, 1);
    assert_eq!(simulator.get_state().store[4], 0x1234_5678);
    assert_eq!(simulator.get_state().store[5], 0);
}

#[test]
fn load_stops_at_the_store_boundary() {
    let mut simulator = ManchesterBabyGateLevel::new();
    let program = [word(11), word(12), word(13)].concat();

    assert_eq!(simulator.load(&program, 31).unwrap(), 1);
    assert_eq!(simulator.get_state().store[31], 11);
}

#[test]
fn load_rejects_an_origin_outside_the_store() {
    let mut simulator = ManchesterBabyGateLevel::new();

    assert_eq!(
        simulator.load(&word(1), STORE_WORDS),
        Err(BabyError::InvalidOrigin {
            origin: STORE_WORDS
        })
    );
}

#[test]
fn stop_sets_the_halt_flip_flop_and_returns_a_trace() {
    let mut simulator = ManchesterBabyGateLevel::new();
    simulator.load(&word(STP), 0).unwrap();

    let trace = simulator.step().unwrap();
    assert_eq!(trace.pc_before, 31);
    assert_eq!(trace.pc_after, 0);
    assert_eq!(trace.instruction, STP);
    assert_eq!(trace.mnemonic, "STP");
    assert!(trace.description.contains("line 0"));
    assert!(simulator.get_state().halted);
    assert_eq!(simulator.step(), Err(BabyError::Halted));
    assert_lockstep_matches_functional(&word(STP), 1);
}

#[test]
fn ldn_negates_through_gates_with_32_bit_wrapping() {
    for (data, expected) in [(42, 0xffff_ffd6), (0x8000_0000, 0x8000_0000)] {
        let mut store = [0; STORE_WORDS];
        store[0] = instruction(Function::LoadNegative, 28);
        store[1] = STP;
        store[28] = data;

        let result = ManchesterBabyGateLevel::new()
            .execute(&image(store), 10)
            .unwrap();
        assert_eq!(result.final_state.accumulator, expected);
    }
}

#[test]
fn sto_clocks_the_accumulator_into_store_flip_flops() {
    let mut store = [0; STORE_WORDS];
    store[0] = instruction(Function::LoadNegative, 28);
    store[1] = instruction(Function::Store, 29);
    store[2] = STP;
    store[28] = 42;
    store[29] = 99;

    let result = ManchesterBabyGateLevel::new()
        .execute(&image(store), 10)
        .unwrap();
    assert_eq!(result.final_state.store[29], 0xffff_ffd6);
    assert_eq!(result.final_state.accumulator, 0xffff_ffd6);
}

#[test]
fn both_one_hot_subtract_selects_share_the_gate_path() {
    let run = |function| {
        let mut store = [0; STORE_WORDS];
        store[0] = instruction(function, 28);
        store[1] = STP;
        store[28] = 7;
        ManchesterBabyGateLevel::new()
            .execute(&image(store), 10)
            .unwrap()
            .final_state
            .accumulator
    };

    assert_eq!(run(Function::Subtract), 0xffff_fff9);
    assert_eq!(run(Function::AlternateSubtract), run(Function::Subtract));
}

#[test]
fn cmp_uses_the_accumulator_sign_bit_to_skip() {
    let mut negative = [0; STORE_WORDS];
    negative[0] = instruction(Function::LoadNegative, 28);
    negative[1] = instruction(Function::Compare, 0);
    negative[2] = instruction(Function::Store, 31);
    negative[3] = STP;
    negative[28] = 1;
    let result = ManchesterBabyGateLevel::new()
        .execute(&image(negative), 10)
        .unwrap();
    assert_eq!(result.final_state.ci, 3);
    assert_eq!(result.final_state.store[31], 0);

    for data in [0, u32::MAX] {
        let mut non_negative = [0; STORE_WORDS];
        non_negative[0] = instruction(Function::LoadNegative, 28);
        non_negative[1] = instruction(Function::Compare, 0);
        non_negative[2] = STP;
        non_negative[3] = instruction(Function::Store, 31);
        non_negative[28] = data;
        let result = ManchesterBabyGateLevel::new()
            .execute(&image(non_negative), 10)
            .unwrap();
        assert_eq!(result.final_state.ci, 2);
    }
}

#[test]
fn jmp_clocks_five_store_bits_into_ci() {
    let mut store = [0; STORE_WORDS];
    store[0] = instruction(Function::Jump, 28);
    store[1] = instruction(Function::Store, 31);
    store[2] = STP;
    store[28] = 0x22;
    store[3] = STP;

    let result = ManchesterBabyGateLevel::new()
        .execute(&image(store), 10)
        .unwrap();
    assert_eq!(result.final_state.ci, 3);
    assert_eq!(result.steps, 2);
}

#[test]
fn jrp_adds_a_twos_complement_displacement_through_gates() {
    let mut store = [0; STORE_WORDS];
    store[0] = instruction(Function::JumpRelative, 28);
    store[28] = u32::MAX;

    assert_eq!(
        ManchesterBabyGateLevel::new().execute(&image(store), 7),
        Err(BabyError::MaxStepsExceeded { max_steps: 7 })
    );
}

#[test]
fn execute_resets_previous_flip_flop_state() {
    let mut simulator = ManchesterBabyGateLevel::new();
    let mut first = [0; STORE_WORDS];
    first[0] = instruction(Function::LoadNegative, 28);
    first[1] = STP;
    first[28] = 5;
    simulator.execute(&image(first), 10).unwrap();

    let result = simulator.execute(&word(STP), 10).unwrap();
    assert_eq!(result.final_state.accumulator, 0);
    assert_eq!(result.final_state.store[28], 0);
}

#[test]
fn reset_and_owned_snapshots_are_deterministic() {
    let mut simulator = ManchesterBabyGateLevel::new();
    let before = simulator.get_state();
    simulator.load(&word(STP), 0).unwrap();
    simulator.step().unwrap();
    simulator.reset();

    assert_eq!(simulator.get_state(), before);
}

#[test]
fn state_helpers_are_signed_and_hardened_to_five_bit_ci() {
    let mut store = [0; STORE_WORDS];
    store[31] = 0x1234_5678;
    let state = BabyState {
        store,
        accumulator: u32::MAX,
        ci: u8::MAX,
        halted: false,
    };

    assert_eq!(state.accumulator_signed(), -1);
    assert_eq!(state.present_instruction(), 0x1234_5678);
}

#[test]
fn countdown_program_matches_the_functional_oracle() {
    let mut store = [0; STORE_WORDS];
    store[0] = instruction(Function::LoadNegative, 28);
    store[1] = instruction(Function::Subtract, 29);
    store[2] = instruction(Function::Compare, 0);
    store[3] = STP;
    store[4] = instruction(Function::JumpRelative, 30);
    store[28] = 3;
    store[29] = u32::MAX;
    store[30] = 0xffff_fffc;

    let program = image(store);
    assert_lockstep_matches_functional(&program, 100);
    assert_matches_functional(&program, 100);
}

#[test]
fn seeded_data_path_programs_match_the_functional_oracle() {
    let mut seed = 0x1948_0621_u32;
    for _ in 0..64 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let left = seed;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let right = seed;

        let mut store = [0; STORE_WORDS];
        store[0] = instruction(Function::LoadNegative, 24);
        store[1] = instruction(Function::Subtract, 25);
        store[2] = instruction(Function::Store, 26);
        store[3] = instruction(Function::Compare, 0);
        store[4] = STP;
        store[5] = STP;
        store[24] = left;
        store[25] = right;
        assert_matches_functional(&image(store), 10);
    }
}

#[test]
fn errors_have_actionable_messages() {
    assert_eq!(
        BabyError::Halted.to_string(),
        "the Manchester Baby is halted"
    );
    assert!(BabyError::InvalidOrigin { origin: 32 }
        .to_string()
        .contains("32"));
    assert!(BabyError::MaxStepsExceeded { max_steps: 9 }
        .to_string()
        .contains("9"));
}
