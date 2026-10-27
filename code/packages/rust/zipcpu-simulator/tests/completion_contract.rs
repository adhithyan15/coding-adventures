use zipcpu_simulator::{
    decode, encoding, AllocationPoint, Bank, Condition, DecodeError, FaultCause, InstructionClass,
    Mode, StopReason, ZipCpuError, ZipCpuSimulator, CC_BREAK, CC_BUS_ERROR, CC_C, CC_CPU_INFO,
    CC_DIV_ERROR, CC_GIE, CC_ILLEGAL, CC_N, CC_RESERVED_MASK, CC_SLEEP, CC_STEP, CC_TRAP, CC_V,
    CC_Z, MEMORY_SIZE,
};

const ORIGIN: u32 = 0x100;

fn image(words: &[u32]) -> Vec<u8> {
    encoding::assemble(words).unwrap()
}

fn cpu_with(words: &[u32]) -> ZipCpuSimulator {
    let mut cpu = ZipCpuSimulator::new().unwrap();
    cpu.load(&image(words), ORIGIN).unwrap();
    cpu
}

fn set_register(cpu: &mut ZipCpuSimulator, bank: Bank, register: usize, value: u32) {
    let mut state = cpu.snapshot().unwrap();
    match (bank, register) {
        (Bank::Supervisor, 0..=13) => state.supervisor_gpr[register] = value,
        (Bank::User, 0..=13) => state.user_gpr[register] = value,
        _ => panic!("test helper only writes ordinary registers"),
    }
    cpu.restore(&state).unwrap();
}

#[test]
fn new_load_reset_and_snapshot_are_complete() {
    let mut cpu = ZipCpuSimulator::new().unwrap();
    let reset = cpu.snapshot().unwrap();
    assert_eq!(reset.memory.len(), MEMORY_SIZE);
    assert_eq!(reset.supervisor_cc, CC_CPU_INFO);
    assert_eq!(reset.user_cc, CC_CPU_INFO | CC_GIE);
    cpu.load(&image(&[encoding::ldi(1, 7)]), ORIGIN).unwrap();
    assert_eq!(cpu.state().supervisor_pc, ORIGIN);
    assert_eq!(cpu.state().user_pc, 0);
    cpu.write_byte(0x200, 0xaa).unwrap();
    cpu.reset();
    assert_eq!(cpu.state().supervisor_pc, 0);
    assert_eq!(cpu.state().user_pc, 0);
    assert_eq!(cpu.read_byte(0x200).unwrap(), 0xaa);
    assert_eq!(cpu.state().image_origin, Some(ORIGIN));
}

#[test]
fn load_rejects_every_transport_boundary_atomically() {
    let mut cpu = cpu_with(&[encoding::ldi(1, 1)]);
    let before = cpu.snapshot().unwrap();
    let cases = [
        cpu.load(&[], 0).unwrap_err(),
        cpu.load(&[0; 4], 2).unwrap_err(),
        cpu.load(&[0; 2], 0).unwrap_err(),
        cpu.load(&vec![0; MEMORY_SIZE + 4], 0).unwrap_err(),
        cpu.load(&[0; 8], u32::MAX - 3).unwrap_err(),
        cpu.load(&[0; 8], (MEMORY_SIZE - 4) as u32).unwrap_err(),
    ];
    assert!(matches!(cases[0], ZipCpuError::EmptyProgram));
    assert!(matches!(cases[1], ZipCpuError::MisalignedProgram { .. }));
    assert!(matches!(cases[2], ZipCpuError::MisalignedProgram { .. }));
    assert!(matches!(cases[3], ZipCpuError::ProgramTooLarge { .. }));
    assert!(matches!(cases[4], ZipCpuError::ProgramRangeOverflow { .. }));
    assert!(matches!(cases[5], ZipCpuError::ProgramOutOfRange { .. }));
    assert_eq!(cpu.snapshot().unwrap(), before);
}

#[test]
fn all_primary_opcodes_destinations_conditions_and_forms_classify() {
    for opcode in 0_u32..32 {
        for destination in 0_u32..16 {
            for condition in 0_u32..8 {
                for register_form in [false, true] {
                    let word = (destination << 27)
                        | (opcode << 22)
                        | (condition << 19)
                        | (u32::from(register_form) << 18)
                        | (3 << 14)
                        | 7;
                    let result = decode(word);
                    let special = destination & 0xe == 0xe;
                    match opcode {
                        0x00..=0x0d | 0x10..=0x19 => assert!(result.is_ok(), "{word:#x}"),
                        0x0e | 0x0f if !special => assert!(result.is_ok(), "{word:#x}"),
                        0x0e | 0x0f => {
                            assert!(matches!(
                                result,
                                Err(DecodeError::ReservedInstruction { .. })
                            ))
                        }
                        0x1a | 0x1b if special => {
                            assert!(matches!(
                                result,
                                Err(DecodeError::ReservedInstruction { .. })
                            ))
                        }
                        0x1c | 0x1f if special => assert!(result.is_ok(), "{word:#x}"),
                        0x1d | 0x1e if special => assert!(matches!(
                            result,
                            Err(DecodeError::UnsupportedInstruction { .. })
                        )),
                        0x1a..=0x1f => assert!(matches!(
                            result,
                            Err(DecodeError::UnsupportedInstruction { .. })
                        )),
                        _ => panic!("unclassified opcode {opcode}"),
                    }
                }
            }
        }
    }
    for word in [0x8000_0000, 0xffff_ffff] {
        assert!(matches!(
            decode(word),
            Err(DecodeError::UnsupportedInstruction { .. })
        ));
    }
}

