use ibm_system360_simulator::{System360Error, System360Simulator, MEMORY_BYTES};

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
    assert!(length > 0);
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
    vec![0x0a, 0x00]
}

#[test]
fn load_reset_transport_and_complete_state_are_strict() {
    let mut cpu = System360Simulator::new();
    let image = program(&[rr(0x18, 1, 2), halt()]);
    assert_eq!(cpu.load(&image, 0x100), Ok(image.len()));
    assert_eq!(cpu.get_state().pc, 0x100);
    assert_eq!(cpu.get_state().loaded_origin, 0x100);
    assert_eq!(cpu.get_state().loaded_len, image.len());
    assert_eq!(cpu.get_state().memory.len(), MEMORY_BYTES);

    let before = cpu.get_state();
    assert!(matches!(
        cpu.load(&[], 0),
        Err(System360Error::EmptyProgram)
    ));
    assert_eq!(cpu.get_state(), before);
    assert!(matches!(
        cpu.load(&image, 1),
        Err(System360Error::MisalignedProgram { origin: 1 })
    ));

    cpu.write_gpr(2, 0x1234_5678).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.raw, rr(0x18, 1, 2));
    assert_eq!(trace.mnemonic, "LR");
    assert_eq!(cpu.read_gpr(1), Ok(0x1234_5678));
    cpu.step().unwrap();
    assert!(cpu.get_state().halted);
    assert!(matches!(cpu.step(), Err(System360Error::Halted)));
}

#[test]
fn fixed_point_rr_sign_control_and_logical_families_set_condition_codes() {
    let image = program(&[
        rr(0x1a, 1, 2),
        rr(0x1b, 1, 3),
        rr(0x12, 4, 1),
        rr(0x13, 5, 4),
        rr(0x10, 6, 5),
        rr(0x11, 7, 6),
        rr(0x14, 8, 9),
        rr(0x16, 8, 10),
        rr(0x17, 8, 11),
        halt(),
    ]);
    let mut cpu = System360Simulator::new();
    cpu.load(&image, 0).unwrap();
    cpu.write_gpr(1, 10).unwrap();
    cpu.write_gpr(2, 7).unwrap();
    cpu.write_gpr(3, 4).unwrap();
    cpu.write_gpr(9, 0xff00_ff00).unwrap();
    cpu.write_gpr(10, 0x00f0_000f).unwrap();
    cpu.write_gpr(11, 0x0000_00ff).unwrap();
    cpu.run(20).unwrap();
    assert_eq!(cpu.read_gpr(1), Ok(13));
    assert_eq!(cpu.read_gpr(4), Ok(13));
    assert_eq!(cpu.read_gpr(5), Ok((-13_i32) as u32));
    assert_eq!(cpu.read_gpr(6), Ok(13));
    assert_eq!(cpu.read_gpr(7), Ok((-13_i32) as u32));
    assert_eq!(cpu.read_gpr(8), Ok(0x00f0_00f0));
    assert_eq!(cpu.get_state().condition_code, 1);
}

#[test]
fn rx_addressing_load_store_compare_and_halfword_operations_are_big_endian() {
    let image = program(&[
        rx(0x58, 1, 2, 3, 4),
        rx(0x50, 1, 0, 3, 8),
        rx(0x48, 4, 0, 3, 12),
        rx(0x4a, 4, 0, 3, 14),
        rx(0x49, 4, 0, 3, 16),
        rx(0x41, 5, 2, 3, 20),
        halt(),
    ]);
    let mut cpu = System360Simulator::new();
    cpu.load(&image, 0).unwrap();
    cpu.write_gpr(2, 4).unwrap();
    cpu.write_gpr(3, 0x300).unwrap();
    cpu.write_memory(0x308, &0x1234_5678_u32.to_be_bytes())
        .unwrap();
    cpu.write_memory(0x30c, &(-3_i16).to_be_bytes()).unwrap();
    cpu.write_memory(0x30e, &5_i16.to_be_bytes()).unwrap();
    cpu.write_memory(0x310, &2_i16.to_be_bytes()).unwrap();
    cpu.run(20).unwrap();
    assert_eq!(cpu.read_gpr(1), Ok(0x1234_5678));
    assert_eq!(
        cpu.read_memory(0x308, 4).unwrap(),
        0x1234_5678_u32.to_be_bytes()
    );
    assert_eq!(cpu.read_gpr(4), Ok(2));
    assert_eq!(cpu.get_state().condition_code, 0);
    assert_eq!(cpu.read_gpr(5), Ok(0x318));
}

