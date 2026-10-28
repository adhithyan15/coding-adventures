use coding_adventures_powerpc601_gatelevel::{PowerPc601GateSimulator, PowerPcError, PowerPcState};
use powerpc601_simulator::encoding::{assemble, b_form, d_form, halt, x_form, xo_form};

fn seeded(raw: u32) -> PowerPc601GateSimulator {
    let mut cpu = PowerPc601GateSimulator::new();
    cpu.load_checked(&assemble(&[raw, halt()])).unwrap();
    let mut state = cpu.get_state();
    state.gpr[1] = 0x7fff_ffff;
    state.gpr[2] = 1;
    state.gpr[3] = 3;
    state.xer = 0xd5a5_5a5a;
    state.cr = 0x1234_5678;
    state.memory[0x100] = 0xa5;
    cpu.restore(&state).unwrap();
    cpu
}

fn step_with(raw: u32, configure: impl FnOnce(&mut PowerPcState)) -> (PowerPcState, PowerPcState) {
    let mut cpu = seeded(raw);
    let mut state = cpu.get_state();
    configure(&mut state);
    cpu.restore(&state).unwrap();
    let trace = cpu.step_checked().unwrap();
    (trace.state_before, trace.state_after)
}

#[test]
fn gate_xo_profile_rejects_all_excluded_modifier_pairs_before_commit() {
    for xo in [10, 40, 104, 138, 235, 266, 459, 491] {
        for oe in [false, true] {
            for rc in [false, true] {
                let raw = xo_form(31, 4, 1, 2, oe, xo, rc);
                let mut step_cpu = seeded(raw);
                let before = step_cpu.get_state();
                if !oe && !rc {
                    step_cpu.step_checked().unwrap();
                    continue;
                }
                assert_eq!(
                    step_cpu.step_checked(),
                    Err(PowerPcError::UnknownInstruction { raw, cia: 0 }),
                    "XO={xo} OE={oe} Rc={rc}"
                );
                assert_eq!(step_cpu.get_state(), before);

                let mut run_cpu = seeded(raw);
                let before = run_cpu.get_state();
                assert_eq!(
                    run_cpu.run_loaded_checked(4),
                    Err(PowerPcError::UnknownInstruction { raw, cia: 0 }),
                    "bounded XO={xo} OE={oe} Rc={rc}"
                );
                assert_eq!(run_cpu.get_state(), before);
            }
        }
    }
}

#[test]
fn ca_instruction_witnesses_cover_borrow_carry_input_and_shift_edges() {
    let opaque = 0xd5a5_5a5a & !(1 << 29);
    for (ra, immediate, expected_ca) in [(0, 0, 1), (1, 0, 0), (u32::MAX, -1, 1)] {
        let raw = d_form(8, 4, 1, immediate);
        let (before, after) = step_with(raw, |state| {
            state.gpr[1] = ra;
            state.xer = opaque | (1 << 29);
        });
        assert_eq!((after.xer >> 29) & 1, expected_ca);
        assert_eq!(after.xer & !(1 << 29), before.xer & !(1 << 29));
    }

    for (left, right, expected_ca) in [(0, 0, 0), (u32::MAX, 1, 1), (0x7fff_ffff, 1, 0)] {
        let raw = xo_form(31, 4, 1, 2, false, 10, false);
        let (before, after) = step_with(raw, |state| {
            state.gpr[1] = left;
            state.gpr[2] = right;
            state.xer = opaque;
        });
        assert_eq!((after.xer >> 29) & 1, expected_ca);
        assert_eq!(after.xer & !(1 << 29), before.xer & !(1 << 29));
    }

    for (left, right, incoming_ca, expected_ca) in [
        (u32::MAX, 0, 0, 0),
        (u32::MAX, 0, 1, 1),
        (u32::MAX, 1, 0, 1),
        (0, 0, 1, 0),
    ] {
        let raw = xo_form(31, 4, 1, 2, false, 138, false);
        let (before, after) = step_with(raw, |state| {
            state.gpr[1] = left;
            state.gpr[2] = right;
            state.xer = opaque | (incoming_ca << 29);
        });
        assert_eq!((after.xer >> 29) & 1, expected_ca);
        assert_eq!(after.xer & !(1 << 29), before.xer & !(1 << 29));
    }

    for (source, count, expected_ca) in [
        (0x8000_0000, 0, 0),
        (0x8000_0000, 1, 0),
        (0x8000_0001, 1, 1),
        (0x8000_0000, 31, 0),
        (0x8000_0000, 32, 1),
        (0x8000_0000, 33, 1),
        (0x8000_0000, 63, 1),
        (0x7fff_ffff, 63, 0),
    ] {
        let raw = x_form(31, 1, 4, 2, 792, false);
        let (before, after) = step_with(raw, |state| {
            state.gpr[1] = source;
            state.gpr[2] = count;
            state.xer = opaque | (1 << 29);
        });
        assert_eq!((after.xer >> 29) & 1, expected_ca, "sraw count={count}");
        assert_eq!(after.xer & !(1 << 29), before.xer & !(1 << 29));
    }

    for (source, count, expected_ca) in [
        (0x8000_0000, 0, 0),
        (0x8000_0001, 1, 1),
        (0x8000_0000, 31, 0),
        (u32::MAX, 31, 1),
    ] {
        let raw = x_form(31, 1, 4, count, 824, false);
        let (before, after) = step_with(raw, |state| {
            state.gpr[1] = source;
            state.xer = opaque | (1 << 29);
        });
        assert_eq!((after.xer >> 29) & 1, expected_ca, "srawi count={count}");
        assert_eq!(after.xer & !(1 << 29), before.xer & !(1 << 29));
    }
}

