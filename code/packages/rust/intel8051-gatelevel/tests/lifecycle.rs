use coding_adventures_intel8051_gatelevel::{
    Cpu8051, Intel8051Error, Intel8051State, FLIP_FLOP_COUNT,
};

const CODE_SIZE: usize = 65_536;
const XDATA_SIZE: usize = 65_536;

#[test]
fn exact_persistent_topology_and_power_on_state() {
    assert_eq!(FLIP_FLOP_COUNT, 1_050_641);
    let cpu = Cpu8051::new();
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
