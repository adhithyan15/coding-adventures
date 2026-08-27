use ibm704_encoder::{encode_type_a, encode_type_b, pack_word, ADDR_MASK, SIGN_BIT, WORD_MASK};
use ibm704_simulator::*;

fn instruction(opcode: u16, tag: u8, address: u16) -> u64 {
    encode_type_b(opcode & 0x800 != 0, opcode & 0x1ff, tag, address)
}

fn program_bytes(words: &[u64]) -> Vec<u8> {
    words.iter().flat_map(|word| pack_word(*word)).collect()
}

fn simulator_with(words: &[u64]) -> IBM704Simulator {
    let mut simulator = IBM704Simulator::new();
    simulator.load_words(words, 0).unwrap();
    simulator
}

#[test]
fn lifecycle_is_bounded_strict_and_snapshot_owned() {
    let mut simulator = IBM704Simulator::with_memory_words(8).unwrap();
    assert_eq!(
        simulator.load(&[0; 4], 0),
        Err(IBM704Error::InvalidProgram(
            ibm704_encoder::DecodeError::InvalidLength(4)
        ))
    );
    assert!(matches!(
        simulator.load(&[0xf0, 0, 0, 0, 0], 0),
        Err(IBM704Error::InvalidProgram(
            ibm704_encoder::DecodeError::ReservedNibble(0xf0)
        ))
    ));
    assert!(matches!(
        simulator.load_words(&[0; 9], 0),
        Err(IBM704Error::ProgramTooLarge { .. })
    ));
    assert_eq!(
        simulator.load_words(&[], usize::MAX),
        Err(IBM704Error::InvalidOrigin { origin: usize::MAX })
    );
    assert_eq!(simulator.load_words(&[], 8), Ok(0));
    assert!(matches!(
        IBM704Simulator::with_memory_words(0),
        Err(IBM704Error::InvalidMemorySize { .. })
    ));

    simulator
        .load_words(&[instruction(OP_NOP, 0, 0)], 0)
        .unwrap();
    assert_eq!(
        simulator.run(1),
        Err(IBM704Error::MaxStepsExceeded { max_steps: 1 })
    );

    simulator.reset();
    simulator
        .load_words(&[instruction(OP_HTR, 0, 3)], 0)
        .unwrap();
    let result = simulator.run(1).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 1);
    assert_eq!(result.final_state.pc, 3);
    assert_eq!(simulator.step(), Err(IBM704Error::Halted));
    let snapshot = simulator.get_state();
    simulator.reset();
    assert!(snapshot.halted);
    assert!(!simulator.get_state().halted);
}

#[test]
fn load_store_exchange_and_negative_zero_match_the_machine() {
    let words = [
        instruction(OP_CLA, 0, 100),
        instruction(OP_STO, 0, 101),
        instruction(OP_LDQ, 0, 102),
        instruction(OP_STQ, 0, 103),
        instruction(OP_XCA, 0, 0),
        instruction(OP_CAL, 0, 104),
        instruction(OP_STZ, 0, 105),
        instruction(OP_HTR, 0, 7),
    ];
    let mut simulator = simulator_with(&words);
    simulator.write_word(100, make_word(true, 42)).unwrap();
    simulator.write_word(102, make_word(false, 9)).unwrap();
    simulator.write_word(104, make_word(true, 7)).unwrap();
    simulator.write_word(105, WORD_MASK).unwrap();
    let result = simulator.run(20).unwrap();
    assert_eq!(result.final_state.memory[101], make_word(true, 42));
    assert_eq!(result.final_state.memory[103], make_word(false, 9));
    assert_eq!(result.final_state.memory[105], 0);
    assert!(!result.final_state.accumulator_sign);
    assert!(result.final_state.accumulator_p);
    assert!(!result.final_state.accumulator_q);
    assert_eq!(result.final_state.accumulator_qp, 1);
    assert_eq!(result.final_state.accumulator_magnitude, 7);
    assert_eq!(result.final_state.mq, make_word(true, 42));

    assert_eq!(make_word(true, 0), SIGN_BIT);
    assert!(word_sign(make_word(true, 0)));
    assert_eq!(word_magnitude(make_word(true, 0)), 0);
    assert_eq!(add_sign_magnitude(false, 0, true, 0), (false, 0, false));
}

