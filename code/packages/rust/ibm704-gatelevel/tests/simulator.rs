use ibm704_encoder::{encode_type_a, encode_type_b, pack_word, ADDR_MASK, SIGN_BIT, WORD_MASK};
use ibm704_gatelevel::*;
use ibm704_simulator::IBM704Simulator as FunctionalIBM704;

const TEST_MEMORY_WORDS: usize = 256;

fn instruction(opcode: u16, tag: u8, address: u16) -> u64 {
    encode_type_b(opcode & 0x800 != 0, opcode & 0x1ff, tag, address)
}

fn program_bytes(words: &[u64]) -> Vec<u8> {
    words.iter().flat_map(|word| pack_word(*word)).collect()
}

fn assert_states_equal(gate: &IBM704State, functional: &ibm704_simulator::IBM704State) {
    assert_eq!(gate.accumulator_sign, functional.accumulator_sign);
    assert_eq!(gate.accumulator_qp, functional.accumulator_qp);
    assert_eq!(gate.accumulator_p, functional.accumulator_p);
    assert_eq!(gate.accumulator_q, functional.accumulator_q);
    assert_eq!(gate.accumulator_magnitude, functional.accumulator_magnitude);
    assert_eq!(gate.mq, functional.mq);
    assert_eq!(gate.mq_sign, functional.mq_sign);
    assert_eq!(gate.mq_magnitude, functional.mq_magnitude);
    assert_eq!(gate.index_a, functional.index_a);
    assert_eq!(gate.index_b, functional.index_b);
    assert_eq!(gate.index_c, functional.index_c);
    assert_eq!(gate.pc, functional.pc);
    assert_eq!(gate.halted, functional.halted);
    assert_eq!(gate.overflow_trigger, functional.overflow_trigger);
    assert_eq!(gate.divide_check_trigger, functional.divide_check_trigger);
    assert_eq!(gate.memory, functional.memory);
}

fn lockstep(
    words: &[u64],
    data: &[(usize, u64)],
    max_steps: usize,
) -> (IBM704State, ibm704_simulator::IBM704State) {
    let mut gate = IBM704GateLevel::with_memory_words(TEST_MEMORY_WORDS).unwrap();
    let mut functional = FunctionalIBM704::with_memory_words(TEST_MEMORY_WORDS).unwrap();
    gate.load_words(words, 0).unwrap();
    functional.load_words(words, 0).unwrap();
    for &(address, word) in data {
        gate.write_word(address, word).unwrap();
        functional.write_word(address, word).unwrap();
    }

    for _ in 0..max_steps {
        assert_states_equal(&gate.get_state(), &functional.get_state());
        if gate.get_state().halted {
            return (gate.get_state(), functional.get_state());
        }
        let gate_trace = gate.step().unwrap();
        let functional_trace = functional.step().unwrap();
        assert_eq!(gate_trace.pc_before, functional_trace.pc_before);
        assert_eq!(gate_trace.pc_after, functional_trace.pc_after);
        assert_eq!(gate_trace.instruction, functional_trace.instruction);
        assert_eq!(gate_trace.mnemonic, functional_trace.mnemonic);
        assert_eq!(gate_trace.description, functional_trace.description);
    }
    panic!("lockstep program did not halt within {max_steps} steps")
}

fn lockstep_small(words: &[u64], data: &[(usize, u64)], max_steps: usize) {
    let mut gate = IBM704GateLevel::with_memory_words(8).unwrap();
    let mut functional = FunctionalIBM704::with_memory_words(8).unwrap();
    gate.load_words(words, 0).unwrap();
    functional.load_words(words, 0).unwrap();
    for &(address, word) in data {
        gate.write_word(address, word).unwrap();
        functional.write_word(address, word).unwrap();
    }

    for _ in 0..max_steps {
        assert_states_equal(&gate.get_state(), &functional.get_state());
        if gate.get_state().halted {
            return;
        }
        let gate_trace = gate.step().unwrap();
        let functional_trace = functional.step().unwrap();
        assert_eq!(gate_trace.pc_before, functional_trace.pc_before);
        assert_eq!(gate_trace.pc_after, functional_trace.pc_after);
        assert_eq!(gate_trace.instruction, functional_trace.instruction);
        assert_eq!(gate_trace.mnemonic, functional_trace.mnemonic);
        assert_eq!(gate_trace.description, functional_trace.description);
        assert_states_equal(&gate.get_state(), &functional.get_state());
    }
    panic!("small lockstep program did not halt within {max_steps} steps")
}

