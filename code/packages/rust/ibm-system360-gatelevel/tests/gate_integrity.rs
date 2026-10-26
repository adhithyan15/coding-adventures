use ibm_system360_gatelevel::gates;

#[test]
fn branch_predicates_cover_every_selector_wire_and_boundary() {
    for mask in 0_u8..16 {
        for condition_code in 0_u8..4 {
            let expected = u8::from(mask & (8 >> condition_code) != 0);
            assert_eq!(
                gates::branch_selected(mask, condition_code),
                expected,
                "mask={mask:#06b} cc={condition_code}"
            );
        }
    }

    for target_register in 0_u8..16 {
        let target_present = u8::from(target_register != 0);
        assert_eq!(
            gates::register_field_nonzero(target_register),
            target_present
        );
        for count in [0, 1, 2, u32::MAX, 0x8000_0000] {
            assert_eq!(
                gates::count_branch_selected(count, target_register),
                u8::from(count != 0 && target_register != 0),
                "count={count:#010x} target=r{target_register}"
            );
        }
    }

    assert_eq!(gates::nonzero_32(0), 0);
    for bit in 0..32 {
        assert_eq!(gates::nonzero_32(1_u32 << bit), 1, "count wire {bit}");
    }

    for opcode in [0x86, 0x87] {
        for comparison in 0_u8..4 {
            let expected = u8::from(if opcode == 0x86 {
                comparison == 2
            } else {
                comparison != 2
            });
            assert_eq!(
                gates::index_branch_selected(opcode, comparison),
                expected,
                "opcode={opcode:#04x} comparison={comparison}"
            );
        }
    }
    assert_eq!(gates::index_branch_selected(0xff, 2), 0);
}

#[test]
fn production_branch_paths_do_not_reconstruct_gate_predicates_on_the_host() {
    let networks = include_str!("../src/gates.rs");
    let executor = include_str!("../src/executor.rs");
    let validator = include_str!("../src/lib.rs");
    for forbidden in [
        "u8::from(r2 != 0)",
        "u8::from(value != 0 && r2 != 0)",
        "u8::from(decremented != 0)",
        "u8::from(decremented != 0 && r2 != 0)",
        "signed_compare_condition_code(incremented, before.gprs[compare]) == 2",
        "(opcode == 0x86 && high) || (opcode == 0x87 && !high)",
    ] {
        assert!(
            !executor.contains(forbidden),
            "executor rebuilt a branch predicate on the host: {forbidden}"
        );
        assert!(
            !validator.contains(forbidden),
            "validator rebuilt a branch predicate on the host: {forbidden}"
        );
    }

    for forbidden in [
        "3_u8.wrapping_sub(condition_code)",
        "mask >> line",
        "u8::from(condition_code < 4)",
        "u8::from(field != 0)",
        "u8::from(value != 0)",
        "compare_condition_code == 2",
        "opcode == 0x86",
        "opcode == 0x87",
    ] {
        assert!(
            !networks.contains(forbidden),
            "gate network rebuilt a branch predicate on the host: {forbidden}"
        );
    }

    for required in [
        "mux2(mask[3], mask[2], condition[0])",
        "mux2(mask[1], mask[0], condition[0])",
        "not_gate(is_zero_32(value))",
        "and_gate(nonzero_32(count), register_field_nonzero(target_register))",
        "let decode = decode_byte(opcode);",
        "and_gate(decode[0x86], high)",
        "and_gate(decode[0x87], not_gate(high))",
    ] {
        assert!(
            networks.contains(required),
            "gate branch network lost required primitive reduction: {required}"
        );
    }

    for source in [executor, validator] {
        assert!(source.contains("gates::register_field_nonzero(r2 as u8)"));
        assert!(source.contains("gates::count_branch_selected("));
        assert!(source.contains("gates::nonzero_32(decremented)"));
        assert!(source.contains("gates::index_branch_selected(opcode, comparison)"));
    }
}
