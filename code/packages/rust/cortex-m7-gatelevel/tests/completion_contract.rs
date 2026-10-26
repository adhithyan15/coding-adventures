use cortex_m7_gatelevel::{
    classify,
    encoding::{self, *},
    CortexM7Error, CortexM7GateLevel, CortexM7State, InstructionClass, FLIP_FLOP_COUNT,
    MAX_RETAINED_STEPS, MEMORY_SIZE,
};

const ORIGIN: u32 = 0x100;
const APSR_Q: u32 = 1 << 27;

fn loaded(words: &[u16]) -> CortexM7GateLevel {
    let mut core = CortexM7GateLevel::new();
    core.load(&program(words), ORIGIN).unwrap();
    core
}

#[test]
fn topology_reset_load_and_restore_are_complete() {
    let mut core = CortexM7GateLevel::new();
    assert_eq!(core.flip_flop_count(), 524_956);
    assert_eq!(FLIP_FLOP_COUNT, 524_956);
    assert!(core.gate_count() > FLIP_FLOP_COUNT * 6);
    assert_eq!(core.get_state().core.memory.len(), MEMORY_SIZE);

    core.load(&program(&qadd(0, 1, 2)), ORIGIN).unwrap();
    let mut state = core.get_state();
    state.ge = 0b1010;
    state.core.registers[4] = 99;
    core.restore(state.clone()).unwrap();
    assert_eq!(core.get_state(), state);

    let mut observer = core.get_state();
    observer.ge = 0;
    observer.core.registers[4] = 0;
    assert_eq!(core.get_state(), state);

    core.reset();
    assert_eq!(core.get_state(), CortexM7State::default());
}

#[test]
fn invalid_lifecycle_inputs_are_typed_and_atomic() {
    let mut core = loaded(&[bkpt(0)]);
    let before = core.get_state();
    assert!(core.load(&[], 0).is_err());
    assert!(core.load(&[0, 0], 1).is_err());
    let mut bad_ge = before.clone();
    bad_ge.ge = 0x10;
    assert!(matches!(
        core.restore(bad_ge),
        Err(CortexM7Error::InvalidState { .. })
    ));
    let mut bad_core = before.clone();
    bad_core.core.memory.pop();
    assert!(matches!(
        core.restore(bad_core),
        Err(CortexM7Error::Core(_))
    ));
    assert_eq!(core.get_state(), before);
}

#[test]
fn decode_and_reserved_dsp_operands_are_extension_first() {
    for (words, class) in [
        (qadd(0, 1, 2), InstructionClass::Qadd),
        (qsub(0, 1, 2), InstructionClass::Qsub),
        (sadd8(0, 1, 2), InstructionClass::Sadd8),
        (uadd8(0, 1, 2), InstructionClass::Uadd8),
    ] {
        assert_eq!(classify(words[0], Some(words[1])), Ok(class));
    }
    assert!(matches!(
        classify(nop(), None),
        Ok(InstructionClass::Inherited(_))
    ));
    assert!(classify(0xfa80, Some(0xf020)).is_err());

    type Encoder = fn(u8, u8, u8) -> [u16; 2];
    for encode in [qadd, qsub, sadd8, uadd8] as [Encoder; 4] {
        for rd in 0..16 {
            for rn in 0..16 {
                for rm in 0..16 {
                    let words = encode(rd, rn, rm);
                    let mut core = loaded(&words);
                    let mut state = core.get_state();
                    state.core.itstate = 0x08;
                    state.core.apsr = 0;
                    core.restore(state).unwrap();
                    let before = core.get_state();
                    let result = core.step();
                    if rd <= 12 && rn <= 12 && rm <= 12 {
                        assert!(result.is_ok(), "{rd}/{rn}/{rm}");
                    } else {
                        assert!(result.is_err(), "{rd}/{rn}/{rm}");
                        assert_eq!(core.get_state(), before);
                    }
                }
            }
        }
    }
}

