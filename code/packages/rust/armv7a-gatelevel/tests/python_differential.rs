use armv7a_gatelevel::Armv7AGateLevel;
use armv7a_simulator::{Armv7ASimulator, Armv7AState, MEMORY_SIZE, PC};

fn seed_register(seed: usize, index: usize) -> u32 {
    ((seed as u32 + 1).wrapping_mul(0x9e37_79b9)) ^ (index as u32).wrapping_mul(0xd1b5_4a33)
}

fn decode_hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            u8::from_str_radix(std::str::from_utf8(pair).expect("ASCII hex pair"), 16)
                .expect("valid hex byte")
        })
        .collect()
}

fn seeded_state(cpu: &Armv7AGateLevel, raw: &[u8], seed: usize) -> Armv7AState {
    let mut state = cpu.get_state();
    for index in 0..15 {
        state.registers[index] = seed_register(seed, index);
    }
    state.pc = 0;
    state.registers[PC] = 0;
    state.cpsr = (1 << 5)
        | (((seed & 1) as u32) << 31)
        | ((((seed >> 1) & 1) as u32) << 30)
        | ((((seed + 1) & 1) as u32) << 29)
        | ((((seed >> 1) & 1) as u32) << 28);
    for (address, byte) in state.memory.iter_mut().enumerate() {
        *byte = (address as u8)
            .wrapping_mul(37)
            .wrapping_add((seed as u8).wrapping_mul(53))
            .wrapping_add(11);
    }
    state.memory[..raw.len()].copy_from_slice(raw);
    state.halted = false;
    state
}

fn hash_projected_state(state: &Armv7AState) -> u64 {
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
    feed(&state.cpsr.to_le_bytes());
    feed(&state.memory);
    feed(&[u8::from(state.halted)]);
    hash
}

#[test]
fn every_python_full_state_vector_matches_through_gates() {
    let mut count = 0;
    for line in include_str!("../../armv7a-simulator/tests/python_oracle_hashes.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('|').collect();
        let seed: usize = fields[1].parse().expect("numeric seed");
        let raw = decode_hex(fields[2]);
        let expected = u64::from_str_radix(fields[3], 16).expect("hash");
        let mut cpu = Armv7AGateLevel::new();
        cpu.load_checked(&raw).expect("valid raw program");
        let seeded = seeded_state(&cpu, &raw, seed);
        cpu.restore(&seeded).expect("valid seeded state");
        let mut functional = Armv7ASimulator::new();
        functional.restore(&seeded).expect("functional seed");
        let functional_trace = functional.step_checked().expect("functional step");
        let gate_trace = cpu
            .step_checked()
            .unwrap_or_else(|error| panic!("{}: {error}", fields[0]));
        assert_eq!(gate_trace, functional_trace, "{} seed {seed}", fields[0]);
        assert_eq!(
            hash_projected_state(&cpu.get_state()),
            expected,
            "{} seed {seed}",
            fields[0]
        );
        count += 1;
    }
    assert_eq!(count, 417);
}
