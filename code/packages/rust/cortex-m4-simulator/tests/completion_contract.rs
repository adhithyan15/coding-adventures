use cortex_m4_simulator::{
    classify,
    core_encoding::{bkpt, it, movs, nop, program},
    encoding::*,
    CortexM3Error, CortexM4Error, CortexM4Simulator, CortexM4State, InstructionClass,
    MAX_RETAINED_STEPS, MEMORY_SIZE,
};

const ORIGIN: u32 = 0x100;

fn words(parts: &[[u16; 2]]) -> Vec<u16> {
    parts.iter().flatten().copied().collect()
}

fn loaded(halfwords: &[u16]) -> CortexM4Simulator {
    let mut simulator = CortexM4Simulator::new();
    simulator.load(&program(halfwords), ORIGIN).unwrap();
    simulator
}

fn loaded_fpu(halfwords: &[u16]) -> CortexM4Simulator {
    let mut simulator = loaded(halfwords);
    simulator.set_fpu_enabled(true);
    simulator
}

#[test]
fn decode_families_are_explicit_and_inherited() {
    let cases = [
        (qadd(0, 1, 2), InstructionClass::Saturating),
        (ssat(0, 8, 1), InstructionClass::Saturating),
        (sadd8(0, 1, 2), InstructionClass::ParallelBytes),
        (smuad(0, 1, 2), InstructionClass::DspMultiply),
        (vadd_f32(0, 1, 2), InstructionClass::FloatingPointBinary),
        (vmov_f32(0, 1), InstructionClass::FloatingPointUnary),
        (vcmp_f32(0, 1), InstructionClass::FloatingPointCompare),
        (vmrs(0), InstructionClass::FloatingPointStatus),
    ];
    for ([first, second], expected) in cases {
        assert_eq!(classify(first, Some(second)), Ok(expected));
    }
    assert!(matches!(
        classify(movs(0, 1), None),
        Ok(InstructionClass::Core(_))
    ));
    assert!(matches!(
        classify(0xf000, Some(0)),
        Err(CortexM4Error::Core(
            CortexM3Error::UnknownInstruction32 { .. }
        ))
    ));
}

#[test]
fn reset_load_and_restore_cover_complete_state() {
    let mut simulator = CortexM4Simulator::new();
    let reset = simulator.get_state();
    assert_eq!(reset.core.memory.len(), MEMORY_SIZE);
    assert_eq!(reset.core.msp, MEMORY_SIZE as u32);
    assert_eq!(reset.ge, 0);
    assert!(!reset.fpu_enabled);
    assert_eq!(reset.s_registers, [0; 32]);

    simulator.load(&program(&[nop(), bkpt(0)]), ORIGIN).unwrap();
    simulator.set_fpu_enabled(true);
    simulator.write_s_register(31, 0x3f80_0000).unwrap();
    let state = simulator.get_state();
    assert_eq!(state.core.loaded_origin, ORIGIN);
    assert_eq!(state.core.loaded_len, 4);
    let mut restored = CortexM4Simulator::new();
    restored.restore(state.clone()).unwrap();
    assert_eq!(restored.get_state(), state);
    restored.reset();
    assert_eq!(restored.get_state(), CortexM4State::default());
}

#[test]
fn load_and_restore_validation_are_atomic() {
    let mut simulator = loaded(&[bkpt(0)]);
    let before = simulator.get_state();
    assert!(matches!(
        simulator.load(&[], 0),
        Err(CortexM4Error::Core(CortexM3Error::EmptyProgram))
    ));
    assert_eq!(simulator.get_state(), before);

    for invalid in [
        CortexM4State {
            ge: 0x10,
            ..before.clone()
        },
        CortexM4State {
            fpscr: 1 << 8,
            fpu_enabled: true,
            ..before.clone()
        },
    ] {
        assert!(simulator.restore(invalid).is_err());
        assert_eq!(simulator.get_state(), before);
    }
    let mut invalid_core = before.clone();
    invalid_core.core.memory.pop();
    assert!(matches!(
        simulator.restore(invalid_core),
        Err(CortexM4Error::Core(CortexM3Error::InvalidState { .. }))
    ));
    assert_eq!(simulator.get_state(), before);

    let mut retained_fpu = before.clone();
    retained_fpu.s_registers[0] = 1.0_f32.to_bits();
    simulator.restore(retained_fpu.clone()).unwrap();
    assert_eq!(simulator.get_state(), retained_fpu);
}

