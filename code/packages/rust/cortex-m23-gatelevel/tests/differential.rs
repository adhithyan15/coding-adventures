use cortex_m23_gatelevel::{
    encoding, gates, CortexM23GateLevel, CortexM23State as GateState, FLIP_FLOP_COUNT, MEMORY_SIZE,
};
use cortex_m23_simulator::{
    CortexM23Simulator as Functional, CortexM23State as FunctionalState,
    SecurityExtension as FunctionalExtension, SecurityState as FunctionalSecurity,
};

const ORIGIN: u32 = 0x100;

fn assert_state_eq(gate: &GateState, functional: &FunctionalState, label: &str) {
    assert_eq!(gate.registers, functional.registers, "{label}: registers");
    assert_eq!(
        gate.msp_secure, functional.msp_secure,
        "{label}: msp secure"
    );
    assert_eq!(
        gate.psp_secure, functional.psp_secure,
        "{label}: psp secure"
    );
    assert_eq!(
        gate.msp_non_secure, functional.msp_non_secure,
        "{label}: msp nonsecure"
    );
    assert_eq!(
        gate.psp_non_secure, functional.psp_non_secure,
        "{label}: psp nonsecure"
    );
    assert_eq!(gate.lr, functional.lr, "{label}: lr");
    assert_eq!(gate.pc, functional.pc, "{label}: pc");
    assert_eq!(gate.apsr, functional.apsr, "{label}: apsr");
    assert_eq!(gate.ipsr, functional.ipsr, "{label}: ipsr");
    assert_eq!(
        gate.primask_secure, functional.primask_secure,
        "{label}: primask secure"
    );
    assert_eq!(
        gate.primask_non_secure, functional.primask_non_secure,
        "{label}: primask_non_secure"
    );
    assert_eq!(
        gate.control_secure, functional.control_secure,
        "{label}: control secure"
    );
    assert_eq!(
        gate.control_non_secure, functional.control_non_secure,
        "{label}: control_non_secure"
    );
    assert_eq!(gate.handler_mode, functional.handler_mode, "{label}: mode");
    assert_eq!(
        gate.security_extension,
        cortex_m23_gatelevel::SecurityExtension::Disabled,
        "{label}: gate extension"
    );
    assert_eq!(
        gate.security_state,
        cortex_m23_gatelevel::SecurityState::NonSecure,
        "{label}: gate security"
    );
    assert!(!gate.sau_enabled, "{label}: gate sau enable");
    assert!(gate.sau_all_non_secure, "{label}: gate allns");
    assert_eq!(
        gate.sau_regions,
        [cortex_m23_gatelevel::SauRegion::default(); 8],
        "{label}: gate sau regions"
    );
    assert_eq!(
        functional.security_extension,
        FunctionalExtension::Disabled,
        "{label}: extension"
    );
    assert_eq!(
        functional.security_state,
        FunctionalSecurity::NonSecure,
        "{label}: security"
    );
    assert!(!functional.sau_enabled, "{label}: sau enable");
    assert!(functional.sau_all_non_secure, "{label}: allns");
    assert!(
        functional.sau_regions.iter().all(|region| !region.enabled
            && region.base == 0
            && region.limit == 0
            && !region.non_secure_callable),
        "{label}: sau regions"
    );
    assert_eq!(gate.memory, functional.memory, "{label}: memory");
    assert_eq!(gate.halted, functional.halted, "{label}: halted");
    assert_eq!(gate.sleeping, functional.sleeping, "{label}: sleeping");
    assert_eq!(
        gate.event_registered, functional.event_registered,
        "{label}: event"
    );
    assert_eq!(
        gate.loaded_origin, functional.loaded_origin,
        "{label}: origin"
    );
    assert_eq!(gate.loaded_len, functional.loaded_len, "{label}: length");
}

