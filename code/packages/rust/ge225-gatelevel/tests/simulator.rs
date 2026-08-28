use coding_adventures_ge225_simulator::{
    assemble_fixed, assemble_fixed_modified, assemble_select_x_group, assemble_shift,
    assemble_shift_modified, encode_instruction as functional_instruction, Simulator as Functional,
};
use ge225_gatelevel::{
    encode_instruction, Ge225GateError, Ge225GateLevel, CENTRAL_FLIP_FLOPS, MIN_MEMORY_WORDS,
};

const PROGRAM: i32 = 0o1000;

fn instruction(opcode: i32, modifier: i32, address: i32) -> i32 {
    encode_instruction(opcode, modifier, address).unwrap()
}

fn machines(program: &[i32]) -> (Ge225GateLevel, Functional) {
    let mut gate = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    let mut functional = Functional::new(MIN_MEMORY_WORDS as i32).unwrap();
    gate.load_words(program, PROGRAM as usize).unwrap();
    functional.load_words(program, PROGRAM).unwrap();
    gate.set_program_counter(PROGRAM).unwrap();
    functional.set_program_counter(PROGRAM).unwrap();
    (gate, functional)
}

fn assert_core_matches(gate: &Ge225GateLevel, functional: &Functional) {
    let gate = gate.get_state();
    let functional = functional.get_state();
    assert_eq!(gate.a, functional.a);
    assert_eq!(gate.q, functional.q);
    assert_eq!(gate.m, functional.m);
    assert_eq!(gate.n, functional.n);
    assert_eq!(gate.pc, functional.pc);
    assert_eq!(gate.ir, functional.ir);
    assert_eq!(gate.overflow, functional.overflow);
    assert_eq!(gate.parity_error, functional.parity_error);
    assert_eq!(gate.decimal_mode, functional.decimal_mode);
    assert_eq!(gate.decimal_carry, functional.decimal_carry);
    assert_eq!(gate.clock_sixths, functional.clock_sixths);
    assert_eq!(gate.n_ready, functional.n_ready);
    assert_eq!(gate.selected_x_group, functional.selected_x_group);
    assert_eq!(gate.halted, functional.halted);
    assert_eq!(gate.memory, functional.memory);
}

fn run_lockstep(gate: &mut Ge225GateLevel, functional: &mut Functional, steps: usize) {
    for _ in 0..steps {
        assert_core_matches(gate, functional);
        let gate_trace = gate.step().unwrap();
        let functional_trace = functional.step().unwrap();
        assert_eq!(gate_trace.pc_before, functional_trace.address);
        assert_eq!(gate_trace.instruction, functional_trace.instruction_word);
        assert_eq!(gate_trace.a_before, functional_trace.a_before);
        assert_eq!(gate_trace.a_after, functional_trace.a_after);
        assert_eq!(gate_trace.q_before, functional_trace.q_before);
        assert_eq!(gate_trace.q_after, functional_trace.q_after);
        assert_eq!(
            gate_trace.effective_address,
            functional_trace.effective_address
        );
    }
    assert_core_matches(gate, functional);
}

#[test]
fn construction_reset_load_and_bounds_are_gate_backed_and_fail_closed() {
    assert!(matches!(
        Ge225GateLevel::new(MIN_MEMORY_WORDS - 1),
        Err(Ge225GateError::InvalidMemorySize { .. })
    ));
    let mut gate = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    assert_eq!(
        gate.flip_flop_count(),
        MIN_MEMORY_WORDS * 20 + CENTRAL_FLIP_FLOPS
    );
    gate.load_words(&[1, 2, 3], 20).unwrap();
    gate.set_program_counter(20).unwrap();
    gate.reset();
    let state = gate.get_state();
    assert_eq!(state.memory, vec![0; MIN_MEMORY_WORDS]);
    assert_eq!(state.pc, 0);
    assert!(state.n_ready);
    assert!(gate.load_words(&[1, 2], MIN_MEMORY_WORDS - 1).is_err());
    gate.load_words(&[], MIN_MEMORY_WORDS).unwrap();
    assert!(gate.load_words(&[1], MIN_MEMORY_WORDS).is_err());
    assert!(gate.load_words(&[], usize::MAX).is_err());
    assert!(gate.set_program_counter(MIN_MEMORY_WORDS as i32).is_err());
}

