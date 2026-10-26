use avr_classic_simulator::{
    AvrClassicSimulator, AvrError, AvrState, TraceAllocation, DATA_SIZE, FLAG_C, FLAG_I, FLAG_Z,
    MAX_RETAINED_STEPS, PROGRAM_SIZE, RAM_END, SREG_ADDR,
};

fn image(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn rr(base: u16, d: usize, r: usize) -> u16 {
    base | ((d as u16 & 0x1f) << 4) | (r as u16 & 0x0f) | ((r as u16 & 0x10) << 5)
}

fn imm(base: u16, d: usize, k: u8) -> u16 {
    base | (((d - 16) as u16) << 4) | (k as u16 & 0x0f) | ((k as u16 & 0xf0) << 4)
}

fn one(word: u16) -> AvrClassicSimulator {
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[word]), 0).unwrap();
    cpu
}

#[test]
fn reset_load_and_mapped_views_are_complete() {
    let mut cpu = AvrClassicSimulator::new();
    assert_eq!(cpu.state().sp(), RAM_END);
    assert_eq!(cpu.state().program.len(), PROGRAM_SIZE);
    assert_eq!(cpu.state().data.len(), DATA_SIZE);
    cpu.set_register(3, 0xaa).unwrap();
    cpu.set_data(SREG_ADDR as u16, FLAG_I).unwrap();
    cpu.load(&image(&[0]), 0x20).unwrap();
    assert_eq!(cpu.state().pc, 0x10);
    assert_eq!(cpu.state().register(3), Some(0));
    assert_eq!(cpu.state().sreg(), 0);
    assert_eq!(cpu.state().loaded_origin, 0x20);
}

#[test]
fn malformed_load_is_atomic() {
    let mut cpu = AvrClassicSimulator::new();
    cpu.set_register(7, 9).unwrap();
    let before = cpu.state().clone();
    assert_eq!(
        cpu.load(&[1], 0),
        Err(AvrError::MisalignedProgram { origin: 0, len: 1 })
    );
    assert_eq!(cpu.state(), &before);
    assert!(matches!(
        cpu.load(&[0, 0], PROGRAM_SIZE),
        Err(AvrError::ProgramOutOfRange { .. })
    ));
    assert_eq!(cpu.state(), &before);
}

#[test]
fn restore_validates_all_owned_storage_and_pc() {
    let mut cpu = one(0);
    let before = cpu.state().clone();
    let mut bad = before.clone();
    bad.program.pop();
    assert!(matches!(
        cpu.restore(bad),
        Err(AvrError::InvalidState { .. })
    ));
    assert_eq!(cpu.state(), &before);
    let mut bad = before.clone();
    bad.data.pop();
    assert!(matches!(
        cpu.restore(bad),
        Err(AvrError::InvalidState { .. })
    ));
    let mut bad = before.clone();
    bad.pc = 0x4000;
    assert!(matches!(
        cpu.restore(bad),
        Err(AvrError::InvalidState { .. })
    ));
}

#[test]
fn terminal_post_states_round_trip_through_restore() {
    let mut nop = one(0x0000);
    let nop_trace = nop.step().unwrap();
    assert_eq!(nop_trace.state_after.pc, 1);
    let nop_state = nop_trace.state_after.clone();
    nop.restore(nop_state.clone()).unwrap();
    assert_eq!(nop.state(), &nop_state);
    assert_eq!(nop.step(), Err(AvrError::FetchOutsideProgram { pc: 1 }));

    let mut halted = one(0x9598);
    let break_trace = halted.step().unwrap();
    assert!(break_trace.state_after.halted);
    assert_eq!(break_trace.state_after.pc, 1);
    let break_state = break_trace.state_after.clone();
    halted.restore(break_state.clone()).unwrap();
    assert_eq!(halted.state(), &break_state);
    assert_eq!(halted.step(), Err(AvrError::Halted));
}

#[test]
fn exact_trace_and_break_lifecycle() {
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[0, 0x9598]), 0).unwrap();
    let result = cpu.run(2).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces[0].fetched, [0, 0]);
    assert_eq!(result.traces[1].fetched, [0x98, 0x95]);
    assert_eq!(result.traces[1].mnemonic, "BREAK");
    let frozen = cpu.state().clone();
    assert_eq!(cpu.step(), Err(AvrError::Halted));
    assert_eq!(cpu.state(), &frozen);
}

