use ibm_system360_gatelevel::{
    gates, System360Error, System360GateLevel, System360Simulator, ESTIMATED_GATE_COUNT,
    FLIP_FLOP_COUNT, MEMORY_BYTES,
};

#[test]
fn fixed_width_gate_networks_cover_carry_overflow_compare_and_shift() {
    for opcode in [0, 0x18, 0x5a, 0xd2, 0xff] {
        let decoded = gates::decode_byte(opcode);
        assert_eq!(decoded[usize::from(opcode)], 1);
        assert_eq!(decoded.into_iter().map(u16::from).sum::<u16>(), 1);
    }
    assert_eq!(gates::select_u32(0x1234, 0xabcd, 0), 0x1234);
    assert_eq!(gates::select_u32(0x1234, 0xabcd, 1), 0xabcd);
    assert_eq!(gates::branch_selected(0b0100, 1), 1);
    assert_eq!(gates::add_32(u32::MAX, 1, 0), (0, 1, 0));
    assert_eq!(gates::add_32(0x7fff_ffff, 1, 0), (0x8000_0000, 0, 1));
    assert_eq!(gates::negate_32(1), (u32::MAX, 0));
    assert_eq!(gates::bit_and_32(0xf0, 0x5a), 0x50);
    assert_eq!(gates::bit_or_32(0xf0, 0x5a), 0xfa);
    assert_eq!(gates::bit_xor_32(0xf0, 0x5a), 0xaa);
    assert_eq!(gates::signed_compare_condition_code(u32::MAX, 1), 1);
    assert_eq!(gates::signed_condition_code_64(u64::MAX), 1);
    assert_eq!(gates::signed_condition_code_64(0), 0);
    assert_eq!(gates::unsigned_compare_condition_code(u32::MAX, 1), 2);
    assert_eq!(gates::shift_left_32(1, 31), 0x8000_0000);
    assert_eq!(gates::shift_right_32(u32::MAX, 32), 0);
    assert_eq!(gates::arithmetic_right_32(0x8000_0000, 4), 0xf800_0000);
    assert_eq!(gates::add_64(u64::MAX, 1, 0), (0, 1, 0));
    assert_eq!(
        gates::multiply_signed_32((-7_i32) as u32, 6),
        (-42_i64) as u64
    );
    assert_eq!(gates::divide_signed_64_by_32(42, 5), Some((2, 8)));
    assert_eq!(
        gates::divide_signed_64_by_32((-42_i64) as u64, 5),
        Some(((-2_i32) as u32, (-8_i32) as u32))
    );
    assert_eq!(gates::divide_signed_64_by_32(1, 0), None);
    assert_eq!(gates::shift_left_64(1, 63), 0x8000_0000_0000_0000);
    assert_eq!(
        gates::arithmetic_right_64(0x8000_0000_0000_0000, 4),
        0xf800_0000_0000_0000
    );
    assert_eq!(gates::hfp_condition_code(0, true), 0);
    assert_eq!(gates::hfp_condition_code(0xc110_0000_0000_0000, true), 1);
    assert_eq!(
        gates::hfp_with_sign(0x4110_0000_0000_0000, true, true),
        0xc110_0000_0000_0000
    );
}

fn rr(opcode: u8, r1: u8, r2: u8) -> Vec<u8> {
    vec![opcode, (r1 << 4) | r2]
}

fn rx(opcode: u8, r1: u8, x2: u8, b2: u8, d2: u16) -> Vec<u8> {
    vec![
        opcode,
        (r1 << 4) | x2,
        (b2 << 4) | ((d2 >> 8) as u8 & 0x0f),
        d2 as u8,
    ]
}

fn rs(opcode: u8, r1: u8, r3: u8, b2: u8, d2: u16) -> Vec<u8> {
    rx(opcode, r1, r3, b2, d2)
}

fn si(opcode: u8, immediate: u8, b1: u8, d1: u16) -> Vec<u8> {
    vec![
        opcode,
        immediate,
        (b1 << 4) | ((d1 >> 8) as u8 & 0x0f),
        d1 as u8,
    ]
}

