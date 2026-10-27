use sparc_v9_simulator::{
    assemble, classify_word, encoding::*, ConditionCodes, InstructionClass, SparcV9Error,
    SparcV9Simulator, SparcV9State, MAX_RETAINED_STEPS, MEMORY_SIZE, NUM_PHYSICAL_REGISTERS,
};

const ORIGIN: u64 = 0x100;

fn loaded(words: &[u32]) -> SparcV9Simulator {
    let mut simulator = SparcV9Simulator::new();
    simulator.load(&assemble(words), ORIGIN).unwrap();
    simulator
}

#[test]
fn format_and_family_classification_is_explicit() {
    let cases = [
        (encode_call(1), InstructionClass::Call),
        (encode_sethi(1, 1), InstructionClass::Sethi),
        (encode_bicc(9, false, 1), InstructionClass::Branch),
        (
            encode_bpcc(9, false, true, 1),
            InstructionClass::PredictedBranch,
        ),
        (
            encode_bpr(5, false, true, 1, 1),
            InstructionClass::RegisterBranch,
        ),
        (encode_add(1, 2, 3), InstructionClass::Arithmetic),
        (encode_xor(1, 2, 3), InstructionClass::Logical),
        (encode_sllx(1, 2, 3), InstructionClass::Shift),
        (encode_mulx(1, 2, 3), InstructionClass::MultiplyDivide),
        (encode_jmpl(1, 2, 4), InstructionClass::ControlTransfer),
        (encode_save(1, 2, 4), InstructionClass::Window),
        (encode_ldx(1, 2, 8), InstructionClass::Memory),
        (encode_ta(0), InstructionClass::Trap),
    ];
    for (word, expected) in cases {
        assert_eq!(classify_word(word), Ok(expected), "word {word:#010x}");
    }
    assert!(matches!(
        classify_word(encode_format3(2, 1, 0x36, 2, true, 0)),
        Err(SparcV9Error::UnsupportedInstruction { .. })
    ));
    assert!(matches!(
        classify_word(0),
        Err(SparcV9Error::UnknownInstruction { .. })
    ));
}

#[test]
fn reset_and_load_expose_complete_processor_state() {
    let simulator = SparcV9Simulator::new();
    let state = simulator.get_state();
    assert_eq!(state.registers.len(), NUM_PHYSICAL_REGISTERS);
    assert_eq!(state.memory.len(), MEMORY_SIZE);
    assert_eq!(state.pc, 0);
    assert_eq!(state.npc, 4);
    assert_eq!(state.cwp, 0);
    assert_eq!(state.cansave, 2);
    assert_eq!(state.canrestore, 0);
    assert_eq!(state.cleanwin, 2);
    assert_eq!(state.loaded_len, 0);

    let simulator = loaded(&[encode_nop(), encode_ta(0)]);
    let state = simulator.get_state();
    assert_eq!(state.pc, ORIGIN);
    assert_eq!(state.npc, ORIGIN + 4);
    assert_eq!(
        &state.memory[ORIGIN as usize..ORIGIN as usize + 8],
        &assemble(&[encode_nop(), encode_ta(0)])
    );
    assert_eq!(state.loaded_origin, ORIGIN);
    assert_eq!(state.loaded_len, 8);
}

#[test]
fn an_empty_default_state_cannot_fetch_zero_filled_memory() {
    let mut simulator = SparcV9Simulator::new();
    let before = simulator.get_state();
    assert_eq!(
        simulator.step(),
        Err(SparcV9Error::FetchOutsideProgram { pc: 0 })
    );
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn load_validation_is_atomic() {
    let mut simulator = loaded(&[encode_ta(0)]);
    let before = simulator.get_state();
    assert_eq!(simulator.load(&[], 0), Err(SparcV9Error::EmptyProgram));
    assert_eq!(simulator.get_state(), before);
    assert!(matches!(
        simulator.load(&[0; 3], 0),
        Err(SparcV9Error::MisalignedProgram { .. })
    ));
    assert_eq!(simulator.get_state(), before);
    assert!(matches!(
        simulator.load(&assemble(&[encode_nop()]), 0xffff),
        Err(SparcV9Error::MisalignedProgram { .. })
    ));
    assert_eq!(simulator.get_state(), before);
    assert!(matches!(
        simulator.load(&assemble(&[encode_nop(), encode_nop()]), 0xfffc),
        Err(SparcV9Error::ProgramOutOfRange { .. })
    ));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn restore_validates_all_structural_invariants_atomically() {
    let mut simulator = loaded(&[encode_ta(0)]);
    let before = simulator.get_state();
    let mut invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        simulator.restore(invalid),
        Err(SparcV9Error::InvalidState { .. })
    ));
    assert_eq!(simulator.get_state(), before);

    for mutate in [
        |state: &mut SparcV9State| state.registers[0] = 1,
        |state: &mut SparcV9State| state.cwp = 4,
        |state: &mut SparcV9State| state.cansave = 3,
        |state: &mut SparcV9State| {
            state.cansave = 1;
            state.canrestore = 1;
            state.cleanwin = 0;
        },
        |state: &mut SparcV9State| state.wstate = 0x40,
        |state: &mut SparcV9State| state.pc = 1,
        |state: &mut SparcV9State| state.npc = 2,
        |state: &mut SparcV9State| state.y = 1 << 32,
    ] {
        let mut invalid = before.clone();
        mutate(&mut invalid);
        assert!(matches!(
            simulator.restore(invalid),
            Err(SparcV9Error::InvalidState { .. })
        ));
        assert_eq!(simulator.get_state(), before);
    }

    let mut valid_high_cleanwin = before.clone();
    valid_high_cleanwin.cleanwin = 3;
    simulator.restore(valid_high_cleanwin.clone()).unwrap();
    assert_eq!(simulator.get_state(), valid_high_cleanwin);
}

