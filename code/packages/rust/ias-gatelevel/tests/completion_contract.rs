use ias_gatelevel::{
    encode_instruction, pack_instructions, Half, IasError, IasGateLevel, IasState, Opcode,
    FLIP_FLOP_COUNT, HALT, MAX_STEPS, MEMORY_WORDS, WORD_MASK,
};
use ias_simulator::IasSimulator;

fn instruction(opcode: Opcode, address: u16) -> u32 {
    encode_instruction(opcode, address).unwrap()
}

fn word(left: u32, right: u32) -> u64 {
    pack_instructions(left, right).unwrap()
}

fn signed(value: i64) -> u64 {
    value as u64 & WORD_MASK
}

fn assert_step_parity(state: IasState) {
    let mut functional = IasSimulator::new();
    functional.restore(&state).unwrap();
    let mut gate = IasGateLevel::new();
    gate.restore(&state).unwrap();
    assert_eq!(gate.step(), functional.step());
    assert_eq!(gate.snapshot(), functional.snapshot());
}

fn state_for(opcode: Opcode) -> IasState {
    let mut state = IasState {
        loaded_words: 4,
        ..IasState::default()
    };
    state.memory[0] = word(instruction(opcode, 3), HALT);
    state.memory[1] = word(HALT, HALT);
    state.memory[2] = word(HALT, HALT);
    state.memory[3] = signed(-5);
    state.accumulator = 17;
    state.arithmetic_register = 3;
    state
}

#[test]
fn every_order_class_matches_the_functional_oracle() {
    let opcodes = [
        Opcode::Halt,
        Opcode::Load,
        Opcode::LoadNegative,
        Opcode::LoadAbsolute,
        Opcode::LoadNegativeAbsolute,
        Opcode::Add,
        Opcode::Subtract,
        Opcode::AddAbsolute,
        Opcode::SubtractAbsolute,
        Opcode::LoadArithmetic,
        Opcode::LoadArithmeticIntoAccumulator,
        Opcode::Multiply,
        Opcode::Divide,
        Opcode::JumpLeft,
        Opcode::JumpRight,
        Opcode::JumpNonNegativeLeft,
        Opcode::JumpNonNegativeRight,
        Opcode::Store,
        Opcode::StoreLeftAddress,
        Opcode::StoreRightAddress,
        Opcode::ShiftLeft,
        Opcode::ShiftRight,
    ];
    for opcode in opcodes {
        let mut state = state_for(opcode);
        if opcode == Opcode::Divide {
            state.memory[3] = 5;
        }
        if matches!(
            opcode,
            Opcode::JumpLeft
                | Opcode::JumpRight
                | Opcode::JumpNonNegativeLeft
                | Opcode::JumpNonNegativeRight
        ) {
            state.accumulator = 1;
        }
        assert_step_parity(state);
    }
}

#[test]
fn paired_halves_branches_and_buffered_self_modification_match() {
    let program = [
        word(
            instruction(Opcode::StoreRightAddress, 0),
            instruction(Opcode::Load, 20),
        ),
        word(
            instruction(Opcode::JumpNonNegativeRight, 2),
            instruction(Opcode::Load, 21),
        ),
        word(
            instruction(Opcode::Load, 22),
            instruction(Opcode::ShiftLeft, 0),
        ),
    ];
    let mut functional = IasSimulator::new();
    let mut gate = IasGateLevel::new();
    functional.load_words(&program, 0).unwrap();
    gate.load_words(&program, 0).unwrap();
    for (address, value) in [(20, 7), (21, 9), (22, 11)] {
        functional.write_memory(address, value).unwrap();
        gate.write_memory(address, value).unwrap();
    }
    functional.set_accumulator(21).unwrap();
    gate.set_accumulator(21).unwrap();
    for _ in 0..4 {
        let expected = functional.step();
        let actual = gate.step();
        assert_eq!(actual, expected);
        assert_eq!(gate.snapshot(), functional.snapshot());
    }
    assert_eq!(gate.snapshot().accumulator, 14);
}

#[test]
fn arithmetic_edges_and_typed_faults_are_atomic() {
    for (accumulator, divisor, expected) in [
        (17, 5, None),
        (7, 0, Some(IasError::DivideByZero)),
        (1_u64 << 39, WORD_MASK, Some(IasError::QuotientOverflow)),
    ] {
        let mut state = state_for(Opcode::Divide);
        state.accumulator = accumulator;
        state.memory[3] = divisor;
        let mut gate = IasGateLevel::new();
        gate.restore(&state).unwrap();
        let before = gate.snapshot();
        let result = gate.step();
        if let Some(error) = expected {
            assert_eq!(result, Err(error));
            assert_eq!(gate.snapshot(), before);
        } else {
            assert!(result.is_ok());
        }
    }

    let mut invalid = state_for(Opcode::Halt);
    invalid.memory[0] = word(22_u32 << 12, HALT);
    let mut gate = IasGateLevel::new();
    gate.restore(&invalid).unwrap();
    let before = gate.snapshot();
    assert_eq!(gate.step(), Err(IasError::InvalidOpcode { opcode: 22 }));
    assert_eq!(gate.snapshot(), before);

    invalid.halted = true;
    gate.restore(&invalid).unwrap();
    let before = gate.snapshot();
    assert_eq!(gate.step(), Err(IasError::Halted));
    assert_eq!(gate.snapshot(), before);
}

