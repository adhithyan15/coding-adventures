mod common;

use common::*;
use pa_risc11_gatelevel::{
    AllocationPoint, PaRisc11Error, PaRisc11GateSimulator, MAX_RETAINED_STEPS, MEMORY_SIZE,
};
use pa_risc11_simulator::PaRisc11Simulator;

#[test]
fn complete_restore_and_malformed_states_are_atomic() {
    let (mut gate, _) = pair(&[0], 0x100);
    let valid = gate.snapshot().unwrap();
    for invalid in [
        {
            let mut s = valid.clone();
            s.gpr[0] = 1;
            s
        },
        {
            let mut s = valid.clone();
            s.pc = 2;
            s
        },
        {
            let mut s = valid.clone();
            s.memory.pop();
            s
        },
        {
            let mut s = valid.clone();
            s.nullify_next = true;
            s.in_delay_slot = false;
            s
        },
    ] {
        let before = gate.snapshot().unwrap();
        assert!(matches!(
            gate.restore(&invalid),
            Err(PaRisc11Error::InvalidState(_))
        ));
        assert_eq!(gate.snapshot().unwrap(), before);
    }
    let mut replacement = valid.clone();
    replacement.gpr[31] = u32::MAX;
    replacement.carry_borrow = [true; 8];
    gate.restore(&replacement).unwrap();
    assert_eq!(gate.snapshot().unwrap(), replacement);
}

#[test]
fn program_halt_trace_limit_and_whole_call_rollback_are_typed() {
    let mut gate = PaRisc11GateSimulator::new();
    assert_eq!(gate.load(&[], 0), Err(PaRisc11Error::EmptyProgram));
    assert!(matches!(
        gate.load(&[0; 4], 2),
        Err(PaRisc11Error::MisalignedProgram { .. })
    ));
    assert!(matches!(
        gate.load(&[0; 8], (MEMORY_SIZE - 4) as u32),
        Err(PaRisc11Error::ProgramOutOfRange { .. })
    ));
    gate.load(&words(&[immediate(0x0d, 0, 1, 7), 0]), 0)
        .unwrap();
    let before = gate.snapshot().unwrap();
    assert_eq!(
        gate.run(1),
        Err(PaRisc11Error::StepLimitExceeded { max_steps: 1 })
    );
    assert_eq!(gate.snapshot().unwrap(), before);
    assert_eq!(
        gate.run(MAX_RETAINED_STEPS + 1),
        Err(PaRisc11Error::TraceLimitExceeded {
            requested: MAX_RETAINED_STEPS + 1,
            maximum: MAX_RETAINED_STEPS
        })
    );
    let result = gate.run(2).unwrap();
    assert_eq!(result.traces.len(), 2);
    let halted = gate.snapshot().unwrap();
    assert_eq!(gate.step(), Err(PaRisc11Error::Halted));
    assert_eq!(gate.snapshot().unwrap(), halted);
}

#[test]
fn every_named_allocation_boundary_is_typed_and_atomic() {
    let production = include_str!("../src/lib.rs");
    assert!(production.contains("inject_allocation_failure_after"));
    assert!(production.contains("Some((armed, remaining - 1))"));
    for point in [
        AllocationPoint::Snapshot,
        AllocationPoint::Restore,
        AllocationPoint::LoadStorage,
        AllocationPoint::Candidate,
        AllocationPoint::StateAfter,
        AllocationPoint::TraceRetention,
        AllocationPoint::FinalState,
    ] {
        let (mut gate, oracle) = pair(&[immediate(0x0d, 0, 1, 7), 0], 0);
        let before = gate.state().clone();
        assert_eq!(oracle.state(), &before);
        gate.inject_allocation_failure(point);
        let result = match point {
            AllocationPoint::Snapshot => gate.snapshot().map(|_| ()),
            AllocationPoint::Restore => gate.restore(&before),
            AllocationPoint::LoadStorage => gate.load(&[0; 4], 0),
            AllocationPoint::Candidate | AllocationPoint::StateAfter => gate.step().map(|_| ()),
            AllocationPoint::TraceRetention | AllocationPoint::FinalState => {
                gate.run(2).map(|_| ())
            }
        };
        assert_eq!(result, Err(PaRisc11Error::AllocationFailed), "{point:?}");
        assert_eq!(gate.snapshot().unwrap(), before, "{point:?}");
        assert_eq!(gate.state(), oracle.state(), "gate-only seam {point:?}");
    }
}

#[test]
fn run_allocation_failures_after_progress_restore_exact_entry_q() {
    for (point, successful_boundaries) in [
        (AllocationPoint::Snapshot, 2),
        (AllocationPoint::Candidate, 1),
        (AllocationPoint::StateAfter, 1),
        (AllocationPoint::FinalState, 0),
    ] {
        let (mut gate, oracle) = pair(
            &[immediate(0x0d, 0, 1, 7), immediate(0x0d, 0, 2, 9), 0],
            0x100,
        );
        let entry = gate.snapshot().unwrap();
        assert_eq!(oracle.state(), &entry);
        gate.inject_allocation_failure_after(point, successful_boundaries);
        assert_eq!(
            gate.run(3),
            Err(PaRisc11Error::AllocationFailed),
            "{point:?}"
        );
        assert_eq!(gate.snapshot().unwrap(), entry, "{point:?}");
        assert_eq!(gate.state(), oracle.state(), "{point:?}");
    }
}

