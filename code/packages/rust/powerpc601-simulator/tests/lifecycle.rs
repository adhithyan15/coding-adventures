use powerpc601_simulator::encoding::{assemble, d_form, halt, i_form, xo_form};
use powerpc601_simulator::{PowerPc601Simulator, PowerPcError, MEMORY_SIZE};

#[test]
fn exact_state_and_restore_are_validated_atomically() {
    let mut cpu = PowerPc601Simulator::new();
    assert_eq!(cpu.get_state().memory.len(), MEMORY_SIZE);
    let before = cpu.get_state();

    let mut invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(PowerPcError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    invalid = before.clone();
    invalid.cia = 1;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(PowerPcError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    invalid = before.clone();
    invalid.loaded_origin = (MEMORY_SIZE - 2) as u32;
    invalid.loaded_len = 4;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(PowerPcError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn checked_loading_and_big_endian_direct_access_are_typed() {
    let mut cpu = PowerPc601Simulator::new();
    cpu.write_byte_checked(60, 0xaa).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.load_at_checked(&[0; 8], (MEMORY_SIZE - 4) as u32),
        Err(PowerPcError::ProgramOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.load_at_checked(&[0; 4], 1),
        Err(PowerPcError::MisalignedProgram { origin: 1 })
    );

    cpu.load_at_checked(&assemble(&[halt()]), 0x100).unwrap();
    assert_eq!(cpu.get_state().cia, 0x100);
    assert_eq!(cpu.get_state().loaded_origin, 0x100);
    assert_eq!(cpu.get_state().loaded_len, 4);
    assert_eq!(cpu.read_byte_checked(60).unwrap(), 0);
    assert_eq!(
        cpu.read_register_checked(32),
        Err(PowerPcError::InvalidRegister { index: 32 })
    );
    cpu.write_word_checked(8, 0x0102_0304).unwrap();
    assert_eq!(cpu.read_word_checked(8).unwrap(), 0x0102_0304);
    assert_eq!(cpu.read_byte_checked(8).unwrap(), 1);
    cpu.write_half_checked(12, 0x4567).unwrap();
    assert_eq!(cpu.read_half_checked(12).unwrap(), 0x4567);
    assert!(matches!(
        cpu.read_word_checked(2),
        Err(PowerPcError::MisalignedAccess { .. })
    ));
    assert!(matches!(
        cpu.read_byte_checked(MEMORY_SIZE as u32),
        Err(PowerPcError::MemoryOutOfRange { .. })
    ));
}

#[test]
fn faults_and_bounded_runs_are_transactional() {
    let mut cpu = PowerPc601Simulator::new();
    cpu.load_checked(&[0, 0, 0]).unwrap();
    assert_eq!(
        cpu.step_checked(),
        Err(PowerPcError::TruncatedInstruction { cia: 0 })
    );

    for raw in [
        0x0400_0000,
        d_form(32, 2, 0, 1),
        xo_form(31, 3, 1, 2, false, 491, false),
    ] {
        cpu.load_checked(&assemble(&[raw])).unwrap();
        if raw >> 26 == 31 {
            cpu.write_register_checked(1, 7).unwrap();
        }
        let before = cpu.get_state();
        assert!(cpu.step_checked().is_err());
        assert_eq!(cpu.get_state(), before);
    }

    cpu.load_checked(&assemble(&[i_form(18, 0, false, false)]))
        .unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_loaded_checked(3),
        Err(PowerPcError::StepLimitExceeded { max_steps: 3 })
    );
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn full_traces_cover_arithmetic_memory_and_halt() {
    let words = [
        d_form(14, 1, 0, 0x100),
        d_form(14, 2, 0, 42),
        d_form(36, 2, 1, 0),
        d_form(32, 3, 1, 0),
        halt(),
    ];
    let mut cpu = PowerPc601Simulator::new();
    let result = cpu.run_checked(&assemble(&words), 8).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 5);
    assert_eq!(result.final_state.gpr[3], 42);
    assert_eq!(&result.final_state.memory[0x100..0x104], &[0, 0, 0, 42]);
    assert_eq!(result.traces[2].mnemonic, "STW");
    assert_eq!(result.traces[2].state_before.memory[0x103], 0);
    assert_eq!(result.traces[2].state_after.memory[0x103], 42);
}