#[test]
fn multiply_divide_and_shift_register_pairs_follow_specification_rules() {
    let image = program(&[
        rr(0x1c, 2, 4),
        rr(0x1d, 2, 5),
        rs(0x89, 6, 0, 0, 4),
        rs(0x88, 6, 0, 0, 1),
        rs(0x8d, 8, 0, 0, 4),
        rs(0x8c, 8, 0, 0, 2),
        halt(),
    ]);
    let mut cpu = System360Simulator::new();
    cpu.load(&image, 0).unwrap();
    cpu.write_gpr(3, 7).unwrap();
    cpu.write_gpr(4, 6).unwrap();
    cpu.write_gpr(5, 5).unwrap();
    cpu.write_gpr(6, 0x1234_5678).unwrap();
    cpu.write_gpr(8, 0x0000_0001).unwrap();
    cpu.write_gpr(9, 0x0000_0002).unwrap();
    cpu.run(20).unwrap();
    assert_eq!(cpu.read_gpr(2), Ok(2));
    assert_eq!(cpu.read_gpr(3), Ok(8));
    assert_eq!(cpu.read_gpr(6), Ok(0x11a2_b3c0));
    assert_eq!((cpu.read_gpr(8).unwrap(), cpu.read_gpr(9).unwrap()), (4, 8));

    let mut odd = System360Simulator::new();
    odd.load(&rr(0x1d, 1, 2), 0).unwrap();
    let before = odd.get_state();
    assert!(matches!(
        odd.step(),
        Err(System360Error::Specification { .. })
    ));
    assert_eq!(odd.get_state(), before);
}

#[test]
fn multiple_register_and_character_transfers_preserve_boundaries() {
    let image = program(&[
        rs(0x90, 14, 1, 3, 0),
        rs(0x98, 4, 7, 3, 0),
        rx(0x42, 4, 0, 3, 32),
        rx(0x43, 8, 0, 3, 32),
        halt(),
    ]);
    let mut cpu = System360Simulator::new();
    cpu.load(&image, 0).unwrap();
    cpu.write_gpr(3, 0x400).unwrap();
    cpu.write_gpr(14, 0x1111_1111).unwrap();
    cpu.write_gpr(15, 0x2222_2222).unwrap();
    cpu.write_gpr(0, 0x3333_3333).unwrap();
    cpu.write_gpr(1, 0x4444_4444).unwrap();
    cpu.write_gpr(4, 0xa5).unwrap();
    cpu.write_gpr(8, 0x1234_0000).unwrap();
    cpu.run(20).unwrap();
    assert_eq!(cpu.read_gpr(4), Ok(0x1111_1111));
    assert_eq!(cpu.read_gpr(5), Ok(0x2222_2222));
    assert_eq!(cpu.read_gpr(6), Ok(0x3333_3333));
    assert_eq!(cpu.read_gpr(7), Ok(0x4444_4444));
    // LM replaced R4 before STC, so IC observes the low byte of 0x11111111.
    assert_eq!(cpu.read_gpr(8), Ok(0x1234_0011));
}

#[test]
fn immediate_and_storage_to_storage_logical_operations_are_left_to_right() {
    let image = program(&[
        si(0x92, 0xaa, 3, 0),
        si(0x94, 0x0f, 3, 0),
        si(0x96, 0x80, 3, 0),
        si(0x97, 0x81, 3, 0),
        si(0x91, 0x0f, 3, 0),
        ss(0xd2, 4, 3, 4, 3, 0),
        ss(0xd4, 4, 3, 8, 3, 4),
        ss(0xd6, 4, 3, 8, 3, 0),
        ss(0xd7, 4, 3, 8, 3, 4),
        ss(0xd5, 4, 3, 8, 3, 4),
        halt(),
    ]);
    let mut cpu = System360Simulator::new();
    cpu.load(&image, 0).unwrap();
    cpu.write_gpr(3, 0x500).unwrap();
    cpu.write_memory(0x500, &[1, 2, 3, 4]).unwrap();
    cpu.run(30).unwrap();
    assert_eq!(cpu.read_memory(0x504, 4).unwrap(), vec![0x0b, 2, 3, 4]);
    assert_eq!(cpu.read_memory(0x508, 4).unwrap(), vec![0, 0, 0, 0]);
    assert_eq!(cpu.get_state().condition_code, 1);
}