fn ss(opcode: u8, length: u8, b1: u8, d1: u16, b2: u8, d2: u16) -> Vec<u8> {
    vec![
        opcode,
        length - 1,
        (b1 << 4) | ((d1 >> 8) as u8 & 0x0f),
        d1 as u8,
        (b2 << 4) | ((d2 >> 8) as u8 & 0x0f),
        d2 as u8,
    ]
}

fn program(parts: &[Vec<u8>]) -> Vec<u8> {
    parts.iter().flatten().copied().collect()
}

fn halt() -> Vec<u8> {
    rr(0x0a, 0, 0)
}

fn pair(image: &[u8], origin: u32) -> (System360Simulator, System360GateLevel) {
    let mut functional = System360Simulator::new();
    let mut gate = System360GateLevel::new();
    functional.load(image, origin).unwrap();
    gate.load(image, origin).unwrap();
    assert_eq!(gate.get_state(), functional.get_state());
    (functional, gate)
}

fn step_equal(functional: &mut System360Simulator, gate: &mut System360GateLevel) {
    let expected = functional.step();
    let actual = gate.step();
    assert_eq!(actual, expected);
    assert_eq!(gate.get_state(), functional.get_state());
}

#[test]
fn topology_and_complete_transport_are_exact() {
    assert_eq!(FLIP_FLOP_COUNT, 8_389_403);
    assert_eq!(ESTIMATED_GATE_COUNT, 50_533_026);
    assert_eq!(System360GateLevel::new().flip_flop_count(), FLIP_FLOP_COUNT);
    assert_eq!(System360GateLevel::new().gate_count(), ESTIMATED_GATE_COUNT);
    let image = program(&[rr(0x18, 1, 2), halt()]);
    let (mut functional, mut gate) = pair(&image, 0x100);
    functional.write_gpr(2, 0x1234_5678).unwrap();
    gate.write_gpr(2, 0x1234_5678).unwrap();
    functional.write_fpr(4, 0x4110_0000_0000_0000).unwrap();
    gate.write_fpr(4, 0x4110_0000_0000_0000).unwrap();
    functional.write_memory(0x400, &[1, 2, 3, 4]).unwrap();
    gate.write_memory(0x400, &[1, 2, 3, 4]).unwrap();
    assert_eq!(gate.get_state(), functional.get_state());
    step_equal(&mut functional, &mut gate);
    step_equal(&mut functional, &mut gate);
}

#[test]
fn rr_fixed_logical_control_multiply_and_divide_lockstep() {
    let image = program(&[
        rr(0x18, 1, 2),
        rr(0x12, 3, 1),
        rr(0x1a, 1, 4),
        rr(0x1b, 1, 5),
        rr(0x1e, 6, 7),
        rr(0x1f, 6, 8),
        rr(0x14, 9, 10),
        rr(0x16, 9, 11),
        rr(0x17, 9, 12),
        rr(0x19, 1, 3),
        rr(0x15, 6, 9),
        rr(0x1c, 2, 4),
        rr(0x1d, 2, 5),
        rr(0x06, 13, 0),
        rr(0x07, 0, 0),
        halt(),
    ]);
    let (mut functional, mut gate) = pair(&image, 0);
    for (register, value) in [
        (2, 7),
        (4, 6),
        (5, 5),
        (7, 0xffff_ffff),
        (8, 1),
        (10, 0xff00_ff00),
        (11, 0x00f0_000f),
        (12, 0x0000_00ff),
        (13, 1),
    ] {
        functional.write_gpr(register, value).unwrap();
        gate.write_gpr(register, value).unwrap();
    }
    while !functional.get_state().halted {
        step_equal(&mut functional, &mut gate);
    }
}

