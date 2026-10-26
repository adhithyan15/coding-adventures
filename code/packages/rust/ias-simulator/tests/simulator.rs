use ias_simulator::{
    encode_instruction, pack_instructions, Half, IasError, IasSimulator, Opcode, HALT, MAX_STEPS,
    MEMORY_WORDS, WORD_MASK,
};

fn instruction(opcode: Opcode, address: u16) -> u32 {
    encode_instruction(opcode, address).unwrap()
}

fn word(left: u32, right: u32) -> u64 {
    pack_instructions(left, right).unwrap()
}

fn bytes(words: &[u64]) -> Vec<u8> {
    words
        .iter()
        .flat_map(|value| {
            let encoded = value.to_be_bytes();
            encoded[3..].to_vec()
        })
        .collect()
}

fn signed(value: i64) -> u64 {
    (value as u64) & WORD_MASK
}

#[test]
fn construction_transport_memory_and_reset_are_strict() {
    let mut machine = IasSimulator::new();
    let initial = machine.snapshot();
    assert_eq!(machine.state(), &initial);
    assert_eq!(initial.memory.len(), MEMORY_WORDS);
    assert!(initial.memory.iter().all(|word| *word == 0));
    assert_eq!(initial.control_counter, 0);
    assert_eq!(initial.buffered_right, None);
    assert!(!initial.halted);

    let image = [word(instruction(Opcode::Load, 40), HALT)];
    assert_eq!(machine.load(&bytes(&image), 32).unwrap(), 1);
    assert_eq!(machine.snapshot().control_counter, 32);
    assert_eq!(machine.read_memory(32).unwrap(), image[0]);

    let loaded = machine.snapshot();
    assert_eq!(
        machine.load(&[0, 1, 2, 3], 0),
        Err(IasError::TransportAlignment { bytes: 4 })
    );
    assert_eq!(machine.snapshot(), loaded);
    assert_eq!(machine.load(&[], 0), Err(IasError::EmptyProgram));
    assert_eq!(machine.load_words(&[], 0), Err(IasError::EmptyProgram));
    assert_eq!(
        machine.load_words(&image, MEMORY_WORDS as u16),
        Err(IasError::OriginOutOfRange {
            origin: MEMORY_WORDS as u16
        })
    );
    assert_eq!(
        machine.load_words(&[0, 0], (MEMORY_WORDS - 1) as u16),
        Err(IasError::ProgramOutOfRange {
            origin: (MEMORY_WORDS - 1) as u16,
            words: 2
        })
    );
    assert_eq!(
        machine.load_words(&[WORD_MASK + 1], 0),
        Err(IasError::InvalidWord {
            value: WORD_MASK + 1
        })
    );
    assert_eq!(machine.snapshot(), loaded);

    machine.write_memory(10, 0x12345).unwrap();
    assert_eq!(machine.read_memory(10).unwrap(), 0x12345);
    let before_direct_fault = machine.snapshot();
    assert_eq!(
        machine.write_memory(MEMORY_WORDS, 1),
        Err(IasError::MemoryOutOfRange {
            address: MEMORY_WORDS
        })
    );
    assert_eq!(
        machine.write_memory(10, WORD_MASK + 1),
        Err(IasError::InvalidWord {
            value: WORD_MASK + 1
        })
    );
    assert_eq!(machine.snapshot(), before_direct_fault);
    assert_eq!(
        machine.read_memory(MEMORY_WORDS),
        Err(IasError::MemoryOutOfRange {
            address: MEMORY_WORDS
        })
    );

    machine.reset();
    assert_eq!(machine.snapshot(), initial);
}

