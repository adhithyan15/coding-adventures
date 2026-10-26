use superh2_simulator::{
    classify, InstructionClass, PendingTransfer, StatusRegister, SuperH2Error, SuperH2Simulator,
    Width, DEFAULT_ORIGIN, MAX_TRACES, MEMORY_BYTES,
};

fn loaded(program: &[u8]) -> SuperH2Simulator {
    let mut cpu = SuperH2Simulator::new();
    cpu.load(program, DEFAULT_ORIGIN).unwrap();
    cpu
}

fn install_state(
    cpu: &mut SuperH2Simulator,
    update: impl FnOnce(&mut superh2_simulator::SuperH2State),
) {
    let mut state = cpu.snapshot();
    update(&mut state);
    cpu.restore(state).unwrap();
}

#[test]
fn move_arithmetic_logic_compare_and_shift_families_execute() {
    let program = [
        0xe0, 0xff, // MOV #-1,R0
        0xe1, 0x01, // MOV #1,R1
        0x30, 0x1e, // ADDC R1,R0
        0x30, 0x1f, // ADDV R1,R0
        0x20, 0x18, // TST R1,R0
        0x30, 0x13, // CMP/GE R1,R0
        0x40, 0x24, // ROTCL R0
        0x60, 0x1e, // EXTS.B R1,R0
        0x60, 0x18, // SWAP.B R1,R0
        0x00, 0x1b, // SLEEP
    ];
    let mut cpu = loaded(&program);
    let result = cpu.run(10).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 10);
    assert_eq!(result.traces[0].bytes, [0xe0, 0xff]);
    assert_eq!(result.traces[2].mnemonic, "ADDC");
    assert_eq!(result.traces[6].mnemonic, "ROTCL");
    assert_eq!(result.traces[9].post_state, result.final_state);
}

#[test]
fn big_endian_memory_modes_widths_and_postincrement_suppression() {
    let mut cpu = loaded(&[
        0x21, 0x02, // MOV.L R0,@R1
        0x62, 0x12, // MOV.L @R1,R2
        0x63, 0x74, // MOV.B @R7+,R3
        0x61, 0x16, // MOV.L @R1+,R1 -- suppress increment
        0x24, 0x26, // MOV.L R2,@-R4
        0xe0, 0x00, // MOV #0,R0
        0x05, 0x26, // MOV.L R2,@(R0,R5)
        0x06, 0x5e, // MOV.L @(R0,R5),R6
        0x00, 0x1b,
    ]);
    install_state(&mut cpu, |s| {
        s.registers[0] = 0x1122_3344;
        s.registers[1] = 0x0200;
        s.registers[4] = 0x0304;
        s.registers[5] = 0x0400;
        s.registers[7] = 0x0200;
    });
    let result = cpu.run(9).unwrap();
    assert_eq!(
        &result.final_state.memory[0x200..0x204],
        &[0x11, 0x22, 0x33, 0x44]
    );
    assert_eq!(result.final_state.registers[2], 0x1122_3344);
    assert_eq!(result.final_state.registers[3], 0x11);
    assert_eq!(result.final_state.registers[7], 0x0201);
    assert_eq!(result.final_state.registers[1], 0x1122_3344);
    assert_eq!(result.final_state.registers[4], 0x0300);
    assert_eq!(result.final_state.registers[6], 0x1122_3344);
}

#[test]
fn displacement_gbr_pc_relative_and_mova_use_scaled_addresses() {
    let mut program = vec![0_u8; 32];
    for word in program.chunks_exact_mut(2) {
        word.copy_from_slice(&[0x00, 0x09]);
    }
    program.splice(
        0..8,
        [
            0x81, 0x01, // MOV.W R0,@(1,R1)
            0x81, 0x51, // MOV.W @(1,R1),R0
            0xc2, 0x01, // MOV.L R0,@(1,GBR)
            0xc6, 0x01, // MOV.L @(1,GBR),R0
        ],
    );
    program[8..16].copy_from_slice(&[
        0x92, 0x03, // MOV.W @(3,PC),R2 => 0x1012
        0xd3, 0x02, // MOV.L @(2,PC),R3 => 0x1014
        0xc7, 0x02, // MOVA @(2,PC),R0 => 0x1018
        0x00, 0x1b,
    ]);
    program[18..20].copy_from_slice(&0xff80u16.to_be_bytes());
    program[20..24].copy_from_slice(&0x89ab_cdefu32.to_be_bytes());
    let mut cpu = loaded(&program);
    install_state(&mut cpu, |s| {
        s.registers[0] = 0x1234_5678;
        s.registers[1] = 0x0200;
        s.gbr = 0x0300;
    });
    let result = cpu.run(8).unwrap();
    assert_eq!(result.final_state.registers[2], 0xffff_ff80);
    assert_eq!(result.final_state.registers[3], 0x89ab_cdef);
    assert_eq!(result.final_state.registers[0], 0x1018);
}

