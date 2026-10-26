use coding_adventures_intel8051_gatelevel::{
    Cpu8051, Intel8051Error, Intel8051State, FLIP_FLOP_COUNT,
};

const CODE_SIZE: usize = 65_536;
const XDATA_SIZE: usize = 65_536;

#[test]
fn exact_persistent_topology_and_power_on_state() {
    let cpu = Cpu8051::new();
    let inventory = cpu.physical_storage_inventory();
    assert_eq!(inventory.code_dffs, cpu.code.len() * 8);
    assert_eq!(inventory.code_dffs, 524_288);
    assert_eq!(inventory.xdata_dffs, cpu.xdata.len() * 8);
    assert_eq!(inventory.xdata_dffs, 524_288);
    assert_eq!(inventory.iram_dffs, 2_048);
    assert_eq!(inventory.program_counter_dffs, 16);
    assert_eq!(inventory.halt_dffs, 1);
    assert_eq!(inventory.total(), FLIP_FLOP_COUNT);
    assert_eq!(inventory.total(), 1_050_641);
    let state = cpu.get_state();
    assert_eq!(state.code.len(), CODE_SIZE);
    assert_eq!(state.xdata.len(), XDATA_SIZE);
    assert_eq!(state.iram[0x81], 0x07);
    for port in [0x80, 0x90, 0xA0, 0xB0] {
        assert_eq!(state.iram[port], 0xFF);
    }
}

#[test]
fn checked_restore_and_load_fail_atomically() {
    let mut cpu = Cpu8051::new();
    cpu.load_checked(&[0x74, 0x2A, 0xA5]).unwrap();
    let before = cpu.get_state();

    let mut invalid = before.clone();
    invalid.code.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(Intel8051Error::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    assert!(matches!(
        cpu.load_at_checked(&[1, 2], 0xFFFF),
        Err(Intel8051Error::ProgramOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn checked_load_clears_both_harvard_data_spaces() {
    let mut cpu = Cpu8051::new();
    let mut state = cpu.get_state();
    state.code[200] = 0xAA;
    state.xdata[300] = 0xBB;
    cpu.restore(&state).unwrap();
    cpu.load_at_checked(&[0xA5], 0x1234).unwrap();
    let loaded = cpu.get_state();
    assert_eq!(loaded.pc, 0x1234);
    assert_eq!(loaded.code[0x1234], 0xA5);
    assert_eq!(loaded.code[200], 0);
    assert_eq!(loaded.xdata[300], 0);
}

#[test]
fn checked_step_rejects_halt_and_truncation_without_mutation() {
    let mut cpu = Cpu8051::new();
    cpu.load_checked(&[0x74]).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(Intel8051Error::TruncatedInstruction { pc: 0, length: 2 })
    );
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&[0xA5]).unwrap();
    cpu.step_checked().unwrap();
    let halted = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(Intel8051Error::Halted));
    assert_eq!(cpu.get_state(), halted);
}

#[test]
fn halt_dff_q_overrides_both_compatibility_observer_divergences() {
    let mut running = Cpu8051::new();
    running.load_checked(&[0x00, 0xA5]).unwrap();
    running.halted = true;
    assert!(!running.halted());
    assert!(!running.get_state().halted);
    let first = running.step_checked().unwrap();
    assert_eq!(first.raw, vec![0x00]);
    assert_eq!(first.pc_after, 1);
    assert!(!running.halted);

    let result = running.run_loaded_checked(2).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 1);
    assert!(running.halted());
    assert!(running.halted);

    running.halted = false;
    let before = running.get_state();
    assert!(before.halted);
    assert_eq!(running.step_checked(), Err(Intel8051Error::Halted));
    assert_eq!(running.get_state(), before);
    assert!(running.halted);
}

#[test]
fn restore_clocks_halt_q_and_fault_rollback_preserves_it() {
    let mut cpu = Cpu8051::new();
    let mut halted = cpu.get_state();
    halted.halted = true;
    cpu.restore(&halted).unwrap();
    assert!(cpu.halted());
    assert!(cpu.halted);

    let mut running = halted;
    running.halted = false;
    running.code.fill(0);
    running.code[0] = 0xE6; // MOV A,@R0
    running.iram[0] = 0x80; // invalid base-8051 indirect address
    running.pc = 0;
    running.loaded_origin = 0;
    running.loaded_len = 1;
    cpu.restore(&running).unwrap();
    cpu.halted = true;
    let before = cpu.get_state();
    assert!(matches!(
        cpu.run_loaded_checked(1),
        Err(Intel8051Error::Execution(_))
    ));
    assert_eq!(cpu.get_state(), before);
    assert!(!cpu.halted());
    assert!(!cpu.halted);
}

#[test]
fn checked_run_returns_complete_traces_and_final_state() {
    let mut cpu = Cpu8051::new();
    let result = cpu.run_checked(&[0x74, 42, 0xA5], 10).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.traces[0].mnemonic, "MOV A,#imm");
    assert_eq!(result.final_state, cpu.get_state());
    assert_eq!(result.final_state.iram[0xE0], 42);
}

#[test]
fn restore_accepts_a_complete_owned_state() {
    let mut cpu = Cpu8051::new();
    let state = Intel8051State {
        pc: 0x4000,
        iram: [0x5A; 256],
        xdata: vec![0xA5; XDATA_SIZE],
        code: vec![0; CODE_SIZE],
        halted: false,
        loaded_origin: 0,
        loaded_len: CODE_SIZE,
    };
    cpu.restore(&state).unwrap();
    assert_eq!(cpu.get_state(), state);
}