#[test]
fn run_and_execute_are_transactional() {
    let mut cpu = one(0);
    cpu.set_register(4, 0x55).unwrap();
    let before = cpu.state().clone();
    assert_eq!(cpu.run(1), Err(AvrError::StepLimitExceeded { limit: 1 }));
    assert_eq!(cpu.state(), &before);
    assert!(matches!(
        cpu.execute(&image(&[0xffff]), 0, 1),
        Err(AvrError::UnknownOpcode { .. })
    ));
    assert_eq!(cpu.state(), &before);
    assert!(matches!(
        cpu.run(MAX_RETAINED_STEPS + 1),
        Err(AvrError::RetentionLimitExceeded { .. })
    ));
    assert_eq!(
        cpu.run_with_allocation(1, TraceAllocation::Fail),
        Err(AvrError::AllocationFailed)
    );
    assert_eq!(cpu.state(), &before);
}

#[test]
fn carry_input_uses_original_operand_in_flag_formula() {
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[rr(0x1c00, 2, 3), rr(0x0800, 4, 5)]), 0)
        .unwrap();
    cpu.set_register(2, 0).unwrap();
    cpu.set_register(3, 0xff).unwrap();
    cpu.set_data(SREG_ADDR as u16, FLAG_C).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().register(2), Some(0));
    assert_eq!(cpu.state().sreg() & (FLAG_C | FLAG_Z), FLAG_C);
    cpu.set_register(4, 0).unwrap();
    cpu.set_register(5, 0xff).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().register(4), Some(0));
    assert_eq!(cpu.state().sreg() & (FLAG_C | FLAG_Z), FLAG_C);
}

#[test]
fn every_updating_ld_st_overlap_is_typed_and_atomic() {
    let forms = [
        (0x900d, 26),
        (0x900d, 27),
        (0x900e, 26),
        (0x900e, 27),
        (0x9009, 28),
        (0x9009, 29),
        (0x900a, 28),
        (0x900a, 29),
        (0x9001, 30),
        (0x9001, 31),
        (0x9002, 30),
        (0x9002, 31),
        (0x920d, 26),
        (0x920d, 27),
        (0x920e, 26),
        (0x920e, 27),
        (0x9209, 28),
        (0x9209, 29),
        (0x920a, 28),
        (0x920a, 29),
        (0x9201, 30),
        (0x9201, 31),
        (0x9202, 30),
        (0x9202, 31),
    ];
    for (base, register) in forms {
        let opcode = base | ((register as u16) << 4);
        let mut cpu = one(opcode);
        cpu.set_register(26, 0x00).unwrap();
        cpu.set_register(27, 0x01).unwrap();
        cpu.set_register(28, 0x00).unwrap();
        cpu.set_register(29, 0x01).unwrap();
        cpu.set_register(30, 0x00).unwrap();
        cpu.set_register(31, 0x01).unwrap();
        let before = cpu.state().clone();
        assert_eq!(
            cpu.step(),
            Err(AvrError::UnsupportedOperandCombination { opcode, pc: 0 }),
            "opcode {opcode:#06x}"
        );
        assert_eq!(cpu.state(), &before, "opcode {opcode:#06x}");
    }
}

#[test]
fn both_lpm_z_plus_overlaps_are_typed_and_atomic() {
    for register in [30usize, 31] {
        let opcode = 0x9005 | ((register as u16) << 4);
        let mut cpu = one(opcode);
        let before = cpu.state().clone();
        assert_eq!(
            cpu.step(),
            Err(AvrError::UnsupportedOperandCombination { opcode, pc: 0 })
        );
        assert_eq!(cpu.state(), &before);
    }
}

#[test]
fn legal_nonupdating_and_displacement_pointer_overlaps_execute() {
    // LD R26,X and ST X,R27 are legal because the pointer is not updated.
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[0x900c | (26 << 4), 0x920c | (27 << 4)]), 0)
        .unwrap();
    cpu.set_register(26, 0x00).unwrap();
    cpu.set_register(27, 0x01).unwrap();
    cpu.set_data(0x0100, 0x34).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().register(26), Some(0x34));
    // Re-form X after deliberately loading its low byte.
    cpu.set_register(26, 0x00).unwrap();
    cpu.set_register(27, 0x01).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().data[0x100], 1);

    // LDD R28,Y+1 is legal even though the destination is part of Y.
    let mut cpu = one(0x8008 | (28 << 4) | 1);
    cpu.set_register(28, 0x00).unwrap();
    cpu.set_register(29, 0x01).unwrap();
    cpu.set_data(0x0101, 0xa5).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().register(28), Some(0xa5));
}

