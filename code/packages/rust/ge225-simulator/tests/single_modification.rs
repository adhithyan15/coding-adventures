use coding_adventures_ge225_simulator::{
    assemble_fixed, assemble_fixed_modified, assemble_select_x_group, assemble_shift,
    assemble_shift_modified, encode_instruction, Simulator,
};

const PROGRAM: i32 = 200;
const DATA: i32 = 300;

fn instruction(opcode: i32, address: i32, modifier: i32) -> i32 {
    encode_instruction(opcode, modifier, address).unwrap()
}

fn simulator_with_program(words: &[i32]) -> Simulator {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.load_words(words, PROGRAM).unwrap();
    simulator.set_program_counter(PROGRAM).unwrap();
    simulator
}

#[test]
fn sxg_uses_the_corrected_five_bit_y_field() {
    let simulator = Simulator::new(4096).unwrap();

    // The corrected 1966 manual writes the pattern as 2506YY3. Group 27
    // decimal is 33 octal, producing the manual's locations 108 through 111.
    assert_eq!(assemble_select_x_group(27).unwrap(), 0o2506333);
    assert_eq!(
        simulator
            .disassemble_word(assemble_select_x_group(27).unwrap())
            .unwrap(),
        "SXG 27"
    );
    assert!(assemble_select_x_group(-1).is_err());
    assert!(assemble_select_x_group(32).is_err());
    assert!(assemble_fixed("SXG").is_err());
}

#[test]
fn sxg_selects_the_encoded_group_instead_of_reading_a() {
    let mut simulator = simulator_with_program(&[
        assemble_select_x_group(27).unwrap(),
        instruction(0o06, DATA, 2),
    ]);
    simulator.write_word(DATA, 0o135746).unwrap();
    simulator.write_word(108, 0o777777).unwrap();

    simulator.run(2).unwrap();

    let state = simulator.get_state();
    assert_eq!(state.a, 0);
    assert_eq!(state.selected_x_group, 27);
    assert_eq!(state.x_words, vec![0o777777, 0, 0o135746, 0]);
    assert_eq!(simulator.read_word(110).unwrap(), 0o135746);
}

#[test]
fn automatic_modification_can_change_a_fixed_instruction() {
    let mut simulator = simulator_with_program(&[
        instruction(0o00, DATA, 0),
        assemble_fixed_modified("LAQ", 1).unwrap(),
    ]);
    simulator.write_word(1, 1).unwrap();
    simulator.write_word(DATA, 0o1234567).unwrap();

    assert_eq!(
        simulator
            .disassemble_word(assemble_fixed_modified("LAQ", 1).unwrap())
            .unwrap(),
        "LAQ,X1"
    );
    simulator.run(2).unwrap();

    // LAQ is 2504001. Adding X1=1 to its operand produces 2504002, LDZ.
    assert_eq!(simulator.get_state().a, 0);
    assert_eq!(simulator.get_state().ir, 0o2524002);
}

#[test]
fn shift_modification_uses_the_selected_group_and_checks_the_limit() {
    let mut simulator = simulator_with_program(&[
        assemble_select_x_group(2).unwrap(),
        instruction(0o00, DATA, 0),
        assemble_shift_modified("SLA", 2, 1).unwrap(),
    ]);
    simulator.write_word(9, 3).unwrap();
    simulator.write_word(DATA, 0o2470).unwrap();

    assert_eq!(
        simulator
            .disassemble_word(assemble_shift_modified("SLA", 2, 1).unwrap())
            .unwrap(),
        "SLA 2,X1"
    );
    simulator.run(3).unwrap();
    assert_eq!(simulator.get_state().a, (0o2470 << 5) & 0o3777777);

    let mut excessive = simulator_with_program(&[
        instruction(0o00, DATA, 0),
        assemble_shift_modified("SLA", 31, 1).unwrap(),
    ]);
    excessive.write_word(1, 1).unwrap();
    excessive.write_word(DATA, 1).unwrap();
    excessive.step().unwrap();
    let before = excessive.get_state();
    let error = excessive.step().unwrap_err();
    assert!(error.contains("shift count exceeds 31"));
    assert_eq!(excessive.get_state(), before);

    assert!(assemble_fixed_modified("LAQ", 4).is_err());
    assert!(assemble_shift_modified("SLA", 1, -1).is_err());
}