#[test]
fn normal_globals_and_g0_are_exact() {
    let mut simulator = loaded(&[encode_ta(0)]);
    simulator.write_register(1, 11).unwrap();
    simulator.write_register(0, u64::MAX).unwrap();
    assert_eq!(simulator.read_register(0).unwrap(), 0);
    assert_eq!(simulator.read_register(1).unwrap(), 11);
    assert_eq!(
        simulator.read_register(32),
        Err(SparcV9Error::InvalidRegister { index: 32 })
    );
}

#[test]
fn save_and_restore_rotate_four_windows_with_overlap() {
    let mut simulator = loaded(&[
        encode_save(16, 14, -32),
        encode_restore(8, 24, 5),
        encode_ta(0),
    ]);
    simulator.write_register(8, 0x55).unwrap();
    simulator.write_register(14, 0x1000).unwrap();
    let save = simulator.step().unwrap();
    assert_eq!(save.mnemonic, "SAVE");
    assert_eq!(simulator.get_state().cwp, 1);
    assert_eq!(simulator.read_register(24).unwrap(), 0x55);
    assert_eq!(simulator.read_register(16).unwrap(), 0xfe0);
    assert_eq!(simulator.get_state().cansave, 1);
    assert_eq!(simulator.get_state().canrestore, 1);

    simulator.step().unwrap();
    assert_eq!(simulator.get_state().cwp, 0);
    assert_eq!(simulator.read_register(8).unwrap(), 0x5a);
    assert_eq!(simulator.get_state().cansave, 2);
    assert_eq!(simulator.get_state().canrestore, 0);
}

#[test]
fn window_spill_fill_and_clean_faults_are_atomic() {
    let cases = [
        (encode_save(1, 0, 0), 0, 0, 2, "spill"),
        (encode_restore(1, 0, 0), 2, 0, 2, "fill"),
        (encode_save(1, 0, 0), 1, 1, 1, "clean"),
    ];
    for (word, cansave, canrestore, cleanwin, kind) in cases {
        let mut simulator = loaded(&[word]);
        let mut state = simulator.get_state();
        state.cansave = cansave;
        state.canrestore = canrestore;
        state.cleanwin = cleanwin;
        state.otherwin = 2_u8.saturating_sub(cansave + canrestore);
        simulator.restore(state).unwrap();
        let before = simulator.get_state();
        let error = simulator.step().unwrap_err();
        match kind {
            "spill" => assert!(matches!(error, SparcV9Error::WindowSpill)),
            "fill" => assert!(matches!(error, SparcV9Error::WindowFill)),
            _ => assert!(matches!(error, SparcV9Error::CleanWindow)),
        }
        assert_eq!(simulator.get_state(), before);
    }
}

#[test]
fn every_configured_window_save_restore_and_return_transition_is_witnessed() {
    for cwp in 0..4 {
        let mut save = loaded(&[encode_save(16, 0, 5)]);
        let mut state = save.get_state();
        state.cwp = cwp;
        save.restore(state).unwrap();
        save.write_register(8, 0x100 + u64::from(cwp)).unwrap();
        save.step().unwrap();
        assert_eq!(save.get_state().cwp, (cwp + 1) % 4);
        assert_eq!(save.read_register(24).unwrap(), 0x100 + u64::from(cwp));
        assert_eq!(save.read_register(16).unwrap(), 5);

        let mut restore = loaded(&[encode_restore(8, 0, 7)]);
        let mut state = restore.get_state();
        state.cwp = cwp;
        state.cansave = 1;
        state.canrestore = 1;
        state.cleanwin = 2;
        restore.restore(state).unwrap();
        restore.step().unwrap();
        assert_eq!(restore.get_state().cwp, (cwp + 3) % 4);
        assert_eq!(restore.read_register(8).unwrap(), 7);

        let mut returned = loaded(&[encode_return(1, 0), encode_nop()]);
        let mut state = returned.get_state();
        state.cwp = cwp;
        state.cansave = 1;
        state.canrestore = 1;
        state.cleanwin = 2;
        returned.restore(state).unwrap();
        returned.write_register(1, ORIGIN + 8).unwrap();
        returned.step().unwrap();
        assert_eq!(returned.get_state().cwp, (cwp + 3) % 4);
        assert_eq!(returned.get_state().npc, ORIGIN + 8);
    }
}

