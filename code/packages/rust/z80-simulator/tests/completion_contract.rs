use z80_simulator::opcodes::{HALT, IN, OUT};
use z80_simulator::{Z80Error, Z80Simulator, Z80State};

#[test]
fn lifecycle_failures_are_typed_transactional_and_atomic() {
    let mut sim = Z80Simulator::new(1);
    assert_eq!(sim.snapshot().memory.len(), 65_536);
    sim.set_input_port(3, 0xA5).unwrap();
    let before = sim.snapshot();

    assert_eq!(
        sim.load_program(&vec![0; 65_537]),
        Err(Z80Error::ProgramTooLarge {
            length: 65_537,
            capacity: 65_536,
        })
    );
    assert_eq!(sim.snapshot(), before);
    assert_eq!(
        sim.set_input_port(256, 0),
        Err(Z80Error::InvalidPort { port: 256 })
    );
    assert_eq!(
        sim.get_output_port(1_000),
        Err(Z80Error::InvalidPort { port: 1_000 })
    );
    assert_eq!(sim.snapshot(), before);

    let mut invalid = before.clone();
    invalid.memory = vec![0; 65_535].into_boxed_slice();
    assert_eq!(
        sim.restore(&invalid),
        Err(Z80Error::InvalidStateMemory { length: 65_535 })
    );
    assert_eq!(sim.snapshot(), before);

    assert_eq!(
        sim.run(&[0x3E, 0x42, 0xED, 0x00], 10),
        Err(Z80Error::UnknownOpcode {
            address: 2,
            raw: vec![0xED, 0x00],
        })
    );
    assert_eq!(sim.snapshot(), before);

    sim.load_program(&[0xED, 0x00]).unwrap();
    let loaded = sim.snapshot();
    assert_eq!(
        sim.step(),
        Err(Z80Error::UnknownOpcode {
            address: 0,
            raw: vec![0xED, 0x00],
        })
    );
    assert_eq!(sim.snapshot(), loaded);

    sim.load_program(&[HALT]).unwrap();
    sim.step().unwrap();
    let halted = sim.snapshot();
    assert_eq!(sim.step(), Err(Z80Error::Halted));
    assert_eq!(sim.snapshot(), halted);
}

#[test]
fn fetch_wraps_and_traces_own_complete_state() {
    let mut sim = Z80Simulator::new(65_536);
    sim.load_program_at(&[0x3E, 0x2A, HALT], 0xFFFF).unwrap();
    let result = sim.run_loaded_with_limit(10).unwrap();

    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.final_state.regs.a, 0x2A);
    assert_eq!(result.traces[0].address, 0xFFFF);
    assert_eq!(result.traces[0].raw, vec![0x3E, 0x2A]);
    assert_eq!(result.traces[0].state_after.pc, 1);
    assert_eq!(result.traces[0].state_after.regs.r, 2);
    assert_eq!(result.traces[1].address, 1);
    assert_eq!(result.final_state.regs.r, 3);

    sim.mem.write_byte(0xFFFF, 0);
    assert_eq!(result.final_state.memory[0xFFFF], 0x3E);
    assert_eq!(result.traces[0].state_before.memory[0], 0x2A);
}

#[test]
fn checked_ports_and_full_state_restore_round_trip() {
    let mut sim = Z80Simulator::new(65_536);
    sim.set_input_port(3, 0xAB).unwrap();
    let result = sim.run(&[IN, 3, OUT, 4, HALT], 10).unwrap();
    assert_eq!(result.final_state.regs.a, 0xAB);
    assert_eq!(sim.get_output_port(4).unwrap(), 0xAB);

    let state = sim.snapshot();
    sim.reset();
    assert_ne!(sim.snapshot(), state);
    sim.restore(&state).unwrap();
    assert_eq!(sim.snapshot(), state);
}

fn interrupt_baseline() -> z80_simulator::Z80State {
    let mut sim = Z80Simulator::new(65_536);
    sim.regs.sp = 0x1000;
    sim.pc = 0x3456;
    sim.regs.i = 0x12;
    sim.snapshot()
}

