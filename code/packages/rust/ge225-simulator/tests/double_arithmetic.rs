use coding_adventures_ge225_simulator::{
    assemble_fixed, assemble_shift, encode_instruction, Simulator,
};

fn instruction(opcode: i32, address: i32, modifier: i32) -> i32 {
    encode_instruction(opcode, modifier, address).unwrap()
}

fn load_double(simulator: &mut Simulator, high: i32, low: i32, address: i32) {
    simulator.load_words(&[high, low], address).unwrap();
}

#[test]
fn manual_double_add_and_subtract_vectors_use_sign_plus_thirty_eight_data_bits() {
    let mut add = Simulator::new(4096).unwrap();
    load_double(&mut add, 0o0000001, 0o0003734, 300);
    load_double(&mut add, 0o0000001, 0o1104677, 302);
    add.load_words(&[instruction(0o10, 300, 0), instruction(0o11, 302, 0)], 200)
        .unwrap();
    add.set_program_counter(200).unwrap();
    add.run(2).unwrap();
    assert_eq!(add.get_state().a, 0o0000002);
    assert_eq!(add.get_state().q, 0o1110633);
    assert!(!add.get_state().overflow);

    let mut subtract = Simulator::new(4096).unwrap();
    load_double(&mut subtract, 0o0000001, 0o1104677, 300);
    load_double(&mut subtract, 0o0000001, 0o0003734, 302);
    subtract
        .load_words(&[instruction(0o10, 300, 0), instruction(0o12, 302, 0)], 200)
        .unwrap();
    subtract.set_program_counter(200).unwrap();
    subtract.run(2).unwrap();
    assert_eq!(subtract.get_state().a, 0o0000000);
    assert_eq!(subtract.get_state().q, 0o1100743);
    assert!(!subtract.get_state().overflow);
}

#[test]
fn negative_double_words_ignore_the_duplicated_q_sign_during_arithmetic() {
    let mut simulator = Simulator::new(4096).unwrap();
    load_double(&mut simulator, 0o3777776, 0o3774044, 300);
    load_double(&mut simulator, 0o0000000, 0o0000001, 302);
    simulator
        .load_words(&[instruction(0o10, 300, 0), instruction(0o11, 302, 0)], 200)
        .unwrap();
    simulator.set_program_counter(200).unwrap();

    simulator.run(2).unwrap();

    assert_eq!(simulator.get_state().a, 0o3777776);
    assert_eq!(simulator.get_state().q, 0o3774045);
}

#[test]
fn double_add_overflow_wraps_at_the_architectural_thirty_nine_bit_width() {
    let mut simulator = Simulator::new(4096).unwrap();
    load_double(&mut simulator, 0o1777777, 0o1777777, 300);
    load_double(&mut simulator, 0o0000000, 0o0000001, 302);
    load_double(&mut simulator, 0o0000000, 0o0000000, 304);
    simulator
        .load_words(
            &[
                instruction(0o10, 300, 0),
                instruction(0o11, 302, 0),
                instruction(0o11, 304, 0),
            ],
            200,
        )
        .unwrap();
    simulator.set_program_counter(200).unwrap();

    simulator.run(3).unwrap();

    assert_eq!(simulator.get_state().a, 0o2000000);
    assert_eq!(simulator.get_state().q, 0o2000000);
    assert!(simulator.get_state().overflow);
}

#[test]
fn manual_multiply_and_divide_vectors_match_the_reference_manual() {
    let mut multiply = Simulator::new(4096).unwrap();
    load_double(&mut multiply, 0o0000000, 0o0122315, 300);
    multiply.write_word(302, 0o0146626).unwrap();
    multiply
        .load_words(&[instruction(0o10, 300, 0), instruction(0o15, 302, 0)], 200)
        .unwrap();
    multiply.set_program_counter(200).unwrap();
    multiply.run(2).unwrap();
    assert_eq!(multiply.get_state().a, 0o0010213);
    assert_eq!(multiply.get_state().q, 0o0134436);
    assert!(!multiply.get_state().overflow);

    let mut multiply_add = Simulator::new(4096).unwrap();
    load_double(&mut multiply_add, 0o0112103, 0o1460716, 300);
    multiply_add.write_word(302, 0o0146626).unwrap();
    multiply_add
        .load_words(&[instruction(0o10, 300, 0), instruction(0o15, 302, 0)], 200)
        .unwrap();
    multiply_add.set_program_counter(200).unwrap();
    multiply_add.run(2).unwrap();
    assert_eq!(multiply_add.get_state().a, 0o0122001);
    assert_eq!(multiply_add.get_state().q, 0o1754367);
    assert!(!multiply_add.get_state().overflow);

    let mut divide = Simulator::new(4096).unwrap();
    load_double(&mut divide, 0o0000000, 0o1777674, 300);
    divide.write_word(302, 0o0146626).unwrap();
    divide
        .load_words(&[instruction(0o10, 300, 0), instruction(0o16, 302, 0)], 200)
        .unwrap();
    divide.set_program_counter(200).unwrap();
    divide.run(2).unwrap();
    assert_eq!(divide.get_state().a, 0o0000011);
    assert_eq!(divide.get_state().q, 0o0142566);
    assert!(!divide.get_state().overflow);
}

