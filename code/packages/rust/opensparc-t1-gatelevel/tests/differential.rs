use opensparc_t1_gatelevel::{
    gates, OpenSparcT1GateLevel, OpenSparcT1State as GateState, FLIP_FLOP_COUNT,
};
use opensparc_t1_simulator::{OpenSparcT1Simulator, OpenSparcT1State as FunctionalState};

fn assert_state_eq(gate: &GateState, functional: &FunctionalState, label: &str) {
    assert_eq!(gate.registers, functional.registers, "{label}: registers");
    assert_eq!(gate.pc, functional.pc, "{label}: pc");
    assert_eq!(gate.npc, functional.npc, "{label}: npc");
    assert_eq!(gate.ccr.icc.n, functional.ccr.icc.n, "{label}: icc.n");
    assert_eq!(gate.ccr.icc.z, functional.ccr.icc.z, "{label}: icc.z");
    assert_eq!(gate.ccr.icc.v, functional.ccr.icc.v, "{label}: icc.v");
    assert_eq!(gate.ccr.icc.c, functional.ccr.icc.c, "{label}: icc.c");
    assert_eq!(gate.ccr.xcc.n, functional.ccr.xcc.n, "{label}: xcc.n");
    assert_eq!(gate.ccr.xcc.z, functional.ccr.xcc.z, "{label}: xcc.z");
    assert_eq!(gate.ccr.xcc.v, functional.ccr.xcc.v, "{label}: xcc.v");
    assert_eq!(gate.ccr.xcc.c, functional.ccr.xcc.c, "{label}: xcc.c");
    assert_eq!(gate.y, functional.y, "{label}: y");
    assert_eq!(gate.asi, functional.asi, "{label}: asi");
    assert_eq!(
        gate.pstate.privileged, functional.pstate.privileged,
        "{label}: privileged"
    );
    assert_eq!(gate.pstate.cle, functional.pstate.cle, "{label}: cle");
    assert_eq!(
        gate.pstate.address_mask, functional.pstate.address_mask,
        "{label}: address mask"
    );
    assert_eq!(gate.gl, functional.gl, "{label}: gl");
    assert_eq!(gate.cwp, functional.cwp, "{label}: cwp");
    assert_eq!(gate.cansave, functional.cansave, "{label}: cansave");
    assert_eq!(
        gate.canrestore, functional.canrestore,
        "{label}: canrestore"
    );
    assert_eq!(gate.cleanwin, functional.cleanwin, "{label}: cleanwin");
    assert_eq!(gate.otherwin, functional.otherwin, "{label}: otherwin");
    assert_eq!(gate.wstate, functional.wstate, "{label}: wstate");
    assert_eq!(gate.memory, functional.memory, "{label}: memory");
    assert_eq!(gate.halted, functional.halted, "{label}: halted");
    assert_eq!(
        gate.delay_slot, functional.delay_slot,
        "{label}: delay slot"
    );
    assert_eq!(
        gate.loaded_origin, functional.loaded_origin,
        "{label}: origin"
    );
    assert_eq!(gate.loaded_len, functional.loaded_len, "{label}: length");
}

fn compare_transition(label: &str, word: u32) {
    let program = word.to_be_bytes();
    let mut gate = OpenSparcT1GateLevel::new();
    let mut functional = OpenSparcT1Simulator::new();
    gate.load(&program, 0x100).unwrap();
    functional.load(&program, 0x100).unwrap();

    for (register, value) in [
        (1, 0x300_u64),
        (2, 3),
        (3, 0x8000_0001_ffff_fffe),
        (4, 7),
        (15, 0x100),
    ] {
        gate.write_register(register, value).unwrap();
        functional.write_register(register, value).unwrap();
    }
    gate.write_memory(0x300, 8, 0x8001_0203_0405_0607).unwrap();
    functional
        .write_memory(0x300, 8, 0x8001_0203_0405_0607)
        .unwrap();

    let mut gate_state = gate.get_state();
    let mut functional_state = functional.get_state();
    gate_state.y = 5;
    functional_state.y = 5;
    gate_state.ccr.icc.c = true;
    functional_state.ccr.icc.c = true;
    gate_state.ccr.xcc.n = true;
    functional_state.ccr.xcc.n = true;
    gate.restore(gate_state).unwrap();
    functional.restore(functional_state).unwrap();

    match (gate.step(), functional.step()) {
        (Ok(gate_trace), Ok(functional_trace)) => {
            assert_eq!(gate_trace.pc_before, functional_trace.pc_before, "{label}");
            assert_eq!(
                gate_trace.npc_before, functional_trace.npc_before,
                "{label}"
            );
            assert_eq!(gate_trace.pc_after, functional_trace.pc_after, "{label}");
            assert_eq!(gate_trace.npc_after, functional_trace.npc_after, "{label}");
            assert_eq!(gate_trace.raw, functional_trace.raw, "{label}");
            assert_eq!(gate_trace.raw_word, functional_trace.raw_word, "{label}");
            assert_eq!(gate_trace.mnemonic, functional_trace.mnemonic, "{label}");
            assert_state_eq(
                &gate_trace.state_after,
                &functional_trace.state_after,
                label,
            );
            assert_state_eq(&gate.get_state(), &functional.get_state(), label);
        }
        (Err(gate_error), Err(functional_error)) => {
            assert_eq!(
                format!("{gate_error:?}"),
                format!("{functional_error:?}"),
                "{label}"
            );
            assert_state_eq(&gate.get_state(), &functional.get_state(), label);
        }
        (gate_result, functional_result) => {
            panic!("{label} diverged: gate={gate_result:?}, functional={functional_result:?}")
        }
    }
}