fn compare_transition(label: &str, words: &[u16], configure: impl Fn(&mut GateState)) -> GateState {
    let program = encoding::program(words);
    let mut gate = CortexM23GateLevel::new();
    let mut functional = Functional::new(FunctionalExtension::Disabled);
    gate.load(&program, ORIGIN).unwrap();
    functional.load(&program, ORIGIN).unwrap();

    let mut gate_state = gate.get_state();
    configure(&mut gate_state);
    let functional_state = FunctionalState {
        registers: gate_state.registers,
        msp_secure: gate_state.msp_secure,
        psp_secure: gate_state.psp_secure,
        msp_non_secure: gate_state.msp_non_secure,
        psp_non_secure: gate_state.psp_non_secure,
        lr: gate_state.lr,
        pc: gate_state.pc,
        apsr: gate_state.apsr,
        ipsr: gate_state.ipsr,
        primask_secure: gate_state.primask_secure,
        primask_non_secure: gate_state.primask_non_secure,
        control_secure: gate_state.control_secure,
        control_non_secure: gate_state.control_non_secure,
        handler_mode: gate_state.handler_mode,
        security_extension: FunctionalExtension::Disabled,
        security_state: FunctionalSecurity::NonSecure,
        sau_enabled: false,
        sau_all_non_secure: true,
        sau_regions: [cortex_m23_simulator::SauRegion::default();
            cortex_m23_simulator::SAU_REGION_COUNT],
        memory: gate_state.memory.clone(),
        halted: gate_state.halted,
        sleeping: gate_state.sleeping,
        event_registered: gate_state.event_registered,
        loaded_origin: gate_state.loaded_origin,
        loaded_len: gate_state.loaded_len,
    };
    gate.restore(gate_state).unwrap();
    functional.restore(functional_state).unwrap();

    match (gate.step(), functional.step()) {
        (Ok(gate_trace), Ok(functional_trace)) => {
            assert_eq!(gate_trace.pc_before, functional_trace.pc_before, "{label}");
            assert_eq!(gate_trace.pc_after, functional_trace.pc_after, "{label}");
            assert_eq!(gate_trace.raw, functional_trace.raw, "{label}");
            assert_eq!(gate_trace.width, functional_trace.width, "{label}");
            assert_eq!(gate_trace.mnemonic, functional_trace.mnemonic, "{label}");
            assert_eq!(
                gate_trace.security_before,
                cortex_m23_gatelevel::SecurityState::NonSecure
            );
            assert_eq!(
                gate_trace.security_after,
                cortex_m23_gatelevel::SecurityState::NonSecure
            );
            assert_eq!(
                functional_trace.security_before,
                FunctionalSecurity::NonSecure
            );
            assert_eq!(
                functional_trace.security_after,
                FunctionalSecurity::NonSecure
            );
            assert_state_eq(
                &gate_trace.state_before,
                &functional_trace.state_before,
                label,
            );
            assert_state_eq(
                &gate_trace.state_after,
                &functional_trace.state_after,
                label,
            );
            assert_state_eq(
                &gate.get_state(),
                &functional.try_get_state().unwrap(),
                label,
            );
            gate.get_state()
        }
        (Err(gate_error), Err(functional_error)) => {
            assert_eq!(
                format!("{gate_error:?}"),
                format!("{functional_error:?}"),
                "{label}"
            );
            assert_state_eq(
                &gate.get_state(),
                &functional.try_get_state().unwrap(),
                label,
            );
            gate.get_state()
        }
        (gate_result, functional_result) => {
            panic!("{label} diverged: gate={gate_result:?}, functional={functional_result:?}")
        }
    }
}

fn seeded(state: &mut GateState) {
    state.registers = std::array::from_fn(|index| {
        0x1111_0001_u32.wrapping_add((index as u32).wrapping_mul(0x0101_0101))
    });
    state.registers[0] = 0x8000_0001;
    state.registers[1] = 0x201;
    state.registers[2] = 4;
    state.registers[3] = 0xffff_ff80;
    state.registers[4] = 0x200;
    state.msp_non_secure = 0x400;
    state.psp_non_secure = 0x500;
    state.lr = 0x105;
    state.apsr = 0x3000_0000;
    state.ipsr = 7;
    state.primask_non_secure = true;
    state.control_non_secure = 3;
    state.handler_mode = true;
    state.event_registered = true;
    state.memory[0x200..0x208].copy_from_slice(&[0x80, 0xff, 0x34, 0x12, 7, 0, 0, 0]);
}

