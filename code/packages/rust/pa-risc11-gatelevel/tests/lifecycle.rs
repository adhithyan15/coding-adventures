mod common;

use common::*;
use pa_risc11_gatelevel::{
    AllocationPoint, PaRisc11Error, PaRisc11GateSimulator, MAX_RETAINED_STEPS, MEMORY_SIZE,
};

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
    for point in [
        AllocationPoint::Snapshot,
        AllocationPoint::Restore,
        AllocationPoint::LoadStorage,
        AllocationPoint::Candidate,
        AllocationPoint::StateAfter,
        AllocationPoint::TraceRetention,
        AllocationPoint::FinalState,
    ] {
        let mut gate = PaRisc11GateSimulator::new();
        gate.load(&words(&[immediate(0x0d, 0, 1, 7), 0]), 0)
            .unwrap();
        let before = gate.state().clone();
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
