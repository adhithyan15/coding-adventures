use cortex_m0_simulator::{
    classify, encoding::*, CortexM0Error, CortexM0Simulator, CortexM0State, InstructionClass,
    MEMORY_SIZE,
};

const ORIGIN: u32 = 0x100;

fn loaded(words: &[u16]) -> CortexM0Simulator {
    let mut simulator = CortexM0Simulator::new();
    simulator.load(&program(words), ORIGIN).unwrap();
    simulator
}

#[test]
fn every_documented_decode_family_is_explicit() {
    let bl = bl(0);
    let cases = [
        (0x0000, None, InstructionClass::ShiftAddSubtract),
        (0x2000, None, InstructionClass::Immediate),
        (0x4000, None, InstructionClass::DataProcessing),
        (0x4400, None, InstructionClass::SpecialData),
        (0x4800, None, InstructionClass::LiteralLoad),
        (0x5000, None, InstructionClass::RegisterMemory),
        (0x6000, None, InstructionClass::ImmediateMemory),
        (0x9000, None, InstructionClass::StackMemory),
        (0xa000, None, InstructionClass::AddressGeneration),
        (0xb000, None, InstructionClass::Miscellaneous),
        (0xc000, None, InstructionClass::MultipleMemory),
        (0xd000, None, InstructionClass::ConditionalBranch),
        (0xe000, None, InstructionClass::UnconditionalBranch),
        (bl[0], Some(bl[1]), InstructionClass::BranchLink),
        (0xf3bf, Some(0x8f5f), InstructionClass::Barrier),
        (0xf3ef, Some(0x8000), InstructionClass::SpecialRegister),
    ];
    for (first, second, expected) in cases {
        assert_eq!(classify(first, second), Ok(expected), "{first:#06x}");
    }
    assert!(matches!(
        classify(0xf000, Some(0)),
        Err(CortexM0Error::UnknownInstruction32 { .. })
    ));
}

#[test]
fn reset_load_and_restore_expose_complete_state() {
    let simulator = CortexM0Simulator::new();
    let state = simulator.get_state();
    assert_eq!(state.registers, [0; 13]);
    assert_eq!(state.msp, MEMORY_SIZE as u32);
    assert_eq!(state.psp, MEMORY_SIZE as u32);
    assert_eq!(state.memory.len(), MEMORY_SIZE);
    assert_eq!(state.loaded_len, 0);

    let simulator = loaded(&[nop(), bkpt(0)]);
    let state = simulator.get_state();
    assert_eq!(state.pc, ORIGIN);
    assert_eq!(state.loaded_origin, ORIGIN);
    assert_eq!(state.loaded_len, 4);
    assert_eq!(
        &state.memory[ORIGIN as usize..ORIGIN as usize + 4],
        &program(&[nop(), bkpt(0)])
    );
}

#[test]
fn load_and_restore_validation_are_atomic() {
    let mut simulator = loaded(&[bkpt(0)]);
    let before = simulator.get_state();
    assert_eq!(simulator.load(&[], 0), Err(CortexM0Error::EmptyProgram));
    assert!(matches!(
        simulator.load(&[0, 0], 1),
        Err(CortexM0Error::MisalignedProgram { .. })
    ));
    assert!(matches!(
        simulator.load(&[0; 4], 0xfffe),
        Err(CortexM0Error::ProgramOutOfRange { .. })
    ));
    assert_eq!(simulator.get_state(), before);

    for mutate in [
        |state: &mut CortexM0State| state.memory.pop().map(|_| ()).unwrap(),
        |state: &mut CortexM0State| state.pc = 1,
        |state: &mut CortexM0State| state.msp = 1,
        |state: &mut CortexM0State| state.apsr = 1,
        |state: &mut CortexM0State| state.control = 4,
        |state: &mut CortexM0State| state.ipsr = 64,
        |state: &mut CortexM0State| state.handler_mode = true,
    ] {
        let mut invalid = before.clone();
        mutate(&mut invalid);
        assert!(matches!(
            simulator.restore(invalid),
            Err(CortexM0Error::InvalidState { .. })
        ));
        assert_eq!(simulator.get_state(), before);
    }
}

