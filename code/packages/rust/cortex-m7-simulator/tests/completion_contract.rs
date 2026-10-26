use cortex_m7_simulator::{
    classify, encoding::*, CortexM3Error, CortexM7Error, CortexM7Simulator, CortexM7State,
    InstructionClass, MAX_RETAINED_STEPS, MEMORY_SIZE,
};

const ORIGIN: u32 = 0x100;
const APSR_Q: u32 = 1 << 27;

fn loaded(words: &[u16]) -> CortexM7Simulator {
    let mut simulator = CortexM7Simulator::new();
    simulator.load(&program(words), ORIGIN).unwrap();
    simulator
}

#[test]
fn every_bounded_decode_family_is_explicit() {
    assert!(matches!(
        classify(nop(), None),
        Ok(InstructionClass::Inherited(_))
    ));
    for (encoded, expected) in [
        (qadd(0, 1, 2), InstructionClass::Qadd),
        (qsub(0, 1, 2), InstructionClass::Qsub),
        (sadd8(0, 1, 2), InstructionClass::Sadd8),
        (uadd8(0, 1, 2), InstructionClass::Uadd8),
    ] {
        assert_eq!(classify(encoded[0], Some(encoded[1])), Ok(expected));
    }
    assert!(matches!(
        classify(0xfa80, None),
        Err(CortexM7Error::Core(CortexM3Error::FetchOutsideProgram {
            width: 4,
            ..
        }))
    ));
    assert!(matches!(
        classify(0xfa80, Some(0xf020)),
        Err(CortexM7Error::Core(
            CortexM3Error::UnknownInstruction32 { .. }
        ))
    ));
}

#[test]
fn encoders_match_armv7e_m_little_endian_words() {
    assert_eq!(qadd(0, 1, 2), [0xfa82, 0xf081]);
    assert_eq!(qsub(3, 4, 5), [0xfa85, 0xf3a4]);
    assert_eq!(sadd8(6, 7, 0), [0xfa87, 0xf600]);
    assert_eq!(uadd8(1, 2, 3), [0xfa82, 0xf143]);
}

#[test]
fn reset_load_and_restore_cover_complete_state() {
    let mut simulator = CortexM7Simulator::new();
    let reset = simulator.get_state();
    assert_eq!(reset.core.memory.len(), MEMORY_SIZE);
    assert_eq!(reset.core.msp, MEMORY_SIZE as u32);
    assert_eq!(reset.core.loaded_len, 0);
    assert_eq!(reset.ge, 0);

    simulator.load(&program(&qadd(0, 1, 2)), ORIGIN).unwrap();
    let loaded = simulator.get_state();
    assert_eq!(loaded.core.pc, ORIGIN);
    assert_eq!(loaded.core.loaded_origin, ORIGIN);
    assert_eq!(loaded.core.loaded_len, 4);

    let mut restored = loaded.clone();
    restored.ge = 0b1010;
    restored.core.registers[4] = 99;
    simulator.restore(restored.clone()).unwrap();
    assert_eq!(simulator.get_state(), restored);

    simulator.reset();
    assert_eq!(simulator.get_state(), CortexM7State::default());
}

