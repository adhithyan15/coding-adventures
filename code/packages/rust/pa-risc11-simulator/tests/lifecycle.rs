mod common;

use common::{immediate, installed, memory, set_state, words};
use pa_risc11_simulator::{PaRisc11Error, PaRisc11Simulator, MAX_RETAINED_STEPS, MEMORY_SIZE};

#[test]
fn program_validation_and_exact_big_endian_trace() {
    let mut cpu = PaRisc11Simulator::new();
    assert_eq!(cpu.load(&[], 0), Err(PaRisc11Error::EmptyProgram));
    assert!(matches!(
        cpu.load(&[0, 0, 0, 0], 2),
        Err(PaRisc11Error::MisalignedProgram { .. })
    ));
    assert!(matches!(
        cpu.load(&[0; 8], (MEMORY_SIZE - 4) as u32),
        Err(PaRisc11Error::ProgramOutOfRange { .. })
    ));

    let word = immediate(0x0d, 0, 1, -1);
    cpu.load(&word.to_be_bytes(), 0x100).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.raw_bytes, word.to_be_bytes());
    assert_eq!(trace.raw_word, word);
    assert_eq!(trace.mnemonic, "LDO");
    assert_eq!((trace.pc_before, trace.next_pc_before), (0x100, 0x104));
    assert_eq!((trace.pc_after, trace.next_pc_after), (0x104, 0x108));
    assert_eq!(trace.state_after.gpr[1], u32::MAX);
}

#[test]
fn restore_validates_complete_state_before_commit() {
    let mut cpu = installed(&[0], 0);
    for mutate in [
        |state: &mut pa_risc11_simulator::PaRisc11State| state.gpr[0] = 1,
        |state: &mut pa_risc11_simulator::PaRisc11State| state.pc = 2,
        |state: &mut pa_risc11_simulator::PaRisc11State| state.memory.pop().map_or((), |_| ()),
    ] {
        let mut invalid = cpu.snapshot().unwrap();
        mutate(&mut invalid);
        let before = cpu.snapshot().unwrap();
        assert!(matches!(
            cpu.restore(&invalid),
            Err(PaRisc11Error::InvalidState(_))
        ));
        assert_eq!(cpu.state(), &before);
    }

    let valid = cpu.snapshot().unwrap();
    set_state(&mut cpu, |state| state.gpr[2] = 7);
    cpu.restore(&valid).unwrap();
    assert_eq!(cpu.state(), &valid);

    let mut invalid_nullify = valid.clone();
    invalid_nullify.nullify_next = true;
    assert!(matches!(
        cpu.restore(&invalid_nullify),
        Err(PaRisc11Error::InvalidState(
            "nullification outside delay slot"
        ))
    ));
}

#[test]
fn halt_fetch_and_post_halt_are_typed_and_atomic() {
    let mut cpu = installed(&[0], 0x100);
    let trace = cpu.step().unwrap();
    assert_eq!(trace.mnemonic, "BREAK");
    let halted = cpu.snapshot().unwrap();
    assert_eq!(cpu.step(), Err(PaRisc11Error::Halted));
    assert_eq!(cpu.state(), &halted);

    let mut exhausted = installed(&[immediate(0x0d, 0, 1, 1)], 0);
    exhausted.step().unwrap();
    let before = exhausted.snapshot().unwrap();
    assert_eq!(
        exhausted.step(),
        Err(PaRisc11Error::FetchOutsideProgram { pc: 4 })
    );
    assert_eq!(exhausted.state(), &before);
}

#[test]
fn run_and_execute_roll_back_whole_transaction() {
    let mut cpu = installed(&[immediate(0x0d, 0, 1, 7), 0], 0);
    let result = cpu.run(2).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.final_state.gpr[1], 7);

    let mut limited = installed(&[immediate(0x0d, 0, 1, 7), immediate(0x0d, 0, 2, 8)], 0);
    let before = limited.snapshot().unwrap();
    assert_eq!(
        limited.run(1),
        Err(PaRisc11Error::StepLimitExceeded { max_steps: 1 })
    );
    assert_eq!(limited.state(), &before);

    let mut faulting = installed(&[immediate(0x0d, 0, 1, 7), 0xfc00_0000], 0);
    let before_fault = faulting.snapshot().unwrap();
    assert!(matches!(
        faulting.run(2),
        Err(PaRisc11Error::UnknownInstruction { pc: 4, .. })
    ));
    assert_eq!(faulting.state(), &before_fault);

    let original = cpu.snapshot().unwrap();
    assert!(matches!(
        cpu.execute(&words(&[memory(0x12, 1, 2, 0)]), 0, 1),
        Err(PaRisc11Error::StepLimitExceeded { .. })
    ));
    assert_eq!(cpu.state(), &original);

    assert_eq!(cpu.execute(&[], 0, 1), Err(PaRisc11Error::EmptyProgram));
    assert_eq!(cpu.state(), &original);
}

#[test]
fn retention_is_rejected_before_mutation() {
    let mut cpu = installed(&[0], 0);
    let before = cpu.snapshot().unwrap();
    assert_eq!(
        cpu.run(MAX_RETAINED_STEPS + 1),
        Err(PaRisc11Error::TraceLimitExceeded {
            requested: MAX_RETAINED_STEPS + 1,
            maximum: MAX_RETAINED_STEPS
        })
    );
    assert_eq!(cpu.state(), &before);
}

#[test]
fn display_text_is_stable_and_nonempty() {
    let errors = [
        PaRisc11Error::EmptyProgram,
        PaRisc11Error::MisalignedFetch { pc: 2 },
        PaRisc11Error::DataOutOfRange {
            address: u32::MAX,
            width: 4,
        },
        PaRisc11Error::AllocationFailed,
        PaRisc11Error::Halted,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}