#[test]
fn inherited_integer_execution_has_complete_outer_trace() {
    let mut simulator = loaded(&[movs(0, 7), bkpt(0)]);
    let trace = simulator.step().unwrap();
    assert_eq!(trace.mnemonic, "movs");
    assert_eq!(trace.width, 2);
    assert_eq!(trace.raw[..2], movs(0, 7).to_le_bytes());
    assert_eq!(trace.state_before.core.pc, ORIGIN);
    assert_eq!(trace.state_after.core.registers[0], 7);
    assert_eq!(simulator.step().unwrap().mnemonic, "bkpt 0");
    assert_eq!(
        simulator.step(),
        Err(CortexM4Error::Core(CortexM3Error::Halted))
    );
}

#[test]
fn qadd_and_qsub_saturate_and_set_sticky_q() {
    let code = words(&[qadd(0, 1, 2), qsub(3, 4, 5)]);
    let mut simulator = loaded(&code);
    simulator.write_register(1, i32::MAX as u32).unwrap();
    simulator.write_register(2, 1).unwrap();
    simulator.write_register(4, i32::MIN as u32).unwrap();
    simulator.write_register(5, 1).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "qadd");
    assert_eq!(simulator.read_register(0).unwrap(), i32::MAX as u32);
    assert_ne!(simulator.get_state().core.apsr & (1 << 27), 0);
    assert_eq!(simulator.step().unwrap().mnemonic, "qsub");
    assert_eq!(simulator.read_register(3).unwrap(), i32::MIN as u32);
}

#[test]
fn signed_and_unsigned_saturation_cover_both_limits() {
    let code = words(&[ssat(0, 8, 1), usat(2, 8, 3)]);
    let mut simulator = loaded(&code);
    simulator.write_register(1, 500).unwrap();
    simulator.write_register(3, (-4_i32) as u32).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "ssat");
    assert_eq!(simulator.read_register(0).unwrap(), 127);
    assert_eq!(simulator.step().unwrap().mnemonic, "usat");
    assert_eq!(simulator.read_register(2).unwrap(), 0);

    let mut boundaries = loaded(&words(&[ssat(4, 32, 5), usat(6, 0, 7)]));
    boundaries.write_register(5, 0x8000_0000).unwrap();
    boundaries.write_register(7, 0).unwrap();
    boundaries.step().unwrap();
    boundaries.step().unwrap();
    assert_eq!(boundaries.read_register(4).unwrap(), 0x8000_0000);
    assert_eq!(boundaries.read_register(6).unwrap(), 0);
}

#[test]
fn parallel_byte_adds_update_ge_per_lane() {
    let mut signed = loaded(&words(&[sadd8(0, 1, 2)]));
    signed.write_register(1, 0x7f80_01ff).unwrap();
    signed.write_register(2, 0x0101_ff01).unwrap();
    assert_eq!(signed.step().unwrap().mnemonic, "sadd8");
    assert_eq!(signed.read_register(0).unwrap(), 0x8081_0000);
    assert_eq!(signed.get_state().ge, 0b1011);

    let mut unsigned = loaded(&words(&[uadd8(3, 4, 5)]));
    unsigned.write_register(4, 0xff01_f080).unwrap();
    unsigned.write_register(5, 0x0101_207f).unwrap();
    assert_eq!(unsigned.step().unwrap().mnemonic, "uadd8");
    assert_eq!(unsigned.read_register(3).unwrap(), 0x0002_10ff);
    assert_eq!(unsigned.get_state().ge, 0b1010);
}