#[test]
fn topology_and_combinational_networks_are_exact() {
    let cpu = OpenSparcT1GateLevel::new();
    assert_eq!(FLIP_FLOP_COUNT, 534_813);
    assert_eq!(cpu.flip_flop_count(), FLIP_FLOP_COUNT);
    assert!(cpu.gate_count() > FLIP_FLOP_COUNT * 6);

    for format in 0_u32..4 {
        let lines = gates::decode_format(format << 30);
        assert_eq!(lines.iter().filter(|&&line| line == 1).count(), 1);
        assert_eq!(lines[format as usize], 1);
    }
    for op3 in 0_u32..64 {
        let lines = gates::decode_op3(op3 << 19);
        assert_eq!(lines.iter().filter(|&&line| line == 1).count(), 1);
        assert_eq!(lines[op3 as usize], 1);
    }

    let values = [0, 1, 2, 31, 63, 0x7fff_ffff, 0x8000_0000, u64::MAX];
    for left in values {
        for right in values {
            assert_eq!(gates::add_64(left, right, 0).0, left.wrapping_add(right));
            assert_eq!(gates::sub_64(left, right, 0).0, left.wrapping_sub(right));
            assert_eq!(gates::bit_and_64(left, right), left & right);
            assert_eq!(gates::bit_or_64(left, right), left | right);
            assert_eq!(gates::bit_xor_64(left, right), left ^ right);
            assert_eq!(gates::equal_64(left, right), u8::from(left == right));
            assert_eq!(gates::unsigned_less_64(left, right), u8::from(left < right));
            assert_eq!(
                gates::signed_less_64(left, right),
                u8::from((left as i64) < (right as i64))
            );
            let amount = (right & 63) as u32;
            assert_eq!(
                gates::shift_left_64(left, amount),
                left.wrapping_shl(amount)
            );
            assert_eq!(
                gates::shift_right_64(left, amount),
                left.wrapping_shr(amount)
            );
            assert_eq!(
                gates::shift_arithmetic_right_64(left, amount),
                ((left as i64) >> amount) as u64
            );
            assert_eq!(
                gates::multiply_unsigned_64(left, right),
                left.wrapping_mul(right)
            );
            assert_eq!(
                gates::multiply_signed_64(left, right),
                (left as i64).wrapping_mul(right as i64) as u64
            );
            if right != 0 {
                assert_eq!(gates::divide_unsigned_64(left, right), left / right);
                if !(left == i64::MIN as u64 && right == u64::MAX) {
                    assert_eq!(
                        gates::divide_signed_64(left, right),
                        ((left as i64) / (right as i64)) as u64
                    );
                }
            }
        }
    }
}

#[test]
fn every_decode_family_and_subfamily_matches_the_functional_oracle() {
    compare_transition("format-1-call", (1 << 30) | 1);

    for op2 in 0_u32..8 {
        let word = (4 << 25) | (op2 << 22) | 1;
        compare_transition(&format!("format-2 op2={op2:#x}"), word);
    }

    for op3 in 0_u32..64 {
        let register = (2 << 30) | (4 << 25) | (op3 << 19) | (1 << 14) | 2;
        compare_transition(
            &format!("format-3 arithmetic op3={op3:#04x} register"),
            register,
        );
        let immediate = (2 << 30) | (4 << 25) | (op3 << 19) | (1 << 14) | (1 << 13) | 2;
        compare_transition(
            &format!("format-3 arithmetic op3={op3:#04x} immediate"),
            immediate,
        );

        let memory = (3 << 30) | (4 << 25) | (op3 << 19) | (1 << 14) | (1 << 13) | 8;
        compare_transition(&format!("format-3 memory op3={op3:#04x}"), memory);
    }
}
