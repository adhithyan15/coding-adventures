use coding_adventures_ge225_simulator::{
    assemble_aau_branch, assemble_aau_general, assemble_aau_memory, pack_aau_words,
    unpack_aau_words, AauMode, Simulator,
};

const PROGRAM: i32 = 0o1000;
const SIGN_BIT: i32 = 1 << 19;
const DATA_MASK: i32 = SIGN_BIT - 1;

fn fixed_words(value: i64) -> (i32, i32) {
    let bits = value & ((1_i64 << 39) - 1);
    let high = ((bits >> 19) & ((1_i64 << 20) - 1)) as i32;
    let low = ((bits & i64::from(DATA_MASK)) as i32) | (high & SIGN_BIT);
    (high, low)
}

fn fixed_value(raw: u64) -> i64 {
    let (high, low) = unpack_aau_words(raw);
    let bits = (i64::from(high) << 19) | i64::from(low & DATA_MASK);
    if high & SIGN_BIT != 0 {
        bits - (1_i64 << 39)
    } else {
        bits
    }
}

fn float_raw(exponent: i32, mantissa: i64) -> u64 {
    let exponent_bits = if (0..=255).contains(&exponent) {
        exponent
    } else if (-256..=-1).contains(&exponent) {
        0x100 | (-exponent & 0xff)
    } else if exponent < -256 {
        (-exponent) & 0xff
    } else {
        0x100 | ((exponent - 256) & 0xff)
    };
    let mantissa_bits = mantissa & ((1_i64 << 31) - 1);
    let sign = (mantissa_bits >> 30) & 1;
    let upper = (mantissa_bits >> 19) & 0x7ff;
    let lower = mantissa_bits & i64::from(DATA_MASK);
    (((exponent_bits as u64) << 31) | ((upper as u64) << 20) | ((sign as u64) << 19) | lower as u64)
        & ((1_u64 << 40) - 1)
}

fn float_parts(raw: u64) -> (i32, i64) {
    let exponent_bits = ((raw >> 31) & 0x1ff) as i32;
    let exponent_magnitude = exponent_bits & 0xff;
    let exponent = if exponent_bits & 0x100 == 0 {
        exponent_magnitude
    } else if exponent_magnitude == 0 {
        -256
    } else {
        -exponent_magnitude
    };
    let mantissa_bits =
        (((raw >> 19) & 1) << 30) | (((raw >> 20) & 0x7ff) << 19) | (raw & DATA_MASK as u64);
    let mantissa = if mantissa_bits & (1 << 30) != 0 {
        mantissa_bits as i64 - (1_i64 << 31)
    } else {
        mantissa_bits as i64
    };
    (exponent, mantissa)
}

fn load_raw(simulator: &mut Simulator, address: i32, raw: u64) {
    let (first, second) = unpack_aau_words(raw);
    simulator.load_words(&[first, second], address).unwrap();
}

fn simulator_with_program(words: &[i32]) -> Simulator {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.load_words(words, PROGRAM).unwrap();
    simulator.set_program_counter(PROGRAM).unwrap();
    simulator
}