#[test]
fn dual_halfword_multiply_and_accumulate_are_signed() {
    let code = words(&[smuad(0, 1, 2), smlad(3, 4, 5, 6)]);
    let mut simulator = loaded(&code);
    simulator.write_register(1, 0x0003_fffe).unwrap();
    simulator.write_register(2, 0xfffc_0005).unwrap();
    simulator.write_register(4, 0x0002_0003).unwrap();
    simulator.write_register(5, 0x0004_0005).unwrap();
    simulator.write_register(6, 7).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "smuad");
    assert_eq!(simulator.read_register(0).unwrap(), (-22_i32) as u32);
    assert_eq!(simulator.step().unwrap().mnemonic, "smlad");
    assert_eq!(simulator.read_register(3).unwrap(), 30);

    let mut overflow = loaded(&words(&[smlad(0, 1, 2, 3)]));
    overflow.write_register(1, 0x7fff_7fff).unwrap();
    overflow.write_register(2, 0x7fff_7fff).unwrap();
    overflow.write_register(3, i32::MAX as u32).unwrap();
    overflow.step().unwrap();
    assert_ne!(overflow.get_state().core.apsr & (1 << 27), 0);
}

#[test]
fn disabled_fpu_fault_is_typed_and_atomic() {
    let mut simulator = loaded(&words(&[vadd_f32(0, 1, 2)]));
    let before = simulator.get_state();
    assert_eq!(
        simulator.step(),
        Err(CortexM4Error::FloatingPointUnavailable { pc: ORIGIN })
    );
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn fpu_configuration_survives_load_and_execute_but_reset_disables_it() {
    let mut simulator = CortexM4Simulator::new();
    simulator.set_fpu_enabled(true);
    simulator
        .load(&program(&words(&[vadd_f32(0, 1, 2)])), ORIGIN)
        .unwrap();
    assert!(simulator.fpu_enabled());
    simulator.write_s_register(1, 1.0_f32.to_bits()).unwrap();
    simulator.write_s_register(2, 2.0_f32.to_bits()).unwrap();
    simulator.step().unwrap();
    simulator.set_fpu_enabled(false);
    assert_eq!(simulator.read_s_register(0).unwrap(), 3.0_f32.to_bits());
    simulator.reset();
    assert!(!simulator.fpu_enabled());
    assert_eq!(simulator.read_s_register(0).unwrap(), 0);
}

#[test]
fn binary_fpu_arithmetic_executes_all_bounded_operations() {
    let code = words(&[
        vadd_f32(0, 1, 2),
        vsub_f32(3, 4, 5),
        vmul_f32(6, 7, 8),
        vdiv_f32(9, 10, 11),
    ]);
    let mut simulator = loaded_fpu(&code);
    for (register, value) in [
        (1, 1.5_f32),
        (2, 2.25),
        (4, 7.0),
        (5, 2.0),
        (7, -3.0),
        (8, 4.0),
        (10, 9.0),
        (11, 2.0),
    ] {
        simulator
            .write_s_register(register, value.to_bits())
            .unwrap();
    }
    let expected = [
        ("vadd.f32", 0, 3.75),
        ("vsub.f32", 3, 5.0),
        ("vmul.f32", 6, -12.0),
        ("vdiv.f32", 9, 4.5),
    ];
    for (mnemonic, register, value) in expected {
        let trace = simulator.step().unwrap();
        assert_eq!(trace.mnemonic, mnemonic);
        assert_eq!(
            f32::from_bits(simulator.read_s_register(register).unwrap()),
            value
        );
    }
}

#[test]
fn unary_fpu_operations_cover_bits_and_square_root() {
    let code = words(&[
        vmov_f32(0, 1),
        vabs_f32(2, 3),
        vneg_f32(4, 5),
        vsqrt_f32(6, 7),
    ]);
    let mut simulator = loaded_fpu(&code);
    simulator.write_s_register(1, (-2.0_f32).to_bits()).unwrap();
    simulator.write_s_register(3, (-3.0_f32).to_bits()).unwrap();
    simulator.write_s_register(5, 4.0_f32.to_bits()).unwrap();
    simulator.write_s_register(7, 9.0_f32.to_bits()).unwrap();
    for expected in ["vmov.f32", "vabs.f32", "vneg.f32", "vsqrt.f32"] {
        assert_eq!(simulator.step().unwrap().mnemonic, expected);
    }
    assert_eq!(f32::from_bits(simulator.read_s_register(0).unwrap()), -2.0);
    assert_eq!(f32::from_bits(simulator.read_s_register(2).unwrap()), 3.0);
    assert_eq!(f32::from_bits(simulator.read_s_register(4).unwrap()), -4.0);
    assert_eq!(f32::from_bits(simulator.read_s_register(6).unwrap()), 3.0);
}

#[test]
fn compare_and_status_transport_cover_fpscr_nzcv() {
    let code = words(&[vcmp_f32(0, 1), vmrs(2), vmsr(3)]);
    let mut simulator = loaded_fpu(&code);
    simulator.write_s_register(0, 1.0_f32.to_bits()).unwrap();
    simulator.write_s_register(1, 2.0_f32.to_bits()).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "vcmp.f32");
    assert_eq!(simulator.get_state().fpscr >> 28, 0b1000);
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(2).unwrap(), 0x8000_0000);
    simulator.write_register(3, 0x6000_0010).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "vmsr");
    assert_eq!(simulator.get_state().fpscr, 0x6000_0010);
}