#[test]
fn decode_extracts_immediate_register_mov_and_ldi_forms() {
    let immediate = decode(encoding::immediate(
        InstructionClass::Add,
        2,
        Condition::NotZero,
        -131_072,
    ))
    .unwrap();
    assert_eq!(
        immediate.operand_b,
        zipcpu_simulator::OperandB::Immediate(-131_072)
    );
    let register = decode(encoding::register(
        InstructionClass::Sub,
        5,
        Condition::Carry,
        15,
        -8192,
    ))
    .unwrap();
    assert_eq!(
        register.operand_b,
        zipcpu_simulator::OperandB::Register {
            register: 15,
            immediate: -8192
        }
    );
    let mov = decode(encoding::mov(1, true, Condition::Zero, 2, true, -4096)).unwrap();
    assert!(mov.destination_user && mov.source_user);
    assert_eq!(mov.condition, Condition::Zero);
    let ldi = decode(encoding::ldi(3, -4_194_304)).unwrap();
    assert_eq!(ldi.condition, Condition::Always);
    assert_eq!(
        ldi.operand_b,
        zipcpu_simulator::OperandB::Immediate(-4_194_304)
    );

    for opcode in 0_u32..=0x17 {
        if opcode == InstructionClass::Mov as u32 {
            continue;
        }
        for immediate in [-131_072, 131_071] {
            let word = (1 << 27) | (opcode << 22) | ((immediate as u32) & 0x3_ffff);
            assert_eq!(
                decode(word).unwrap().operand_b,
                zipcpu_simulator::OperandB::Immediate(immediate),
                "opcode {opcode:#x}"
            );
        }
        for source in [0_u32, 13, 14, 15] {
            for immediate in [-8192, 8191] {
                let word = (1 << 27)
                    | (opcode << 22)
                    | (1 << 18)
                    | (source << 14)
                    | ((immediate as u32) & 0x3fff);
                assert_eq!(
                    decode(word).unwrap().operand_b,
                    zipcpu_simulator::OperandB::Register {
                        register: source as u8,
                        immediate,
                    },
                    "opcode {opcode:#x}"
                );
            }
        }
    }
    for destination_user in [false, true] {
        for source_user in [false, true] {
            for immediate in [-4096, 4095] {
                let decoded = decode(encoding::mov(
                    13,
                    destination_user,
                    Condition::NotCarry,
                    15,
                    source_user,
                    immediate,
                ))
                .unwrap();
                assert_eq!(
                    decoded.operand_b,
                    zipcpu_simulator::OperandB::Register {
                        register: 15,
                        immediate,
                    }
                );
                assert_eq!(decoded.destination_user, destination_user);
                assert_eq!(decoded.source_user, source_user);
            }
        }
    }
    for immediate in [-4_194_304, 4_194_303] {
        assert_eq!(
            decode(encoding::ldi(13, immediate)).unwrap().operand_b,
            zipcpu_simulator::OperandB::Immediate(immediate)
        );
    }
}

#[test]
fn every_admitted_family_executes_boundary_forms_with_complete_traces() {
    let classes = [
        InstructionClass::Sub,
        InstructionClass::And,
        InstructionClass::Add,
        InstructionClass::Or,
        InstructionClass::Xor,
        InstructionClass::Lsr,
        InstructionClass::Lsl,
        InstructionClass::Asr,
        InstructionClass::Brev,
        InstructionClass::Ldilo,
        InstructionClass::Mpyuhi,
        InstructionClass::Mpyshi,
        InstructionClass::Mpy,
        InstructionClass::Mov,
        InstructionClass::Divu,
        InstructionClass::Divs,
        InstructionClass::Cmp,
        InstructionClass::Tst,
        InstructionClass::Lw,
        InstructionClass::Sw,
        InstructionClass::Lh,
        InstructionClass::Sh,
        InstructionClass::Lb,
        InstructionClass::Sb,
    ];
    for class in classes {
        for destination in [0_u8, 13, 14, 15] {
            if class != InstructionClass::Mov {
                for immediate in [-131_072, 131_071] {
                    let word =
                        encoding::immediate(class, destination, Condition::Always, immediate);
                    let mut cpu = cpu_with(&[word]);
                    let trace = cpu.step().unwrap();
                    assert_eq!(trace.raw_word, word, "{class:?} R{destination}");
                    assert_eq!(trace.state_after, *cpu.state());
                }
            }
            for immediate in [-8192, 8191] {
                let word = if class == InstructionClass::Mov {
                    encoding::mov(
                        destination,
                        false,
                        Condition::Always,
                        destination,
                        false,
                        immediate,
                    )
                } else {
                    encoding::register(
                        class,
                        destination,
                        Condition::Always,
                        destination,
                        immediate,
                    )
                };
                let mut cpu = cpu_with(&[word]);
                let trace = cpu.step().unwrap();
                assert_eq!(trace.raw_word, word, "{class:?} R{destination}");
                assert_eq!(trace.state_after, *cpu.state());
            }
        }
    }
    for destination in [0_u8, 13, 14, 15] {
        for immediate in [-4_194_304, 4_194_303] {
            let word = encoding::ldi(destination, immediate);
            let mut cpu = cpu_with(&[word]);
            let trace = cpu.step().unwrap();
            assert_eq!(trace.raw_word, word);
            assert_eq!(trace.state_after, *cpu.state());
        }
    }
}

#[test]
fn arithmetic_flags_cover_sign_carry_borrow_and_overflow_edges() {
    let values = [0, 1, 2, 0x7fff_ffff, 0x8000_0000, 0xffff_fffe, 0xffff_ffff];
    for class in [
        InstructionClass::Add,
        InstructionClass::Sub,
        InstructionClass::Cmp,
    ] {
        for a in values {
            for b in values {
                let mut cpu = cpu_with(&[
                    encoding::register(class, 1, Condition::Always, 2, 0),
                    encoding::mov_to_cc(CC_SLEEP),
                ]);
                set_register(&mut cpu, Bank::Supervisor, 1, a);
                set_register(&mut cpu, Bank::Supervisor, 2, b);
                cpu.step().unwrap();
                let result = a.wrapping_add(if class == InstructionClass::Add {
                    b
                } else {
                    b.wrapping_neg()
                });
                let overflow = if class == InstructionClass::Add {
                    (!(a ^ b) & (a ^ result) & 0x8000_0000) != 0
                } else {
                    ((a ^ b) & (a ^ result) & 0x8000_0000) != 0
                };
                let cc = cpu.state().supervisor_cc;
                assert_eq!(cc & CC_Z != 0, result == 0);
                assert_eq!(
                    cc & CC_C != 0,
                    if class == InstructionClass::Add {
                        a.overflowing_add(b).1
                    } else {
                        a < b
                    }
                );
                assert_eq!(cc & CC_V != 0, overflow);
                assert_eq!(cc & CC_N != 0, (result & 0x8000_0000 != 0) ^ overflow);
                if class != InstructionClass::Cmp {
                    assert_eq!(cpu.state().supervisor_gpr[1], result);
                } else {
                    assert_eq!(cpu.state().supervisor_gpr[1], a);
                }
            }
        }
    }
}

