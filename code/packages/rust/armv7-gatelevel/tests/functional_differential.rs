use arm_simulator::functional::{
    assemble_words, encode_branch, encode_data_immediate, encode_data_register, encode_transfer,
    ArmState, Armv7Simulator, Condition, DataOpcode, Flags, HALT_WORD,
};
use armv7_gatelevel::Armv7GateLevel;

fn compare_one(raw: u32, seed: usize, prepare: impl Fn(&mut ArmState)) {
    let program = assemble_words(&[raw]);
    let mut functional = Armv7Simulator::new();
    let mut gate = Armv7GateLevel::new();
    functional.load_checked(&program).unwrap();
    gate.load_checked(&program).unwrap();
    let mut state = functional.get_state();
    for index in 0..15 {
        state.registers[index] = ((seed as u32 + 1).wrapping_mul(0x7f4a_7c15))
            ^ (index as u32).wrapping_mul(0x6c8e_9cf5);
    }
    state.flags = [
        Flags::default(),
        Flags {
            n: false,
            z: true,
            c: true,
            v: false,
        },
        Flags {
            n: true,
            z: false,
            c: false,
            v: true,
        },
        Flags {
            n: true,
            z: false,
            c: true,
            v: true,
        },
    ][seed];
    prepare(&mut state);
    functional.restore(&state).unwrap();
    gate.restore(&state).unwrap();
    assert_eq!(
        gate.step_checked(),
        functional.step_checked(),
        "raw={raw:08x} seed={seed}"
    );
}

#[test]
fn every_condition_and_data_processing_path_matches() {
    for condition in [
        Condition::Eq,
        Condition::Ne,
        Condition::Cs,
        Condition::Cc,
        Condition::Mi,
        Condition::Pl,
        Condition::Vs,
        Condition::Vc,
        Condition::Hi,
        Condition::Ls,
        Condition::Ge,
        Condition::Lt,
        Condition::Gt,
        Condition::Le,
        Condition::Al,
        Condition::Nv,
    ] {
        for seed in 0..4 {
            compare_one(
                encode_data_immediate(condition, DataOpcode::Mov, true, 0, 2, seed as u32, 0x81),
                seed,
                |_| {},
            );
        }
    }
    for opcode in [
        DataOpcode::And,
        DataOpcode::Sub,
        DataOpcode::Add,
        DataOpcode::Cmp,
        DataOpcode::Orr,
        DataOpcode::Mov,
    ] {
        for seed in 0..4 {
            compare_one(
                encode_data_register(Condition::Al, opcode, true, 1, 2, 3),
                seed,
                |_| {},
            );
            compare_one(
                encode_data_immediate(Condition::Al, opcode, true, 1, 2, seed as u32 * 3, 0xa5),
                seed,
                |_| {},
            );
        }
    }
}

#[test]
fn memory_branch_link_and_halt_paths_match() {
    for seed in 0..4 {
        for raw in [
            encode_transfer(Condition::Al, false, true, 1, 2, 4),
            encode_transfer(Condition::Al, true, true, 1, 2, 4),
            encode_transfer(Condition::Al, false, false, 1, 2, 4),
            encode_transfer(Condition::Al, true, false, 1, 2, 4),
        ] {
            compare_one(raw, seed, |state| {
                state.registers[1] = 0x104;
                state.registers[2] = 0x1234_5678;
                state.memory[0x100..0x10c]
                    .copy_from_slice(&[0x78, 0x56, 0x34, 0x12, 0xef, 0xcd, 0xab, 0x90, 1, 2, 3, 4]);
            });
        }
        for raw in [
            encode_branch(Condition::Al, false, 0),
            encode_branch(Condition::Al, true, 0),
            HALT_WORD,
        ] {
            compare_one(raw, seed, |_| {});
        }
    }
}