#[test]
fn arithmetic_logical_and_both_condition_code_widths_execute() {
    let mut simulator = loaded(&[
        encode_addcc(3, 1, 2),
        encode_subcc(4, 0, 2),
        encode_xorcc(5, 1, 2),
        encode_ta(0),
    ]);
    simulator.write_register(1, u64::MAX).unwrap();
    simulator.write_register(2, 1).unwrap();
    let add = simulator.step().unwrap();
    assert_eq!(add.state_after.registers[3], 0);
    assert_eq!(
        add.state_after.ccr.xcc,
        ConditionCodes {
            n: false,
            z: true,
            v: false,
            c: true
        }
    );
    assert_eq!(
        add.state_after.ccr.icc,
        ConditionCodes {
            n: false,
            z: true,
            v: false,
            c: true
        }
    );
    let sub = simulator.step().unwrap();
    assert_eq!(simulator.read_register(4).unwrap(), u64::MAX);
    assert!(sub.state_after.ccr.xcc.n);
    assert!(sub.state_after.ccr.xcc.c);
    let logic = simulator.step().unwrap();
    assert_eq!(simulator.read_register(5).unwrap(), u64::MAX - 1);
    assert!(!logic.state_after.ccr.xcc.v && !logic.state_after.ccr.xcc.c);
}

#[test]
fn addc_and_subc_read_the_32_bit_carry_bank() {
    let mut simulator = loaded(&[
        encode_addc(3, 1, 2),
        encode_addccc(4, 1, 2),
        encode_subc(5, 1, 2),
        encode_subccc(6, 1, 2),
        encode_ta(0),
    ]);
    simulator.write_register(1, 10).unwrap();
    simulator.write_register(2, 3).unwrap();
    let mut state = simulator.get_state();
    state.ccr.icc.c = true;
    state.ccr.xcc.c = false;
    simulator.restore(state).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(3).unwrap(), 14);
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(4).unwrap(), 14);
    let mut state = simulator.get_state();
    state.ccr.icc.c = true;
    state.ccr.xcc.c = false;
    simulator.restore(state).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(5).unwrap(), 6);
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(6).unwrap(), 6);
}

#[test]
fn immediates_shifts_and_multiply_divide_execute() {
    let mut simulator = loaded(&[
        encode_add_imm(1, 0, -1),
        encode_sllx(2, 1, 1),
        encode_srl(3, 1, 1),
        encode_srax(4, 1, 4),
        encode_mulx(5, 2, 3),
        encode_udivx(6, 5, 3),
        encode_sdivx_imm(7, 4, -1),
        encode_ta(0),
    ]);
    let state = simulator.run(8).unwrap().final_state;
    assert_eq!(simulator.read_register(1).unwrap(), u64::MAX);
    assert_eq!(simulator.read_register(2).unwrap(), u64::MAX - 1);
    assert_eq!(simulator.read_register(3).unwrap(), 0x7fff_ffff);
    assert_eq!(simulator.read_register(4).unwrap(), u64::MAX);
    assert_eq!(simulator.read_register(6).unwrap(), 8_589_934_594);
    assert_eq!(simulator.read_register(7).unwrap(), 1);
    assert!(state.halted);
}

#[test]
fn reserved_shift_fields_are_rejected_atomically() {
    let malformed = [
        encode_sllx(1, 2, 3) | (1 << 6),
        encode_srl(1, 2, 3) | (1 << 5),
        encode_format3(2, 1, 0x27, 2, false, 3) | (1 << 5),
    ];
    for word in malformed {
        let mut simulator = loaded(&[word]);
        let before = simulator.get_state();
        assert!(matches!(
            simulator.step(),
            Err(SparcV9Error::UnknownInstruction { .. })
        ));
        assert_eq!(simulator.get_state(), before);
    }
}

#[test]
fn register_branches_execute_delay_and_annul_rules() {
    let mut taken = loaded(&[
        encode_bpr(5, false, true, 1, 2),
        encode_add_imm(2, 0, 7),
        encode_ta(0),
    ]);
    taken.write_register(1, 1).unwrap();
    assert_eq!(taken.step().unwrap().mnemonic, "BRNZ");
    assert!(taken.get_state().delay_slot);
    taken.step().unwrap();
    assert_eq!(taken.read_register(2).unwrap(), 7);
    assert!(taken.step().unwrap().state_after.halted);

    let mut annulled = loaded(&[
        encode_bpr(1, true, false, 1, 2),
        encode_add_imm(2, 0, 9),
        encode_ta(0),
    ]);
    annulled.write_register(1, 1).unwrap();
    annulled.step().unwrap();
    assert_eq!(annulled.get_state().pc, ORIGIN + 8);
    assert!(!annulled.get_state().delay_slot);
    assert_eq!(annulled.read_register(2).unwrap(), 0);

    let mut negative = loaded(&[
        encode_bpr(3, false, false, 1, 2),
        encode_nop(),
        encode_ta(0),
    ]);
    negative.write_register(1, u64::MAX).unwrap();
    negative.step().unwrap();
    assert!(negative.get_state().delay_slot);
}