#[test]
fn loads_adds_subtracts_absolute_values_and_stores() {
    let mut machine = IasSimulator::new();
    let program = [
        word(
            instruction(Opcode::Load, 100),
            instruction(Opcode::Add, 101),
        ),
        word(
            instruction(Opcode::Subtract, 102),
            instruction(Opcode::Store, 103),
        ),
        word(
            instruction(Opcode::LoadNegative, 100),
            instruction(Opcode::LoadAbsolute, 104),
        ),
        word(
            instruction(Opcode::AddAbsolute, 104),
            instruction(Opcode::SubtractAbsolute, 104),
        ),
        word(
            instruction(Opcode::LoadNegativeAbsolute, 104),
            instruction(Opcode::Add, 100),
        ),
        word(instruction(Opcode::Subtract, 100), HALT),
    ];
    machine.load_words(&program, 0).unwrap();
    machine.write_memory(100, 7).unwrap();
    machine.write_memory(101, 5).unwrap();
    machine.write_memory(102, 2).unwrap();
    machine.write_memory(104, signed(-9)).unwrap();

    assert_eq!(machine.step().unwrap().accumulator_after, 7);
    assert_eq!(machine.step().unwrap().accumulator_after, 12);
    assert_eq!(machine.step().unwrap().accumulator_after, 10);
    machine.step().unwrap();
    assert_eq!(machine.read_memory(103).unwrap(), 10);
    assert_eq!(machine.step().unwrap().accumulator_after, signed(-7));
    assert_eq!(machine.step().unwrap().accumulator_after, 9);
    assert_eq!(machine.step().unwrap().accumulator_after, 18);
    assert_eq!(machine.step().unwrap().accumulator_after, 9);
    assert_eq!(machine.step().unwrap().accumulator_after, signed(-9));
    assert_eq!(machine.step().unwrap().accumulator_after, signed(-2));
    assert_eq!(machine.step().unwrap().accumulator_after, signed(-9));
    assert!(machine.step().unwrap().state_after.halted);

    machine.reset();
    machine
        .load_words(
            &[word(
                instruction(Opcode::LoadAbsolute, 100),
                instruction(Opcode::LoadNegativeAbsolute, 100),
            )],
            0,
        )
        .unwrap();
    machine.write_memory(100, 1 << 39).unwrap();
    assert_eq!(machine.step().unwrap().accumulator_after, 1 << 39);
    assert_eq!(machine.step().unwrap().accumulator_after, 1 << 39);
}

#[test]
fn arithmetic_register_multiply_divide_and_shift_are_exact() {
    let mut machine = IasSimulator::new();
    let program = [
        word(
            instruction(Opcode::LoadArithmetic, 100),
            instruction(Opcode::Multiply, 101),
        ),
        word(
            instruction(Opcode::LoadArithmeticIntoAccumulator, 0),
            instruction(Opcode::ShiftLeft, 0),
        ),
        word(instruction(Opcode::ShiftRight, 0), HALT),
    ];
    machine.load_words(&program, 0).unwrap();
    machine.write_memory(100, 3).unwrap();
    machine.write_memory(101, signed(-2)).unwrap();

    assert_eq!(machine.step().unwrap().arithmetic_after, 3);
    let product = machine.step().unwrap();
    assert_eq!(product.accumulator_after, WORD_MASK);
    assert_eq!(product.arithmetic_after, signed(-6));
    assert_eq!(machine.step().unwrap().accumulator_after, signed(-6));
    assert_eq!(machine.step().unwrap().accumulator_after, signed(-12));
    assert_eq!(machine.step().unwrap().accumulator_after, signed(-6));
    machine.step().unwrap();

    machine.reset();
    machine
        .load_words(&[word(instruction(Opcode::Divide, 100), HALT)], 0)
        .unwrap();
    machine.set_accumulator(17).unwrap();
    machine.write_memory(100, 5).unwrap();
    let division = machine.step().unwrap();
    assert_eq!(division.arithmetic_after, 3);
    assert_eq!(division.accumulator_after, 2);
}

#[test]
fn divide_faults_are_typed_and_atomic() {
    let mut machine = IasSimulator::new();
    machine
        .load_words(&[word(instruction(Opcode::Divide, 100), HALT)], 0)
        .unwrap();
    machine.set_accumulator(7).unwrap();
    let before_zero = machine.snapshot();
    assert_eq!(machine.step(), Err(IasError::DivideByZero));
    assert_eq!(machine.snapshot(), before_zero);

    machine.write_memory(100, signed(-1)).unwrap();
    machine.set_accumulator(1 << 39).unwrap();
    let before_overflow = machine.snapshot();
    assert_eq!(machine.step(), Err(IasError::QuotientOverflow));
    assert_eq!(machine.snapshot(), before_overflow);
}

