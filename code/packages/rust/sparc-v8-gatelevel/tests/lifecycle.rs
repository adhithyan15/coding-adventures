use coding_adventures_sparc_v8_gatelevel::alu::{sdiv64_with_overflow, udiv64_with_overflow};
use coding_adventures_sparc_v8_gatelevel::{SparcCpu, SparcError, SparcState, FLIP_FLOP_COUNT};
use sparc_v8_simulator::encoding::{
    assemble, encode_add_imm, encode_alu_imm, encode_bicc, encode_ld, encode_save, encode_ta,
    encode_ticc, encode_udiv,
};
use sparc_v8_simulator::execute::Psr;
use sparc_v8_simulator::MEMORY_SIZE;

#[test]
fn exact_topology_and_restore_are_atomic() {
    let mut cpu = SparcCpu::new();
    assert_eq!(FLIP_FLOP_COUNT, 526_185);
    assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);
    assert_eq!(cpu.get_state().npc, 4);
    let before = cpu.get_state();

    let mut invalid = before.clone();
    invalid.regs[0] = 1;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(SparcError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(SparcError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    let mutations: [fn(&mut SparcState); 5] = [
        |state| state.cwp = 3,
        |state| state.save_depth = 3,
        |state| state.pc = 1,
        |state| state.npc = 8,
        |state| state.loaded_origin = 1,
    ];
    for mutate in mutations {
        invalid = before.clone();
        mutate(&mut invalid);
        assert!(matches!(
            cpu.restore(&invalid),
            Err(SparcError::InvalidState(_))
        ));
        assert_eq!(cpu.get_state(), before);
    }
}

#[test]
fn checked_load_and_direct_access_are_typed_and_atomic() {
    let mut cpu = SparcCpu::new();
    cpu.write_byte_checked(60, 0xaa).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.load_at_checked(&[0; 8], (MEMORY_SIZE - 4) as u32),
        Err(SparcError::ProgramOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.load_at_checked(&[0; 4], 1),
        Err(SparcError::MisalignedProgram { origin: 1 })
    );
    cpu.load_checked(&assemble(&[encode_ta(0)])).unwrap();
    assert_eq!(cpu.read_byte_checked(60).unwrap(), 0);

    assert_eq!(
        cpu.read_register_checked(32),
        Err(SparcError::InvalidRegister { index: 32 })
    );
    cpu.write_register_checked(2, 0xdead_beef).unwrap();
    assert_eq!(cpu.read_register_checked(2).unwrap(), 0xdead_beef);
    cpu.write_word_checked(4, 0x1020_3040).unwrap();
    assert_eq!(cpu.read_word_checked(4).unwrap(), 0x1020_3040);
    assert_eq!(cpu.read_byte_checked(4).unwrap(), 0x10);
    assert!(matches!(
        cpu.read_word_checked(5),
        Err(SparcError::MisalignedAccess { .. })
    ));
    assert!(matches!(
        cpu.write_word_checked(MEMORY_SIZE as u32, 0),
        Err(SparcError::MemoryOutOfRange { .. })
    ));
}

#[test]
fn faults_halt_and_transactional_runs_preserve_complete_state() {
    let mut cpu = SparcCpu::new();
    cpu.load_checked(&[0, 0, 0]).unwrap();
    assert_eq!(
        cpu.step_checked(),
        Err(SparcError::TruncatedInstruction { pc: 0 })
    );

    let fault_words = [
        encode_udiv(4, 2, 0),
        encode_ticc(1, 0, 0),
        encode_alu_imm(0x3f, 4, 2, 3),
        encode_ld(4, 1, 1),
    ];
    for word in fault_words {
        cpu.load_checked(&assemble(&[word])).unwrap();
        let before = cpu.get_state();
        assert!(cpu.step_checked().is_err(), "{word:#010x}");
        assert_eq!(cpu.get_state(), before, "{word:#010x} must be atomic");
    }

    cpu.load_checked(&assemble(&[encode_save(4, 2, 3)]))
        .unwrap();
    let mut overflow = cpu.get_state();
    overflow.save_depth = 2;
    cpu.restore(&overflow).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.step_checked(),
        Err(SparcError::WindowOverflow { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    let program = assemble(&[encode_add_imm(2, 0, 42), encode_ta(0)]);
    let result = cpu.run_checked(&program, 10).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.final_state, cpu.get_state());

    let bad = assemble(&[encode_add_imm(2, 0, 7), encode_ld(3, 0, 1)]);
    cpu.load_checked(&bad).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.run_loaded_checked(10),
        Err(SparcError::MisalignedAccess { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&assemble(&[encode_ta(0)])).unwrap();
    cpu.step_checked().unwrap();
    let halted = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(SparcError::Halted));
    assert_eq!(cpu.get_state(), halted);
}

#[test]
fn all_sixteen_branch_conditions_match_the_manual_truth_table() {
    fn expected(cond: u32, psr: Psr) -> bool {
        let Psr { n, z, v, c } = psr;
        match cond {
            0x0 => false,
            0x1 => z,
            0x2 => z || (n != v),
            0x3 => n != v,
            0x4 => c || z,
            0x5 => c,
            0x6 => n,
            0x7 => v,
            0x8 => true,
            0x9 => !z,
            0xa => !z && (n == v),
            0xb => n == v,
            0xc => !c && !z,
            0xd => !c,
            0xe => !n,
            0xf => !v,
            _ => unreachable!(),
        }
    }

    let mut cpu = SparcCpu::new();
    for flags in 0..16u32 {
        let psr = Psr {
            n: flags & 1 != 0,
            z: flags & 2 != 0,
            v: flags & 4 != 0,
            c: flags & 8 != 0,
        };
        for cond in 0..16u32 {
            cpu.load_checked(&assemble(&[encode_bicc(cond, 2), encode_ta(0)]))
                .unwrap();
            let mut state = cpu.get_state();
            state.psr = psr;
            cpu.restore(&state).unwrap();
            cpu.step_checked().unwrap();
            assert_eq!(cpu.get_state().pc, if expected(cond, psr) { 8 } else { 4 });
        }
    }
}

#[test]
fn documented_divide_cc_encodings_set_saturation_flags() {
    let mut cpu = SparcCpu::new();
    let udivcc = encode_alu_imm(0x1e, 4, 2, 1);
    cpu.load_checked(&assemble(&[udivcc])).unwrap();
    cpu.write_register_checked(2, 10).unwrap();
    cpu.step_checked().unwrap();
    assert_eq!(cpu.read_register_checked(4).unwrap(), 10);
    assert_eq!(cpu.get_state().psr, Psr::default());

    cpu.load_checked(&assemble(&[udivcc])).unwrap();
    let mut overflow = cpu.get_state();
    overflow.y = 1;
    cpu.restore(&overflow).unwrap();
    cpu.step_checked().unwrap();
    assert_eq!(cpu.read_register_checked(4).unwrap(), u32::MAX);
    assert_eq!(
        cpu.get_state().psr,
        Psr {
            n: true,
            z: false,
            v: true,
            c: false,
        }
    );

    let sdivcc = encode_alu_imm(0x1f, 4, 2, -1);
    cpu.load_checked(&assemble(&[sdivcc])).unwrap();
    let mut overflow = cpu.get_state();
    overflow.y = 0x8000_0000;
    overflow.regs[2] = 0;
    cpu.restore(&overflow).unwrap();
    cpu.step_checked().unwrap();
    assert_eq!(cpu.read_register_checked(4).unwrap(), i32::MAX as u32);
    assert!(cpu.get_state().psr.v);
}

#[test]
fn fixed_round_gate_dividers_match_wide_integer_oracles() {
    let edges = [
        (0, 0, 1),
        (0, u32::MAX, 1),
        (1, 0, 2),
        (u32::MAX, u32::MAX, u32::MAX),
        (0x7fff_ffff, u32::MAX, 3),
        (0x8000_0000, 0, u32::MAX),
        (u32::MAX, 0xffff_fffe, 2),
    ];
    let mut samples = edges.to_vec();
    let mut seed = 0x6a09_e667_f3bc_c909u64;
    for _ in 0..128 {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let y = (seed >> 32) as u32;
        seed = seed.rotate_left(17) ^ 0x9e37_79b9_7f4a_7c15;
        let rs1 = seed as u32;
        seed = seed
            .wrapping_mul(2_862_933_555_777_941_757)
            .wrapping_add(3_037_000_493);
        let src2 = (seed as u32) | 1;
        samples.push((y, rs1, src2));
    }

    for (y, rs1, src2) in samples {
        let unsigned_dividend = (u128::from(y) << 32) | u128::from(rs1);
        let unsigned_quotient = unsigned_dividend / u128::from(src2);
        let unsigned_overflow = unsigned_quotient > u128::from(u32::MAX);
        let unsigned_expected = if unsigned_overflow {
            u32::MAX
        } else {
            unsigned_quotient as u32
        };
        assert_eq!(
            udiv64_with_overflow(y, rs1, src2),
            (unsigned_expected, unsigned_overflow)
        );

        let signed_dividend = i128::from(((y as i32 as i64) << 32) | i64::from(rs1));
        let signed_divisor = i128::from(src2 as i32);
        let signed_quotient = signed_dividend / signed_divisor;
        let signed_overflow =
            signed_quotient > i128::from(i32::MAX) || signed_quotient < i128::from(i32::MIN);
        let signed_expected = if signed_quotient > i128::from(i32::MAX) {
            i32::MAX as u32
        } else if signed_quotient < i128::from(i32::MIN) {
            i32::MIN as u32
        } else {
            signed_quotient as i32 as u32
        };
        assert_eq!(
            sdiv64_with_overflow(y, rs1, src2),
            (signed_expected, signed_overflow),
            "signed Y:rs1={y:#010x}:{rs1:#010x} / {:#010x}",
            src2
        );
    }
}