#[test]
fn reserved_register_branch_encodings_are_rejected_atomically() {
    for word in [
        encode_bpr(5, false, false, 1, 1) | (1 << 28),
        encode_bpr(0, false, false, 1, 1),
        encode_bpr(4, false, false, 1, 1),
    ] {
        let mut simulator = loaded(&[word]);
        let before = simulator.get_state();
        assert!(matches!(
            simulator.step(),
            Err(SparcV9Error::UnknownInstruction { .. })
        ));
        assert_eq!(simulator.get_state(), before);
    }
}

#[test]
fn thirty_two_bit_left_shift_preserves_the_full_register_width() {
    let word = encode_format3(2, 2, 0x25, 1, true, 1);
    let mut simulator = loaded(&[word, encode_ta(0)]);
    simulator.write_register(1, 0x1_0000_0001).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(2).unwrap(), 0x2_0000_0002);
}

#[test]
fn sethi_zero_extends_and_nop_preserves_registers() {
    let mut simulator = loaded(&[encode_sethi(1, 0x3f_ffff), encode_nop(), encode_ta(0)]);
    simulator.write_register(1, u64::MAX).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(1).unwrap(), 0xffff_fc00);
    let before = simulator.get_state();
    simulator.step().unwrap();
    let after = simulator.get_state();
    assert_eq!(before.registers, after.registers);
}

#[test]
fn delayed_branch_executes_slot_before_target() {
    let mut simulator = loaded(&[
        encode_bicc(8, false, 3),
        encode_add_imm(1, 0, 1),
        encode_add_imm(1, 0, 2),
        encode_add_imm(2, 0, 3),
        encode_ta(0),
    ]);
    let branch = simulator.step().unwrap();
    assert_eq!(branch.mnemonic, "BA");
    assert_eq!(
        (branch.pc_after, branch.npc_after),
        (ORIGIN + 4, ORIGIN + 12)
    );
    assert!(branch.state_after.delay_slot);
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(1).unwrap(), 1);
    assert_eq!(simulator.get_state().pc, ORIGIN + 12);
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(2).unwrap(), 3);
}

#[test]
fn conditional_annul_skips_only_an_untaken_delay_slot() {
    let mut simulator = loaded(&[
        encode_bicc(1, true, 3),
        encode_add_imm(1, 0, 1),
        encode_add_imm(2, 0, 2),
        encode_ta(0),
    ]);
    assert_eq!(simulator.step().unwrap().mnemonic, "BE");
    assert_eq!(simulator.get_state().pc, ORIGIN + 8);
    assert!(!simulator.get_state().delay_slot);
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(1).unwrap(), 0);
    assert_eq!(simulator.read_register(2).unwrap(), 2);

    let mut taken = loaded(&[
        encode_bicc(1, true, 3),
        encode_add_imm(1, 0, 1),
        encode_nop(),
        encode_ta(0),
    ]);
    let mut state = taken.get_state();
    state.ccr.icc.z = true;
    taken.restore(state).unwrap();
    taken.step().unwrap();
    assert!(taken.get_state().delay_slot);
    taken.step().unwrap();
    assert_eq!(taken.read_register(1).unwrap(), 1);
    assert_eq!(taken.get_state().pc, ORIGIN + 12);
}

#[test]
fn unconditional_annul_has_no_delay_slot() {
    let mut simulator = loaded(&[
        encode_bicc(8, true, 3),
        encode_add_imm(1, 0, 1),
        encode_nop(),
        encode_ta(0),
    ]);
    simulator.step().unwrap();
    assert_eq!(simulator.get_state().pc, ORIGIN + 12);
    assert_eq!(simulator.read_register(1).unwrap(), 0);
}

#[test]
fn branch_never_annul_skips_the_slot_without_fetching_it() {
    let mut simulator = loaded(&[
        encode_bicc(0, true, 3),
        0xffff_ffff,
        encode_ta(0),
        encode_nop(),
    ]);
    let trace = simulator.step().unwrap();
    assert_eq!(trace.mnemonic, "BN");
    assert!(trace.annulled);
    assert_eq!(trace.skipped_pc, Some(ORIGIN + 4));
    assert_eq!(simulator.get_state().pc, ORIGIN + 8);
}

#[test]
fn predicted_branch_selects_xcc() {
    let mut simulator = loaded(&[
        encode_bpcc(1, false, true, 3),
        encode_nop(),
        encode_nop(),
        encode_ta(0),
    ]);
    let mut state = simulator.get_state();
    state.ccr.icc.z = false;
    state.ccr.xcc.z = true;
    simulator.restore(state).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "BPE");
    assert_eq!(simulator.get_state().npc, ORIGIN + 12);
}