#[test]
fn oversized_step_bound_does_not_allocate_before_the_first_step() {
    let mut gate = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    let unknown = 0o34 << 15;
    gate.write_word(0, unknown).unwrap();

    let error = gate.run(usize::MAX).unwrap_err();

    assert_eq!(
        error,
        Ge225GateError::UnknownInstruction {
            word: unknown,
            pc: 0
        }
    );
}

#[test]
fn load_add_subtract_store_runs_in_functional_lockstep() {
    let program = [
        instruction(0o00, 0, 0o400),
        instruction(0o01, 0, 0o402),
        instruction(0o02, 0, 0o404),
        instruction(0o03, 0, 0o406),
    ];
    let (mut gate, mut functional) = machines(&program);
    for (address, value) in [(0o400, 40), (0o402, 2), (0o404, 1)] {
        gate.write_word(address, value).unwrap();
        functional.write_word(address, value).unwrap();
    }
    run_lockstep(&mut gate, &mut functional, 4);
    assert_eq!(gate.get_state().a, 41);
    assert_eq!(gate.read_word(0o406).unwrap(), 41);
}

#[test]
fn complement_negate_transfers_and_branches_run_in_lockstep() {
    let program = [
        instruction(0o00, 0, 0o400),
        assemble_fixed("LQA").unwrap(),
        assemble_fixed("CPL").unwrap(),
        assemble_fixed("NEG").unwrap(),
        assemble_fixed("XAQ").unwrap(),
        assemble_fixed("BMI").unwrap(),
        assemble_fixed("NOP").unwrap(),
    ];
    let (mut gate, mut functional) = machines(&program);
    gate.write_word(0o400, 42).unwrap();
    functional.write_word(0o400, 42).unwrap();
    run_lockstep(&mut gate, &mut functional, 6);
}

#[test]
fn logic_compare_address_store_and_branch_use_gate_results() {
    let program = [
        instruction(0o00, 0, 0o400),
        instruction(0o20, 0, 0o402),
        instruction(0o23, 0, 0o404),
        instruction(0o27, 0, 0o406),
        instruction(0o21, 0, 0o410),
        instruction(0o26, 0, PROGRAM + 6),
        assemble_fixed("NOP").unwrap(),
    ];
    let (mut gate, mut functional) = machines(&program);
    for (address, value) in [
        (0o400, 0o765432),
        (0o402, 0o070070),
        (0o404, 0o001001),
        (0o406, 0o765432),
        (0o410, 0o700000),
    ] {
        gate.write_word(address, value).unwrap();
        functional.write_word(address, value).unwrap();
    }
    run_lockstep(&mut gate, &mut functional, 6);
}

#[test]
fn modification_uses_core_x_words_and_places_the_effective_operand_in_ir() {
    let program = [instruction(0o00, 1, 0o400)];
    let (mut gate, mut functional) = machines(&program);
    gate.write_word(1, 2).unwrap();
    functional.write_word(1, 2).unwrap();
    gate.write_word(0o402, 42).unwrap();
    functional.write_word(0o402, 42).unwrap();
    run_lockstep(&mut gate, &mut functional, 1);
    assert_eq!(gate.get_state().ir, instruction(0o00, 1, 0o402));
}

#[test]
fn signed_overflow_latches_and_branch_test_clears_it() {
    let program = [
        instruction(0o00, 0, 0o400),
        instruction(0o01, 0, 0o402),
        assemble_fixed("BOV").unwrap(),
    ];
    let (mut gate, mut functional) = machines(&program);
    for (address, value) in [(0o400, (1 << 19) - 1), (0o402, 1)] {
        gate.write_word(address, value).unwrap();
        functional.write_word(address, value).unwrap();
    }
    run_lockstep(&mut gate, &mut functional, 3);
    assert!(!gate.get_state().overflow);
}

#[test]
fn double_load_add_subtract_store_and_compare_run_in_lockstep() {
    let program = [
        instruction(0o10, 0, 0o400),
        instruction(0o11, 0, 0o402),
        instruction(0o12, 0, 0o404),
        instruction(0o13, 0, 0o406),
        instruction(0o22, 0, 0o406),
    ];
    let (mut gate, mut functional) = machines(&program);
    for (address, value) in [
        (0o400, 0o0000001),
        (0o401, 0o0003734),
        (0o402, 0o0000001),
        (0o403, 0o1104677),
        (0o404, 0o0000000),
        (0o405, 0o0000001),
    ] {
        gate.write_word(address, value).unwrap();
        functional.write_word(address, value).unwrap();
    }
    run_lockstep(&mut gate, &mut functional, 5);
}