#[test]
fn paired_control_flow_and_condition_sign_are_preserved() {
    let mut machine = IasSimulator::new();
    let program = [
        word(
            instruction(Opcode::JumpRight, 2),
            instruction(Opcode::Load, 100),
        ),
        word(HALT, HALT),
        word(
            instruction(Opcode::Load, 101),
            instruction(Opcode::Load, 102),
        ),
    ];
    machine.load_words(&program, 0).unwrap();
    machine.write_memory(100, 1).unwrap();
    machine.write_memory(101, 2).unwrap();
    machine.write_memory(102, 3).unwrap();

    let jump = machine.step().unwrap();
    assert_eq!(jump.half, Half::Left);
    assert_eq!(jump.state_after.control_counter, 3);
    assert_eq!(
        jump.state_after.buffered_right,
        Some(instruction(Opcode::Load, 102))
    );
    let target = machine.step().unwrap();
    assert_eq!(target.word_address, 2);
    assert_eq!(target.half, Half::Right);
    assert_eq!(target.accumulator_after, 3);
    assert_eq!(
        machine.step(),
        Err(IasError::FetchOutsideProgram { word: 3 })
    );

    machine.reset();
    let branches = [
        word(
            instruction(Opcode::JumpNonNegativeLeft, 2),
            instruction(Opcode::Load, 100),
        ),
        word(HALT, HALT),
        word(HALT, HALT),
    ];
    machine.load_words(&branches, 0).unwrap();
    machine.set_accumulator(signed(-1)).unwrap();
    machine.write_memory(100, 11).unwrap();
    assert_eq!(
        machine.step().unwrap().state_after.buffered_right,
        Some(instruction(Opcode::Load, 100))
    );
    assert_eq!(machine.step().unwrap().accumulator_after, 11);
    assert!(machine.step().unwrap().state_after.halted);

    machine.reset();
    machine.load_words(&branches, 0).unwrap();
    machine.set_accumulator(0).unwrap();
    let taken = machine.step().unwrap();
    assert_eq!(taken.state_after.control_counter, 2);
    assert_eq!(taken.state_after.buffered_right, None);
    assert!(machine.step().unwrap().state_after.halted);

    machine.reset();
    machine
        .load_words(
            &[
                word(instruction(Opcode::JumpNonNegativeRight, 1), HALT),
                word(instruction(Opcode::Load, 100), HALT),
            ],
            0,
        )
        .unwrap();
    let right = machine.step().unwrap();
    assert_eq!(right.state_after.control_counter, 2);
    assert_eq!(right.state_after.buffered_right, Some(HALT));
    assert!(machine.step().unwrap().state_after.halted);
}

#[test]
fn partial_stores_respect_the_already_buffered_right_order() {
    let mut machine = IasSimulator::new();
    let original_right = instruction(Opcode::Load, 100);
    let first = word(instruction(Opcode::StoreRightAddress, 0), original_right);
    machine.load_words(&[first], 0).unwrap();
    machine.set_accumulator(0x123).unwrap();
    machine.write_memory(100, 9).unwrap();

    let store = machine.step().unwrap();
    assert_eq!(store.state_after.buffered_right, Some(original_right));
    let changed = machine.read_memory(0).unwrap();
    assert_eq!((changed & 0xfff) as u16, 0x123);
    assert_eq!(machine.step().unwrap().accumulator_after, 9);

    machine.reset();
    let target = word(instruction(Opcode::Load, 1), HALT);
    machine
        .load_words(
            &[word(instruction(Opcode::StoreLeftAddress, 1), HALT), target],
            0,
        )
        .unwrap();
    machine.set_accumulator(0x234).unwrap();
    machine.step().unwrap();
    let changed = machine.read_memory(1).unwrap();
    assert_eq!(((changed >> 20) & 0xfff) as u16, 0x234);
    assert_eq!(changed & 0xfffff, HALT as u64);
}