#[test]
fn rx_rs_si_and_ss_storage_paths_lockstep() {
    let image = program(&[
        rx(0x58, 1, 2, 3, 0),
        rx(0x50, 1, 0, 3, 8),
        rx(0x48, 4, 0, 3, 12),
        rx(0x4a, 4, 0, 3, 14),
        rx(0x49, 4, 0, 3, 16),
        rx(0x41, 5, 2, 3, 20),
        rs(0x90, 14, 1, 3, 32),
        rs(0x98, 6, 9, 3, 32),
        si(0x92, 0xaa, 3, 96),
        si(0x94, 0x0f, 3, 96),
        si(0x96, 0x80, 3, 96),
        si(0x97, 0x81, 3, 96),
        si(0x91, 0x0f, 3, 96),
        ss(0xd2, 4, 3, 104, 3, 100),
        ss(0xd4, 4, 3, 108, 3, 104),
        ss(0xd6, 4, 3, 108, 3, 100),
        ss(0xd7, 4, 3, 108, 3, 104),
        ss(0xd5, 4, 3, 108, 3, 104),
        halt(),
    ]);
    let (mut functional, mut gate) = pair(&image, 0);
    for (register, value) in [
        (2, 8),
        (3, 0x400),
        (14, 0x1111_1111),
        (15, 0x2222_2222),
        (0, 0x3333_3333),
        (1, 0x4444_4444),
    ] {
        functional.write_gpr(register, value).unwrap();
        gate.write_gpr(register, value).unwrap();
    }
    let setup = [
        (0x408, 0x1234_5678_u32.to_be_bytes().to_vec()),
        (0x40c, (-3_i16).to_be_bytes().to_vec()),
        (0x40e, 5_i16.to_be_bytes().to_vec()),
        (0x410, 2_i16.to_be_bytes().to_vec()),
        (0x464, vec![1, 2, 3, 4]),
    ];
    for (address, data) in setup {
        functional.write_memory(address, &data).unwrap();
        gate.write_memory(address, &data).unwrap();
    }
    while !functional.get_state().halted {
        step_equal(&mut functional, &mut gate);
    }
}

#[test]
fn shifts_branches_and_floating_point_lockstep() {
    let one = 0x4110_0000_0000_0000_u64;
    let two = 0x4120_0000_0000_0000_u64;
    let image = program(&[
        rs(0x89, 1, 0, 0, 4),
        rs(0x88, 2, 0, 0, 1),
        rs(0x8d, 4, 0, 0, 4),
        rs(0x8c, 4, 0, 0, 2),
        rx(0x68, 0, 0, 3, 0),
        rx(0x6a, 0, 0, 3, 8),
        rx(0x60, 0, 0, 3, 16),
        rr(0x23, 4, 0),
        rr(0x20, 6, 4),
        rr(0x29, 0, 2),
        halt(),
    ]);
    let (mut functional, mut gate) = pair(&image, 0);
    for (register, value) in [(1, 1), (2, u32::MAX), (4, 1), (5, 2), (3, 0x600)] {
        functional.write_gpr(register, value).unwrap();
        gate.write_gpr(register, value).unwrap();
    }
    functional.write_fpr(2, two).unwrap();
    gate.write_fpr(2, two).unwrap();
    for (address, data) in [(0x600, one), (0x608, one)] {
        functional
            .write_memory(address, &data.to_be_bytes())
            .unwrap();
        gate.write_memory(address, &data.to_be_bytes()).unwrap();
    }
    while !functional.get_state().halted {
        step_equal(&mut functional, &mut gate);
    }
}