#[test]
fn multiplication_division_and_mac_alias_order_are_visible() {
    let mut cpu = loaded(&[
        0x01, 0x27, // MUL.L R2,R1
        0x31, 0x2d, // DMULS.L R2,R1
        0x21, 0x27, // DIV0S R2,R1
        0x31, 0x24, // DIV1 R2,R1
        0x43, 0x3f, // MAC.W @R3+,@R3+
        0x04, 0x4f, // MAC.L @R4+,@R4+
        0x00, 0x1b,
    ]);
    install_state(&mut cpu, |s| {
        s.registers[1] = (-3_i32) as u32;
        s.registers[2] = 7;
        s.registers[3] = 0x0200;
        s.registers[4] = 0x0300;
        s.memory[0x200..0x204].copy_from_slice(&[0xff, 0xfe, 0x00, 0x03]);
        s.memory[0x300..0x308].copy_from_slice(&[0xff, 0xff, 0xff, 0xfc, 0, 0, 0, 5]);
    });
    let result = cpu.run(7).unwrap();
    assert_eq!(result.traces[0].post_state.macl, (-21_i32) as u32);
    assert_eq!(result.traces[1].post_state.mach, u32::MAX);
    assert_eq!(result.final_state.registers[3], 0x0204);
    assert_eq!(result.final_state.registers[4], 0x0308);
    assert_eq!(result.traces[3].mnemonic, "DIV1");
}

#[test]
fn unsigned_word_multiply_retains_the_full_32_bit_product() {
    let mut cpu = loaded(&[0x21, 0x2e]);
    install_state(&mut cpu, |s| {
        s.registers[1] = 0xffff;
        s.registers[2] = 0xffff;
    });
    assert_eq!(cpu.step().unwrap().post_state.macl, 0xfffe_0001);
}

#[test]
fn mac_second_source_fault_is_atomic_even_for_alias() {
    let mut cpu = loaded(&[0x01, 0x1f]);
    install_state(&mut cpu, |s| {
        s.registers[1] = (MEMORY_BYTES - 4) as u32;
        s.mach = 0x1234;
        s.macl = 0x5678;
    });
    let before = cpu.snapshot();
    assert!(matches!(
        cpu.step(),
        Err(SuperH2Error::DataOutOfRange { .. })
    ));
    assert_eq!(cpu.snapshot(), before);
}

#[test]
fn delayed_branches_link_and_pc_relative_slot_basis_are_exact() {
    let mut program = vec![0; 0x20];
    program[0..4].copy_from_slice(&[0xa0, 0x06, 0x91, 0x00]); // target 0x1010
    program[0x10..0x14].copy_from_slice(&[0x00, 0x1b, 0x7f, 0x01]);
    let mut cpu = loaded(&program);
    let branch = cpu.step().unwrap();
    assert_eq!(branch.pc_after, DEFAULT_ORIGIN + 2);
    assert_eq!(
        branch.post_state.pending_transfer.unwrap().target,
        DEFAULT_ORIGIN + 0x10
    );
    let slot = cpu.step().unwrap();
    assert!(slot.delay_slot);
    assert_eq!(slot.pc_after, DEFAULT_ORIGIN + 0x10);
    assert_eq!(slot.post_state.registers[1], 0x7f01);
    assert!(cpu.step().unwrap().post_state.halted);

    let mut linked = loaded(&[0xb0, 0x00, 0x00, 0x09, 0x00, 0x1b]);
    let result = linked.run(3).unwrap();
    assert_eq!(result.final_state.pr, DEFAULT_ORIGIN + 4);
}

