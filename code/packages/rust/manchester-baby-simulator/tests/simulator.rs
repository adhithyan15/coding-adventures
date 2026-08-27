use manchester_baby_simulator::{
    encode_instruction, BabyError, BabySimulator, Function, STORE_WORDS,
};

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

#[test]
fn construction_matches_the_power_on_state() {
    let simulator = BabySimulator::new();
    let state = simulator.get_state();

    assert_eq!(BabySimulator::default(), simulator);
    assert_eq!(state.store, [0; STORE_WORDS]);
    assert_eq!(state.accumulator, 0);
    assert_eq!(state.ci, 31);
    assert!(!state.halted);
}

#[test]
fn load_decodes_complete_little_endian_words() {
    let mut simulator = BabySimulator::new();
    let loaded = simulator.load(&[0x78, 0x56, 0x34, 0x12, 0xaa], 4).unwrap();

    assert_eq!(loaded, 1);
    assert_eq!(simulator.get_state().store[4], 0x1234_5678);
    assert_eq!(simulator.get_state().store[5], 0);
}

#[test]
fn load_stops_at_the_store_boundary() {
    let mut simulator = BabySimulator::new();
    let program = [word(11), word(12), word(13)].concat();

    assert_eq!(simulator.load(&program, 31).unwrap(), 1);
    assert_eq!(simulator.get_state().store[31], 11);
}

#[test]
fn load_rejects_an_origin_outside_the_store() {
    let mut simulator = BabySimulator::new();

    assert_eq!(
        simulator.load(&word(1), STORE_WORDS),
        Err(BabyError::InvalidOrigin {
            origin: STORE_WORDS
        })
    );
}

#[test]
fn stop_halts_in_one_step_and_returns_a_trace() {
    let mut simulator = BabySimulator::new();
    simulator.load(&word(STP), 0).unwrap();

    let trace = simulator.step().unwrap();
    assert_eq!(trace.pc_before, 31);
    assert_eq!(trace.pc_after, 0);
    assert_eq!(trace.instruction, STP);
    assert_eq!(trace.mnemonic, "STP");
    assert!(trace.description.contains("line 0"));
    assert!(simulator.get_state().halted);
}

#[test]
fn stepping_a_halted_machine_is_an_error() {
    let mut simulator = BabySimulator::new();
    simulator.load(&word(STP), 0).unwrap();
    simulator.step().unwrap();

    assert_eq!(simulator.step(), Err(BabyError::Halted));
}

#[test]
fn ldn_negates_with_32_bit_wrapping() {
    let mut store = [0; STORE_WORDS];
    store[0] = instruction(Function::LoadNegative, 28);
    store[1] = STP;
    store[28] = 42;

    let result = BabySimulator::new().execute(&image(store), 10).unwrap();
    assert_eq!(result.final_state.accumulator, 0xffff_ffd6);
    assert_eq!(result.final_state.accumulator_signed(), -42);
}

#[test]
fn ldn_of_minimum_integer_wraps_to_itself() {
    let mut store = [0; STORE_WORDS];
    store[0] = instruction(Function::LoadNegative, 28);
    store[1] = STP;
    store[28] = 0x8000_0000;

    let result = BabySimulator::new().execute(&image(store), 10).unwrap();
    assert_eq!(result.final_state.accumulator, 0x8000_0000);
    assert_eq!(result.final_state.accumulator_signed(), i32::MIN);
}

#[test]
fn sto_can_modify_code_or_data() {
    let mut store = [0; STORE_WORDS];
    store[0] = instruction(Function::LoadNegative, 28);
    store[1] = instruction(Function::Store, 29);
    store[2] = STP;
    store[28] = 42;
    store[29] = 99;

    let result = BabySimulator::new().execute(&image(store), 10).unwrap();
    assert_eq!(result.final_state.store[29], 0xffff_ffd6);
    assert_eq!(result.final_state.accumulator, 0xffff_ffd6);
}

#[test]
fn both_subtract_encodings_are_equivalent() {
    let run = |function| {
        let mut store = [0; STORE_WORDS];
        store[0] = instruction(function, 28);
        store[1] = STP;
        store[28] = 7;
        BabySimulator::new()
            .execute(&image(store), 10)
            .unwrap()
            .final_state
            .accumulator
    };

    assert_eq!(run(Function::Subtract), 0xffff_fff9);
    assert_eq!(run(Function::AlternateSubtract), run(Function::Subtract));
}

