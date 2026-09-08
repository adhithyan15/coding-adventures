use aarch64_simulator::encoding as enc;
use aarch64_simulator::{AArch64Error, AArch64Simulator, MEMORY_SIZE, REGISTER_COUNT, XZR};

fn assert_reset(cpu: &AArch64Simulator) {
    let state = cpu.get_state();
    assert_eq!(state.registers, [0; REGISTER_COUNT]);
    assert_eq!(state.sp, 0);
    assert_eq!(state.pc, 0);
    assert_eq!(state.nzcv, 0);
    assert_eq!(state.memory, vec![0; MEMORY_SIZE]);
    assert!(!state.halted);
    assert_eq!(state.loaded_origin, 0);
    assert_eq!(state.loaded_len, 0);
}

#[test]
fn reset_load_and_origin_metadata_are_exact() {
    let mut cpu = AArch64Simulator::new();
    assert_reset(&cpu);
    let program = enc::program(&[enc::move_wide(1, 2, 0, 7, 0), enc::halt()]);
    cpu.load_at_checked(&program, 0x80).unwrap();
    let state = cpu.get_state();
    assert_eq!(state.pc, 0x80);
    assert_eq!(state.loaded_origin, 0x80);
    assert_eq!(state.loaded_len, 8);
    assert_eq!(&state.memory[0x80..0x88], program);
    cpu.run_checked(2).unwrap();
    cpu.reset();
    assert_reset(&cpu);
}

#[test]
fn invalid_loads_and_restore_are_atomic() {
    let mut cpu = AArch64Simulator::new();
    cpu.load_checked(&enc::program(&[enc::nop(), enc::halt()]))
        .unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.load_at_checked(&[0; 4], 2),
        Err(AArch64Error::MisalignedProgram { origin: 2 })
    );
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.load_at_checked(&[0; 4], MEMORY_SIZE as u64),
        Err(AArch64Error::ProgramOutOfRange {
            origin: MEMORY_SIZE as u64,
            length: 4
        })
    );
    assert_eq!(cpu.get_state(), before);

    let mut invalid = before.clone();
    invalid.registers[XZR] = 1;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(AArch64Error::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
    invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(AArch64Error::InvalidState(_))
    ));
    invalid = before.clone();
    invalid.nzcv = 16;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(AArch64Error::InvalidState(_))
    ));
    invalid = before.clone();
    invalid.pc = 2;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(AArch64Error::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn direct_access_is_checked_and_xzr_is_immutable() {
    let mut cpu = AArch64Simulator::new();
    cpu.write_register(XZR, u64::MAX).unwrap();
    cpu.write_register(30, 0xfeed_face_cafe_beef).unwrap();
    assert_eq!(cpu.read_register(XZR), Ok(0));
    assert_eq!(cpu.read_register(30), Ok(0xfeed_face_cafe_beef));
    assert_eq!(
        cpu.read_register(REGISTER_COUNT),
        Err(AArch64Error::InvalidRegister {
            index: REGISTER_COUNT
        })
    );
    assert_eq!(
        cpu.write_register(REGISTER_COUNT, 1),
        Err(AArch64Error::InvalidRegister {
            index: REGISTER_COUNT
        })
    );
    cpu.set_stack_pointer(u64::MAX);
    assert_eq!(cpu.stack_pointer(), u64::MAX);
    assert_eq!(
        cpu.set_nzcv(16),
        Err(AArch64Error::InvalidState("NZCV must fit four bits".into()))
    );
    cpu.write_byte((MEMORY_SIZE - 1) as u64, 0xa5).unwrap();
    assert_eq!(cpu.read_byte((MEMORY_SIZE - 1) as u64), Ok(0xa5));
    assert_eq!(
        cpu.read_byte(MEMORY_SIZE as u64),
        Err(AArch64Error::DataOutOfRange {
            address: MEMORY_SIZE as u64,
            width: 1
        })
    );
}

#[test]
fn traces_capture_complete_instruction_boundaries() {
    let words = [
        enc::move_wide(1, 2, 0, 0x100, 0),
        enc::move_wide(1, 2, 0, 42, 1),
        enc::load_store_unsigned(3, 0, 0, 0, 0, 1),
        enc::load_store_unsigned(3, 0, 1, 0, 0, 2),
        enc::halt(),
    ];
    let mut cpu = AArch64Simulator::new();
    cpu.load_checked(&enc::program(&words)).unwrap();
    let result = cpu.run_checked(words.len()).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, words.len());
    assert_eq!(result.traces[2].mnemonic, "str");
    assert_eq!(result.traces[3].mnemonic, "ldr");
    assert_eq!(result.traces[0].state_before.pc, 0);
    assert_eq!(result.traces[0].state_after.pc, 4);
    assert_eq!(result.traces[3].state_after.registers[2], 42);
    assert_eq!(result.final_state, cpu.get_state());
}

