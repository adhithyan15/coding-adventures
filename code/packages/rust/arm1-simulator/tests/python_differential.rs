use arm1_simulator::{Arm1State, ARM1};

const MEMORY_SIZE: usize = 16_384;
const PC: usize = 0x100;

fn fnv_byte(value: u64, byte: u8) -> u64 {
    (value ^ u64::from(byte)).wrapping_mul(0x100000001B3)
}

fn state_hash(state: &Arm1State) -> u64 {
    let mut value = 0xCBF29CE484222325;
    for register in state.regs {
        for byte in register.to_le_bytes() {
            value = fnv_byte(value, byte);
        }
    }
    for &byte in &state.memory {
        value = fnv_byte(value, byte);
    }
    fnv_byte(value, u8::from(state.halted))
}

fn seeded_state(word: u32) -> Arm1State {
    let mut regs =
        std::array::from_fn(|index| ((index as u32 + 1).wrapping_mul(0x1020_3041)) ^ word);
    let flags = ((word >> 4) & 0xF) << 28;
    regs[15] = flags | PC as u32 | 3;
    regs[1] = 0x800;
    regs[2] = 0x20;
    regs[3] = 3;
    let mut memory: Vec<u8> = (0..MEMORY_SIZE)
        .map(|index| (index as u8).wrapping_mul(29).wrapping_add(0x47))
        .collect();
    memory[PC..PC + 4].copy_from_slice(&word.to_le_bytes());
    Arm1State {
        regs,
        memory,
        halted: false,
        loaded_origin: 0,
        loaded_len: MEMORY_SIZE,
    }
}

#[test]
fn python_instruction_family_corpus_matches_full_state() {
    let fixture = include_str!("python_oracle_hashes.txt");
    let mut cpu = ARM1::new(MEMORY_SIZE);
    let mut count = 0;
    for line in fixture.lines().filter(|line| !line.is_empty()) {
        let (word, expected) = line.split_once(' ').expect("word and hash");
        let word = u32::from_str_radix(word, 16).expect("instruction word");
        let expected = u64::from_str_radix(expected, 16).expect("state hash");
        cpu.restore(&seeded_state(word)).unwrap();
        let trace = cpu
            .step_checked()
            .unwrap_or_else(|error| panic!("instruction {word:#010x}: {error}"));
        assert_eq!(state_hash(&trace.state_after), expected, "{word:#010x}");
        count += 1;
    }
    assert_eq!(count, 599);
}