#[test]
fn exact_aau_words_and_disassembly_match_the_manual() {
    let simulator = Simulator::new(4096).unwrap();
    let general = [
        ("SET_FIXPOINT", 0o3500010),
        ("SET_NFLPOINT", 0o3100010),
        ("SET_UFLPOINT", 0o3200010),
        ("LAQ", 0o3600002),
        ("LQA", 0o3200002),
        ("MAQ", 0o3100002),
        ("XAQ", 0o3500002),
        ("ROV", 0o3100004),
        ("RUN", 0o3200004),
        ("RIN", 0o3500004),
        ("NOX", 0o3100005),
    ];
    for (mnemonic, word) in general {
        assert_eq!(assemble_aau_general(mnemonic).unwrap(), word);
        assert_eq!(simulator.disassemble_word(word).unwrap(), mnemonic);
    }
    assert_eq!(assemble_aau_memory("FLD", 0o1234, 2).unwrap(), 0o3041234);
    assert_eq!(assemble_aau_memory("FDV", 0o1234, 0).unwrap(), 0o3601234);
    let branches = [
        ("BAR", 0o2514720),
        ("BAN", 0o2516720),
        ("BMI", 0o2514721),
        ("BPL", 0o2516721),
        ("BZE", 0o2514722),
        ("BNZ", 0o2516722),
        ("BOV", 0o2514723),
        ("BNO", 0o2516723),
        ("BUF", 0o2514724),
        ("BNU", 0o2516724),
        ("BOO", 0o2514725),
        ("BON", 0o2516725),
        ("BUO", 0o2514726),
        ("BUN", 0o2516726),
        ("BER", 0o2514727),
        ("BNE", 0o2516727),
    ];
    for (mnemonic, word) in branches {
        assert_eq!(assemble_aau_branch(mnemonic).unwrap(), word);
        assert_eq!(
            simulator.disassemble_word(word).unwrap(),
            format!("BAR {mnemonic}")
        );
    }
    assert_eq!(
        simulator
            .disassemble_word(assemble_aau_branch("BAR").unwrap())
            .unwrap(),
        "BAR BAR"
    );
    assert!(assemble_aau_general("UNKNOWN").is_err());
    assert!(assemble_aau_memory("UNKNOWN", 0, 0).is_err());
    assert!(assemble_aau_branch("UNKNOWN").is_err());
}

#[test]
fn aau_memory_instructions_use_cpu_modification_and_capture_the_modified_ix_word() {
    let raw = pack_aau_words(0o1234567, 0o3654321);
    let mut simulator = simulator_with_program(&[
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0, 1).unwrap(),
    ]);
    simulator.write_word(1, 0o400).unwrap();
    load_raw(&mut simulator, 0o400, raw);
    simulator.run(2).unwrap();
    assert_eq!(simulator.get_state().aau.ax, raw);
    assert_eq!(
        simulator.get_state().aau.ix,
        assemble_aau_memory("FLD", 0o400, 1).unwrap() as u64
    );

    for opcode in [0o34, 0o37] {
        let mut invalid = simulator_with_program(&[(opcode << 15) | 0o400]);
        let before = invalid.get_state();
        assert!(invalid.step().is_err());
        assert_eq!(invalid.get_state(), before);
    }
}

#[test]
fn load_store_and_internal_transfers_preserve_all_forty_bits() {
    let raw = pack_aau_words(0o1234567, 0o3654321);
    let mut simulator = simulator_with_program(&[
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_general("MAQ").unwrap(),
        assemble_aau_general("XAQ").unwrap(),
        assemble_aau_memory("FST", 0o402, 0).unwrap(),
    ]);
    load_raw(&mut simulator, 0o400, raw);
    simulator.run(5).unwrap();

    let state = simulator.get_state();
    assert_eq!(state.aau.ax, raw);
    assert_eq!(state.aau.qx, 0);
    assert_eq!(simulator.read_word(0o402).unwrap(), 0o1234567);
    assert_eq!(simulator.read_word(0o403).unwrap(), 0o3654321);

    let mut odd = simulator_with_program(&[
        assemble_aau_memory("FLD", 0o401, 1).unwrap(),
        assemble_aau_memory("FST", 0o405, 1).unwrap(),
    ]);
    odd.write_word(1, 0).unwrap();
    odd.write_word(0o401, 0o765432).unwrap();
    odd.run(2).unwrap();
    assert_eq!(odd.get_state().aau.ax, pack_aau_words(0o765432, 0o765432));
    assert_eq!(odd.read_word(0o405).unwrap(), 0o765432);
}