#[test]
fn legal_lpm_forms_preserve_or_update_z() {
    let mut cpu = one(0x95c8);
    cpu.set_register(30, 0).unwrap();
    cpu.set_register(31, 0).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().register(0), Some(0xc8));
    assert_eq!(cpu.state().register(30), Some(0));

    let mut cpu = one(0x9005 | (2 << 4));
    cpu.set_register(30, 0).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().register(2), Some(0x25));
    assert_eq!(cpu.state().register(30), Some(1));
}

#[test]
fn arithmetic_logic_and_chained_zero_flags() {
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(
        &image(&[
            imm(0xe000, 16, 0xff),
            imm(0xe000, 17, 1),
            rr(0x0c00, 16, 17),
            rr(0x1c00, 16, 17),
            rr(0x2000, 16, 17),
            rr(0x2400, 16, 17),
        ]),
        0,
    )
    .unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().register(16), Some(0));
    assert_eq!(cpu.state().sreg() & (FLAG_C | FLAG_Z), FLAG_C | FLAG_Z);
    cpu.step().unwrap();
    assert_eq!(cpu.state().register(16), Some(2));
    assert_eq!(cpu.state().sreg() & FLAG_Z, 0);
    cpu.step().unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().register(16), Some(1));
}

#[test]
fn immediate_word_unary_and_multiply_families() {
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(
        &image(&[
            imm(0xe000, 24, 1),
            0x9601,
            0x9403 | (24 << 4),
            0x940a | (24 << 4),
            imm(0xe000, 16, 3),
            imm(0xe000, 17, 4),
            rr(0x9c00, 16, 17),
        ]),
        0,
    )
    .unwrap();
    for _ in 0..7 {
        cpu.step().unwrap();
    }
    assert_eq!(cpu.state().register(24), Some(2));
    assert_eq!(cpu.state().register(0), Some(12));
    assert_eq!(cpu.state().register(1), Some(0));
}

#[test]
fn every_arithmetic_subfamily_and_multiplier_variant_decodes() {
    let words = [
        rr(0x1800, 2, 3),
        rr(0x1400, 2, 3),
        rr(0x0400, 2, 3),
        rr(0x2c00, 4, 3),
        0x0100 | (3 << 4) | 1,
        imm(0x3000, 16, 1),
        imm(0x4000, 16, 1),
        imm(0x5000, 16, 1),
        imm(0x6000, 16, 0x80),
        imm(0x7000, 16, 0x7f),
        0x9701,
        0x9400 | (6 << 4),
        0x9401 | (6 << 4),
        0x9402 | (6 << 4),
        0x9405 | (6 << 4),
        0x9406 | (6 << 4),
        0x9407 | (6 << 4),
        0x0201,
        0x0301,
        0x0309,
        0x0381,
        0x0389,
    ];
    let expected = [
        "SUB", "CP", "CPC", "MOV", "MOVW", "CPI", "SBCI", "SUBI", "ORI", "ANDI", "SBIW", "COM",
        "NEG", "SWAP", "ASR", "LSR", "ROR", "MULS", "MULSU", "FMUL", "FMULS", "FMULSU",
    ];
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&words), 0).unwrap();
    cpu.set_register(2, 0x81).unwrap();
    cpu.set_register(3, 1).unwrap();
    cpu.set_register(6, 0x81).unwrap();
    cpu.set_register(17, 2).unwrap();
    cpu.set_register(24, 4).unwrap();
    for name in expected {
        assert_eq!(cpu.step().unwrap().mnemonic, name);
    }
}

