use cdp1802_gatelevel::{
    Cdp1802Error, Cdp1802Simulator, DmaTransaction, ExecutionError, MAX_RETAINED_STEPS, MEMORY_SIZE,
};

fn loaded(program: &[u8]) -> Cdp1802Simulator {
    let mut cpu = Cdp1802Simulator::new();
    cpu.load(program, 0x0100).unwrap();
    cpu
}

#[test]
fn reset_load_snapshot_and_restore_cover_complete_state() {
    let mut cpu = Cdp1802Simulator::new();
    let reset = cpu.get_state();
    assert_eq!(reset.memory.len(), MEMORY_SIZE);
    assert_eq!(reset.registers, [0; 16]);
    assert_eq!((reset.p, reset.x, reset.d, reset.t), (0, 0, 0, 0));
    assert_eq!((reset.df, reset.q, reset.idle), (false, false, false));
    assert!(reset.ie);
    assert!(reset.first_instruction_pending);
    assert_eq!((reset.origin, reset.length), (None, 0));

    cpu.load(&[0xf8, 0x5a, 0x00], 0xffff).unwrap_err();
    assert_eq!(cpu.get_state(), reset);
    cpu.load(&[0xf8, 0x5a, 0], 0x200).unwrap();
    let loaded = cpu.try_get_state().unwrap();
    assert_eq!(loaded.registers[0], 0x200);
    assert_eq!((loaded.origin, loaded.length), (Some(0x200), 3));
    assert_eq!(&loaded.memory[0x200..0x203], &[0xf8, 0x5a, 0]);

    cpu.step().unwrap();
    cpu.restore(loaded.clone()).unwrap();
    assert_eq!(cpu.get_state(), loaded);
    cpu.reset();
    assert_eq!(cpu.get_state(), reset);
}