#[test]
fn invalid_load_and_restore_are_atomic() {
    let mut simulator = loaded(&[bkpt(0)]);
    let before = simulator.get_state();
    assert!(matches!(
        simulator.load(&[], 0),
        Err(CortexM7Error::Core(CortexM3Error::EmptyProgram))
    ));
    assert!(matches!(
        simulator.load(&[0, 0], 1),
        Err(CortexM7Error::Core(CortexM3Error::MisalignedProgram { .. }))
    ));
    assert_eq!(simulator.get_state(), before);

    let mut invalid_ge = before.clone();
    invalid_ge.ge = 0x10;
    assert!(matches!(
        simulator.restore(invalid_ge),
        Err(CortexM7Error::InvalidState { .. })
    ));
    let mut invalid_core = before.clone();
    invalid_core.core.memory.pop();
    assert!(matches!(
        simulator.restore(invalid_core),
        Err(CortexM7Error::Core(CortexM3Error::InvalidState { .. }))
    ));
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn qadd_saturates_both_directions_and_sets_sticky_q() {
    let encoded = qadd(0, 1, 2);
    let mut positive = loaded(&encoded);
    positive.write_register(1, i32::MAX as u32).unwrap();
    positive.write_register(2, 1).unwrap();
    let trace = positive.step().unwrap();
    assert_eq!(trace.mnemonic, "qadd");
    assert_eq!(trace.raw, [0x82, 0xfa, 0x81, 0xf0]);
    assert_eq!(trace.width, 4);
    assert_eq!(positive.read_register(0).unwrap(), i32::MAX as u32);
    assert_ne!(positive.get_state().core.apsr & APSR_Q, 0);

    let mut negative = loaded(&encoded);
    negative.write_register(1, i32::MIN as u32).unwrap();
    negative.write_register(2, (-1_i32) as u32).unwrap();
    negative.step().unwrap();
    assert_eq!(negative.read_register(0).unwrap(), i32::MIN as u32);
    assert_ne!(negative.get_state().core.apsr & APSR_Q, 0);
}

#[test]
fn qsub_uses_signed_operands_and_never_clears_q() {
    let encoded = qsub(3, 4, 5);
    let mut simulator = loaded(&[encoded[0], encoded[1], encoded[0], encoded[1]]);
    simulator.write_register(4, i32::MIN as u32).unwrap();
    simulator.write_register(5, 1).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "qsub");
    assert_eq!(simulator.read_register(3).unwrap(), i32::MIN as u32);
    assert_ne!(simulator.get_state().core.apsr & APSR_Q, 0);

    simulator.write_register(4, 9).unwrap();
    simulator.write_register(5, 4).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(3).unwrap(), 5);
    assert_ne!(simulator.get_state().core.apsr & APSR_Q, 0);
}

#[test]
fn sadd8_updates_each_lane_and_signed_ge_results() {
    let encoded = sadd8(0, 1, 2);
    let mut simulator = loaded(&encoded);
    simulator.write_register(1, 0x7f80_ff01).unwrap();
    simulator.write_register(2, 0x0101_01ff).unwrap();
    let trace = simulator.step().unwrap();
    assert_eq!(trace.mnemonic, "sadd8");
    assert_eq!(simulator.read_register(0).unwrap(), 0x8081_0000);
    // Lane sums: 0, 0, -127, +128.
    assert_eq!(simulator.get_state().ge, 0b1011);
}

#[test]
fn uadd8_updates_each_lane_and_unsigned_carry_ge_results() {
    let encoded = uadd8(0, 1, 2);
    let mut simulator = loaded(&encoded);
    simulator.write_register(1, 0xffff_0100).unwrap();
    simulator.write_register(2, 0x0101_ffff).unwrap();
    let trace = simulator.step().unwrap();
    assert_eq!(trace.mnemonic, "uadd8");
    assert_eq!(simulator.read_register(0).unwrap(), 0x0000_00ff);
    assert_eq!(simulator.get_state().ge, 0b1110);
}

#[test]
fn inherited_instructions_preserve_cortex_m7_only_state() {
    let parallel = uadd8(0, 1, 2);
    let mut words = Vec::from(parallel);
    words.extend([movs(3, 7), bkpt(0)]);
    let mut simulator = loaded(&words);
    simulator.write_register(1, u32::MAX).unwrap();
    simulator.write_register(2, 0x0101_0101).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.get_state().ge, 0b1111);
    let inherited = simulator.step().unwrap();
    assert_eq!(inherited.mnemonic, "movs");
    assert_eq!(inherited.state_before.ge, 0b1111);
    assert_eq!(inherited.state_after.ge, 0b1111);
    assert_eq!(simulator.read_register(3).unwrap(), 7);
}