#[test]
fn maximum_negative_multiply_sets_the_documented_overflow_result() {
    let mut simulator = Simulator::new(4096).unwrap();
    load_double(&mut simulator, 0o0000000, 0o2000000, 300);
    simulator.write_word(302, 0o2000000).unwrap();
    simulator
        .load_words(&[instruction(0o10, 300, 0), instruction(0o15, 302, 0)], 200)
        .unwrap();
    simulator.set_program_counter(200).unwrap();

    simulator.run(2).unwrap();

    assert_eq!(simulator.get_state().a, 0o2000000);
    assert_eq!(simulator.get_state().q, 0o2000000);
    assert!(simulator.get_state().overflow);
}

#[test]
fn illegal_divide_sets_overflow_without_mutating_the_dividend() {
    let mut simulator = Simulator::new(4096).unwrap();
    load_double(&mut simulator, 0o0000001, 0o0000002, 300);
    simulator.write_word(302, 0).unwrap();
    simulator
        .load_words(&[instruction(0o10, 300, 0), instruction(0o16, 302, 0)], 200)
        .unwrap();
    simulator.set_program_counter(200).unwrap();

    simulator.run(2).unwrap();

    assert_eq!(simulator.get_state().a, 0o0000001);
    assert_eq!(simulator.get_state().q, 0o0000002);
    assert!(simulator.get_state().overflow);

    let mut oversized = Simulator::new(4096).unwrap();
    load_double(&mut oversized, 0o0000010, 0o0000002, 300);
    oversized.write_word(302, 0o0000005).unwrap();
    oversized
        .load_words(&[instruction(0o10, 300, 0), instruction(0o16, 302, 0)], 200)
        .unwrap();
    oversized.set_program_counter(200).unwrap();
    oversized.run(2).unwrap();
    assert_eq!(oversized.get_state().a, 0o0000010);
    assert_eq!(oversized.get_state().q, 0o0000002);
    assert!(oversized.get_state().overflow);
}

#[test]
fn manual_shift_right_double_vector_uses_only_the_two_nineteen_bit_data_fields() {
    let mut simulator = Simulator::new(4096).unwrap();
    load_double(&mut simulator, 0o1234567, 0o3654321, 300);
    simulator
        .load_words(
            &[instruction(0o10, 300, 0), assemble_shift("SRD", 6).unwrap()],
            200,
        )
        .unwrap();
    simulator.set_program_counter(200).unwrap();

    simulator.run(2).unwrap();

    assert_eq!(simulator.get_state().a, 0o0012345);
    assert_eq!(simulator.get_state().q, 0o1576543);
}

#[test]
fn negative_shift_right_double_sign_extends_the_thirty_eight_data_bits() {
    let mut simulator = Simulator::new(4096).unwrap();
    load_double(&mut simulator, 0o3777777, 0o3777770, 300);
    simulator
        .load_words(
            &[instruction(0o10, 300, 0), assemble_shift("SRD", 1).unwrap()],
            200,
        )
        .unwrap();
    simulator.set_program_counter(200).unwrap();

    simulator.run(2).unwrap();

    assert_eq!(simulator.get_state().a, 0o3777777);
    assert_eq!(simulator.get_state().q, 0o3777774);
}

#[test]
fn double_compare_ignores_operand_and_q_duplicate_sign_bits() {
    let mut simulator = Simulator::new(4096).unwrap();
    load_double(&mut simulator, 0o3777776, 0o3774044, 300);
    load_double(&mut simulator, 0o3777776, 0o1774044, 302);
    simulator
        .load_words(&[instruction(0o10, 300, 0), instruction(0o22, 302, 0)], 200)
        .unwrap();
    simulator.set_program_counter(200).unwrap();

    simulator.run(2).unwrap();

    assert_eq!(simulator.get_state().pc, 203);
}

