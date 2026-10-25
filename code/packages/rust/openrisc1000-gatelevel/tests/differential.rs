use openrisc1000_gatelevel::{
    gates, OpenRisc1000GateLevel, OpenRisc1000State as GateState, FLIP_FLOP_COUNT, MEMORY_SIZE,
};
use openrisc1000_simulator::{OpenRisc1000Simulator, OpenRisc1000State as FunctionalState};

const SR_SM: u32 = 1;
const SR_F: u32 = 1 << 9;
const SR_CY: u32 = 1 << 10;
const SR_FO: u32 = 1 << 15;

fn reg(rd: u32, ra: u32, rb: u32, function: u32) -> u32 {
    (0x38 << 26) | (rd << 21) | (ra << 16) | (rb << 11) | function
}

fn compare(condition: u32, ra: u32, rb: u32) -> u32 {
    (0x39 << 26) | (condition << 21) | (ra << 16) | (rb << 11)
}

fn compare_imm(condition: u32, ra: u32, value: u16) -> u32 {
    (0x2f << 26) | (condition << 21) | (ra << 16) | u32::from(value)
}

fn imm(opcode: u32, rd: u32, ra: u32, value: u16) -> u32 {
    (opcode << 26) | (rd << 21) | (ra << 16) | u32::from(value)
}

fn store(opcode: u32, ra: u32, rb: u32, offset: u16) -> u32 {
    (opcode << 26)
        | ((u32::from(offset) >> 11) << 21)
        | (ra << 16)
        | (rb << 11)
        | (u32::from(offset) & 0x7ff)
}

fn mtspr(ra: u32, rb: u32, address: u16) -> u32 {
    (0x30 << 26)
        | (((u32::from(address) >> 11) & 0x1f) << 21)
        | (ra << 16)
        | (rb << 11)
        | (u32::from(address) & 0x7ff)
}

fn assert_state_eq(gate: &GateState, functional: &FunctionalState) {
    assert_eq!(gate.gpr, functional.gpr);
    assert_eq!(gate.pc, functional.pc);
    assert_eq!(gate.ppc, functional.ppc);
    assert_eq!(gate.sr, functional.sr);
    assert_eq!(gate.epcr0, functional.epcr0);
    assert_eq!(gate.eear0, functional.eear0);
    assert_eq!(gate.esr0, functional.esr0);
    assert_eq!(gate.maclo, functional.maclo);
    assert_eq!(gate.machi, functional.machi);
    assert_eq!(gate.reservation, functional.reservation);
    assert_eq!(gate.memory, functional.memory);
    assert_eq!(gate.halted, functional.halted);
    assert_eq!(gate.loaded_origin, functional.loaded_origin);
    assert_eq!(gate.loaded_len, functional.loaded_len);
}

fn compare_seeded_transition(label: &str, word: u32) {
    let program = [word.to_be_bytes(), 0x1500_0001_u32.to_be_bytes()].concat();
    let mut gate = OpenRisc1000GateLevel::new();
    let mut functional = OpenRisc1000Simulator::new();
    gate.load(&program, 0x100).unwrap();
    functional.load(&program, 0x100).unwrap();

    let mut gate_state = gate.get_state();
    let mut functional_state = functional.get_state();
    for state in [&mut gate_state.gpr, &mut functional_state.gpr] {
        state[1] = 0x200;
        state[2] = 3;
        state[3] = 0x8001_8001;
        state[9] = 0x104;
    }
    gate_state.sr = SR_FO | SR_SM | SR_F | SR_CY;
    functional_state.sr = SR_FO | SR_SM | SR_F | SR_CY;
    gate_state.epcr0 = 0x104;
    functional_state.epcr0 = 0x104;
    gate_state.esr0 = SR_FO | SR_SM;
    functional_state.esr0 = SR_FO | SR_SM;
    gate_state.maclo = 7;
    functional_state.maclo = 7;
    gate_state.machi = 1;
    functional_state.machi = 1;
    gate_state.reservation = Some(0x200);
    functional_state.reservation = Some(0x200);
    gate_state.memory[0x200..0x204].copy_from_slice(&[0x80, 0x01, 0x02, 0x03]);
    functional_state.memory[0x200..0x204].copy_from_slice(&[0x80, 0x01, 0x02, 0x03]);
    gate.restore(gate_state).unwrap();
    functional.restore(functional_state).unwrap();

    let gate_result = gate.step();
    let functional_result = functional.step();
    match (gate_result, functional_result) {
        (Ok(gate_trace), Ok(functional_trace)) => {
            assert_eq!(gate_trace.pc_before, functional_trace.pc_before, "{label}");
            assert_eq!(gate_trace.pc_after, functional_trace.pc_after, "{label}");
            assert_eq!(gate_trace.raw, functional_trace.raw, "{label}");
            assert_eq!(gate_trace.raw_word, functional_trace.raw_word, "{label}");
            assert_eq!(gate_trace.mnemonic, functional_trace.mnemonic, "{label}");
            assert_state_eq(&gate_trace.state_after, &functional_trace.state_after);
            assert_state_eq(&gate.get_state(), &functional.get_state());
        }
        (Err(gate_error), Err(functional_error)) => {
            assert_eq!(
                format!("{gate_error:?}"),
                format!("{functional_error:?}"),
                "{label}"
            );
            assert_state_eq(&gate.get_state(), &functional.get_state());
        }
        (gate_result, functional_result) => {
            panic!("{label} diverged: gate={gate_result:?}, functional={functional_result:?}")
        }
    }
}

