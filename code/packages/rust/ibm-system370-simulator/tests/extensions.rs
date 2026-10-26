use ibm_system370_simulator::{System370Error, System370Simulator, MEMORY_BYTES};

fn loaded(instruction: &[u8]) -> System370Simulator {
    let mut cpu = System370Simulator::new();
    cpu.load(instruction, 0).unwrap();
    cpu
}

fn rs(opcode: u8, register: u8, mask: u8, address: u16) -> [u8; 4] {
    [
        opcode,
        (register << 4) | mask,
        ((address >> 8) & 0x0f) as u8,
        address as u8,
    ]
}

#[test]
fn mvcl_moves_pads_and_reports_all_length_relations() {
    let mut cpu = loaded(&[0x0e, 0x24]);
    cpu.write_gpr(2, 0x100).unwrap();
    cpu.write_gpr(3, 0xa500_0005).unwrap();
    cpu.write_gpr(4, 0x200).unwrap();
    cpu.write_gpr(5, 0x4000_0003).unwrap();
    cpu.write_memory(0x200, b"ABC").unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.mnemonic, "MVCL");
    assert_eq!(trace.raw, [0x0e, 0x24]);
    assert_eq!(trace.width, 2);
    assert!(!trace.description.is_empty());
    assert_eq!(cpu.read_memory(0x100, 5).unwrap(), b"ABC@@");
    assert_eq!(cpu.read_gpr(2).unwrap(), 0x105);
    assert_eq!(cpu.read_gpr(3).unwrap(), 0xa500_0000);
    assert_eq!(cpu.read_gpr(4).unwrap(), 0x203);
    assert_eq!(cpu.read_gpr(5).unwrap(), 0x4000_0000);
    assert_eq!(cpu.get_state().condition_code, 2);

    let mut shorter = loaded(&[0x0e, 0x24]);
    shorter.write_gpr(2, 0x100).unwrap();
    shorter.write_gpr(3, 2).unwrap();
    shorter.write_gpr(4, 0x200).unwrap();
    shorter.write_gpr(5, 3).unwrap();
    shorter.write_memory(0x200, b"XYZ").unwrap();
    shorter.step().unwrap();
    assert_eq!(shorter.read_memory(0x100, 2).unwrap(), b"XY");
    assert_eq!(shorter.read_gpr(4).unwrap(), 0x202);
    assert_eq!(shorter.read_gpr(5).unwrap(), 1);
    assert_eq!(shorter.get_state().condition_code, 1);

    let mut equal = loaded(&[0x0e, 0x24]);
    equal.write_gpr(2, 0x100).unwrap();
    equal.write_gpr(3, 2).unwrap();
    equal.write_gpr(4, 0x200).unwrap();
    equal.write_gpr(5, 2).unwrap();
    equal.write_memory(0x200, b"OK").unwrap();
    let trace = equal.step().unwrap();
    assert_eq!(trace.raw, [0x0e, 0x24]);
    assert_eq!(trace.width, 2);
    assert_eq!(trace.mnemonic, "MVCL");
    assert!(!trace.description.is_empty());
    assert_eq!(equal.get_state().condition_code, 0);
}

#[test]
fn mvcl_overlap_is_directional_and_condition_three_changes_no_pairs_or_data() {
    let mut destructive = loaded(&[0x0e, 0x24]);
    destructive.write_gpr(2, 0x201).unwrap();
    destructive.write_gpr(3, 3).unwrap();
    destructive.write_gpr(4, 0x200).unwrap();
    destructive.write_gpr(5, 3).unwrap();
    destructive.write_memory(0x200, b"ABCD").unwrap();
    let before_pairs = [
        destructive.read_gpr(2).unwrap(),
        destructive.read_gpr(3).unwrap(),
        destructive.read_gpr(4).unwrap(),
        destructive.read_gpr(5).unwrap(),
    ];
    let before_memory = destructive.read_memory(0x200, 4).unwrap();
    destructive.step().unwrap();
    assert_eq!(destructive.get_state().condition_code, 3);
    assert_eq!(
        [
            destructive.read_gpr(2).unwrap(),
            destructive.read_gpr(3).unwrap(),
            destructive.read_gpr(4).unwrap(),
            destructive.read_gpr(5).unwrap(),
        ],
        before_pairs
    );
    assert_eq!(destructive.read_memory(0x200, 4).unwrap(), before_memory);

    let mut safe = loaded(&[0x0e, 0x24]);
    safe.write_gpr(2, 0x200).unwrap();
    safe.write_gpr(3, 3).unwrap();
    safe.write_gpr(4, 0x201).unwrap();
    safe.write_gpr(5, 3).unwrap();
    safe.write_memory(0x201, b"ABC").unwrap();
    safe.step().unwrap();
    assert_eq!(safe.read_memory(0x200, 3).unwrap(), b"ABC");
    assert_eq!(safe.get_state().condition_code, 0);
}