#[test]
fn apsr_masks_isolate_nzcvq_and_ge_for_every_alias() {
    for sysm in 0..=3 {
        for (mask, expected_apsr, expected_ge) in [
            (1, 0x5000_0000, 0b1011),
            (2, 0xa800_0000, 0b0101),
            (3, 0xa800_0000, 0b1011),
        ] {
            let write = msr_masked(0, sysm, mask);
            let read = mrs(1, sysm);
            let mut simulator = loaded(&[write[0], write[1], read[0], read[1]]);
            let mut state = simulator.get_state();
            state.core.apsr = 0x5000_0000;
            state.ge = 0b0101;
            state.core.registers[0] = 0xa80b_0000;
            simulator.restore(state).unwrap();

            assert_eq!(simulator.step().unwrap().mnemonic, "msr");
            assert_eq!(simulator.get_state().core.apsr, expected_apsr);
            assert_eq!(simulator.get_state().ge, expected_ge);

            assert_eq!(simulator.step().unwrap().mnemonic, "mrs");
            assert_eq!(
                simulator.read_register(1).unwrap(),
                expected_apsr | (u32::from(expected_ge) << 16)
            );
        }
    }
}

#[test]
fn reserved_apsr_mask_is_typed_and_atomic() {
    let write = msr_masked(0, 0, 0);
    for false_it in [false, true] {
        let mut simulator = loaded(&write);
        let mut state = simulator.get_state();
        state.core.apsr = 0x5000_0000;
        state.ge = 0b0101;
        state.core.registers[0] = 0xa80b_0000;
        if false_it {
            state.core.itstate = 0x08;
        }
        simulator.restore(state).unwrap();
        let before = simulator.get_state();

        assert!(matches!(
            simulator.step(),
            Err(CortexM7Error::Core(
                CortexM3Error::UnknownInstruction32 { .. }
            ))
        ));
        assert_eq!(simulator.get_state(), before);
    }
}

#[test]
fn false_it_conditions_skip_ge_special_register_transport() {
    let read = mrs(1, 0);
    let mut reader = loaded(&read);
    let mut state = reader.get_state();
    state.core.itstate = 0x08;
    state.core.apsr = 0;
    state.core.registers[1] = 0xdead_beef;
    state.ge = 0b1011;
    reader.restore(state).unwrap();
    assert_eq!(reader.step().unwrap().mnemonic, "it-skip");
    assert_eq!(reader.read_register(1).unwrap(), 0xdead_beef);
    assert_eq!(reader.get_state().ge, 0b1011);

    let write = msr_masked(0, 0, 1);
    let mut writer = loaded(&write);
    let mut state = writer.get_state();
    state.core.itstate = 0x08;
    state.core.apsr = 0;
    state.core.registers[0] = 0xa805_0000;
    state.ge = 0b1011;
    writer.restore(state).unwrap();
    assert_eq!(writer.step().unwrap().mnemonic, "it-skip");
    assert_eq!(writer.get_state().core.apsr, 0);
    assert_eq!(writer.get_state().ge, 0b1011);
}

#[test]
fn dsp_members_obey_and_advance_it_state() {
    let encoded = qadd(0, 1, 2);
    let mut words = vec![movs(3, 1), it(0, 8)];
    words.extend(encoded);
    words.extend([bkpt(0)]);
    let mut skipped = loaded(&words);
    skipped.write_register(1, 4).unwrap();
    skipped.write_register(2, 5).unwrap();
    skipped.step().unwrap();
    skipped.step().unwrap();
    let trace = skipped.step().unwrap();
    assert_eq!(trace.mnemonic, "it-skip");
    assert_eq!(skipped.read_register(0).unwrap(), 0);
    assert_eq!(skipped.get_state().core.itstate, 0);

    let mut words = vec![movs(3, 1), it(1, 8)];
    words.extend(encoded);
    let mut executed = loaded(&words);
    executed.write_register(1, 4).unwrap();
    executed.write_register(2, 5).unwrap();
    executed.step().unwrap();
    executed.step().unwrap();
    assert_eq!(executed.step().unwrap().mnemonic, "qadd");
    assert_eq!(executed.read_register(0).unwrap(), 9);
}