#[test]
fn single_length_overflow_stays_latched_until_tested() {
    let mut simulator = simulator_with_program(&[
        instruction(0o00, DATA, 0),
        instruction(0o01, DATA + 1, 0),
        instruction(0o01, DATA + 2, 0),
        instruction(0o02, DATA + 2, 0),
        instruction(0o00, DATA + 1, 0),
        assemble_fixed("NEG").unwrap(),
        instruction(0o00, DATA + 2, 0),
        assemble_fixed("ADO").unwrap(),
        instruction(0o00, DATA + 2, 0),
        assemble_fixed("SBO").unwrap(),
        instruction(0o00, DATA + 1, 0),
        assemble_shift("SLA", 1).unwrap(),
        assemble_fixed("BOV").unwrap(),
    ]);
    simulator.load_words(&[0o1777777, 1, 0], DATA).unwrap();

    simulator.run(2).unwrap();
    assert!(simulator.get_state().overflow);
    for _ in 0..10 {
        simulator.step().unwrap();
        assert!(simulator.get_state().overflow);
    }
    simulator.step().unwrap();
    assert!(!simulator.get_state().overflow);
}

#[test]
fn manual_single_shift_and_compare_vectors_match() {
    let mut shift = simulator_with_program(&[
        instruction(0o00, DATA, 0),
        assemble_shift("SLA", 2).unwrap(),
    ]);
    shift.write_word(DATA, 0o2470).unwrap();
    shift.run(2).unwrap();
    assert_eq!(shift.get_state().a, 0o12340);

    for (a, expected_pc) in [(9, PROGRAM + 2), (10, PROGRAM + 3), (11, PROGRAM + 4)] {
        let mut compare =
            simulator_with_program(&[instruction(0o00, DATA + 1, 0), instruction(0o21, DATA, 0)]);
        compare.write_word(DATA, 10).unwrap();
        compare.write_word(DATA + 1, a).unwrap();
        compare.run(2).unwrap();
        assert_eq!(compare.get_state().pc, expected_pc);
    }
}

#[test]
fn n_input_shifts_fail_closed_when_n_is_not_ready() {
    for mnemonic in ["SNA", "NAQ", "ANQ"] {
        let mut simulator = simulator_with_program(&[
            assemble_fixed("HPT").unwrap(),
            assemble_shift(mnemonic, 1).unwrap(),
        ]);
        simulator.step().unwrap();
        let before = simulator.get_state();
        let error = simulator.step().unwrap_err();
        assert!(error.contains("N register"));
        assert_eq!(simulator.get_state(), before);
    }
}

#[test]
fn register_branch_tests_follow_the_manual_skip_rule() {
    let cases = [
        ("BOD", 1, true),
        ("BOD", 2, false),
        ("BEV", 2, true),
        ("BEV", 1, false),
        ("BMI", 0o2000000, true),
        ("BMI", 1, false),
        ("BPL", 1, true),
        ("BPL", 0o2000000, false),
        ("BZE", 0, true),
        ("BZE", 1, false),
        ("BNZ", 1, true),
        ("BNZ", 0, false),
        ("BNO", 1, true),
    ];

    for (mnemonic, a, condition) in cases {
        let mut simulator = simulator_with_program(&[
            instruction(0o00, DATA, 0),
            assemble_fixed(mnemonic).unwrap(),
        ]);
        simulator.write_word(DATA, a).unwrap();
        simulator.run(2).unwrap();
        assert_eq!(
            simulator.get_state().pc,
            if condition { PROGRAM + 2 } else { PROGRAM + 3 },
            "{mnemonic} with A={a:07o}"
        );
    }

    for (mnemonic, condition) in [("BNR", true), ("BNN", false)] {
        let mut simulator = simulator_with_program(&[assemble_fixed(mnemonic).unwrap()]);
        simulator.step().unwrap();
        assert_eq!(
            simulator.get_state().pc,
            if condition { PROGRAM + 1 } else { PROGRAM + 2 }
        );
    }

    for (mnemonic, condition) in [("BNR", false), ("BNN", true)] {
        let mut simulator = simulator_with_program(&[
            assemble_fixed("HPT").unwrap(),
            assemble_fixed(mnemonic).unwrap(),
        ]);
        simulator.run(2).unwrap();
        assert_eq!(
            simulator.get_state().pc,
            if condition { PROGRAM + 2 } else { PROGRAM + 3 }
        );
    }
}

#[test]
fn circular_single_shift_rotates_only_the_nineteen_data_bits() {
    let value = 0o1234567;
    let data_mask = (1 << 19) - 1;
    let expected = ((value >> 8) | (value << 11)) & data_mask;
    let mut simulator = simulator_with_program(&[
        instruction(0o00, DATA, 0),
        assemble_shift("SCA", 8).unwrap(),
    ]);
    simulator.write_word(DATA, value).unwrap();

    simulator.run(2).unwrap();

    assert_eq!(simulator.get_state().a, expected);
}