#[test]
fn multiply_add_and_divide_use_gate_datapaths_in_lockstep() {
    let program = [
        instruction(0o10, 0, 0o400),
        instruction(0o15, 0, 0o402),
        instruction(0o16, 0, 0o404),
    ];
    let (mut gate, mut functional) = machines(&program);
    for (address, value) in [
        (0o400, 0o0000000),
        (0o401, 0o0122315),
        (0o402, 0o0146626),
        (0o404, 0o0146626),
    ] {
        gate.write_word(address, value).unwrap();
        functional.write_word(address, value).unwrap();
    }
    run_lockstep(&mut gate, &mut functional, 3);
}

#[test]
fn divide_check_is_atomic_and_sets_overflow_in_lockstep() {
    let program = [instruction(0o10, 0, 0o400), instruction(0o16, 0, 0o402)];
    let (mut gate, mut functional) = machines(&program);
    for (address, value) in [(0o400, 1), (0o401, 2), (0o402, 0)] {
        gate.write_word(address, value).unwrap();
        functional.write_word(address, value).unwrap();
    }
    run_lockstep(&mut gate, &mut functional, 2);
    assert!(gate.get_state().overflow);
    assert_eq!(gate.get_state().a, 1);
    assert_eq!(gate.get_state().q, 2);
}

#[test]
fn index_load_increment_store_branch_and_subroutine_run_in_lockstep() {
    let program = [
        instruction(0o06, 1, 0o400),
        instruction(0o14, 1, 2),
        instruction(0o17, 1, 0o402),
        instruction(0o04, 1, 7),
        assemble_fixed("NOP").unwrap(),
        instruction(0o07, 1, PROGRAM + 7),
        assemble_fixed("NOP").unwrap(),
        assemble_fixed("NOP").unwrap(),
    ];
    let (mut gate, mut functional) = machines(&program);
    gate.write_word(0o400, 5).unwrap();
    functional.write_word(0o400, 5).unwrap();
    run_lockstep(&mut gate, &mut functional, 6);
    assert_eq!(gate.read_word(0o402).unwrap(), 7);
}

#[test]
fn every_central_shift_family_runs_in_functional_lockstep() {
    for (mnemonic, count) in [
        ("SRA", 6),
        ("SNA", 5),
        ("SCA", 8),
        ("SAN", 6),
        ("SRD", 6),
        ("NAQ", 4),
        ("SCD", 9),
        ("ANQ", 3),
        ("SLA", 2),
        ("SLD", 7),
        ("NOR", 12),
        ("DNO", 12),
    ] {
        let program = [
            instruction(0o10, 0, 0o400),
            assemble_shift(mnemonic, count).unwrap(),
        ];
        let (mut gate, mut functional) = machines(&program);
        for (address, value) in [(0o400, 0o1234567), (0o401, 0o3654321)] {
            gate.write_word(address, value).unwrap();
            functional.write_word(address, value).unwrap();
        }
        run_lockstep(&mut gate, &mut functional, 2);
    }
}

#[test]
fn move_preflights_and_copies_overlaps_in_functional_lockstep() {
    let program = [instruction(0o10, 0, 0o400), instruction(0o24, 0, 0o500)];
    let (mut gate, mut functional) = machines(&program);
    for (address, value) in [
        (0o400, 0o502),
        (0o401, 0o3777775),
        (0o500, 11),
        (0o501, 22),
        (0o502, 33),
    ] {
        gate.write_word(address, value).unwrap();
        functional.write_word(address, value).unwrap();
    }
    run_lockstep(&mut gate, &mut functional, 2);
    assert_eq!(gate.read_word(0o502).unwrap(), 11);
    assert_eq!(gate.read_word(0o503).unwrap(), 22);
    assert_eq!(gate.read_word(0o504).unwrap(), 33);

    let (mut empty_gate, mut empty_functional) =
        machines(&[instruction(0o24, 0, MIN_MEMORY_WORDS as i32)]);
    run_lockstep(&mut empty_gate, &mut empty_functional, 1);
}

#[test]
fn sxg_and_automatic_fixed_and_shift_modification_run_in_lockstep() {
    let program = [
        assemble_select_x_group(2).unwrap(),
        instruction(0o00, 0, 0o400),
        assemble_shift_modified("SLA", 2, 1).unwrap(),
        assemble_fixed_modified("LAQ", 2).unwrap(),
    ];
    let (mut gate, mut functional) = machines(&program);
    for (address, value) in [(9, 3), (10, 1), (0o400, 0o2470)] {
        gate.write_word(address, value).unwrap();
        functional.write_word(address, value).unwrap();
    }
    run_lockstep(&mut gate, &mut functional, 4);
}