#[test]
fn every_legal_it_condition_controls_dsp_execution() {
    let encoded = qadd(0, 1, 2);
    let true_flags = [
        (2, 1 << 29),
        (3, 0),
        (4, 1 << 31),
        (5, 0),
        (6, 1 << 28),
        (7, 0),
        (8, 1 << 29),
        (9, 0),
        (10, 0),
        (11, 1 << 31),
        (12, 0),
        (13, 1 << 30),
        (14, 0),
    ];
    for (condition, apsr) in true_flags {
        let mut simulator = loaded(&encoded);
        let mut state = simulator.get_state();
        state.core.itstate = (condition << 4) | 8;
        state.core.apsr = apsr;
        state.core.registers[1] = 4;
        state.core.registers[2] = 5;
        simulator.restore(state).unwrap();
        assert_eq!(simulator.step().unwrap().mnemonic, "qadd");
        assert_eq!(simulator.read_register(0).unwrap(), 9);
    }

    let mut multi = loaded(&[encoded[0], encoded[1], encoded[0], encoded[1]]);
    let mut state = multi.get_state();
    state.core.itstate = 0x24;
    state.core.apsr = 1 << 29;
    state.core.registers[1] = 1;
    state.core.registers[2] = 2;
    multi.restore(state).unwrap();
    multi.step().unwrap();
    assert_ne!(multi.get_state().core.itstate, 0);
}

#[test]
fn dsp_fetch_and_reserved_register_failures_are_typed_and_atomic() {
    let first = qadd(0, 1, 2)[0];
    let mut truncated = loaded(&[first]);
    let before = truncated.get_state();
    assert_eq!(
        truncated.step(),
        Err(CortexM7Error::Core(CortexM3Error::FetchOutsideProgram {
            pc: ORIGIN,
            width: 4,
        }))
    );
    assert_eq!(truncated.get_state(), before);

    let encoded = qadd(15, 1, 2);
    let mut invalid = loaded(&encoded);
    let before = invalid.get_state();
    assert!(matches!(
        invalid.step(),
        Err(CortexM7Error::Core(
            CortexM3Error::UnknownInstruction32 { .. }
        ))
    ));
    assert_eq!(invalid.get_state(), before);

    let mut short = CortexM7Simulator::new();
    short.load(&[0xfa], ORIGIN).unwrap();
    assert!(matches!(
        short.step(),
        Err(CortexM7Error::Core(CortexM3Error::FetchOutsideProgram {
            width: 2,
            ..
        }))
    ));
}

#[test]
fn false_it_conditions_still_reject_every_reserved_dsp_operand() {
    type DspEncoder = fn(u8, u8, u8) -> [u16; 2];
    let encoders: [DspEncoder; 4] = [qadd, qsub, sadd8, uadd8];
    for encode in encoders {
        for operand in 0..3 {
            for reserved in [13, 15] {
                let mut registers = [0, 1, 2];
                registers[operand] = reserved;
                let encoded = encode(registers[0], registers[1], registers[2]);
                let mut simulator = loaded(&encoded);
                let mut state = simulator.get_state();
                state.core.itstate = 0x08;
                state.core.apsr = 0;
                simulator.restore(state).unwrap();
                let before = simulator.get_state();

                assert!(matches!(
                    simulator.step(),
                    Err(CortexM7Error::Core(
                        CortexM3Error::UnknownInstruction32 { .. }
                    ))
                ));
                assert_eq!(simulator.get_state(), before);
            }
        }
    }
}

#[test]
fn register_memory_and_power_lifecycle_delegate_checked_behavior() {
    let mut simulator = loaded(&[0xbf20, bkpt(0)]);
    simulator.write_register(0, 0x1234_5678).unwrap();
    assert_eq!(simulator.read_register(0).unwrap(), 0x1234_5678);
    assert!(matches!(
        simulator.read_register(16),
        Err(CortexM7Error::Core(CortexM3Error::InvalidRegister {
            index: 16
        }))
    ));
    simulator.write_memory(0x200, 4, 0x89ab_cdef).unwrap();
    assert_eq!(simulator.read_memory(0x200, 4).unwrap(), 0x89ab_cdef);
    assert!(simulator.read_memory(MEMORY_SIZE as u32, 1).is_err());

    assert_eq!(simulator.step().unwrap().mnemonic, "wfe");
    assert!(matches!(
        simulator.step(),
        Err(CortexM7Error::Core(CortexM3Error::Sleeping))
    ));
    simulator.signal_event();
    assert!(!simulator.get_state().core.sleeping);
    simulator.signal_interrupt();
}

