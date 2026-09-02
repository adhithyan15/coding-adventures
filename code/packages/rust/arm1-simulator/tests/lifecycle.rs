use arm1_simulator::{
    encode_halt, encode_mov_imm, Arm1Error, ARM1, COND_AL, FLAG_F, FLAG_I, MEMORY_SIZE, MODE_FIQ,
    MODE_IRQ, MODE_USR,
};

fn words_bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

#[test]
fn architectural_constructor_owns_the_full_address_space() {
    let cpu = ARM1::architectural();
    assert_eq!(cpu.memory().len(), MEMORY_SIZE);
    assert_eq!(cpu.get_state().regs.len(), 27);
}

#[test]
fn checked_load_and_restore_fail_atomically() {
    let mut cpu = ARM1::new(1024);
    cpu.load_checked(&words_bytes(&[encode_halt()])).unwrap();
    let before = cpu.get_state();

    assert!(matches!(
        cpu.load_at_checked(&[0; 8], 1020),
        Err(Arm1Error::ProgramOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    assert_eq!(
        cpu.load_at_checked(&[0; 4], 1),
        Err(Arm1Error::MisalignedProgram { origin: 1 })
    );
    assert_eq!(cpu.get_state(), before);

    let mut invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(Arm1Error::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn checked_register_and_memory_accesses_are_typed() {
    let mut cpu = ARM1::new(64);
    assert_eq!(
        cpu.read_register_checked(16),
        Err(Arm1Error::InvalidRegister { index: 16 })
    );
    assert_eq!(
        cpu.write_register_checked(99, 1),
        Err(Arm1Error::InvalidRegister { index: 99 })
    );
    assert!(matches!(
        cpu.read_word_checked(64),
        Err(Arm1Error::MemoryOutOfRange { .. })
    ));
    assert!(matches!(
        cpu.write_byte_checked(64, 1),
        Err(Arm1Error::MemoryOutOfRange { .. })
    ));
}

#[test]
fn checked_load_clears_stale_memory_and_tracks_exact_fetch_range() {
    let mut cpu = ARM1::new(1024);
    cpu.write_byte(900, 0xAA);
    cpu.load_at_checked(&words_bytes(&[encode_halt()]), 128)
        .unwrap();
    assert_eq!(cpu.read_byte(900), 0);
    assert_eq!(cpu.get_state().loaded_origin, 128);
    assert_eq!(cpu.get_state().loaded_len, 4);
    cpu.step_checked().unwrap();
    assert!(cpu.halted());
}

#[test]
fn checked_step_rejects_halt_truncation_and_data_bounds_atomically() {
    let mut cpu = ARM1::new(64);
    cpu.load_checked(&[0, 0, 0]).unwrap();
    let truncated = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(Arm1Error::TruncatedInstruction { pc: 0 })
    );
    assert_eq!(cpu.get_state(), truncated);

    // LDR R0, [R1] with R1 beyond this bounded test machine.
    let ldr = (COND_AL << 28) | 0x0591_0000;
    cpu.load_words_checked(&[ldr], 0).unwrap();
    cpu.write_register(1, 0x1000);
    let before = cpu.get_state();
    assert!(matches!(
        cpu.step_checked(),
        Err(Arm1Error::MemoryOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    cpu.load_words_checked(&[encode_halt()], 0).unwrap();
    cpu.step_checked().unwrap();
    let halted = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(Arm1Error::Halted));
    assert_eq!(cpu.get_state(), halted);
}

#[test]
fn checked_run_returns_complete_physical_state_and_traces() {
    let code = words_bytes(&[encode_mov_imm(COND_AL, 0, 42), encode_halt()]);
    let mut cpu = ARM1::new(1024);
    let result = cpu.run_checked(&code, 10).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.traces[0].state_before.regs.len(), 27);
    assert_eq!(result.final_state, cpu.get_state());
    assert_eq!(cpu.read_register(0), 42);
}

#[test]
fn run_rolls_back_the_complete_machine_on_late_failure() {
    let mut cpu = ARM1::new(64);
    let ldr = (COND_AL << 28) | 0x0591_0000;
    cpu.load_words_checked(&[encode_mov_imm(COND_AL, 0, 7), ldr], 0)
        .unwrap();
    cpu.write_register(1, 0x1000);
    let before = cpu.get_state();
    assert!(matches!(
        cpu.run_loaded_checked(10),
        Err(Arm1Error::MemoryOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn irq_and_fiq_entry_use_banked_links_and_respect_masks() {
    let mut cpu = ARM1::new(1024);
    cpu.write_register(15, MODE_USR | 0x100);
    assert!(cpu.raise_irq());
    assert_eq!(cpu.mode(), MODE_IRQ);
    assert_eq!(cpu.pc(), 0x18);
    assert_eq!(cpu.get_state().regs[24], MODE_USR | 0x100);
    assert_ne!(cpu.r15_raw() & FLAG_I, 0);
    assert!(!cpu.raise_irq());

    cpu.write_register(15, MODE_USR | 0x200);
    assert!(cpu.raise_fiq());
    assert_eq!(cpu.mode(), MODE_FIQ);
    assert_eq!(cpu.pc(), 0x1C);
    assert_eq!(cpu.get_state().regs[22], MODE_USR | 0x200);
    assert_ne!(cpu.r15_raw() & (FLAG_I | FLAG_F), 0);
    assert!(!cpu.raise_fiq());
}

#[test]
fn block_transfer_force_user_targets_the_unbanked_registers() {
    let mut cpu = ARM1::new(1024);
    // STMIA R0, {R8}^ ; HALT
    let stm_user = (COND_AL << 28) | 0x08C0_0100;
    cpu.load_words_checked(&[stm_user, encode_halt()], 0)
        .unwrap();
    cpu.write_register(15, MODE_FIQ);
    cpu.write_register(0, 0x100);
    let mut state = cpu.get_state();
    state.regs[8] = 0x1111_1111;
    state.regs[16] = 0x2222_2222;
    cpu.restore(&state).unwrap();
    cpu.step_checked().unwrap();
    assert_eq!(cpu.read_word(0x100), 0x1111_1111);
}