#[test]
fn excessive_modified_shift_fails_before_mutating_gate_state() {
    let program = [assemble_shift_modified("SLA", 31, 1).unwrap()];
    let (mut gate, _) = machines(&program);
    gate.write_word(1, 1).unwrap();
    let before = gate.get_state();
    assert!(matches!(
        gate.step(),
        Err(Ge225GateError::ShiftCountOutOfRange { count: 32 })
    ));
    assert_eq!(gate.get_state(), before);
}

#[test]
fn out_of_range_skip_fails_before_clocking_any_gate_state() {
    let mut gate = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    gate.write_word(
        (MIN_MEMORY_WORDS - 2) as i32,
        assemble_fixed("BOD").unwrap(),
    )
    .unwrap();
    gate.set_program_counter((MIN_MEMORY_WORDS - 2) as i32)
        .unwrap();
    let before = gate.get_state();
    assert!(matches!(
        gate.step(),
        Err(Ge225GateError::AddressOutOfRange { .. })
    ));
    assert_eq!(gate.get_state(), before);
}

#[test]
fn encoder_matches_the_functional_instruction_layout() {
    for opcode in 0..=0o37 {
        for modifier in 0..=3 {
            assert_eq!(
                encode_instruction(opcode, modifier, 0o1234).unwrap(),
                functional_instruction(opcode, modifier, 0o1234).unwrap()
            );
        }
    }
    assert!(encode_instruction(0o40, 0, 0).is_none());
    assert!(encode_instruction(0, 4, 0).is_none());
    assert!(encode_instruction(0, 0, 0o20000).is_none());
}

fn bcd(negative: bool, flagged: bool, digits: i32) -> i32 {
    let hundreds = digits / 100;
    let tens = (digits / 10) % 10;
    let ones = digits % 10;
    (if negative { 1 << 19 } else { 0 })
        | (if flagged { 1 << 18 } else { 0 })
        | (hundreds << 12)
        | (tens << 6)
        | ones
}

#[test]
fn decimal_single_add_subtract_and_one_words_use_gate_bcd() {
    let program = [
        assemble_fixed("SET_DECMODE").unwrap(),
        instruction(0o00, 0, 0o400),
        instruction(0o01, 0, 0o401),
        instruction(0o00, 0, 0o400),
        instruction(0o01, 0, 0o402),
        assemble_fixed("SBO").unwrap(),
        assemble_fixed("ADO").unwrap(),
        assemble_fixed("SET_BINMODE").unwrap(),
    ];
    let (mut gate, mut functional) = machines(&program);
    for (address, word) in [
        (0o400, bcd(false, true, 444)),
        (0o401, bcd(false, false, 333)),
        (0o402, bcd(true, false, 667)),
    ] {
        gate.write_word(address, word).unwrap();
        functional.write_word(address, word).unwrap();
    }
    run_lockstep(&mut gate, &mut functional, program.len());
    assert!(!gate.get_state().decimal_mode);
    assert_eq!(gate.get_state().a, bcd(false, true, 111));
}

#[test]
fn decimal_double_operations_and_complements_run_in_lockstep() {
    let program = [
        assemble_fixed("SET_DECMODE").unwrap(),
        instruction(0o10, 0, 0o400),
        instruction(0o11, 0, 0o402),
        instruction(0o10, 0, 0o400),
        instruction(0o12, 0, 0o402),
        instruction(0o10, 0, 0o400),
        instruction(0o11, 0, 0o404),
    ];
    let (mut gate, mut functional) = machines(&program);
    for (address, word) in [
        (0o400, bcd(false, true, 543)),
        (0o401, bcd(false, false, 210)),
        (0o402, bcd(false, true, 123)),
        (0o403, bcd(false, false, 456)),
        (0o404, bcd(true, true, 876)),
        (0o405, bcd(false, false, 544)),
    ] {
        gate.write_word(address, word).unwrap();
        functional.write_word(address, word).unwrap();
    }
    run_lockstep(&mut gate, &mut functional, program.len());
    assert_eq!(gate.get_state().a, bcd(false, true, 419));
    assert_eq!(gate.get_state().q, bcd(false, false, 754));
}