#[test]
fn integer_arithmetic_overflow_multiply_and_divide_work() {
    let mut arithmetic = simulator_with(&[
        instruction(OP_CLA, 0, 100),
        instruction(OP_ADD, 0, 101),
        instruction(OP_SUB, 0, 102),
        instruction(OP_ADM, 0, 103),
        instruction(OP_HTR, 0, 4),
    ]);
    arithmetic.write_word(100, make_word(false, 3)).unwrap();
    arithmetic.write_word(101, make_word(false, 4)).unwrap();
    arithmetic.write_word(102, make_word(false, 10)).unwrap();
    arithmetic.write_word(103, make_word(true, 2)).unwrap();
    let state = arithmetic.run(10).unwrap().final_state;
    assert!(state.accumulator_sign);
    assert_eq!(state.accumulator_magnitude, 1);

    let mut overflow = simulator_with(&[
        instruction(OP_CLA, 0, 100),
        instruction(OP_ADD, 0, 101),
        instruction(OP_TOV, 0, 4),
        instruction(OP_HTR, 0, 3),
        instruction(OP_HTR, 0, 4),
    ]);
    overflow.write_word(100, MAGNITUDE_MASK).unwrap();
    overflow.write_word(101, 1).unwrap();
    let state = overflow.run(10).unwrap().final_state;
    assert_eq!(state.pc, 4);
    assert!(state.accumulator_p);
    assert!(!state.overflow_trigger);

    let mut multiply = simulator_with(&[
        instruction(OP_LDQ, 0, 100),
        instruction(OP_MPY, 0, 101),
        instruction(OP_DVP, 0, 102),
        instruction(OP_HTR, 0, 3),
    ]);
    multiply.write_word(100, make_word(false, 6)).unwrap();
    multiply.write_word(101, make_word(true, 7)).unwrap();
    multiply.write_word(102, make_word(true, 3)).unwrap();
    let state = multiply.run(10).unwrap().final_state;
    assert_eq!(state.mq, make_word(false, 14));
    assert_eq!(state.accumulator_magnitude, 0);

    let mut checks = simulator_with(&[instruction(OP_DVP, 0, 100), instruction(OP_DVH, 0, 100)]);
    assert_eq!(checks.step().unwrap().pc_after, 1);
    assert!(checks.get_state().divide_check_trigger);
    checks.step().unwrap();
    assert!(checks.get_state().halted);
    assert_eq!(checks.get_state().pc, 2);

    assert_eq!(
        add_sign_magnitude(false, u64::MAX, false, 1),
        (false, 0, true),
        "public helpers mask operands instead of overflowing"
    );

    let mut no_overflow = simulator_with(&[
        instruction(OP_CLA, 0, 100),
        instruction(OP_ADD, 0, 101),
        instruction(OP_TNO, 0, 4),
        instruction(OP_HTR, 0, 3),
    ]);
    no_overflow.write_word(100, MAGNITUDE_MASK).unwrap();
    no_overflow.write_word(101, 1).unwrap();
    let state = no_overflow.run(10).unwrap().final_state;
    assert_eq!(state.pc, 3);
    assert!(!state.overflow_trigger);
}

#[test]
fn transfers_observe_sign_zero_and_triggers() {
    let mut simulator = simulator_with(&[
        instruction(OP_CLA, 0, 100),
        instruction(OP_TZE, 0, 4),
        instruction(OP_HTR, 0, 2),
        instruction(OP_NOP, 0, 0),
        instruction(OP_CLA, 0, 101),
        instruction(OP_TNZ, 0, 7),
        instruction(OP_HTR, 0, 6),
        instruction(OP_TPL, 0, 9),
        instruction(OP_TMI, 0, 10),
        instruction(OP_HTR, 0, 9),
        instruction(OP_TNO, 0, 12),
        instruction(OP_HTR, 0, 11),
        instruction(OP_TRA, 0, 13),
        instruction(OP_HTR, 0, 13),
    ]);
    simulator.write_word(100, make_word(true, 0)).unwrap();
    simulator.write_word(101, make_word(true, 1)).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "CLA 100");
    assert_eq!(simulator.step().unwrap().pc_after, 4);
    simulator.step().unwrap();
    assert_eq!(simulator.step().unwrap().pc_after, 7);
    assert_eq!(simulator.step().unwrap().pc_after, 8);
    assert_eq!(simulator.step().unwrap().pc_after, 10);
    assert_eq!(simulator.step().unwrap().pc_after, 12);
    assert_eq!(simulator.step().unwrap().pc_after, 13);
    simulator.step().unwrap();
    assert!(simulator.get_state().halted);
}