#[test]
fn topology_and_combinational_networks_are_exact() {
    let cpu = CortexM23GateLevel::new();
    assert_eq!(FLIP_FLOP_COUNT, 524_898);
    assert_eq!(cpu.flip_flop_count(), 524_898);
    assert!(cpu.gate_count() > FLIP_FLOP_COUNT * 6);
    assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);

    for left in [0, 1, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
        for right in [0, 1, 31, 0x5555_aaaa, u32::MAX] {
            assert_eq!(gates::add_32(left, right, 0).0, left.wrapping_add(right));
            assert_eq!(gates::sub_32(left, right, 0).0, left.wrapping_sub(right));
            assert_eq!(gates::bit_and_32(left, right), left & right);
            assert_eq!(gates::bit_or_32(left, right), left | right);
            assert_eq!(gates::bit_xor_32(left, right), left ^ right);
            assert_eq!(gates::equal_32(left, right), u8::from(left == right));
            assert_eq!(
                gates::less_than_unsigned_32(left, right),
                u8::from(left < right)
            );
            assert_eq!(
                gates::less_or_equal_unsigned_32(left, right),
                u8::from(left <= right)
            );
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
            assert_eq!(gates::multiply_32(left, right), left.wrapping_mul(right));
        }
    }
    assert_eq!(gates::reverse_bytes(0x1122_3344), 0x4433_2211);
    assert_eq!(gates::reverse_halfwords(0x1122_3344), 0x2211_4433);
    assert_eq!(gates::bits_to_u8(&gates::u8_to_bits(0xa5)), 0xa5);
    assert_eq!(gates::bits_to_u16(&gates::u16_to_bits(0xa55a)), 0xa55a);
    assert_eq!(gates::bits_to_u8(&[1; 64]), u8::MAX);
    assert_eq!(gates::bits_to_u16(&[1; 64]), u16::MAX);
    assert_eq!(gates::bits_to_u32(&[1; 64]), u32::MAX);
    assert_eq!(gates::assemble_le_u16([0x34, 0x12]), 0x1234);
    assert_eq!(
        gates::assemble_le_u32([0x78, 0x56, 0x34, 0x12]),
        0x1234_5678
    );
    assert_eq!(gates::split_le_u16(0x1234), [0x34, 0x12]);
    assert_eq!(gates::split_le_u32(0x1234_5678), [0x78, 0x56, 0x34, 0x12]);
    assert_eq!(gates::select_32(7, 9, 0), 7);
    assert_eq!(gates::select_32(7, 9, 1), 9);
    assert_eq!(gates::byte_changed(0x55, 0x55), 0);
    assert_eq!(gates::byte_changed(0x55, 0xaa), 1);
    assert_eq!(gates::is_zero_8(0), 1);
    assert_eq!(gates::is_zero_8(1), 0);
    assert_eq!(gates::popcount_8(0b1010_1100), 4);
}

#[test]
fn exhaustive_decode_space_matches_the_functional_oracle() {
    for first in 0_u16..=u16::MAX {
        let gate = cortex_m23_gatelevel::classify(first, Some(0));
        let functional = cortex_m23_simulator::classify(first, Some(0));
        assert_eq!(
            format!("{gate:?}"),
            format!("{functional:?}"),
            "first halfword {first:#06x}"
        );
    }
    for first in [0xf000_u16, 0xf380, 0xf3bf, 0xf3ef] {
        for second in 0_u16..=u16::MAX {
            let gate = cortex_m23_gatelevel::classify(first, Some(second));
            let functional = cortex_m23_simulator::classify(first, Some(second));
            assert_eq!(
                format!("{gate:?}"),
                format!("{functional:?}"),
                "encoding {first:#06x} {second:#06x}"
            );
        }
    }
    for address in [0, 1, 2, 3, 4, 0xfffc, u32::MAX] {
        for width in [1, 2, 4] {
            assert_eq!(
                gates::aligned_32(address, width),
                u8::from(address % width as u32 == 0)
            );
        }
    }
}