#[test]
fn run_and_execute_are_transactional_and_bounded() {
    let mut machine = IasSimulator::new();
    let loop_word = word(instruction(Opcode::JumpLeft, 0), HALT);
    machine.load_words(&[loop_word], 0).unwrap();
    let before = machine.snapshot();
    assert_eq!(
        machine.run(3),
        Err(IasError::StepLimitExceeded { limit: 3 })
    );
    assert_eq!(machine.snapshot(), before);

    let halt_image = [word(HALT, HALT)];
    let result = machine.execute_words(&halt_image, 20, 1).unwrap();
    assert!(result.halted);
    assert_eq!(result.traces.len(), 1);
    assert_eq!(result.final_state.loaded_origin, 20);
    assert_eq!(machine.snapshot(), result.final_state);

    let result = machine.execute(&bytes(&halt_image), 30, 1).unwrap();
    assert_eq!(result.final_state.loaded_origin, 30);
    assert!(machine.run(0).unwrap().halted);

    let committed = machine.snapshot();
    assert_eq!(
        machine.execute_words(&[loop_word], 0, 2),
        Err(IasError::StepLimitExceeded { limit: 2 })
    );
    assert_eq!(machine.snapshot(), committed);

    assert_eq!(
        machine.execute(&[1, 2, 3], 0, 1),
        Err(IasError::TransportAlignment { bytes: 3 })
    );
    assert_eq!(
        machine.execute_words(&[WORD_MASK + 1], 0, 1),
        Err(IasError::InvalidWord {
            value: WORD_MASK + 1,
        })
    );
    assert_eq!(machine.snapshot(), committed);

    machine
        .load_words(&[word((0xff << 12) | 1, HALT)], 0)
        .unwrap();
    let before_bad_run = machine.snapshot();
    assert_eq!(
        machine.run(1),
        Err(IasError::InvalidOpcode { opcode: 0xff })
    );
    assert_eq!(machine.snapshot(), before_bad_run);

    assert_eq!(
        machine.run(MAX_STEPS + 1),
        Err(IasError::StepLimitTooLarge {
            limit: MAX_STEPS + 1,
            maximum: MAX_STEPS,
        })
    );
    assert_eq!(
        machine.execute_words(&halt_image, 0, MAX_STEPS + 1),
        Err(IasError::StepLimitTooLarge {
            limit: MAX_STEPS + 1,
            maximum: MAX_STEPS,
        })
    );
    assert_eq!(machine.snapshot(), before_bad_run);
}

#[test]
fn decode_fetch_and_post_halt_faults_are_atomic() {
    let mut machine = IasSimulator::new();
    machine
        .load_words(&[word((0xff << 12) | 1, HALT)], 0)
        .unwrap();
    let before_opcode = machine.snapshot();
    assert_eq!(
        machine.step(),
        Err(IasError::InvalidOpcode { opcode: 0xff })
    );
    assert_eq!(machine.snapshot(), before_opcode);

    machine.reset();
    machine.load_words(&[word(HALT, HALT)], 0).unwrap();
    machine.step().unwrap();
    let halted = machine.snapshot();
    assert_eq!(machine.step(), Err(IasError::Halted));
    assert_eq!(machine.snapshot(), halted);

    machine.reset();
    assert_eq!(
        machine.step(),
        Err(IasError::FetchOutsideProgram { word: 0 })
    );
}

#[test]
fn restore_validates_complete_state_before_mutation() {
    let mut machine = IasSimulator::new();
    machine
        .load_words(&[word(instruction(Opcode::Load, 100), HALT)], 0)
        .unwrap();
    machine.write_memory(100, 7).unwrap();
    machine.step().unwrap();
    let valid = machine.snapshot();

    machine.reset();
    machine.restore(&valid).unwrap();
    assert_eq!(machine.snapshot(), valid);

    let cases = [
        {
            let mut state = valid.clone();
            state.memory[10] = WORD_MASK + 1;
            (state, IasError::InvalidStateWord { address: 10 })
        },
        {
            let mut state = valid.clone();
            state.accumulator = WORD_MASK + 1;
            (state, IasError::InvalidAccumulator)
        },
        {
            let mut state = valid.clone();
            state.arithmetic_register = WORD_MASK + 1;
            (state, IasError::InvalidArithmeticRegister)
        },
        {
            let mut state = valid.clone();
            state.current_instruction = 1 << 20;
            (state, IasError::InvalidControlRegister)
        },
        {
            let mut state = valid.clone();
            state.buffered_right = Some(1 << 20);
            (state, IasError::InvalidBufferedInstruction)
        },
        {
            let mut state = valid.clone();
            state.function_table = 22;
            (state, IasError::InvalidFunctionTable)
        },
        {
            let mut state = valid.clone();
            state.memory_address = MEMORY_WORDS as u16;
            (state, IasError::InvalidMemoryAddress)
        },
        {
            let mut state = valid.clone();
            state.selectron_register = WORD_MASK + 1;
            (state, IasError::InvalidSelectronRegister)
        },
        {
            let mut state = valid.clone();
            state.loaded_origin = MEMORY_WORDS as u16;
            (state, IasError::InvalidLoadedRange)
        },
        {
            let mut state = valid.clone();
            state.loaded_words = MEMORY_WORDS + 1;
            (state, IasError::InvalidLoadedRange)
        },
        {
            let mut state = valid.clone();
            state.loaded_origin = (MEMORY_WORDS - 1) as u16;
            state.loaded_words = 2;
            (state, IasError::InvalidLoadedRange)
        },
        {
            let mut state = valid.clone();
            state.loaded_origin = 0;
            state.loaded_words = 0;
            state.control_counter = 1;
            (state, IasError::InvalidLoadedRange)
        },
        {
            let mut state = valid.clone();
            state.control_counter = MEMORY_WORDS as u16;
            state.buffered_right = None;
            (state, IasError::InvalidControlCounter)
        },
        {
            let mut state = valid.clone();
            state.control_counter = state.loaded_origin;
            state.buffered_right = Some(HALT);
            (state, IasError::InvalidControlCounter)
        },
    ];

    for (invalid, expected) in cases {
        let before = machine.snapshot();
        assert_eq!(machine.restore(&invalid), Err(expected));
        assert_eq!(machine.snapshot(), before);
    }
}