#[test]
fn fixed_add_subtract_and_alert_holds_follow_the_manual() {
    let mut simulator = simulator_with_program(&[
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FAD", 0o402, 0).unwrap(),
        assemble_aau_memory("FSU", 0o404, 0).unwrap(),
    ]);
    let (forty, forty_low) = fixed_words(40);
    let (two, two_low) = fixed_words(2);
    let (one, one_low) = fixed_words(1);
    simulator.load_words(&[forty, forty_low], 0o400).unwrap();
    simulator.load_words(&[two, two_low], 0o402).unwrap();
    simulator.load_words(&[one, one_low], 0o404).unwrap();
    simulator.run(4).unwrap();
    assert_eq!(fixed_value(simulator.get_state().aau.ax), 41);
    assert_eq!(simulator.get_state().aau.mode, Some(AauMode::FixedPoint));

    let maximum = (1_i64 << 38) - 1;
    let mut overflow = simulator_with_program(&[
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FAD", 0o402, 0).unwrap(),
        assemble_aau_branch("BOV").unwrap(),
        assemble_aau_general("ROV").unwrap(),
    ]);
    let max_words = fixed_words(maximum);
    let one_words = fixed_words(1);
    overflow
        .load_words(&[max_words.0, max_words.1], 0o400)
        .unwrap();
    overflow
        .load_words(&[one_words.0, one_words.1], 0o402)
        .unwrap();
    overflow.run(4).unwrap();
    assert!(overflow.get_state().aau.overflow);
    assert!(overflow.get_state().aau.overflow_hold);
    overflow.step().unwrap();
    assert!(!overflow.get_state().aau.overflow);
    assert!(!overflow.get_state().aau.overflow_hold);
}

#[test]
fn fixed_multiply_and_divide_use_qx_and_the_ax_qx_product() {
    let mut multiply = simulator_with_program(&[
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_memory("FMP", 0o402, 0).unwrap(),
    ]);
    let six = fixed_words(6);
    let seven = fixed_words(7);
    multiply.load_words(&[six.0, six.1], 0o400).unwrap();
    multiply.load_words(&[seven.0, seven.1], 0o402).unwrap();
    multiply.run(4).unwrap();
    assert_eq!(fixed_value(multiply.get_state().aau.ax), 0);
    assert_eq!(fixed_value(multiply.get_state().aau.qx), 42);

    multiply
        .load_words(
            &[assemble_aau_memory("FDV", 0o402, 0).unwrap()],
            PROGRAM + 4,
        )
        .unwrap();
    multiply.step().unwrap();
    assert_eq!(fixed_value(multiply.get_state().aau.ax), 6);
    assert_eq!(fixed_value(multiply.get_state().aau.qx), 0);

    let mut negative = simulator_with_program(&[
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_memory("FMP", 0o402, 0).unwrap(),
        assemble_aau_memory("FDV", 0o402, 0).unwrap(),
    ]);
    let minus_six = fixed_words(-6);
    negative
        .load_words(&[minus_six.0, minus_six.1], 0o400)
        .unwrap();
    negative.load_words(&[seven.0, seven.1], 0o402).unwrap();
    negative.run(4).unwrap();
    negative.step().unwrap();
    assert_eq!(fixed_value(negative.get_state().aau.ax), -6);
    assert_eq!(fixed_value(negative.get_state().aau.qx), 0);

    let mut invalid_negative = simulator_with_program(&[
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_memory("FMP", 0o402, 0).unwrap(),
        assemble_aau_memory("FDV", 0o404, 0).unwrap(),
    ]);
    invalid_negative
        .load_words(&[minus_six.0, minus_six.1], 0o400)
        .unwrap();
    invalid_negative
        .load_words(&[seven.0, seven.1], 0o402)
        .unwrap();
    let one = fixed_words(1);
    invalid_negative.load_words(&[one.0, one.1], 0o404).unwrap();
    invalid_negative.run(5).unwrap();
    assert!(invalid_negative.get_state().aau.overflow);
    assert_eq!(fixed_value(invalid_negative.get_state().aau.ax), 0);
    assert_eq!(fixed_value(invalid_negative.get_state().aau.qx), 42);
}

