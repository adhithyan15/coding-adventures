use intel8051_simulator::Intel8051Simulator;

const PC: usize = 0x1000;

fn fnv1a(parts: &[&[u8]]) -> String {
    let mut value = 0xcbf2_9ce4_8422_2325u64;
    for part in parts {
        for byte in *part {
            value ^= u64::from(*byte);
            value = value.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("{value:016x}")
}

#[test]
fn every_opcode_matches_python_full_state() {
    let expected: Vec<&str> = include_str!("python_oracle_hashes.txt").lines().collect();
    assert_eq!(expected.len(), 256);

    for opcode in 0u8..=u8::MAX {
        let mut simulator = Intel8051Simulator::new();
        let mut state = simulator.get_state();
        for (index, byte) in state.code.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(37).wrapping_add(11);
        }
        for (index, byte) in state.xdata.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(17).wrapping_add(3);
        }
        for (index, byte) in state.iram.iter_mut().enumerate() {
            *byte = (index as u8).wrapping_mul(29).wrapping_add(7);
        }
        state.pc = PC as u16;
        state.halted = false;
        state.loaded_origin = 0;
        state.loaded_len = 65_536;
        state.code[PC..PC + 3].copy_from_slice(&[opcode, 0x20, 0x02]);
        state.iram[0] = 0x40;
        state.iram[1] = 0x41;
        state.iram[0x81] = 0x30;
        state.iram[0x82] = 0x45;
        state.iram[0x83] = 0x23;
        state.iram[0xa0] = 0x12;
        state.iram[0xd0] = 0xc0;
        state.iram[0xe0] = 0x35;
        state.iram[0xf0] = 0x07;
        simulator.restore(&state).unwrap();

        simulator
            .step_checked()
            .unwrap_or_else(|error| panic!("opcode {opcode:#04x}: {error}"));
        let state = simulator.get_state();
        let pc = state.pc.to_be_bytes();
        let halted = [u8::from(state.halted)];
        let digest = fnv1a(&[&pc, &halted, &state.iram, &state.xdata, &state.code]);
        assert_eq!(digest, expected[opcode as usize], "opcode {opcode:#04x}");
    }
}