#[test]
fn all_pointer_update_families_and_displacements_execute_legally() {
    let forms = [
        (0x900d, 26, true),
        (0x900e, 26, true),
        (0x920d, 26, false),
        (0x920e, 26, false),
        (0x9009, 28, true),
        (0x900a, 28, true),
        (0x9209, 28, false),
        (0x920a, 28, false),
        (0x9001, 30, true),
        (0x9002, 30, true),
        (0x9201, 30, false),
        (0x9202, 30, false),
    ];
    for (base, pointer, load) in forms {
        let opcode = base | (2 << 4);
        let mut cpu = one(opcode);
        let start = if base & 0x000f == 0x000a || base & 0x000f == 0x000e || base & 0x000f == 0x0002
        {
            0x0101
        } else {
            0x0100
        };
        cpu.set_register(pointer, start as u8).unwrap();
        cpu.set_register(pointer + 1, (start >> 8) as u8).unwrap();
        cpu.set_register(2, 0x5a).unwrap();
        cpu.set_data(0x0100, 0xa5).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(trace.mnemonic, if load { "LD" } else { "ST" });
        if load {
            assert_eq!(cpu.state().register(2), Some(0xa5));
        } else {
            assert_eq!(cpu.state().data[0x0100], 0x5a);
        }
    }

    for (opcode, pointer) in [(0x8000 | (2 << 4) | 3, 30), (0x8208 | (2 << 4) | 3, 28)] {
        let mut cpu = one(opcode);
        cpu.set_register(pointer, 0x00).unwrap();
        cpu.set_register(pointer + 1, 0x01).unwrap();
        cpu.set_register(2, 0x33).unwrap();
        cpu.set_data(0x0103, 0x44).unwrap();
        cpu.step().unwrap();
    }
}

#[test]
fn status_branches_and_absolute_and_indirect_control_execute() {
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[0xf009, 0, 0x9598]), 0).unwrap(); // BRBS Z,+1
    cpu.set_data(SREG_ADDR as u16, FLAG_Z).unwrap();
    assert!(cpu.step().unwrap().branch_taken);
    cpu.step().unwrap();

    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[0xf409, 0, 0x9598]), 0).unwrap(); // BRBC Z,+1
    assert!(cpu.step().unwrap().branch_taken);

    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[0x940c, 2, 0x9598]), 0).unwrap();
    assert_eq!(cpu.step().unwrap().mnemonic, "JMP");
    cpu.step().unwrap();

    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[0x940e, 3, 0x9598, 0x9508]), 0).unwrap();
    assert_eq!(cpu.step().unwrap().mnemonic, "CALL");
    assert_eq!(cpu.step().unwrap().mnemonic, "RET");
    cpu.step().unwrap();

    for (opcode, target, expected) in [(0x9409, 2, "IJMP"), (0x9509, 2, "ICALL")] {
        let tail = if opcode == 0x9509 { 0x9508 } else { 0x9598 };
        let mut cpu = AvrClassicSimulator::new();
        cpu.load(&image(&[opcode, 0x9598, tail]), 0).unwrap();
        cpu.set_register(30, target).unwrap();
        assert_eq!(cpu.step().unwrap().mnemonic, expected);
        if opcode == 0x9509 {
            assert_eq!(cpu.step().unwrap().mnemonic, "RET");
        }
    }
}

#[test]
fn profile_exclusions_have_their_own_error_class() {
    for opcode in [
        0x9419, 0x9519, 0x95d8, 0x9006, 0x9007, 0x9204, 0x9205, 0x9206, 0x9207, 0x940b,
    ] {
        let mut cpu = one(opcode);
        assert!(matches!(
            cpu.step(),
            Err(AvrError::UnsupportedProfileInstruction { .. })
        ));
    }
}

#[test]
fn absolute_target_high_bits_are_decoded_in_architectural_positions() {
    // Opcode bit zero is target bit 16. On this two-byte-PC device that must
    // produce a typed profile rejection, not be misplaced as target bit 21.
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[0x940d, 0x0000]), 0).unwrap();
    assert_eq!(
        cpu.step(),
        Err(AvrError::UnsupportedProfileInstruction {
            opcode: 0x940d,
            pc: 0,
        })
    );
}