#[test]
fn immediate_arithmetic_flags_and_halt_execute() {
    let mut simulator = loaded(&[movs(0, 255), adds_imm(0, 1), subs_imm(0, 1), bkpt(0)]);
    assert_eq!(simulator.step().unwrap().mnemonic, "movs");
    simulator.step().unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(0).unwrap(), 255);
    let trace = simulator.step().unwrap();
    assert_eq!(trace.mnemonic, "bkpt 0");
    assert!(trace.state_after.halted);
    assert_eq!(simulator.step(), Err(CortexM0Error::Halted));
}

#[test]
fn shifts_and_all_data_processing_groups_execute() {
    for operation in 0_u16..16 {
        let raw = 0x4000 | (operation << 6) | (1 << 3);
        let mut simulator = loaded(&[raw, bkpt(0)]);
        simulator.write_register(0, 0x8000_0001).unwrap();
        simulator
            .write_register(1, if operation == 13 { 3 } else { 1 })
            .unwrap();
        assert!(simulator.step().is_ok(), "operation {operation}");
    }
    for raw in [0x0008, 0x0808, 0x1008, 0x1808, 0x1a08, 0x1c08, 0x1e08] {
        let mut simulator = loaded(&[raw, bkpt(0)]);
        simulator.write_register(1, 8).unwrap();
        assert!(simulator.step().is_ok(), "word {raw:#06x}");
    }
}

#[test]
fn scalar_memory_modes_are_little_endian_checked_and_signed() {
    let mut simulator = loaded(&[str_imm(0, 1, 0), ldr_imm(2, 1, 0), bkpt(0)]);
    simulator.write_register(0, 0x89ab_cdef).unwrap();
    simulator.write_register(1, 0x200).unwrap();
    simulator.step().unwrap();
    assert_eq!(
        &simulator.get_state().memory[0x200..0x204],
        &[0xef, 0xcd, 0xab, 0x89]
    );
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(2).unwrap(), 0x89ab_cdef);

    for (operation, expected) in [(3_u16, 0xffff_ff80), (5, 0x0000_0080), (6, 0x80), (7, 0x80)] {
        let raw = 0x5000 | (operation << 9) | (1 << 3) | 2;
        let mut simulator = loaded(&[raw]);
        simulator.write_register(1, 0x200).unwrap();
        simulator.write_memory(0x200, 2, 0x80).unwrap();
        simulator.step().unwrap();
        assert_eq!(
            simulator.read_register(2).unwrap(),
            expected,
            "operation {operation}"
        );
    }
}

#[test]
fn literal_sp_and_address_generation_use_architectural_bases() {
    let mut literal = loaded(&[0x4801]);
    literal.write_memory(ORIGIN + 8, 4, 0x1234_5678).unwrap();
    literal.step().unwrap();
    assert_eq!(literal.read_register(0).unwrap(), 0x1234_5678);

    let mut address = loaded(&[0xa001, 0xa902, bkpt(0)]);
    address.step().unwrap();
    assert_eq!(address.read_register(0).unwrap(), ORIGIN + 8);
    address.step().unwrap();
    assert_eq!(address.read_register(1).unwrap(), MEMORY_SIZE as u32 + 8);
}

#[test]
fn extension_reverse_and_stack_operations_execute_in_order() {
    for (raw, input, expected) in [
        (0xb248, 0x1122_3380, 0xffff_ff80),
        (0xb288, 0x1122_ff80, 0x0000_ff80),
        (0xba08, 0x1122_3344, 0x4433_2211),
        (0xba48, 0x1122_3344, 0x2211_4433),
        (0xbac8, 0x1122_3344, 0x0000_4433),
    ] {
        let mut simulator = loaded(&[raw]);
        simulator.write_register(1, input).unwrap();
        simulator.step().unwrap();
        assert_eq!(simulator.read_register(0).unwrap(), expected, "{raw:#06x}");
    }

    let mut stack = loaded(&[0xb503, 0xbd0c, bkpt(0)]);
    stack.write_register(0, 10).unwrap();
    stack.write_register(1, 11).unwrap();
    stack.write_register(14, (ORIGIN + 5) | 1).unwrap();
    stack.step().unwrap();
    assert_eq!(stack.read_register(13).unwrap(), MEMORY_SIZE as u32 - 12);
    stack.step().unwrap();
    assert_eq!(stack.read_register(2).unwrap(), 10);
    assert_eq!(stack.read_register(3).unwrap(), 11);
    assert_eq!(stack.get_state().pc, ORIGIN + 4);
}

