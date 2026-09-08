use riscv_gatelevel::Rv32IGateLevel;
use riscv_simulator::csr::{CSR_MCAUSE, CSR_MEPC, CSR_MSCRATCH, CSR_MSTATUS, CSR_MTVEC};
use riscv_simulator::{Rv32ISimulator, Rv32IState, RV32I_MEMORY_SIZE};

fn seed_register(seed: usize, index: usize) -> u32 {
    ((seed as u32 + 1).wrapping_mul(0x9e37_79b9)) ^ (index as u32).wrapping_mul(0xd1b5_4a33)
}

fn seeded_state(cpu: &Rv32IGateLevel, name: &str, raw: u32, seed: usize) -> Rv32IState {
    let mut state = cpu.get_state();
    for index in 1..32 {
        state.registers[index] = seed_register(seed, index);
    }
    state.registers[2] = 0x200;
    state.pc = 0;
    for (index, csr) in [CSR_MSTATUS, CSR_MTVEC, CSR_MSCRATCH, CSR_MEPC, CSR_MCAUSE]
        .into_iter()
        .enumerate()
    {
        let value = seed_register(seed + 7, index);
        match csr {
            CSR_MSTATUS => state.csr_mstatus = value,
            CSR_MTVEC => state.csr_mtvec = value,
            CSR_MSCRATCH => state.csr_mscratch = value,
            CSR_MEPC => state.csr_mepc = value,
            CSR_MCAUSE => state.csr_mcause = value,
            _ => unreachable!(),
        }
    }
    if name == "ecall-halt" {
        state.csr_mtvec = 0;
    } else if name == "ecall-trap" {
        state.csr_mtvec = 0x100;
    } else if name == "mret" {
        state.csr_mepc = 0x100;
    }
    for (address, byte) in state.memory.iter_mut().enumerate() {
        *byte = (address as u8)
            .wrapping_mul(37)
            .wrapping_add((seed as u8).wrapping_mul(53))
            .wrapping_add(11);
    }
    state.memory[..4].copy_from_slice(&raw.to_le_bytes());
    state.halted = false;
    state.loaded_origin = 0;
    state.loaded_len = 4;
    state
}

fn hash_projected_state(state: &Rv32IState) -> u64 {
    assert_eq!(state.memory.len(), RV32I_MEMORY_SIZE);
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
    for csr in [
        state.csr_mstatus,
        state.csr_mtvec,
        state.csr_mscratch,
        state.csr_mepc,
        state.csr_mcause,
    ] {
        feed(&csr.to_le_bytes());
    }
    feed(&state.memory);
    feed(&[u8::from(state.halted)]);
    hash
}

#[test]
fn every_python_full_state_vector_matches_through_gates() {
    let mut count = 0;
    for line in include_str!("../../riscv-simulator/tests/python_oracle_hashes.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let fields: Vec<_> = line.split('|').collect();
        let name = fields[0];
        let seed: usize = fields[1].parse().expect("numeric seed");
        let raw = u32::from_str_radix(fields[2], 16).expect("raw word");
        let expected = u64::from_str_radix(fields[3], 16).expect("hash");
        let mut gate = Rv32IGateLevel::new();
        gate.load_checked(&raw.to_le_bytes())
            .expect("valid raw program");
        let seeded = seeded_state(&gate, name, raw, seed);
        gate.restore(&seeded).expect("valid seeded state");
        let mut functional = Rv32ISimulator::new();
        functional.restore(&seeded).expect("functional seed");
        let functional_trace = functional.step_checked().expect("functional step");
        let gate_trace = gate
            .step_checked()
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(gate_trace, functional_trace, "{name} seed {seed}");
        assert_eq!(
            hash_projected_state(&gate.get_state()),
            expected,
            "{name} seed {seed}"
        );
        count += 1;
    }
    assert_eq!(count, 256);
}