#[test]
fn conditional_delays_only_when_taken_and_illegal_slot_is_atomic() {
    let mut cpu = loaded(&[0x8d, 0x01, 0xa0, 0x00, 0x00, 0x1b, 0x00, 0x1b]);
    install_state(&mut cpu, |s| s.sr.t = true);
    cpu.step().unwrap();
    let pending = cpu.snapshot();
    assert!(matches!(
        cpu.step(),
        Err(SuperH2Error::IllegalDelaySlot { .. })
    ));
    assert_eq!(cpu.snapshot(), pending);

    let mut not_taken = loaded(&[0x8d, 0x01, 0xe0, 0x02, 0x00, 0x1b]);
    let first = not_taken.step().unwrap();
    assert!(first.post_state.pending_transfer.is_none());
    assert_eq!(not_taken.step().unwrap().post_state.registers[0], 2);
}

#[test]
fn sleep_in_slot_and_delayed_target_faults_preserve_state() {
    let mut sleep = loaded(&[0xa0, 0x00, 0x00, 0x1b, 0x00, 0x09]);
    sleep.step().unwrap();
    let pending = sleep.snapshot();
    assert_eq!(
        sleep.step(),
        Err(SuperH2Error::UnsupportedDelaySlot {
            word: 0x001b,
            pc: DEFAULT_ORIGIN + 2,
        })
    );
    assert_eq!(sleep.snapshot(), pending);

    let mut odd = loaded(&[0x41, 0x2b, 0x00, 0x09]);
    install_state(&mut odd, |s| s.registers[1] = 3);
    odd.step().unwrap();
    odd.step().unwrap();
    let before = odd.snapshot();
    assert!(matches!(
        odd.step(),
        Err(SuperH2Error::MisalignedFetch { .. })
    ));
    assert_eq!(odd.snapshot(), before);
}

#[test]
fn backward_delay_and_slot_fault_roll_back_the_whole_run() {
    let mut backward = loaded(&[0xaf, 0xfe, 0x00, 0x09]);
    backward.step().unwrap();
    let slot = backward.step().unwrap();
    assert_eq!(slot.pc_after, DEFAULT_ORIGIN);

    let mut faulty = loaded(&[0xa0, 0x00, 0xff, 0xff, 0x00, 0x1b]);
    let before = faulty.snapshot();
    assert!(matches!(
        faulty.run(2),
        Err(SuperH2Error::UnknownInstruction { .. })
    ));
    assert_eq!(faulty.snapshot(), before);
}

#[test]
fn run_and_execute_roll_back_every_complete_state_field() {
    let mut cpu = loaded(&[0x70, 0x01, 0xff, 0xff]);
    install_state(&mut cpu, |s| {
        s.registers[0] = 9;
        s.memory[0x200] = 7;
    });
    let before = cpu.snapshot();
    assert!(matches!(
        cpu.run(2),
        Err(SuperH2Error::UnknownInstruction { .. })
    ));
    assert_eq!(cpu.snapshot(), before);

    assert_eq!(cpu.run(0), Err(SuperH2Error::MaxStepsZero));
    assert_eq!(
        cpu.run(MAX_TRACES + 1),
        Err(SuperH2Error::TraceLimitExceeded {
            requested: MAX_TRACES + 1
        })
    );
    assert!(matches!(
        cpu.run(1),
        Err(SuperH2Error::MaxStepsExceeded { .. })
    ));
    assert_eq!(cpu.snapshot(), before);

    assert!(matches!(
        cpu.execute(&[0x00], DEFAULT_ORIGIN, 1),
        Err(SuperH2Error::OddProgramLength { .. })
    ));
    assert_eq!(cpu.snapshot(), before);
}