#[test]
fn normalized_float_add_multiply_divide_and_nox_are_deterministic() {
    let half = float_raw(0, 1 << 29);
    let mut add = simulator_with_program(&[
        assemble_aau_general("SET_NFLPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FAD", 0o402, 0).unwrap(),
    ]);
    load_raw(&mut add, 0o400, half);
    load_raw(&mut add, 0o402, half);
    add.run(3).unwrap();
    assert_eq!(float_parts(add.get_state().aau.ax), (1, 1 << 29));

    let mut multiply = simulator_with_program(&[
        assemble_aau_general("SET_NFLPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_memory("FMP", 0o402, 0).unwrap(),
        assemble_aau_memory("FDV", 0o402, 0).unwrap(),
    ]);
    load_raw(&mut multiply, 0o400, half);
    load_raw(&mut multiply, 0o402, half);
    multiply.run(4).unwrap();
    assert_eq!(float_parts(multiply.get_state().aau.ax), (-1, 1 << 29));
    multiply.step().unwrap();
    assert_eq!(float_parts(multiply.get_state().aau.ax), (0, 1 << 29));

    let unnormalized = float_raw(0, 1 << 27);
    let mut nox = simulator_with_program(&[
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("NOX").unwrap(),
    ]);
    load_raw(&mut nox, 0o400, unnormalized);
    nox.run(2).unwrap();
    assert_eq!(float_parts(nox.get_state().aau.ax), (-2, 1 << 29));

    let mut qx_exponent_wrap = simulator_with_program(&[
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("NOX").unwrap(),
    ]);
    load_raw(&mut qx_exponent_wrap, 0o400, float_raw(-255, 1 << 29));
    qx_exponent_wrap.run(2).unwrap();
    assert_eq!(float_parts(qx_exponent_wrap.get_state().aau.qx), (29, 0));

    let minus_half = float_raw(0, -(1 << 29));
    let mut signed = simulator_with_program(&[
        assemble_aau_general("SET_NFLPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FAD", 0o402, 0).unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_memory("FMP", 0o404, 0).unwrap(),
        assemble_aau_memory("FDV", 0o404, 0).unwrap(),
    ]);
    load_raw(&mut signed, 0o400, minus_half);
    load_raw(&mut signed, 0o402, half);
    load_raw(&mut signed, 0o404, half);
    signed.run(3).unwrap();
    assert_eq!(signed.get_state().aau.ax, 0);
    assert_eq!(signed.get_state().aau.qx, 0);
    signed.run(3).unwrap();
    assert_eq!(float_parts(signed.get_state().aau.ax), (-1, -(1 << 29)));
    signed.step().unwrap();
    assert_eq!(float_parts(signed.get_state().aau.ax), (0, -(1 << 29)));

    let mut divide_check = simulator_with_program(&[
        assemble_aau_general("SET_NFLPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FDV", 0o402, 0).unwrap(),
    ]);
    load_raw(&mut divide_check, 0o400, minus_half);
    load_raw(&mut divide_check, 0o402, float_raw(0, 1 << 28));
    divide_check.run(3).unwrap();
    assert!(divide_check.get_state().aau.overflow);
    assert!(float_parts(divide_check.get_state().aau.ax).1 >= 0);
}

#[test]
fn floating_modes_distinguish_normalization_and_latch_exponent_alerts() {
    let eighth_mantissa = float_raw(0, 1 << 27);
    let mut unnormalized = simulator_with_program(&[
        assemble_aau_general("SET_UFLPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FAD", 0o402, 0).unwrap(),
    ]);
    load_raw(&mut unnormalized, 0o400, eighth_mantissa);
    load_raw(&mut unnormalized, 0o402, eighth_mantissa);
    unnormalized.run(3).unwrap();
    assert_eq!(float_parts(unnormalized.get_state().aau.ax), (0, 1 << 28));
    assert_eq!(
        unnormalized.get_state().aau.mode,
        Some(AauMode::UnnormalizedFloatingPoint)
    );

    let half_at_255 = float_raw(255, 1 << 29);
    let mut overflow = simulator_with_program(&[
        assemble_aau_general("SET_NFLPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FAD", 0o402, 0).unwrap(),
    ]);
    load_raw(&mut overflow, 0o400, half_at_255);
    load_raw(&mut overflow, 0o402, half_at_255);
    overflow.run(3).unwrap();
    assert!(overflow.get_state().aau.overflow);
    assert!(overflow.get_state().aau.overflow_hold);

    let tiny_qx = float_raw(-200, 1 << 29);
    let tiny_operand = float_raw(-100, 1 << 29);
    let mut underflow = simulator_with_program(&[
        assemble_aau_general("SET_NFLPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_memory("FMP", 0o402, 0).unwrap(),
    ]);
    load_raw(&mut underflow, 0o400, tiny_qx);
    load_raw(&mut underflow, 0o402, tiny_operand);
    underflow.run(4).unwrap();
    assert!(underflow.get_state().aau.underflow);
    assert!(underflow.get_state().aau.underflow_hold);
    assert_eq!(underflow.get_state().aau.ax, 0);
    assert_eq!(underflow.get_state().aau.qx, 0);
}

#[test]
fn normalized_divide_with_zero_quotient_terminates_deterministically() {
    let mut simulator = simulator_with_program(&[
        assemble_aau_general("SET_NFLPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_memory("FLD", 0o402, 0).unwrap(),
        assemble_aau_memory("FDV", 0o404, 0).unwrap(),
    ]);
    load_raw(&mut simulator, 0o400, float_raw(0, 1));
    load_raw(&mut simulator, 0o402, 0);
    load_raw(&mut simulator, 0o404, float_raw(0, 1 << 29));

    simulator.run(5).unwrap();

    assert_eq!(simulator.get_state().aau.ax, 0);
    assert_eq!(float_parts(simulator.get_state().aau.qx), (-30, 1));
}

#[test]
fn aau_preflight_fails_closed_for_mode_address_and_memory_errors() {
    let mut no_mode = simulator_with_program(&[assemble_aau_memory("FAD", 0o400, 0).unwrap()]);
    let before = no_mode.get_state();
    assert!(no_mode.step().is_err());
    assert_eq!(no_mode.get_state(), before);

    let mut reserved = simulator_with_program(&[assemble_aau_memory("FLD", 0o17, 0).unwrap()]);
    let before = reserved.get_state();
    assert!(reserved.step().is_err());
    assert_eq!(reserved.get_state(), before);

    let mut not_ready = simulator_with_program(&[assemble_aau_general("SET_FIXPOINT").unwrap()]);
    not_ready.set_aau_ready(false);
    let before = not_ready.get_state();
    assert!(not_ready.step().is_err());
    assert_eq!(not_ready.get_state(), before);

    let mut boundary = Simulator::new(4096).unwrap();
    boundary
        .load_words(&[assemble_aau_memory("FLD", 4095, 1).unwrap()], PROGRAM)
        .unwrap();
    boundary.write_word(1, 1).unwrap();
    boundary.set_program_counter(PROGRAM).unwrap();
    let before = boundary.get_state();
    assert!(boundary.step().is_err());
    assert_eq!(boundary.get_state(), before);
}

#[test]
fn aau_status_skip_past_memory_fails_before_clearing_ix_or_holds() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator
        .write_word(4094, assemble_aau_branch("BAN").unwrap())
        .unwrap();
    simulator.set_program_counter(4094).unwrap();
    let before = simulator.get_state();

    let error = simulator.step().unwrap_err();

    assert!(error.contains("address out of range: 4096"));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn all_aau_status_branches_use_skip_semantics_and_hold_tests_clear_holds() {
    let minimum = fixed_words(-(1_i64 << 38));
    let minus_one = fixed_words(-1);
    let mut simulator = simulator_with_program(&[
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FAD", 0o402, 0).unwrap(),
        assemble_aau_branch("BUF").unwrap(),
        assemble_aau_branch("BUO").unwrap(),
        assemble_aau_branch("BUN").unwrap(),
    ]);
    simulator
        .load_words(&[minimum.0, minimum.1], 0o400)
        .unwrap();
    simulator
        .load_words(&[minus_one.0, minus_one.1], 0o402)
        .unwrap();
    simulator.run(4).unwrap();
    assert!(simulator.get_state().aau.underflow);
    assert!(simulator.get_state().aau.underflow_hold);
    simulator.step().unwrap();
    assert!(!simulator.get_state().aau.underflow_hold);
    simulator.step().unwrap();
    assert_eq!(simulator.get_state().pc, PROGRAM + 6);

    let mut readiness = simulator_with_program(&[
        assemble_aau_branch("BAR").unwrap(),
        assemble_aau_branch("BAN").unwrap(),
        assemble_aau_branch("BAN").unwrap(),
    ]);
    readiness.set_aau_ready(false);
    readiness.step().unwrap();
    assert_eq!(readiness.get_state().pc, PROGRAM + 2);
    readiness.step().unwrap();
    assert_eq!(readiness.get_state().pc, PROGRAM + 3);

    let negative = fixed_words(-1);
    let mut fixed_sign = simulator_with_program(&[
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_branch("BMI").unwrap(),
    ]);
    fixed_sign
        .load_words(&[negative.0, negative.1], 0o400)
        .unwrap();
    fixed_sign.run(3).unwrap();
    assert_eq!(fixed_sign.get_state().pc, PROGRAM + 3);

    let mut floating_sign = simulator_with_program(&[
        assemble_aau_general("SET_NFLPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_branch("BPL").unwrap(),
    ]);
    load_raw(&mut floating_sign, 0o400, float_raw(-1, 1 << 29));
    floating_sign.run(3).unwrap();
    assert_eq!(floating_sign.get_state().pc, PROGRAM + 3);

    let mut clear_status = simulator_with_program(&[
        assemble_aau_branch("BZE").unwrap(),
        assemble_aau_branch("BNO").unwrap(),
        assemble_aau_branch("BNU").unwrap(),
        assemble_aau_branch("BON").unwrap(),
        assemble_aau_branch("BUN").unwrap(),
        assemble_aau_branch("BNE").unwrap(),
    ]);
    clear_status.run(6).unwrap();
    assert_eq!(clear_status.get_state().pc, PROGRAM + 6);
}

#[test]
fn transient_alerts_clear_on_the_next_accepted_instruction_but_holds_persist() {
    let maximum = fixed_words((1_i64 << 38) - 1);
    let one = fixed_words(1);
    let mut simulator = simulator_with_program(&[
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FAD", 0o402, 0).unwrap(),
        assemble_aau_branch("BER").unwrap(),
        assemble_aau_memory("FLD", 0o404, 0).unwrap(),
    ]);
    simulator
        .load_words(&[maximum.0, maximum.1], 0o400)
        .unwrap();
    simulator.load_words(&[one.0, one.1], 0o402).unwrap();
    simulator.load_words(&[one.0, one.1], 0o404).unwrap();
    simulator.run(4).unwrap();
    assert!(simulator.get_state().aau.overflow);
    simulator.step().unwrap();
    assert!(!simulator.get_state().aau.overflow);
    assert!(simulator.get_state().aau.overflow_hold);
    simulator.clear_aau_alerts();
    assert!(!simulator.get_state().aau.overflow_hold);
}

#[test]
fn reset_clears_all_aau_state() {
    let mut simulator = simulator_with_program(&[
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
    ]);
    load_raw(&mut simulator, 0o400, pack_aau_words(1, 2));
    simulator.run(2).unwrap();
    simulator.reset();
    let aau = simulator.get_state().aau;
    assert_eq!(aau.mode, None);
    assert!(aau.ready);
    assert_eq!(aau.ax, 0);
    assert_eq!(aau.bx, 0);
    assert_eq!(aau.qx, 0);
    assert_eq!(aau.ix, 0);
    assert!(!aau.overflow);
    assert!(!aau.underflow);
    assert!(!aau.overflow_hold);
    assert!(!aau.underflow_hold);
}
