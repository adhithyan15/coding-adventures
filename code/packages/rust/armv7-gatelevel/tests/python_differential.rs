use arm_simulator::functional::{ArmState, Flags};
use armv7_gatelevel::Armv7GateLevel;

fn seeded_state(cpu: &Armv7GateLevel, raw: u32, seed: usize) -> ArmState {
    let mut state = cpu.get_state();
    for index in 0..15 {
        state.registers[index] = ((seed as u32 + 1).wrapping_mul(0x9e37_79b9))
            ^ (index as u32).wrapping_mul(0xd1b5_4a33);
    }
    for (address, byte) in state.memory.iter_mut().enumerate() {
        *byte = (address as u8)
            .wrapping_mul(37)
            .wrapping_add((seed as u8).wrapping_mul(53))
            .wrapping_add(11);
    }
    state.memory[..4].copy_from_slice(&raw.to_le_bytes());
    state.flags = Flags::default();
    state
}

fn hash_state(state: &ArmState) -> u64 {
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
    feed(&[
        u8::from(state.flags.n),
        u8::from(state.flags.z),
        u8::from(state.flags.c),
        u8::from(state.flags.v),
    ]);
    feed(&state.memory);
    feed(&[u8::from(state.halted)]);
    hash
}

#[test]
fn all_python_common_surface_vectors_match_through_gates() {
    let mut count = 0;
    for line in include_str!("../../arm-simulator/tests/python_oracle_hashes.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('|').collect();
        let seed: usize = fields[1].parse().unwrap();
        let raw = u32::from_str_radix(fields[2], 16).unwrap();
        let mut cpu = Armv7GateLevel::new();
        cpu.load_checked(&raw.to_le_bytes()).unwrap();
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
    assert_eq!(count, 388);
}