#[test]
fn condition_count_link_and_index_branches_cover_both_paths() {
    let image = program(&[
        rr(0x19, 1, 2),
        rx(0x47, 4, 0, 0, 12),
        rr(0x18, 3, 4),
        rr(0x18, 3, 5),
        rr(0x06, 6, 0),
        rr(0x05, 7, 8),
        rs(0x86, 9, 10, 0, 24),
        halt(),
        halt(),
        halt(),
    ]);
    let mut cpu = System360Simulator::new();
    cpu.load(&image, 0).unwrap();
    cpu.write_gpr(1, 1).unwrap();
    cpu.write_gpr(2, 2).unwrap();
    cpu.write_gpr(4, 0xdead).unwrap();
    cpu.write_gpr(5, 0xbeef).unwrap();
    cpu.write_gpr(6, 1).unwrap();
    cpu.write_gpr(8, 20).unwrap();
    cpu.write_gpr(9, 1).unwrap();
    cpu.write_gpr(10, 2).unwrap();
    cpu.write_gpr(11, 2).unwrap();
    cpu.run(20).unwrap();
    assert_eq!(cpu.read_gpr(3), Ok(0));
    assert_eq!(cpu.read_gpr(6), Ok(1));
    assert_eq!(cpu.read_gpr(7).unwrap() & 0x00ff_ffff, 14);

    let index_program = program(&[rs(0x86, 9, 10, 0, 6), halt(), halt()]);
    let mut index = System360Simulator::new();
    index.load(&index_program, 0).unwrap();
    index.write_gpr(9, 1).unwrap();
    index.write_gpr(10, 2).unwrap();
    index.write_gpr(11, 2).unwrap();
    index.run(4).unwrap();
    assert_eq!(index.read_gpr(9), Ok(3));
}

#[test]
fn index_branches_use_original_comparands_when_registers_overlap() {
    for (opcode, r1, r3, should_branch) in [
        (0x86, 1, 1, true),
        (0x86, 3, 2, true),
        (0x87, 1, 1, false),
        (0x87, 3, 2, false),
    ] {
        let mut cpu = System360Simulator::new();
        cpu.load(&program(&[rs(opcode, r1, r3, 0, 8), halt(), halt()]), 0)
            .unwrap();
        cpu.write_gpr(r1 as usize, 1).unwrap();
        if r3 != r1 {
            cpu.write_gpr(r3 as usize, 1).unwrap();
        }
        let trace = cpu.step().unwrap();
        assert_eq!(
            trace.pc_after,
            if should_branch { 8 } else { 4 },
            "opcode {opcode:#04x}, R1={r1}, R3={r3}"
        );
        assert_eq!(cpu.read_gpr(r1 as usize), Ok(2));
    }
}

#[test]
fn hexadecimal_floating_point_load_arithmetic_compare_store_and_sign_are_exact() {
    let image = program(&[
        rx(0x68, 0, 0, 3, 0),
        rx(0x6a, 0, 0, 3, 8),
        rx(0x60, 0, 0, 3, 16),
        rr(0x23, 4, 0),
        rr(0x20, 6, 4),
        rr(0x29, 0, 2),
        halt(),
    ]);
    let one = 0x4110_0000_0000_0000_u64;
    let two = 0x4120_0000_0000_0000_u64;
    let mut cpu = System360Simulator::new();
    cpu.load(&image, 0).unwrap();
    cpu.write_gpr(3, 0x600).unwrap();
    cpu.write_memory(0x600, &one.to_be_bytes()).unwrap();
    cpu.write_memory(0x608, &one.to_be_bytes()).unwrap();
    cpu.write_fpr(2, two).unwrap();
    cpu.run(20).unwrap();
    assert_eq!(cpu.read_fpr(0), Ok(two));
    assert_eq!(cpu.read_memory(0x610, 8).unwrap(), two.to_be_bytes());
    assert_eq!(cpu.get_state().condition_code, 0);
    assert_eq!(cpu.read_fpr(4), Ok(two ^ (1_u64 << 63)));
    assert_eq!(cpu.read_fpr(6), Ok(two));
}