#[test]
fn logic_conditions_and_conditional_flag_rules_are_complete() {
    for flag_bits in 0_u32..16 {
        for selector in [
            Condition::Always,
            Condition::Zero,
            Condition::LessThan,
            Condition::Carry,
            Condition::Overflow,
            Condition::NotZero,
            Condition::GreaterEqual,
            Condition::NotCarry,
        ] {
            let expected = match selector {
                Condition::Always => true,
                Condition::Zero => flag_bits & CC_Z != 0,
                Condition::LessThan => flag_bits & CC_N != 0,
                Condition::Carry => flag_bits & CC_C != 0,
                Condition::Overflow => flag_bits & CC_V != 0,
                Condition::NotZero => flag_bits & CC_Z == 0,
                Condition::GreaterEqual => flag_bits & CC_N == 0,
                Condition::NotCarry => flag_bits & CC_C == 0,
            };
            let mut cpu = cpu_with(&[encoding::immediate(
                InstructionClass::Xor,
                1,
                selector,
                0xffff,
            )]);
            let mut state = cpu.snapshot().unwrap();
            state.supervisor_gpr[1] = 0x1234;
            state.supervisor_cc = CC_CPU_INFO | flag_bits;
            cpu.restore(&state).unwrap();
            cpu.step().unwrap();
            assert_eq!(cpu.state().supervisor_gpr[1] != 0x1234, expected);
            if selector != Condition::Always {
                assert_eq!(cpu.state().supervisor_cc & 0xf, flag_bits);
            }
        }
    }
    let mut cpu = cpu_with(&[encoding::immediate(
        InstructionClass::Tst,
        1,
        Condition::Zero,
        0,
    )]);
    let mut state = cpu.snapshot().unwrap();
    state.supervisor_gpr[1] = 0x1234;
    state.supervisor_cc |= CC_Z;
    cpu.restore(&state).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().supervisor_cc & CC_Z, CC_Z);
}

#[test]
fn shifts_cover_zero_one_31_32_63_and_full_register_counts() {
    for class in [
        InstructionClass::Lsr,
        InstructionClass::Lsl,
        InstructionClass::Asr,
    ] {
        for count in [0, 1, 31, 32, 63, u32::MAX] {
            for value in [0, 1, 0x8000_0001, u32::MAX] {
                let mut cpu = cpu_with(&[encoding::register(class, 1, Condition::Always, 2, 0)]);
                set_register(&mut cpu, Bank::Supervisor, 1, value);
                set_register(&mut cpu, Bank::Supervisor, 2, count);
                cpu.step().unwrap();
                let expected = match class {
                    InstructionClass::Lsr => {
                        if count >= 32 {
                            0
                        } else {
                            value >> count
                        }
                    }
                    InstructionClass::Lsl => {
                        if count >= 32 {
                            0
                        } else {
                            value << count
                        }
                    }
                    InstructionClass::Asr => {
                        if count >= 32 {
                            if value & 0x8000_0000 != 0 {
                                u32::MAX
                            } else {
                                0
                            }
                        } else {
                            ((value as i32) >> count) as u32
                        }
                    }
                    _ => unreachable!(),
                };
                assert_eq!(cpu.state().supervisor_gpr[1], expected);
            }
        }
    }
}

#[test]
fn multiply_divide_brev_ldilo_and_divide_fault_edges() {
    let cases = [
        (InstructionClass::Mpy, 0xffff_ffff, 2, 0xffff_fffe),
        (InstructionClass::Mpyuhi, 0xffff_ffff, 2, 1),
        (InstructionClass::Mpyshi, 0xffff_ffff, 2, 0xffff_ffff),
        (InstructionClass::Brev, 0, 1, 0x8000_0000),
        (InstructionClass::Ldilo, 0xaaaa_5555, 0x1234, 0xaaaa_1234),
    ];
    for (class, a, b, expected) in cases {
        let mut cpu = cpu_with(&[encoding::register(class, 1, Condition::Always, 2, 0)]);
        set_register(&mut cpu, Bank::Supervisor, 1, a);
        set_register(&mut cpu, Bank::Supervisor, 2, b);
        cpu.step().unwrap();
        assert_eq!(cpu.state().supervisor_gpr[1], expected, "{class:?}");
    }
    for (class, a, b, expected, carry) in [
        (InstructionClass::Divu, 7, 3, 2, false),
        (InstructionClass::Divu, 6, 3, 2, true),
        (
            InstructionClass::Divs,
            (-7_i32) as u32,
            3,
            (-2_i32) as u32,
            false,
        ),
        (
            InstructionClass::Divs,
            0x8000_0000,
            u32::MAX,
            0x8000_0000,
            true,
        ),
    ] {
        let mut cpu = cpu_with(&[encoding::register(class, 1, Condition::Always, 2, 0)]);
        set_register(&mut cpu, Bank::Supervisor, 1, a);
        set_register(&mut cpu, Bank::Supervisor, 2, b);
        cpu.step().unwrap();
        assert_eq!(cpu.state().supervisor_gpr[1], expected);
        assert_eq!(cpu.state().supervisor_cc & CC_C != 0, carry);
    }
    let mut cpu = cpu_with(&[encoding::immediate(
        InstructionClass::Divu,
        1,
        Condition::Always,
        0,
    )]);
    set_register(&mut cpu, Bank::Supervisor, 1, 99);
    let trace = cpu.step().unwrap();
    assert_eq!(trace.cause, Some(FaultCause::DivideByZero));
    assert_eq!(trace.stop_reason, StopReason::ExternalBreak);
    assert_eq!(cpu.state().supervisor_gpr[1], 99);
    assert_ne!(cpu.state().supervisor_cc & CC_DIV_ERROR, 0);
}