#[test]
fn encoders_and_setters_reject_wide_values() {
    assert_eq!(
        encode_instruction(Opcode::Load, MEMORY_WORDS as u16),
        Err(IasError::InvalidAddress {
            address: MEMORY_WORDS as u16
        })
    );
    assert_eq!(
        pack_instructions(1 << 20, HALT),
        Err(IasError::InvalidInstruction { value: 1 << 20 })
    );
    assert_eq!(
        pack_instructions(HALT, 1 << 20),
        Err(IasError::InvalidInstruction { value: 1 << 20 })
    );

    let mut machine = IasSimulator::new();
    assert_eq!(
        machine.set_accumulator(WORD_MASK + 1),
        Err(IasError::InvalidWord {
            value: WORD_MASK + 1
        })
    );
    assert_eq!(
        machine.set_arithmetic_register(WORD_MASK + 1),
        Err(IasError::InvalidWord {
            value: WORD_MASK + 1
        })
    );
}

#[test]
fn traces_name_orders_and_capture_complete_boundaries() {
    let mut machine = IasSimulator::new();
    machine
        .load_words(&[word(instruction(Opcode::Load, 100), HALT)], 0)
        .unwrap();
    machine.write_memory(100, 42).unwrap();
    let trace = machine.step().unwrap();
    assert_eq!(trace.word_address, 0);
    assert_eq!(trace.half, Half::Left);
    assert_eq!(trace.raw, instruction(Opcode::Load, 100));
    assert_eq!(trace.opcode, Opcode::Load);
    assert_eq!(trace.address, 100);
    assert_eq!(trace.mnemonic, "LOAD M(X)");
    assert_eq!(trace.accumulator_before, 0);
    assert_eq!(trace.accumulator_after, 42);
    assert_eq!(trace.state_after, machine.snapshot());
    assert_eq!(Opcode::Halt.mnemonic(), "HALT");
}

#[test]
fn every_typed_error_has_a_stable_nonempty_message() {
    let errors = [
        IasError::TransportAlignment { bytes: 3 },
        IasError::EmptyProgram,
        IasError::OriginOutOfRange { origin: 4096 },
        IasError::ProgramOutOfRange {
            origin: 4095,
            words: 2,
        },
        IasError::MemoryOutOfRange { address: 4096 },
        IasError::InvalidAddress { address: 4096 },
        IasError::InvalidWord {
            value: WORD_MASK + 1,
        },
        IasError::InvalidInstruction { value: 1 << 20 },
        IasError::InvalidOpcode { opcode: 0xff },
        IasError::DivideByZero,
        IasError::QuotientOverflow,
        IasError::FetchOutsideProgram { word: 3 },
        IasError::Halted,
        IasError::StepLimitTooLarge {
            limit: MAX_STEPS + 1,
            maximum: MAX_STEPS,
        },
        IasError::StepLimitExceeded { limit: 2 },
        IasError::InvalidStateWord { address: 10 },
        IasError::InvalidAccumulator,
        IasError::InvalidArithmeticRegister,
        IasError::InvalidControlCounter,
        IasError::InvalidBufferedInstruction,
        IasError::InvalidControlRegister,
        IasError::InvalidFunctionTable,
        IasError::InvalidMemoryAddress,
        IasError::InvalidSelectronRegister,
        IasError::InvalidLoadedRange,
    ];

    for error in errors {
        assert!(!error.to_string().is_empty());
        let as_error: &dyn std::error::Error = &error;
        assert!(as_error.source().is_none());
    }
}
