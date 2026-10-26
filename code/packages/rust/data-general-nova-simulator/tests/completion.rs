use data_general_nova_simulator::{
    encode_memory_reference, encode_operate, AddressMode, AluOperation, CarryControl,
    MemoryOperation, NovaError, NovaSimulator, NovaState, ShiftOperation, SkipCondition,
    ADDRESS_MASK, DEFAULT_ORIGIN, HALT, MEMORY_WORDS,
};

fn program(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_be_bytes()).collect()
}

fn mref(operation: MemoryOperation, indirect: bool, mode: AddressMode, disp: u8) -> u16 {
    encode_memory_reference(operation, indirect, mode, disp).unwrap()
}

fn loaded(words: &[u16]) -> NovaSimulator {
    let mut cpu = NovaSimulator::new();
    cpu.load(&program(words)).unwrap();
    cpu
}

#[test]
fn all_twelve_memory_reference_opac_values_have_exact_destinations() {
    for ac in 0..4_u8 {
        let mut cpu = loaded(&[
            mref(
                MemoryOperation::Lda(ac),
                false,
                AddressMode::PageZero,
                0o40 + ac,
            ),
            mref(
                MemoryOperation::Sta(ac),
                false,
                AddressMode::PageZero,
                0o50 + ac,
            ),
            HALT,
        ]);
        cpu.set_memory_word(u16::from(0o40 + ac), 0x1200 + u16::from(ac))
            .unwrap();
        cpu.run(3).unwrap();
        assert_eq!(cpu.accumulator(ac).unwrap(), 0x1200 + u16::from(ac));
        assert_eq!(
            cpu.memory_word(u16::from(0o50 + ac)).unwrap(),
            0x1200 + u16::from(ac)
        );
    }

    for (operation, initial, expected, skipped) in [
        (MemoryOperation::Isz, 0xffff, 0, true),
        (MemoryOperation::Dsz, 1, 0, true),
        (MemoryOperation::Isz, 7, 8, false),
        (MemoryOperation::Dsz, 7, 6, false),
    ] {
        let mut cpu = loaded(&[
            mref(operation, false, AddressMode::PageZero, 0o40),
            HALT,
            HALT,
        ]);
        cpu.set_memory_word(0o40, initial).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(cpu.memory_word(0o40).unwrap(), expected);
        assert_eq!(trace.pc_after, DEFAULT_ORIGIN + 1 + u16::from(skipped));
    }

    let mut cpu = loaded(&[
        mref(MemoryOperation::Jsr, false, AddressMode::PageZero, 0o203),
        HALT,
        HALT,
        mref(MemoryOperation::Jmp, false, AddressMode::PageZero, 0o201),
    ]);
    cpu.step().unwrap();
    assert_eq!(cpu.accumulator(3).unwrap(), 0o201);
    assert_eq!(cpu.state().pc, 0o203);
    cpu.step().unwrap();
    assert_eq!(cpu.state().pc, 0o201);
}

#[test]
fn direct_address_modes_sign_extend_and_wrap_to_fifteen_bits() {
    for (mode, base, displacement, target) in [
        (AddressMode::PageZero, 0, 0xfe, 0o376),
        (AddressMode::PcRelative, 0, 0xfe, DEFAULT_ORIGIN - 1),
        (AddressMode::Ac2Relative, 1, 0xff, 0),
        (AddressMode::Ac3Relative, 0, 1, 1),
    ] {
        let mut cpu = loaded(&[
            mref(MemoryOperation::Lda(0), false, mode, displacement),
            HALT,
        ]);
        cpu.set_accumulator(2, base).unwrap();
        cpu.set_accumulator(3, base).unwrap();
        cpu.set_memory_word(target, 0x5500 | target).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(trace.effective_address, Some(target));
        assert_eq!(cpu.accumulator(0).unwrap(), 0x5500 | target);
    }

    let instruction = mref(MemoryOperation::Lda(0), false, AddressMode::Ac2Relative, 1);
    let mut cpu = loaded(&[instruction, HALT]);
    cpu.set_accumulator(2, ADDRESS_MASK).unwrap();
    cpu.set_memory_word(0, 0x1234).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.accumulator(0).unwrap(), 0x1234);
}

#[test]
fn autoindex_uses_the_old_indirect_bit_and_new_pointer_value() {
    let mut cpu = loaded(&[
        mref(MemoryOperation::Lda(0), true, AddressMode::PageZero, 0o20),
        HALT,
    ]);
    cpu.set_memory_word(0o20, 0x8000 | 0o377).unwrap();
    cpu.set_memory_word(0o400, 0o500).unwrap();
    cpu.set_memory_word(0o500, 0xbeef).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.memory_word(0o20).unwrap(), 0x8000 | 0o400);
    assert_eq!(cpu.memory_word(0o400).unwrap(), 0o500);
    assert_eq!(cpu.accumulator(0).unwrap(), 0xbeef);
}

#[test]
fn indirect_chain_limit_is_typed_and_whole_state_atomic() {
    let mut cpu = loaded(&[
        mref(MemoryOperation::Lda(0), true, AddressMode::PageZero, 0o40),
        HALT,
    ]);
    cpu.set_memory_word(0o40, 0x8000 | 0o40).unwrap();
    let before = cpu.state();
    assert!(matches!(
        cpu.step(),
        Err(NovaError::IndirectAddressLimit { pc: DEFAULT_ORIGIN })
    ));
    assert_eq!(cpu.state(), before);
}