#[test]
fn execute_allocation_failures_after_progress_restore_preload_entry_q() {
    let image = words(&[immediate(0x0d, 0, 1, 7), immediate(0x0d, 0, 2, 9), 0]);
    for (point, successful_boundaries) in [
        (AllocationPoint::Snapshot, 3),
        (AllocationPoint::Candidate, 1),
        (AllocationPoint::StateAfter, 1),
        (AllocationPoint::FinalState, 0),
    ] {
        let (mut gate, oracle) = pair(&[0], 0x200);
        let entry = gate.snapshot().unwrap();
        assert_eq!(oracle.state(), &entry);
        gate.inject_allocation_failure_after(point, successful_boundaries);
        assert_eq!(
            gate.execute(&image, 0x100, 3),
            Err(PaRisc11Error::AllocationFailed),
            "{point:?}"
        );
        assert_eq!(gate.snapshot().unwrap(), entry, "{point:?}");
        assert_eq!(gate.state(), oracle.state(), "{point:?}");
    }
}

#[test]
fn execute_failure_after_partial_progress_restores_entry_state() {
    let mut gate = PaRisc11GateSimulator::new();
    gate.load(&[0; 4], 0x100).unwrap();
    let before = gate.snapshot().unwrap();
    let image = words(&[immediate(0x0d, 0, 1, 7), 0xfc00_0000]);
    assert!(matches!(
        gate.execute(&image, 0, 2),
        Err(PaRisc11Error::UnknownInstruction { pc: 4, .. })
    ));
    assert_eq!(gate.snapshot().unwrap(), before);
}

#[test]
fn functional_oracle_lifecycle_and_fault_results_are_exact() {
    let mut gate = PaRisc11GateSimulator::new();
    let mut oracle = PaRisc11Simulator::new();
    assert_eq!(gate.snapshot().unwrap(), oracle.snapshot().unwrap());
    for (image, origin) in [
        (Vec::new(), 0),
        (vec![0; 4], 2),
        (vec![0; 8], (MEMORY_SIZE - 4) as u32),
    ] {
        let gate_before = gate.snapshot().unwrap();
        let oracle_before = oracle.snapshot().unwrap();
        assert_eq!(gate.load(&image, origin), oracle.load(&image, origin));
        assert_eq!(gate.snapshot().unwrap(), gate_before);
        assert_eq!(oracle.snapshot().unwrap(), oracle_before);
    }

    let image = words(&[immediate(0x0d, 0, 1, -7), 0]);
    assert_eq!(gate.load(&image, 0x240), oracle.load(&image, 0x240));
    assert_eq!(gate.snapshot().unwrap(), oracle.snapshot().unwrap());
    assert_eq!(gate.step(), oracle.step());
    assert_eq!(gate.snapshot().unwrap(), oracle.snapshot().unwrap());

    let valid = gate.snapshot().unwrap();
    let mut replacement = valid.clone();
    replacement.gpr[31] = 0xfeed_face;
    replacement.carry_borrow = [true, false, true, false, true, false, true, false];
    assert_eq!(gate.restore(&replacement), oracle.restore(&replacement));
    assert_eq!(gate.snapshot().unwrap(), oracle.snapshot().unwrap());

    for invalid in [
        {
            let mut state = replacement.clone();
            state.gpr[0] = 1;
            state
        },
        {
            let mut state = replacement.clone();
            state.pc |= 2;
            state
        },
        {
            let mut state = replacement.clone();
            state.memory.pop();
            state
        },
        {
            let mut state = replacement.clone();
            state.nullify_next = true;
            state.in_delay_slot = false;
            state
        },
    ] {
        let gate_before = gate.snapshot().unwrap();
        let oracle_before = oracle.snapshot().unwrap();
        assert_eq!(gate.restore(&invalid), oracle.restore(&invalid));
        assert_eq!(gate.snapshot().unwrap(), gate_before);
        assert_eq!(oracle.snapshot().unwrap(), oracle_before);
    }

    let trace_limit = MAX_RETAINED_STEPS + 1;
    assert_eq!(gate.run(trace_limit), oracle.run(trace_limit));
    assert_eq!(gate.snapshot().unwrap(), oracle.snapshot().unwrap());

    let run_image = words(&[immediate(0x0d, 0, 2, 9), 0]);
    assert_eq!(gate.load(&run_image, 0x100), oracle.load(&run_image, 0x100));
    assert_eq!(gate.run(1), oracle.run(1));
    assert_eq!(gate.snapshot().unwrap(), oracle.snapshot().unwrap());
    assert_eq!(gate.run(2), oracle.run(2));
    assert_eq!(gate.snapshot().unwrap(), oracle.snapshot().unwrap());
    assert_eq!(gate.run(1), oracle.run(1));
    assert_eq!(gate.snapshot().unwrap(), oracle.snapshot().unwrap());
    assert_eq!(gate.step(), oracle.step());

    let faulting = words(&[immediate(0x0d, 0, 1, 7), 0xfc00_0000]);
    assert_eq!(
        gate.execute(&faulting, 0, 2),
        oracle.execute(&faulting, 0, 2)
    );
    assert_eq!(gate.snapshot().unwrap(), oracle.snapshot().unwrap());
}