#[test]
fn construction_has_exact_flip_flop_topology_and_owned_state() {
    let simulator = IBM704GateLevel::new();
    let state = simulator.get_state();
    assert_eq!(IBM704GateLevel::default(), simulator);
    assert_eq!(state.memory, vec![0; MEMORY_WORDS]);
    assert_eq!(state.pc, 0);
    assert!(!state.halted);
    assert_eq!(simulator.flip_flop_count(), FLIP_FLOP_COUNT);
    assert_eq!(FLIP_FLOP_COUNT, 1_179_786);
    assert_eq!(simulator.gate_count(), ESTIMATED_GATE_COUNT);

    let small = IBM704GateLevel::with_memory_words(8).unwrap();
    assert_eq!(small.flip_flop_count(), 8 * 36 + 38 + 36 + 45 + 15 + 4);
}

#[test]
fn lifecycle_transport_bounds_and_errors_are_strict() {
    let mut simulator = IBM704GateLevel::with_memory_words(8).unwrap();
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
        IBM704GateLevel::with_memory_words(0),
        Err(IBM704Error::InvalidMemorySize { .. })
    ));
    assert_eq!(
        simulator.write_word(8, 1),
        Err(IBM704Error::AddressOutOfRange {
            address: 8,
            capacity: 8
        })
    );
    assert_eq!(
        simulator.run(0),
        Err(IBM704Error::MaxStepsExceeded { max_steps: 0 })
    );

    let words = [instruction(OP_NOP, 0, 0), instruction(OP_HPR, 0, 0)];
    let result = simulator.execute(&program_bytes(&words), 4).unwrap();
    assert_eq!(result.steps, 2);
    assert_eq!(result.final_state.pc, 2);
    assert_eq!(simulator.step(), Err(IBM704Error::Halted));
}

#[test]
fn load_store_exchange_and_ac_qp_paths_match_functional() {
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
    let data = [
        (100, make_word(true, 42)),
        (102, make_word(false, 9)),
        (104, make_word(true, 7)),
        (105, WORD_MASK),
    ];
    let (gate, _) = lockstep(&words, &data, 20);
    assert_eq!(gate.memory[101], make_word(true, 42));
    assert_eq!(gate.memory[103], make_word(false, 9));
    assert_eq!(gate.memory[105], 0);
    assert!(!gate.accumulator_sign);
    assert!(gate.accumulator_p);
    assert_eq!(gate.accumulator_magnitude, 7);
    assert_eq!(gate.mq, make_word(true, 42));
}

#[test]
fn integer_adder_multiply_divide_and_trigger_networks_match_functional() {
    let words = [
        instruction(OP_CLA, 0, 100),
        instruction(OP_ADD, 0, 101),
        instruction(OP_SUB, 0, 102),
        instruction(OP_ADM, 0, 103),
        instruction(OP_LDQ, 0, 104),
        instruction(OP_MPY, 0, 105),
        instruction(OP_DVP, 0, 106),
        instruction(OP_TOV, 0, 9),
        instruction(OP_HTR, 0, 8),
        instruction(OP_TNO, 0, 11),
        instruction(OP_HTR, 0, 10),
        instruction(OP_HTR, 0, 11),
    ];
    let data = [
        (100, MAGNITUDE_MASK),
        (101, 1),
        (102, make_word(false, 2)),
        (103, make_word(true, 3)),
        (104, make_word(false, 6)),
        (105, make_word(true, 7)),
        (106, make_word(true, 3)),
    ];
    let (gate, _) = lockstep(&words, &data, 30);
    assert_eq!(gate.pc, 11);
    assert_eq!(gate.mq, make_word(false, 14));
    assert_eq!(gate.accumulator_magnitude, 0);
    assert!(!gate.overflow_trigger);

    let check_words = [instruction(OP_DVP, 0, 100), instruction(OP_DVH, 0, 100)];
    let (gate, _) = lockstep(&check_words, &[], 3);
    assert!(gate.divide_check_trigger);
    assert!(gate.halted);
    assert_eq!(gate.pc, 2);
}

#[test]
fn conditional_and_unconditional_transfers_match_functional() {
    let words = [
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
        instruction(OP_TQO, 0, 12),
        instruction(OP_TQP, 0, 13),
        instruction(OP_HTR, 0, 12),
        instruction(OP_TRA, 0, 14),
        instruction(OP_HTR, 0, 14),
    ];
    let data = [(100, make_word(true, 0)), (101, make_word(true, 1))];
    let (gate, _) = lockstep(&words, &data, 30);
    assert!(gate.halted);
    assert_eq!(gate.pc, 14);
}