#[test]
fn malformed_restore_is_refused_before_commit() {
    let mut cpu = loaded(&[0]);
    let before = cpu.get_state();

    let mut bad = before.clone();
    bad.p = 16;
    assert!(matches!(
        cpu.restore(bad),
        Err(Cdp1802Error::InvalidState { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    let mut bad = before.clone();
    bad.memory.pop();
    assert!(matches!(
        cpu.restore(bad),
        Err(Cdp1802Error::InvalidState { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    let mut bad = before.clone();
    bad.origin = None;
    assert!(matches!(
        cpu.restore(bad),
        Err(Cdp1802Error::InvalidState { .. })
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn fetch_decode_and_truncation_faults_are_atomic_and_preserve_inhibit() {
    let mut cpu = loaded(&[0x68]);
    let before = cpu.get_state();
    assert_eq!(
        cpu.step(),
        Err(Cdp1802Error::UnknownInstruction { raw: 0x68 })
    );
    assert_eq!(cpu.get_state(), before);
    assert_eq!(cpu.interrupt(), Err(Cdp1802Error::InterruptInhibited));

    let mut cpu = loaded(&[0xf8]);
    let before = cpu.get_state();
    assert_eq!(
        cpu.step(),
        Err(Cdp1802Error::TruncatedInstruction {
            pc: 0x0100,
            needed: 2
        })
    );
    assert_eq!(cpu.get_state(), before);

    let mut state = before.clone();
    state.registers[0] = 0x0200;
    cpu.restore(state).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step(),
        Err(Cdp1802Error::FetchOutsideProgram { pc: 0x0200 })
    );
    assert_eq!(cpu.get_state(), before);

    let mut blank = Cdp1802Simulator::new();
    assert_eq!(blank.step(), Err(Cdp1802Error::NoProgramLoaded));
}

#[test]
fn interrupt_inhibit_has_precedence_and_dma_does_not_clear_it() {
    let mut cpu = loaded(&[0]);
    let mut overlap = cpu.get_state();
    overlap.ie = false;
    overlap.first_instruction_pending = true;
    cpu.restore(overlap.clone()).unwrap();
    assert_eq!(cpu.interrupt(), Err(Cdp1802Error::InterruptInhibited));
    assert_eq!(cpu.get_state(), overlap);

    cpu.dma_in(0xaa).unwrap();
    cpu.dma_out().unwrap();
    assert!(cpu.get_state().first_instruction_pending);
    assert_eq!(cpu.interrupt(), Err(Cdp1802Error::InterruptInhibited));

    let mut masked = cpu.get_state();
    masked.first_instruction_pending = false;
    cpu.restore(masked.clone()).unwrap();
    assert_eq!(cpu.interrupt(), Err(Cdp1802Error::InterruptMasked));
    assert_eq!(cpu.get_state(), masked);
}

#[test]
fn idl_completes_first_instruction_and_dma_or_interrupt_resumes() {
    let mut cpu = loaded(&[0]);
    let trace = cpu.step().unwrap();
    assert!(trace.idle_after);
    assert!(!cpu.get_state().first_instruction_pending);
    let idle = cpu.get_state();
    assert_eq!(cpu.step(), Err(Cdp1802Error::Idling));
    assert_eq!(cpu.get_state(), idle);

    cpu.dma_in(0x55).unwrap();
    assert!(!cpu.get_state().idle);

    let mut cpu = loaded(&[0]);
    cpu.step().unwrap();
    let acknowledged = cpu.interrupt().unwrap();
    assert_eq!(acknowledged.saved_selectors, 0);
    let state = cpu.get_state();
    assert_eq!(
        (state.t, state.x, state.p, state.ie, state.idle),
        (0, 2, 1, false, false)
    );

    let mut cpu = loaded(&[0xf8, 0]);
    let mut state = cpu.get_state();
    state.x = 5;
    cpu.restore(state).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.interrupt().unwrap().saved_selectors, 0x50);
    assert_eq!(cpu.get_state().t, 0x50);
}

#[test]
fn dma_uses_r0_wrap_and_reports_exact_transactions() {
    let mut cpu = loaded(&[0]);
    let mut state = cpu.get_state();
    state.registers[0] = 0xffff;
    state.idle = true;
    cpu.restore(state).unwrap();
    assert_eq!(
        cpu.dma_in(0xa5).unwrap(),
        DmaTransaction::In {
            address: 0xffff,
            value: 0xa5
        }
    );
    assert_eq!(cpu.get_state().registers[0], 0);
    cpu.write_memory(0, 0x5a);
    assert_eq!(
        cpu.dma_out().unwrap(),
        DmaTransaction::Out {
            address: 0,
            value: 0x5a
        }
    );
    assert_eq!(cpu.get_state().registers[0], 1);
}

#[test]
fn run_stops_only_on_idl_and_rolls_back_every_failure() {
    let mut cpu = loaded(&[0xf8, 0x2a, 0]);
    let result = cpu.run(2).unwrap();
    assert!(result.stopped);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces[0].mnemonic, "LDI");
    assert_eq!(result.traces[1].mnemonic, "IDL");
    assert_eq!(result.final_state, cpu.get_state());

    let mut cpu = loaded(&[0xf8, 0x2a, 0]);
    let before = cpu.get_state();
    assert_eq!(
        cpu.run(1),
        Err(Cdp1802Error::StepLimitExceeded { limit: 1 })
    );
    assert_eq!(cpu.get_state(), before);

    let mut cpu = loaded(&[0x68]);
    let before = cpu.get_state();
    assert_eq!(
        cpu.run(1),
        Err(Cdp1802Error::UnknownInstruction { raw: 0x68 })
    );
    assert_eq!(cpu.get_state(), before);
    assert!(matches!(
        cpu.run(MAX_RETAINED_STEPS + 1),
        Err(Cdp1802Error::RetentionLimitExceeded { .. })
    ));
}

#[test]
fn execute_wraps_source_and_restores_entry_state() {
    let mut cpu = loaded(&[0]);
    let original = cpu.get_state();
    let result = cpu.execute(&[0xf8, 0x66, 0], 0x0200, 2).unwrap();
    assert_eq!(result.final_state.d, 0x66);
    assert_eq!(cpu.get_state(), result.final_state);

    cpu.restore(original.clone()).unwrap();
    assert_eq!(
        cpu.execute(&[0x68], 0x300, 1),
        Err(ExecutionError {
            source: Cdp1802Error::UnknownInstruction { raw: 0x68 }
        })
    );
    assert_eq!(cpu.get_state(), original);

    assert_eq!(
        cpu.execute(&[], 0, 1),
        Err(ExecutionError {
            source: Cdp1802Error::EmptyProgram
        })
    );
    assert_eq!(cpu.get_state(), original);
}

#[test]
fn allocation_failures_are_typed_and_exactly_atomic() {
    let mut witnessed_success = false;
    for checkpoint in 0..8 {
        let mut cpu = loaded(&[0xf8, 1]);
        let before = cpu.get_state();
        cpu.fail_allocation_after(Some(checkpoint));
        match cpu.step() {
            Err(Cdp1802Error::AllocationFailed) => assert_eq!(cpu.get_state(), before),
            Ok(_) => {
                witnessed_success = true;
                break;
            }
            other => panic!("unexpected step result at checkpoint {checkpoint}: {other:?}"),
        }
    }
    assert!(witnessed_success);

    let mut witnessed_success = false;
    for checkpoint in 0..20 {
        let mut cpu = loaded(&[0]);
        let before = cpu.get_state();
        cpu.fail_allocation_after(Some(checkpoint));
        match cpu.run(1) {
            Err(Cdp1802Error::AllocationFailed) => assert_eq!(cpu.get_state(), before),
            Ok(_) => {
                witnessed_success = true;
                break;
            }
            other => panic!("unexpected run result at checkpoint {checkpoint}: {other:?}"),
        }
    }
    assert!(witnessed_success);

    let mut witnessed_success = false;
    for checkpoint in 0..24 {
        let mut cpu = loaded(&[0]);
        let before = cpu.get_state();
        cpu.fail_allocation_after(Some(checkpoint));
        match cpu.execute(&[0], 0x0200, 1) {
            Err(ExecutionError {
                source: Cdp1802Error::AllocationFailed,
            }) => assert_eq!(cpu.get_state(), before),
            Ok(_) => {
                witnessed_success = true;
                break;
            }
            other => panic!("unexpected execute result at checkpoint {checkpoint}: {other:?}"),
        }
    }
    assert!(witnessed_success);
}

#[test]
fn load_input_errors_and_checked_snapshot_are_atomic() {
    let mut cpu = loaded(&[0]);
    let before = cpu.get_state();
    assert_eq!(cpu.load(&[], 0), Err(Cdp1802Error::EmptyProgram));
    assert!(matches!(
        cpu.load(&vec![0; MEMORY_SIZE + 1], 0),
        Err(Cdp1802Error::ProgramTooLarge { .. })
    ));
    assert!(matches!(
        cpu.load(&[0, 0], 0xffff),
        Err(Cdp1802Error::ProgramRangeOverflow { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    cpu.fail_allocation_after(Some(0));
    assert_eq!(cpu.load(&[0], 0x0200), Err(Cdp1802Error::AllocationFailed));
    assert_eq!(cpu.get_state(), before);

    cpu.fail_allocation_after(Some(0));
    assert_eq!(cpu.try_get_state(), Err(Cdp1802Error::AllocationFailed));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn trace_contains_all_required_before_after_fields() {
    let mut cpu = loaded(&[0xf8, 0x42]);
    let trace = cpu.step().unwrap();
    assert_eq!((trace.pc_before, trace.pc_after), (0x0100, 0x0102));
    assert_eq!(
        (trace.p_before, trace.p_after, trace.x_before, trace.x_after),
        (0, 0, 0, 0)
    );
    assert_eq!(
        (trace.raw, trace.width, trace.mnemonic),
        ([0xf8, 0x42, 0], 2, "LDI")
    );
    assert_eq!((trace.d_before, trace.d_after), (0, 0x42));
    assert_eq!((trace.df_before, trace.df_after), (false, false));
    assert_eq!((trace.q_before, trace.q_after), (false, false));
    assert_eq!((trace.ie_before, trace.ie_after), (true, true));
    assert_eq!((trace.idle_before, trace.idle_after), (false, false));
    assert_eq!(trace.state_after, cpu.get_state());
}