#[test]
fn every_error_has_stable_nonempty_display_text() {
    let errors = vec![
        AvrError::EmptyProgram,
        AvrError::MisalignedProgram { origin: 1, len: 1 },
        AvrError::ProgramOutOfRange {
            origin: usize::MAX,
            len: 2,
        },
        AvrError::FetchOutsideProgram { pc: 1 },
        AvrError::TruncatedInstruction { pc: 0 },
        AvrError::DataOutOfRange { address: 0xffff },
        AvrError::ProgramReadOutOfRange { address: 0xffff },
        AvrError::StackOutOfRange {
            sp: 0x100,
            width: 2,
        },
        AvrError::InvalidStackPointer { sp: 0 },
        AvrError::InvalidState { reason: "test" },
        AvrError::UnknownOpcode {
            opcode: 0xffff,
            pc: 0,
        },
        AvrError::UnsupportedProfileInstruction {
            opcode: 0x9419,
            pc: 0,
        },
        AvrError::UnsupportedSystemInstruction {
            opcode: 0x9588,
            pc: 0,
        },
        AvrError::UnsupportedOperandCombination {
            opcode: 0x91ad,
            pc: 0,
        },
        AvrError::Halted,
        AvrError::RetentionLimitExceeded {
            requested: 65,
            limit: 64,
        },
        AvrError::AllocationFailed,
        AvrError::StepLimitExceeded { limit: 1 },
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn direct_mapped_io_and_stack_transfers() {
    let mut cpu = AvrClassicSimulator::new();
    let out_r16_a0 = 0xb800 | (16 << 4);
    let in_r17_a0 = 0xb000 | (17 << 4);
    cpu.load(
        &image(&[
            imm(0xe000, 16, 0x5a),
            out_r16_a0,
            in_r17_a0,
            0x920f | (17 << 4),
            0x900f | (18 << 4),
            0x9200 | (18 << 4),
            0x0100,
            0x9000 | (19 << 4),
            0x0100,
        ]),
        0,
    )
    .unwrap();
    for _ in 0..7 {
        cpu.step().unwrap();
    }
    assert_eq!(cpu.state().register(17), Some(0x5a));
    assert_eq!(cpu.state().register(18), Some(0x5a));
    assert_eq!(cpu.state().register(19), Some(0x5a));
    assert_eq!(cpu.state().sp(), RAM_END);
}

#[test]
fn stack_faults_and_data_faults_roll_back() {
    let mut cpu = one(0x920f);
    cpu.set_register(0, 7).unwrap();
    cpu.set_data(0x5d, 0).unwrap();
    cpu.set_data(0x5e, 1).unwrap();
    let before = cpu.state().clone();
    assert!(matches!(cpu.step(), Err(AvrError::StackOutOfRange { .. })));
    assert_eq!(cpu.state(), &before);

    let mut cpu = one(0x900c);
    cpu.set_register(26, 0xff).unwrap();
    cpu.set_register(27, 0xff).unwrap();
    let before = cpu.state().clone();
    assert_eq!(
        cpu.step(),
        Err(AvrError::DataOutOfRange { address: 0xffff })
    );
    assert_eq!(cpu.state(), &before);
}

#[test]
fn calls_preflight_the_whole_two_byte_return_address() {
    let cases = [
        (image(&[0xd000, 0x9598]), None),
        (image(&[0x940e, 2, 0x9598]), None),
        (image(&[0x9509, 0x9598]), Some(1_u16)),
    ];

    for (program, indirect_target) in cases {
        let mut cpu = AvrClassicSimulator::new();
        cpu.load(&program, 0).unwrap();
        if let Some(target) = indirect_target {
            cpu.set_register(30, target as u8).unwrap();
            cpu.set_register(31, (target >> 8) as u8).unwrap();
        }
        cpu.set_data(0x5d, 0x01).unwrap();
        cpu.set_data(0x5e, 0x01).unwrap();
        let before = cpu.state().clone();
        assert_eq!(
            cpu.step(),
            Err(AvrError::StackOutOfRange {
                sp: 0x0101,
                width: 2,
            })
        );
        assert_eq!(cpu.state(), &before);
    }
}

#[test]
fn bit_status_and_io_bit_families() {
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(
        &image(&[
            0x9408 | (6 << 4),     // BSET T
            0xf800 | (2 << 4) | 3, // BLD R2,3
            0xfa00 | (2 << 4) | 3, // BST R2,3
            0x9a00 | (3 << 3) | 2, // SBI 3,2
            0x9800 | (3 << 3) | 2, // CBI 3,2
        ]),
        0,
    )
    .unwrap();
    for _ in 0..5 {
        cpu.step().unwrap();
    }
    assert_eq!(cpu.state().register(2), Some(8));
    assert_eq!(cpu.state().data[0x23] & 4, 0);
}

#[test]
fn all_skip_sources_classify_two_word_successors() {
    let cases = [
        (rr(0x1000, 2, 3), true, false),
        (0xfc00 | (2 << 4), true, false),
        (0xfe00 | (2 << 4), false, false),
        (0x9900, true, false),
        (0x9b00, true, true),
    ];
    for (skip, zero_bit, io_set) in cases {
        let mut cpu = AvrClassicSimulator::new();
        cpu.load(&image(&[skip, 0x9000, 0x0100, 0x9598]), 0)
            .unwrap();
        cpu.set_register(2, if zero_bit { 0 } else { 1 }).unwrap();
        cpu.set_register(3, 0).unwrap();
        cpu.set_data(0x20, u8::from(io_set)).unwrap();
        let trace = cpu.step().unwrap();
        assert!(trace.skipped, "opcode {skip:#06x}");
        assert_eq!(cpu.state().pc, 3);
    }
}

#[test]
fn truncated_two_word_fetch_and_skip_are_atomic() {
    for prefix in [0x9000, 0x9200, 0x940c, 0x940e] {
        let mut cpu = one(prefix);
        let before = cpu.state().clone();
        assert_eq!(cpu.step(), Err(AvrError::TruncatedInstruction { pc: 0 }));
        assert_eq!(cpu.state(), &before);
    }
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[rr(0x1000, 2, 2), 0x9000]), 0).unwrap();
    let before = cpu.state().clone();
    assert_eq!(cpu.step(), Err(AvrError::TruncatedInstruction { pc: 1 }));
    assert_eq!(cpu.state(), &before);
}