#[test]
fn call_and_jmpl_link_before_their_delay_slots() {
    let mut call = loaded(&[
        encode_call(3),
        encode_add(1, 15, 0),
        encode_nop(),
        encode_ta(0),
    ]);
    call.step().unwrap();
    assert_eq!(call.read_register(15).unwrap(), ORIGIN);
    call.step().unwrap();
    assert_eq!(call.read_register(1).unwrap(), ORIGIN);
    assert_eq!(call.get_state().pc, ORIGIN + 12);

    let mut jmpl = loaded(&[encode_jmpl(2, 1, 0), encode_add(3, 2, 0), encode_ta(0)]);
    jmpl.write_register(1, ORIGIN + 8).unwrap();
    jmpl.step().unwrap();
    assert_eq!(jmpl.read_register(2).unwrap(), ORIGIN);
    jmpl.step().unwrap();
    assert_eq!(jmpl.read_register(3).unwrap(), ORIGIN);
    assert_eq!(jmpl.get_state().pc, ORIGIN + 8);
}

#[test]
fn control_transfer_in_delay_slot_chains_normally() {
    let mut simulator = loaded(&[encode_call(2), encode_call(2), encode_ta(0)]);
    simulator.step().unwrap();
    let trace = simulator.step().unwrap();
    assert!(trace.in_delay_slot);
    assert!(trace.scheduled_delay);
    assert_eq!(simulator.get_state().pc, ORIGIN + 8);
    assert_eq!(simulator.get_state().npc, ORIGIN + 12);
}

#[test]
fn big_endian_scalar_loads_and_stores_cover_all_widths() {
    let mut simulator = loaded(&[
        encode_stx(2, 1, 0),
        encode_ldx(3, 1, 0),
        encode_lduw(4, 1, 0),
        encode_ldsw(5, 1, 0),
        encode_lduh(6, 1, 0),
        encode_ldsh(7, 1, 0),
        encode_ldub(8, 1, 0),
        encode_ldsb(9, 1, 0),
        encode_stw(2, 1, 8),
        encode_sth(2, 1, 12),
        encode_stb(2, 1, 14),
        encode_ta(0),
    ]);
    simulator.write_register(1, 0x200).unwrap();
    simulator.write_register(2, 0x80ff_7e01_2345_6789).unwrap();
    let state = simulator.run(12).unwrap().final_state;
    assert_eq!(
        &state.memory[0x200..0x208],
        &0x80ff_7e01_2345_6789_u64.to_be_bytes()
    );
    assert_eq!(simulator.read_register(3).unwrap(), 0x80ff_7e01_2345_6789);
    assert_eq!(simulator.read_register(4).unwrap(), 0x80ff_7e01);
    assert_eq!(simulator.read_register(5).unwrap(), 0xffff_ffff_80ff_7e01);
    assert_eq!(simulator.read_register(6).unwrap(), 0x80ff);
    assert_eq!(simulator.read_register(7).unwrap(), 0xffff_ffff_ffff_80ff);
    assert_eq!(simulator.read_register(8).unwrap(), 0x80);
    assert_eq!(simulator.read_register(9).unwrap(), 0xffff_ffff_ffff_ff80);
    assert_eq!(
        &state.memory[0x208..0x20f],
        &[0x23, 0x45, 0x67, 0x89, 0x67, 0x89, 0x89]
    );
}

#[test]
fn data_and_fetch_faults_are_typed_and_atomic() {
    let words = [encode_ldx(2, 1, 0), encode_ta(0)];
    for address in [1_u64, (MEMORY_SIZE - 4) as u64] {
        let mut simulator = loaded(&words);
        simulator.write_register(1, address).unwrap();
        let before = simulator.get_state();
        assert!(matches!(
            simulator.step(),
            Err(SparcV9Error::MisalignedData { .. }) | Err(SparcV9Error::DataOutOfRange { .. })
        ));
        assert_eq!(simulator.get_state(), before);
    }

    let mut simulator = loaded(&[encode_nop()]);
    simulator.step().unwrap();
    let before = simulator.get_state();
    assert_eq!(
        simulator.step(),
        Err(SparcV9Error::FetchOutsideProgram { pc: ORIGIN + 4 })
    );
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn division_and_target_faults_are_atomic() {
    let cases = [
        (encode_udivx(3, 1, 2), SparcV9Error::DivideByZero),
        (
            encode_jmpl(3, 1, 0),
            SparcV9Error::MisalignedTarget { target: 3 },
        ),
    ];
    for (word, expected) in cases {
        let mut simulator = loaded(&[word]);
        simulator
            .write_register(
                1,
                if matches!(expected, SparcV9Error::MisalignedTarget { .. }) {
                    3
                } else {
                    1
                },
            )
            .unwrap();
        simulator.write_register(2, 0).unwrap();
        let before = simulator.get_state();
        assert_eq!(simulator.step(), Err(expected));
        assert_eq!(simulator.get_state(), before);
    }
}

#[test]
fn signed_division_minimum_by_negative_one_saturates() {
    let mut simulator = loaded(&[encode_sdivx_imm(3, 1, -1), encode_ta(0)]);
    simulator.write_register(1, i64::MIN as u64).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(3).unwrap(), i64::MIN as u64);
}