#[test]
fn fetch_data_and_step_limit_faults_roll_back_everything() {
    let mut cpu = AArch64Simulator::new();
    cpu.load_checked(&enc::program(&[enc::nop(), enc::halt()]))
        .unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_checked(1),
        Err(AArch64Error::StepLimitExceeded { max_steps: 1 })
    );
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&enc::program(&[enc::load_store_unsigned(3, 0, 1, 0, 0, 1)]))
        .unwrap();
    cpu.write_register(0, 1).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(AArch64Error::MisalignedData {
            address: 1,
            width: 8
        })
    );
    assert_eq!(cpu.get_state(), before);

    cpu.load_at_checked(&enc::program(&[enc::nop()]), 0x40)
        .unwrap();
    cpu.step_checked().unwrap();
    let exhausted = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(AArch64Error::FetchOutsideProgram { pc: 0x44 })
    );
    assert_eq!(cpu.get_state(), exhausted);
}

#[test]
fn control_flow_alignment_and_post_halt_are_typed() {
    let mut cpu = AArch64Simulator::new();
    cpu.load_checked(&enc::program(&[enc::branch_register(0, 1)]))
        .unwrap();
    cpu.write_register(1, 2).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(AArch64Error::MisalignedFetch { pc: 2 })
    );
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&enc::program(&[enc::halt()])).unwrap();
    let trace = cpu.step_checked().unwrap();
    assert_eq!(trace.mnemonic, "halt");
    let halted = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(AArch64Error::Halted));
    assert_eq!(cpu.get_state(), halted);
    let result = cpu.run_checked(0).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 0);
    assert_eq!(result.final_state, halted);
}

#[test]
fn every_malformed_decode_family_fails_atomically() {
    let malformed = [
        u32::MAX,
        enc::branch_conditional(0, 15),
        enc::branch_register(0, 1) | 1,
        enc::move_wide(1, 1, 0, 0, 1),
        enc::move_wide(0, 2, 2, 0, 1),
        enc::logical_immediate(1, 0, 1, 0, 63, 2, 1),
        enc::logical_immediate(0, 0, 1, 0, 21, 2, 1),
        enc::load_store_unsigned(3, 1, 1, 0, 2, 1),
        enc::load_store_unsigned(3, 0, 2, 0, 2, 1),
        enc::add_sub_register(1, 0, 0, (3, 0), 3, 2, 1),
        enc::add_sub_register(0, 0, 0, (0, 32), 3, 2, 1),
        enc::data_two_source(1, 3, 7, 2, 1),
        enc::data_one_source(0, 3, 2, 1),
        enc::conditional_select(1, false, 3, 0, false, 2, 1) | (1 << 29),
        enc::conditional_select(1, false, 3, 0, false, 2, 1) | (2 << 10),
    ];
    for raw in malformed {
        let mut cpu = AArch64Simulator::new();
        cpu.load_checked(&raw.to_be_bytes()).unwrap();
        let before = cpu.get_state();
        assert_eq!(
            cpu.step_checked(),
            Err(AArch64Error::UnknownInstruction { raw, pc: 0 }),
            "raw {raw:08x}"
        );
        assert_eq!(cpu.get_state(), before, "raw {raw:08x}");
    }
}