#[test]
fn every_sixteen_bit_subfamily_matches_the_functional_oracle() {
    let mut cases: Vec<(String, u16)> = Vec::new();
    for raw in [0x0008, 0x0808, 0x1008, 0x1808, 0x1a08, 0x1c08, 0x1e08] {
        cases.push((format!("shift-add-sub-{raw:04x}"), raw));
    }
    for raw in [0x2001, 0x2801, 0x3001, 0x3801] {
        cases.push((format!("immediate-{raw:04x}"), raw));
    }
    for operation in 0_u16..16 {
        cases.push((
            format!("data-processing-{operation}"),
            0x4000 | (operation << 6) | 8,
        ));
    }
    for raw in [0x4408, 0x4508, 0x4608, 0x4708, 0x4780, 0x4487, 0x4687] {
        cases.push((format!("special-{raw:04x}"), raw));
    }
    cases.push(("literal-load".into(), 0x4800));
    for operation in 0_u16..8 {
        cases.push((
            format!("register-memory-{operation}"),
            0x5000 | (operation << 9) | (4 << 3),
        ));
    }
    for raw in [0x6020, 0x6820, 0x7020, 0x7820, 0x8020, 0x8820] {
        cases.push((format!("immediate-memory-{raw:04x}"), raw));
    }
    for raw in [0x9400, 0x9c00, 0xa000, 0xa800] {
        cases.push((format!("stack-address-{raw:04x}"), raw));
    }
    for raw in [
        0xb001, 0xb081, 0xb240, 0xb248, 0xb280, 0xb2c0, 0xb401, 0xbc01, 0xba00, 0xba40, 0xbac0,
        0xb662, 0xb672, 0xbe00, 0xbe01, 0xbf00, 0xbf10, 0xbf20, 0xbf30, 0xbf40,
    ] {
        cases.push((format!("misc-{raw:04x}"), raw));
    }
    for raw in [0xc011, 0xc811] {
        cases.push((format!("multiple-{raw:04x}"), raw));
    }
    for condition in 0_u16..16 {
        cases.push((
            format!("conditional-{condition}"),
            0xd000 | (condition << 8),
        ));
    }
    cases.push(("unconditional".into(), 0xe000));
    for raw in [0xb100, 0xb600, 0xde00] {
        cases.push((format!("reserved-{raw:04x}"), raw));
    }

    for (label, raw) in cases {
        compare_transition(&label, &[raw], seeded);
    }
}

#[test]
fn every_thirty_two_bit_subfamily_matches_the_functional_oracle() {
    let bl = encoding::bl(0);
    let mut cases = vec![
        ("bl", bl),
        ("dsb", [0xf3bf, 0x8f4f]),
        ("dmb", [0xf3bf, 0x8f5f]),
        ("isb", [0xf3bf, 0x8f6f]),
    ];
    for sysm in [0_u8, 1, 2, 3, 5, 6, 7, 8, 9, 16, 20, 31] {
        cases.push(("mrs", encoding::mrs(0, sysm)));
        cases.push(("msr", encoding::msr(0, sysm)));
    }
    cases.push(("unknown", [0xf000, 0]));
    for (label, words) in cases {
        compare_transition(&format!("{label}-{words:04x?}"), &words, seeded);
    }
}

#[test]
fn branch_outcomes_privilege_and_process_stack_match() {
    for apsr in [0, 0x4000_0000] {
        compare_transition("branch-outcome", &[0xd000], |state| {
            seeded(state);
            state.apsr = apsr;
        });
    }
    for (control_non_secure, handler_mode, ipsr) in
        [(1, false, 0), (2, false, 0), (3, false, 0), (3, true, 7)]
    {
        compare_transition("privilege-stack", &[0xb401], |state| {
            seeded(state);
            state.control_non_secure = control_non_secure;
            state.handler_mode = handler_mode;
            state.ipsr = ipsr;
        });
    }
}

#[test]
fn every_condition_and_shift_boundary_matches_full_state() {
    for flags in 0_u32..16 {
        let apsr = flags << 28;
        for condition in 0_u16..14 {
            compare_transition(
                &format!("condition-{condition}-flags-{flags:04b}"),
                &[0xd000 | (condition << 8)],
                |state| {
                    seeded(state);
                    state.apsr = apsr;
                },
            );
        }
    }

    for kind in 0_u16..3 {
        for encoded in [0_u16, 1, 31] {
            for (value, carry) in [
                (0_u32, false),
                (1, true),
                (0x8000_0001, false),
                (u32::MAX, true),
            ] {
                let raw = (kind << 11) | (encoded << 6) | (1 << 3);
                compare_transition(
                    &format!("shift-immediate-{kind}-{encoded}-{value:08x}-{carry}"),
                    &[raw],
                    |state| {
                        seeded(state);
                        state.registers[1] = value;
                        state.apsr = if carry { 1 << 29 } else { 0 };
                    },
                );
            }
        }
    }

    for (operation, name) in [(2_u16, "lsl"), (3, "lsr"), (4, "asr"), (7, "ror")] {
        for amount in [0_u32, 1, 31, 32, 33, 63, 64, 255] {
            for (value, carry) in [(1_u32, false), (0x8000_0001, true), (u32::MAX, false)] {
                let raw = 0x4000 | (operation << 6) | (1 << 3);
                compare_transition(
                    &format!("shift-register-{name}-{amount}-{value:08x}-{carry}"),
                    &[raw],
                    |state| {
                        seeded(state);
                        state.registers[0] = value;
                        state.registers[1] = amount;
                        state.apsr = if carry { 1 << 29 } else { 0 };
                    },
                );
            }
        }
    }
}