#[test]
fn floating_point_exception_flags_and_nan_are_deterministic() {
    let code = words(&[
        vdiv_f32(0, 1, 2),
        vmul_f32(3, 4, 5),
        vsqrt_f32(6, 7),
        vcmp_f32(8, 9),
    ]);
    let mut simulator = loaded_fpu(&code);
    simulator.write_s_register(1, 1.0_f32.to_bits()).unwrap();
    simulator.write_s_register(2, 0.0_f32.to_bits()).unwrap();
    simulator.write_s_register(4, f32::MAX.to_bits()).unwrap();
    simulator.write_s_register(5, 2.0_f32.to_bits()).unwrap();
    simulator.write_s_register(7, (-1.0_f32).to_bits()).unwrap();
    simulator.write_s_register(8, f32::NAN.to_bits()).unwrap();
    simulator.write_s_register(9, 1.0_f32.to_bits()).unwrap();
    simulator.step().unwrap();
    assert_ne!(simulator.get_state().fpscr & (1 << 1), 0);
    simulator.step().unwrap();
    assert_ne!(simulator.get_state().fpscr & (1 << 2), 0);
    simulator.step().unwrap();
    assert_eq!(simulator.read_s_register(6).unwrap(), 0x7fc0_0000);
    assert_ne!(simulator.get_state().fpscr & 1, 0);
    simulator.step().unwrap();
    assert_eq!(simulator.get_state().fpscr >> 28, 0b0011);
}

#[test]
fn fpscr_reserved_control_write_faults_atomically() {
    let mut simulator = loaded_fpu(&words(&[vmsr(0)]));
    simulator.write_register(0, 1 << 22).unwrap();
    let before = simulator.get_state();
    assert_eq!(
        simulator.step(),
        Err(CortexM4Error::UnsupportedFpscr { value: 1 << 22 })
    );
    assert_eq!(simulator.get_state(), before);
}

#[test]
fn extension_instructions_obey_it_conditions() {
    let qadd_words = qadd(1, 2, 3);
    let mut skipped = loaded(&[movs(0, 1), it(0, 8), qadd_words[0], qadd_words[1]]);
    skipped.write_register(2, 10).unwrap();
    skipped.write_register(3, 20).unwrap();
    skipped.step().unwrap();
    skipped.step().unwrap();
    assert_eq!(skipped.step().unwrap().mnemonic, "it-skip");
    assert_eq!(skipped.read_register(1).unwrap(), 0);
    assert_eq!(skipped.get_state().core.itstate, 0);

    let mut executed = loaded(&[it(1, 8), qadd_words[0], qadd_words[1]]);
    executed.write_register(2, 10).unwrap();
    executed.write_register(3, 20).unwrap();
    executed.step().unwrap();
    assert_eq!(executed.step().unwrap().mnemonic, "qadd");
    assert_eq!(executed.read_register(1).unwrap(), 30);
}

