use cortex_m7_gatelevel::{encoding::*, CortexM7GateLevel, CortexM7State};
use cortex_m7_simulator::{CortexM7Simulator, CortexM7State as FunctionalState};

const ORIGIN: u32 = 0x100;

fn functional(state: &CortexM7State) -> FunctionalState {
    FunctionalState {
        core: cortex_m7_simulator::CortexM3State {
            registers: state.core.registers,
            msp: state.core.msp,
            psp: state.core.psp,
            lr: state.core.lr,
            pc: state.core.pc,
            apsr: state.core.apsr,
            ipsr: state.core.ipsr,
            itstate: state.core.itstate,
            primask: state.core.primask,
            basepri: state.core.basepri,
            faultmask: state.core.faultmask,
            control: state.core.control,
            handler_mode: state.core.handler_mode,
            memory: state.core.memory.clone(),
            halted: state.core.halted,
            sleeping: state.core.sleeping,
            waiting_for_event: state.core.waiting_for_event,
            event_registered: state.core.event_registered,
            exclusive_address: state.core.exclusive_address,
            loaded_origin: state.core.loaded_origin,
            loaded_len: state.core.loaded_len,
        },
        ge: state.ge,
    }
}

fn compare(words: &[u16], configure: impl Fn(&mut CortexM7State)) {
    let bytes = program(words);
    let mut gate = CortexM7GateLevel::new();
    let mut oracle = CortexM7Simulator::new();
    gate.load(&bytes, ORIGIN).unwrap();
    oracle.load(&bytes, ORIGIN).unwrap();
    let mut state = gate.get_state();
    configure(&mut state);
    oracle.restore(functional(&state)).unwrap();
    gate.restore(state).unwrap();

    match (gate.step(), oracle.step()) {
        (Ok(g), Ok(o)) => {
            assert_eq!(g.pc_before, o.pc_before);
            assert_eq!(g.pc_after, o.pc_after);
            assert_eq!(g.raw, o.raw);
            assert_eq!(g.width, o.width);
            assert_eq!(g.mnemonic, o.mnemonic);
            assert_eq!(functional(&g.state_before), o.state_before);
            assert_eq!(functional(&g.state_after), o.state_after);
            assert_eq!(functional(&gate.get_state()), oracle.get_state());
        }
        (Err(g), Err(o)) => {
            assert_eq!(format!("{g:?}"), format!("{o:?}"));
            assert_eq!(functional(&gate.get_state()), oracle.get_state());
        }
        (g, o) => panic!("gate/oracle divergence: {g:?} / {o:?}"),
    }
}

#[test]
fn added_and_inherited_transitions_match_the_checked_oracle() {
    for words in [
        Vec::from(qadd(0, 1, 2)),
        Vec::from(qsub(0, 1, 2)),
        Vec::from(sadd8(0, 1, 2)),
        Vec::from(uadd8(0, 1, 2)),
        vec![movs(0, 7)],
        vec![bkpt(0)],
    ] {
        compare(&words, |state| {
            state.core.registers[1] = 0x7fff_ff80;
            state.core.registers[2] = 0x0101_01ff;
            state.ge = 0b0101;
        });
    }
}

#[test]
fn special_register_and_it_paths_match_the_checked_oracle() {
    for sysm in 0..=3 {
        compare(&msr(0, sysm), |state| {
            state.core.registers[0] = 0xa80b_0000;
            state.ge = 3;
        });
        compare(&mrs(1, sysm), |state| state.ge = 0b1011);
    }
    compare(&qadd(0, 1, 2), |state| {
        state.core.itstate = 0x08;
        state.core.apsr = 0;
        state.core.registers[0] = 99;
    });
}

#[test]
fn every_inherited_primary_family_crosses_the_reused_gate_engine() {
    let branch_link = bl(0);
    let cases = vec![
        vec![0x0000],
        vec![0x2001],
        vec![0x4008],
        vec![0x4400],
        vec![0x4800],
        vec![0x5000],
        vec![0x6000],
        vec![0x9000],
        vec![0xa000],
        vec![0xb000],
        vec![0xc001],
        vec![0xd000],
        vec![0xe000],
        Vec::from(branch_link),
        vec![0xf3bf, 0x8f5f],
        Vec::from(mrs(0, 0)),
        Vec::from(movw(0, 1)),
        vec![0xf8c1, 0x0000],
        vec![0xe850, 0x0f00],
        vec![0xe8d0, 0xf000],
    ];
    for words in cases {
        compare(&words, |state| {
            state.core.registers[0] = 1;
            state.core.registers[1] = 0x200;
            state.core.msp = 0x400;
            state.ge = 0b1010;
        });
    }
}

#[test]
fn production_manifest_has_no_functional_dependency() {
    let manifest = include_str!("../Cargo.toml");
    let production = manifest.split("[dev-dependencies]").next().unwrap();
    assert!(!production.contains("cortex-m7-simulator"));
    assert!(!production.contains("cortex-m3-simulator"));
}
