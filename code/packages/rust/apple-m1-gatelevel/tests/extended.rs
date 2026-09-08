use apple_m1_gatelevel::encoding as enc;
use apple_m1_gatelevel::{AppleM1Error, AppleM1GateLevel};

fn one(raw: u32, setup: impl FnOnce(&mut AppleM1GateLevel)) -> AppleM1GateLevel {
    let mut cpu = AppleM1GateLevel::new();
    cpu.load_checked(&raw.to_be_bytes()).unwrap();
    setup(&mut cpu);
    cpu.step_checked().unwrap();
    cpu
}

#[test]
fn scalar_fp_one_two_compare_move_and_convert_are_exact() {
    let cpu = one(enc::fp_one_source(1, 1, 1, 2), |cpu| {
        cpu.write_vector(1, u128::from((-3.5_f64).to_bits()))
            .unwrap();
    });
    assert_eq!(cpu.read_vector(2).unwrap() as u64, 3.5_f64.to_bits());

    let cpu = one(enc::fp_two_source(0, 2, 2, 1, 3), |cpu| {
        cpu.write_vector(1, u128::from(1.25_f32.to_bits())).unwrap();
        cpu.write_vector(2, u128::from(2.5_f32.to_bits())).unwrap();
    });
    assert_eq!(cpu.read_vector(3).unwrap() as u32, 3.75_f32.to_bits());

    let cpu = one(enc::fp_compare(1, 2, 1, 0), |cpu| {
        cpu.write_vector(1, u128::from(1.0_f64.to_bits())).unwrap();
        cpu.write_vector(2, u128::from(2.0_f64.to_bits())).unwrap();
    });
    assert_eq!(cpu.nzcv(), 0b1000);

    let cpu = one(enc::fp_gpr_move(true, true, 1, 2), |cpu| {
        cpu.write_register(1, 0x4009_21fb_5444_2d18).unwrap();
    });
    assert_eq!(cpu.read_vector(2).unwrap() as u64, 0x4009_21fb_5444_2d18);

    let cpu = one(enc::integer_to_fp(1, 1, true, 1, 2), |cpu| {
        cpu.write_register(1, (-42_i64) as u64).unwrap();
    });
    assert_eq!(cpu.read_vector(2).unwrap() as u64, (-42.0_f64).to_bits());

    let cpu = one(enc::fp_to_signed(1, 1, 1, 2), |cpu| {
        cpu.write_vector(1, u128::from((-42.75_f64).to_bits()))
            .unwrap();
    });
    assert_eq!(cpu.read_register(2).unwrap(), (-42_i64) as u64);
}

#[test]
fn vector_load_store_and_neon_integer_floating_families_are_exact() {
    let store = enc::fp_load_store(3, false, 0, 1, 2);
    let mut cpu = one(store, |cpu| {
        cpu.write_register(1, 0x100).unwrap();
        cpu.write_vector(2, u128::from(0x0123_4567_89ab_cdef_u64))
            .unwrap();
    });
    let load = enc::fp_load_store(3, true, 0, 1, 3);
    let mut state = cpu.get_state();
    state.pc = 0;
    state.memory[..4].copy_from_slice(&load.to_be_bytes());
    cpu.restore(&state).unwrap();
    cpu.step_checked().unwrap();
    assert_eq!(cpu.read_vector(3).unwrap() as u64, 0x0123_4567_89ab_cdef);

    let cpu = one(enc::neon_duplicate(true, 0b00100, 1, 2), |cpu| {
        cpu.write_register(1, 0xa5a5_5a5a).unwrap();
    });
    assert_eq!(
        cpu.read_vector(2).unwrap(),
        0xa5a5_5a5a_a5a5_5a5a_a5a5_5a5a_a5a5_5a5a
    );

    let cpu = one(enc::neon_three_same(true, false, 1, 2, 0x10, 1, 3), |cpu| {
        cpu.write_vector(1, 0x0001_0002_0003_0004_0005_0006_0007_0008)
            .unwrap();
        cpu.write_vector(2, 0x0008_0007_0006_0005_0004_0003_0002_0001)
            .unwrap();
    });
    assert_eq!(
        cpu.read_vector(3).unwrap(),
        0x0009_0009_0009_0009_0009_0009_0009_0009
    );

    let cpu = one(enc::neon_three_same(true, false, 0, 2, 0x1a, 1, 3), |cpu| {
        let lanes = u128::from(1.5_f32.to_bits()) | (u128::from(2.0_f32.to_bits()) << 32);
        let rhs = u128::from(0.5_f32.to_bits()) | (u128::from(3.0_f32.to_bits()) << 32);
        cpu.write_vector(1, lanes).unwrap();
        cpu.write_vector(2, rhs).unwrap();
    });
    assert_eq!(cpu.read_vector(3).unwrap() as u32, 2.0_f32.to_bits());
    assert_eq!(
        (cpu.read_vector(3).unwrap() >> 32) as u32,
        5.0_f32.to_bits()
    );
}

#[test]
fn reserved_extended_encodings_are_typed_and_atomic() {
    let malformed = [
        enc::fp_one_source(2, 0, 1, 2),
        enc::fp_two_source(1, 2, 15, 1, 3),
        enc::fp_load_store(1, true, 0, 1, 2),
        enc::neon_three_same(true, false, 3, 2, 0x13, 1, 3),
    ];
    for raw in malformed {
        let mut cpu = AppleM1GateLevel::new();
        cpu.load_checked(&raw.to_be_bytes()).unwrap();
        let before = cpu.get_state();
        assert_eq!(
            cpu.step_checked(),
            Err(AppleM1Error::UnknownInstruction { raw, pc: 0 })
        );
        assert_eq!(cpu.get_state(), before);
    }
}