#[test]
fn register_memory_and_signal_apis_are_checked() {
    let mut simulator = loaded(&[0xbf30, nop()]);
    simulator.write_register(13, 0x203).unwrap();
    assert_eq!(simulator.read_register(13).unwrap(), 0x200);
    assert!(matches!(
        simulator.write_register(16, 0),
        Err(CortexM4Error::Core(CortexM3Error::InvalidRegister { .. }))
    ));
    simulator.write_memory(0x200, 4, 0x4433_2211).unwrap();
    assert_eq!(simulator.read_memory(0x200, 4).unwrap(), 0x4433_2211);
    assert!(simulator.read_memory(0xffff, 4).is_err());
    assert!(simulator.write_s_register(32, 0).is_err());
    assert!(simulator.read_s_register(32).is_err());

    simulator.step().unwrap();
    assert!(simulator.get_state().core.sleeping);
    simulator.signal_event();
    assert!(simulator.get_state().core.sleeping);
    simulator.signal_interrupt();
    assert!(!simulator.get_state().core.sleeping);
}

#[test]
fn run_and_execute_are_bounded_and_transactional() {
    let mut simulator = loaded(&[nop()]);
    let before = simulator.get_state();
    assert!(matches!(
        simulator.run(1),
        Err(CortexM4Error::Core(CortexM3Error::StepLimitExceeded {
            limit: 1
        }))
    ));
    assert_eq!(simulator.get_state(), before);
    assert!(matches!(
        simulator.run(MAX_RETAINED_STEPS + 1),
        Err(CortexM4Error::Core(
            CortexM3Error::RetentionLimitExceeded { .. }
        ))
    ));

    let result = simulator
        .execute(&program(&[movs(0, 9), bkpt(0)]), ORIGIN, 2)
        .unwrap();
    assert!(result.stopped);
    assert_eq!(result.steps, 2);
    assert_eq!(result.final_state.core.registers[0], 9);

    let after = simulator.get_state();
    assert!(simulator.execute(&[], 0, 1).is_err());
    assert_eq!(simulator.get_state(), after);
}

#[test]
fn reserved_extension_operands_are_typed_and_atomic() {
    for [first, second] in [qadd(15, 1, 2), qsub(0, 13, 2), smuad(0, 1, 15), vmrs(15)] {
        let mut simulator = loaded(&[first, second]);
        if first & 0xff00 == 0xee00 {
            simulator.set_fpu_enabled(true);
        }
        let before = simulator.get_state();
        assert!(matches!(
            simulator.step(),
            Err(CortexM4Error::UnknownInstruction32 { .. })
        ));
        assert_eq!(simulator.get_state(), before);
    }
}

#[test]
fn encoders_match_reference_halfwords() {
    assert_eq!(qadd(0, 1, 2), [0xfa82, 0xf081]);
    assert_eq!(qsub(3, 4, 5), [0xfa85, 0xf3a4]);
    assert_eq!(ssat(6, 8, 7), [0xf307, 0x0607]);
    assert_eq!(usat(0, 16, 1), [0xf381, 0x0010]);
    assert_eq!(sadd8(2, 3, 4), [0xfa83, 0xf204]);
    assert_eq!(uadd8(5, 6, 7), [0xfa86, 0xf547]);
    assert_eq!(smuad(0, 1, 2), [0xfb21, 0xf002]);
    assert_eq!(smlad(3, 4, 5, 6), [0xfb24, 0x6305]);
    assert_eq!(vadd_f32(0, 1, 2), [0xee30, 0x0a81]);
    assert_eq!(vsub_f32(3, 4, 5), [0xee72, 0x1a62]);
    assert_eq!(vmul_f32(6, 7, 8), [0xee23, 0x3a84]);
    assert_eq!(vdiv_f32(9, 10, 11), [0xeec5, 0x4a25]);
    assert_eq!(vmov_f32(12, 13), [0xeeb0, 0x6a66]);
    assert_eq!(vabs_f32(14, 15), [0xeeb0, 0x7ae7]);
    assert_eq!(vneg_f32(16, 17), [0xeeb1, 0x8a68]);
    assert_eq!(vsqrt_f32(18, 19), [0xeeb1, 0x9ae9]);
    assert_eq!(vcmp_f32(20, 21), [0xeeb4, 0xaa6a]);
    assert_eq!(vmrs(0), [0xeef1, 0x0a10]);
    assert_eq!(vmsr(1), [0xeee1, 0x1a10]);
}