#[test]
fn big_endian_memory_widths_overlaps_and_self_modification_are_exact() {
    let words = [
        encoding::register(InstructionClass::Sw, 1, Condition::Always, 2, 0),
        encoding::register(InstructionClass::Lh, 3, Condition::Always, 2, 0),
        encoding::register(InstructionClass::Lb, 4, Condition::Always, 2, 3),
    ];
    let mut cpu = cpu_with(&words);
    set_register(&mut cpu, Bank::Supervisor, 1, 0x1234_abcd);
    set_register(&mut cpu, Bank::Supervisor, 2, 0x200);
    cpu.step().unwrap();
    assert_eq!(cpu.read_word(0x200).unwrap(), 0x1234_abcd);
    cpu.step().unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().supervisor_gpr[3], 0x1234);
    assert_eq!(cpu.state().supervisor_gpr[4], 0xcd);

    let replacement = encoding::ldi(5, 77);
    let mut cpu = cpu_with(&[encoding::register(
        InstructionClass::Sw,
        1,
        Condition::Always,
        2,
        0,
    )]);
    set_register(&mut cpu, Bank::Supervisor, 1, replacement);
    set_register(&mut cpu, Bank::Supervisor, 2, ORIGIN);
    let trace = cpu.step().unwrap();
    assert_eq!(
        trace.raw_word,
        encoding::register(InstructionClass::Sw, 1, Condition::Always, 2, 0)
    );
    assert_eq!(cpu.read_word(ORIGIN).unwrap(), replacement);
}

#[test]
fn every_memory_width_residue_endpoint_and_transaction_lane_is_checked() {
    let classes = [
        (InstructionClass::Lw, InstructionClass::Sw, 4_u32),
        (InstructionClass::Lh, InstructionClass::Sh, 2),
        (InstructionClass::Lb, InstructionClass::Sb, 1),
    ];
    for (load, store, width) in classes {
        for residue in 0..4_u32 {
            let address = 0x200 + residue;
            for class in [load, store] {
                let mut cpu = cpu_with(&[encoding::register(class, 1, Condition::Always, 2, 0)]);
                set_register(&mut cpu, Bank::Supervisor, 1, 0x89ab_cdef);
                set_register(&mut cpu, Bank::Supervisor, 2, address);
                let trace = cpu.step().unwrap();
                let aligned = width == 1 || address.is_multiple_of(width);
                assert_eq!(
                    trace.cause.is_none(),
                    aligned,
                    "{class:?} residue {residue}"
                );
                assert_eq!(trace.transactions.len(), 2);
                assert_eq!(trace.transactions[1].width, width as u8);
                assert_eq!(trace.transactions[1].success, aligned);
                if aligned {
                    let expected_enable = match width {
                        4 => 0x0f,
                        2 if address & 2 == 0 => 0x0c,
                        2 => 0x03,
                        1 => 1 << (3 - residue),
                        _ => unreachable!(),
                    };
                    assert_eq!(trace.transactions[1].byte_enable, expected_enable);
                } else {
                    assert_eq!(trace.cause, Some(FaultCause::DataBusError));
                }
            }
        }
    }

    for (store, load, address, expected) in [
        (
            InstructionClass::Sw,
            InstructionClass::Lw,
            0xfffc,
            0x89ab_cdef,
        ),
        (
            InstructionClass::Sh,
            InstructionClass::Lh,
            0xfffe,
            0x0000_cdef,
        ),
        (
            InstructionClass::Sb,
            InstructionClass::Lb,
            0xffff,
            0x0000_00ef,
        ),
    ] {
        let mut cpu = cpu_with(&[
            encoding::register(store, 1, Condition::Always, 2, 0),
            encoding::register(load, 3, Condition::Always, 2, 0),
        ]);
        set_register(&mut cpu, Bank::Supervisor, 1, 0x89ab_cdef);
        set_register(&mut cpu, Bank::Supervisor, 2, address);
        assert!(cpu.step().unwrap().cause.is_none());
        assert!(cpu.step().unwrap().cause.is_none());
        assert_eq!(cpu.state().supervisor_gpr[3], expected);
    }
}

#[test]
fn memory_pc_cc_destinations_and_register_sources_follow_commit_order() {
    let mut cpu = cpu_with(&[
        encoding::register(InstructionClass::Lw, 14, Condition::Always, 2, 0),
        encoding::register(InstructionClass::Lw, 15, Condition::Always, 2, 4),
    ]);
    set_register(&mut cpu, Bank::Supervisor, 2, 0x200);
    cpu.write_word(0x200, CC_RESERVED_MASK | CC_BREAK | CC_SLEEP)
        .unwrap();
    cpu.write_word(0x204, 0x303).unwrap();
    let cc_trace = cpu.step().unwrap();
    assert_eq!(
        cpu.state().supervisor_cc & CC_RESERVED_MASK,
        CC_RESERVED_MASK
    );
    assert_ne!(cpu.state().supervisor_cc & CC_BREAK, 0);
    assert_eq!(cc_trace.stop_reason, StopReason::Halted);

    let mut state = cpu.snapshot().unwrap();
    state.supervisor_cc &= !CC_SLEEP;
    state.user_cc &= !CC_SLEEP;
    cpu.restore(&state).unwrap();
    let pc_trace = cpu.step().unwrap();
    assert_eq!(pc_trace.selected_pc, 0x300);
    assert_eq!(cpu.state().supervisor_pc, 0x300);

    let mut cpu = cpu_with(&[encoding::register(
        InstructionClass::Sw,
        14,
        Condition::Always,
        2,
        0,
    )]);
    set_register(&mut cpu, Bank::Supervisor, 2, 0x200);
    cpu.step().unwrap();
    assert_eq!(cpu.read_word(0x200).unwrap(), CC_CPU_INFO);
}

