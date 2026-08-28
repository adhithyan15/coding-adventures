use coding_adventures_ge225_simulator::Simulator as Functional;
use ge225_gatelevel::{
    assemble_aau_branch, assemble_aau_general, assemble_aau_memory, pack_aau_words,
    unpack_aau_words, AauMode, Ge225GateLevel, CENTRAL_FLIP_FLOPS, MIN_MEMORY_WORDS,
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
    let magnitude = exponent_bits & 0xff;
    let exponent = if exponent_bits & 0x100 == 0 {
        magnitude
    } else if magnitude == 0 {
        -256
    } else {
        -magnitude
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

fn machine(words: &[i32]) -> Ge225GateLevel {
    let mut gate = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    gate.load_words(words, PROGRAM as usize).unwrap();
    gate.set_program_counter(PROGRAM).unwrap();
    gate
}

fn functional(words: &[i32]) -> Functional {
    let mut simulator = Functional::new(MIN_MEMORY_WORDS as i32).unwrap();
    simulator.load_words(words, PROGRAM).unwrap();
    simulator.set_program_counter(PROGRAM).unwrap();
    simulator
}

fn load_raw(gate: &mut Ge225GateLevel, address: i32, raw: u64) {
    let (first, second) = unpack_aau_words(raw);
    gate.load_words(&[first, second], address as usize).unwrap();
}

fn assert_aau_matches(gate: &Ge225GateLevel, oracle: &Functional) {
    let actual = gate.get_state().aau;
    let expected = oracle.get_state().aau;
    assert_eq!(
        actual.mode.map(|mode| format!("{mode:?}")),
        expected.mode.map(|mode| format!("{mode:?}"))
    );
    assert_eq!(actual.ready, expected.ready);
    assert_eq!(actual.ax, expected.ax);
    assert_eq!(actual.bx, expected.bx);
    assert_eq!(actual.qx, expected.qx);
    assert_eq!(actual.ix, expected.ix);
    assert_eq!(actual.overflow, expected.overflow);
    assert_eq!(actual.underflow, expected.underflow);
    assert_eq!(actual.overflow_hold, expected.overflow_hold);
    assert_eq!(actual.underflow_hold, expected.underflow_hold);
}

#[test]
fn exact_words_and_flip_flop_inventory_cover_the_complete_aau() {
    assert_eq!(assemble_aau_general("SET_FIXPOINT").unwrap(), 0o3500010);
    assert_eq!(assemble_aau_general("NOX").unwrap(), 0o3100005);
    assert_eq!(assemble_aau_memory("FLD", 0o1234, 2).unwrap(), 0o3041234);
    assert_eq!(assemble_aau_memory("FDV", 0o1234, 0).unwrap(), 0o3601234);
    assert_eq!(assemble_aau_branch("BAR").unwrap(), 0o2514720);
    assert_eq!(assemble_aau_branch("BNE").unwrap(), 0o2516727);
    assert!(assemble_aau_general("UNKNOWN").is_err());
    assert!(assemble_aau_memory("UNKNOWN", 0, 0).is_err());
    assert!(assemble_aau_branch("UNKNOWN").is_err());
    let gate = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    assert_eq!(
        gate.flip_flop_count(),
        MIN_MEMORY_WORDS * 20 + CENTRAL_FLIP_FLOPS
    );
    assert_eq!(CENTRAL_FLIP_FLOPS, 1_437);
}

#[test]
fn modification_capture_transfers_and_odd_word_rules_match_the_oracle() {
    let raw = pack_aau_words(0o1234567, 0o3654321);
    let program = [
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0, 1).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_general("MAQ").unwrap(),
        assemble_aau_general("XAQ").unwrap(),
        assemble_aau_memory("FST", 0o405, 1).unwrap(),
    ];
    let mut gate = machine(&program);
    let mut oracle = functional(&program);
    oracle.write_word(1, 0o400).unwrap();
    let (first, second) = unpack_aau_words(raw);
    oracle.load_words(&[first, second], 0o400).unwrap();
    gate.write_word(1, 0o400).unwrap();
    load_raw(&mut gate, 0o400, raw);
    gate.run(program.len()).unwrap();
    oracle.run(program.len()).unwrap();
    assert_aau_matches(&gate, &oracle);
    assert_eq!(
        gate.read_word(0o405).unwrap(),
        oracle.read_word(0o405).unwrap()
    );
    assert_eq!(
        gate.get_state().aau.ix,
        assemble_aau_memory("FST", 0o1005, 1).unwrap() as u64
    );

    for opcode in [0o34, 0o37] {
        let mut invalid = machine(&[(opcode << 15) | 0o400]);
        let before = invalid.get_state();
        assert!(invalid.step().is_err());
        assert_eq!(invalid.get_state(), before);
    }
}

#[test]
fn fixed_add_subtract_alerts_and_holds_match_the_oracle() {
    let program = [
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FAD", 0o402, 0).unwrap(),
        assemble_aau_branch("BOV").unwrap(),
        assemble_aau_general("ROV").unwrap(),
    ];
    let maximum = fixed_words((1_i64 << 38) - 1);
    let one = fixed_words(1);
    let mut gate = machine(&program);
    let mut oracle = functional(&program);
    gate.load_words(&[maximum.0, maximum.1], 0o400).unwrap();
    gate.load_words(&[one.0, one.1], 0o402).unwrap();
    oracle.load_words(&[maximum.0, maximum.1], 0o400).unwrap();
    oracle.load_words(&[one.0, one.1], 0o402).unwrap();
    for _ in 0..program.len() {
        gate.step().unwrap();
        oracle.step().unwrap();
        assert_aau_matches(&gate, &oracle);
    }
    assert_eq!(gate.get_state().aau.mode, Some(AauMode::FixedPoint));
}

#[test]
fn fixed_multiply_divide_and_negative_results_match_the_oracle() {
    for left in [6_i64, -6] {
        let program = [
            assemble_aau_general("SET_FIXPOINT").unwrap(),
            assemble_aau_memory("FLD", 0o400, 0).unwrap(),
            assemble_aau_general("LQA").unwrap(),
            assemble_aau_memory("FMP", 0o402, 0).unwrap(),
            assemble_aau_memory("FDV", 0o402, 0).unwrap(),
        ];
        let left_words = fixed_words(left);
        let seven = fixed_words(7);
        let mut gate = machine(&program);
        let mut oracle = functional(&program);
        gate.load_words(&[left_words.0, left_words.1], 0o400)
            .unwrap();
        gate.load_words(&[seven.0, seven.1], 0o402).unwrap();
        oracle
            .load_words(&[left_words.0, left_words.1], 0o400)
            .unwrap();
        oracle.load_words(&[seven.0, seven.1], 0o402).unwrap();
        gate.run(program.len()).unwrap();
        oracle.run(program.len()).unwrap();
        assert_aau_matches(&gate, &oracle);
        assert_eq!(fixed_value(gate.get_state().aau.ax), left);
        assert_eq!(fixed_value(gate.get_state().aau.qx), 0);
    }
}

#[test]
fn floating_add_multiply_divide_and_normalize_match_the_oracle() {
    let half = float_raw(0, 1 << 29);
    let program = [
        assemble_aau_general("SET_NFLPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_memory("FMP", 0o402, 0).unwrap(),
        assemble_aau_memory("FDV", 0o402, 0).unwrap(),
        assemble_aau_memory("FAD", 0o404, 0).unwrap(),
        assemble_aau_general("NOX").unwrap(),
    ];
    let mut gate = machine(&program);
    let mut oracle = functional(&program);
    for (address, raw) in [(0o400, half), (0o402, half), (0o404, float_raw(0, 1 << 27))] {
        load_raw(&mut gate, address, raw);
        let words = unpack_aau_words(raw);
        oracle.load_words(&[words.0, words.1], address).unwrap();
    }
    for _ in 0..program.len() {
        gate.step().unwrap();
        oracle.step().unwrap();
        assert_aau_matches(&gate, &oracle);
    }
}

#[test]
fn signed_floating_and_exponent_alert_edges_match_the_oracle() {
    let cases = [
        (float_raw(0, -(1 << 29)), float_raw(0, 1 << 29)),
        (float_raw(255, 1 << 29), float_raw(255, 1 << 29)),
        (float_raw(-200, 1 << 29), float_raw(-100, 1 << 29)),
    ];
    for (left, right) in cases {
        let program = [
            assemble_aau_general("SET_NFLPOINT").unwrap(),
            assemble_aau_memory("FLD", 0o400, 0).unwrap(),
            assemble_aau_memory("FAD", 0o402, 0).unwrap(),
        ];
        let mut gate = machine(&program);
        let mut oracle = functional(&program);
        for (address, raw) in [(0o400, left), (0o402, right)] {
            load_raw(&mut gate, address, raw);
            let words = unpack_aau_words(raw);
            oracle.load_words(&[words.0, words.1], address).unwrap();
        }
        gate.run(program.len()).unwrap();
        oracle.run(program.len()).unwrap();
        assert_aau_matches(&gate, &oracle);
    }
}

#[test]
fn unnormalized_mode_and_multiply_underflow_match_the_oracle() {
    let eighth = float_raw(0, 1 << 27);
    let program = [
        assemble_aau_general("SET_UFLPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FAD", 0o402, 0).unwrap(),
        assemble_aau_memory("FLD", 0o406, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_memory("FMP", 0o404, 0).unwrap(),
    ];
    let mut gate = machine(&program);
    let mut oracle = functional(&program);
    for (address, raw) in [
        (0o400, eighth),
        (0o402, eighth),
        (0o404, float_raw(-100, 1 << 29)),
        (0o406, float_raw(-200, 1 << 29)),
    ] {
        load_raw(&mut gate, address, raw);
        let words = unpack_aau_words(raw);
        oracle.load_words(&[words.0, words.1], address).unwrap();
    }
    for _ in 0..program.len() {
        gate.step().unwrap();
        oracle.step().unwrap();
        assert_aau_matches(&gate, &oracle);
    }
    assert_eq!(
        gate.get_state().aau.mode,
        Some(AauMode::UnnormalizedFloatingPoint)
    );
}

#[test]
fn divide_zero_quotient_and_invalid_divisor_match_the_oracle() {
    let program = [
        assemble_aau_general("SET_NFLPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_memory("FLD", 0o402, 0).unwrap(),
        assemble_aau_memory("FDV", 0o404, 0).unwrap(),
    ];
    let mut gate = machine(&program);
    let mut oracle = functional(&program);
    for (address, raw) in [
        (0o400, float_raw(0, 1)),
        (0o402, 0),
        (0o404, float_raw(0, 1 << 29)),
    ] {
        load_raw(&mut gate, address, raw);
        let words = unpack_aau_words(raw);
        oracle.load_words(&[words.0, words.1], address).unwrap();
    }
    gate.run(program.len()).unwrap();
    oracle.run(program.len()).unwrap();
    assert_aau_matches(&gate, &oracle);
    assert_eq!(float_parts(gate.get_state().aau.qx), (-30, 1));
}

#[test]
fn preflight_failures_leave_every_gate_unchanged() {
    let mut no_mode = machine(&[assemble_aau_memory("FAD", 0o400, 0).unwrap()]);
    let before = no_mode.get_state();
    assert!(no_mode.step().is_err());
    assert_eq!(no_mode.get_state(), before);

    let mut reserved = machine(&[assemble_aau_memory("FLD", 0o17, 0).unwrap()]);
    let before = reserved.get_state();
    assert!(reserved.step().is_err());
    assert_eq!(reserved.get_state(), before);

    let mut not_ready = machine(&[assemble_aau_general("SET_FIXPOINT").unwrap()]);
    not_ready.set_aau_ready(false);
    let before = not_ready.get_state();
    assert!(not_ready.step().is_err());
    assert_eq!(not_ready.get_state(), before);

    let mut boundary = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    boundary
        .load_words(
            &[assemble_aau_memory("FLD", 4095, 1).unwrap()],
            PROGRAM as usize,
        )
        .unwrap();
    boundary.write_word(1, 1).unwrap();
    boundary.set_program_counter(PROGRAM).unwrap();
    let before = boundary.get_state();
    assert!(boundary.step().is_err());
    assert_eq!(boundary.get_state(), before);
}

#[test]
fn status_skip_boundary_and_not_ready_branches_are_atomic_and_visible() {
    let mut boundary = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    boundary
        .write_word(4094, assemble_aau_branch("BAN").unwrap())
        .unwrap();
    boundary.set_program_counter(4094).unwrap();
    let before = boundary.get_state();
    assert!(boundary.step().is_err());
    assert_eq!(boundary.get_state(), before);

    let program = [
        assemble_aau_branch("BAR").unwrap(),
        assemble_aau_branch("BAN").unwrap(),
        assemble_aau_branch("BAN").unwrap(),
    ];
    let mut gate = machine(&program);
    let mut oracle = functional(&program);
    gate.set_aau_ready(false);
    oracle.set_aau_ready(false);
    gate.step().unwrap();
    oracle.step().unwrap();
    assert_eq!(gate.get_state().pc, oracle.get_state().pc);
    assert_aau_matches(&gate, &oracle);
    gate.step().unwrap();
    oracle.step().unwrap();
    assert_eq!(gate.get_state().pc, oracle.get_state().pc);
    assert_aau_matches(&gate, &oracle);
}

#[test]
fn invalid_fixed_divide_preserves_the_product_and_matches_the_oracle() {
    let program = [
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_memory("FMP", 0o402, 0).unwrap(),
        assemble_aau_memory("FDV", 0o404, 0).unwrap(),
    ];
    let minus_six = fixed_words(-6);
    let seven = fixed_words(7);
    let one = fixed_words(1);
    let mut gate = machine(&program);
    let mut oracle = functional(&program);
    for (address, words) in [(0o400, minus_six), (0o402, seven), (0o404, one)] {
        gate.load_words(&[words.0, words.1], address as usize)
            .unwrap();
        oracle.load_words(&[words.0, words.1], address).unwrap();
    }
    gate.run(program.len()).unwrap();
    oracle.run(program.len()).unwrap();
    assert_aau_matches(&gate, &oracle);
    assert!(gate.get_state().aau.overflow);
}

#[test]
fn most_negative_fixed_divisor_uses_the_widened_remainder_wire() {
    let program = [
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_general("LQA").unwrap(),
        assemble_aau_memory("FMP", 0o402, 0).unwrap(),
        assemble_aau_memory("FDV", 0o402, 0).unwrap(),
    ];
    let one = fixed_words(1);
    let minimum = fixed_words(-(1_i64 << 38));
    let mut gate = machine(&program);
    let mut oracle = functional(&program);
    gate.load_words(&[one.0, one.1], 0o400).unwrap();
    gate.load_words(&[minimum.0, minimum.1], 0o402).unwrap();
    oracle.load_words(&[one.0, one.1], 0o400).unwrap();
    oracle.load_words(&[minimum.0, minimum.1], 0o402).unwrap();
    gate.run(program.len()).unwrap();
    oracle.run(program.len()).unwrap();
    assert_aau_matches(&gate, &oracle);
    assert_eq!(fixed_value(gate.get_state().aau.ax), 1);
    assert_eq!(fixed_value(gate.get_state().aau.qx), 0);
}

#[test]
fn every_status_branch_and_hold_clear_rule_matches_the_oracle() {
    let minimum = fixed_words(-(1_i64 << 38));
    let minus_one = fixed_words(-1);
    let program = [
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FAD", 0o402, 0).unwrap(),
        assemble_aau_branch("BUF").unwrap(),
        assemble_aau_branch("BUO").unwrap(),
        assemble_aau_branch("BUN").unwrap(),
    ];
    let mut gate = machine(&program);
    let mut oracle = functional(&program);
    gate.load_words(&[minimum.0, minimum.1], 0o400).unwrap();
    gate.load_words(&[minus_one.0, minus_one.1], 0o402).unwrap();
    oracle.load_words(&[minimum.0, minimum.1], 0o400).unwrap();
    oracle
        .load_words(&[minus_one.0, minus_one.1], 0o402)
        .unwrap();
    for _ in 0..program.len() {
        gate.step().unwrap();
        oracle.step().unwrap();
        assert_aau_matches(&gate, &oracle);
        assert_eq!(gate.get_state().pc, oracle.get_state().pc);
    }

    let clear_program: Vec<_> = ["BZE", "BNO", "BNU", "BON", "BUN", "BNE"]
        .into_iter()
        .map(|name| assemble_aau_branch(name).unwrap())
        .collect();
    let mut clear = machine(&clear_program);
    clear.run(clear_program.len()).unwrap();
    assert_eq!(clear.get_state().pc, PROGRAM + clear_program.len() as i32);
}

#[test]
fn reset_and_explicit_alert_clear_cover_all_aau_latches() {
    let maximum = fixed_words((1_i64 << 38) - 1);
    let one = fixed_words(1);
    let program = [
        assemble_aau_general("SET_FIXPOINT").unwrap(),
        assemble_aau_memory("FLD", 0o400, 0).unwrap(),
        assemble_aau_memory("FAD", 0o402, 0).unwrap(),
    ];
    let mut gate = machine(&program);
    gate.load_words(&[maximum.0, maximum.1], 0o400).unwrap();
    gate.load_words(&[one.0, one.1], 0o402).unwrap();
    gate.run(program.len()).unwrap();
    assert!(gate.get_state().aau.overflow_hold);
    gate.clear_aau_alerts();
    assert!(!gate.get_state().aau.overflow_hold);
    gate.reset();
    let aau = gate.get_state().aau;
    assert_eq!(aau.mode, None);
    assert!(aau.ready);
    assert_eq!((aau.ax, aau.bx, aau.qx, aau.ix), (0, 0, 0, 0));
    assert!(!aau.overflow && !aau.underflow && !aau.overflow_hold && !aau.underflow_hold);
}