#[test]
fn cmp_skips_the_next_instruction_for_a_negative_accumulator() {
    let mut store = [0; STORE_WORDS];
    store[0] = instruction(Function::LoadNegative, 28);
    store[1] = instruction(Function::Compare, 0);
    store[2] = instruction(Function::Store, 31);
    store[3] = STP;
    store[28] = 1;

    let result = BabySimulator::new().execute(&image(store), 10).unwrap();
    assert_eq!(result.final_state.ci, 3);
    assert_eq!(result.final_state.store[31], 0);
}

#[test]
fn cmp_does_not_skip_for_zero_or_positive_values() {
    for data in [0, 0xffff_ffff] {
        let mut store = [0; STORE_WORDS];
        store[0] = instruction(Function::LoadNegative, 28);
        store[1] = instruction(Function::Compare, 0);
        store[2] = STP;
        store[3] = instruction(Function::Store, 31);
        store[28] = data;

        let result = BabySimulator::new().execute(&image(store), 10).unwrap();
        assert_eq!(result.final_state.ci, 2);
    }
}

#[test]
fn jmp_uses_the_low_five_bits_and_obeys_preincrement_fetch() {
    let mut store = [0; STORE_WORDS];
    store[0] = instruction(Function::Jump, 28);
    store[1] = instruction(Function::Store, 31);
    store[2] = STP;
    store[28] = 0x22; // low five bits are 2; next fetch is therefore line 3
    store[3] = STP;

    let result = BabySimulator::new().execute(&image(store), 10).unwrap();
    assert_eq!(result.final_state.ci, 3);
    assert_eq!(result.steps, 2);
}

#[test]
fn jrp_accepts_a_twos_complement_backward_displacement() {
    let mut store = [0; STORE_WORDS];
    store[0] = instruction(Function::JumpRelative, 28);
    store[28] = 0xffff_ffff; // -1: return CI to 31, then preincrement to 0

    let error = BabySimulator::new().execute(&image(store), 7).unwrap_err();
    assert_eq!(error, BabyError::MaxStepsExceeded { max_steps: 7 });
}

#[test]
fn control_instruction_wraps_from_line_31_to_line_0() {
    let mut simulator = BabySimulator::new();
    simulator.load(&word(STP), 0).unwrap();

    assert_eq!(simulator.step().unwrap().pc_after, 0);
}

#[test]
fn execute_resets_previous_machine_state() {
    let mut simulator = BabySimulator::new();
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
fn reset_clears_store_registers_and_halt_state() {
    let mut simulator = BabySimulator::new();
    simulator.load(&word(STP), 0).unwrap();
    simulator.step().unwrap();
    simulator.reset();

    assert_eq!(simulator.get_state(), BabySimulator::new().get_state());
}

#[test]
fn state_snapshots_are_owned_and_stable() {
    let mut simulator = BabySimulator::new();
    let before = simulator.get_state();
    simulator.load(&word(0xdead_beef), 0).unwrap();

    assert_eq!(before.store[0], 0);
    assert_eq!(simulator.get_state().store[0], 0xdead_beef);
}

#[test]
fn present_instruction_reads_the_word_at_ci() {
    let mut simulator = BabySimulator::new();
    simulator.load(&word(STP), 0).unwrap();
    simulator.step().unwrap();

    assert_eq!(simulator.get_state().present_instruction(), STP);
}

#[test]
fn countdown_program_exercises_a_backward_loop() {
    let mut store = [0; STORE_WORDS];
    store[0] = instruction(Function::LoadNegative, 28);
    store[1] = instruction(Function::Subtract, 29);
    store[2] = instruction(Function::Compare, 0);
    store[3] = STP;
    store[4] = instruction(Function::JumpRelative, 30);
    store[28] = 3;
    store[29] = 0xffff_ffff;
    store[30] = 0xffff_fffc;

    let result = BabySimulator::new().execute(&image(store), 100).unwrap();
    assert_eq!(result.final_state.accumulator, 0);
    assert_eq!(result.final_state.ci, 3);
    assert_eq!(result.traces.len(), result.steps);
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