#[test]
fn data_alignment_precedes_range_and_faults_roll_back_effects() {
    for (class, address, request_value) in [
        (InstructionClass::Lw, 0xffff, 0),
        (InstructionClass::Lh, 0xffff, 0),
        (InstructionClass::Sw, 0x1_0000, 0xdead_beef),
        (InstructionClass::Sh, 0xffff, 0x0000_beef),
        (InstructionClass::Sb, 0x1_0000, 0x0000_00ef),
    ] {
        let mut cpu = cpu_with(&[encoding::register(class, 1, Condition::Always, 2, 0)]);
        set_register(&mut cpu, Bank::Supervisor, 1, 0xdead_beef);
        set_register(&mut cpu, Bank::Supervisor, 2, address);
        let before_memory = cpu.state().memory.clone();
        let trace = cpu.step().unwrap();
        assert_eq!(trace.cause, Some(FaultCause::DataBusError));
        assert_eq!(cpu.state().supervisor_gpr[1], 0xdead_beef);
        assert_eq!(cpu.state().memory, before_memory);
        assert!(!trace.transactions.last().unwrap().success);
        assert_eq!(trace.transactions.last().unwrap().value, request_value);
    }
}

#[test]
fn false_memory_condition_skips_address_validation() {
    let mut cpu = cpu_with(&[encoding::register(
        InstructionClass::Lw,
        1,
        Condition::Zero,
        2,
        0,
    )]);
    set_register(&mut cpu, Bank::Supervisor, 2, 3);
    let trace = cpu.step().unwrap();
    assert!(!trace.condition_met);
    assert_eq!(trace.transactions.len(), 1);
    assert_eq!(cpu.state().supervisor_pc, ORIGIN + 4);
}

#[test]
fn illegal_decode_precedes_false_condition() {
    let illegal_fpu = (1_u32 << 27) | (0x1a << 22) | (Condition::Zero as u32) << 19;
    let mut cpu = cpu_with(&[illegal_fpu]);
    let trace = cpu.step().unwrap();
    assert_eq!(trace.cause, Some(FaultCause::IllegalInstruction));
    assert_eq!(trace.stop_reason, StopReason::ExternalBreak);
    assert_eq!(cpu.state().supervisor_pc, ORIGIN);
    assert_ne!(cpu.state().supervisor_cc & CC_ILLEGAL, 0);
}

#[test]
fn pc_writes_commit_before_next_fetch_classification() {
    for target in [0x200, 0x1_0000] {
        let mut cpu = cpu_with(&[encoding::ldi(15, target as i32)]);
        let trace = cpu.step().unwrap();
        assert_eq!(trace.selected_pc, target);
        assert_eq!(cpu.state().supervisor_pc, target);
        if target < MEMORY_SIZE as u32 {
            assert_eq!(
                cpu.step(),
                Err(ZipCpuError::FetchOutsideProgram { pc: target })
            );
            assert_eq!(cpu.state().supervisor_pc, target);
        } else {
            let next = cpu.step().unwrap();
            assert_eq!(next.cause, Some(FaultCause::IllegalFetch));
            assert_eq!(next.stop_reason, StopReason::ExternalBreak);
        }
    }
    let mut cpu = cpu_with(&[encoding::ldi(15, 0x203)]);
    cpu.step().unwrap();
    assert_eq!(cpu.state().supervisor_pc, 0x200);
}

#[test]
fn run_rolls_back_committed_branch_when_following_fetch_is_typed() {
    let mut cpu = cpu_with(&[encoding::ldi(15, 0x200)]);
    let before = cpu.snapshot().unwrap();
    assert_eq!(
        cpu.run(2),
        Err(ZipCpuError::FetchOutsideProgram { pc: 0x200 })
    );
    assert_eq!(cpu.snapshot().unwrap(), before);
}

#[test]
fn mov_cross_bank_and_pc_pipeline_rules_are_read_before_write() {
    let mut cpu = cpu_with(&[
        encoding::mov(2, true, Condition::Always, 1, false, 4),
        encoding::mov(15, true, Condition::Always, 15, false, 1),
    ]);
    set_register(&mut cpu, Bank::Supervisor, 1, 10);
    cpu.step().unwrap();
    assert_eq!(cpu.state().user_gpr[2], 14);
    cpu.step().unwrap();
    assert_eq!(cpu.state().user_pc, (ORIGIN + 8).wrapping_add(4));
    assert_eq!(cpu.state().supervisor_pc, ORIGIN + 8);
}

#[test]
fn pc_as_a_and_b_uses_pipeline_scaling_and_conditional_commit_rules() {
    let mut cpu = cpu_with(&[encoding::immediate(
        InstructionClass::Add,
        15,
        Condition::Always,
        4,
    )]);
    let trace = cpu.step().unwrap();
    assert_eq!(trace.sources[0].value, ORIGIN + 4);
    assert_eq!(cpu.state().supervisor_pc, ORIGIN + 8);

    let mut cpu = cpu_with(&[encoding::mov(1, false, Condition::Always, 15, false, 1)]);
    cpu.step().unwrap();
    assert_eq!(cpu.state().supervisor_gpr[1], ORIGIN + 8);

    let mut cpu = cpu_with(&[encoding::mov(1, false, Condition::Always, 15, true, 1)]);
    let mut state = cpu.snapshot().unwrap();
    state.user_pc = 0x400;
    cpu.restore(&state).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().supervisor_gpr[1], 0x404);

    let mut cpu = cpu_with(&[encoding::ldi(15, 0x400) | (1 << 19)]);
    let trace = cpu.step().unwrap();
    assert_eq!(trace.selected_pc, 0x8_0400, "LDI ignores condition bits");

    let mut cpu = cpu_with(&[encoding::immediate(
        InstructionClass::Add,
        15,
        Condition::Zero,
        0x100,
    )]);
    let trace = cpu.step().unwrap();
    assert!(!trace.condition_met);
    assert_eq!(cpu.state().supervisor_pc, ORIGIN + 4);
}

