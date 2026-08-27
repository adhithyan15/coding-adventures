use coding_adventures_ge225_simulator::{
    assemble_fixed, decode_instruction, encode_instruction, Simulator,
};

const SIGN_BIT: i32 = 1 << 19;
const DECIMAL_FLAG_BIT: i32 = 1 << 18;

fn instruction(opcode: i32, address: i32) -> i32 {
    encode_instruction(opcode, 0, address).unwrap()
}

fn bcd(negative: bool, flagged: bool, digits: i32) -> i32 {
    let hundreds = digits / 100;
    let tens = (digits / 10) % 10;
    let ones = digits % 10;
    (if negative { SIGN_BIT } else { 0 })
        | (if flagged { DECIMAL_FLAG_BIT } else { 0 })
        | (hundreds << 12)
        | (tens << 6)
        | ones
}

#[test]
fn manual_single_decimal_add_and_subtract_examples() {
    let mut sim = Simulator::new(4096).unwrap();
    sim.load_words(
        &[
            assemble_fixed("SET_DECMODE").unwrap(),
            instruction(0o00, 20),
            instruction(0o01, 21),
            instruction(0o00, 20),
            instruction(0o01, 22),
            instruction(0o00, 20),
            instruction(0o02, 21),
            instruction(0o00, 20),
            instruction(0o02, 22),
        ],
        4,
    )
    .unwrap();
    sim.write_word(20, bcd(false, true, 444)).unwrap();
    sim.write_word(21, bcd(false, false, 333)).unwrap();
    sim.write_word(22, bcd(true, false, 667)).unwrap();
    sim.set_program_counter(4).unwrap();

    sim.run(3).unwrap();
    assert_eq!(sim.get_state().a, bcd(false, true, 777));
    sim.run(2).unwrap();
    assert_eq!(sim.get_state().a, bcd(false, true, 111));
    sim.run(2).unwrap();
    assert_eq!(sim.get_state().a, bcd(false, true, 111));
    sim.run(2).unwrap();
    assert_eq!(sim.get_state().a, bcd(false, true, 777));
}

#[test]
fn manual_double_decimal_examples_and_negative_complement() {
    let mut sim = Simulator::new(4096).unwrap();
    sim.load_words(
        &[
            assemble_fixed("SET_DECMODE").unwrap(),
            instruction(0o10, 20),
            instruction(0o11, 22),
            instruction(0o10, 20),
            instruction(0o12, 22),
            instruction(0o10, 20),
            instruction(0o11, 24),
        ],
        4,
    )
    .unwrap();
    sim.load_words(
        &[
            bcd(false, true, 543),
            bcd(false, false, 210),
            bcd(false, true, 123),
            bcd(false, false, 456),
            bcd(true, true, 876),
            bcd(false, false, 544),
        ],
        20,
    )
    .unwrap();
    sim.set_program_counter(4).unwrap();

    sim.run(3).unwrap();
    let state = sim.get_state();
    assert_eq!(state.a, bcd(false, true, 666));
    assert_eq!(state.q, bcd(false, false, 666));

    sim.run(2).unwrap();
    let state = sim.get_state();
    assert_eq!(state.a, bcd(false, true, 419));
    assert_eq!(state.q, bcd(false, false, 754));

    sim.run(2).unwrap();
    let state = sim.get_state();
    assert_eq!(state.a, bcd(false, true, 419));
    assert_eq!(state.q, bcd(false, false, 754));
}

