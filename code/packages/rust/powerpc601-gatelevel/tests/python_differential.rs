use coding_adventures_powerpc601_gatelevel::{PowerPc601GateSimulator, PowerPcState};

fn seeded_state(cpu: &PowerPc601GateSimulator, raw: u32, seed: usize) -> PowerPcState {
    let mut state = cpu.get_state();
    for (index, register) in state.gpr.iter_mut().enumerate() {
        *register = ((seed as u32 + 1).wrapping_mul(0x9e37_79b9))
            ^ (index as u32).wrapping_mul(0xd1b5_4a33);
    }
    state.gpr[1] = 0x200;
    state.gpr[2] = [0, 1, u32::MAX, 0x8000_0000][seed];
    state.gpr[3] = [0, 3, 31, 63][seed];
    for (address, byte) in state.memory.iter_mut().enumerate() {
        *byte = (address as u8)
            .wrapping_mul(37)
            .wrapping_add((seed as u8).wrapping_mul(53))
            .wrapping_add(11);
    }
    state.memory[..4].copy_from_slice(&raw.to_be_bytes());
    state.lr = 0x100 + seed as u32 * 4;
    state.ctr = seed as u32 + 1;
    state.xer = [0, 1 << 29, 1 << 31, (1 << 31) | (1 << 29)][seed];
    state.cr = [0, 0x8000_0000, 0x2000_0000, 0xf0f0_0f0f][seed];
    state
}

fn hash_state(state: &PowerPcState) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
    };
    feed(&state.cia.to_be_bytes());
    for register in state.gpr {
        feed(&register.to_be_bytes());
    }
    for register in [state.lr, state.ctr, state.xer, state.cr] {
        feed(&register.to_be_bytes());
    }
    feed(&state.memory);
    feed(&[u8::from(state.halted)]);
    hash
}

#[test]
fn python_full_state_decode_corpus_matches_gate_machine() {
    let mut count = 0;
    for line in include_str!("../../powerpc601-simulator/tests/python_oracle_hashes.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('|').collect();
        let seed: usize = fields[1].parse().unwrap();
        let raw = u32::from_str_radix(fields[2], 16).unwrap();
        let mut cpu = PowerPc601GateSimulator::new();
        cpu.load_checked(&raw.to_be_bytes()).unwrap();
        cpu.restore(&seeded_state(&cpu, raw, seed)).unwrap();
        cpu.step_checked()
            .unwrap_or_else(|error| panic!("{}: {error}", fields[0]));
        assert_eq!(
            hash_state(&cpu.get_state()),
            u64::from_str_radix(fields[3], 16).unwrap(),
            "{}",
            fields[0]
        );
        count += 1;
    }
    assert_eq!(count, 239);
}