#[test]
fn saturation_parallel_lanes_and_status_transport_match_the_contract() {
    let mut q = loaded(&[
        qadd(0, 1, 2)[0],
        qadd(0, 1, 2)[1],
        qsub(3, 4, 5)[0],
        qsub(3, 4, 5)[1],
    ]);
    q.write_register(1, i32::MAX as u32).unwrap();
    q.write_register(2, 1).unwrap();
    assert_eq!(q.step().unwrap().mnemonic, "qadd");
    assert_eq!(q.read_register(0).unwrap(), i32::MAX as u32);
    assert_ne!(q.get_state().core.apsr & APSR_Q, 0);
    q.write_register(4, i32::MIN as u32).unwrap();
    q.write_register(5, 1).unwrap();
    q.step().unwrap();
    assert_eq!(q.read_register(3).unwrap(), i32::MIN as u32);
    assert_ne!(q.get_state().core.apsr & APSR_Q, 0);

    let mut signed = loaded(&sadd8(0, 1, 2));
    signed.write_register(1, 0x7f80_ff01).unwrap();
    signed.write_register(2, 0x0101_01ff).unwrap();
    signed.step().unwrap();
    assert_eq!(signed.read_register(0).unwrap(), 0x8081_0000);
    assert_eq!(signed.get_state().ge, 0b1011);

    let mut unsigned = loaded(&uadd8(0, 1, 2));
    unsigned.write_register(1, 0xffff_0100).unwrap();
    unsigned.write_register(2, 0x0101_ffff).unwrap();
    unsigned.step().unwrap();
    assert_eq!(unsigned.read_register(0).unwrap(), 0x0000_00ff);
    assert_eq!(unsigned.get_state().ge, 0b1110);

    for sysm in 0..=3 {
        let write = msr(0, sysm);
        let read = mrs(1, sysm);
        let mut core = loaded(&[write[0], write[1], read[0], read[1]]);
        core.write_register(0, 0xa80b_0000).unwrap();
        core.step().unwrap();
        assert_eq!(core.get_state().ge, 0b1011);
        core.step().unwrap();
        assert_eq!(core.read_register(1).unwrap(), 0xa80b_0000);
    }
}

#[test]
fn every_ordered_lane_pair_has_exact_result_and_ge() {
    use cortex_m7_gatelevel::gates::{parallel_add, RepositoryGates};
    for a in 0_u32..=255 {
        for b in 0_u32..=255 {
            let (unsigned, uge) = parallel_add(&mut RepositoryGates, a, b, 0);
            assert_eq!(unsigned & 0xff, (a + b) & 0xff);
            assert_eq!(uge & 1, u8::from(a + b > 255));

            let sa = i16::from(a as u8 as i8);
            let sb = i16::from(b as u8 as i8);
            let (signed, sge) = parallel_add(&mut RepositoryGates, a, b, 1);
            assert_eq!(signed & 0xff, u32::from((sa + sb) as u8));
            assert_eq!(sge & 1, u8::from(sa + sb >= 0));
        }
    }
}

#[test]
fn reduced_add_sub_exhausts_carry_overflow_saturation_and_sticky_q() {
    use cortex_m7_gatelevel::gates::{reduced_saturating_8, RepositoryGates};
    for left in 0_u16..=255 {
        for right in 0_u16..=255 {
            for subtract in [false, true] {
                for old_q in [0, 1] {
                    let (result, carry, overflow, q) = reduced_saturating_8(
                        &mut RepositoryGates,
                        left as u8,
                        right as u8,
                        u8::from(subtract),
                        old_q,
                    );
                    let signed = if subtract {
                        i16::from(left as u8 as i8) - i16::from(right as u8 as i8)
                    } else {
                        i16::from(left as u8 as i8) + i16::from(right as u8 as i8)
                    };
                    let saturated = signed.clamp(i16::from(i8::MIN), i16::from(i8::MAX));
                    let expected_overflow = u8::from(signed != saturated);
                    let expected_carry = if subtract {
                        u8::from(left >= right)
                    } else {
                        u8::from(left + right > 255)
                    };
                    assert_eq!(result, saturated as i8 as u8);
                    assert_eq!(carry, expected_carry);
                    assert_eq!(overflow, expected_overflow);
                    assert_eq!(q, old_q | expected_overflow);
                }
            }
        }
    }
}

