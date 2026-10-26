use coding_adventures_motorola68k_gatelevel::{Cpu68K, M68kError, FLIP_FLOP_COUNT};

const MEMORY_SIZE: usize = 16 * 1024 * 1024;

#[test]
fn exact_topology_is_public_and_stable() {
    let cpu = Cpu68K::new();
    let inventory = cpu.physical_storage_inventory();
    assert_eq!(inventory.memory_dffs, cpu.mem.len() * 8);
    assert_eq!(inventory.memory_dffs, 134_217_728);
    assert_eq!(inventory.data_register_dffs, 8 * 32);
    assert_eq!(inventory.address_register_dffs, 8 * 32);
    assert_eq!(inventory.program_counter_dffs, 32);
    assert_eq!(inventory.status_register_dffs, 16);
    assert_eq!(inventory.halt_dffs, 1);
    assert_eq!(inventory.total(), FLIP_FLOP_COUNT);
    assert_eq!(inventory.total(), 134_218_289);

    let state = cpu.get_state();
    assert_eq!(state.d, [0; 8]);
    assert_eq!(state.a, [0, 0, 0, 0, 0, 0, 0, 0xF000]);
    assert_eq!(state.pc, 0x1000);
    assert_eq!(state.sr, 0x2700);
    assert!(!state.halted);
    assert_eq!(state.memory.len(), MEMORY_SIZE);
}

#[test]
fn checked_load_and_restore_failures_are_atomic() {
    let mut cpu = Cpu68K::new();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.load_at_checked(&[1, 2], 0x00ff_ffff),
        Err(M68kError::ProgramTooLarge { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    let mut invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(M68kError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn checked_step_failures_are_typed_and_atomic() {
    let mut cpu = Cpu68K::new();
    let mut misaligned = cpu.get_state();
    misaligned.pc = 0x1001;
    cpu.restore(&misaligned).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.step_checked(),
        Err(M68kError::Execution(message)) if message.contains("misaligned")
    ));
    assert_eq!(cpu.get_state(), before);

    let mut halted = cpu.get_state();
    halted.halted = true;
    cpu.restore(&halted).unwrap();
    let before = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(M68kError::Halted));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn dff_q_drives_snapshot_step_restore_run_results_and_fault_rollback() {
    let mut cpu = Cpu68K::new();
    cpu.load_checked(&[0x70, 0x2A, 0x4E, 0x4F]).unwrap();

    cpu.rf.d = [0xAAAA_AAAA; 8];
    cpu.rf.a = [0xBBBB_BBBB; 8];
    cpu.rf.pc = 0xCCCC_CCCC;
    cpu.rf.sr = 0xDDDD;
    cpu.halted = true;
    let before = cpu.get_state();
    assert_eq!(before.d, [0; 8]);
    assert_eq!(before.a[7], 0xF000);
    assert_eq!(before.pc, 0x1000);
    assert_eq!(before.sr, 0x2700);
    assert!(!before.halted);

    let trace = cpu.step_checked().unwrap();
    assert_eq!(trace.pc_before, 0x1000);
    assert_eq!(trace.pc_after, 0x1002);
    assert_eq!(trace.state_after.d[0], 42);
    assert_eq!(cpu.rf.d, trace.state_after.d);
    assert_eq!(cpu.rf.a, trace.state_after.a);
    assert_eq!(cpu.rf.pc, trace.state_after.pc);
    assert_eq!(cpu.rf.sr, trace.state_after.sr);
    assert_eq!(cpu.halted, trace.state_after.halted);

    cpu.rf.d = [1; 8];
    cpu.rf.a = [2; 8];
    cpu.rf.pc = 3;
    cpu.rf.sr = 4;
    cpu.halted = true;
    let result = cpu.run_loaded_checked(2).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 1);
    assert_eq!(result.pc, 0x1004);
    assert_eq!(result.final_state, cpu.get_state());
    assert_eq!(cpu.rf.d, result.final_state.d);
    assert_eq!(cpu.rf.a, result.final_state.a);
    assert_eq!(cpu.rf.pc, result.final_state.pc);
    assert_eq!(cpu.rf.sr, result.final_state.sr);
    assert_eq!(cpu.halted, result.final_state.halted);

    let mut restored = result.final_state.clone();
    restored.d = [0x11; 8];
    restored.a = [0x22; 8];
    restored.pc = 0x2000;
    restored.sr = 0x2305;
    restored.halted = false;
    cpu.restore(&restored).unwrap();
    assert_eq!(cpu.get_state(), restored);
    assert_eq!(cpu.rf.d, restored.d);
    assert_eq!(cpu.rf.a, restored.a);
    assert_eq!(cpu.rf.pc, restored.pc);
    assert_eq!(cpu.rf.sr, restored.sr);
    assert_eq!(cpu.halted, restored.halted);

    let mut faulting = restored;
    faulting.pc = 0x2001;
    cpu.restore(&faulting).unwrap();
    cpu.rf.d = [0xAAAA_AAAA; 8];
    cpu.rf.a = [0xBBBB_BBBB; 8];
    cpu.rf.pc = 0xCCCC_CCCC;
    cpu.rf.sr = 0xDDDD;
    cpu.halted = true;
    let authoritative_before = cpu.get_state();
    assert!(matches!(
        cpu.run_loaded_checked(1),
        Err(M68kError::Execution(message)) if message.contains("misaligned")
    ));
    assert_eq!(cpu.get_state(), authoritative_before);
    assert_eq!(cpu.rf.d, authoritative_before.d);
    assert_eq!(cpu.rf.a, authoritative_before.a);
    assert_eq!(cpu.rf.pc, authoritative_before.pc);
    assert_eq!(cpu.rf.sr, authoritative_before.sr);
    assert_eq!(cpu.halted, authoritative_before.halted);
}

#[test]
fn halt_dff_q_overrides_both_observer_divergences() {
    let mut running = Cpu68K::new();
    running.load_checked(&[0x4E, 0x71, 0x4E, 0x4F]).unwrap();
    running.halted = true;
    assert!(!running.halted());
    let trace = running.step_checked().unwrap();
    assert_eq!(trace.pc_after, 0x1002);
    assert!(!running.halted);

    running.run_loaded_checked(1).unwrap();
    assert!(running.halted());
    running.halted = false;
    let before = running.get_state();
    assert_eq!(running.step_checked(), Err(M68kError::Halted));
    assert_eq!(running.get_state(), before);
    assert!(running.halted);
}

#[test]
fn checked_run_returns_complete_traces_and_state() {
    let mut cpu = Cpu68K::new();
    let result = cpu
        .run_checked(&[0x70, 0x2a, 0x4e, 0x4f], 8)
        .expect("MOVEQ and TRAP #15 agree with the functional oracle");
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.traces[0].mnemonic, "MOVEQ");
    assert_eq!(result.final_state.d[0], 42);
    assert_eq!(result.final_state.memory.len(), MEMORY_SIZE);
}