#[test]
fn halt_and_other_traps_have_distinct_behavior() {
    let mut simulator = loaded(&[encode_ta(0)]);
    let trace = simulator.step().unwrap();
    assert_eq!(trace.mnemonic, "TA 0");
    assert!(trace.state_after.halted);
    let before = simulator.get_state();
    assert_eq!(simulator.step(), Err(SparcV9Error::Halted));
    assert_eq!(simulator.get_state(), before);

    let mut trap = loaded(&[encode_ta(1)]);
    let before = trap.get_state();
    assert_eq!(
        trap.step(),
        Err(SparcV9Error::UnsupportedTrap {
            number: 1,
            pc: ORIGIN
        })
    );
    assert_eq!(trap.get_state(), before);
}

#[test]
fn only_exact_canonical_ta_zero_halts() {
    let mut noncanonical = loaded(&[encode_ta(0x80)]);
    let before = noncanonical.get_state();
    assert_eq!(
        noncanonical.step(),
        Err(SparcV9Error::UnsupportedTrap {
            number: 0,
            pc: ORIGIN
        })
    );
    assert_eq!(noncanonical.get_state(), before);

    let mut xcc = loaded(&[encode_tcc(1, true, 0, 1, true)]);
    let mut state = xcc.get_state();
    state.ccr.xcc.z = true;
    state.ccr.icc.z = false;
    xcc.restore(state).unwrap();
    assert_eq!(
        xcc.step(),
        Err(SparcV9Error::UnsupportedTrap {
            number: 1,
            pc: ORIGIN
        })
    );

    let mut icc = loaded(&[encode_tcc(1, false, 0, 1, true)]);
    let before = icc.get_state();
    assert_eq!(
        icc.step(),
        Err(SparcV9Error::UnsupportedTrap {
            number: 1,
            pc: ORIGIN
        })
    );
    assert_eq!(icc.get_state(), before);

    for word in [
        encode_ta(0) | (1 << 29),
        encode_ta(0) | (1 << 11),
        encode_ta(0) | (1 << 8),
        encode_tcc(8, false, 0, 1, false) | (1 << 5),
    ] {
        let mut simulator = loaded(&[word]);
        let before = simulator.get_state();
        assert!(matches!(
            simulator.step(),
            Err(SparcV9Error::UnsupportedTrap { .. })
        ));
        assert_eq!(simulator.get_state(), before);
    }
}

#[test]
fn run_is_transactional_and_traces_complete_state() {
    let mut simulator = loaded(&[encode_add_imm(1, 0, 7), encode_ta(0)]);
    let result = simulator.run(2).unwrap();
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert!(result.halted);
    assert_eq!(result.final_state, simulator.get_state());
    assert_eq!(result.traces[0].raw, encode_add_imm(1, 0, 7).to_be_bytes());

    let mut rollback = loaded(&[encode_add_imm(1, 0, 7), encode_ldx(2, 0, 1)]);
    let before = rollback.get_state();
    assert!(matches!(
        rollback.run(2),
        Err(SparcV9Error::MisalignedData { .. })
    ));
    assert_eq!(rollback.get_state(), before);
}

