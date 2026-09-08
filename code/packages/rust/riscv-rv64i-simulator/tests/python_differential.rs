use riscv_rv64i_simulator::{Rv64ISimulator, Rv64IState, MEMORY_SIZE};

fn seed_register(seed: usize, index: usize) -> u64 {
    (seed as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ (index as u64).wrapping_mul(0xd1b5_4a32_d192_ed03)
}

fn seeded_state(cpu: &Rv64ISimulator, name: &str, raw: u32, seed: usize) -> Rv64IState {
    let mut state = cpu.get_state();
    for index in 1..32 {
        state.registers[index] = seed_register(seed, index);
    }
    state.registers[2] = 0x200;
    if (name.starts_with("beq") || name.starts_with("bne")) && seed == 0 {
        state.registers[3] = state.registers[2];
    }
    if name.starts_with("div") || name.starts_with("rem") {
        if seed == 0 {
            state.registers[3] = 0;
        } else if seed == 1 {
            state.registers[2] = 1 << 63;
            state.registers[3] = u64::MAX;
        }
    }
    state.pc = 0;
    state.halted = false;
    for (address, byte) in state.memory.iter_mut().enumerate() {
        *byte = (address as u8)
            .wrapping_mul(37)
            .wrapping_add((seed as u8).wrapping_mul(53))
            .wrapping_add(11);
    }
    state.memory[..4].copy_from_slice(&raw.to_le_bytes());
    state.loaded_origin = 0;
    state.loaded_len = 4;
    state
}

fn hash_projected_state(state: &Rv64IState) -> u64 {
    assert_eq!(state.memory.len(), MEMORY_SIZE);
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
    };
    feed(&state.pc.to_le_bytes());
    for register in state.registers {
        feed(&register.to_le_bytes());
    }
    feed(&state.memory);
    feed(&[u8::from(state.halted)]);
    hash
}

#[test]
fn python_full_state_corpus_matches() {
    let mut count = 0;
    for line in include_str!("python_oracle_hashes.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('|').collect();
        let name = fields[0];
        let seed: usize = fields[1].parse().expect("numeric seed");
        let raw = u32::from_str_radix(fields[2], 16).expect("raw word");
        let expected = u64::from_str_radix(fields[3], 16).expect("hash");
        let mut cpu = Rv64ISimulator::new();
        cpu.load_checked(&raw.to_le_bytes()).unwrap();
        cpu.restore(&seeded_state(&cpu, name, raw, seed)).unwrap();
        cpu.step_checked()
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(
            hash_projected_state(&cpu.get_state()),
            expected,
            "{name} seed {seed}"
        );
        count += 1;
    }
    assert_eq!(count, 364, "corpus must retain every generated vector");
}