#[test]
fn thread_mode_process_stack_is_the_active_stack_when_selected() {
    let mut simulator = loaded(&[0xb401, 0xbc02, bkpt(0)]);
    let mut state = simulator.get_state();
    state.control = 0b10;
    state.psp = 0x300;
    state.msp = 0x400;
    state.registers[0] = 0x1234_5678;
    simulator.restore(state).unwrap();

    simulator.step().unwrap();
    assert_eq!(simulator.get_state().psp, 0x2fc);
    assert_eq!(simulator.get_state().msp, 0x400);
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(1).unwrap(), 0x1234_5678);
    assert_eq!(simulator.get_state().psp, 0x300);
    assert_eq!(simulator.get_state().msp, 0x400);
}

#[test]
fn multiple_memory_writeback_and_base_in_list_rules_execute() {
    let mut store = loaded(&[0xc10d, bkpt(0)]);
    store.write_register(0, 1).unwrap();
    store.write_register(1, 0x200).unwrap();
    store.write_register(2, 2).unwrap();
    store.write_register(3, 3).unwrap();
    store.step().unwrap();
    assert_eq!(store.read_register(1).unwrap(), 0x20c);
    assert_eq!(store.read_memory(0x200, 4).unwrap(), 1);

    let mut load = loaded(&[0xc903]);
    load.write_register(1, 0x200).unwrap();
    load.write_memory(0x200, 4, 7).unwrap();
    load.write_memory(0x204, 4, 8).unwrap();
    load.step().unwrap();
    assert_eq!(load.read_register(0).unwrap(), 7);
    assert_eq!(load.read_register(1).unwrap(), 8);
}

#[test]
fn direct_and_register_control_flow_preserve_thumb_state() {
    let mut conditional = loaded(&[b_cond(1, 0), movs(0, 1), bkpt(0)]);
    assert_eq!(conditional.step().unwrap().mnemonic, "bne");
    assert_eq!(conditional.get_state().pc, ORIGIN + 4);

    let mut direct = loaded(&[branch(0), movs(0, 1), bkpt(0)]);
    direct.step().unwrap();
    assert_eq!(direct.get_state().pc, ORIGIN + 4);

    let encoded = bl(0);
    let mut linked = loaded(&[encoded[0], encoded[1], bkpt(0)]);
    assert_eq!(linked.step().unwrap().mnemonic, "bl");
    assert_eq!(linked.read_register(14).unwrap(), ORIGIN + 5);
    assert_eq!(linked.get_state().pc, ORIGIN + 4);

    let mut bx = loaded(&[0x4708, bkpt(0)]);
    bx.write_register(1, ORIGIN + 3).unwrap();
    bx.step().unwrap();
    assert_eq!(bx.get_state().pc, ORIGIN + 2);
}

#[test]
fn invalid_thumb_targets_and_exceptions_are_typed_and_atomic() {
    let mut bx = loaded(&[0x4708]);
    bx.write_register(1, 0x200).unwrap();
    let before = bx.get_state();
    assert_eq!(
        bx.step(),
        Err(CortexM0Error::InvalidThumbTarget { target: 0x200 })
    );
    assert_eq!(bx.get_state(), before);

    for (raw, expected) in [
        (
            0xbe01,
            CortexM0Error::UnsupportedBreakpoint {
                immediate: 1,
                pc: ORIGIN,
            },
        ),
        (
            0xdf02,
            CortexM0Error::SupervisorCall {
                immediate: 2,
                pc: ORIGIN,
            },
        ),
    ] {
        let mut simulator = loaded(&[raw]);
        let before = simulator.get_state();
        assert_eq!(simulator.step(), Err(expected));
        assert_eq!(simulator.get_state(), before);
    }
}