#[test]
fn maskable_interrupt_modes_and_nmi_match_the_architectural_stack_contract() {
    let baseline = interrupt_baseline();
    let mut sim = Z80Simulator::new(65_536);
    sim.restore(&baseline).unwrap();
    assert!(!sim.interrupt(0xCF));
    assert_eq!(sim.snapshot(), baseline);

    let mut enabled = baseline.clone();
    enabled.iff1 = true;
    enabled.iff2 = true;

    sim.restore(&enabled).unwrap();
    assert!(sim.interrupt(0xCF));
    let im0 = sim.snapshot();
    assert_eq!(im0.pc, 0x0008);
    assert_eq!(im0.regs.sp, 0x0FFE);
    assert_eq!(&im0.memory[0x0FFE..=0x0FFF], &[0x56, 0x34]);
    assert!(!im0.iff1 && !im0.iff2);

    enabled.im = 1;
    sim.restore(&enabled).unwrap();
    assert!(sim.interrupt(0));
    assert_eq!(sim.pc, 0x0038);

    enabled.im = 2;
    enabled.memory[0x1234] = 0x78;
    enabled.memory[0x1235] = 0x56;
    sim.restore(&enabled).unwrap();
    assert!(sim.interrupt(0x35));
    assert_eq!(sim.pc, 0x5678);

    sim.restore(&enabled).unwrap();
    sim.nmi();
    let nmi = sim.snapshot();
    assert_eq!(nmi.pc, 0x0066);
    assert_eq!(nmi.regs.sp, 0x0FFE);
    assert_eq!(&nmi.memory[0x0FFE..=0x0FFF], &[0x56, 0x34]);
    assert!(!nmi.iff1 && nmi.iff2);
}

const FNV_OFFSET: u64 = 0xCBF2_9CE4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01B3;

fn fnv_bytes(hash: &mut u64, bytes: impl IntoIterator<Item = u8>) {
    for byte in bytes {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

fn seeded_oracle_simulator(program: &[u8]) -> Z80Simulator {
    let mut sim = Z80Simulator::new(65_536);
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
    sim.restore(&state).unwrap();
    sim.load_program(program).unwrap();
    sim
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

fn oracle_corpus(programs: impl IntoIterator<Item = Vec<u8>>) -> (usize, u64) {
    let mut count = 0;
    let mut hash = FNV_OFFSET;
    for program in programs {
        let mut sim = seeded_oracle_simulator(&program);
        match sim.step() {
            Ok(_) => {
                count += 1;
                fnv_bytes(&mut hash, [program.len() as u8]);
                fnv_bytes(&mut hash, program);
                hash_state(&mut hash, &sim.snapshot());
            }
            Err(Z80Error::UnknownOpcode { .. }) => {}
            Err(error) => panic!("unexpected oracle execution failure: {error}"),
        }
    }
    (count, hash)
}

#[test]
fn every_defined_opcode_matches_the_python_full_state_oracle() {
    let base = (0_u8..=255)
        .filter(|opcode| !matches!(opcode, 0xCB | 0xDD | 0xED | 0xFD))
        .map(|opcode| vec![opcode, 0x20, 0x20, 0x20]);
    let cb = (0_u8..=255).map(|opcode| vec![0xCB, opcode]);
    let ed = (0_u8..=255).map(|opcode| vec![0xED, opcode, 0x20, 0x20]);
    let dd = (0_u8..=255).map(|opcode| vec![0xDD, opcode, 0x20, 0x20, 0x20]);
    let fd = (0_u8..=255).map(|opcode| vec![0xFD, opcode, 0x20, 0x20, 0x20]);
    let ddcb = (0_u8..=255).map(|opcode| vec![0xDD, 0xCB, 0x20, opcode]);
    let fdcb = (0_u8..=255).map(|opcode| vec![0xFD, 0xCB, 0x20, opcode]);

    assert_eq!(oracle_corpus(base), (252, 0xAB49_78BC_A314_5ACD));
    assert_eq!(oracle_corpus(cb), (256, 0x3B82_2E1A_CD58_E28C));
    assert_eq!(oracle_corpus(ed), (60, 0x8F0A_BA6A_E3D6_F823));
    assert_eq!(oracle_corpus(dd), (40, 0x0844_1319_8C7D_B26A));
    assert_eq!(oracle_corpus(fd), (40, 0xD2AC_C3E6_1B35_27DA));
    assert_eq!(oracle_corpus(ddcb), (256, 0xF93C_0EA9_AE1C_9ABB));
    assert_eq!(oracle_corpus(fdcb), (256, 0x1C35_5C25_2DA3_D83B));
}
