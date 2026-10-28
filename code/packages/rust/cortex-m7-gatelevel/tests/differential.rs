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

/// Regression for the #18343 APSR mask contract after composition.
///
/// The source register `0x5007_0000` and the starting image (APSR = Z|C,
/// GE = 0b1010) are chosen so every field group is distinguishable:
///
/// | mask             | NZCVQ after          | GE after |
/// |------------------|----------------------|----------|
/// | `APSR_g`     01  | unchanged `0x6000_0000` | `0b0111` |
/// | `APSR_nzcvq` 10  | `0x5000_0000` (Z clear, V set) | unchanged `0b1010` |
/// | `APSR_nzcvqg` 11 | `0x5000_0000`        | `0b0111` |
///
/// A gate that writes GE on mask `10`, or NZCVQ on mask `01`, diverges from
/// the functional Cortex-M7 oracle in the complete-state comparison.
#[test]
fn every_accepted_apsr_msr_mask_moves_only_its_field_groups() {
    use cortex_m7_gatelevel::gates::{apsr_msr_mask, RepositoryGates};

    for sysm in 0..=3 {
        for mask in [ApsrMask::Ge, ApsrMask::Nzcvq, ApsrMask::NzcvqGe] {
            let words = msr_masked(0, sysm, mask);
            let configure = |state: &mut CortexM7State| {
                state.core.registers[0] = 0x5007_0000;
                state.core.apsr = 0x6000_0000;
                state.ge = 0b1010;
            };
            compare(&words, configure);

            // Pin the expected group isolation directly as well, so the
            // contract is visible without reading the oracle.
            let mut gate = CortexM7GateLevel::new();
            gate.load(&program(&words), ORIGIN).unwrap();
            let mut state = gate.get_state();
            configure(&mut state);
            gate.restore(state).unwrap();
            let after = gate.step().unwrap().state_after;
            let (nzcvq, g) = apsr_msr_mask(&mut RepositoryGates, words[1]);
            let expected_apsr = if nzcvq == 1 { 0x5000_0000 } else { 0x6000_0000 };
            let expected_ge = if g == 1 { 0b0111 } else { 0b1010 };
            assert_eq!(after.core.apsr, expected_apsr, "{mask:?} sysm {sysm}");
            assert_eq!(after.ge, expected_ge, "{mask:?} sysm {sysm}");
        }
    }
}

/// Masked APSR `MSR` edges the companion must refuse or skip identically:
/// reserved mask `00`, GE masks on non-APSR `SYSm`, SP/LR/PC sources, a
/// failed IT condition, a passing IT condition, and a missing second
/// halfword.
#[test]
fn masked_apsr_msr_faults_and_it_paths_match_the_checked_oracle() {
    let configure = |state: &mut CortexM7State| {
        state.core.registers[0] = 0xf80f_0000;
        state.ge = 0b0101;
    };
    // Reserved mask 00 on APSR, and GE masks aimed at PRIMASK/CONTROL.
    compare(&[0xf380, 0x8000], configure);
    compare(&msr_masked(0, 16, ApsrMask::Ge), configure);
    compare(&msr_masked(0, 20, ApsrMask::NzcvqGe), configure);
    // Malformed fixed bits (9:8) with a GE mask.
    compare(&[0xf380, 0x8500], configure);
    for rn in 13..=15 {
        compare(&msr_masked(rn, 0, ApsrMask::NzcvqGe), configure);
        compare(&msr_masked(rn, 0, ApsrMask::Ge), configure);
    }
    for (itstate, apsr) in [(0x08, 0), (0x08, 0x4000_0000), (0x0c, 0x4000_0000)] {
        for mask in [ApsrMask::Ge, ApsrMask::NzcvqGe] {
            compare(&msr_masked(0, 0, mask), |state| {
                configure(state);
                state.core.itstate = itstate;
                state.core.apsr = apsr;
            });
        }
    }
    // A lone first halfword at the end of the image is a 4-byte fetch fault.
    compare(&[0xf380], configure);
}

#[test]
fn masked_apsr_msr_classification_matches_the_oracle() {
    for second in [
        0x8000, 0x8400, 0x8800, 0x8c00, 0x8401, 0x8c03, 0x8410, 0x8500,
    ] {
        let gate = cortex_m7_gatelevel::classify(0xf380, Some(second));
        let oracle = cortex_m7_simulator::classify(0xf380, Some(second));
        assert_eq!(gate.is_ok(), oracle.is_ok(), "second {second:#06x}");
        assert_eq!(
            format!("{gate:?}"),
            format!("{oracle:?}"),
            "second {second:#06x}"
        );
    }
    let gate = cortex_m7_gatelevel::classify(0xf380, None);
    let oracle = cortex_m7_simulator::classify(0xf380, None);
    assert_eq!(format!("{gate:?}"), format!("{oracle:?}"));
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
