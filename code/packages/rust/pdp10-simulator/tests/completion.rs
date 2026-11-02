use pdp10_simulator::{
    encode_instruction, encode_word, ArithmeticFlags, Pdp10Error, Pdp10Simulator, Pdp10State,
    ADDRESS_MASK, DEFAULT_ORIGIN, MEMORY_WORDS, WORD_MASK,
};

fn instruction(opcode: u16, accumulator: u8, indirect: bool, index: u8, address: u32) -> u64 {
    let encoded = encode_instruction(opcode, accumulator, indirect, index, address).unwrap();
    encoded
        .iter()
        .fold(0_u64, |word, byte| (word << 8) | u64::from(*byte))
}

fn program(words: &[u64]) -> Vec<u8> {
    words
        .iter()
        .flat_map(|word| encode_word(*word).unwrap())
        .collect()
}

fn loaded(words: &[u64]) -> Pdp10Simulator {
    let mut cpu = Pdp10Simulator::new();
    cpu.load(&program(words)).unwrap();
    cpu
}

fn signed_word(value: i64) -> u64 {
    (value as u64) & WORD_MASK
}

#[test]
fn checked_public_boundaries_are_atomic() {
    let halt = instruction(0o254, 4, false, 0, 0);
    let mut cpu = loaded(&[halt]);
    cpu.set_accumulator(1, 0o123).unwrap();
    let before = cpu.state();

    assert!(matches!(
        cpu.load_at(&[0; 4], DEFAULT_ORIGIN),
        Err(Pdp10Error::InvalidTransportLength { bytes: 4 })
    ));
    assert_eq!(cpu.state(), before);
    assert!(matches!(
        cpu.load_at(&program(&[halt, halt]), ADDRESS_MASK),
        Err(Pdp10Error::ProgramOutOfRange { .. })
    ));
    assert_eq!(cpu.state(), before);
    assert!(matches!(
        cpu.load_at(&program(&[halt]), ADDRESS_MASK + 1),
        Err(Pdp10Error::AddressOutOfRange { .. })
    ));
    assert_eq!(cpu.state(), before);
    assert!(matches!(
        cpu.set_accumulator(16, 0),
        Err(Pdp10Error::InvalidAccumulator { index: 16 })
    ));
    assert!(matches!(
        cpu.set_accumulator(0, WORD_MASK + 1),
        Err(Pdp10Error::WordOutOfRange { .. })
    ));
    assert!(matches!(
        cpu.set_memory_word(ADDRESS_MASK + 1, 0),
        Err(Pdp10Error::AddressOutOfRange { .. })
    ));
    assert_eq!(cpu.state(), before);
}

#[test]
fn restore_rejects_every_invalid_complete_state_without_mutation() {
    let mut cpu = loaded(&[instruction(0o254, 4, false, 0, 0)]);
    let before = cpu.state();

    let mut bad_memory_length = before.clone();
    bad_memory_length.memory.pop();
    assert!(matches!(
        cpu.restore(&bad_memory_length),
        Err(Pdp10Error::InvalidMemoryLength { .. })
    ));

    let mut bad_pc = before.clone();
    bad_pc.pc = ADDRESS_MASK + 1;
    assert!(matches!(
        cpu.restore(&bad_pc),
        Err(Pdp10Error::AddressOutOfRange { .. })
    ));

    let mut bad_word = before.clone();
    bad_word.memory[0o400] = WORD_MASK + 1;
    assert!(matches!(
        cpu.restore(&bad_word),
        Err(Pdp10Error::WordOutOfRange { .. })
    ));

    let mut bad_range = before.clone();
    bad_range.installed_origin = ADDRESS_MASK;
    bad_range.installed_words = 2;
    assert!(matches!(
        cpu.restore(&bad_range),
        Err(Pdp10Error::InvalidInstalledRange { .. })
    ));
    assert_eq!(cpu.state(), before);
}