#[test]
fn topology_and_gate_primitives_are_exact() {
    let cpu = OpenRisc1000GateLevel::new();
    assert_eq!(FLIP_FLOP_COUNT, 525_651);
    assert_eq!(cpu.flip_flop_count(), 525_651);
    assert!(cpu.gate_count() > FLIP_FLOP_COUNT * 6);

    for opcode in 0_u8..64 {
        let lines = gates::decode_primary(opcode);
        assert_eq!(lines.iter().filter(|&&line| line == 1).count(), 1);
        assert_eq!(lines[usize::from(opcode)], 1);
        let unsupported = matches!(
            opcode,
            0x0a | 0x1a | 0x20 | 0x32 | 0x1c..=0x1f | 0x3c..=0x3f
        );
        assert_eq!(gates::primary_is_unsupported(opcode), u8::from(unsupported));
        assert_eq!(
            gates::primary_is_implemented(opcode) & gates::primary_is_unsupported(opcode),
            0
        );
    }
    for left in [0, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
        for right in [0, 1, 31, 0x5555_aaaa, u32::MAX] {
            assert_eq!(gates::add_32(left, right, 0).0, left.wrapping_add(right));
            assert_eq!(gates::sub_32(left, right).0, left.wrapping_sub(right));
            assert_eq!(gates::bit_and_32(left, right), left & right);
            assert_eq!(gates::bit_or_32(left, right), left | right);
            assert_eq!(gates::bit_xor_32(left, right), left ^ right);
            assert_eq!(gates::equal_32(left, right), u8::from(left == right));
            assert_eq!(gates::select_32(left, right, 0), left);
            assert_eq!(gates::select_32(left, right, 1), right);
            assert_eq!(gates::shift_left_32(left, right & 31), left << (right & 31));
            assert_eq!(
                gates::shift_right_32(left, right & 31),
                left >> (right & 31)
            );
            assert_eq!(
                gates::shift_arithmetic_right_32(left, right & 31),
                ((left as i32) >> (right & 31)) as u32
            );
            assert_eq!(
                gates::rotate_right_32(left, right & 31),
                left.rotate_right(right & 31)
            );
            assert_eq!(gates::unsigned_less_32(left, right), u8::from(left < right));
            assert_eq!(
                gates::signed_less_32(left, right),
                u8::from((left as i32) < (right as i32))
            );
        }
    }
    for value in [0, 1, 0x80, 0x8001_0000, u32::MAX] {
        assert_eq!(
            gates::first_one(value),
            if value == 0 {
                0
            } else {
                value.trailing_zeros() + 1
            }
        );
        assert_eq!(
            gates::last_one(value),
            if value == 0 {
                0
            } else {
                32 - value.leading_zeros()
            }
        );
        assert_eq!(
            gates::sign_extend(value, 8),
            (value as u8 as i8 as i32) as u32
        );
    }
    for (left, right) in [(0, 1), (6, 7), (u32::MAX, 2), (0x8000_0000, 3)] {
        assert_eq!(
            gates::multiply_unsigned_32(left, right),
            u64::from(left) * u64::from(right)
        );
        assert_eq!(
            gates::multiply_signed_32(left, right),
            (i64::from(left as i32) * i64::from(right as i32)) as u64
        );
        if right != 0 {
            assert_eq!(gates::divide_unsigned_32(left, right).0, left / right);
            if !(left == i32::MIN as u32 && right == u32::MAX) {
                assert_eq!(
                    gates::divide_signed_32(left, right),
                    ((left as i32) / (right as i32)) as u32
                );
            }
        }
    }
    for left in [0_u8, 1, 0x55, 0xff] {
        for right in [0_u8, 1, 0xaa, 0xff] {
            assert_eq!(gates::byte_changed(left, right), u8::from(left != right));
        }
    }
}