#[test]
fn branches_calls_returns_and_reti_validate_targets() {
    // RCALL +1 => word 2; RET returns to word 1; RJMP +1 => BREAK.
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[0xd001, 0xc001, 0x9508, 0x9598]), 0)
        .unwrap();
    assert_eq!(cpu.step().unwrap().mnemonic, "RCALL");
    assert_eq!(cpu.state().pc, 2);
    assert_eq!(cpu.step().unwrap().mnemonic, "RET");
    assert_eq!(cpu.state().pc, 1);
    cpu.step().unwrap();
    cpu.step().unwrap();
    assert!(cpu.state().halted);

    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[0xd001, 0xc001, 0x9518, 0x9598]), 0)
        .unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();
    assert_ne!(cpu.state().sreg() & FLAG_I, 0);
}

#[test]
fn branch_and_indirect_target_faults_are_atomic() {
    let mut cpu = one(0xc001);
    let before = cpu.state().clone();
    assert!(matches!(
        cpu.step(),
        Err(AvrError::FetchOutsideProgram { .. })
    ));
    assert_eq!(cpu.state(), &before);

    let mut cpu = one(0x9409);
    cpu.set_register(30, 4).unwrap();
    let before = cpu.state().clone();
    assert!(matches!(
        cpu.step(),
        Err(AvrError::FetchOutsideProgram { .. })
    ));
    assert_eq!(cpu.state(), &before);
}

#[test]
fn unsupported_system_and_unknown_opcodes_are_atomic_and_displayed() {
    for opcode in [0x9588, 0x95a8, 0x95e8, 0x95f8] {
        let mut cpu = one(opcode);
        let before = cpu.state().clone();
        let error = cpu.step().unwrap_err();
        assert!(matches!(
            error,
            AvrError::UnsupportedSystemInstruction { .. }
        ));
        assert!(!error.to_string().is_empty());
        assert_eq!(cpu.state(), &before);
    }
    let mut cpu = one(0xffff);
    let before = cpu.state().clone();
    assert!(matches!(cpu.step(), Err(AvrError::UnknownOpcode { .. })));
    assert_eq!(cpu.state(), &before);
}

#[test]
fn self_modifying_mapped_write_does_not_change_fetched_trace() {
    // STS writes data only (Harvard separation), and a trace owns pre-step bytes.
    let mut cpu = AvrClassicSimulator::new();
    cpu.load(&image(&[0x9200 | (2 << 4), 0x0000]), 0).unwrap();
    cpu.set_register(2, 0xaa).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.fetched, [0x20, 0x92, 0x00, 0x00]);
    assert_eq!(cpu.state().program[0], 0x20);
    assert_eq!(cpu.state().data[0], 0xaa);
}

#[test]
fn public_state_can_round_trip_exactly() {
    let mut cpu = one(0);
    cpu.set_register(31, 0x77).unwrap();
    let snapshot: AvrState = cpu.state().clone();
    cpu.reset();
    cpu.restore(snapshot.clone()).unwrap();
    assert_eq!(cpu.state(), &snapshot);
}
