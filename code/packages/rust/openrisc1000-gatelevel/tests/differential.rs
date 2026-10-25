use openrisc1000_gatelevel::{
    gates, OpenRisc1000GateLevel, OpenRisc1000State as GateState, FLIP_FLOP_COUNT, MEMORY_SIZE,
};
use openrisc1000_simulator::{OpenRisc1000Simulator, OpenRisc1000State as FunctionalState};

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
