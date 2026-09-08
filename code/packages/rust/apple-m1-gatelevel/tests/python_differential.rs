use apple_m1_gatelevel::AppleM1GateLevel;
use apple_m1_simulator::{AppleM1Simulator, AppleM1State, MEMORY_SIZE};

fn seeded_state(cpu: &AppleM1Simulator, name: &str, raw: u32, seed: usize) -> AppleM1State {
    let mut state = cpu.get_state();
    for index in 0..32 {
        state.registers[index] = (seed as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15)
            ^ (index as u64).wrapping_mul(0xd1b5_4a32_d192_ed03);
        state.vectors[index] = (index as u128 + 1)
            .wrapping_mul(0x0102_0304_0506_0708_090a_0b0c_0d0e_0f11)
            .wrapping_add(seed as u128);
    }
    state.registers[31] = 0;
    state.registers[1] = if name.contains("signed") {
        (-37_i64 - seed as i64) as u64
    } else {
        0x1122_3344_5566_7788 + seed as u64
    };
    state.registers[2] = 0x200;
    if name.ends_with("-d") {
        state.vectors[1] = u128::from((1.25 + seed as f64).to_bits());
        state.vectors[2] = u128::from((2.5 - seed as f64).to_bits());
        state.vectors[3] = u128::from((-3.75_f64).to_bits());
    } else {
        state.vectors[1] = u128::from((1.25 + seed as f32).to_bits());
        state.vectors[2] = u128::from((2.5 - seed as f32).to_bits());
        state.vectors[3] = u128::from((-3.75_f32).to_bits());
    }
    if name.starts_with("div") && seed == 0 {
        state.vectors[2] = 0;
    }
    state.sp = 0x300;
    state.pc = 0;
    state.nzcv = (seed & 0xf) as u8;
    for (address, byte) in state.memory.iter_mut().enumerate() {
        *byte = (address as u8)
            .wrapping_mul(37)
            .wrapping_add((seed as u8).wrapping_mul(53))
            .wrapping_add(11);
    }
    state.memory[..4].copy_from_slice(&raw.to_be_bytes());
    state.loaded_origin = 0;
    state.loaded_len = 4;
    state
}

fn hash_state(state: &AppleM1State) -> u64 {
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
    feed(&state.sp.to_le_bytes());
    feed(&[state.nzcv]);
    for vector in state.vectors {
        feed(&vector.to_le_bytes());
    }
    feed(&state.memory);
    feed(&[u8::from(state.halted)]);
    hash
}

#[test]
fn python_fp_neon_full_state_corpus_matches() {
    let mut count = 0;
    for line in include_str!("../../apple-m1-simulator/tests/python_oracle_hashes.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('|').collect();
        let name = fields[0];
        let seed: usize = fields[1].parse().unwrap();
        let raw = u32::from_str_radix(fields[2], 16).unwrap();
        let expected = u64::from_str_radix(fields[3], 16).unwrap();
        let mut seed_cpu = AppleM1Simulator::new();
        seed_cpu.load_checked(&raw.to_be_bytes()).unwrap();
        let initial = seeded_state(&seed_cpu, name, raw, seed);
        let mut cpu = AppleM1GateLevel::new();
        let mut functional = AppleM1Simulator::new();
        cpu.restore(&initial).unwrap();
        functional.restore(&initial).unwrap();
        let gate_trace = cpu
            .step_checked()
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let functional_trace = functional
            .step_checked()
            .unwrap_or_else(|error| panic!("functional {name}: {error}"));
        assert_eq!(gate_trace, functional_trace, "{name} seed {seed}");
        assert_eq!(hash_state(&cpu.get_state()), expected, "{name} seed {seed}");
        count += 1;
    }
    assert_eq!(count, 360);
}