#[test]
fn restore_transport_fetch_data_and_halt_boundaries_are_checked() {
    let mut cpu = SuperH2Simulator::new();
    assert_eq!(cpu.load(&[], 0), Err(SuperH2Error::EmptyProgram));
    assert!(matches!(
        cpu.load(&[0, 9], 1),
        Err(SuperH2Error::MisalignedProgram { .. })
    ));
    assert!(matches!(
        cpu.load(&[0, 9], MEMORY_BYTES as u32),
        Err(SuperH2Error::ProgramOutOfRange { .. })
    ));

    cpu.load(&[0, 0x1b], DEFAULT_ORIGIN).unwrap();
    let before = cpu.snapshot();
    let mut invalid = before.clone();
    invalid.sr.interrupt_mask = 16;
    assert!(matches!(
        cpu.restore(invalid),
        Err(SuperH2Error::InvalidInterruptMask { .. })
    ));
    assert_eq!(cpu.snapshot(), before);
    let mut invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(invalid),
        Err(SuperH2Error::InvalidMemoryLength { .. })
    ));

    cpu.write_data(0x0200, Width::Long, 0x1234_5678).unwrap();
    assert_eq!(cpu.read_data(0x0200, Width::Long).unwrap(), 0x1234_5678);
    assert!(matches!(
        cpu.read_data(0x0201, Width::Word),
        Err(SuperH2Error::MisalignedData { .. })
    ));
    assert!(cpu.step().unwrap().post_state.halted);
    let halted = cpu.snapshot();
    assert!(matches!(cpu.step(), Err(SuperH2Error::Halted { .. })));
    assert_eq!(cpu.snapshot(), halted);
}

#[test]
fn restore_validates_pending_invariants_and_error_text_is_stable() {
    let mut cpu = loaded(&[0x00, 0x09, 0x00, 0x1b]);
    let mut state = cpu.snapshot();
    state.pending_transfer = Some(PendingTransfer {
        target: DEFAULT_ORIGIN,
        slot_pc: DEFAULT_ORIGIN + 2,
    });
    assert_eq!(
        cpu.restore(state),
        Err(SuperH2Error::InvalidPendingTransfer)
    );
    let sr = StatusRegister {
        interrupt_mask: 15,
        ..StatusRegister::default()
    };
    assert_eq!(sr.interrupt_mask, 15);
    for error in [
        SuperH2Error::EmptyProgram,
        SuperH2Error::AllocationFailure,
        SuperH2Error::MaxStepsZero,
    ] {
        assert!(!error.to_string().is_empty());
    }
    assert_eq!(classify(0x0009), Ok(InstructionClass::Nop));
}

#[test]
fn every_admitted_executor_arm_has_a_behavior_witness() {
    let words: &[u16] = &[
        0xe12a, 0x6123, 0x6120, 0x2122, 0x6125, 0x2126, 0x012e, 0x0124, 0x8153, 0x1123, 0xc501,
        0xc201, 0x9101, 0xd101, 0x0129, 0xc701, 0x6128, 0x6129, 0x212d, 0x7123, 0x312c, 0x312e,
        0x312f, 0x3128, 0x312a, 0x312b, 0x612b, 0x612a, 0x4110, 0x612e, 0x612f, 0x612c, 0x612d,
        0x2129, 0xc901, 0xcd01, 0x212b, 0xcb01, 0xcf01, 0x212a, 0xca01, 0xce01, 0x6127, 0x2128,
        0xc801, 0xcc01, 0x3120, 0x3122, 0x3123, 0x3126, 0x3127, 0x4211, 0x4215, 0x4124, 0x4125,
        0x4104, 0x4105, 0x4120, 0x4121, 0x4100, 0x4101, 0x4108, 0x4118, 0x4128, 0x4109, 0x4119,
        0x4129, 0x212e, 0x212f, 0x0127, 0x3125, 0x312d, 0x412f, 0x012f, 0x0019, 0x2127, 0x3124,
        0x0008, 0x0018, 0x0028, 0x010a, 0x011a, 0x012a, 0x0112, 0x410a, 0x411a, 0x412a, 0x411e,
        0x4102, 0x4112, 0x4122, 0x4113, 0x4106, 0x4116, 0x4126, 0x4117, 0xa001, 0xb001, 0x8901,
        0x8b01, 0x8d01, 0x8f01, 0x0103, 0x0123, 0x410b, 0x412b, 0x000b, 0x0009, 0x001b,
    ];
    for &word in words {
        let mut cpu = loaded(&[
            word.to_be_bytes()[0],
            word.to_be_bytes()[1],
            0,
            0x1b,
            0,
            9,
            0,
            9,
        ]);
        install_state(&mut cpu, |s| {
            s.registers.fill(0x0200);
            s.registers[0] = 0;
            s.gbr = 0x0300;
            s.pr = DEFAULT_ORIGIN + 4;
            s.mach = 1;
            s.macl = 2;
            s.memory[0x1f0..0x220].fill(1);
            s.memory[0x300..0x320].fill(1);
        });
        let outcome = cpu.step();
        assert!(outcome.is_ok(), "word {word:#06x}: {outcome:?}");
    }
}