#[test]
fn arithmetic_flag_and_multiply_edges_match_full_state() {
    let mut observed_nzcv = std::collections::BTreeSet::new();
    for (operation, left, right, carry) in [
        (5_u16, 0_u32, 0_u32, false),
        (5, u32::MAX, 1, false),
        (5, 0x7fff_ffff, 1, false),
        (5, 0x8000_0000, 0x8000_0000, false),
        (5, u32::MAX, 0, true),
        (6, 0, 1, true),
        (6, 1, 1, true),
        (6, 0x8000_0000, 1, true),
        (6, 0x7fff_ffff, u32::MAX, true),
        (10, 0, 0, false),
        (10, 0, 1, false),
        (11, 0x7fff_ffff, 1, false),
    ] {
        let raw = 0x4000 | (operation << 6) | (1 << 3);
        let final_state = compare_transition(
            &format!("flags-{operation}-{left:08x}-{right:08x}-{carry}"),
            &[raw],
            |state| {
                seeded(state);
                state.registers[0] = left;
                state.registers[1] = right;
                state.apsr = if carry { 1 << 29 } else { 0 };
            },
        );
        observed_nzcv.insert(final_state.apsr >> 28);
    }
    for bit in 0..4 {
        assert!(
            observed_nzcv.iter().any(|flags| flags & (1 << bit) == 0),
            "flag bit {bit} never cleared: {observed_nzcv:?}"
        );
        assert!(
            observed_nzcv.iter().any(|flags| flags & (1 << bit) != 0),
            "flag bit {bit} never set: {observed_nzcv:?}"
        );
    }

    for left in [0_u32, 1, 2, 0x7fff_ffff, 0x8000_0000, u32::MAX] {
        for right in [0_u32, 1, 3, 0x8000_0000, u32::MAX] {
            let raw = 0x4000 | (13 << 6) | (1 << 3);
            compare_transition(
                &format!("multiply-{left:08x}-{right:08x}"),
                &[raw],
                |state| {
                    seeded(state);
                    state.registers[0] = left;
                    state.registers[1] = right;
                },
            );
        }
    }
}

#[test]
fn memory_addressing_and_overlap_matrix_matches_full_state() {
    for operation in 0_u16..8 {
        for (rt, rn, rm) in [(0_u16, 1_u16, 2_u16), (1, 1, 2), (2, 1, 2), (1, 1, 1)] {
            let raw = 0x5000 | (operation << 9) | (rm << 6) | (rn << 3) | rt;
            compare_transition(
                &format!("register-memory-{operation}-rt{rt}-rn{rn}-rm{rm}"),
                &[raw],
                |state| {
                    seeded(state);
                    state.registers[rn as usize] = 0x100;
                    state.registers[rm as usize] = 0x100;
                    state.memory[0x200..0x204].copy_from_slice(&[0x80, 0xff, 0x34, 0x12]);
                },
            );
        }
    }

    for opcode in [0b01100_u16, 0b01101, 0b01110, 0b01111, 0b10000, 0b10001] {
        for immediate in [0_u16, 1, 31] {
            for (rt, rn) in [(0_u16, 1_u16), (1, 1)] {
                let raw = (opcode << 11) | (immediate << 6) | (rn << 3) | rt;
                compare_transition(
                    &format!("immediate-memory-{opcode:05b}-{immediate}-rt{rt}-rn{rn}"),
                    &[raw],
                    |state| {
                        seeded(state);
                        state.registers[rn as usize] = 0x200;
                    },
                );
            }
        }
    }

    for rn in 0_u16..8 {
        for mask in [0_u16, 1 << rn, 1, 0x55, 0xff] {
            for load in [false, true] {
                let raw = 0xc000 | (u16::from(load) << 11) | (rn << 8) | mask;
                compare_transition(
                    &format!(
                        "multiple-{}-rn{rn}-mask{mask:02x}",
                        if load { "load" } else { "store" }
                    ),
                    &[raw],
                    |state| {
                        seeded(state);
                        state.registers[rn as usize] = 0x200;
                    },
                );
            }
        }
    }
}

#[test]
fn production_manifest_keeps_the_oracle_test_only() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    let dependency_section = manifest.split("[dev-dependencies]").next().unwrap();
    assert!(!dependency_section.contains("cortex-m23-simulator"));
}