#[test]
fn all_move_and_magnitude_forms_have_architectural_destinations() {
    for (opcode, expected_ac, expected_memory) in [
        (0o200, 0o123, 0o123),
        (0o201, 0o400, 0o123),
        (0o202, 0o765, 0o765),
        (0o203, 0o123, 0o123),
    ] {
        let mut cpu = loaded(&[instruction(opcode, 1, false, 0, 0o400)]);
        cpu.set_accumulator(1, 0o765).unwrap();
        cpu.set_memory_word(0o400, 0o123).unwrap();
        cpu.step().unwrap();
        assert_eq!(
            cpu.accumulator(1).unwrap(),
            expected_ac,
            "opcode {opcode:o}"
        );
        assert_eq!(cpu.memory_word(0o400).unwrap(), expected_memory);
    }

    for (opcode, expected_ac, expected_memory) in [
        (0o210, 5, signed_word(-5)),
        (0o211, signed_word(-0o400), signed_word(-5)),
        (0o212, 7, signed_word(-7)),
        (0o213, 5, 5),
        (0o214, 5, signed_word(-5)),
        (0o215, 0o400, signed_word(-5)),
        (0o216, 7, 7),
        (0o217, 5, 5),
    ] {
        let mut cpu = loaded(&[instruction(opcode, 1, false, 0, 0o400)]);
        cpu.set_accumulator(1, 7).unwrap();
        cpu.set_memory_word(0o400, signed_word(-5)).unwrap();
        cpu.step().unwrap();
        assert_eq!(
            cpu.accumulator(1).unwrap(),
            expected_ac,
            "opcode {opcode:o}"
        );
        assert_eq!(cpu.memory_word(0o400).unwrap(), expected_memory);
    }

    let mut cpu = loaded(&[instruction(0o210, 1, false, 0, 0o400)]);
    cpu.set_memory_word(0o400, 1 << 35).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.accumulator(1).unwrap(), 1 << 35);
    assert!(cpu.state().flags.overflow);

    let mut cpu = loaded(&[instruction(0o203, 0, false, 0, 0o400)]);
    cpu.set_memory_word(0o400, 0o765).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.accumulator(0).unwrap(), 0);
    assert_eq!(cpu.memory_word(0o400).unwrap(), 0o765);
}

#[test]
fn immediate_magnitude_flags_and_ac0_self_forms_follow_ka10_rules() {
    for opcode in [0o211, 0o215] {
        let mut cpu = loaded(&[instruction(opcode, 1, false, 0, 0)]);
        let mut state = cpu.state();
        state.flags = ArithmeticFlags {
            overflow: true,
            carry0: false,
            carry1: true,
            divide_check: true,
            pushdown_overflow: true,
        };
        cpu.restore(&state).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.state().flags, state.flags, "opcode {opcode:o}");
    }

    for opcode in [0o213, 0o217] {
        let mut cpu = loaded(&[instruction(opcode, 0, false, 0, 0o400)]);
        cpu.set_memory_word(0o400, signed_word(-5)).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(0).unwrap(), 0, "opcode {opcode:o}");
        assert_eq!(cpu.memory_word(0o400).unwrap(), 5, "opcode {opcode:o}");
    }
}

#[test]
fn exactly_sixty_four_indirect_links_are_valid_and_sixty_five_are_atomic() {
    fn install_chain(cpu: &mut Pdp10Simulator, links: usize, target: u32) {
        for link in 0..links {
            let address = 0o400 + u32::try_from(link).unwrap();
            let is_indirect = link + 1 < links;
            let next = if is_indirect { address + 1 } else { target };
            cpu.set_memory_word(address, instruction(0, 0, is_indirect, 0, next))
                .unwrap();
        }
    }

    for links in [63, 64] {
        let mut cpu = loaded(&[instruction(0o200, 1, true, 0, 0o400)]);
        install_chain(&mut cpu, links, 0o700);
        cpu.set_memory_word(0o700, 0o123456).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), 0o123456, "{links} links");
    }

    let mut cpu = loaded(&[instruction(0o200, 1, true, 0, 0o400)]);
    install_chain(&mut cpu, 65, 0o700);
    let before = cpu.state();
    assert!(matches!(
        cpu.step(),
        Err(Pdp10Error::IndirectAddressLimit { .. })
    ));
    assert_eq!(cpu.state(), before);
}