#[test]
fn barriers_special_registers_and_privilege_execute() {
    for (second, mnemonic) in [(0x8f4f, "dsb"), (0x8f5f, "dmb"), (0x8f6f, "isb")] {
        let mut simulator = loaded(&[0xf3bf, second]);
        assert_eq!(simulator.step().unwrap().mnemonic, mnemonic);
    }

    let encoded = msr(0, 20);
    let mut simulator = loaded(&[encoded[0], encoded[1], mrs(1, 20)[0], mrs(1, 20)[1]]);
    simulator.write_register(0, 2).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.get_state().control, 2);
    simulator.step().unwrap();
    assert_eq!(simulator.read_register(1).unwrap(), 2);

    let encoded = msr(0, 16);
    let mut unprivileged = loaded(&[encoded[0], encoded[1]]);
    let mut state = unprivileged.get_state();
    state.control = 1;
    unprivileged.restore(state).unwrap();
    let before = unprivileged.get_state();
    assert!(matches!(
        unprivileged.step(),
        Err(CortexM0Error::PrivilegeViolation { .. })
    ));
    assert_eq!(unprivileged.get_state(), before);
}

#[test]
fn cps_and_low_power_hints_track_interrupt_event_and_sleep_state() {
    let mut cps = loaded(&[0xb672, 0xb662, bkpt(0)]);
    cps.step().unwrap();
    assert!(cps.get_state().primask);
    cps.step().unwrap();
    assert!(!cps.get_state().primask);

    let mut event = loaded(&[0xbf40, 0xbf20, 0xbf30]);
    event.step().unwrap();
    assert!(event.get_state().event_registered);
    event.step().unwrap();
    assert!(!event.get_state().event_registered);
    assert!(!event.get_state().sleeping);
    event.step().unwrap();
    assert!(event.get_state().sleeping);
    assert_eq!(event.step(), Err(CortexM0Error::Sleeping));
    event.signal_event();
    assert!(!event.get_state().sleeping);
}

#[test]
fn data_fetch_and_reserved_faults_are_atomic() {
    let mut data = loaded(&[ldr_imm(0, 1, 0)]);
    data.write_register(1, 1).unwrap();
    let before = data.get_state();
    assert!(matches!(
        data.step(),
        Err(CortexM0Error::MisalignedData { .. })
    ));
    assert_eq!(data.get_state(), before);

    let mut default = CortexM0Simulator::new();
    assert_eq!(
        default.step(),
        Err(CortexM0Error::FetchOutsideProgram { pc: 0, width: 2 })
    );

    for raw in [0xde00, 0xb100, 0xb600] {
        let mut simulator = loaded(&[raw]);
        let before = simulator.get_state();
        assert!(simulator.step().is_err());
        assert_eq!(simulator.get_state(), before);
    }

    let mut truncated = loaded(&[0xf000]);
    let before = truncated.get_state();
    assert_eq!(
        truncated.step(),
        Err(CortexM0Error::FetchOutsideProgram {
            pc: ORIGIN,
            width: 4,
        })
    );
    assert_eq!(truncated.get_state(), before);
}

#[test]
fn run_and_execute_are_bounded_and_transactional() {
    let mut simulator = loaded(&[movs(0, 1), nop()]);
    let before = simulator.get_state();
    assert_eq!(
        simulator.run(2),
        Err(CortexM0Error::StepLimitExceeded { limit: 2 })
    );
    assert_eq!(simulator.get_state(), before);
    assert_eq!(
        simulator.run(1025),
        Err(CortexM0Error::RetentionLimitExceeded {
            requested: 1025,
            limit: 1024,
        })
    );

    let mut fresh = CortexM0Simulator::new();
    let result = fresh
        .execute(&program(&[movs(0, 9), bkpt(0)]), 0x200, 2)
        .unwrap();
    assert_eq!(result.steps, 2);
    assert!(result.stopped);
    assert_eq!(result.final_state.registers[0], 9);
    let before = fresh.get_state();
    assert!(fresh.execute(&[0], 1, 1).is_err());
    assert_eq!(fresh.get_state(), before);
    assert_eq!(
        fresh.execute(&[0], 1, 1025),
        Err(CortexM0Error::RetentionLimitExceeded {
            requested: 1025,
            limit: 1024,
        })
    );
}

#[test]
fn every_error_has_display_text() {
    let errors = [
        CortexM0Error::EmptyProgram,
        CortexM0Error::InvalidRegister { index: 16 },
        CortexM0Error::MisalignedFetch { pc: 1 },
        CortexM0Error::UnknownInstruction16 { raw: 0, pc: 0 },
        CortexM0Error::AllocationFailed,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}