#[test]
fn reserved_cc_fields_are_banked_restorable_and_reset_by_load() {
    let mut cpu = cpu_with(&[encoding::mov_to_cc(CC_SLEEP)]);
    for supervisor in [0_u32, 1, 0x2a, 0x7f] {
        for user in [0_u32, 1, 0x55, 0x7f] {
            let mut state = cpu.snapshot().unwrap();
            state.supervisor_cc = CC_CPU_INFO | (supervisor << 16);
            state.user_cc = CC_CPU_INFO | CC_GIE | (user << 16);
            cpu.restore(&state).unwrap();
            let roundtrip = cpu.snapshot().unwrap();
            assert_eq!(roundtrip.supervisor_cc & CC_RESERVED_MASK, supervisor << 16);
            assert_eq!(roundtrip.user_cc & CC_RESERVED_MASK, user << 16);
        }
    }
    cpu.load(&image(&[encoding::mov_to_cc(CC_SLEEP)]), ORIGIN)
        .unwrap();
    assert_eq!(cpu.state().supervisor_cc & CC_RESERVED_MASK, 0);
    assert_eq!(cpu.state().user_cc & CC_RESERVED_MASK, 0);
}

#[test]
fn every_cc_input_bit_obeys_privilege_bank_and_fixed_field_rules() {
    for bit in 0..32_u32 {
        let value = 1_u32 << bit;
        let mut cpu = cpu_with(&[encoding::mov(14, false, Condition::Always, 1, false, 0)]);
        set_register(&mut cpu, Bank::Supervisor, 1, value);
        cpu.step().unwrap();
        if bit == 5 {
            assert_eq!(cpu.state().mode, Mode::User);
            assert_eq!(cpu.state().supervisor_cc, CC_CPU_INFO);
            assert_eq!(cpu.state().user_cc, CC_CPU_INFO | CC_GIE);
            continue;
        }
        let writable = CC_RESERVED_MASK | CC_Z | CC_C | CC_N | CC_V | CC_BREAK | CC_SLEEP;
        let expected_supervisor = CC_CPU_INFO | (value & writable);
        let expected_user = CC_CPU_INFO | CC_GIE | (value & CC_SLEEP);
        assert_eq!(cpu.state().supervisor_cc, expected_supervisor, "bit {bit}");
        assert_eq!(cpu.state().user_cc, expected_user, "bit {bit}");

        let mut cpu = cpu_with(&[encoding::mov(14, false, Condition::Always, 1, false, 0)]);
        let mut state = cpu.snapshot().unwrap();
        state.mode = Mode::User;
        state.user_pc = ORIGIN;
        state.user_gpr[1] = CC_GIE | value;
        cpu.restore(&state).unwrap();
        cpu.step().unwrap();
        let user_writable = CC_RESERVED_MASK | CC_Z | CC_C | CC_N | CC_V | CC_SLEEP;
        assert_eq!(cpu.state().mode, Mode::User, "bit {bit}");
        assert_eq!(
            cpu.state().user_cc,
            CC_CPU_INFO | CC_GIE | ((CC_GIE | value) & user_writable),
            "user bit {bit}"
        );
        assert_eq!(
            cpu.state().supervisor_cc,
            CC_CPU_INFO | (value & CC_SLEEP),
            "shared bit {bit}"
        );
    }

    for retained in [0, CC_BREAK, CC_ILLEGAL, CC_BUS_ERROR, CC_DIV_ERROR, CC_TRAP] {
        for keep in [false, true] {
            let mut cpu = cpu_with(&[encoding::mov(14, true, Condition::Always, 1, false, 0)]);
            let mut state = cpu.snapshot().unwrap();
            state.user_cc |= retained;
            if retained == CC_TRAP {
                state.supervisor_cc |= CC_TRAP;
            }
            state.supervisor_gpr[1] = if keep { retained } else { 0 };
            cpu.restore(&state).unwrap();
            cpu.step().unwrap();
            assert_eq!(
                cpu.state().user_cc & retained,
                if keep { retained } else { 0 }
            );
            if retained == CC_TRAP {
                assert_eq!(
                    cpu.state().supervisor_cc & CC_TRAP,
                    if keep { CC_TRAP } else { 0 }
                );
            }
        }
    }
}

#[test]
fn restore_rejects_each_noncanonical_complete_state_atomically() {
    let mut cpu = cpu_with(&[encoding::mov_to_cc(CC_SLEEP)]);
    let before = cpu.snapshot().unwrap();
    let mut candidates = Vec::new();
    let mut bad = before.clone();
    bad.memory.pop();
    candidates.push(bad);
    let mut bad = before.clone();
    bad.supervisor_pc |= 2;
    candidates.push(bad);
    let mut bad = before.clone();
    bad.supervisor_cc ^= 1 << 31;
    candidates.push(bad);
    let mut bad = before.clone();
    bad.user_cc &= !CC_GIE;
    candidates.push(bad);
    let mut bad = before.clone();
    bad.user_cc |= 1 << 13;
    candidates.push(bad);
    let mut bad = before.clone();
    bad.user_cc ^= CC_TRAP;
    candidates.push(bad);
    let mut bad = before.clone();
    bad.image_origin = None;
    candidates.push(bad);
    for candidate in candidates {
        assert!(matches!(
            cpu.restore(&candidate),
            Err(ZipCpuError::InvalidState { .. })
        ));
        assert_eq!(cpu.snapshot().unwrap(), before);
    }
}

