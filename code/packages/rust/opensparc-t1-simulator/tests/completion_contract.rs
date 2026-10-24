use opensparc_t1_simulator::{
    assemble, classify_word, encoding::*, ConditionCodes, InstructionClass, OpenSparcT1Error,
    OpenSparcT1Simulator, OpenSparcT1State, MEMORY_SIZE, NUM_PHYSICAL_REGISTERS,
};

const ORIGIN: u64 = 0x100;

fn loaded(words: &[u32]) -> OpenSparcT1Simulator {
    let mut simulator = OpenSparcT1Simulator::new();
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
        Err(OpenSparcT1Error::UnsupportedInstruction { .. })
    ));
    assert!(matches!(
        classify_word(0),
        Err(OpenSparcT1Error::UnknownInstruction { .. })
    ));
}

#[test]
fn reset_and_load_expose_complete_strand_state() {
    let simulator = OpenSparcT1Simulator::new();
    let state = simulator.get_state();
    assert_eq!(state.registers.len(), NUM_PHYSICAL_REGISTERS);
    assert_eq!(state.memory.len(), MEMORY_SIZE);
    assert_eq!(state.pc, 0);
    assert_eq!(state.npc, 4);
    assert_eq!(state.cwp, 0);
    assert_eq!(state.cansave, 6);
    assert_eq!(state.canrestore, 0);
    assert_eq!(state.cleanwin, 7);
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
fn load_validation_is_atomic() {
    let mut simulator = loaded(&[encode_ta(0)]);
    let before = simulator.get_state();
    assert_eq!(simulator.load(&[], 0), Err(OpenSparcT1Error::EmptyProgram));
    assert_eq!(simulator.get_state(), before);
    assert!(matches!(
        simulator.load(&[0; 3], 0),
        Err(OpenSparcT1Error::MisalignedProgram { .. })
    ));
    assert_eq!(simulator.get_state(), before);
    assert!(matches!(
        simulator.load(&assemble(&[encode_nop()]), 0xffff),
        Err(OpenSparcT1Error::MisalignedProgram { .. })
    ));
    assert_eq!(simulator.get_state(), before);
    assert!(matches!(
        simulator.load(&assemble(&[encode_nop(), encode_nop()]), 0xfffc),
        Err(OpenSparcT1Error::ProgramOutOfRange { .. })
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
        Err(OpenSparcT1Error::InvalidState { .. })
    ));
    assert_eq!(simulator.get_state(), before);

    for mutate in [
        |state: &mut OpenSparcT1State| state.registers[0] = 1,
        |state: &mut OpenSparcT1State| state.registers[8] = 1,
        |state: &mut OpenSparcT1State| state.gl = 4,
        |state: &mut OpenSparcT1State| state.cwp = 8,
        |state: &mut OpenSparcT1State| state.cansave = 7,
        |state: &mut OpenSparcT1State| state.pc = 1,
        |state: &mut OpenSparcT1State| state.npc = 2,
        |state: &mut OpenSparcT1State| state.pstate.cle = true,
    ] {
        let mut invalid = before.clone();
        mutate(&mut invalid);
        assert!(matches!(
            simulator.restore(invalid),
            Err(OpenSparcT1Error::InvalidState { .. })
        ));
        assert_eq!(simulator.get_state(), before);
    }
}

#[test]
fn four_global_levels_and_g0_are_exact() {
    let mut simulator = loaded(&[encode_ta(0)]);
    simulator.write_register(1, 11).unwrap();
    let mut state = simulator.get_state();
    state.gl = 1;
    simulator.restore(state).unwrap();
    assert_eq!(simulator.read_register(1).unwrap(), 0);
    simulator.write_register(1, 22).unwrap();
    simulator.write_register(0, u64::MAX).unwrap();
    assert_eq!(simulator.read_register(0).unwrap(), 0);
    let mut state = simulator.get_state();
    state.gl = 0;
    simulator.restore(state).unwrap();
    assert_eq!(simulator.read_register(1).unwrap(), 11);
    assert_eq!(
        simulator.read_register(32),
        Err(OpenSparcT1Error::InvalidRegister { index: 32 })
    );
}

#[test]
fn save_and_restore_rotate_eight_windows_with_overlap() {
    let mut simulator = loaded(&[
        encode_save(16, 14, -32),
        encode_restore(8, 24, 5),
        encode_ta(0),
    ]);
    simulator.write_register(8, 0x55).unwrap();
    simulator.write_register(14, 0x1000).unwrap();
    let save = simulator.step().unwrap();
    assert_eq!(save.mnemonic, "save");
    assert_eq!(simulator.get_state().cwp, 1);
    assert_eq!(simulator.read_register(24).unwrap(), 0x55);
    assert_eq!(simulator.read_register(16).unwrap(), 0xfe0);
    assert_eq!(simulator.get_state().cansave, 5);
    assert_eq!(simulator.get_state().canrestore, 1);

    simulator.step().unwrap();
    assert_eq!(simulator.get_state().cwp, 0);
    assert_eq!(simulator.read_register(8).unwrap(), 0x5a);
    assert_eq!(simulator.get_state().cansave, 6);
    assert_eq!(simulator.get_state().canrestore, 0);
}

#[test]
fn window_spill_fill_and_clean_faults_are_atomic() {
    let cases = [
        (encode_save(1, 0, 0), 0, 0, 7, "spill"),
        (encode_restore(1, 0, 0), 6, 0, 7, "fill"),
        (encode_save(1, 0, 0), 1, 1, 1, "clean"),
    ];
    for (word, cansave, canrestore, cleanwin, kind) in cases {
        let mut simulator = loaded(&[word]);
        let mut state = simulator.get_state();
        state.cansave = cansave;
        state.canrestore = canrestore;
        state.cleanwin = cleanwin;
        state.otherwin = 6_u8.saturating_sub(cansave + canrestore);
        simulator.restore(state).unwrap();
        let before = simulator.get_state();
        let error = simulator.step().unwrap_err();
        match kind {
            "spill" => assert!(matches!(error, OpenSparcT1Error::WindowSpill)),
            "fill" => assert!(matches!(error, OpenSparcT1Error::WindowFill)),
            _ => assert!(matches!(error, OpenSparcT1Error::CleanWindow)),
        }
        assert_eq!(simulator.get_state(), before);
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
    simulator.step().unwrap();
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
    simulator.step().unwrap();
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
fn control_transfer_in_delay_slot_is_rejected_atomically() {
    let mut simulator = loaded(&[encode_call(2), encode_call(2), encode_ta(0)]);
    simulator.step().unwrap();
    let before = simulator.get_state();
    assert_eq!(
        simulator.step(),
        Err(OpenSparcT1Error::ControlTransferInDelaySlot { pc: ORIGIN + 4 })
    );
    assert_eq!(simulator.get_state(), before);
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
            Err(OpenSparcT1Error::MisalignedData { .. })
                | Err(OpenSparcT1Error::DataOutOfRange { .. })
        ));
        assert_eq!(simulator.get_state(), before);
    }

    let mut simulator = loaded(&[encode_nop()]);
    simulator.step().unwrap();
    let before = simulator.get_state();
    assert_eq!(
        simulator.step(),
        Err(OpenSparcT1Error::FetchOutsideProgram { pc: ORIGIN + 4 })
    );
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn division_and_target_faults_are_atomic() {
    let cases = [
        (encode_udivx(3, 1, 2), OpenSparcT1Error::DivideByZero),
        (
            encode_jmpl(3, 1, 0),
            OpenSparcT1Error::MisalignedTarget { target: 3 },
        ),
    ];
    for (word, expected) in cases {
        let mut simulator = loaded(&[word]);
        simulator
            .write_register(
                1,
                if matches!(expected, OpenSparcT1Error::MisalignedTarget { .. }) {
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
    assert_eq!(trace.mnemonic, "ta 0");
    assert!(trace.state_after.halted);
    let before = simulator.get_state();
    assert_eq!(simulator.step(), Err(OpenSparcT1Error::Halted));
    assert_eq!(simulator.get_state(), before);

    let mut trap = loaded(&[encode_ta(1)]);
    let before = trap.get_state();
    assert_eq!(
        trap.step(),
        Err(OpenSparcT1Error::UnsupportedTrap {
            number: 1,
            pc: ORIGIN
        })
    );
    assert_eq!(trap.get_state(), before);
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
        Err(OpenSparcT1Error::MisalignedData { .. })
    ));
    assert_eq!(rollback.get_state(), before);
}

#[test]
fn run_limits_fail_before_or_rollback_after_execution() {
    let mut simulator = loaded(&[encode_nop(), encode_ta(0)]);
    let before = simulator.get_state();
    assert_eq!(
        simulator.run(1),
        Err(OpenSparcT1Error::StepLimitExceeded { limit: 1 })
    );
    assert_eq!(simulator.get_state(), before);
    assert_eq!(
        simulator.run(1025),
        Err(OpenSparcT1Error::RetentionLimitExceeded {
            requested: 1025,
            limit: 1024
        })
    );
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn execute_loads_and_runs_transactionally() {
    let mut simulator = OpenSparcT1Simulator::new();
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
}

#[test]
fn every_error_has_nonempty_display_text() {
    let errors = [
        OpenSparcT1Error::EmptyProgram,
        OpenSparcT1Error::InvalidRegister { index: 32 },
        OpenSparcT1Error::MisalignedFetch { pc: 1 },
        OpenSparcT1Error::UnknownInstruction { word: 0, pc: 0 },
        OpenSparcT1Error::UnsupportedInstruction { word: 1, pc: 0 },
        OpenSparcT1Error::PrivilegedInstruction { word: 2, pc: 0 },
        OpenSparcT1Error::AllocationFailed,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}