#[test]
fn every_hexadecimal_floating_order_uses_the_gate_network() {
    let one = 0x4110_0000_0000_0000_u64;
    let two = 0x4120_0000_0000_0000_u64;
    for opcode in [
        0x28, 0x38, 0x22, 0x32, 0x33, 0x20, 0x30, 0x21, 0x31, 0x2a, 0x3a, 0x2b, 0x3b, 0x29, 0x39,
        0x24, 0x34, 0x2c, 0x3c, 0x2d, 0x3d,
    ] {
        let image = rr(opcode, 0, 2);
        let (mut functional, mut gate) = pair(&image, 0);
        functional.write_fpr(0, one).unwrap();
        gate.write_fpr(0, one).unwrap();
        functional.write_fpr(2, two).unwrap();
        gate.write_fpr(2, two).unwrap();
        step_equal(&mut functional, &mut gate);
    }

    for opcode in [
        0x68, 0x78, 0x60, 0x70, 0x6a, 0x7a, 0x6b, 0x7b, 0x69, 0x79, 0x6c, 0x7c, 0x6d, 0x7d,
    ] {
        let image = rx(opcode, 0, 0, 3, 0);
        let (mut functional, mut gate) = pair(&image, 0);
        functional.write_gpr(3, 0x400).unwrap();
        gate.write_gpr(3, 0x400).unwrap();
        functional.write_fpr(0, one).unwrap();
        gate.write_fpr(0, one).unwrap();
        functional.write_memory(0x400, &two.to_be_bytes()).unwrap();
        gate.write_memory(0x400, &two.to_be_bytes()).unwrap();
        step_equal(&mut functional, &mut gate);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FrozenClass {
    Unknown,
    Reserved2,
    Rr,
    Reserved4,
    Rx,
    Rs,
    Si,
    Ss,
}

use FrozenClass::{
    Reserved2 as Q2, Reserved4 as Q4, Rr as R, Rs as S, Rx as X, Si as I, Ss as E, Unknown as U,
};

// Independent frozen classification from GA22-7000's operation-code tables.
// This deliberately does not use ranges, DecodeAuthority, or CANDIDATES.
const FROZEN_OPCODE_CLASSES: [FrozenClass; 256] = [
    // 0x00
    U, U, U, U, U, R, R, R, U, U, R, U, U, U, U, U, // 0x10
    R, R, R, R, R, R, R, R, R, R, R, R, R, R, R, R, // 0x20
    R, R, R, R, R, Q2, Q2, Q2, R, R, R, R, R, R, Q2, Q2, // 0x30
    R, R, R, R, R, Q2, Q2, Q2, R, R, R, R, R, R, Q2, Q2, // 0x40
    X, X, X, X, Q4, X, X, X, X, X, X, X, X, Q4, Q4, Q4, // 0x50
    X, Q4, Q4, Q4, X, X, X, X, X, X, X, X, X, X, X, X, // 0x60
    X, Q4, Q4, Q4, Q4, Q4, Q4, Q4, X, X, X, X, X, X, Q4, Q4, // 0x70
    X, Q4, Q4, Q4, Q4, Q4, Q4, Q4, X, X, X, X, X, X, Q4, Q4, // 0x80
    U, U, U, U, U, U, S, S, S, S, S, S, S, S, S, S, // 0x90
    S, I, I, Q4, I, I, I, I, S, U, U, U, U, U, U, U, // 0xa0
    U, U, U, U, U, U, U, U, U, U, U, U, U, U, U, U, // 0xb0
    U, U, U, U, U, U, U, U, U, U, U, U, U, U, U, U, // 0xc0
    U, U, U, U, U, U, U, U, U, U, U, U, U, U, U, U, // 0xd0
    U, U, E, U, E, E, E, E, U, U, U, U, U, U, U, U, // 0xe0
    U, U, U, U, U, U, U, U, U, U, U, U, U, U, U, U, // 0xf0
    U, U, U, U, U, U, U, U, U, U, U, U, U, U, U, U,
];

fn class_width(class: FrozenClass) -> Option<usize> {
    match class {
        FrozenClass::Unknown => None,
        FrozenClass::Reserved2 | FrozenClass::Rr => Some(2),
        FrozenClass::Reserved4 | FrozenClass::Rx | FrozenClass::Rs | FrozenClass::Si => Some(4),
        FrozenClass::Ss => Some(6),
    }
}

fn frozen_image(opcode: u8, class: FrozenClass) -> Vec<u8> {
    match class {
        FrozenClass::Unknown => vec![opcode],
        FrozenClass::Reserved2 => rr(opcode, 0, 2),
        FrozenClass::Rr => rr(opcode, 0, if opcode == 0x0a { 0 } else { 2 }),
        FrozenClass::Reserved4 => rx(opcode, 0, 0, 3, 0),
        FrozenClass::Rx => rx(opcode, 0, 0, 3, 0),
        FrozenClass::Rs => rs(opcode, 0, 2, 3, 0),
        FrozenClass::Si => si(opcode, 0x5a, 3, 0),
        FrozenClass::Ss => ss(opcode, 4, 3, 0x80, 3, 0x100),
    }
}

#[test]
fn frozen_256_opcode_matrix_covers_every_transport_length_and_atomic_fault() {
    for (opcode, class) in (0_u8..=u8::MAX).zip(FROZEN_OPCODE_CLASSES) {
        for loaded_len in [2_usize, 4, 6] {
            let mut image = frozen_image(opcode, class);
            image.resize(loaded_len, 0);
            image.truncate(loaded_len);
            let (mut functional, mut gate) = pair(&image, 0);
            for (register, value) in [(2, 1), (3, 0x400)] {
                functional.write_gpr(register, value).unwrap();
                gate.write_gpr(register, value).unwrap();
            }
            for register in [0, 2] {
                functional
                    .write_fpr(register, 0x4110_0000_0000_0000)
                    .unwrap();
                gate.write_fpr(register, 0x4110_0000_0000_0000).unwrap();
            }
            let word = 0x4110_0000_0000_0000_u64.to_be_bytes();
            functional.write_memory(0x400, &word).unwrap();
            gate.write_memory(0x400, &word).unwrap();
            functional.write_memory(0x480, &[1, 2, 3, 4]).unwrap();
            gate.write_memory(0x480, &[1, 2, 3, 4]).unwrap();
            functional.write_memory(0x500, &[5, 6, 7, 8]).unwrap();
            gate.write_memory(0x500, &[5, 6, 7, 8]).unwrap();
            let before = gate.get_state();
            let expected = functional.step();
            let actual = gate.step();
            let label = format!("opcode {opcode:#04x}, class {class:?}, length {loaded_len}");
            assert_eq!(actual, expected, "typed result: {label}");
            assert_eq!(gate.get_state(), functional.get_state(), "state: {label}");

            match (class_width(class), class) {
                (None, _) => assert_eq!(
                    actual,
                    Err(System360Error::UnknownOpcode { opcode, pc: 0 }),
                    "{label}"
                ),
                (Some(width), _) if loaded_len < width => assert_eq!(
                    actual,
                    Err(System360Error::TruncatedInstruction {
                        pc: 0,
                        needed: width
                    }),
                    "{label}"
                ),
                (_, FrozenClass::Reserved2 | FrozenClass::Reserved4) => assert_eq!(
                    actual,
                    Err(System360Error::UnknownOpcode { opcode, pc: 0 }),
                    "{label}"
                ),
                (Some(width), _) => {
                    if let Ok(trace) = &actual {
                        assert_eq!(usize::from(trace.width), width, "trace width: {label}");
                        assert_eq!(trace.raw, image[..width], "raw transport: {label}");
                    }
                }
            }
            if actual.is_err() {
                assert_eq!(gate.get_state(), before, "fault atomicity: {label}");
            }
        }
    }
}

#[test]
fn known_width_reserved_holes_truncate_before_unknown_opcode() {
    for (image, needed) in [
        (&[0x25][..], 2),
        (&[0x44, 0, 0][..], 4),
        (&[0x93, 0, 0][..], 4),
    ] {
        let mut gate = System360GateLevel::new();
        gate.load(image, 0).unwrap();
        let before = gate.get_state();
        assert_eq!(
            gate.step(),
            Err(System360Error::TruncatedInstruction { pc: 0, needed })
        );
        assert_eq!(gate.get_state(), before);
    }
}

#[test]
fn branch_paths_register_overlap_and_left_to_right_storage_are_exact() {
    for (opcode, r1, r3, should_branch) in [
        (0x86, 1, 1, true),
        (0x86, 3, 2, true),
        (0x87, 1, 1, false),
        (0x87, 3, 2, false),
    ] {
        let image = program(&[rs(opcode, r1, r3, 0, 8), halt(), halt()]);
        let (mut functional, mut gate) = pair(&image, 0);
        functional.write_gpr(r1 as usize, 1).unwrap();
        gate.write_gpr(r1 as usize, 1).unwrap();
        if r3 != r1 {
            functional.write_gpr(r3 as usize, 1).unwrap();
            gate.write_gpr(r3 as usize, 1).unwrap();
        }
        step_equal(&mut functional, &mut gate);
        assert_eq!(gate.get_state().pc, if should_branch { 8 } else { 4 });
        assert_eq!(gate.read_gpr(r1 as usize), Ok(2));
    }

    for (mask, condition_code, selected) in [(4, 1, true), (4, 2, false)] {
        let image = program(&[rx(0x47, mask, 0, 0, 8), halt(), halt()]);
        let (mut functional, mut gate) = pair(&image, 0);
        let mut functional_state = functional.get_state();
        functional_state.condition_code = condition_code;
        functional.restore(functional_state).unwrap();
        let mut gate_state = gate.get_state();
        gate_state.condition_code = condition_code;
        gate.restore(gate_state).unwrap();
        step_equal(&mut functional, &mut gate);
        assert_eq!(gate.get_state().pc, if selected { 8 } else { 4 });
    }

    for opcode in [0xd2, 0xd4, 0xd6, 0xd7] {
        let image = ss(opcode, 6, 3, 1, 3, 0);
        let (mut functional, mut gate) = pair(&image, 0);
        functional.write_gpr(3, 0x600).unwrap();
        gate.write_gpr(3, 0x600).unwrap();
        functional
            .write_memory(0x600, &[1, 2, 4, 8, 16, 32, 64])
            .unwrap();
        gate.write_memory(0x600, &[1, 2, 4, 8, 16, 32, 64]).unwrap();
        step_equal(&mut functional, &mut gate);
    }
}

#[test]
fn arithmetic_shift_divide_and_hfp_faults_leave_every_dff_unchanged() {
    fn assert_atomic(image: Vec<u8>, gprs: &[(usize, u32)], fprs: &[(usize, u64)]) {
        let (mut functional, mut gate) = pair(&image, 0);
        for &(register, value) in gprs {
            functional.write_gpr(register, value).unwrap();
            gate.write_gpr(register, value).unwrap();
        }
        for &(register, value) in fprs {
            functional.write_fpr(register, value).unwrap();
            gate.write_fpr(register, value).unwrap();
        }
        let before = gate.get_state();
        assert_eq!(gate.step(), functional.step());
        assert_eq!(gate.get_state(), before);
    }

    assert_atomic(rr(0x13, 0, 2), &[(2, 0x8000_0000)], &[]);
    assert_atomic(rr(0x1a, 0, 2), &[(0, 0x7fff_ffff), (2, 1)], &[]);
    assert_atomic(rr(0x1d, 0, 2), &[(0, 1), (1, 1), (2, 0)], &[]);
    assert_atomic(rs(0x8b, 0, 0, 0, 32), &[(0, 1)], &[]);
    assert_atomic(rs(0x8f, 0, 0, 0, 32), &[(0, 1), (1, 0)], &[]);
    assert_atomic(rr(0x2d, 0, 2), &[], &[(0, 0x4110_0000_0000_0000), (2, 0)]);
}

#[test]
fn every_functional_fault_is_atomic_at_the_gate_boundary() {
    for image in [vec![0xff, 0], vec![0x58, 0, 0], rr(0x1d, 2, 3)] {
        let (mut functional, mut gate) = pair(&image, 0);
        let before = gate.get_state();
        assert_eq!(gate.step(), functional.step());
        assert_eq!(gate.get_state(), before);
    }

    let image = rx(0x58, 1, 0, 0, 2);
    let (mut functional, mut gate) = pair(&image, 0);
    assert_eq!(gate.step(), functional.step());
    assert_eq!(gate.get_state(), functional.get_state());

    let mut invalid = gate.get_state();
    invalid.condition_code = 4;
    let before = gate.get_state();
    assert!(matches!(
        gate.restore(invalid),
        Err(System360Error::InvalidState { .. })
    ));
    assert_eq!(gate.get_state(), before);

    assert!(gate.write_gpr(16, 0).is_err());
    assert!(gate.write_fpr(1, 0).is_err());
    assert!(gate.write_memory(MEMORY_BYTES as u32, &[1]).is_err());
}

#[test]
fn bounded_run_and_execute_match_and_roll_back() {
    let loop_program = rx(0x47, 15, 0, 0, 0);
    let (mut functional, mut gate) = pair(&loop_program, 0);
    let before = gate.get_state();
    assert_eq!(gate.run(3), functional.run(3));
    assert_eq!(gate.get_state(), before);

    let expected = System360Simulator::new().execute(&halt(), 0x100, 2);
    let actual = System360GateLevel::new().execute(&halt(), 0x100, 2);
    assert_eq!(actual, expected);
}
