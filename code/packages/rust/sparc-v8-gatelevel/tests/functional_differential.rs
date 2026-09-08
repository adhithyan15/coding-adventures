use coding_adventures_sparc_v8_gatelevel::SparcCpu;
use sparc_v8_simulator::execute::Psr;
use sparc_v8_simulator::{SparcState, MEMORY_SIZE};

const PC: usize = 0x100;

fn fnv_byte(value: u64, byte: u8) -> u64 {
    (value ^ u64::from(byte)).wrapping_mul(0x100000001b3)
}

fn state_hash(state: &SparcState) -> u64 {
    let mut value = 0xcbf29ce484222325;
    for scalar in std::iter::once(state.pc)
        .chain([state.npc])
        .chain(state.regs)
        .chain([state.cwp, state.save_depth])
    {
        for byte in scalar.to_le_bytes() {
            value = fnv_byte(value, byte);
        }
    }
    for flag in [state.psr.n, state.psr.z, state.psr.v, state.psr.c] {
        value = fnv_byte(value, u8::from(flag));
    }
    for byte in state.y.to_le_bytes() {
        value = fnv_byte(value, byte);
    }
    for &byte in &state.memory {
        value = fnv_byte(value, byte);
    }
    fnv_byte(value, u8::from(state.halted))
}

fn seeded_state(seed: usize, word: u32) -> SparcState {
    let variant = seed & 3;
    let mut regs =
        std::array::from_fn(|index| ((index as u32 + 1).wrapping_mul(0x1020_3041)) ^ word);
    regs[0] = 0;
    regs[1] = 0x800;
    regs[2] = [0, 1, 0xffff_ffff, 0x8000_0000][variant];
    regs[3] = 3;
    let mut memory: Vec<u8> = (0..MEMORY_SIZE)
        .map(|index| (index as u8).wrapping_mul(29).wrapping_add(0x47))
        .collect();
    memory[PC..PC + 4].copy_from_slice(&word.to_be_bytes());
    SparcState {
        pc: PC as u32,
        npc: PC as u32 + 4,
        regs,
        cwp: (variant % 3) as u32,
        save_depth: if seed == 4 { 2 } else { 0 },
        psr: Psr {
            n: variant & 1 != 0,
            z: variant & 2 != 0,
            v: variant == 3,
            c: matches!(variant, 1 | 2),
        },
        y: 0,
        memory,
        halted: false,
        loaded_origin: 0,
        loaded_len: MEMORY_SIZE,
    }
}

#[test]
fn python_and_functional_full_state_corpus_matches_gate_transitions() {
    let fixture = include_str!("../../sparc-v8-simulator/tests/python_oracle_hashes.txt");
    let mut cpu = SparcCpu::new();
    let mut count = 0;
    let mut errors = 0;
    for line in fixture.lines().filter(|line| !line.is_empty()) {
        let mut fields = line.split_ascii_whitespace();
        let seed: usize = fields.next().unwrap().parse().unwrap();
        let word = u32::from_str_radix(fields.next().unwrap(), 16).unwrap();
        let expected = fields.next().unwrap();
        cpu.restore(&seeded_state(seed, word)).unwrap();
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
    assert_eq!(count, 248);
    assert_eq!(errors, 7);
}