#[test]
fn compare_and_record_status_feed_the_authoritative_branch_network() {
    let compare = x_form(31, 0, 1, 2, 0, false);
    let branch_equal = b_form(16, 18, 2, 8, false, false);
    let program = assemble(&[compare, branch_equal, d_form(14, 3, 0, 99), halt()]);
    let mut cpu = PowerPc601GateSimulator::new();
    cpu.load_checked(&program).unwrap();
    let mut state = cpu.get_state();
    state.gpr[1] = 0x8000_0000;
    state.gpr[2] = 0x8000_0000;
    state.xer = 0xd5a5_5a5a | (1 << 31);
    cpu.restore(&state).unwrap();

    let compare_trace = cpu.step_checked().unwrap();
    assert_eq!(compare_trace.state_after.cr >> 28, 0x3);
    let branch_trace = cpu.step_checked().unwrap();
    assert_eq!(branch_trace.cia_after, 12);
    assert_eq!(branch_trace.state_after.cr, compare_trace.state_after.cr);

    let record = x_form(31, 1, 4, 2, 28, true);
    let (_, after) = step_with(record, |state| {
        state.gpr[1] = 0x8000_0000;
        state.gpr[2] = u32::MAX;
        state.xer = 0x4000_0000;
        state.cr = 0x0123_4567;
    });
    assert_eq!(after.cr >> 28, 0x8);
    assert_eq!(after.cr & 0x0fff_ffff, 0x0123_4567);
    assert_eq!(after.xer, 0x4000_0000);
}

#[test]
fn snapshots_traces_and_bounded_rollback_observe_exact_xer_and_cr_dff_state() {
    let raw = xo_form(31, 4, 1, 2, true, 10, false);
    let mut cpu = seeded(raw);
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_loaded_checked(2),
        Err(PowerPcError::UnknownInstruction { raw, cia: 0 })
    );
    assert_eq!(cpu.get_state(), before);

    let raw = x_form(31, 1, 4, 2, 792, false);
    let mut cpu = seeded(raw);
    let mut state = cpu.get_state();
    state.gpr[1] = 0x8000_0000;
    state.gpr[2] = 32;
    state.xer = 0x9555_aaaa;
    state.cr = 0xfedc_ba98;
    cpu.restore(&state).unwrap();
    let trace = cpu.step_checked().unwrap();
    assert_eq!(trace.state_before.xer, state.xer);
    assert_eq!(trace.state_before.cr, state.cr);
    assert_eq!(trace.state_after.xer, cpu.get_state().xer);
    assert_eq!(trace.state_after.cr, cpu.get_state().cr);
    assert_eq!(trace.state_after.xer & !(1 << 29), state.xer & !(1 << 29));
}