#[test]
fn every_primary_opcode_transition_matches_the_functional_oracle() {
    for opcode in 0_u8..64 {
        let word = u32::from(opcode) << 26;
        let program = word.to_be_bytes();
        let mut gate = OpenRisc1000GateLevel::new();
        let mut functional = OpenRisc1000Simulator::new();
        gate.load(&program, 0x100).unwrap();
        functional.load(&program, 0x100).unwrap();

        let gate_result = gate.step();
        let functional_result = functional.step();
        match (gate_result, functional_result) {
            (Ok(gate_trace), Ok(functional_trace)) => {
                assert_eq!(gate_trace.pc_before, functional_trace.pc_before, "{opcode:#04x}");
                assert_eq!(gate_trace.pc_after, functional_trace.pc_after, "{opcode:#04x}");
                assert_eq!(gate_trace.raw, functional_trace.raw, "{opcode:#04x}");
                assert_eq!(gate_trace.raw_word, functional_trace.raw_word, "{opcode:#04x}");
                assert_eq!(gate_trace.mnemonic, functional_trace.mnemonic, "{opcode:#04x}");
                assert_state_eq(&gate_trace.state_after, &functional_trace.state_after);
                assert_state_eq(&gate.get_state(), &functional.get_state());
            }
            (Err(gate_error), Err(functional_error)) => {
                assert_eq!(format!("{gate_error:?}"), format!("{functional_error:?}"));
                assert_state_eq(&gate.get_state(), &functional.get_state());
            }
            (gate_result, functional_result) => panic!(
                "primary opcode {opcode:#04x} diverged: gate={gate_result:?}, functional={functional_result:?}"
            ),
        }
    }
}