#[test]
fn representative_alu_carry_shift_and_skip_cross_products_are_stable() {
    for (alu, expected) in [
        (AluOperation::Complement, 0xfffc),
        (AluOperation::Negate, 0xfffd),
        (AluOperation::Move, 3),
        (AluOperation::Increment, 4),
        (AluOperation::AddComplement, 1),
        (AluOperation::Subtract, 2),
        (AluOperation::Add, 8),
        (AluOperation::And, 1),
    ] {
        let instruction = encode_operate(
            0,
            1,
            alu,
            ShiftOperation::None,
            CarryControl::Clear,
            false,
            SkipCondition::None,
        )
        .unwrap();
        let mut cpu = loaded(&[instruction, HALT]);
        cpu.set_accumulator(0, 3).unwrap();
        cpu.set_accumulator(1, 5).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), expected, "{alu:?}");
    }

    for (shift, expected, carry) in [
        (ShiftOperation::None, 0x8001, true),
        (ShiftOperation::Left, 0x0003, true),
        (ShiftOperation::Right, 0xc000, true),
        (ShiftOperation::ByteSwap, 0x0180, true),
    ] {
        let instruction = encode_operate(
            0,
            1,
            AluOperation::Move,
            shift,
            CarryControl::Set,
            false,
            SkipCondition::None,
        )
        .unwrap();
        let mut cpu = loaded(&[instruction, HALT]);
        cpu.set_accumulator(0, 0x8001).unwrap();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), expected, "{shift:?}");
        assert_eq!(cpu.state().carry, carry, "{shift:?}");
    }
}

#[test]
fn no_load_evaluates_skips_without_mutating_registers_or_carry() {
    for skip in [
        SkipCondition::None,
        SkipCondition::Always,
        SkipCondition::CarryZero,
        SkipCondition::CarryNonzero,
        SkipCondition::ResultZero,
        SkipCondition::ResultNonzero,
        SkipCondition::EitherZero,
        SkipCondition::BothNonzero,
    ] {
        let instruction = encode_operate(
            0,
            1,
            AluOperation::Move,
            ShiftOperation::None,
            CarryControl::Set,
            true,
            skip,
        )
        .unwrap();
        let mut cpu = loaded(&[instruction, HALT, HALT]);
        cpu.set_accumulator(0, 7).unwrap();
        cpu.set_accumulator(1, 9).unwrap();
        let before = cpu.state();
        cpu.step().unwrap();
        assert_eq!(cpu.accumulator(1).unwrap(), 9);
        assert_eq!(cpu.state().carry, before.carry);
        let should_skip = matches!(
            skip,
            SkipCondition::Always
                | SkipCondition::CarryNonzero
                | SkipCondition::ResultNonzero
                | SkipCondition::BothNonzero
        );
        assert_eq!(cpu.state().pc, DEFAULT_ORIGIN + 1 + u16::from(should_skip));
    }
}

#[test]
fn unsupported_io_fetch_and_halt_faults_are_atomic() {
    let mut cpu = loaded(&[0o60_000, HALT]);
    let before = cpu.state();
    assert!(matches!(cpu.step(), Err(NovaError::UnsupportedIo { .. })));
    assert_eq!(cpu.state(), before);

    let mut cpu = loaded(&[mref(
        MemoryOperation::Jmp,
        false,
        AddressMode::PageZero,
        0o10,
    )]);
    cpu.step().unwrap();
    let before = cpu.state();
    assert_eq!(cpu.step(), Err(NovaError::FetchOutsideProgram { pc: 0o10 }));
    assert_eq!(cpu.state(), before);

    let mut cpu = loaded(&[HALT]);
    let trace = cpu.step().unwrap();
    assert_eq!(trace.pc_after, DEFAULT_ORIGIN + 1);
    let before = cpu.state();
    assert_eq!(cpu.step(), Err(NovaError::Halted));
    assert_eq!(cpu.state(), before);
}

#[test]
fn restore_load_reset_and_bounded_run_validate_complete_state() {
    let mut cpu = loaded(&[HALT]);
    cpu.set_accumulator(2, 0xabcd).unwrap();
    cpu.set_memory_word(0o777, 0xface).unwrap();
    let state = cpu.state();
    cpu.reset();
    cpu.restore(&state).unwrap();
    assert_eq!(cpu.state(), state);

    let mut bad_memory = state.clone();
    bad_memory.memory.pop();
    assert!(matches!(
        cpu.restore(&bad_memory),
        Err(NovaError::InvalidMemoryLength { .. })
    ));
    let mut bad_range = state.clone();
    bad_range.installed_origin = ADDRESS_MASK;
    bad_range.installed_words = 2;
    assert!(matches!(
        cpu.restore(&bad_range),
        Err(NovaError::InvalidInstalledRange { .. })
    ));
    assert_eq!(cpu.state(), state);

    let result = NovaSimulator::execute(&program(&[HALT]), 0o1000, 1).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 1);
    assert_eq!(result.final_state.memory.len(), MEMORY_WORDS);
    assert_eq!(result.traces[0].pc_before, 0o1000);

    assert_eq!(cpu.run(0), Err(NovaError::ZeroStepLimit));
    assert_eq!(cpu.state(), state);
}

#[test]
fn restore_rejects_out_of_range_pc_even_when_manually_constructed() {
    let mut cpu = NovaSimulator::new();
    let invalid = NovaState {
        accumulators: [0; 4],
        pc: 0xffff,
        carry: false,
        memory: vec![0; MEMORY_WORDS],
        halted: false,
        installed_origin: 0,
        installed_words: 0,
    };
    let before = cpu.state();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(NovaError::AddressOutOfRange { .. })
    ));
    assert_eq!(cpu.state(), before);
}