#[test]
fn saturation_flags_branch_senses_and_link_register_targets_are_witnessed() {
    let mut macw = loaded(&[0x41, 0x2f, 0x00, 0x1b]);
    install_state(&mut macw, |s| {
        s.sr.s = true;
        s.macl = i32::MAX as u32;
        s.registers[1] = 0x200;
        s.registers[2] = 0x204;
        s.memory[0x200..0x202].copy_from_slice(&2_i16.to_be_bytes());
        s.memory[0x204..0x206].copy_from_slice(&2_i16.to_be_bytes());
    });
    assert_eq!(macw.step().unwrap().post_state.macl, i32::MAX as u32);

    let mut macl = loaded(&[0x01, 0x2f, 0x00, 0x1b]);
    install_state(&mut macl, |s| {
        s.sr.s = true;
        s.mach = 0x7fff;
        s.macl = u32::MAX;
        s.registers[1] = 0x200;
        s.registers[2] = 0x204;
        s.memory[0x200..0x204].copy_from_slice(&2_i32.to_be_bytes());
        s.memory[0x204..0x208].copy_from_slice(&2_i32.to_be_bytes());
    });
    let state = macl.step().unwrap().post_state;
    assert_eq!(state.mach, 0x7fff);
    assert_eq!(state.macl, u32::MAX);

    let mut negative = loaded(&[0x01, 0x2f]);
    install_state(&mut negative, |s| {
        s.sr.s = true;
        s.mach = 0xffff_8000;
        s.macl = 0;
        s.registers[1] = 0x200;
        s.registers[2] = 0x204;
        s.memory[0x200..0x204].copy_from_slice(&(-2_i32).to_be_bytes());
        s.memory[0x204..0x208].copy_from_slice(&2_i32.to_be_bytes());
    });
    let state = negative.step().unwrap().post_state;
    assert_eq!(state.mach, 0xffff_8000);
    assert_eq!(state.macl, 0);

    let mut bsrf = loaded(&[0x01, 0x03, 0x00, 0x09, 0x00, 0x1b]);
    install_state(&mut bsrf, |s| s.registers[1] = 0);
    let trace = bsrf.step().unwrap();
    assert_eq!(trace.post_state.pr, DEFAULT_ORIGIN + 4);
    assert_eq!(
        trace.post_state.pending_transfer.unwrap().target,
        DEFAULT_ORIGIN + 4
    );
}

#[test]
fn malformed_snapshot_and_fetch_variants_remain_atomic() {
    let mut cpu = loaded(&[0, 9]);
    let before = cpu.snapshot();
    for mutate in [0_u8, 1, 2, 3] {
        let mut state = before.clone();
        match mutate {
            0 => state.installed_length = 0,
            1 => state.installed_length = 3,
            2 => state.installed_origin = 1,
            _ => state.pc = DEFAULT_ORIGIN + 1,
        }
        assert!(cpu.restore(state).is_err());
        assert_eq!(cpu.snapshot(), before);
    }
    let mut state = before.clone();
    state.pending_transfer = Some(PendingTransfer {
        target: 0,
        slot_pc: DEFAULT_ORIGIN,
    });
    state.halted = true;
    assert_eq!(
        cpu.restore(state),
        Err(SuperH2Error::InvalidPendingTransfer)
    );

    let before = cpu.snapshot();
    assert!(matches!(
        cpu.write_data((MEMORY_BYTES - 1) as u32, Width::Long, 1),
        Err(SuperH2Error::MisalignedData { .. }) | Err(SuperH2Error::DataOutOfRange { .. })
    ));
    assert_eq!(cpu.snapshot(), before);
}