#[test]
fn clcl_tracks_only_equal_prefix_and_uses_the_second_pair_pad() {
    let mut equal = loaded(&[0x0f, 0x24]);
    equal.write_gpr(2, 0xaa00_0100).unwrap();
    equal.write_gpr(3, 3).unwrap();
    equal.write_gpr(4, 0xbb00_0200).unwrap();
    equal.write_gpr(5, 0x4000_0003).unwrap();
    equal.write_memory(0x100, b"ABC").unwrap();
    equal.write_memory(0x200, b"ABC").unwrap();
    let trace = equal.step().unwrap();
    assert_eq!(trace.raw, [0x0f, 0x24]);
    assert_eq!(trace.width, 2);
    assert_eq!(trace.mnemonic, "CLCL");
    assert!(!trace.description.is_empty());
    assert_eq!(equal.get_state().condition_code, 0);
    assert_eq!(equal.read_gpr(2).unwrap(), 0x103);
    assert_eq!(equal.read_gpr(3).unwrap(), 0);
    assert_eq!(equal.read_gpr(4).unwrap(), 0x203);
    assert_eq!(equal.read_gpr(5).unwrap(), 0x4000_0000);

    let mut mismatch = loaded(&[0x0f, 0x24]);
    mismatch.write_gpr(2, 0x100).unwrap();
    mismatch.write_gpr(3, 3).unwrap();
    mismatch.write_gpr(4, 0x200).unwrap();
    mismatch.write_gpr(5, 0x4000_0003).unwrap();
    mismatch.write_memory(0x100, b"AZZ").unwrap();
    mismatch.write_memory(0x200, b"ABZ").unwrap();
    mismatch.step().unwrap();
    assert_eq!(mismatch.get_state().condition_code, 2);
    assert_eq!(mismatch.read_gpr(2).unwrap(), 0x101);
    assert_eq!(mismatch.read_gpr(3).unwrap(), 2);
    assert_eq!(mismatch.read_gpr(4).unwrap(), 0x201);
    assert_eq!(mismatch.read_gpr(5).unwrap(), 0x4000_0002);

    let mut padded = loaded(&[0x0f, 0x24]);
    padded.write_gpr(2, 0x100).unwrap();
    padded.write_gpr(3, 2).unwrap();
    padded.write_gpr(4, 0x200).unwrap();
    padded.write_gpr(5, 0x4000_0001).unwrap();
    padded.write_memory(0x100, b"AA").unwrap();
    padded.write_memory(0x200, b"A").unwrap();
    padded.step().unwrap();
    assert_eq!(padded.get_state().condition_code, 2);
    assert_eq!(padded.read_gpr(2).unwrap(), 0x101);
    assert_eq!(padded.read_gpr(3).unwrap(), 1);
    assert_eq!(padded.read_gpr(4).unwrap(), 0x201);
    assert_eq!(padded.read_gpr(5).unwrap(), 0x4000_0000);

    let mut first_shorter = loaded(&[0x0f, 0x24]);
    first_shorter.write_gpr(2, 0x100).unwrap();
    first_shorter.write_gpr(3, 1).unwrap();
    first_shorter.write_gpr(4, 0x200).unwrap();
    first_shorter.write_gpr(5, 0x4000_0002).unwrap();
    first_shorter.write_memory(0x100, b"A").unwrap();
    first_shorter.write_memory(0x200, b"A@").unwrap();
    first_shorter.step().unwrap();
    assert_eq!(first_shorter.get_state().condition_code, 0);
    assert_eq!(first_shorter.read_gpr(2).unwrap(), 0x101);
    assert_eq!(first_shorter.read_gpr(3).unwrap(), 0);
    assert_eq!(first_shorter.read_gpr(4).unwrap(), 0x202);
    assert_eq!(first_shorter.read_gpr(5).unwrap(), 0x4000_0000);
}

#[test]
fn clm_selects_register_bytes_in_order_and_sets_compare_codes() {
    for (storage, expected) in [([0x11, 0x33], 0), ([0x12, 0x33], 1), ([0x10, 0x33], 2)] {
        let instruction = rs(0xbd, 2, 0b1010, 0x100);
        let mut cpu = loaded(&instruction);
        cpu.write_gpr(2, 0x1122_3344).unwrap();
        cpu.write_memory(0x100, &storage).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(trace.raw, instruction);
        assert_eq!(trace.width, 4);
        assert_eq!(trace.mnemonic, "CLM");
        assert_eq!(cpu.get_state().condition_code, expected);
    }
    let mut zero = loaded(&rs(0xbd, 2, 0, 0));
    let mut state = zero.get_state();
    state.condition_code = 3;
    zero.restore(state).unwrap();
    zero.step().unwrap();
    assert_eq!(zero.get_state().condition_code, 0);
}

