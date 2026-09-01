use armv7a_gatelevel::Armv7AGateLevel;
use armv7a_simulator::{encoding, Armv7AError, MEMORY_SIZE, PC};

#[test]
fn reset_load_and_origin_are_exact() {
    let mut cpu = Armv7AGateLevel::new();
    cpu.write_byte(17, 0xaa);
    cpu.write_register(4, 99).unwrap();
    let bytes = encoding::program(&[encoding::mov_imm8(0, 7), encoding::halt()]);
    cpu.load_at_checked(&bytes, 0x100).unwrap();
    let state = cpu.get_state();
    assert_eq!(state.pc, 0x100);
    assert_eq!(state.registers[PC], 0x100);
    assert_eq!(state.registers[13], 0xfff8);
    assert_eq!(state.loaded_origin, 0x100);
    assert_eq!(state.loaded_len, bytes.len());
    assert_eq!(&state.memory[0x100..0x100 + bytes.len()], bytes);
    assert_eq!(state.memory[17], 0);
    assert_eq!(state.registers[4], 0);

    let result = cpu.run_checked(3).unwrap();
    assert_eq!(result.final_state.registers[0], 7);
}

#[test]
fn failed_loads_preserve_the_complete_prior_state() {
    let mut cpu = Armv7AGateLevel::new();
    cpu.load_checked(&encoding::program(&[encoding::halt()]))
        .unwrap();
    cpu.write_register(2, 0x1234).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.load_at_checked(&[1, 2], 1),
        Err(Armv7AError::MisalignedProgram { origin: 1 })
    );
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.load_at_checked(&[1, 2, 3], 0xfffe),
        Err(Armv7AError::ProgramOutOfRange {
            origin: 0xfffe,
            length: 3
        })
    );
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn restore_validates_every_invariant_before_commit() {
    let mut cpu = Armv7AGateLevel::new();
    cpu.load_checked(&encoding::program(&[encoding::halt()]))
        .unwrap();
    let before = cpu.get_state();

    let mut invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(Armv7AError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    let mut invalid = before.clone();
    invalid.pc = 2;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(Armv7AError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    let mut invalid = before.clone();
    invalid.cpsr &= !(1 << 5);
    assert!(matches!(
        cpu.restore(&invalid),
        Err(Armv7AError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    let mut valid = before.clone();
    valid.registers[3] = 0xfeed_beef;
    cpu.restore(&valid).unwrap();
    assert_eq!(cpu.get_state(), valid);
}

#[test]
fn direct_register_and_wrapping_memory_access_are_checked_as_documented() {
    let mut cpu = Armv7AGateLevel::new();
    assert_eq!(
        cpu.read_register(16),
        Err(Armv7AError::InvalidRegister { index: 16 })
    );
    assert_eq!(
        cpu.write_register(42, 1),
        Err(Armv7AError::InvalidRegister { index: 42 })
    );
    cpu.write_register(PC, 0x101).unwrap();
    assert_eq!(cpu.get_state().pc, 0x100);
    assert_eq!(cpu.read_register(PC), Ok(0x100));
    cpu.write_byte(0x1_ffff, 0xa5);
    assert_eq!(cpu.read_byte(0xffff), 0xa5);
}

#[test]
fn fetch_and_decode_faults_are_step_atomic() {
    let cases = [
        (
            vec![0x01, 0xde],
            Armv7AError::UnknownInstruction16 { raw: 0xde01, pc: 0 },
        ),
        (
            vec![0x00, 0xf8],
            Armv7AError::FetchOutsideProgram { pc: 0, width: 4 },
        ),
    ];
    for (bytes, expected) in cases {
        let mut cpu = Armv7AGateLevel::new();
        cpu.load_checked(&bytes).unwrap();
        let before = cpu.get_state();
        assert_eq!(cpu.step_checked(), Err(expected));
        assert_eq!(cpu.get_state(), before);
    }

    let mut cpu = Armv7AGateLevel::new();
    cpu.load_checked(&[0x00, 0xe8, 0x00, 0x00]).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(Armv7AError::UnknownInstruction32 {
            first: 0xe800,
            second: 0,
            pc: 0
        })
    );
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn trace_is_complete_and_runs_are_transactional() {
    let mut cpu = Armv7AGateLevel::new();
    let bytes = encoding::program(&[encoding::mov_imm8(0, 9), encoding::halt()]);
    cpu.load_checked(&bytes).unwrap();
    let trace = cpu.step_checked().unwrap();
    assert_eq!(trace.pc_before, 0);
    assert_eq!(trace.pc_after, 2);
    assert_eq!(trace.raw, 0x2009);
    assert_eq!(trace.width, 2);
    assert_eq!(trace.mnemonic, "movs");
    assert_eq!(trace.state_before.registers[0], 0);
    assert_eq!(trace.state_after.registers[0], 9);

    cpu.step_checked().unwrap();
    let halted = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(Armv7AError::Halted));
    assert_eq!(cpu.get_state(), halted);

    let mut cpu = Armv7AGateLevel::new();
    let bytes = encoding::program(&[encoding::mov_imm8(0, 7), encoding::branch(-1)]);
    cpu.load_checked(&bytes).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_checked(5),
        Err(Armv7AError::StepLimitExceeded { max_steps: 5 })
    );
    assert_eq!(cpu.get_state(), before);

    let mut cpu = Armv7AGateLevel::new();
    let mut bytes = encoding::program(&[encoding::mov_imm8(0, 7)]);
    bytes.extend_from_slice(&0xde01_u16.to_le_bytes());
    cpu.load_checked(&bytes).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.run_checked(3),
        Err(Armv7AError::UnknownInstruction16 { .. })
    ));
    assert_eq!(cpu.get_state(), before);
    assert_eq!(before.memory.len(), MEMORY_SIZE);
}