#[test]
fn run_limits_fail_before_or_rollback_after_execution() {
    let mut simulator = loaded(&[encode_nop(), encode_ta(0)]);
    let before = simulator.get_state();
    assert_eq!(
        simulator.run(1),
        Err(SparcV9Error::StepLimitExceeded { limit: 1 })
    );
    assert_eq!(simulator.get_state(), before);
    assert_eq!(
        simulator.run(MAX_RETAINED_STEPS + 1),
        Err(SparcV9Error::RetentionLimitExceeded {
            requested: MAX_RETAINED_STEPS + 1,
            limit: MAX_RETAINED_STEPS
        })
    );
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn execute_loads_and_runs_transactionally() {
    let mut simulator = SparcV9Simulator::new();
    let result = simulator
        .execute(
            &assemble(&[encode_add_imm(1, 0, 9), encode_ta(0)]),
            0x200,
            2,
        )
        .unwrap();
    assert_eq!(result.final_state.pc, 0x208);
    assert_eq!(simulator.read_register(1).unwrap(), 9);
    let before = simulator.get_state();
    assert!(simulator.execute(&[0; 3], 0, 1).is_err());
    assert_eq!(simulator.get_state(), before);
    assert_eq!(
        simulator.execute(&[0; 3], 0, MAX_RETAINED_STEPS + 1),
        Err(SparcV9Error::RetentionLimitExceeded {
            requested: MAX_RETAINED_STEPS + 1,
            limit: MAX_RETAINED_STEPS
        })
    );
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn every_error_has_nonempty_display_text() {
    let errors = [
        SparcV9Error::EmptyProgram,
        SparcV9Error::InvalidRegister { index: 32 },
        SparcV9Error::InvalidRegisterPair { index: 3 },
        SparcV9Error::MisalignedFetch { pc: 1 },
        SparcV9Error::UnknownInstruction { word: 0, pc: 0 },
        SparcV9Error::UnsupportedInstruction { word: 1, pc: 0 },
        SparcV9Error::PrivilegedInstruction { word: 2, pc: 0 },
        SparcV9Error::AllocationFailed,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn v9_extension_families_and_neighbors_classify_without_aliases() {
    let admitted = [
        (encode_popc(1, 2), InstructionClass::MultiplyDivide),
        (encode_umul(1, 2, 3, true), InstructionClass::MultiplyDivide),
        (
            encode_sdiv(1, 2, 3, false),
            InstructionClass::MultiplyDivide,
        ),
        (
            encode_movcc(1, 8, false, true, 7),
            InstructionClass::ConditionalMove,
        ),
        (
            encode_movr(1, 2, 5, false, 3),
            InstructionClass::RegisterMove,
        ),
        (encode_membar(7, 15), InstructionClass::Barrier),
        (encode_return(1, 0), InstructionClass::ControlTransfer),
        (encode_ldd(2, 1, 0), InstructionClass::Memory),
        (encode_std(2, 1, 0), InstructionClass::Memory),
    ];
    for (word, class) in admitted {
        assert_eq!(classify_word(word), Ok(class), "{word:#010x}");
    }
    for word in [
        encode_format3(2, 1, 0x29, 0, false, 0),
        encode_format3(2, 1, 0x30, 0, false, 0),
        encode_format3(3, 1, 0x0c, 0, false, 0),
        encode_format3(3, 1, 0x10, 0, false, 0),
    ] {
        assert!(matches!(
            classify_word(word),
            Err(SparcV9Error::UnsupportedInstruction { .. })
                | Err(SparcV9Error::UnknownInstruction { .. })
        ));
    }
}

#[test]
fn movcc_and_movr_cover_both_sources_and_reserved_conditions() {
    let mut simulator = loaded(&[
        encode_movcc(3, 1, false, true, -7),
        encode_movcc(4, 9, true, false, 2),
        encode_movr(5, 1, 5, true, -9),
        encode_movr(6, 1, 1, false, 2),
        encode_ta(0),
    ]);
    simulator.write_register(1, 1).unwrap();
    simulator.write_register(2, 0x55).unwrap();
    let mut state = simulator.get_state();
    state.ccr.icc.z = true;
    state.ccr.xcc.z = false;
    simulator.restore(state).unwrap();
    simulator.run(5).unwrap();
    assert_eq!(simulator.read_register(3).unwrap(), (-7_i64) as u64);
    assert_eq!(simulator.read_register(4).unwrap(), 0x55);
    assert_eq!(simulator.read_register(5).unwrap(), (-9_i64) as u64);
    assert_eq!(simulator.read_register(6).unwrap(), 0);

    for word in [
        encode_movr(1, 2, 0, true, 1),
        encode_movr(1, 2, 4, true, 1),
        encode_movcc(1, 8, false, false, 2) | (1 << 5),
    ] {
        let mut cpu = loaded(&[word]);
        let before = cpu.get_state();
        assert!(cpu.step().is_err());
        assert_eq!(cpu.get_state(), before);
    }
}

#[test]
fn legacy_multiply_divide_update_y_flags_and_saturate() {
    let mut multiply = loaded(&[
        encode_umul(3, 1, 2, true),
        encode_smul(4, 1, 2, true),
        encode_ta(0),
    ]);
    multiply.write_register(1, 0xffff_ffff).unwrap();
    multiply.write_register(2, 2).unwrap();
    multiply.step().unwrap();
    assert_eq!(multiply.read_register(3).unwrap(), 0x1_ffff_fffe);
    assert_eq!(multiply.get_state().y, 1);
    assert!(multiply.get_state().ccr.icc.n);
    multiply.step().unwrap();
    assert_eq!(multiply.read_register(4).unwrap(), (-2_i64) as u64);
    assert_eq!(multiply.get_state().y, u64::from(u32::MAX));
    assert!(multiply.get_state().ccr.xcc.n);

    let mut divide = loaded(&[
        encode_udiv(3, 1, 2, true),
        encode_sdiv(4, 1, 2, true),
        encode_ta(0),
    ]);
    let mut state = divide.get_state();
    state.y = 1;
    divide.restore(state).unwrap();
    divide.write_register(1, 0).unwrap();
    divide.write_register(2, 1).unwrap();
    divide.step().unwrap();
    assert_eq!(divide.read_register(3).unwrap(), u64::from(u32::MAX));
    assert!(divide.get_state().ccr.icc.v);
    assert_eq!(divide.get_state().y, 1);
    divide.step().unwrap();
    assert_eq!(divide.read_register(4).unwrap(), i64::from(i32::MAX) as u64);
    assert!(divide.get_state().ccr.icc.v);
}

#[test]
fn popc_counts_all_bits_and_rejects_nonzero_reserved_rs1() {
    let mut simulator = loaded(&[encode_popc(2, 1), encode_ta(0)]);
    simulator.write_register(1, 0xf000_0000_0000_000f).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "POPC");
    assert_eq!(simulator.read_register(2).unwrap(), 8);

    let mut invalid = loaded(&[encode_popc(2, 1) | (1 << 14)]);
    let before = invalid.get_state();
    assert!(matches!(
        invalid.step(),
        Err(SparcV9Error::UnknownInstruction { .. })
    ));
    assert_eq!(invalid.get_state(), before);
}

#[test]
fn ldd_std_pairs_are_big_endian_even_and_atomic() {
    let mut simulator = loaded(&[encode_std(2, 1, 0), encode_ldd(4, 1, 0), encode_ta(0)]);
    simulator.write_register(1, 0x200).unwrap();
    simulator.write_register(2, 0xaaaa_bbbb_cccc_dddd).unwrap();
    simulator.write_register(3, 0x1111_2222_3333_4444).unwrap();
    simulator.step().unwrap();
    assert_eq!(
        &simulator.get_state().memory[0x200..0x208],
        &[0xcc, 0xcc, 0xdd, 0xdd, 0x33, 0x33, 0x44, 0x44]
    );
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(4).unwrap(), 0xcccc_dddd);
    assert_eq!(simulator.read_register(5).unwrap(), 0x3333_4444);

    for word in [encode_ldd(3, 1, 0), encode_std(3, 1, 0)] {
        let mut invalid = loaded(&[word]);
        invalid.write_register(1, 0x200).unwrap();
        let before = invalid.get_state();
        assert_eq!(
            invalid.step(),
            Err(SparcV9Error::InvalidRegisterPair { index: 3 })
        );
        assert_eq!(invalid.get_state(), before);
    }
}

#[test]
fn ldd_g0_discards_first_word_but_loads_g1() {
    let mut simulator = loaded(&[encode_ldd(0, 2, 0), encode_ta(0)]);
    simulator.write_register(2, 0x200).unwrap();
    simulator
        .write_memory(0x200, 8, 0xaaaa_bbbb_cccc_dddd)
        .unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(0).unwrap(), 0);
    assert_eq!(simulator.read_register(1).unwrap(), 0xcccc_dddd);
}

#[test]
fn membar_validates_exact_format_and_is_a_traced_noop() {
    let mut simulator = loaded(&[encode_membar(7, 15), encode_ta(0)]);
    let before = simulator.get_state();
    let trace = simulator.step().unwrap();
    assert_eq!(trace.mnemonic, "MEMBAR");
    assert_eq!(trace.state_after.pc, before.npc);
    assert_eq!(trace.state_after.registers, before.registers);
    assert_eq!(trace.state_after.memory, before.memory);

    for word in [
        encode_membar(0, 0) | (1 << 25),
        encode_membar(0, 0) ^ (1 << 13),
        encode_membar(0, 0) | (1 << 7),
    ] {
        let mut invalid = loaded(&[word]);
        let before = invalid.get_state();
        assert!(invalid.step().is_err());
        assert_eq!(invalid.get_state(), before);
    }
}

#[test]
fn return_restores_window_and_delays_transfer_atomically() {
    let mut simulator = loaded(&[
        encode_save(0, 0, 0),
        encode_return(1, 0),
        encode_add_imm(2, 0, 9),
        encode_ta(0),
    ]);
    simulator.write_register(1, ORIGIN + 12).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.get_state().cwp, 1);
    let trace = simulator.step().unwrap();
    assert_eq!(trace.mnemonic, "RETURN");
    assert!(trace.scheduled_delay);
    assert_eq!(simulator.get_state().cwp, 0);
    assert_eq!(simulator.get_state().pc, ORIGIN + 8);
    assert_eq!(simulator.get_state().npc, ORIGIN + 12);
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(2).unwrap(), 9);
}