#[test]
fn pushdown_pointers_use_full_word_arithmetic_and_expose_overflow() {
    let mut push = loaded(&[instruction(0o261, 3, false, 0, 0o500)]);
    push.set_accumulator(3, 0o777777).unwrap();
    push.set_memory_word(0o500, 0o123).unwrap();
    push.step().unwrap();
    assert_eq!(push.accumulator(3).unwrap(), 0o000002_000000);

    let mut push_overflow = loaded(&[instruction(0o261, 3, false, 0, 0o500)]);
    push_overflow.set_accumulator(3, 0o777777_777776).unwrap();
    push_overflow.set_memory_word(0o500, 0o123).unwrap();
    push_overflow.step().unwrap();
    assert_eq!(push_overflow.accumulator(3).unwrap(), 0o000000_777777);
    assert!(push_overflow.state().flags.pushdown_overflow);

    let mut pop = loaded(&[instruction(0o262, 3, false, 0, 0o500)]);
    pop.set_accumulator(0, 0o456).unwrap();
    pop.set_accumulator(3, 0o000001_000000).unwrap();
    pop.step().unwrap();
    assert_eq!(pop.accumulator(3).unwrap(), 0o777777_777777);
    assert_eq!(pop.memory_word(0o500).unwrap(), 0o456);
    assert!(pop.state().flags.pushdown_overflow);

    let mut aliased_pop = loaded(&[instruction(0o262, 3, false, 0, 3)]);
    aliased_pop.set_accumulator(0, 0o123).unwrap();
    aliased_pop.set_accumulator(3, 0o000001_000000).unwrap();
    aliased_pop.step().unwrap();
    // KA10 POP is RDP; WR; SOBAC: when E is P, decrement the popped word.
    assert_eq!(aliased_pop.accumulator(3).unwrap(), 0o777777_000122);
    assert!(aliased_pop.state().flags.pushdown_overflow);
}

#[test]
fn add_sub_and_short_multiply_cover_all_four_forms() {
    for base in [0o270_u16, 0o274, 0o220] {
        for form in 0..4_u16 {
            let opcode = base + form;
            let mut cpu = loaded(&[instruction(opcode, 1, false, 0, 0o400)]);
            cpu.set_accumulator(1, 6).unwrap();
            cpu.set_memory_word(0o400, 3).unwrap();
            cpu.step().unwrap();
            let expected = match base {
                0o270 => {
                    if form == 1 {
                        6 + 0o400
                    } else {
                        9
                    }
                }
                0o274 => {
                    if form == 1 {
                        signed_word(6 - 0o400)
                    } else {
                        3
                    }
                }
                0o220 => {
                    if form == 1 {
                        6 * 0o400
                    } else {
                        18
                    }
                }
                _ => unreachable!(),
            };
            if form != 2 {
                assert_eq!(cpu.accumulator(1).unwrap(), expected, "opcode {opcode:o}");
            } else {
                assert_eq!(cpu.accumulator(1).unwrap(), 6);
            }
            if form >= 2 {
                assert_eq!(cpu.memory_word(0o400).unwrap(), expected);
            } else {
                assert_eq!(cpu.memory_word(0o400).unwrap(), 3);
            }
        }
    }

    let mut cpu = loaded(&[
        instruction(0o271, 1, false, 0, 1),
        instruction(0o271, 1, false, 0, 0),
    ]);
    cpu.set_accumulator(1, (1 << 35) - 1).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.accumulator(1).unwrap(), 1 << 35);
    assert_eq!(
        cpu.state().flags,
        ArithmeticFlags {
            overflow: true,
            carry0: false,
            carry1: true,
            divide_check: false,
            pushdown_overflow: false,
        }
    );
    cpu.step().unwrap();
    assert_eq!(
        cpu.state().flags,
        ArithmeticFlags {
            overflow: true,
            carry0: false,
            carry1: true,
            divide_check: false,
            pushdown_overflow: false,
        }
    );
}

#[test]
fn integer_divide_is_single_word_signed_and_faults_atomically() {
    for (dividend, divisor, quotient, remainder) in [
        (42, 5, 8, 2),
        (-42, 5, -8, -2),
        (42, -5, -8, 2),
        (-42, -5, 8, -2),
    ] {
        let mut cpu = loaded(&[instruction(0o230, 1, false, 0, 0o400)]);
        cpu.set_accumulator(1, signed_word(dividend)).unwrap();
        cpu.set_accumulator(2, 0o777).unwrap();
        cpu.set_memory_word(0o400, signed_word(divisor)).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), signed_word(quotient));
        assert_eq!(cpu.accumulator(2).unwrap(), signed_word(remainder));
    }

    for (dividend, divisor, expected) in [(7, 3, 2), (7, 0o400, 0)] {
        let mut cpu = loaded(&[instruction(0o231, 1, false, 0, divisor)]);
        cpu.set_accumulator(1, dividend).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), expected);
    }

    let mut memory_only = loaded(&[instruction(0o232, 1, false, 0, 0o400)]);
    memory_only.set_accumulator(1, 42).unwrap();
    memory_only.set_accumulator(2, 0o777).unwrap();
    memory_only.set_memory_word(0o400, 5).unwrap();
    memory_only.step().unwrap();
    assert_eq!(memory_only.accumulator(1).unwrap(), 42);
    assert_eq!(memory_only.accumulator(2).unwrap(), 0o777);
    assert_eq!(memory_only.memory_word(0o400).unwrap(), 8);

    let mut both = loaded(&[instruction(0o233, 1, false, 0, 0o400)]);
    both.set_accumulator(1, 42).unwrap();
    both.set_accumulator(2, 0o777).unwrap();
    both.set_memory_word(0o400, 5).unwrap();
    both.step().unwrap();
    assert_eq!(both.accumulator(1).unwrap(), 8);
    assert_eq!(both.accumulator(2).unwrap(), 2);
    assert_eq!(both.memory_word(0o400).unwrap(), 8);

    for (dividend, divisor, expected_error) in [
        (7, 0, Pdp10Error::DivideByZero { pc: DEFAULT_ORIGIN }),
        (
            1 << 35,
            WORD_MASK,
            Pdp10Error::QuotientOverflow { pc: DEFAULT_ORIGIN },
        ),
    ] {
        let mut cpu = loaded(&[instruction(0o230, 1, false, 0, 0o400)]);
        cpu.set_accumulator(1, dividend).unwrap();
        cpu.set_memory_word(0o400, divisor).unwrap();
        let before = cpu.state();
        assert_eq!(cpu.step().unwrap_err(), expected_error);
        assert_eq!(cpu.state(), before);
    }
}

