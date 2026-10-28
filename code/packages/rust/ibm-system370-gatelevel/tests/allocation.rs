use ibm_system370_gatelevel::{
    clear_allocation_failure, fail_allocation, AllocationSite, System370Error, System370GateLevel,
    System370State,
};
use ibm_system370_simulator::System370Simulator;

struct Clear;

impl Drop for Clear {
    fn drop(&mut self) {
        clear_allocation_failure();
    }
}

fn assert_allocation_failure(
    site: AllocationSite,
    skip: usize,
    operation: impl FnOnce() -> System370Error,
) {
    let _clear = Clear;
    fail_allocation(site, skip);
    assert_eq!(operation(), System370Error::AllocationFailed);
}

fn assert_functional_parity(cpu: &System370GateLevel, entry: &System370State) {
    let mut oracle = System370Simulator::new();
    oracle.restore(entry.clone()).unwrap();
    assert_eq!(cpu.get_state(), *entry);
    assert_eq!(cpu.get_state(), oracle.get_state());
}

#[test]
fn initial_load_restore_and_step_allocations_are_typed_and_atomic() {
    assert_allocation_failure(AllocationSite::InitialStorage, 0, || {
        System370GateLevel::try_new().unwrap_err()
    });

    let mut load = System370GateLevel::new();
    load.write_gpr(1, 9).unwrap();
    let before = load.get_state();
    assert_allocation_failure(AllocationSite::LoadImage, 0, || {
        load.load(&[0x0a, 0], 0).unwrap_err()
    });
    assert_functional_parity(&load, &before);

    let mut restore = System370GateLevel::new();
    restore.load(&[0x0a, 0], 0).unwrap();
    let before = restore.get_state();
    let mut replacement = before.clone();
    replacement.gprs[1] = 7;
    assert_allocation_failure(AllocationSite::RestoreMemory, 0, || {
        restore.restore(replacement).unwrap_err()
    });
    assert_functional_parity(&restore, &before);

    for (site, skip, program) in [
        (AllocationSite::StateSnapshot, 0, vec![0x0e, 0x24]),
        (AllocationSite::RawInstruction, 0, vec![0x0e, 0x24]),
        (AllocationSite::Description, 0, vec![0x0e, 0x24]),
        (AllocationSite::Description, 1, vec![0x0e, 0x24]),
        (
            AllocationSite::MemoryWrites,
            0,
            vec![0xbe, 0x2f, 0x01, 0x00],
        ),
        (AllocationSite::MemoryWrites, 0, vec![0x0e, 0x24]),
        (AllocationSite::OracleState, 0, vec![0x0e, 0x24]),
    ] {
        let mut cpu = System370GateLevel::new();
        cpu.load(&program, 0).unwrap();
        cpu.write_gpr(2, 0x100).unwrap();
        cpu.write_gpr(3, 1).unwrap();
        cpu.write_gpr(4, 0x200).unwrap();
        cpu.write_gpr(5, 1).unwrap();
        cpu.write_memory(0x200, b"A").unwrap();
        let before = cpu.get_state();
        assert_allocation_failure(site, skip, || cpu.step().unwrap_err());
        assert_functional_parity(&cpu, &before);
    }
}

#[test]
fn run_and_execute_post_progress_failures_restore_complete_entry_state() {
    let program = [0x18, 0x12, 0x18, 0x12, 0x0a, 0];

    let mut run = System370GateLevel::new();
    run.load(&program, 0).unwrap();
    run.write_gpr(2, 0xfeed_face).unwrap();
    let run_before = run.get_state();
    assert_allocation_failure(AllocationSite::TraceRetention, 1, || {
        run.run(3).unwrap_err()
    });
    assert_functional_parity(&run, &run_before);

    let mut execute = System370GateLevel::new();
    execute.write_gpr(1, 0x1234).unwrap();
    execute.write_memory(0x100, b"keep").unwrap();
    let execute_before = execute.get_state();
    assert_allocation_failure(AllocationSite::TraceRetention, 1, || {
        execute.execute(&program, 0, 3).unwrap_err()
    });
    assert_functional_parity(&execute, &execute_before);
}

#[test]
fn reservation_and_final_state_failures_are_atomic() {
    let mut run_snapshot = System370GateLevel::new();
    run_snapshot.load(&[0x0a, 0], 0).unwrap();
    let before = run_snapshot.get_state();
    assert_allocation_failure(AllocationSite::StateSnapshot, 0, || {
        run_snapshot.run(1).unwrap_err()
    });
    assert_functional_parity(&run_snapshot, &before);

    let mut reservation = System370GateLevel::new();
    reservation.load(&[0x0a, 0], 0).unwrap();
    let before = reservation.get_state();
    assert_allocation_failure(AllocationSite::TraceReservation, 0, || {
        reservation.run(1).unwrap_err()
    });
    assert_functional_parity(&reservation, &before);

    let mut final_state = System370GateLevel::new();
    final_state.load(&[0x0a, 0], 0).unwrap();
    let before = final_state.get_state();
    assert_allocation_failure(AllocationSite::FinalState, 0, || {
        final_state.run(1).unwrap_err()
    });
    assert_functional_parity(&final_state, &before);

    let program = [0x18, 0x12, 0x0a, 0];
    for skip in [0, 1] {
        let mut execute_snapshot = System370GateLevel::new();
        execute_snapshot.write_gpr(1, 7).unwrap();
        let before = execute_snapshot.get_state();
        assert_allocation_failure(AllocationSite::StateSnapshot, skip, || {
            execute_snapshot.execute(&program, 0, 2).unwrap_err()
        });
        assert_functional_parity(&execute_snapshot, &before);
    }

    let mut execute_final = System370GateLevel::new();
    execute_final.write_gpr(1, 7).unwrap();
    let before = execute_final.get_state();
    assert_allocation_failure(AllocationSite::FinalState, 0, || {
        execute_final.execute(&program, 0, 2).unwrap_err()
    });
    assert_functional_parity(&execute_final, &before);
}
