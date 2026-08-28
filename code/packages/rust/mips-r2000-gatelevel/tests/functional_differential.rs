use coding_adventures_mips_r2000_gatelevel::{CpuMipsR2000, MipsState};

const PC: usize = 0x100;

fn fnv_byte(value: u64, byte: u8) -> u64 {
    (value ^ u64::from(byte)).wrapping_mul(0x100000001B3)
}

fn state_hash(state: &MipsState) -> u64 {
    let mut value = 0xCBF29CE484222325;
    for scalar in std::iter::once(state.pc)
        .chain(state.regs)
        .chain([state.hi, state.lo])
    {
        for byte in scalar.to_le_bytes() {
            value = fnv_byte(value, byte);
        }
    }
    for &byte in &state.memory {
        value = fnv_byte(value, byte);
    }
    fnv_byte(value, u8::from(state.halted))
}

fn seeded_state(variant: usize, word: u32) -> MipsState {
    let mut regs =
        std::array::from_fn(|index| ((index as u32 + 1).wrapping_mul(0x1020_3041)) ^ word);
    regs[0] = 0;
    regs[1] = 0x800;
    regs[2] = [0, 1, 0xFFFF_FFFF, 0x8000_0000][variant];
    regs[3] = 3;
    regs[31] = 0x300;
    let mut memory: Vec<u8> = (0..65_536)
        .map(|index| (index as u8).wrapping_mul(29).wrapping_add(0x47))
        .collect();
    memory[PC..PC + 4].copy_from_slice(&word.to_be_bytes());
    MipsState {
        pc: PC as u32,
        regs,
        hi: 0x1357_9BDF ^ word,
        lo: 0x2468_ACE0 ^ word,
        memory,
        halted: false,
        loaded_origin: 0,
        loaded_len: 65_536,
    }
}

#[test]
fn gate_machine_matches_python_decode_and_fault_corpus_complete_state() {
    let fixture = include_str!("../../mips-r2000-simulator/tests/python_oracle_hashes.txt");
    let mut cpu = CpuMipsR2000::new();
    let mut count = 0;
    let mut errors = 0;
    for line in fixture.lines().filter(|line| !line.is_empty()) {
        let mut fields = line.split_ascii_whitespace();
        let variant: usize = fields.next().unwrap().parse().unwrap();
        let word = u32::from_str_radix(fields.next().unwrap(), 16).unwrap();
        let expected = fields.next().unwrap();
        cpu.restore(&seeded_state(variant, word)).unwrap();
        let before = cpu.get_state();
        if expected == "ERROR" {
            assert!(cpu.step_checked().is_err(), "{word:#010x}");
            assert_eq!(cpu.get_state(), before, "{word:#010x} must fail atomically");
            errors += 1;
        } else {
            let trace = cpu
                .step_checked()
                .unwrap_or_else(|error| panic!("instruction {word:#010x}: {error}"));
            let expected = u64::from_str_radix(expected, 16).unwrap();
            assert_eq!(state_hash(&trace.state_after), expected, "{word:#010x}");
        }
        count += 1;
    }
    assert_eq!(count, 218);
    assert!(errors >= 6);
}