#[test]
fn boolean_families_and_destinations_are_complete() {
    for (base, expected) in [
        (0o400_u16, 0),
        (0o404, 0b1000),
        (0o430, 0b0110),
        (0o434, 0b1110),
        (0o444, WORD_MASK ^ 0b0110),
        (0o474, WORD_MASK),
    ] {
        for form in 0..4_u16 {
            let opcode = base + form;
            let mut cpu = loaded(&[instruction(opcode, 1, false, 0, 0o400)]);
            cpu.set_accumulator(1, 0b1010).unwrap();
            cpu.set_memory_word(0o400, 0b1100).unwrap();
            cpu.step().unwrap();
            let form_expected = if form == 1 {
                match base {
                    0o400 => 0,
                    0o404 => 0,
                    0o430 => 0o412,
                    0o434 => 0o412,
                    0o444 => WORD_MASK ^ 0o412,
                    0o474 => WORD_MASK,
                    _ => unreachable!(),
                }
            } else {
                expected
            };
            if form != 2 {
                assert_eq!(
                    cpu.accumulator(1).unwrap(),
                    form_expected,
                    "opcode {opcode:o}"
                );
            }
            if form >= 2 {
                assert_eq!(cpu.memory_word(0o400).unwrap(), form_expected);
            }
        }
    }
}

#[test]
fn all_signed_condition_selectors_drive_skip_and_jump_families() {
    let orderings = [(4_u64, 5_u64), (5, 5), (6, 5)];
    let conditions = [
        [false, false, false],
        [true, false, false],
        [false, true, false],
        [true, true, false],
        [true, true, true],
        [false, true, true],
        [true, false, true],
        [false, false, true],
    ];
    for selector in 0..8_u16 {
        for (case, (lhs, rhs)) in orderings.into_iter().enumerate() {
            for family in [0o300_u16, 0o310] {
                let operand = if family == 0o300 { rhs as u32 } else { 0o202 };
                let mut cpu = loaded(&[instruction(family + selector, 1, false, 0, operand)]);
                cpu.set_accumulator(1, lhs).unwrap();
                if family == 0o310 {
                    cpu.set_memory_word(0o202, rhs).unwrap();
                }
                let trace = cpu.step().unwrap();
                let taken = conditions[usize::from(selector)][case];
                let expected = if taken { 0o202 } else { 0o201 };
                assert_eq!(
                    trace.pc_after, expected,
                    "family {family:o} selector {selector}"
                );
            }

            let jump_value = [signed_word(-1), 0, 1][case];
            let mut cpu = loaded(&[instruction(0o320 + selector, 1, false, 0, 0o202)]);
            cpu.set_accumulator(1, jump_value).unwrap();
            let expected = if conditions[usize::from(selector)][case] {
                0o202
            } else {
                0o201
            };
            assert_eq!(cpu.step().unwrap().pc_after, expected);
        }
    }
}