#[test]
fn transport_restore_direct_access_and_run_are_transactional() {
    let mut gate = IasGateLevel::new();
    let initial = gate.snapshot();
    assert_eq!(gate.load(&[], 0), Err(IasError::EmptyProgram));
    assert_eq!(
        gate.load(&[1, 2, 3, 4], 0),
        Err(IasError::TransportAlignment { bytes: 4 })
    );
    assert_eq!(
        gate.load(&[0; 5], MEMORY_WORDS as u16),
        Err(IasError::OriginOutOfRange {
            origin: MEMORY_WORDS as u16
        })
    );
    let oversized_transport = vec![0; (MEMORY_WORDS + 1) * 5];
    assert_eq!(
        gate.load(&oversized_transport, 0),
        Err(IasError::ProgramOutOfRange {
            origin: 0,
            words: MEMORY_WORDS + 1
        })
    );
    assert_eq!(
        gate.load_words(&[0], MEMORY_WORDS as u16),
        Err(IasError::OriginOutOfRange {
            origin: MEMORY_WORDS as u16
        })
    );
    assert_eq!(gate.snapshot(), initial);

    let image = [word(instruction(Opcode::Load, 10), HALT)];
    let bytes: Vec<u8> = image
        .iter()
        .flat_map(|value| value.to_be_bytes()[3..].to_vec())
        .collect();
    assert_eq!(gate.load(&bytes, 4), Ok(1));
    gate.write_memory(10, 42).unwrap();
    let loaded = gate.snapshot();
    assert_eq!(gate.run(0), Err(IasError::StepLimitExceeded { limit: 0 }));
    assert_eq!(gate.snapshot(), loaded);
    assert_eq!(
        gate.run(MAX_STEPS + 1),
        Err(IasError::StepLimitTooLarge {
            limit: MAX_STEPS + 1,
            maximum: MAX_STEPS
        })
    );
    let result = gate.run(2).unwrap();
    assert!(result.halted);
    assert_eq!(result.traces.len(), 2);
    assert_eq!(result.traces[0].half, Half::Left);
    assert_eq!(result.traces[1].half, Half::Right);

    let halted = gate.snapshot();
    assert!(gate.run(0).unwrap().traces.is_empty());
    assert_eq!(gate.snapshot(), halted);

    let before_execute_fault = gate.snapshot();
    assert_eq!(
        gate.execute(&[1, 2], 0, 1),
        Err(IasError::TransportAlignment { bytes: 2 })
    );
    assert_eq!(gate.snapshot(), before_execute_fault);

    assert_eq!(
        gate.read_memory(MEMORY_WORDS),
        Err(IasError::MemoryOutOfRange {
            address: MEMORY_WORDS
        })
    );
    assert_eq!(
        gate.write_memory(0, WORD_MASK + 1),
        Err(IasError::InvalidWord {
            value: WORD_MASK + 1
        })
    );
    assert_eq!(
        gate.set_arithmetic_register(WORD_MASK + 1),
        Err(IasError::InvalidWord {
            value: WORD_MASK + 1
        })
    );
    assert_eq!(gate.get_state(), gate.snapshot());

    let halt_image = [word(HALT, HALT)];
    let halt_bytes: Vec<u8> = halt_image
        .iter()
        .flat_map(|value| value.to_be_bytes()[3..].to_vec())
        .collect();
    let mut execute_gate = IasGateLevel::new();
    assert!(
        execute_gate
            .execute_words(&halt_image, 0, 1)
            .unwrap()
            .halted
    );
    assert!(execute_gate.execute(&halt_bytes, 0, 1).unwrap().halted);
    let before_limit = execute_gate.snapshot();
    assert_eq!(
        execute_gate.execute_words(&halt_image, 0, MAX_STEPS + 1),
        Err(IasError::StepLimitTooLarge {
            limit: MAX_STEPS + 1,
            maximum: MAX_STEPS
        })
    );
    assert_eq!(execute_gate.snapshot(), before_limit);
}

#[test]
fn exact_topology_and_complete_state_round_trip() {
    assert_eq!(FLIP_FLOP_COUNT, 164_060);
    let mut gate = IasGateLevel::new();
    let mut state = state_for(Opcode::Load);
    state.buffered_right = Some(instruction(Opcode::ShiftRight, 0));
    state.control_counter = 1;
    state.current_instruction = instruction(Opcode::Add, 3);
    state.function_table = Opcode::Add as u8;
    state.memory_address = 3;
    state.selectron_register = 0x12345;
    gate.restore(&state).unwrap();
    assert_eq!(gate.snapshot(), state);

    let mut invalid = state.clone();
    invalid.memory[8] = WORD_MASK + 1;
    let before = gate.snapshot();
    assert_eq!(
        gate.restore(&invalid),
        Err(IasError::InvalidStateWord { address: 8 })
    );
    assert_eq!(gate.snapshot(), before);
}
