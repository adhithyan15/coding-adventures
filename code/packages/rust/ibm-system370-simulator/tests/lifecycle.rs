use ibm_system370_simulator::{System370Error, System370Simulator, MEMORY_BYTES};

#[test]
fn load_restore_and_helpers_are_complete_checked_and_atomic() {
    let mut cpu = System370Simulator::new();
    cpu.load(&[0x18, 0x12], 0x20).unwrap();
    cpu.write_gpr(1, 0x1234).unwrap();
    cpu.write_fpr(4, 0x1122_3344_5566_7788).unwrap();
    cpu.write_memory(0x100, &[1, 2, 3]).unwrap();
    cpu.set_program_counter(0x20).unwrap();
    let complete = cpu.get_state();
    cpu.reset();
    cpu.restore(complete.clone()).unwrap();
    assert_eq!(cpu.get_state(), complete);
    assert_eq!(cpu.read_gpr(1).unwrap(), 0x1234);
    assert_eq!(cpu.read_fpr(4).unwrap(), 0x1122_3344_5566_7788);
    assert_eq!(cpu.read_memory(0x100, 3).unwrap(), [1, 2, 3]);

    for invalid in [
        {
            let mut state = complete.clone();
            state.memory.pop();
            state
        },
        {
            let mut state = complete.clone();
            state.condition_code = 4;
            state
        },
        {
            let mut state = complete.clone();
            state.pc = 1;
            state
        },
        {
            let mut state = complete.clone();
            state.loaded_origin = 1;
            state
        },
        {
            let mut state = complete.clone();
            state.loaded_origin = MEMORY_BYTES as u32;
            state.loaded_len = 1;
            state
        },
    ] {
        assert!(matches!(
            cpu.restore(invalid),
            Err(System370Error::InvalidState { .. })
        ));
        assert_eq!(cpu.get_state(), complete);
    }

    assert!(matches!(
        cpu.read_gpr(16),
        Err(System370Error::InvalidRegister { .. })
    ));
    assert!(matches!(
        cpu.write_fpr(1, 0),
        Err(System370Error::InvalidRegister { .. })
    ));
    assert!(matches!(
        cpu.write_memory(MEMORY_BYTES as u32, &[1]),
        Err(System370Error::MemoryOutOfRange { .. })
    ));
    assert!(cpu.set_program_counter(1).is_err());
    assert_eq!(cpu.get_state(), complete);
}

#[test]
fn load_rejects_bad_images_without_mutation_and_success_resets_everything() {
    let mut cpu = System370Simulator::new();
    cpu.write_gpr(1, 7).unwrap();
    let before = cpu.get_state();
    for error in [
        cpu.load(&[], 0).unwrap_err(),
        cpu.load(&[0, 0], 1).unwrap_err(),
        cpu.load(&[0, 0], MEMORY_BYTES as u32).unwrap_err(),
    ] {
        assert!(matches!(
            error,
            System370Error::EmptyProgram
                | System370Error::MisalignedProgram { .. }
                | System370Error::ProgramOutOfRange { .. }
        ));
        assert_eq!(cpu.get_state(), before);
    }
    cpu.load(&[0x0a, 0], 0x20).unwrap();
    let state = cpu.get_state();
    assert_eq!(state.gprs, [0; 16]);
    assert_eq!(state.pc, 0x20);
    assert_eq!(state.loaded_origin, 0x20);
    assert_eq!(state.loaded_len, 2);
}

#[test]
fn fetch_halt_unknown_svc_and_truncation_are_typed_atomic_faults() {
    let mut truncated = System370Simulator::new();
    truncated.load(&[0xbf, 0x20], 0).unwrap();
    let before = truncated.get_state();
    assert_eq!(
        truncated.step(),
        Err(System370Error::TruncatedInstruction { pc: 0, needed: 4 })
    );
    assert_eq!(truncated.get_state(), before);

    let mut unknown = System370Simulator::new();
    unknown.load(&[0, 0], 0).unwrap();
    let before = unknown.get_state();
    assert_eq!(
        unknown.step(),
        Err(System370Error::UnknownOpcode { opcode: 0, pc: 0 })
    );
    assert_eq!(unknown.get_state(), before);

    let mut svc = System370Simulator::new();
    svc.load(&[0x0a, 1], 0).unwrap();
    let before = svc.get_state();
    assert_eq!(
        svc.step(),
        Err(System370Error::UnsupportedSupervisorCall { number: 1 })
    );
    assert_eq!(svc.get_state(), before);

    let mut halted = System370Simulator::new();
    halted.load(&[0x0a, 0], 0).unwrap();
    let trace = halted.step().unwrap();
    assert_eq!(trace.mnemonic, "SVC 0");
    let before = halted.get_state();
    assert_eq!(halted.step(), Err(System370Error::Halted));
    assert_eq!(halted.get_state(), before);

    let mut outside = System370Simulator::new();
    outside.load(&[0x18, 0], 0).unwrap();
    outside.set_program_counter(2).unwrap();
    let before = outside.get_state();
    assert_eq!(
        outside.step(),
        Err(System370Error::FetchOutsideProgram { pc: 2 })
    );
    assert_eq!(outside.get_state(), before);
}

#[test]
fn bounded_run_execute_and_result_transport_are_transactional() {
    let program = [0x18, 0x12, 0x0a, 0x00];
    let mut cpu = System370Simulator::new();
    cpu.load(&program, 0).unwrap();
    cpu.write_gpr(2, 0xfeed_face).unwrap();
    let result = cpu.run(2).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.final_state.gprs[1], 0xfeed_face);
    assert_eq!(result.final_state, cpu.get_state());

    let mut bounded = System370Simulator::new();
    bounded.load(&[0x18, 0x12, 0x18, 0x12], 0).unwrap();
    let before = bounded.get_state();
    assert_eq!(
        bounded.run(1),
        Err(System370Error::MaxStepsExceeded { max_steps: 1 })
    );
    assert_eq!(bounded.get_state(), before);

    let result = System370Simulator::new()
        .execute(&[0x0a, 0], 0x20, 1)
        .unwrap();
    assert!(result.halted);
    assert_eq!(result.final_state.pc, 0x22);

    let mut mid_run_fault = System370Simulator::new();
    mid_run_fault.load(&[0x18, 0x12, 0x00, 0x00], 0).unwrap();
    mid_run_fault.write_gpr(2, 9).unwrap();
    let before = mid_run_fault.get_state();
    assert!(matches!(
        mid_run_fault.run(2),
        Err(System370Error::UnknownOpcode { pc: 2, .. })
    ));
    assert_eq!(mid_run_fault.get_state(), before);

    assert_eq!(
        System370Simulator::new().execute(&[], 0, 1),
        Err(System370Error::EmptyProgram)
    );
}