#[test]
fn all_index_tags_and_type_a_instructions_match_functional() {
    for tag in 0..=7 {
        let words = [
            instruction(OP_LXA, 1, 100),
            instruction(OP_LXA, 2, 101),
            instruction(OP_LXA, 4, 102),
            instruction(OP_CLA, tag, 20),
            instruction(OP_HTR, 0, 4),
        ];
        let data = [
            (100, 1),
            (101, 2),
            (102, 4),
            (20 - tag as usize, tag as u64),
        ];
        let (gate, _) = lockstep(&words, &data, 10);
        assert_eq!(gate.accumulator_magnitude, tag as u64);
    }

    for (prefix, ir, decrement, expected_pc, expected_ir) in [
        (PREFIX_TXI, 10, 3, 7, 13),
        (PREFIX_TIX, 10, 3, 7, 7),
        (PREFIX_TXH, 10, 3, 7, 10),
        (PREFIX_TXH, 3, 3, 2, 3),
        (PREFIX_TXL, 3, 3, 7, 3),
        (PREFIX_TXL, 10, 3, 2, 10),
        (PREFIX_TNX, 3, 3, 7, 3),
        (PREFIX_TNX, 10, 3, 2, 7),
        (PREFIX_TIX, 3, 3, 2, 3),
    ] {
        let words = [
            instruction(OP_LXA, 1, 100),
            encode_type_a(prefix, decrement, 1, 7).unwrap(),
            instruction(OP_HTR, 0, 2),
            0,
            0,
            0,
            0,
            instruction(OP_HTR, 0, 7),
        ];
        let (gate, _) = lockstep(&words, &[(100, ir)], 5);
        assert_eq!(gate.pc, expected_pc);
        assert_eq!(gate.index_a, expected_ir);
    }
}

#[test]
fn index_load_store_and_ac_transfer_family_match_functional() {
    let words = [
        instruction(OP_LXA, 1, 100),
        instruction(OP_LXD, 2, 101),
        instruction(OP_CLA, 3, 20),
        instruction(OP_SXA, 1, 102),
        instruction(OP_SXD, 2, 103),
        instruction(OP_PAX, 4, 0),
        instruction(OP_PDX, 1, 0),
        instruction(OP_PXA, 2, 0),
        instruction(OP_HTR, 0, 8),
    ];
    let data = [
        (100, 5),
        (101, (3 << 18) | 99),
        (13, (9 << 18) | 1234),
        (102, 11 << 18),
        (103, 444),
    ];
    let (gate, _) = lockstep(&words, &data, 20);
    assert_eq!(gate.memory[102] & ADDR_MASK, 5);
    assert_eq!((gate.memory[103] >> 18) & ADDR_MASK, 3);
    assert_eq!(gate.index_c, 1234);
    assert_eq!(gate.index_a, 9);
    assert_eq!(gate.accumulator_magnitude, 3);
}

#[test]
fn floating_instruction_family_matches_functional() {
    for value in [1.0, 2.0, 0.5, -1.5, 3.25] {
        assert_eq!(fp_to_float(float_to_fp(value)), value);
    }
    let words = [
        instruction(OP_CLA, 0, 100),
        instruction(OP_FAD, 0, 101),
        instruction(OP_FSB, 0, 102),
        instruction(OP_LDQ, 0, 103),
        instruction(OP_FMP, 0, 104),
        instruction(OP_FDP, 0, 105),
        instruction(OP_FDP, 0, 106),
        instruction(OP_HTR, 0, 7),
    ];
    let data = [
        (100, float_to_fp(1.5)),
        (101, float_to_fp(2.5)),
        (102, float_to_fp(1.0)),
        (103, float_to_fp(3.0)),
        (104, float_to_fp(4.0)),
        (105, float_to_fp(2.0)),
        (106, 0),
    ];
    let (gate, _) = lockstep(&words, &data, 20);
    assert_eq!(fp_to_float(gate.mq), 6.0);
    assert!(gate.divide_check_trigger);

    let (halt, _) = lockstep(&[instruction(OP_FDH, 0, 100)], &[], 2);
    assert!(halt.halted);
    assert!(halt.divide_check_trigger);
    assert_eq!(OP_FDH, 0o240);
    assert_eq!(OP_FDP, 0o241);
}