#[test]
fn index_family_and_effective_address_follow_or_and_subtract_rules() {
    let type_a = encode_type_a(PREFIX_TXI, 5, 1, 8).unwrap();
    let mut simulator = simulator_with(&[
        instruction(OP_LXA, 1, 100),
        instruction(OP_LXD, 2, 101),
        instruction(OP_CLA, 3, 20),
        instruction(OP_SXA, 1, 102),
        instruction(OP_SXD, 2, 103),
        instruction(OP_PAX, 4, 0),
        instruction(OP_PDX, 1, 0),
        instruction(OP_PXA, 2, 0),
        type_a,
    ]);
    simulator.write_word(100, 5).unwrap();
    simulator.write_word(101, (3 << 18) | 99).unwrap();
    simulator.write_word(13, (9 << 18) | 1234).unwrap(); // 20 - (5 | 3)
    simulator.write_word(102, 11 << 18).unwrap();
    simulator.write_word(103, 444).unwrap();
    for _ in 0..8 {
        simulator.step().unwrap();
    }
    let state = simulator.get_state();
    assert_eq!(state.memory[102] & ADDR_MASK, 5);
    assert_eq!((state.memory[103] >> 18) & ADDR_MASK, 3);
    assert_eq!(state.index_c, 1234);
    assert_eq!(state.index_a, 9);
    assert_eq!(state.accumulator_magnitude, 3);
    simulator.step().unwrap();
    assert_eq!(simulator.get_state().index_a, 14);
    assert_eq!(simulator.get_state().pc, 8);

    for (prefix, ir, decrement, expected_pc, expected_ir) in [
        (PREFIX_TIX, 10, 3, 7, 7),
        (PREFIX_TXH, 10, 3, 7, 10),
        (PREFIX_TXL, 3, 3, 7, 3),
        (PREFIX_TNX, 3, 3, 7, 3),
        (PREFIX_TNX, 10, 3, 2, 7),
    ] {
        let mut machine = simulator_with(&[
            instruction(OP_LXA, 1, 10),
            encode_type_a(prefix, decrement, 1, 7).unwrap(),
        ]);
        machine.write_word(10, ir).unwrap();
        machine.step().unwrap();
        machine.step().unwrap();
        assert_eq!(machine.get_state().pc, expected_pc);
        assert_eq!(machine.get_state().index_a, expected_ir);
    }

    for tag in 0..=7 {
        let mut machine = simulator_with(&[
            instruction(OP_LXA, 1, 100),
            instruction(OP_LXA, 2, 101),
            instruction(OP_LXA, 4, 102),
            instruction(OP_CLA, tag, 20),
        ]);
        machine.write_word(100, 1).unwrap();
        machine.write_word(101, 2).unwrap();
        machine.write_word(102, 4).unwrap();
        machine.write_word(20 - tag as usize, tag as u64).unwrap();
        for _ in 0..4 {
            machine.step().unwrap();
        }
        assert_eq!(machine.get_state().accumulator_magnitude, tag as u64);
    }
}

#[test]
fn floating_point_round_trips_and_instructions_execute() {
    for value in [1.0, 2.0, 0.5, -1.5, 3.25] {
        assert_eq!(fp_to_float(float_to_fp(value)), value);
    }
    assert_eq!(float_to_fp(f64::INFINITY), 0);
    assert_eq!(float_to_fp(f64::NAN), 0);

    let mut simulator = simulator_with(&[
        instruction(OP_CLA, 0, 100),
        instruction(OP_FAD, 0, 101),
        instruction(OP_FSB, 0, 102),
        instruction(OP_LDQ, 0, 103),
        instruction(OP_FMP, 0, 104),
        instruction(OP_FDP, 0, 105),
        instruction(OP_FDP, 0, 106),
        instruction(OP_HTR, 0, 7),
    ]);
    simulator.write_word(100, float_to_fp(1.5)).unwrap();
    simulator.write_word(101, float_to_fp(2.5)).unwrap();
    simulator.write_word(102, float_to_fp(1.0)).unwrap();
    simulator.write_word(103, float_to_fp(3.0)).unwrap();
    simulator.write_word(104, float_to_fp(4.0)).unwrap();
    simulator.write_word(105, float_to_fp(2.0)).unwrap();
    simulator.write_word(106, 0).unwrap();
    let state = simulator.run(20).unwrap().final_state;
    assert_eq!(fp_to_float(state.mq), 6.0);
    assert!(state.divide_check_trigger);

    let mut halt = simulator_with(&[instruction(OP_FDH, 0, 100)]);
    halt.step().unwrap();
    assert!(halt.get_state().halted);
    assert!(halt.get_state().divide_check_trigger);
    assert_eq!(halt.get_state().pc, 1);
    assert_eq!(OP_FDH, 0o240);
    assert_eq!(OP_FDP, 0o241);
}

#[test]
fn unknown_opcode_and_out_of_range_access_fail_closed() {
    let mut unknown = simulator_with(&[instruction(0x0f0, 0, 0)]);
    assert_eq!(
        unknown.step(),
        Err(IBM704Error::UnknownOpcode {
            opcode: 0x0f0,
            pc: 0
        })
    );
    assert!(unknown.get_state().halted);

    let mut unknown_type_a = simulator_with(&[encode_type_a(0b111, 0, 0, 0).unwrap()]);
    assert_eq!(
        unknown_type_a.step(),
        Err(IBM704Error::UnknownTypeAPrefix {
            prefix: 0b111,
            pc: 0
        })
    );
    assert!(unknown_type_a.get_state().halted);

    let mut small = IBM704Simulator::with_memory_words(4).unwrap();
    small.load_words(&[instruction(OP_CLA, 0, 7)], 0).unwrap();
    assert_eq!(
        small.step(),
        Err(IBM704Error::AddressOutOfRange {
            address: 7,
            capacity: 4
        })
    );
    assert!(small.get_state().halted);
}

