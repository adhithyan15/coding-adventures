use apple_m1_simulator::encoding as enc;
use apple_m1_simulator::{
    AppleM1Error, AppleM1Simulator, MEMORY_SIZE, REGISTER_COUNT, VECTOR_REGISTER_COUNT, XZR,
};

fn assert_reset(cpu: &AppleM1Simulator) {
    let state = cpu.get_state();
    assert_eq!(state.registers, [0; REGISTER_COUNT]);
    assert_eq!(state.vectors, [0; VECTOR_REGISTER_COUNT]);
    assert_eq!(state.memory, vec![0; MEMORY_SIZE]);
    assert_eq!((state.sp, state.pc, state.nzcv), (0, 0, 0));
    assert!(!state.halted);
    assert_eq!((state.loaded_origin, state.loaded_len), (0, 0));
}

#[test]
fn reset_load_restore_and_direct_access_are_exact() {
    let mut cpu = AppleM1Simulator::new();
    assert_reset(&cpu);
    let program = enc::program(&[enc::nop(), enc::halt()]);
    cpu.load_at_checked(&program, 0x80).unwrap();
    cpu.write_register(XZR, u64::MAX).unwrap();
    cpu.write_vector(7, u128::MAX).unwrap();
    cpu.set_stack_pointer(0x200);
    cpu.set_nzcv(0xa).unwrap();
    assert_eq!(cpu.read_register(XZR), Ok(0));
    let snapshot = cpu.get_state();
    cpu.reset();
    cpu.restore(&snapshot).unwrap();
    assert_eq!(cpu.get_state(), snapshot);
}

#[test]
fn invalid_load_restore_and_access_are_atomic() {
    let mut cpu = AppleM1Simulator::new();
    cpu.load_checked(&enc::program(&[enc::nop()])).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.load_at_checked(&[0; 4], 2),
        Err(AppleM1Error::MisalignedProgram { origin: 2 })
    );
    assert_eq!(cpu.get_state(), before);
    let mut invalid = before.clone();
    invalid.vectors[0] = 1;
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(AppleM1Error::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.read_register(REGISTER_COUNT),
        Err(AppleM1Error::InvalidRegister {
            index: REGISTER_COUNT
        })
    );
    assert_eq!(
        cpu.read_vector(VECTOR_REGISTER_COUNT),
        Err(AppleM1Error::InvalidVectorRegister {
            index: VECTOR_REGISTER_COUNT
        })
    );
}

#[test]
fn run_step_fetch_decode_and_post_halt_are_transactional() {
    let mut cpu = AppleM1Simulator::new();
    cpu.load_checked(&enc::program(&[enc::nop(), enc::halt()]))
        .unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_checked(1),
        Err(AppleM1Error::StepLimitExceeded { max_steps: 1 })
    );
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&u32::MAX.to_be_bytes()).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(AppleM1Error::UnknownInstruction {
            raw: u32::MAX,
            pc: 0
        })
    );
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&enc::program(&[enc::halt()])).unwrap();
    cpu.step_checked().unwrap();
    let halted = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(AppleM1Error::Halted));
    assert_eq!(cpu.get_state(), halted);
}

#[test]
fn extended_data_faults_leave_every_bit_unchanged() {
    let raw = enc::fp_load_store(3, true, 0, 1, 0);
    let mut cpu = AppleM1Simulator::new();
    cpu.load_checked(&raw.to_be_bytes()).unwrap();
    cpu.write_register(1, 1).unwrap();
    cpu.write_vector(0, u128::MAX).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(AppleM1Error::MisalignedData {
            address: 1,
            width: 8
        })
    );
    assert_eq!(cpu.get_state(), before);
}