#[test]
fn seeded_floating_instructions_match_full_functional_machine_state() {
    let edge_pairs = [
        (0, 0),
        (SIGN_BIT, 0),
        (make_word(false, 1), make_word(true, 1)),
        (
            make_word(false, (255 << 27) | ((1 << 27) - 1)),
            make_word(true, (255 << 27) | (1 << 26)),
        ),
        (
            make_word(false, (1 << 27) | 1),
            make_word(false, (254 << 27) | ((1 << 27) - 1)),
        ),
        (float_to_fp(1.5), float_to_fp(2.5)),
        (float_to_fp(-7.0), float_to_fp(2.0)),
        (float_to_fp(3.25), float_to_fp(-0.5)),
    ];
    let mut seed = 0x7040_f10a_71e5_1954_u64;
    let mut next_word = |iteration: usize| {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let characteristic = match iteration % 16 {
            0 => 0,
            1 => 255,
            _ => (seed >> 27) & 0xff,
        };
        let fraction = match iteration % 8 {
            0 => 0,
            1 => 1,
            _ => (seed & ((1 << 27) - 1)) | (1 << 26),
        };
        make_word(seed >> 63 != 0, (characteristic << 27) | fraction)
    };

    for iteration in 0..128 {
        let (left, right) = if iteration < edge_pairs.len() {
            edge_pairs[iteration]
        } else {
            (next_word(iteration), next_word(iteration + 1))
        };
        for opcode in [OP_FAD, OP_FSB, OP_FMP, OP_FDP] {
            let load = if opcode == OP_FMP { OP_LDQ } else { OP_CLA };
            let words = [
                instruction(load, 0, 6),
                instruction(opcode, 0, 7),
                instruction(OP_HTR, 0, 2),
            ];
            lockstep_small(&words, &[(6, left), (7, right)], 4);
        }
    }
}

#[test]
fn fortran_lisp_and_floating_programs_match_end_to_end() {
    let sum_words = [
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
    ];
    let (sum, _) = lockstep(&sum_words, &[(100, 5)], 100);
    assert_eq!(sum.memory[101], 15);

    let cell = (99 << 18) | 42;
    let lisp_words = [
        instruction(OP_CLA, 0, 100),
        instruction(OP_PAX, 1, 0),
        instruction(OP_PDX, 2, 0),
        instruction(OP_PXA, 1, 0),
        instruction(OP_STO, 0, 101),
        instruction(OP_PXA, 2, 0),
        instruction(OP_STO, 0, 102),
        instruction(OP_HTR, 0, 7),
    ];
    let (lisp, _) = lockstep(&lisp_words, &[(100, cell)], 20);
    assert_eq!(lisp.memory[101], 42);
    assert_eq!(lisp.memory[102], 99);

    let polynomial_words = [
        instruction(OP_LDQ, 0, 100),
        instruction(OP_FMP, 0, 101),
        instruction(OP_FAD, 0, 102),
        instruction(OP_STO, 0, 103),
        instruction(OP_HTR, 0, 4),
    ];
    let data = [
        (100, float_to_fp(2.0)),
        (101, float_to_fp(3.0)),
        (102, float_to_fp(1.0)),
    ];
    let (polynomial, _) = lockstep(&polynomial_words, &data, 20);
    assert_eq!(fp_to_float(polynomial.memory[103]), 7.0);
}

#[test]
fn unknown_decodes_and_bad_effective_addresses_fail_closed() {
    let mut unknown = IBM704GateLevel::with_memory_words(8).unwrap();
    unknown.load_words(&[instruction(0x0f0, 0, 0)], 0).unwrap();
    assert_eq!(
        unknown.step(),
        Err(IBM704Error::UnknownOpcode {
            opcode: 0x0f0,
            pc: 0
        })
    );
    assert!(unknown.get_state().halted);

    let mut type_a = IBM704GateLevel::with_memory_words(8).unwrap();
    type_a
        .load_words(&[encode_type_a(0b111, 0, 0, 0).unwrap()], 0)
        .unwrap();
    assert_eq!(
        type_a.step(),
        Err(IBM704Error::UnknownTypeAPrefix {
            prefix: 0b111,
            pc: 0
        })
    );
    assert!(type_a.get_state().halted);

    let mut bad_address = IBM704GateLevel::with_memory_words(4).unwrap();
    bad_address
        .load_words(&[instruction(OP_CLA, 0, 7)], 0)
        .unwrap();
    assert_eq!(
        bad_address.step(),
        Err(IBM704Error::AddressOutOfRange {
            address: 7,
            capacity: 4
        })
    );
    assert!(bad_address.get_state().halted);
}

#[test]
fn public_word_helpers_mask_and_preserve_negative_zero() {
    assert_eq!(make_word(true, 0), SIGN_BIT);
    assert!(word_sign(make_word(true, 0)));
    assert_eq!(word_magnitude(make_word(true, 0)), 0);
    assert_eq!(make_word(false, u64::MAX), MAGNITUDE_MASK);
    assert_eq!(
        add_sign_magnitude(false, u64::MAX, false, 1),
        (false, 0, true)
    );
}