#[test]
fn rtu_trap_interrupt_wait_halt_step_and_break_boundaries() {
    let mut cpu = cpu_with(&[encoding::return_to_user()]);
    let mut state = cpu.snapshot().unwrap();
    state.user_pc = ORIGIN;
    cpu.restore(&state).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.mode_after, Mode::User);

    let mut state = cpu.snapshot().unwrap();
    state.user_cc |= CC_STEP;
    cpu.restore(&state).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.stop_reason, StopReason::UserStep);
    assert_eq!(cpu.state().mode, Mode::Supervisor);

    let mut cpu = cpu_with(&[encoding::return_to_user()]);
    let mut state = cpu.snapshot().unwrap();
    state.user_pc = ORIGIN;
    cpu.restore(&state).unwrap();
    cpu.set_interrupt(true).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().mode, Mode::Supervisor);

    let mut cpu = cpu_with(&[encoding::mov_to_cc(CC_SLEEP)]);
    assert_eq!(cpu.step().unwrap().stop_reason, StopReason::Halted);
    assert_eq!(cpu.step(), Err(ZipCpuError::Halted));

    let mut cpu = cpu_with(&[encoding::mov_to_cc(CC_SLEEP)]);
    let mut state = cpu.snapshot().unwrap();
    state.mode = Mode::User;
    state.user_pc = ORIGIN;
    cpu.restore(&state).unwrap();
    assert_eq!(
        cpu.step().unwrap().stop_reason,
        StopReason::WaitingForInterrupt
    );
    assert_eq!(cpu.step(), Err(ZipCpuError::WaitingForInterrupt));
    cpu.set_interrupt(true).unwrap();
    assert_eq!(cpu.step().unwrap().stop_reason, StopReason::Interrupt);

    let mut cpu = cpu_with(&[encoding::special(InstructionClass::Break, 0x155)]);
    assert_eq!(cpu.step().unwrap().stop_reason, StopReason::ExternalBreak);
}

#[test]
fn interrupt_masking_break_enable_noop_and_fetch_faults_are_exact() {
    let mut cpu = cpu_with(&[
        encoding::special(InstructionClass::Noop, 0x3f_ffff),
        encoding::return_to_user(),
    ]);
    cpu.set_interrupt(true).unwrap();
    let noop = cpu.step().unwrap();
    assert_eq!(noop.mnemonic, "NOOP");
    assert_eq!(cpu.state().mode, Mode::Supervisor);
    let suppressed_rtu = cpu.step().unwrap();
    assert_eq!(suppressed_rtu.stop_reason, StopReason::Running);
    assert_eq!(cpu.state().mode, Mode::Supervisor);

    let mut cpu = cpu_with(&[encoding::special(InstructionClass::Break, 0)]);
    let mut state = cpu.snapshot().unwrap();
    state.mode = Mode::User;
    state.user_pc = ORIGIN;
    state.supervisor_cc |= CC_BREAK;
    cpu.restore(&state).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.stop_reason, StopReason::ExternalBreak);
    assert_eq!(cpu.state().mode, Mode::Supervisor);
    assert_ne!(cpu.state().user_cc & CC_BREAK, 0);

    let mut cpu = ZipCpuSimulator::new().unwrap();
    let before = cpu.state().clone();
    assert_eq!(cpu.step(), Err(ZipCpuError::NoProgramLoaded));
    assert_eq!(cpu.state(), &before);

    let mut cpu = cpu_with(&[encoding::ldi(15, 0x1_0000)]);
    cpu.step().unwrap();
    let illegal_fetch = cpu.step().unwrap();
    assert_eq!(illegal_fetch.cause, Some(FaultCause::IllegalFetch));
    assert_eq!(illegal_fetch.transactions.len(), 1);
    assert!(!illegal_fetch.transactions[0].success);
}

#[test]
fn public_memory_helpers_check_alignment_range_and_byte_order() {
    let mut cpu = ZipCpuSimulator::new().unwrap();
    cpu.write_word(0x100, 0x1234_5678).unwrap();
    assert_eq!(cpu.read_halfword(0x100).unwrap(), 0x1234);
    assert_eq!(cpu.read_halfword(0x102).unwrap(), 0x5678);
    assert_eq!(cpu.read_byte(0x103).unwrap(), 0x78);
    let before = cpu.snapshot().unwrap();
    assert!(matches!(
        cpu.write_word(1, 0),
        Err(ZipCpuError::MisalignedData { .. })
    ));
    assert!(matches!(
        cpu.write_halfword(0xffff, 0),
        Err(ZipCpuError::MisalignedData { .. })
    ));
    assert!(matches!(
        cpu.write_byte(0x1_0000, 0),
        Err(ZipCpuError::DataOutOfRange { .. })
    ));
    assert_eq!(cpu.snapshot().unwrap(), before);
}

#[test]
fn allocation_seams_are_real_and_all_atomic() {
    let cpu = cpu_with(&[encoding::ldi(1, 9)]);
    let before = cpu.state().clone();
    cpu.fail_allocation_at(AllocationPoint::SnapshotState, 0);
    assert_eq!(cpu.snapshot(), Err(ZipCpuError::AllocationFailed));
    assert_eq!(cpu.state(), &before);

    let mut cpu = cpu_with(&[encoding::ldi(1, 9)]);
    let before = cpu.snapshot().unwrap();
    let mut replacement = before.clone();
    replacement.supervisor_gpr[1] = 99;
    cpu.fail_allocation_at(AllocationPoint::RestoreStaging, 0);
    assert_eq!(
        cpu.restore(&replacement),
        Err(ZipCpuError::AllocationFailed)
    );
    assert_eq!(cpu.state(), &before);

    let mut cpu = cpu_with(&[encoding::ldi(1, 9)]);
    let before = cpu.snapshot().unwrap();
    cpu.fail_allocation_at(AllocationPoint::LoadImage, 0);
    assert_eq!(
        cpu.load(&image(&[encoding::ldi(2, 10)]), ORIGIN),
        Err(ZipCpuError::AllocationFailed)
    );
    assert_eq!(cpu.state(), &before);

    let step_points = [
        AllocationPoint::StepEntryState,
        AllocationPoint::StepCandidateState,
        AllocationPoint::FetchedBytes,
        AllocationPoint::Mnemonic,
        AllocationPoint::Sources,
        AllocationPoint::Transactions,
        AllocationPoint::Effects,
        AllocationPoint::TraceState,
    ];
    for point in step_points {
        let mut cpu = cpu_with(&[encoding::register(
            InstructionClass::Add,
            1,
            Condition::Always,
            2,
            0,
        )]);
        set_register(&mut cpu, Bank::Supervisor, 1, 7);
        set_register(&mut cpu, Bank::Supervisor, 2, 8);
        let before = cpu.snapshot().unwrap();
        cpu.fail_allocation_at(point, 0);
        assert_eq!(cpu.step(), Err(ZipCpuError::AllocationFailed), "{point:?}");
        assert_eq!(cpu.state(), &before, "{point:?}");
    }

    let program = image(&[
        encoding::ldi(1, 1),
        encoding::ldi(2, 2),
        encoding::mov_to_cc(CC_SLEEP),
    ]);
    for point in [
        AllocationPoint::RunEntryState,
        AllocationPoint::RunTraceRetention,
        AllocationPoint::RunFinalState,
    ] {
        let mut cpu = ZipCpuSimulator::new().unwrap();
        cpu.load(&program, ORIGIN).unwrap();
        let before = cpu.snapshot().unwrap();
        cpu.fail_allocation_at(point, 0);
        assert_eq!(cpu.run(3), Err(ZipCpuError::AllocationFailed), "{point:?}");
        assert_eq!(cpu.state(), &before, "{point:?}");
    }

    for point in [
        AllocationPoint::ExecuteEntryState,
        AllocationPoint::ExecuteWrapping,
    ] {
        let mut cpu = ZipCpuSimulator::new().unwrap();
        let before = cpu.snapshot().unwrap();
        cpu.fail_allocation_at(point, 0);
        assert_eq!(
            cpu.execute(&program, ORIGIN, 3).unwrap_err().source,
            ZipCpuError::AllocationFailed,
            "{point:?}"
        );
        assert_eq!(cpu.state(), &before, "{point:?}");
    }
}