#[test]
fn decimal_add_one_subtract_one_overflow_and_carry_latch() {
    let mut sim = Simulator::new(4096).unwrap();
    sim.load_words(
        &[
            assemble_fixed("SET_DECMODE").unwrap(),
            instruction(0o00, 20),
            assemble_fixed("ADO").unwrap(),
            instruction(0o00, 21),
            assemble_fixed("SBO").unwrap(),
            instruction(0o00, 22),
            assemble_fixed("ADO").unwrap(),
            instruction(0o00, 23),
            instruction(0o01, 24),
            instruction(0o00, 25),
            instruction(0o01, 26),
        ],
        4,
    )
    .unwrap();
    sim.write_word(20, bcd(false, true, 832)).unwrap();
    sim.write_word(21, bcd(true, true, 237)).unwrap();
    sim.write_word(22, bcd(false, true, 999)).unwrap();
    sim.write_word(23, bcd(false, false, 999)).unwrap();
    sim.write_word(24, bcd(false, false, 1)).unwrap();
    sim.write_word(25, bcd(false, true, 0)).unwrap();
    sim.write_word(26, bcd(false, true, 0)).unwrap();
    sim.set_program_counter(4).unwrap();

    sim.run(3).unwrap();
    assert_eq!(sim.get_state().a, bcd(false, true, 833));
    sim.run(2).unwrap();
    assert_eq!(sim.get_state().a, bcd(true, true, 236));

    sim.run(2).unwrap();
    let state = sim.get_state();
    assert_eq!(state.a, bcd(true, true, 0));
    assert!(state.overflow);

    sim.run(2).unwrap();
    let state = sim.get_state();
    assert_eq!(state.a, bcd(false, false, 0));
    assert_eq!(state.decimal_carry, 1);
    sim.run(2).unwrap();
    let state = sim.get_state();
    assert_eq!(state.a, bcd(false, true, 1));
    assert_eq!(state.decimal_carry, 0);
    sim.clear_decimal_carry();
    assert_eq!(sim.get_state().decimal_carry, 0);
}

#[test]
fn decimal_mode_rejects_invalid_bcd_without_changing_arithmetic_registers() {
    let mut sim = Simulator::new(4096).unwrap();
    sim.load_words(
        &[
            assemble_fixed("SET_DECMODE").unwrap(),
            instruction(0o00, 20),
            instruction(0o01, 21),
        ],
        4,
    )
    .unwrap();
    sim.write_word(20, bcd(false, true, 123)).unwrap();
    sim.write_word(21, DECIMAL_FLAG_BIT | (0x0a << 12)).unwrap();
    sim.set_program_counter(4).unwrap();
    sim.run(2).unwrap();
    let before = sim.get_state();

    let error = sim.step().unwrap_err();
    assert!(error.contains("invalid GE-225 BCD digits"));
    let after = sim.get_state();
    assert_eq!(after, before);
}

#[test]
fn flagged_decimal_operand_requires_a_flagged_accumulator_field() {
    let mut sim = Simulator::new(4096).unwrap();
    sim.load_words(
        &[
            assemble_fixed("SET_DECMODE").unwrap(),
            instruction(0o00, 20),
            instruction(0o01, 21),
        ],
        4,
    )
    .unwrap();
    sim.write_word(20, bcd(false, false, 123)).unwrap();
    sim.write_word(21, bcd(false, true, 456)).unwrap();
    sim.set_program_counter(4).unwrap();
    sim.run(2).unwrap();
    let before = sim.get_state();

    let error = sim.step().unwrap_err();
    assert!(error.contains("operand is flagged while A is unflagged"));
    assert_eq!(sim.get_state(), before);
}

#[test]
fn real_time_clock_load_store_and_day_wrap_are_deterministic() {
    let mut sim = Simulator::new(4096).unwrap();
    sim.set_clock_sixths(0o1205701).unwrap();
    sim.load_words(
        &[
            assemble_fixed("LAC").unwrap(),
            instruction(0o00, 20),
            assemble_fixed("LCA").unwrap(),
        ],
        4,
    )
    .unwrap();
    sim.write_word(20, SIGN_BIT | 0o0772200).unwrap();
    sim.set_program_counter(4).unwrap();

    sim.step().unwrap();
    assert_eq!(sim.get_state().a, 0o1205701);
    sim.run(2).unwrap();
    assert_eq!(sim.get_state().clock_sixths, 0o0772200);

    sim.set_clock_sixths(24 * 60 * 60 * 6 - 1).unwrap();
    sim.advance_clock_sixths(1);
    assert_eq!(sim.get_state().clock_sixths, 0);
    sim.set_clock_sixths((1 << 19) - 1).unwrap();
    sim.advance_clock_sixths(1);
    assert_eq!(sim.get_state().clock_sixths, 0);
    sim.advance_clock_sixths(u64::MAX);
    assert!(sim.get_state().clock_sixths < 24 * 60 * 60 * 6);
    assert!(sim.set_clock_sixths(-1).is_err());
    assert!(sim.set_clock_sixths(1 << 19).is_err());
}

#[test]
fn opcode_24_uses_the_manuals_canonical_mov_name() {
    let sim = Simulator::new(4096).unwrap();
    let word = instruction(0o24, 0o1234);
    assert_eq!(decode_instruction(word), (0o24, 0, 0o1234));
    assert_eq!(sim.disassemble_word(word).unwrap(), "MOV 0x29C,X0");
}