#[test]
fn execute_accepts_canonical_transport() {
    let words = [instruction(OP_NOP, 0, 0), instruction(OP_HPR, 0, 7)];
    let mut simulator = IBM704Simulator::new();
    let result = simulator.execute(&program_bytes(&words), 10).unwrap();
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces[0].mnemonic, "NOP");
    assert_eq!(result.traces[1].mnemonic, "HPR");
    assert_eq!(result.final_state.pc, 2);
}

#[test]
fn fortran_style_sum_and_factorial_programs_run_end_to_end() {
    let mut sum = simulator_with(&[
        instruction(OP_CLA, 0, 100),
        instruction(OP_PAX, 1, 0),
        instruction(OP_STZ, 0, 101),
        instruction(OP_PXA, 1, 0),
        instruction(OP_STO, 0, 102),
        instruction(OP_CLA, 0, 101),
        instruction(OP_ADD, 0, 102),
        instruction(OP_STO, 0, 101),
        encode_type_a(PREFIX_TIX, 1, 1, 3).unwrap(),
        instruction(OP_HTR, 0, 9),
    ]);
    sum.write_word(100, 5).unwrap();
    assert_eq!(sum.run(100).unwrap().final_state.memory[101], 15);

    let mut factorial = simulator_with(&[
        instruction(OP_CLA, 0, 100),
        instruction(OP_PAX, 1, 0),
        instruction(OP_CLA, 0, 103),
        instruction(OP_STO, 0, 101),
        instruction(OP_PXA, 1, 0),
        instruction(OP_STO, 0, 102),
        instruction(OP_LDQ, 0, 101),
        instruction(OP_MPY, 0, 102),
        instruction(OP_STQ, 0, 101),
        encode_type_a(PREFIX_TIX, 1, 1, 4).unwrap(),
        instruction(OP_HTR, 0, 10),
    ]);
    factorial.write_word(100, 5).unwrap();
    factorial.write_word(103, 1).unwrap();
    assert_eq!(factorial.run(100).unwrap().final_state.memory[101], 120);
}

#[test]
fn lisp_cons_fields_and_floating_polynomial_run_end_to_end() {
    let cell = (99 << 18) | 42;
    let mut lisp = simulator_with(&[
        instruction(OP_CLA, 0, 100),
        instruction(OP_PAX, 1, 0),
        instruction(OP_PDX, 2, 0),
        instruction(OP_PXA, 1, 0),
        instruction(OP_STO, 0, 101),
        instruction(OP_PXA, 2, 0),
        instruction(OP_STO, 0, 102),
        instruction(OP_HTR, 0, 7),
    ]);
    lisp.write_word(100, cell).unwrap();
    let state = lisp.run(20).unwrap().final_state;
    assert_eq!(state.memory[101], 42);
    assert_eq!(state.memory[102], 99);

    let mut polynomial = simulator_with(&[
        instruction(OP_LDQ, 0, 100),
        instruction(OP_FMP, 0, 101),
        instruction(OP_FAD, 0, 102),
        instruction(OP_STO, 0, 103),
        instruction(OP_HTR, 0, 4),
    ]);
    polynomial.write_word(100, float_to_fp(2.0)).unwrap();
    polynomial.write_word(101, float_to_fp(3.0)).unwrap();
    polynomial.write_word(102, float_to_fp(1.0)).unwrap();
    let state = polynomial.run(20).unwrap().final_state;
    assert_eq!(fp_to_float(state.memory[103]), 7.0);
}

#[test]
fn mq_transfers_cover_plus_and_clear_overflow_path() {
    let mut simulator = simulator_with(&[
        instruction(OP_LDQ, 0, 100),
        instruction(OP_TQP, 0, 3),
        instruction(OP_HTR, 0, 2),
        instruction(OP_TQO, 0, 5),
        instruction(OP_HTR, 0, 4),
        instruction(OP_HTR, 0, 5),
    ]);
    simulator.write_word(100, 1).unwrap();
    assert_eq!(simulator.step().unwrap().pc_after, 1);
    assert_eq!(simulator.step().unwrap().pc_after, 3);
    assert_eq!(simulator.step().unwrap().pc_after, 4);
    simulator.step().unwrap();
    assert_eq!(simulator.get_state().pc, 4);
}