#[test]
fn run_and_execute_roll_back_limits_and_source_errors() {
    let mut cpu = cpu_with(&[encoding::ldi(1, 1)]);
    let before = cpu.snapshot().unwrap();
    assert_eq!(cpu.run(0), Err(ZipCpuError::ZeroStepBudget));
    assert!(matches!(
        cpu.run(65),
        Err(ZipCpuError::TraceLimitExceeded { .. })
    ));
    assert!(matches!(
        cpu.run(1),
        Err(ZipCpuError::StepLimitExceeded { .. })
    ));
    assert_eq!(cpu.snapshot().unwrap(), before);

    let error = cpu.execute(&[], ORIGIN, 1).unwrap_err();
    assert_eq!(error.source, ZipCpuError::EmptyProgram);
    assert_eq!(cpu.snapshot().unwrap(), before);
}

#[test]
fn traces_retain_exact_sources_destination_transactions_and_state() {
    let mut cpu = cpu_with(&[encoding::register(
        InstructionClass::Add,
        1,
        Condition::Always,
        2,
        -1,
    )]);
    set_register(&mut cpu, Bank::Supervisor, 1, 10);
    set_register(&mut cpu, Bank::Supervisor, 2, 20);
    let trace = cpu.step().unwrap();
    assert_eq!(trace.raw_bytes, image(&[trace.raw_word]));
    assert_eq!(trace.mnemonic, "ADD");
    assert_eq!(trace.sources.len(), 2);
    assert_eq!(trace.destination.as_ref().unwrap().value, 29);
    assert_eq!(trace.transactions.len(), 1);
    assert_eq!(trace.state_after, *cpu.state());
}

#[test]
fn cc_cpu_info_and_read_zero_bits_cannot_be_overwritten() {
    let mut cpu = cpu_with(&[encoding::ldi(14, -1)]);
    cpu.step().unwrap();
    let cc = cpu.state().supervisor_cc;
    assert_eq!(cc & 0xff80_0000, CC_CPU_INFO);
    assert_eq!(cc & 0xe000, 0);
    assert_eq!(cc & CC_RESERVED_MASK, CC_RESERVED_MASK);
    assert_eq!(cc & CC_GIE, 0);
}

#[test]
fn user_faults_preserve_destination_and_switch_to_retained_supervisor_pc() {
    let illegal = 0x8000_0000;
    let mut cpu = cpu_with(&[illegal]);
    let mut state = cpu.snapshot().unwrap();
    state.mode = Mode::User;
    state.user_pc = ORIGIN;
    state.supervisor_pc = 0x400;
    state.user_gpr[1] = 0x1234;
    cpu.restore(&state).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(
        trace.stop_reason,
        StopReason::FaultToSupervisor(FaultCause::IllegalInstruction)
    );
    assert_eq!(cpu.state().mode, Mode::Supervisor);
    assert_eq!(cpu.state().supervisor_pc, 0x400);
    assert_eq!(cpu.state().user_pc, ORIGIN);
    assert_eq!(cpu.state().user_gpr[1], 0x1234);
    assert_ne!(cpu.state().user_cc & CC_ILLEGAL, 0);
}

#[test]
fn cc_trap_and_exception_status_filtering_is_privilege_safe() {
    let mut cpu = cpu_with(&[encoding::return_to_user()]);
    let mut state = cpu.snapshot().unwrap();
    state.user_pc = ORIGIN;
    state.user_cc |= CC_ILLEGAL | CC_BUS_ERROR | CC_DIV_ERROR | CC_BREAK | CC_TRAP;
    state.supervisor_cc |= CC_TRAP;
    cpu.restore(&state).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().mode, Mode::User);
    assert_eq!(
        cpu.state().user_cc & (CC_ILLEGAL | CC_BUS_ERROR | CC_DIV_ERROR | CC_BREAK | CC_TRAP),
        0
    );

    let mut cpu = cpu_with(&[encoding::immediate(
        InstructionClass::And,
        14,
        Condition::Always,
        !(CC_GIE as i32),
    )]);
    let mut state = cpu.snapshot().unwrap();
    state.mode = Mode::User;
    state.user_pc = ORIGIN;
    cpu.restore(&state).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(
        trace.stop_reason,
        StopReason::FaultToSupervisor(FaultCause::SoftwareTrap)
    );
    assert_ne!(cpu.state().user_cc & CC_TRAP, 0);
}
