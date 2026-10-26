use latticemico32_simulator::{Lm32Error, Lm32Simulator, MAX_RETAINED_STEPS, MEMORY_SIZE};

fn bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_be_bytes()).collect()
}

fn error_tag(error: &Lm32Error) -> &'static str {
    match error {
        Lm32Error::EmptyProgram => "EmptyProgram",
        Lm32Error::UnalignedProgram { .. } => "UnalignedProgram",
        Lm32Error::ProgramTooLarge { .. } => "ProgramTooLarge",
        Lm32Error::ProgramOutOfRange { .. } => "ProgramOutOfRange",
        Lm32Error::NoProgramLoaded => "NoProgramLoaded",
        Lm32Error::MisalignedFetch { .. } => "MisalignedFetch",
        Lm32Error::FetchOutsideProgram { .. } => "FetchOutsideProgram",
        Lm32Error::MisalignedData { .. } => "MisalignedData",
        Lm32Error::MisalignedControlFlow { .. } => "MisalignedControlFlow",
        Lm32Error::UnknownInstruction { .. } => "UnknownInstruction",
        Lm32Error::UnsupportedInstruction { .. } => "UnsupportedInstruction",
        Lm32Error::UnsupportedCsr { .. } => "UnsupportedCsr",
        Lm32Error::InvalidRegister { .. } => "InvalidRegister",
        Lm32Error::InvalidState { .. } => "InvalidState",
        Lm32Error::MemoryOutOfRange { .. } => "MemoryOutOfRange",
        Lm32Error::RetentionLimitExceeded { .. } => "RetentionLimitExceeded",
        Lm32Error::ZeroStepBudget => "ZeroStepBudget",
        Lm32Error::StepLimitExceeded { .. } => "StepLimitExceeded",
        Lm32Error::AllocationFailed => "AllocationFailed",
    }
}

fn loaded() -> Lm32Simulator {
    let mut cpu = Lm32Simulator::new();
    cpu.load(&bytes(&[0x3401_0001, 0xE000_0000]), 0x100)
        .unwrap();
    cpu
}

#[test]
fn generated_lifecycle_and_fault_matrix_is_typed_and_exactly_atomic() {
    let corpus = include_str!("vectors/lifecycle-faults.csv");
    let mut rows = 0;
    for line in corpus.lines().skip(1).filter(|line| !line.is_empty()) {
        let fields = line.split(',').collect::<Vec<_>>();
        assert_eq!(fields.len(), 5);
        let [id, action, dc_re, expected_error, rollback] = fields.as_slice() else {
            unreachable!()
        };
        assert_eq!(*rollback, "true");
        let mut cpu = loaded();
        let mut configured = cpu.snapshot().unwrap();
        configured.dc_re = *dc_re == "1";
        configured.eba = 0x1000;
        configured.deba = 0x2000;
        cpu.restore(configured).unwrap();
        let before = cpu.snapshot().unwrap();
        let mut call_entry = before.clone();

        let error = match *action {
            "load_empty" => cpu.load(&[], 0).unwrap_err(),
            "load_unaligned" => cpu.load(&[0; 3], 0).unwrap_err(),
            "load_too_large" => cpu.load(&vec![0; MEMORY_SIZE + 4], 0).unwrap_err(),
            "load_out_of_range" => cpu.load(&[0; 8], 0xfffc).unwrap_err(),
            "restore_short_memory" => {
                let mut candidate = before.clone();
                candidate.memory.pop();
                cpu.restore(candidate).unwrap_err()
            }
            "restore_bad_metadata" => {
                let mut candidate = before.clone();
                candidate.origin = None;
                cpu.restore(candidate).unwrap_err()
            }
            "restore_bad_base" => {
                let mut candidate = before.clone();
                candidate.eba = 1;
                cpu.restore(candidate).unwrap_err()
            }
            "step_no_program" => {
                cpu = Lm32Simulator::new();
                call_entry = cpu.snapshot().unwrap();
                cpu.step().unwrap_err()
            }
            "step_outside" => {
                let mut candidate = before.clone();
                candidate.pc = 0x108;
                cpu.restore(candidate).unwrap();
                call_entry = cpu.snapshot().unwrap();
                cpu.step().unwrap_err()
            }
            "step_unknown" => {
                cpu.load(&0xa800_0000_u32.to_be_bytes(), 0x100).unwrap();
                call_entry = cpu.snapshot().unwrap();
                cpu.step().unwrap_err()
            }
            "step_unsupported" => {
                cpu.load(&0x9c00_0000_u32.to_be_bytes(), 0x100).unwrap();
                call_entry = cpu.snapshot().unwrap();
                cpu.step().unwrap_err()
            }
            "step_csr_read" => {
                cpu.load(
                    &((0x24_u32 << 26) | (31 << 21) | (1 << 11)).to_be_bytes(),
                    0x100,
                )
                .unwrap();
                call_entry = cpu.snapshot().unwrap();
                cpu.step().unwrap_err()
            }
            "step_csr_write" => {
                cpu.load(
                    &((0x34_u32 << 26) | (31 << 21) | (1 << 16)).to_be_bytes(),
                    0x100,
                )
                .unwrap();
                call_entry = cpu.snapshot().unwrap();
                cpu.step().unwrap_err()
            }
            "step_data_misaligned" => {
                cpu.load(&0x2822_0000_u32.to_be_bytes(), 0x100).unwrap();
                cpu.write_register(1, 1).unwrap();
                call_entry = cpu.snapshot().unwrap();
                cpu.step().unwrap_err()
            }
            "step_control_misaligned" => {
                cpu.load(&((0x30_u32 << 26) | (1 << 21)).to_be_bytes(), 0x100)
                    .unwrap();
                cpu.write_register(1, 3).unwrap();
                call_entry = cpu.snapshot().unwrap();
                cpu.step().unwrap_err()
            }
            "helper_out_of_range" => cpu.read_memory(0x1_0000, 1).unwrap_err(),
            "run_zero" => cpu.run(0).unwrap_err(),
            "run_retention" => cpu.run(MAX_RETAINED_STEPS + 1).unwrap_err(),
            "run_exhausted" => cpu.run(1).unwrap_err(),
            "execute_empty" => cpu.execute(&[], 0x100, 1).unwrap_err().source,
            "execute_exhausted" => {
                cpu.execute(&0xe000_0000_u32.to_be_bytes(), 0x100, 1)
                    .unwrap_err()
                    .source
            }
            "alloc_snapshot" => {
                cpu.fail_allocation_after(Some(0));
                cpu.snapshot().unwrap_err()
            }
            "alloc_load" => {
                cpu.fail_allocation_after(Some(0));
                cpu.load(&0xac00_0007_u32.to_be_bytes(), 0x100).unwrap_err()
            }
            "alloc_step" => {
                cpu.fail_allocation_after(Some(0));
                cpu.step().unwrap_err()
            }
            "alloc_run_late" => {
                cpu.fail_after_candidate_steps(Some(0));
                cpu.run(2).unwrap_err()
            }
            "alloc_execute_late" => {
                cpu.fail_after_candidate_steps(Some(1));
                cpu.execute(&bytes(&[0x3401_0001, 0xe3ff_ffff]), 0x100, 3)
                    .unwrap_err()
                    .source
            }
            _ => panic!("unrecognized generated action {action}"),
        };
        assert_eq!(error_tag(&error), *expected_error, "{id}");

        assert_eq!(cpu.snapshot().unwrap(), call_entry, "{id} rollback");
        rows += 1;
    }
    assert_eq!(rows, 52);
}