#[test]
fn zero_count_double_shifts_still_apply_the_documented_sign_transfers() {
    let mut srd = Simulator::new(4096).unwrap();
    load_double(&mut srd, 0o2000001, 0o0000002, 300);
    srd.load_words(
        &[instruction(0o10, 300, 0), assemble_shift("SRD", 0).unwrap()],
        200,
    )
    .unwrap();
    srd.set_program_counter(200).unwrap();
    srd.run(2).unwrap();
    assert_eq!(srd.get_state().a, 0o2000001);
    assert_eq!(srd.get_state().q, 0o2000002);

    for mnemonic in ["SCD", "NAQ", "ANQ"] {
        let mut shift = Simulator::new(4096).unwrap();
        load_double(&mut shift, 0o2000001, 0o0000002, 300);
        shift
            .load_words(
                &[
                    instruction(0o10, 300, 0),
                    assemble_shift(mnemonic, 0).unwrap(),
                ],
                200,
            )
            .unwrap();
        shift.set_program_counter(200).unwrap();
        shift.run(2).unwrap();
        assert_eq!(shift.get_state().a, 0o2000001, "{mnemonic}");
        assert_eq!(shift.get_state().q, 0o2000002, "{mnemonic}");
    }

    let mut sld = Simulator::new(4096).unwrap();
    load_double(&mut sld, 0o0000001, 0o2000002, 300);
    sld.load_words(
        &[instruction(0o10, 300, 0), assemble_shift("SLD", 0).unwrap()],
        200,
    )
    .unwrap();
    sld.set_program_counter(200).unwrap();
    sld.run(2).unwrap();
    assert_eq!(sld.get_state().a, 0o2000001);
    assert_eq!(sld.get_state().q, 0o2000002);

    let mut latched_overflow = Simulator::new(4096).unwrap();
    load_double(&mut latched_overflow, 0o0000000, 0o2000000, 300);
    latched_overflow.write_word(302, 0o2000000).unwrap();
    latched_overflow
        .load_words(
            &[
                instruction(0o10, 300, 0),
                instruction(0o15, 302, 0),
                assemble_shift("SLD", 0).unwrap(),
            ],
            200,
        )
        .unwrap();
    latched_overflow.set_program_counter(200).unwrap();
    latched_overflow.run(3).unwrap();
    assert!(latched_overflow.get_state().overflow);
}

#[test]
fn normalize_remainder_always_uses_absolute_location_zero() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.write_word(4, 0o1234567).unwrap();
    simulator.write_word(300, 1).unwrap();
    simulator
        .load_words(
            &[
                instruction(0o00, 300, 0),
                assemble_fixed("SXG").unwrap(),
                assemble_shift("NOR", 0).unwrap(),
            ],
            200,
        )
        .unwrap();
    simulator.set_program_counter(200).unwrap();

    simulator.run(3).unwrap();

    assert_eq!(simulator.get_state().selected_x_group, 1);
    assert_eq!(simulator.read_word(0).unwrap(), 0);
    assert_eq!(simulator.read_word(4).unwrap(), 0o1234567);
}

#[test]
fn zero_count_double_normalize_copies_q_sign_to_a_and_clears_location_zero() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.write_word(0, 0o1777777).unwrap();
    load_double(&mut simulator, 0o0000001, 0o2000002, 300);
    simulator
        .load_words(
            &[instruction(0o10, 300, 0), assemble_shift("DNO", 0).unwrap()],
            200,
        )
        .unwrap();
    simulator.set_program_counter(200).unwrap();

    simulator.run(2).unwrap();

    assert_eq!(simulator.get_state().a, 0o2000001);
    assert_eq!(simulator.get_state().q, 0o2000002);
    assert_eq!(simulator.read_word(0).unwrap(), 0);
}

#[test]
fn manual_double_normalize_vectors_shift_data_and_store_the_remainder_in_zero() {
    let mut short = Simulator::new(4096).unwrap();
    load_double(&mut short, 0o0001234, 0o0076543, 300);
    short
        .load_words(
            &[instruction(0o10, 300, 0), assemble_shift("DNO", 6).unwrap()],
            200,
        )
        .unwrap();
    short.set_program_counter(200).unwrap();
    short.run(2).unwrap();
    assert_eq!(short.get_state().a, 0o0123403);
    assert_eq!(short.get_state().q, 0o1654300);
    assert_eq!(short.read_word(0).unwrap(), 0);

    let mut limited = Simulator::new(4096).unwrap();
    load_double(&mut limited, 0o0001777, 0o0000177, 300);
    limited
        .load_words(
            &[
                instruction(0o10, 300, 0),
                assemble_shift("DNO", 15).unwrap(),
            ],
            200,
        )
        .unwrap();
    limited.set_program_counter(200).unwrap();
    limited.run(2).unwrap();
    assert_eq!(limited.get_state().a, 0o1777000);
    assert_eq!(limited.get_state().q, 0o0177000);
    assert_eq!(limited.read_word(0).unwrap(), 6);
}
