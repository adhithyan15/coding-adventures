use coding_adventures_z80_gatelevel::{GateLevelCpuZ80, Z80Error, Z80State, FLIP_FLOP_COUNT};
use z80_simulator::Z80Simulator;

const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

fn fnv_bytes(hash: &mut u64, bytes: impl IntoIterator<Item = u8>) {
    for byte in bytes {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

fn oracle_seed() -> Z80State {
    let sim = Z80Simulator::new(65_536);
    let mut state = sim.snapshot();
    state.regs.a = 0x91;
    state.regs.b = 0x02;
    state.regs.c = 0x10;
    state.regs.d = 0x20;
    state.regs.e = 0x30;
    state.regs.h = 0x40;
    state.regs.l = 0x20;
    state.regs.a2 = 0xA1;
    state.regs.f2 = 0xD7;
    state.regs.b2 = 0xB2;
    state.regs.c2 = 0xC3;
    state.regs.d2 = 0xD4;
    state.regs.e2 = 0xE5;
    state.regs.h2 = 0xF6;
    state.regs.l2 = 0x17;
    state.regs.ix = 0x2000;
    state.regs.iy = 0x3000;
    state.regs.sp = 0xF000;
    state.regs.i = 0x12;
    state.regs.r = 0x34;
    state.flags.s = true;
    state.flags.z = false;
    state.flags.h = true;
    state.flags.pv = false;
    state.flags.n = true;
    state.flags.c = true;
    state.iff1 = true;
    state.iff2 = true;
    state.im = 2;
    for (address, byte) in state.memory.iter_mut().enumerate() {
        *byte = (address as u8).wrapping_mul(37).wrapping_add(11);
    }
    for (port, byte) in state.input_ports.iter_mut().enumerate() {
        *byte = (port as u8).wrapping_mul(13).wrapping_add(7);
    }
    for (port, byte) in state.output_ports.iter_mut().enumerate() {
        *byte = (port as u8).wrapping_mul(17).wrapping_add(3);
    }
    state
}

fn hash_state(hash: &mut u64, state: &Z80State) {
    let regs = state.regs;
    fnv_bytes(
        hash,
        [
            regs.a, regs.b, regs.c, regs.d, regs.e, regs.h, regs.l, regs.a2, regs.f2, regs.b2,
            regs.c2, regs.d2, regs.e2, regs.h2, regs.l2,
        ],
    );
    for word in [regs.ix, regs.iy, regs.sp, state.pc] {
        fnv_bytes(hash, word.to_le_bytes());
    }
    fnv_bytes(hash, [regs.i, regs.r]);
    fnv_bytes(
        hash,
        [
            state.flags.s as u8,
            state.flags.z as u8,
            state.flags.h as u8,
            state.flags.pv as u8,
            state.flags.n as u8,
            state.flags.c as u8,
            state.iff1 as u8,
            state.iff2 as u8,
            state.im,
            state.halted as u8,
        ],
    );
    fnv_bytes(hash, state.memory.iter().copied());
    fnv_bytes(hash, state.input_ports);
    fnv_bytes(hash, state.output_ports);
}

fn oracle_corpus(
    seed: &GateLevelCpuZ80,
    programs: impl IntoIterator<Item = Vec<u8>>,
) -> (usize, u64) {
    let mut count = 0;
    let mut hash = FNV_OFFSET;
    for program in programs {
        let mut gate = seed.clone();
        gate.load(&program, 0).unwrap();
        match gate.step() {
            Ok(trace) => {
                count += 1;
                fnv_bytes(&mut hash, [program.len() as u8]);
                fnv_bytes(&mut hash, program);
                hash_state(&mut hash, &trace.state_after);
            }
            Err(Z80Error::UnknownOpcode { .. }) => {}
            Err(error) => panic!("unexpected gate execution failure: {error}"),
        }
    }
    (count, hash)
}

#[test]
fn exact_topology_and_checked_lifecycle_are_pinned() {
    assert_eq!(FLIP_FLOP_COUNT, 528_597);
    let mut gate = GateLevelCpuZ80::new();
    let before = gate.snapshot();
    assert_eq!(before.memory.len(), 65_536);
    assert_eq!(
        gate.load(&vec![0; 65_537], 0),
        Err(Z80Error::ProgramTooLarge {
            length: 65_537,
            capacity: 65_536,
        })
    );
    assert_eq!(gate.snapshot(), before);
    assert_eq!(
        gate.set_input_port(256, 0),
        Err(Z80Error::InvalidPort { port: 256 })
    );
    assert_eq!(
        gate.get_output_port(999),
        Err(Z80Error::InvalidPort { port: 999 })
    );
    assert_eq!(gate.snapshot(), before);
}

#[test]
fn failures_are_atomic_runs_are_transactional_and_traces_own_state() {
    let mut gate = GateLevelCpuZ80::new();
    gate.set_input_port(3, 0xA5).unwrap();
    let before = gate.snapshot();

    assert_eq!(
        gate.run(&[0x3E, 0x42, 0xED, 0x00], 10),
        Err(Z80Error::UnknownOpcode {
            address: 2,
            raw: vec![0xED, 0x00],
        })
    );
    assert_eq!(gate.snapshot(), before);

    gate.load(&[0xED, 0x00], 0).unwrap();
    let loaded = gate.snapshot();
    assert_eq!(
        gate.step(),
        Err(Z80Error::UnknownOpcode {
            address: 0,
            raw: vec![0xED, 0x00],
        })
    );
    assert_eq!(gate.snapshot(), loaded);

    gate.load(&[0x3E, 0x2A, 0x76], 0xFFFF).unwrap();
    let result = gate.run_loaded_with_limit(10).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces[0].address, 0xFFFF);
    assert_eq!(result.traces[0].raw, vec![0x3E, 0x2A]);
    assert_eq!(result.traces[0].state_after.pc, 1);
    assert_eq!(result.final_state.regs.a, 0x2A);
    assert_eq!(result.final_state.regs.r, 3);

    let mut invalid = result.final_state.clone();
    invalid.memory = vec![0; 65_535].into_boxed_slice();
    let final_state = gate.snapshot();
    assert_eq!(
        gate.restore(&invalid),
        Err(Z80Error::InvalidStateMemory { length: 65_535 })
    );
    assert_eq!(gate.snapshot(), final_state);
}

#[test]
fn maskable_interrupt_modes_and_nmi_match_the_functional_contract() {
    let mut functional = Z80Simulator::new(65_536);
    functional.regs.sp = 0x1000;
    functional.pc = 0x3456;
    functional.regs.i = 0x12;
    functional.iff1 = true;
    functional.iff2 = true;
    let baseline = functional.snapshot();

    for (mode, data) in [(0, 0xCF), (1, 0), (2, 0x35)] {
        let mut state = baseline.clone();
        state.im = mode;
        state.memory[0x1234] = 0x78;
        state.memory[0x1235] = 0x56;
        let mut gate = GateLevelCpuZ80::new();
        gate.restore(&state).unwrap();
        functional.restore(&state).unwrap();
        assert_eq!(gate.interrupt(data), functional.interrupt(data));
        assert_eq!(gate.snapshot(), functional.snapshot());
    }

    let mut gate = GateLevelCpuZ80::new();
    gate.restore(&baseline).unwrap();
    functional.restore(&baseline).unwrap();
    gate.nmi();
    functional.nmi();
    assert_eq!(gate.snapshot(), functional.snapshot());
}

#[test]
fn every_defined_opcode_matches_the_python_full_state_oracle() {
    let mut seed = GateLevelCpuZ80::new();
    seed.restore(&oracle_seed()).unwrap();

    let base = (0_u8..=255)
        .filter(|opcode| !matches!(opcode, 0xCB | 0xDD | 0xED | 0xFD))
        .map(|opcode| vec![opcode, 0x20, 0x20, 0x20]);
    let cb = (0_u8..=255).map(|opcode| vec![0xCB, opcode]);
    let ed = (0_u8..=255).map(|opcode| vec![0xED, opcode, 0x20, 0x20]);
    let dd = (0_u8..=255).map(|opcode| vec![0xDD, opcode, 0x20, 0x20, 0x20]);
    let fd = (0_u8..=255).map(|opcode| vec![0xFD, opcode, 0x20, 0x20, 0x20]);
    let ddcb = (0_u8..=255).map(|opcode| vec![0xDD, 0xCB, 0x20, opcode]);
    let fdcb = (0_u8..=255).map(|opcode| vec![0xFD, 0xCB, 0x20, opcode]);

    assert_eq!(oracle_corpus(&seed, base), (252, 0xAB49_78BC_A314_5ACD));
    assert_eq!(oracle_corpus(&seed, cb), (256, 0x3B82_2E1A_CD58_E28C));
    assert_eq!(oracle_corpus(&seed, ed), (60, 0x8F0A_BA6A_E3D6_F823));
    assert_eq!(oracle_corpus(&seed, dd), (40, 0x0844_1319_8C7D_B26A));
    assert_eq!(oracle_corpus(&seed, fd), (40, 0xD2AC_C3E6_1B35_27DA));
    assert_eq!(oracle_corpus(&seed, ddcb), (256, 0xF93C_0EA9_AE1C_9ABB));
    assert_eq!(oracle_corpus(&seed, fdcb), (256, 0x1C35_5C25_2DA3_D83B));
}