#[test]
fn exact_subfamily_mnemonics_prevent_decoder_aliases() {
    let cases: &[(u16, &str)] = &[
        (0x3120, "CMP/EQ"),
        (0x3122, "CMP/HS"),
        (0x3123, "CMP/GE"),
        (0x3126, "CMP/HI"),
        (0x3127, "CMP/GT"),
        (0x4111, "CMP/PZ"),
        (0x4115, "CMP/PL"),
        (0x212c, "CMP/STR"),
        (0x8801, "CMP/EQ"),
        (0x4100, "SHLL"),
        (0x4101, "SHLR"),
        (0x4120, "SHAL"),
        (0x4121, "SHAR"),
        (0x4104, "ROTL"),
        (0x4105, "ROTR"),
        (0x4124, "ROTCL"),
        (0x4125, "ROTCR"),
        (0x4108, "SHLL2"),
        (0x4118, "SHLL8"),
        (0x4128, "SHLL16"),
        (0x4109, "SHLR2"),
        (0x4119, "SHLR8"),
        (0x4129, "SHLR16"),
        (0x212e, "MULU.W"),
        (0x212f, "MULS.W"),
        (0x0127, "MUL.L"),
        (0x3125, "DMULU.L"),
        (0x312d, "DMULS.L"),
        (0x8901, "BT"),
        (0x8b01, "BF"),
        (0x8d01, "BT/S"),
        (0x8f01, "BF/S"),
        (0xa001, "BRA"),
        (0xb001, "BSR"),
        (0x0103, "BSRF"),
        (0x0123, "BRAF"),
        (0x410b, "JSR"),
        (0x412b, "JMP"),
        (0x000b, "RTS"),
        (0x010a, "STS MACH"),
        (0x011a, "STS MACL"),
        (0x012a, "STS PR"),
        (0x0112, "STC GBR"),
        (0x410a, "LDS MACH"),
        (0x411a, "LDS MACL"),
        (0x412a, "LDS PR"),
        (0x411e, "LDC GBR"),
        (0x4102, "STS.L"),
        (0x4113, "STC.L"),
        (0x4106, "LDS.L"),
        (0x4117, "LDC.L"),
    ];
    for &(word, expected) in cases {
        let bytes = word.to_be_bytes();
        let mut cpu = loaded(&[bytes[0], bytes[1], 0, 9, 0, 0x1b, 0, 9]);
        install_state(&mut cpu, |s| {
            s.registers.fill(0x200);
            s.pr = DEFAULT_ORIGIN + 4;
            s.memory[0x1f0..0x210].fill(1);
        });
        let trace = cpu.step().unwrap();
        assert_eq!(trace.mnemonic, expected, "word {word:#06x}");
    }
}

#[test]
fn excluded_and_reserved_neighbors_fail_without_mutation() {
    let unsupported = [
        0x002b_u16, 0xc300, 0xc3ff, 0x411b, 0x410e, 0x412e, 0x4107, 0x4127, 0x0102, 0x0122, 0x4103,
        0x4123,
    ];
    for word in unsupported {
        let bytes = word.to_be_bytes();
        let mut cpu = loaded(&[bytes[0], bytes[1]]);
        let before = cpu.snapshot();
        assert!(matches!(
            cpu.step(),
            Err(SuperH2Error::UnsupportedInstruction { .. })
        ));
        assert_eq!(cpu.snapshot(), before);
    }
    for word in [0x0000_u16, 0xffff, 0x8020, 0x400c] {
        let bytes = word.to_be_bytes();
        let mut cpu = loaded(&[bytes[0], bytes[1]]);
        let before = cpu.snapshot();
        let outcome = cpu.step();
        assert!(
            matches!(outcome, Err(SuperH2Error::UnknownInstruction { .. })),
            "word {word:#06x}: {outcome:?}"
        );
        assert_eq!(cpu.snapshot(), before);
    }
}
