use alpha_axp_simulator::{AlphaSimulator, AlphaState};

fn seeded_state(cpu: &AlphaSimulator, raw: u32, seed: usize) -> AlphaState {
    let mut state = cpu.get_state();
    for index in 0..31 {
        state.regs[index] = ((seed as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15))
            ^ (index as u64).wrapping_mul(0xd1b5_4a32_d192_ed03);
    }
    state.regs[1] = [0, 1, u64::MAX, 0x8000_0000_0000_0000][seed];
    state.regs[2] = 0x200 + seed as u64 * 8;
    state.regs[3] = 0x0123_4567_89ab_cdef;
    for (address, byte) in state.memory.iter_mut().enumerate() {
        *byte = (address as u8)
            .wrapping_mul(37)
            .wrapping_add((seed as u8).wrapping_mul(53))
            .wrapping_add(11);
    }
    state.memory[..4].copy_from_slice(&raw.to_le_bytes());
    state
}

fn hash_state(state: &AlphaState) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
    };
    feed(&state.pc.to_le_bytes());
    feed(&state.npc.to_le_bytes());
    for register in state.regs {
        feed(&register.to_le_bytes());
    }
    feed(&state.memory);
    feed(&[u8::from(state.halted)]);
    hash
}

#[test]
fn python_full_state_decode_and_fault_corpus_matches() {
    let fixture = include_str!("python_oracle_hashes.txt");
    let mut count = 0;
    let mut errors = 0;
    for line in fixture.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<_> = line.split('|').collect();
        assert_eq!(fields.len(), 5, "{line}");
        let seed: usize = fields[1].parse().unwrap();
        let raw = u32::from_str_radix(fields[2], 16).unwrap();
        let mut cpu = AlphaSimulator::new();
        cpu.load_checked(&raw.to_le_bytes()).unwrap();
        let state = seeded_state(&cpu, raw, seed);
        cpu.restore(&state).unwrap();
        let before = cpu.get_state();
        if fields[3] == "ERROR" {
            assert!(cpu.step_checked().is_err(), "{}", fields[0]);
            assert_eq!(cpu.get_state(), before, "{}", fields[0]);
            errors += 1;
        } else {
            let trace = cpu
                .step_checked()
                .unwrap_or_else(|error| panic!("{}: {error}", fields[0]));
            assert_eq!(trace.mnemonic, fields[3], "{}", fields[0]);
            let expected = u64::from_str_radix(fields[4], 16).unwrap();
            assert_eq!(hash_state(&cpu.get_state()), expected, "{}", fields[0]);
        }
        count += 1;
    }
    assert!(
        count >= 500,
        "expected a broad full-state corpus, got {count}"
    );
    assert_eq!(errors, 7);
}
