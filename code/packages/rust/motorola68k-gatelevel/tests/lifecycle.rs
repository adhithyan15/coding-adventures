use coding_adventures_motorola68k_gatelevel::{Cpu68K, M68kError, FLIP_FLOP_COUNT};

const MEMORY_SIZE: usize = 16 * 1024 * 1024;

#[test]
fn exact_topology_is_public_and_stable() {
    assert_eq!(FLIP_FLOP_COUNT, 134_218_289);
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
    cpu.rf.pc = 0x1001;
    let before = cpu.get_state();
    assert!(matches!(
        cpu.step_checked(),
        Err(M68kError::Execution(message)) if message.contains("misaligned")
    ));
    assert_eq!(cpu.get_state(), before);

    cpu.halted = true;
    let before = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(M68kError::Halted));
    assert_eq!(cpu.get_state(), before);
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
