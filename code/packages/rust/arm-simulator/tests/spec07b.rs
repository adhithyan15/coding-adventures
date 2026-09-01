use arm_simulator::functional::{
    assemble_words, encode_branch, encode_data_immediate, encode_data_register, encode_transfer,
    Armv7Simulator, Condition, DataOpcode, HALT_WORD,
};

fn run(words: &[u32]) -> arm_simulator::functional::ExecutionResult {
    Armv7Simulator::new()
        .run_checked(&assemble_words(words), words.len() + 4)
        .unwrap()
}

#[test]
fn add_and_sub_report_arm_nzcv_edges() {
    let mut cpu = Armv7Simulator::new();
    let program = assemble_words(&[
        encode_data_register(Condition::Al, DataOpcode::Add, true, 0, 2, 1),
        HALT_WORD,
    ]);
    cpu.load_checked(&program).unwrap();
    cpu.write_register_checked(0, 0x7fff_ffff).unwrap();
    cpu.write_register_checked(1, 1).unwrap();
    cpu.step_checked().unwrap();
    let state = cpu.get_state();
    assert_eq!(state.registers[2], 0x8000_0000);
    assert!(state.flags.n);
    assert!(!state.flags.z);
    assert!(!state.flags.c);
    assert!(state.flags.v);

    cpu.load_checked(&program).unwrap();
    cpu.write_register_checked(0, u32::MAX).unwrap();
    cpu.write_register_checked(1, 1).unwrap();
    cpu.step_checked().unwrap();
    let state = cpu.get_state();
    assert_eq!(state.registers[2], 0);
    assert!(!state.flags.n);
    assert!(state.flags.z);
    assert!(state.flags.c);
    assert!(!state.flags.v);

    let program = assemble_words(&[
        encode_data_register(Condition::Al, DataOpcode::Sub, true, 0, 2, 1),
        HALT_WORD,
    ]);
    cpu.load_checked(&program).unwrap();
    cpu.write_register_checked(0, 0x8000_0000).unwrap();
    cpu.write_register_checked(1, 1).unwrap();
    cpu.step_checked().unwrap();
    let state = cpu.get_state();
    assert_eq!(state.registers[2], 0x7fff_ffff);
    assert!(!state.flags.n);
    assert!(!state.flags.z);
    assert!(state.flags.c);
    assert!(state.flags.v);
}

#[test]
fn rotated_immediate_and_logical_s_bit_use_shifter_carry() {
    let result = run(&[
        encode_data_immediate(Condition::Al, DataOpcode::Mov, true, 0, 0, 1, 2),
        encode_data_immediate(Condition::Al, DataOpcode::And, true, 0, 1, 4, 1),
        encode_data_immediate(Condition::Al, DataOpcode::Orr, false, 1, 2, 0, 1),
        HALT_WORD,
    ]);
    assert_eq!(result.final_state.registers[0], 0x8000_0000);
    assert_eq!(result.final_state.registers[1], 0);
    assert_eq!(result.final_state.registers[2], 1);
    assert!(!result.final_state.flags.n);
    assert!(result.final_state.flags.z);
    assert!(!result.final_state.flags.c);
}

#[test]
fn cmp_drives_beq_bne_and_bl_uses_arm_pc_plus_eight_base() {
    let result = run(&[
        encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 0, 0, 9),
        encode_data_immediate(Condition::Al, DataOpcode::Cmp, true, 0, 0, 0, 9),
        encode_branch(Condition::Ne, false, 0),
        encode_branch(Condition::Eq, true, 0),
        encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 1, 0, 99),
        HALT_WORD,
    ]);
    assert!(!result.traces[2].condition_passed);
    assert_eq!(result.traces[3].pc_before, 12);
    assert_eq!(result.traces[3].pc_after, 20);
    assert_eq!(result.final_state.registers[14], 16);
    assert_eq!(result.final_state.registers[1], 0);
}

#[test]
fn transfer_supports_positive_and_negative_immediate_offsets() {
    let words = [
        encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 0, 0, 0x84),
        encode_data_immediate(Condition::Al, DataOpcode::Mov, false, 0, 1, 0, 0xab),
        encode_transfer(Condition::Al, false, false, 0, 1, 4),
        encode_transfer(Condition::Al, true, false, 0, 2, 4),
        encode_transfer(Condition::Al, true, true, 0, 3, 0),
        HALT_WORD,
    ];
    let result = run(&words);
    assert_eq!(result.final_state.registers[2], 0xab);
    assert_eq!(result.final_state.registers[3], 0);
    assert_eq!(&result.final_state.memory[0x80..0x84], &[0xab, 0, 0, 0]);
}

#[test]
fn failed_conditions_suppress_register_memory_and_decode_effects() {
    let words = [
        encode_data_immediate(Condition::Al, DataOpcode::Cmp, true, 0, 0, 0, 0),
        encode_data_immediate(Condition::Ne, DataOpcode::Mov, false, 0, 1, 0, 77),
        encode_transfer(Condition::Ne, true, true, 0, 2, 2),
        0x1e00_0000,
        HALT_WORD,
    ];
    let result = run(&words);
    assert_eq!(result.final_state.registers[1], 0);
    assert_eq!(result.final_state.registers[2], 0);
    assert_eq!(result.traces[1].mnemonic, "SKIP");
    assert_eq!(result.traces[2].mnemonic, "SKIP");
    assert_eq!(result.traces[3].mnemonic, "SKIP");
}