#[test]
fn every_implemented_subfamily_matches_the_functional_oracle() {
    let mut witnesses: Vec<(String, u32)> = vec![
        ("jump".into(), 0x00 << 26),
        ("jump-and-link".into(), 0x01 << 26),
        ("page-address".into(), (0x02 << 26) | (4 << 21) | 1),
        ("branch-not-flag".into(), (0x03 << 26) | 1),
        ("branch-flag".into(), (0x04 << 26) | 1),
        ("nop".into(), 0x1500_0000),
        ("halt".into(), 0x1500_0001),
        (
            "mac-read-clear".into(),
            (0x06 << 26) | (4 << 21) | 0x0001_0000,
        ),
        ("move-high".into(), imm(0x06, 4, 0, 0x1234)),
        ("system-call".into(), 0x2000_0000),
        ("trap".into(), 0x2100_0000),
        ("memory-sync".into(), 0x2200_0000),
        ("pipeline-sync".into(), 0x2280_0000),
        ("context-sync".into(), 0x2300_0000),
        ("return-from-exception".into(), 0x09 << 26),
        ("jump-register".into(), (0x11 << 26) | (9 << 11)),
        ("jump-link-register".into(), (0x12 << 26) | (2 << 11)),
        (
            "multiply-accumulate-immediate".into(),
            imm(0x13, 0, 2, 0xfffd),
        ),
        ("load-word-atomic".into(), imm(0x1b, 4, 1, 0)),
        ("load-word-zero".into(), imm(0x21, 4, 1, 0)),
        ("load-word-signed".into(), imm(0x22, 4, 1, 0)),
        ("load-byte-zero".into(), imm(0x23, 4, 1, 0)),
        ("load-byte-signed".into(), imm(0x24, 4, 1, 0)),
        ("load-half-zero".into(), imm(0x25, 4, 1, 0)),
        ("load-half-signed".into(), imm(0x26, 4, 1, 0)),
        ("add-immediate".into(), imm(0x27, 4, 3, 0xffff)),
        ("add-immediate-carry".into(), imm(0x28, 4, 3, 1)),
        ("and-immediate".into(), imm(0x29, 4, 3, 0x55aa)),
        ("or-immediate".into(), imm(0x2a, 4, 3, 0x55aa)),
        ("xor-immediate".into(), imm(0x2b, 4, 3, 0xfffe)),
        ("multiply-immediate".into(), imm(0x2c, 4, 3, 3)),
        ("read-spr".into(), imm(0x2d, 4, 0, 0x2801)),
        ("write-spr".into(), mtspr(0, 2, 0x2802)),
        ("store-word-atomic".into(), store(0x33, 1, 2, 0)),
        ("store-word".into(), store(0x35, 1, 2, 0)),
        ("store-byte".into(), store(0x36, 1, 2, 0)),
        ("store-half".into(), store(0x37, 1, 2, 0)),
    ];

    for (function, name) in [
        (0, "shift-left-immediate"),
        (1, "shift-right-immediate"),
        (2, "shift-arithmetic-immediate"),
        (3, "rotate-right-immediate"),
    ] {
        witnesses.push((name.into(), imm(0x2e, 4, 3, (function << 6) | 3)));
    }
    for (condition, name) in [
        (0x00, "equal"),
        (0x01, "not-equal"),
        (0x02, "greater-unsigned"),
        (0x03, "greater-equal-unsigned"),
        (0x04, "less-unsigned"),
        (0x05, "less-equal-unsigned"),
        (0x0a, "greater-signed"),
        (0x0b, "greater-equal-signed"),
        (0x0c, "less-signed"),
        (0x0d, "less-equal-signed"),
    ] {
        witnesses.push((
            format!("compare-immediate-{name}"),
            compare_imm(condition, 3, 3),
        ));
        witnesses.push((format!("compare-register-{name}"), compare(condition, 3, 2)));
    }
    for (function, name) in [
        (1, "mac-signed"),
        (2, "mac-subtract-signed"),
        (3, "mac-unsigned"),
        (4, "mac-subtract-unsigned"),
    ] {
        witnesses.push((name.into(), (0x31 << 26) | (3 << 16) | (2 << 11) | function));
    }
    for (function, name) in [
        (0x000, "add"),
        (0x001, "add-carry"),
        (0x002, "subtract"),
        (0x003, "and"),
        (0x004, "or"),
        (0x005, "xor"),
        (0x008, "shift-left"),
        (0x048, "shift-right"),
        (0x088, "shift-arithmetic"),
        (0x0c8, "rotate-right"),
        (0x00e, "conditional-move"),
        (0x00f, "find-first-one"),
        (0x04f, "find-last-one"),
        (0x00c, "extend-half-signed"),
        (0x00d, "extend-word-signed"),
        (0x04c, "extend-byte-signed"),
        (0x04d, "extend-word-zero"),
        (0x08c, "extend-half-zero"),
        (0x0cc, "extend-byte-zero"),
        (0x306, "multiply-signed-low"),
        (0x307, "multiply-signed-double"),
        (0x309, "divide-signed"),
        (0x30a, "divide-unsigned"),
        (0x30b, "multiply-unsigned-low"),
        (0x30c, "multiply-unsigned-double"),
    ] {
        witnesses.push((name.into(), reg(4, 3, 2, function)));
    }
    for (name, word) in [
        ("unsupported-primary", 0x0a << 26),
        ("unknown-shift", (0x2e << 26) | 0x100),
        ("unknown-compare", (0x2f << 26) | (6 << 21)),
        ("unknown-mac", (0x31 << 26) | 5),
        ("unknown-alu", (0x38 << 26) | 0x3ff),
        ("unknown-register-compare", (0x39 << 26) | (6 << 21)),
    ] {
        witnesses.push((name.into(), word));
    }

    for (label, word) in witnesses {
        compare_seeded_transition(&label, word);
    }
}

#[test]
fn complete_state_restore_and_external_memory_writes_match() {
    let program = 0x1500_0001_u32.to_be_bytes();
    let mut gate = OpenRisc1000GateLevel::new();
    let mut functional = OpenRisc1000Simulator::new();
    gate.load(&program, 0x100).unwrap();
    functional.load(&program, 0x100).unwrap();
    gate.write_register(7, 0xdead_beef).unwrap();
    functional.write_register(7, 0xdead_beef).unwrap();
    gate.write_memory(0x200, &[1, 2, 3, 4]).unwrap();
    functional.write_memory(0x200, &[1, 2, 3, 4]).unwrap();
    assert_state_eq(&gate.get_state(), &functional.get_state());

    let before = gate.get_state();
    let mut invalid = before.clone();
    invalid.memory.push(0);
    assert!(gate.restore(invalid).is_err());
    assert_eq!(gate.get_state(), before);

    gate.reset();
    let reset = gate.get_state();
    assert_eq!(reset.memory, vec![0; MEMORY_SIZE]);
    assert_eq!(reset.loaded_len, 0);
}