#[test]
fn checked_faults_are_typed_atomic_and_have_stable_messages() {
    let cases = [
        (vec![0xff, 0], "unknown"),
        (vec![0x58, 0, 0], "truncated"),
        (rr(0x1d, 2, 3), "divide"),
    ];
    for (image, needle) in cases {
        let mut cpu = System360Simulator::new();
        cpu.load(&image, 0).unwrap();
        let before = cpu.get_state();
        let error = cpu.step().unwrap_err();
        assert!(error.to_string().to_lowercase().contains(needle));
        assert_eq!(cpu.get_state(), before);
    }

    let mut cpu = System360Simulator::new();
    cpu.load(&rx(0x58, 1, 0, 0, 2), 0).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.step(),
        Err(System360Error::MisalignedAccess { .. })
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn run_execute_and_restore_are_transactional_and_bounded() {
    let loop_program = rx(0x47, 15, 0, 0, 0);
    let mut cpu = System360Simulator::new();
    cpu.load(&loop_program, 0).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.run(3),
        Err(System360Error::MaxStepsExceeded { max_steps: 3 })
    ));
    assert_eq!(cpu.get_state(), before);

    let mut invalid = before.clone();
    invalid.condition_code = 4;
    assert!(matches!(
        cpu.restore(invalid),
        Err(System360Error::InvalidState { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    let result = System360Simulator::new()
        .execute(&halt(), 0x100, 2)
        .unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 1);
    assert_eq!(result.traces[0].mnemonic, "SVC 0");
}

#[test]
fn remaining_fixed_point_register_storage_and_shift_orders_execute() {
    for opcode in [0x1e, 0x1f, 0x15, 0x07, 0x06] {
        let mut cpu = System360Simulator::new();
        cpu.load(&program(&[rr(opcode, 2, 0), halt()]), 0).unwrap();
        cpu.write_gpr(2, 1).unwrap();
        cpu.step().unwrap();
    }

    for opcode in [
        0x40, 0x5a, 0x5b, 0x4b, 0x5e, 0x5f, 0x59, 0x55, 0x54, 0x56, 0x57, 0x5c, 0x4c, 0x5d, 0x45,
        0x46,
    ] {
        let mut cpu = System360Simulator::new();
        cpu.load(&program(&[rx(opcode, 2, 0, 4, 0), halt()]), 0)
            .unwrap();
        cpu.write_gpr(2, 0).unwrap();
        cpu.write_gpr(3, 42).unwrap();
        cpu.write_gpr(4, 0x400).unwrap();
        cpu.write_memory(0x400, &5_u32.to_be_bytes()).unwrap();
        cpu.step().unwrap();
    }

    let shifts = program(&[
        rs(0x8a, 2, 0, 0, 1),
        rs(0x8b, 2, 0, 0, 1),
        rs(0x8e, 4, 0, 0, 1),
        rs(0x8f, 4, 0, 0, 1),
        halt(),
    ]);
    let mut cpu = System360Simulator::new();
    cpu.load(&shifts, 0).unwrap();
    cpu.write_gpr(2, (-8_i32) as u32).unwrap();
    cpu.write_gpr(4, u32::MAX).unwrap();
    cpu.write_gpr(5, 0xffff_fff0).unwrap();
    cpu.run(10).unwrap();
    assert_eq!(cpu.read_gpr(2), Ok((-8_i32) as u32));
}

#[test]
fn invalid_si_opcode_is_a_typed_atomic_fault() {
    let mut cpu = System360Simulator::new();
    cpu.load(&[0x93, 0, 0, 0], 0).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step(),
        Err(System360Error::UnknownOpcode {
            opcode: 0x93,
            pc: 0,
        })
    );
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn architectural_shift_opcodes_and_large_counts_are_exact() {
    let mut logical = System360Simulator::new();
    logical
        .load(
            &program(&[
                rs(0x89, 1, 0, 0, 32),
                rs(0x88, 2, 0, 0, 63),
                rs(0x8a, 3, 0, 0, 63),
                halt(),
            ]),
            0,
        )
        .unwrap();
    logical.write_gpr(1, 1).unwrap();
    logical.write_gpr(2, u32::MAX).unwrap();
    logical.write_gpr(3, 0x8000_0000).unwrap();
    logical.run(8).unwrap();
    assert_eq!(logical.read_gpr(1), Ok(0));
    assert_eq!(logical.read_gpr(2), Ok(0));
    assert_eq!(logical.read_gpr(3), Ok(u32::MAX));

    let mut arithmetic_left_zero = System360Simulator::new();
    arithmetic_left_zero
        .load(&program(&[rs(0x8b, 1, 0, 0, 63), halt()]), 0)
        .unwrap();
    arithmetic_left_zero.run(4).unwrap();
    assert_eq!(arithmetic_left_zero.read_gpr(1), Ok(0));

    let mut arithmetic_left_overflow = System360Simulator::new();
    arithmetic_left_overflow
        .load(&rs(0x8b, 1, 0, 0, 32), 0)
        .unwrap();
    arithmetic_left_overflow.write_gpr(1, 1).unwrap();
    let before = arithmetic_left_overflow.get_state();
    assert_eq!(
        arithmetic_left_overflow.step(),
        Err(System360Error::FixedPointOverflow)
    );
    assert_eq!(arithmetic_left_overflow.get_state(), before);

    let mut wrong_opcode = System360Simulator::new();
    wrong_opcode.load(&[0xba, 0, 0, 0], 0).unwrap();
    assert_eq!(
        wrong_opcode.step(),
        Err(System360Error::UnknownOpcode {
            opcode: 0xba,
            pc: 0,
        })
    );
}

#[test]
fn hexadecimal_float_compare_normalizes_unnormalized_operands() {
    let mut cpu = System360Simulator::new();
    cpu.load(&program(&[rr(0x29, 0, 2), halt()]), 0).unwrap();
    cpu.write_fpr(0, 0x4110_0000_0000_0000).unwrap();
    cpu.write_fpr(2, 0x4201_0000_0000_0000).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.get_state().condition_code, 0);
}

#[test]
fn all_long_and_short_hexadecimal_floating_orders_execute_exact_integer_paths() {
    let one = 0x4110_0000_0000_0000_u64;
    let two = 0x4120_0000_0000_0000_u64;
    for opcode in [
        0x28, 0x38, 0x22, 0x32, 0x33, 0x20, 0x30, 0x21, 0x31, 0x2a, 0x3a, 0x2b, 0x3b, 0x29, 0x39,
        0x24, 0x34, 0x2c, 0x3c, 0x2d, 0x3d,
    ] {
        let mut cpu = System360Simulator::new();
        cpu.load(&program(&[rr(opcode, 0, 2), halt()]), 0).unwrap();
        cpu.write_fpr(0, one).unwrap();
        cpu.write_fpr(2, two).unwrap();
        cpu.step().unwrap();
    }

    for opcode in [
        0x68, 0x78, 0x60, 0x70, 0x6a, 0x7a, 0x6b, 0x7b, 0x69, 0x79, 0x6c, 0x7c, 0x6d, 0x7d,
    ] {
        let mut cpu = System360Simulator::new();
        cpu.load(&program(&[rx(opcode, 0, 0, 3, 0), halt()]), 0)
            .unwrap();
        cpu.write_gpr(3, 0x400).unwrap();
        cpu.write_fpr(0, one).unwrap();
        cpu.write_memory(0x400, &two.to_be_bytes()).unwrap();
        cpu.step().unwrap();
    }
}

#[test]
fn lifecycle_validation_and_every_error_message_are_exercised() {
    let mut cpu = System360Simulator::default();
    cpu.reset();
    assert!(cpu.write_gpr(16, 0).is_err());
    assert!(cpu.read_fpr(1).is_err());
    assert!(cpu.write_fpr(8, 0).is_err());
    assert!(cpu.set_program_counter(1).is_err());
    assert!(cpu.write_memory(MEMORY_BYTES as u32, &[1]).is_err());
    assert!(matches!(
        cpu.load(&halt(), MEMORY_BYTES as u32),
        Err(System360Error::ProgramOutOfRange { .. })
    ));

    cpu.load(&[0x0a, 1], 0).unwrap();
    assert!(matches!(
        cpu.step(),
        Err(System360Error::UnsupportedSupervisorCall { number: 1 })
    ));

    let mut bad_memory = System360Simulator::new().get_state();
    bad_memory.memory.clear();
    assert!(cpu.restore(bad_memory).is_err());
    let mut bad_pc = System360Simulator::new().get_state();
    bad_pc.pc = 1;
    assert!(cpu.restore(bad_pc).is_err());
    let mut bad_range = System360Simulator::new().get_state();
    bad_range.loaded_origin = MEMORY_BYTES as u32;
    bad_range.loaded_len = 1;
    assert!(cpu.restore(bad_range).is_err());

    let errors = [
        System360Error::EmptyProgram,
        System360Error::MisalignedProgram { origin: 1 },
        System360Error::ProgramOutOfRange {
            origin: 0,
            bytes: MEMORY_BYTES + 1,
        },
        System360Error::FetchOutsideProgram { pc: 2 },
        System360Error::TruncatedInstruction { pc: 0, needed: 4 },
        System360Error::UnknownOpcode {
            opcode: 0xff,
            pc: 0,
        },
        System360Error::UnsupportedSupervisorCall { number: 1 },
        System360Error::MisalignedAccess {
            address: 2,
            alignment: 4,
        },
        System360Error::MemoryOutOfRange {
            address: MEMORY_BYTES as u32,
            bytes: 1,
        },
        System360Error::InvalidRegister {
            bank: "general",
            index: 16,
        },
        System360Error::InvalidState { reason: "test" },
        System360Error::Specification { reason: "test" },
        System360Error::FixedPointOverflow,
        System360Error::FixedPointDivide,
        System360Error::FloatingExponent,
        System360Error::FloatingDivide,
        System360Error::Halted,
        System360Error::MaxStepsExceeded { max_steps: 1 },
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}