#[test]
fn decimal_carry_overflow_and_clear_latch_match_the_oracle() {
    let program = [
        assemble_fixed("SET_DECMODE").unwrap(),
        instruction(0o00, 0, 0o400),
        instruction(0o01, 0, 0o401),
        instruction(0o00, 0, 0o402),
        instruction(0o02, 0, 0o403),
        instruction(0o00, 0, 0o404),
        instruction(0o01, 0, 0o405),
    ];
    let (mut gate, mut functional) = machines(&program);
    for (address, word) in [
        (0o400, bcd(false, false, 999)),
        (0o401, bcd(false, false, 1)),
        (0o402, bcd(false, false, 0)),
        (0o403, bcd(false, false, 1)),
        (0o404, bcd(false, true, 999)),
        (0o405, bcd(false, true, 1)),
    ] {
        gate.write_word(address, word).unwrap();
        functional.write_word(address, word).unwrap();
    }
    run_lockstep(&mut gate, &mut functional, program.len());
    assert!(gate.get_state().overflow);
    gate.clear_decimal_carry();
    functional.clear_decimal_carry();
    assert_core_matches(&gate, &functional);
}

#[test]
fn seeded_decimal_vectors_match_the_functional_oracle() {
    let mut program = vec![assemble_fixed("SET_DECMODE").unwrap()];
    let mut operands = Vec::new();
    let mut seed = 0x225_u32;
    for vector in 0..48_i32 {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let left_digits = (seed % 1_000) as i32;
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let right_digits = (seed % 1_000) as i32;
        let flagged = vector % 3 != 0;
        let base = 0o3000 + vector * 2;
        operands.push((base, bcd(flagged && vector & 1 != 0, flagged, left_digits)));
        operands.push((
            base + 1,
            bcd(
                flagged && vector & 2 != 0,
                flagged && vector % 5 == 0,
                right_digits,
            ),
        ));
        program.push(instruction(0o00, 0, base));
        program.push(instruction(
            if vector & 1 == 0 { 0o01 } else { 0o02 },
            0,
            base + 1,
        ));
    }
    let (mut gate, mut functional) = machines(&program);
    for (address, word) in operands {
        gate.write_word(address, word).unwrap();
        functional.write_word(address, word).unwrap();
    }
    run_lockstep(&mut gate, &mut functional, program.len());
}

#[test]
fn invalid_decimal_words_fail_before_clocking_any_state() {
    for (accumulator, operand, expected) in [
        (
            bcd(false, true, 123),
            (1 << 18) | (0x0a << 12),
            Ge225GateError::InvalidBcd {
                word: (1 << 18) | (0x0a << 12),
            },
        ),
        (
            bcd(false, false, 123),
            bcd(false, true, 456),
            Ge225GateError::FlaggedDecimalOperand { double: false },
        ),
    ] {
        let program = [
            assemble_fixed("SET_DECMODE").unwrap(),
            instruction(0o00, 0, 0o400),
            instruction(0o01, 0, 0o401),
        ];
        let (mut gate, _) = machines(&program);
        gate.write_word(0o400, accumulator).unwrap();
        gate.write_word(0o401, operand).unwrap();
        gate.step().unwrap();
        gate.step().unwrap();
        let before = gate.get_state();
        assert_eq!(gate.step().unwrap_err(), expected);
        assert_eq!(gate.get_state(), before);
    }
}

#[test]
fn real_time_clock_register_and_wrap_match_the_oracle() {
    let program = [
        assemble_fixed("LAC").unwrap(),
        instruction(0o00, 0, 0o400),
        assemble_fixed("LCA").unwrap(),
    ];
    let (mut gate, mut functional) = machines(&program);
    gate.set_clock_sixths(0o1205701).unwrap();
    functional.set_clock_sixths(0o1205701).unwrap();
    gate.write_word(0o400, (1 << 19) | 0o0772200).unwrap();
    functional.write_word(0o400, (1 << 19) | 0o0772200).unwrap();
    run_lockstep(&mut gate, &mut functional, program.len());
    assert_eq!(gate.get_state().clock_sixths, 0o0772200);

    for (start, ticks) in [
        (24 * 60 * 60 * 6 - 1, 1),
        ((1 << 19) - 1, 1),
        (17, u64::MAX),
    ] {
        gate.set_clock_sixths(start).unwrap();
        functional.set_clock_sixths(start).unwrap();
        gate.advance_clock_sixths(ticks);
        functional.advance_clock_sixths(ticks);
        assert_core_matches(&gate, &functional);
    }
    assert!(matches!(
        gate.set_clock_sixths(-1),
        Err(Ge225GateError::InvalidClock { value: -1 })
    ));
    assert!(gate.set_clock_sixths(1 << 19).is_err());
}