#[test]
fn run_is_bounded_and_rolls_back_on_fault_or_exhaustion() {
    let encoded = qadd(0, 1, 2);
    let mut words = Vec::from(encoded);
    words.push(bkpt(0));
    let mut simulator = loaded(&words);
    simulator.write_register(1, 1).unwrap();
    simulator.write_register(2, 2).unwrap();
    let result = simulator.run(2).unwrap();
    assert!(result.stopped);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces[0].mnemonic, "qadd");
    assert_eq!(result.final_state.core.registers[0], 3);
    assert!(matches!(
        simulator.step(),
        Err(CortexM7Error::Core(CortexM3Error::Halted))
    ));

    let mut exhausted = loaded(&[nop(), nop()]);
    let before = exhausted.get_state();
    assert_eq!(
        exhausted.run(2),
        Err(CortexM7Error::StepLimitExceeded { limit: 2 })
    );
    assert_eq!(exhausted.get_state(), before);

    let mut faulting = loaded(&[nop(), 0xde00]);
    let before = faulting.get_state();
    assert!(matches!(
        faulting.run(2),
        Err(CortexM7Error::Core(
            CortexM3Error::UnknownInstruction16 { .. }
        ))
    ));
    assert_eq!(faulting.get_state(), before);

    assert_eq!(
        exhausted.run(MAX_RETAINED_STEPS + 1),
        Err(CortexM7Error::RetentionLimitExceeded {
            requested: MAX_RETAINED_STEPS + 1,
            limit: MAX_RETAINED_STEPS,
        })
    );
}

#[test]
fn execute_is_atomic_and_retains_complete_traces() {
    let encoded = qsub(0, 1, 2);
    let mut words = Vec::from(encoded);
    words.push(bkpt(0));
    let mut simulator = CortexM7Simulator::new();
    let result = simulator.execute(&program(&words), 0x200, 2).unwrap();
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces[0].pc_before, 0x200);
    assert_eq!(result.traces[0].pc_after, 0x204);
    assert_eq!(result.traces[0].state_before.core.loaded_len, 6);
    assert_eq!(result.traces[0].state_after.core.loaded_len, 6);

    let before = simulator.get_state();
    assert!(simulator.execute(&[0], 1, 1).is_err());
    assert_eq!(simulator.get_state(), before);
    assert_eq!(
        simulator.execute(&program(&[nop(), nop()]), 0x200, 2),
        Err(CortexM7Error::StepLimitExceeded { limit: 2 })
    );
    assert_eq!(simulator.get_state(), before);
    assert_eq!(
        simulator.execute(&[0, 0], 0, MAX_RETAINED_STEPS + 1),
        Err(CortexM7Error::RetentionLimitExceeded {
            requested: MAX_RETAINED_STEPS + 1,
            limit: MAX_RETAINED_STEPS,
        })
    );
}

#[test]
fn cache_bus_and_timing_are_explicit_composition_boundaries() {
    let readme = include_str!("../README.md");
    for boundary in ["caches", "TCM", "branch prediction", "AXI/AHB", "timing"] {
        assert!(readme.contains(boundary), "missing boundary {boundary}");
    }
}

#[test]
fn every_error_variant_has_stable_display_text() {
    let errors = [
        CortexM7Error::Core(CortexM3Error::EmptyProgram),
        CortexM7Error::InvalidState { reason: "test" },
        CortexM7Error::RetentionLimitExceeded {
            requested: 65,
            limit: 64,
        },
        CortexM7Error::StepLimitExceeded { limit: 1 },
        CortexM7Error::AllocationFailed,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}