#[test]
fn every_condition_and_it_advance_path_matches_arm_truth() {
    use cortex_m7_gatelevel::gates::{condition_holds, RepositoryGates};
    for flags in 0_u32..16 {
        let apsr = flags << 28;
        let n = flags & 8 != 0;
        let z = flags & 4 != 0;
        let c = flags & 2 != 0;
        let v = flags & 1 != 0;
        let expected = [
            z,
            !z,
            c,
            !c,
            n,
            !n,
            v,
            !v,
            c && !z,
            !c || z,
            n == v,
            n != v,
            !z && n == v,
            z || n != v,
            true,
            false,
        ];
        for (condition, expected) in expected.into_iter().enumerate() {
            assert_eq!(
                condition_holds(&mut RepositoryGates, apsr, condition as u8),
                u8::from(expected)
            );
        }
    }
}

#[test]
fn it_skip_fetch_fault_halt_and_allocation_paths_are_atomic() {
    let mut skipped = loaded(&qadd(0, 1, 2));
    let mut state = skipped.get_state();
    state.core.itstate = 0x08;
    state.core.apsr = 0;
    state.core.registers[0] = 77;
    skipped.restore(state).unwrap();
    assert_eq!(skipped.step().unwrap().mnemonic, "it-skip");
    assert_eq!(skipped.read_register(0).unwrap(), 77);

    let mut truncated = loaded(&[qadd(0, 1, 2)[0]]);
    let before = truncated.get_state();
    assert!(truncated.step().is_err());
    assert_eq!(truncated.get_state(), before);

    let mut halted = loaded(&[bkpt(0)]);
    halted.step().unwrap();
    let before = halted.get_state();
    assert!(halted.step().is_err());
    assert_eq!(halted.get_state(), before);

    let mut allocation = loaded(&qadd(0, 1, 2));
    let before = allocation.get_state();
    allocation.fail_allocation_after(0);
    assert_eq!(allocation.step(), Err(CortexM7Error::AllocationFailed));
    assert_eq!(allocation.get_state(), before);
}

#[test]
fn run_and_execute_preflight_and_roll_back_whole_invocations() {
    let mut core = loaded(&[movs(0, 1), 0xffff]);
    let before = core.get_state();
    assert!(core.run(2).is_err());
    assert_eq!(core.get_state(), before);
    assert!(matches!(
        core.run(MAX_RETAINED_STEPS + 1),
        Err(CortexM7Error::RetentionLimitExceeded { .. })
    ));

    let mut execute = loaded(&[bkpt(0)]);
    let before = execute.get_state();
    assert!(execute.execute(&program(&[movs(0, 1)]), ORIGIN, 1).is_err());
    assert_eq!(execute.get_state(), before);

    let mut success = CortexM7GateLevel::new();
    let result = success
        .execute(&program(&[movs(0, 7), bkpt(0)]), ORIGIN, 2)
        .unwrap();
    assert!(result.stopped);
    assert_eq!(result.final_state.core.registers[0], 7);
    assert_eq!(
        result.traces[0].raw,
        encoding::program(&[movs(0, 7), 0, 0])[..4]
    );
}

#[test]
fn deterministic_failure_covers_every_step_run_and_execute_allocation_site() {
    for failure in 0..=4 {
        let mut core = loaded(&qadd(0, 1, 2));
        let before = core.get_state();
        core.fail_allocation_after(failure);
        assert_eq!(core.step(), Err(CortexM7Error::AllocationFailed));
        assert_eq!(core.get_state(), before, "step allocation {failure}");
    }

    for failure in 0..=10 {
        let mut core = loaded(&[bkpt(0)]);
        let before = core.get_state();
        core.fail_allocation_after(failure);
        assert_eq!(core.run(1), Err(CortexM7Error::AllocationFailed));
        assert_eq!(core.get_state(), before, "run allocation {failure}");
    }

    for failure in 0..=12 {
        let mut core = loaded(&[bkpt(0)]);
        let before = core.get_state();
        core.fail_allocation_after(failure);
        assert_eq!(
            core.execute(&program(&[bkpt(0)]), ORIGIN, 1),
            Err(CortexM7Error::AllocationFailed)
        );
        assert_eq!(core.get_state(), before, "execute allocation {failure}");
    }
}