#[test]
fn failed_delay_slot_preserves_pending_transfer() {
    let mut simulator = loaded(&[
        encode_call(3),
        encode_ldx(1, 0, 1),
        encode_nop(),
        encode_ta(0),
    ]);
    simulator.step().unwrap();
    let pending = simulator.get_state();
    assert!(pending.delay_slot);
    assert_eq!(pending.npc, ORIGIN + 12);
    assert!(matches!(
        simulator.step(),
        Err(SparcV9Error::MisalignedData { .. })
    ));
    assert_eq!(simulator.get_state(), pending);
}

#[test]
fn self_modifying_store_trace_keeps_original_fetched_bytes() {
    let store = encode_stw(2, 1, 0);
    let mut simulator = loaded(&[store, encode_ta(0)]);
    simulator.write_register(1, ORIGIN).unwrap();
    simulator.write_register(2, 0).unwrap();
    let trace = simulator.step().unwrap();
    assert_eq!(trace.raw, store.to_be_bytes());
    assert_eq!(
        &trace.state_after.memory[ORIGIN as usize..ORIGIN as usize + 4],
        &[0; 4]
    );
}

#[test]
fn annul_trace_names_skipped_address_and_skipped_word_cannot_fault() {
    let mut simulator = loaded(&[
        encode_bicc(1, true, 3),
        0xffff_ffff,
        encode_nop(),
        encode_ta(0),
    ]);
    let trace = simulator.step().unwrap();
    assert!(trace.annulled);
    assert_eq!(trace.skipped_pc, Some(ORIGIN + 4));
    assert!(!trace.scheduled_delay);
    assert_eq!(simulator.get_state().pc, ORIGIN + 8);
}