#[test]
fn count_jump_aobj_jrst_and_stack_control_are_deterministic() {
    for (family, initial, expected) in [(0o340_u16, 0_u64, 1_u64), (0o360, 2, 1)] {
        let mut cpu = loaded(&[instruction(family + 7, 1, false, 0, 0o202)]);
        cpu.set_accumulator(1, initial).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), expected);
        assert_eq!(trace.pc_after, 0o202);
    }

    let mut cpu = loaded(&[instruction(0o253, 1, false, 0, 0o202)]);
    cpu.set_accumulator(1, (0o777776_u64 << 18) | 0o400)
        .unwrap();
    assert_eq!(cpu.step().unwrap().pc_after, 0o202);
    assert_eq!(cpu.accumulator(1).unwrap(), (0o777777_u64 << 18) | 0o401);

    let mut cpu = loaded(&[instruction(0o254, 0, false, 0, 0o200)]);
    assert_eq!(cpu.step().unwrap().pc_after, 0o200);

    let mut cpu = loaded(&[instruction(0o254, 1, false, 0, 0)]);
    let before = cpu.state();
    assert!(matches!(
        cpu.step(),
        Err(Pdp10Error::InvalidInstruction { .. })
    ));
    assert_eq!(cpu.state(), before);

    let words = [
        instruction(0o260, 3, false, 0, 0o203),
        instruction(0o254, 4, false, 0, 0),
        instruction(0o254, 4, false, 0, 0),
        instruction(0o263, 3, false, 0, 0),
    ];
    let mut cpu = loaded(&words);
    cpu.set_accumulator(3, (0o777_u64 << 18) | 0o400).unwrap();
    let mut state = cpu.state();
    state.flags = ArithmeticFlags {
        overflow: true,
        carry0: true,
        carry1: true,
        divide_check: true,
        pushdown_overflow: false,
    };
    cpu.restore(&state).unwrap();
    assert_eq!(cpu.step().unwrap().pc_after, 0o203);
    let packed_flags = (1_u64 << 17) | (1 << 16) | (1 << 15) | (1 << 5);
    assert_eq!(
        cpu.memory_word(0o401).unwrap(),
        (packed_flags << 18) | 0o201
    );
    assert_eq!(cpu.step().unwrap().pc_after, 0o201);
    assert_eq!(cpu.accumulator(3).unwrap(), (0o777_u64 << 18) | 0o400);
}

#[test]
fn lifecycle_faults_and_bounded_runs_preserve_complete_state() {
    let mut cpu = loaded(&[instruction(0o254, 4, false, 0, 0)]);
    cpu.step().unwrap();
    let halted = cpu.state();
    assert_eq!(halted.pc, 0);
    assert!(matches!(cpu.step(), Err(Pdp10Error::Halted { .. })));
    assert_eq!(cpu.state(), halted);

    let mut fetch = loaded(&[instruction(0o254, 0, false, 0, DEFAULT_ORIGIN + 1)]);
    fetch.step().unwrap();
    let before = fetch.state();
    assert!(matches!(
        fetch.step(),
        Err(Pdp10Error::FetchOutsideProgram { .. })
    ));
    assert_eq!(fetch.state(), before);

    let mut unsupported = loaded(&[instruction(0, 0, false, 0, 0)]);
    let before = unsupported.state();
    assert!(matches!(
        unsupported.step(),
        Err(Pdp10Error::UnsupportedOpcode { opcode: 0, .. })
    ));
    assert_eq!(unsupported.state(), before);

    let mut bounded = loaded(&[instruction(0o254, 0, false, 0, DEFAULT_ORIGIN)]);
    let before = bounded.state();
    assert!(matches!(bounded.run(0), Err(Pdp10Error::MaxStepsZero)));
    assert_eq!(bounded.state(), before);
    assert!(matches!(
        bounded.run(3),
        Err(Pdp10Error::MaxStepsExceeded { max_steps: 3 })
    ));
    assert_eq!(bounded.state(), before);
}

#[test]
fn reset_and_complete_state_own_the_entire_memory_image() {
    let mut cpu = loaded(&[instruction(0o254, 4, false, 0, 0)]);
    cpu.set_accumulator(15, WORD_MASK).unwrap();
    cpu.set_memory_word(ADDRESS_MASK, WORD_MASK).unwrap();
    let snapshot = cpu.state();
    assert_eq!(snapshot.memory.len(), MEMORY_WORDS);
    assert_eq!(snapshot.installed_origin, DEFAULT_ORIGIN);
    assert_eq!(snapshot.installed_words, 1);

    cpu.reset();
    assert_eq!(cpu.accumulator(15).unwrap(), 0);
    assert_eq!(cpu.memory_word(ADDRESS_MASK).unwrap(), 0);
    assert_eq!(cpu.state().installed_words, 0);

    let external = Pdp10State {
        memory: vec![0; MEMORY_WORDS],
        ..snapshot
    };
    cpu.restore(&external).unwrap();
    assert_eq!(cpu.memory_word(ADDRESS_MASK).unwrap(), 0);
    assert_eq!(cpu.accumulator(15).unwrap(), WORD_MASK);
}
