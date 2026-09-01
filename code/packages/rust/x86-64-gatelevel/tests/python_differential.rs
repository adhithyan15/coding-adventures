use coding_adventures_x86_64_gatelevel::{X86GateSimulator, X86State};

fn seeded_state(cpu: &X86GateSimulator, program: &[u8], seed: usize) -> X86State {
    let mut state = cpu.get_state();
    for index in 0..16 {
        state.gpr[index] = ((seed as u64 + 1).wrapping_mul(0x9e37_79b9_7f4a_7c15))
            ^ (index as u64).wrapping_mul(0xd1b5_4a32_d192_ed03);
    }
    state.gpr[0] = [0, 1, 0x8000_0000_0000_0000, u64::MAX][seed];
    state.gpr[1] = [0, 1, 7, 63][seed];
    state.gpr[2] = 0;
    state.gpr[4] = 0xff00;
    state.gpr[7] = 0x0200;
    state.rflags = [0, 1, 1 << 6, (1 << 7) | (1 << 11)][seed];
    for (address, byte) in state.memory.iter_mut().enumerate() {
        *byte = (address as u8)
            .wrapping_mul(37)
            .wrapping_add((seed as u8).wrapping_mul(53))
            .wrapping_add(11);
    }
    state.memory[..program.len()].copy_from_slice(program);
    state
}

fn hash_state(state: &X86State) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    let mut feed = |bytes: &[u8]| {
        for byte in bytes {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
    };
    feed(&state.rip.to_le_bytes());
    for register in state.gpr {
        feed(&register.to_le_bytes());
    }
    feed(&state.rflags.to_le_bytes());
    feed(&state.memory);
    feed(&[u8::from(state.halted)]);
    hash
}

fn decode_hex(text: &str) -> Vec<u8> {
    assert_eq!(text.len() % 2, 0);
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
        .collect()
}

#[test]
fn python_full_state_decode_and_fault_corpus_matches_gate_machine() {
    let fixture = include_str!("../../x86-simulator/tests/python_oracle_hashes.txt");
    let mut count = 0;
    let mut errors = 0;
    for line in fixture.lines().filter(|line| !line.starts_with('#')) {
        let fields: Vec<_> = line.split('|').collect();
        assert_eq!(fields.len(), 5, "{line}");
        let seed: usize = fields[1].parse().unwrap();
        let instruction = decode_hex(fields[2]);
        let mut program = instruction.clone();
        if fields[3] != "ERROR" {
            program.push(0xf4);
        }
        let mut cpu = X86GateSimulator::new();
        cpu.load_checked(&program).unwrap();
        let state = seeded_state(&cpu, &program, seed);
        cpu.restore(&state).unwrap();
        let before = cpu.get_state();
        if fields[3] == "ERROR" {
            assert!(cpu.step_checked().is_err(), "{}", fields[0]);
            assert_eq!(cpu.get_state(), before, "{}", fields[0]);
            errors += 1;
        } else {
            cpu.step_checked()
                .unwrap_or_else(|error| panic!("{}: {error}", fields[0]));
            let expected = u64::from_str_radix(fields[4], 16).unwrap();
            assert_eq!(
                hash_state(&cpu.get_state()),
                expected,
                "{}/seed{} flags={:#x}",
                fields[0],
                seed,
                cpu.get_state().rflags
            );
        }
        count += 1;
    }
    assert!(
        count >= 250,
        "expected broad full-state corpus, got {count}"
    );
    assert_eq!(errors, 2);
}