#[test]
fn stcm_and_icm_cover_masks_byte_order_and_every_condition_code() {
    let mut store = loaded(&rs(0xbe, 2, 0b1010, 0x100));
    store.write_gpr(2, 0x1122_3344).unwrap();
    let mut state = store.get_state();
    state.condition_code = 3;
    store.restore(state).unwrap();
    let trace = store.step().unwrap();
    assert_eq!(trace.raw, rs(0xbe, 2, 0b1010, 0x100));
    assert_eq!(trace.width, 4);
    assert_eq!(trace.mnemonic, "STCM");
    assert_eq!(store.read_memory(0x100, 2).unwrap(), [0x11, 0x33]);
    assert_eq!(store.get_state().condition_code, 3);

    for (bytes, expected) in [([0x00, 0x00], 0), ([0x80, 0x00], 1), ([0x01, 0x00], 2)] {
        let mut insert = loaded(&rs(0xbf, 2, 0b0101, 0x100));
        insert.write_gpr(2, 0xaabb_ccdd).unwrap();
        insert.write_memory(0x100, &bytes).unwrap();
        let trace = insert.step().unwrap();
        assert_eq!(trace.raw, rs(0xbf, 2, 0b0101, 0x100));
        assert_eq!(trace.width, 4);
        assert_eq!(trace.mnemonic, "ICM");
        assert_eq!(
            insert.read_gpr(2).unwrap(),
            0xaa00_cc00 | (u32::from(bytes[0]) << 16) | u32::from(bytes[1])
        );
        assert_eq!(insert.get_state().condition_code, expected);
    }

    let mut zero_store = loaded(&rs(0xbe, 2, 0, 0));
    let before = zero_store.get_state();
    zero_store.step().unwrap();
    assert_eq!(zero_store.get_state().memory, before.memory);
    let mut zero_insert = loaded(&rs(0xbf, 2, 0, 0));
    zero_insert.write_gpr(2, u32::MAX).unwrap();
    zero_insert.step().unwrap();
    assert_eq!(zero_insert.read_gpr(2).unwrap(), u32::MAX);
    assert_eq!(zero_insert.get_state().condition_code, 0);
}

#[test]
fn extension_faults_are_typed_and_atomic() {
    let mut odd_pair = loaded(&[0x0e, 0x12]);
    let before = odd_pair.get_state();
    assert!(matches!(
        odd_pair.step(),
        Err(System370Error::Specification { .. })
    ));
    assert_eq!(odd_pair.get_state(), before);

    let mut range = loaded(&[0x0e, 0x24]);
    range.write_gpr(2, 0x0f_fffe).unwrap();
    range.write_gpr(3, 3).unwrap();
    range.write_gpr(4, 0x100).unwrap();
    range.write_gpr(5, 3).unwrap();
    let before = range.get_state();
    assert!(matches!(
        range.step(),
        Err(System370Error::MemoryOutOfRange { .. })
    ));
    assert_eq!(range.get_state(), before);

    let mut instruction = rs(0xbf, 2, 0b1111, 0);
    instruction[2] = 0x10; // Base register 1, displacement zero.
    let mut masked = loaded(&instruction);
    masked.write_gpr(1, 0x0f_ffff).unwrap();
    let before = masked.get_state();
    assert!(matches!(
        masked.step(),
        Err(System370Error::MemoryOutOfRange { .. })
    ));
    assert_eq!(masked.get_state(), before);

    for opcode in [0xbd, 0xbf] {
        let mut instruction = rs(opcode, 2, 0, 0);
        instruction[2] = 0x10; // Base register 1, displacement zero.
        let mut zero_mask = loaded(&instruction);
        zero_mask.write_gpr(1, MEMORY_BYTES as u32).unwrap();
        zero_mask.write_gpr(2, 0x1122_3344).unwrap();
        let mut state = zero_mask.get_state();
        state.condition_code = 3;
        state.memory[0x100] = 0xa5;
        zero_mask.restore(state).unwrap();
        let before = zero_mask.get_state();
        assert_eq!(
            zero_mask.step(),
            Err(System370Error::MemoryOutOfRange {
                address: MEMORY_BYTES as u32,
                bytes: 1,
            }),
            "opcode {opcode:#04x}"
        );
        assert_eq!(zero_mask.get_state(), before, "opcode {opcode:#04x}");
    }

    let mut instruction = rs(0xbe, 2, 0, 0);
    instruction[2] = 0x10; // Base register 1, displacement zero.
    let mut zero_mask_stcm = loaded(&instruction);
    zero_mask_stcm.write_gpr(1, MEMORY_BYTES as u32).unwrap();
    zero_mask_stcm.write_gpr(2, 0x1122_3344).unwrap();
    let mut expected = zero_mask_stcm.get_state();
    expected.condition_code = 3;
    expected.memory[0x100] = 0xa5;
    zero_mask_stcm.restore(expected.clone()).unwrap();
    expected.pc = 4;
    let trace = zero_mask_stcm.step().unwrap();
    assert_eq!(trace.mnemonic, "STCM");
    assert_eq!(trace.raw, instruction);
    assert_eq!(zero_mask_stcm.get_state(), expected);
}
