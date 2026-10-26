use cortex_m3_gatelevel::{
    classify, encoding::*, CortexM3Error, CortexM3GateLevel, CortexM3State, InstructionClass,
    MEMORY_SIZE,
};

const ORIGIN: u32 = 0x100;

fn loaded(words: &[u16]) -> CortexM3GateLevel {
    let mut simulator = CortexM3GateLevel::new();
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
        (0xf240, Some(0), InstructionClass::Thumb2Data),
        (0xf8c0, Some(0), InstructionClass::Thumb2Memory),
        (0xe850, Some(0x0f00), InstructionClass::Exclusive),
        (0xe8d0, Some(0xf000), InstructionClass::TableBranch),
    ];
    for (first, second, expected) in cases {
        assert_eq!(classify(first, second), Ok(expected), "{first:#06x}");
    }
    assert!(matches!(
        classify(0xf000, Some(0)),
        Err(CortexM3Error::UnknownInstruction32 { .. })
    ));
}

#[test]
fn reset_load_and_restore_expose_complete_state() {
    let simulator = CortexM3GateLevel::new();
    let state = simulator.get_state();
    assert_eq!(state.registers, [0; 13]);
    assert_eq!(state.msp, MEMORY_SIZE as u32);
    assert_eq!(state.psp, MEMORY_SIZE as u32);
    assert_eq!(state.memory.len(), MEMORY_SIZE);
    assert_eq!(state.loaded_len, 0);
    assert_eq!(state.itstate, 0);
    assert_eq!(state.basepri, 0);
    assert!(!state.faultmask);
    assert!(!state.waiting_for_event);
    assert_eq!(state.exclusive_address, None);

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
    assert_eq!(simulator.load(&[], 0), Err(CortexM3Error::EmptyProgram));
    assert!(matches!(
        simulator.load(&[0, 0], 1),
        Err(CortexM3Error::MisalignedProgram { .. })
    ));
    assert!(matches!(
        simulator.load(&[0; 4], 0xfffe),
        Err(CortexM3Error::ProgramOutOfRange { .. })
    ));
    assert_eq!(simulator.get_state(), before);

    for mutate in [
        |state: &mut CortexM3State| state.memory.pop().map(|_| ()).unwrap(),
        |state: &mut CortexM3State| state.pc = 1,
        |state: &mut CortexM3State| state.msp = 1,
        |state: &mut CortexM3State| state.apsr = 1,
        |state: &mut CortexM3State| state.control = 4,
        |state: &mut CortexM3State| state.handler_mode = true,
    ] {
        let mut invalid = before.clone();
        mutate(&mut invalid);
        assert!(matches!(
            simulator.restore(invalid),
            Err(CortexM3Error::InvalidState { .. })
        ));
        assert_eq!(simulator.get_state(), before);
    }

    for invalid in [
        CortexM3State {
            itstate: 0xec,
            ..before.clone()
        },
        CortexM3State {
            exclusive_address: Some(3),
            ..before.clone()
        },
        CortexM3State {
            waiting_for_event: true,
            ..before.clone()
        },
        CortexM3State {
            sleeping: true,
            waiting_for_event: true,
            event_registered: true,
            ..before.clone()
        },
        CortexM3State {
            halted: true,
            sleeping: true,
            ..before.clone()
        },
    ] {
        assert!(matches!(
            simulator.restore(invalid),
            Err(CortexM3Error::InvalidState { .. })
        ));
        assert_eq!(simulator.get_state(), before);
    }

    for ipsr in [1, 7, 8, 9, 10, 13] {
        let mut invalid = before.clone();
        invalid.ipsr = ipsr;
        invalid.handler_mode = true;
        assert!(matches!(
            simulator.restore(invalid),
            Err(CortexM3Error::InvalidState { .. })
        ));
        assert_eq!(simulator.get_state(), before);
    }

    let mut maximum_exception = before.clone();
    maximum_exception.ipsr = u8::MAX;
    maximum_exception.handler_mode = true;
    simulator.restore(maximum_exception).unwrap();
    assert_eq!(simulator.get_state().ipsr, u8::MAX);

    let mut invalid_handler_stack = simulator.get_state();
    invalid_handler_stack.control = 2;
    assert!(matches!(
        simulator.restore(invalid_handler_stack),
        Err(CortexM3Error::InvalidState { .. })
    ));
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
    assert_eq!(simulator.step(), Err(CortexM3Error::Halted));
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

    let mut misaligned = loaded(&[0xc101]);
    misaligned.write_register(1, 0x201).unwrap();
    let before = misaligned.get_state();
    assert!(matches!(
        misaligned.step(),
        Err(CortexM3Error::MisalignedData { .. })
    ));
    assert_eq!(misaligned.get_state(), before);
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

    for raw in [0x4768, 0x47e8] {
        let mut stack_target = loaded(&[raw]);
        let mut state = stack_target.get_state();
        state.msp = 0x300;
        stack_target.restore(state).unwrap();
        let before = stack_target.get_state();
        assert_eq!(
            stack_target.step(),
            Err(CortexM3Error::InvalidThumbTarget { target: 0x300 })
        );
        assert_eq!(stack_target.get_state(), before);
    }

    let mut pc_target = loaded(&[0x4778]);
    let before = pc_target.get_state();
    assert_eq!(
        pc_target.step(),
        Err(CortexM3Error::InvalidThumbTarget { target: ORIGIN + 4 })
    );
    assert_eq!(pc_target.get_state(), before);

    let mut invalid_blx_pc = loaded(&[0x47f8]);
    let before = invalid_blx_pc.get_state();
    assert!(matches!(
        invalid_blx_pc.step(),
        Err(CortexM3Error::UnknownInstruction16 { .. })
    ));
    assert_eq!(invalid_blx_pc.get_state(), before);

    for raw in [0x4508, 0x4587, 0x45f8, 0x44ff] {
        let mut reserved = loaded(&[raw]);
        let before = reserved.get_state();
        assert_eq!(
            reserved.step(),
            Err(CortexM3Error::UnknownInstruction16 { raw, pc: ORIGIN })
        );
        assert_eq!(reserved.get_state(), before);
    }

    let mut compare_sp = loaded(&[0x4568]);
    compare_sp.write_register(0, MEMORY_SIZE as u32).unwrap();
    assert_eq!(compare_sp.step().unwrap().mnemonic, "cmp");
    assert_eq!(bx.get_state().pc, ORIGIN + 2);

    for (raw, expected) in [(0x4487, 0x304), (0x4687, 0x200)] {
        let mut alu_pc = loaded(&[raw]);
        alu_pc.write_register(0, 0x200).unwrap();
        alu_pc.step().unwrap();
        assert_eq!(alu_pc.get_state().pc, expected, "{raw:#06x}");
    }
}

#[test]
fn invalid_thumb_targets_and_exceptions_are_typed_and_atomic() {
    let mut bx = loaded(&[0x4708]);
    bx.write_register(1, 0x200).unwrap();
    let before = bx.get_state();
    assert_eq!(
        bx.step(),
        Err(CortexM3Error::InvalidThumbTarget { target: 0x200 })
    );
    assert_eq!(bx.get_state(), before);

    let mut malformed_bx = loaded(&[0x4709]);
    let before = malformed_bx.get_state();
    assert!(matches!(
        malformed_bx.step(),
        Err(CortexM3Error::UnknownInstruction16 { .. })
    ));
    assert_eq!(malformed_bx.get_state(), before);

    for raw in [0x4770, 0xbd00] {
        let mut exception_return = loaded(&[raw]);
        let mut state = exception_return.get_state();
        state.handler_mode = true;
        state.ipsr = 11;
        state.lr = 0xffff_fff9;
        state.msp = 0x300;
        state.memory[0x300..0x304].copy_from_slice(&0xffff_fff9_u32.to_le_bytes());
        exception_return.restore(state).unwrap();
        let before = exception_return.get_state();
        assert_eq!(
            exception_return.step(),
            Err(CortexM3Error::UnsupportedExceptionReturn {
                target: 0xffff_fff9,
            })
        );
        assert_eq!(exception_return.get_state(), before);
    }

    let mut handler_blx = loaded(&[0x47f0]);
    let mut state = handler_blx.get_state();
    state.handler_mode = true;
    state.ipsr = 11;
    state.lr = 0xffff_fff9;
    handler_blx.restore(state).unwrap();
    assert_eq!(handler_blx.step().unwrap().mnemonic, "blx");
    assert_eq!(handler_blx.get_state().pc, 0xffff_fff8);

    for (raw, expected) in [
        (
            0xbe01,
            CortexM3Error::UnsupportedBreakpoint {
                immediate: 1,
                pc: ORIGIN,
            },
        ),
        (
            0xdf02,
            CortexM3Error::SupervisorCall {
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
    let reads = [mrs(1, 0), mrs(2, 5), mrs(3, 6), mrs(4, 8), mrs(5, 20)];
    let mut words = vec![encoded[0], encoded[1]];
    for read in reads {
        words.extend(read);
    }
    words.extend(msr(0, 3));
    words.extend(msr(0, 7));
    let mut unprivileged = loaded(&words);
    let mut state = unprivileged.get_state();
    state.control = 1;
    state.apsr = 0x5000_0000;
    state.primask = true;
    state.msp = 0x400;
    state.registers[0] = 0xa000_0000;
    unprivileged.restore(state).unwrap();
    unprivileged.step().unwrap();
    assert!(unprivileged.get_state().primask);
    for _ in 0..5 {
        unprivileged.step().unwrap();
    }
    assert_eq!(unprivileged.read_register(1).unwrap(), 0x5000_0000);
    assert_eq!(unprivileged.read_register(2).unwrap(), 0);
    assert_eq!(unprivileged.read_register(3).unwrap(), 0);
    assert_eq!(unprivileged.read_register(4).unwrap(), 0);
    assert_eq!(unprivileged.read_register(5).unwrap(), 1);
    unprivileged.step().unwrap();
    assert_eq!(unprivileged.get_state().apsr, 0xa000_0000);
    unprivileged.step().unwrap();
    assert_eq!(unprivileged.get_state().ipsr, 0);

    let composite_reads = [mrs(0, 1), mrs(1, 2), mrs(2, 3)];
    let mut words = Vec::new();
    for read in composite_reads {
        words.extend(read);
    }
    let mut unprivileged_composites = loaded(&words);
    let mut state = unprivileged_composites.get_state();
    state.control = 1;
    state.apsr = 0xa800_0000;
    unprivileged_composites.restore(state).unwrap();
    for _ in 0..3 {
        unprivileged_composites.step().unwrap();
    }
    assert_eq!(
        unprivileged_composites.read_register(0).unwrap(),
        0xa800_0000
    );
    assert_eq!(
        unprivileged_composites.read_register(1).unwrap(),
        0xa800_0000
    );
    assert_eq!(
        unprivileged_composites.read_register(2).unwrap(),
        0xa800_0000
    );

    let reads = [
        mrs(0, 1),
        mrs(1, 2),
        mrs(2, 3),
        mrs(3, 5),
        mrs(4, 6),
        mrs(5, 7),
    ];
    let mut words = Vec::new();
    for read in reads {
        words.extend(read);
    }
    let mut handler = loaded(&words);
    let mut state = handler.get_state();
    state.handler_mode = true;
    state.ipsr = 11;
    state.apsr = 0x5000_0000;
    handler.restore(state).unwrap();
    for _ in 0..6 {
        handler.step().unwrap();
    }
    assert_eq!(handler.read_register(0).unwrap(), 0x5000_000b);
    assert_eq!(handler.read_register(1).unwrap(), 0x5000_0000);
    assert_eq!(handler.read_register(2).unwrap(), 0x5000_000b);
    assert_eq!(handler.read_register(3).unwrap(), 11);
    assert_eq!(handler.read_register(4).unwrap(), 0);
    assert_eq!(handler.read_register(5).unwrap(), 11);

    let set_control = msr(0, 20);
    let get_control = mrs(1, 20);
    let mut words = Vec::new();
    words.extend(set_control);
    words.extend(get_control);
    let mut handler_control = loaded(&words);
    let mut state = handler_control.get_state();
    state.handler_mode = true;
    state.ipsr = 11;
    state.registers[0] = 2;
    handler_control.restore(state).unwrap();
    handler_control.step().unwrap();
    handler_control.step().unwrap();
    assert_eq!(handler_control.get_state().control & 2, 0);
    assert_eq!(handler_control.read_register(1).unwrap(), 0);
}

#[test]
fn apsr_msr_masks_classify_encode_and_preserve_groups() {
    assert_eq!(msr(0, 0), msr_masked(0, 0, ApsrMask::Nzcvq));
    assert_eq!(msr_masked(0, 0, ApsrMask::Ge), [0xf380, 0x8400]);
    assert_eq!(msr_masked(0, 0, ApsrMask::Nzcvq), [0xf380, 0x8800]);
    assert_eq!(msr_masked(0, 0, ApsrMask::NzcvqGe), [0xf380, 0x8c00]);

    for sysm in 0..=3 {
        let encoded = msr_masked(0, sysm, ApsrMask::Nzcvq);
        assert_eq!(
            classify(encoded[0], Some(encoded[1])),
            Ok(InstructionClass::SpecialRegister)
        );
        let mut simulator = loaded(&encoded);
        let mut state = simulator.get_state();
        state.apsr = 0x5000_0000;
        state.registers[0] = 0xa80b_0000;
        simulator.restore(state).unwrap();
        assert_eq!(simulator.step().unwrap().mnemonic, "msr");
        assert_eq!(simulator.get_state().apsr, 0xa800_0000);

        for reserved in [
            [0xf380, 0x8000 | u16::from(sysm)],
            msr_masked(0, sysm, ApsrMask::Ge),
            msr_masked(0, sysm, ApsrMask::NzcvqGe),
        ] {
            assert!(matches!(
                classify(reserved[0], Some(reserved[1])),
                Err(CortexM3Error::UnknownInstruction32 { .. })
            ));
            let mut simulator = loaded(&reserved);
            let before = simulator.get_state();
            assert!(matches!(
                simulator.step(),
                Err(CortexM3Error::UnknownInstruction32 { .. })
            ));
            assert_eq!(simulator.get_state(), before);
        }
    }

    for mask in [ApsrMask::Ge, ApsrMask::NzcvqGe] {
        let reserved = msr_masked(0, 20, mask);
        assert!(matches!(
            classify(reserved[0], Some(reserved[1])),
            Err(CortexM3Error::UnknownInstruction32 { .. })
        ));
    }
}

#[test]
fn cps_and_low_power_hints_track_interrupt_event_and_sleep_state() {
    let mut cps = loaded(&[0xb672, 0xb662, bkpt(0)]);
    cps.step().unwrap();
    assert!(cps.get_state().primask);
    cps.step().unwrap();
    assert!(!cps.get_state().primask);

    let mut unprivileged = loaded(&[0xb672, 0xb662, 0xbf50]);
    let mut state = unprivileged.get_state();
    state.control = 1;
    unprivileged.restore(state).unwrap();
    unprivileged.step().unwrap();
    assert!(!unprivileged.get_state().primask);

    for ipsr in [2, 3] {
        let set_faultmask = msr(0, 19);
        let mut words = vec![0xb671];
        words.extend(set_faultmask);
        let mut protected = loaded(&words);
        let mut state = protected.get_state();
        state.handler_mode = true;
        state.ipsr = ipsr;
        state.registers[0] = 1;
        protected.restore(state).unwrap();
        protected.step().unwrap();
        assert!(!protected.get_state().faultmask);
        protected.step().unwrap();
        assert!(!protected.get_state().faultmask);
    }
    unprivileged.step().unwrap();
    assert!(!unprivileged.get_state().primask);
    assert_eq!(unprivileged.step().unwrap().mnemonic, "nop");

    let mut event = loaded(&[0xbf40, 0xbf20, 0xbf30]);
    event.step().unwrap();
    assert!(event.get_state().event_registered);
    event.step().unwrap();
    assert!(!event.get_state().event_registered);
    assert!(!event.get_state().sleeping);
    event.step().unwrap();
    assert!(event.get_state().sleeping);
    assert!(!event.get_state().waiting_for_event);
    assert_eq!(event.step(), Err(CortexM3Error::Sleeping));
    event.signal_event();
    assert!(event.get_state().sleeping);
    assert!(event.get_state().event_registered);
    event.signal_interrupt();
    assert!(!event.get_state().sleeping);

    let mut waiting = loaded(&[0xbf20, nop()]);
    waiting.step().unwrap();
    assert!(waiting.get_state().sleeping);
    assert!(waiting.get_state().waiting_for_event);
    waiting.signal_event();
    assert!(!waiting.get_state().sleeping);
    assert!(!waiting.get_state().waiting_for_event);
    assert!(!waiting.get_state().event_registered);
}

#[test]
fn data_fetch_and_reserved_faults_are_atomic() {
    let mut data = loaded(&[ldr_imm(0, 1, 0)]);
    data.write_register(1, 1).unwrap();
    data.write_memory(1, 4, 0x4433_2211).unwrap();
    data.step().unwrap();
    assert_eq!(data.read_register(0).unwrap(), 0x4433_2211);

    let mut default = CortexM3GateLevel::new();
    assert_eq!(
        default.step(),
        Err(CortexM3Error::FetchOutsideProgram { pc: 0, width: 2 })
    );

    for raw in [0xde00, 0xb600] {
        let mut simulator = loaded(&[raw]);
        let before = simulator.get_state();
        assert!(simulator.step().is_err());
        assert_eq!(simulator.get_state(), before);
    }

    let mut truncated = loaded(&[0xf000]);
    let before = truncated.get_state();
    assert_eq!(
        truncated.step(),
        Err(CortexM3Error::FetchOutsideProgram {
            pc: ORIGIN,
            width: 4,
        })
    );
    assert_eq!(truncated.get_state(), before);
}

#[test]
fn it_blocks_execute_and_skip_from_architectural_conditions() {
    let mut skipped = loaded(&[movs(0, 1), it(0, 8), movs(1, 7), bkpt(0)]);
    skipped.step().unwrap();
    assert_eq!(skipped.step().unwrap().mnemonic, "it");
    let trace = skipped.step().unwrap();
    assert_eq!(trace.mnemonic, "it-skip");
    assert_eq!(skipped.read_register(1).unwrap(), 0);
    assert_eq!(skipped.get_state().itstate, 0);

    let mut executed = loaded(&[movs(0, 1), it(1, 8), movs(2, 9), bkpt(0)]);
    executed.step().unwrap();
    executed.step().unwrap();
    assert_eq!(executed.step().unwrap().mnemonic, "movs");
    assert_eq!(executed.read_register(2).unwrap(), 9);
    assert_eq!(executed.get_state().itstate, 0);

    let mut flags = loaded(&[it(1, 8), movs(0, 0)]);
    let mut state = flags.get_state();
    state.apsr = 1 << 29;
    flags.restore(state).unwrap();
    flags.step().unwrap();
    flags.step().unwrap();
    assert_eq!(flags.get_state().apsr, 1 << 29);

    let mut breakpoint = loaded(&[movs(0, 1), it(0, 8), bkpt(0)]);
    breakpoint.step().unwrap();
    breakpoint.step().unwrap();
    assert_eq!(breakpoint.step().unwrap().mnemonic, "bkpt 0");
    assert!(breakpoint.get_state().halted);

    let mut always = loaded(&[it(14, 4), movs(0, 1), movs(1, 2)]);
    for _ in 0..3 {
        always.step().unwrap();
    }
    assert_eq!(always.read_register(0).unwrap(), 1);
    assert_eq!(always.read_register(1).unwrap(), 2);

    let mut invalid_al = loaded(&[it(14, 12)]);
    let before = invalid_al.get_state();
    assert!(matches!(
        invalid_al.step(),
        Err(CortexM3Error::UnknownInstruction16 { .. })
    ));
    assert_eq!(invalid_al.get_state(), before);
}

#[test]
fn it_rejects_forbidden_and_nonfinal_control_flow_even_when_condition_fails() {
    for (mask, raw) in [(8, cbz(0, 0)), (8, 0xb672), (8, it(1, 8)), (4, 0x4708)] {
        let mut simulator = loaded(&[movs(0, 1), it(0, mask), raw, nop()]);
        simulator.step().unwrap();
        simulator.step().unwrap();
        let before = simulator.get_state();
        assert_eq!(
            simulator.step(),
            Err(CortexM3Error::UnknownInstruction16 {
                raw,
                pc: ORIGIN + 4,
            })
        );
        assert_eq!(simulator.get_state(), before);
    }

    for condition in [0, 1] {
        let mut simulator = loaded(&[movs(0, 1), it(condition, 8), 0x0001]);
        simulator.step().unwrap();
        simulator.step().unwrap();
        let before = simulator.get_state();
        assert_eq!(
            simulator.step(),
            Err(CortexM3Error::UnknownInstruction16 {
                raw: 0x0001,
                pc: ORIGIN + 4,
            })
        );
        assert_eq!(simulator.get_state(), before);
    }

    let mismatched_branch = b_cond(1, 0);
    let mut mismatch = loaded(&[movs(0, 1), it(0, 8), mismatched_branch]);
    mismatch.step().unwrap();
    mismatch.step().unwrap();
    let before = mismatch.get_state();
    assert_eq!(
        mismatch.step(),
        Err(CortexM3Error::UnknownInstruction16 {
            raw: mismatched_branch,
            pc: ORIGIN + 4,
        })
    );
    assert_eq!(mismatch.get_state(), before);

    let mut skipped_svc = loaded(&[movs(0, 1), it(0, 4), 0xdf01, nop()]);
    skipped_svc.step().unwrap();
    skipped_svc.step().unwrap();
    assert_eq!(skipped_svc.step().unwrap().mnemonic, "it-skip");

    let mut executed_svc = loaded(&[movs(0, 1), it(1, 4), 0xdf01, nop()]);
    executed_svc.step().unwrap();
    executed_svc.step().unwrap();
    let before = executed_svc.get_state();
    assert_eq!(
        executed_svc.step(),
        Err(CortexM3Error::SupervisorCall {
            immediate: 1,
            pc: ORIGIN + 4,
        })
    );
    assert_eq!(executed_svc.get_state(), before);

    let mut skipped_udf = loaded(&[movs(0, 1), it(0, 4), 0xde01, nop()]);
    skipped_udf.step().unwrap();
    skipped_udf.step().unwrap();
    assert_eq!(skipped_udf.step().unwrap().mnemonic, "it-skip");

    let mut executed_udf = loaded(&[movs(0, 1), it(1, 4), 0xde01, nop()]);
    executed_udf.step().unwrap();
    executed_udf.step().unwrap();
    let before = executed_udf.get_state();
    assert_eq!(
        executed_udf.step(),
        Err(CortexM3Error::UnknownInstruction16 {
            raw: 0xde01,
            pc: ORIGIN + 4,
        })
    );
    assert_eq!(executed_udf.get_state(), before);
}

#[test]
fn compare_and_branch_zero_uses_forward_thumb_target() {
    let mut zero = loaded(&[cbz(0, 0), movs(1, 1), bkpt(0)]);
    zero.step().unwrap();
    assert_eq!(zero.get_state().pc, ORIGIN + 4);
    assert_eq!(zero.read_register(1).unwrap(), 0);

    let mut nonzero = loaded(&[cbnz(0, 0), movs(1, 1), bkpt(0)]);
    nonzero.write_register(0, 9).unwrap();
    nonzero.step().unwrap();
    assert_eq!(nonzero.get_state().pc, ORIGIN + 4);
}

#[test]
fn thumb2_wide_data_and_memory_operations_execute() {
    let lo = movw(0, 0x5678);
    let hi = movt(0, 0x1234);
    let unsigned = udiv(3, 1, 2);
    let signed = sdiv(4, 5, 6);
    let mut words = Vec::new();
    words.extend(lo);
    words.extend(hi);
    words.extend(unsigned);
    words.extend(signed);
    words.extend([0xf8c7, 0x0001, 0xf8d7, 0x8001, bkpt(0)]);
    let mut simulator = loaded(&words);
    simulator.write_register(1, 100).unwrap();
    simulator.write_register(2, 4).unwrap();
    simulator.write_register(5, (-21_i32) as u32).unwrap();
    simulator.write_register(6, 3).unwrap();
    simulator.write_register(7, 0x300).unwrap();
    for _ in 0..6 {
        simulator.step().unwrap();
    }
    assert_eq!(simulator.read_register(0).unwrap(), 0x1234_5678);
    assert_eq!(simulator.read_register(3).unwrap(), 25);
    assert_eq!(simulator.read_register(4).unwrap(), (-7_i32) as u32);
    assert_eq!(simulator.read_register(8).unwrap(), 0x1234_5678);
    assert_eq!(
        &simulator.get_state().memory[0x301..0x305],
        &[0x78, 0x56, 0x34, 0x12]
    );

    let mut sp = loaded(&[0xf8cd, 0xd004, 0xf8dd, 0xd004]);
    let mut state = sp.get_state();
    state.msp = 0x300;
    sp.restore(state).unwrap();
    sp.step().unwrap();
    assert_eq!(sp.read_memory(0x304, 4).unwrap(), 0x300);
    sp.write_memory(0x304, 4, 0x340).unwrap();
    sp.step().unwrap();
    assert_eq!(sp.get_state().msp, 0x340);
}

#[test]
fn exclusive_monitor_commit_failure_and_clear_are_explicit() {
    let load = ldrex(0, 1, 0);
    let store = strex(3, 2, 1, 0);
    let clear = clrex();
    let mut words = Vec::new();
    words.extend(load);
    words.extend(store);
    words.extend(clear);
    let mut simulator = loaded(&words);
    simulator.write_register(1, 0x300).unwrap();
    simulator.write_register(2, 0xaabb_ccdd).unwrap();
    simulator.write_memory(0x300, 4, 0x1122_3344).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "ldrex");
    assert_eq!(simulator.get_state().exclusive_address, Some(0x300));
    assert_eq!(simulator.step().unwrap().mnemonic, "strex");
    assert_eq!(simulator.read_register(3).unwrap(), 0);
    assert_eq!(simulator.read_memory(0x300, 4).unwrap(), 0xaabb_ccdd);
    assert_eq!(simulator.get_state().exclusive_address, None);

    let mut failed = loaded(&[load[0], load[1], store[0], store[1]]);
    failed.write_register(1, 0x300).unwrap();
    failed.write_register(2, 7).unwrap();
    failed.write_memory(0x300, 4, 1).unwrap();
    failed.step().unwrap();
    failed.write_memory(0x304, 4, 2).unwrap();
    failed.step().unwrap();
    assert_eq!(failed.read_register(3).unwrap(), 1);
    assert_eq!(failed.read_memory(0x300, 4).unwrap(), 1);

    let mut cleared = loaded(&[load[0], load[1], clear[0], clear[1]]);
    cleared.write_register(1, 0x300).unwrap();
    cleared.step().unwrap();
    cleared.step().unwrap();
    assert_eq!(cleared.get_state().exclusive_address, None);

    let mut misaligned = loaded(&[load[0], load[1]]);
    misaligned.write_register(1, 0x301).unwrap();
    let before = misaligned.get_state();
    assert!(matches!(
        misaligned.step(),
        Err(CortexM3Error::MisalignedData { .. })
    ));
    assert_eq!(misaligned.get_state(), before);

    let sp_load = ldrex(0, 13, 0);
    let sp_store = strex(1, 0, 13, 0);
    let mut stack_program = Vec::new();
    stack_program.extend(sp_load);
    stack_program.extend(sp_store);
    let mut stack_based = loaded(&stack_program);
    let mut state = stack_based.get_state();
    state.msp = 0x300;
    state.memory[0x300..0x304].copy_from_slice(&7_u32.to_le_bytes());
    stack_based.restore(state).unwrap();
    stack_based.step().unwrap();
    assert_eq!(stack_based.read_register(0).unwrap(), 7);
    assert_eq!(stack_based.get_state().exclusive_address, Some(0x300));
    stack_based.step().unwrap();
    assert_eq!(stack_based.read_register(1).unwrap(), 0);

    for invalid in [strex(2, 2, 1, 0), strex(1, 2, 1, 0)] {
        let mut aliased = loaded(&invalid);
        aliased.write_register(1, 0x300).unwrap();
        let before = aliased.get_state();
        assert!(matches!(
            aliased.step(),
            Err(CortexM3Error::UnknownInstruction32 { .. })
        ));
        assert_eq!(aliased.get_state(), before);
    }
}

#[test]
fn table_branches_and_extended_masks_execute() {
    let branch = tbb(0, 1);
    let mut simulator = loaded(&[branch[0], branch[1], nop(), nop(), bkpt(0)]);
    simulator.write_register(0, 0x300).unwrap();
    simulator.write_register(1, 1).unwrap();
    simulator.write_memory(0x301, 1, 2).unwrap();
    assert_eq!(simulator.step().unwrap().mnemonic, "tbb");
    assert_eq!(simulator.get_state().pc, ORIGIN + 8);

    let pc_branch = tbb(15, 0);
    let mut pc_relative = loaded(&[pc_branch[0], pc_branch[1], nop(), nop(), bkpt(0)]);
    pc_relative.write_register(0, 0).unwrap();
    pc_relative.write_memory(ORIGIN + 4, 1, 2).unwrap();
    pc_relative.step().unwrap();
    assert_eq!(pc_relative.get_state().pc, ORIGIN + 8);

    let mut unaligned_pc_base = loaded(&[nop(), pc_branch[0], pc_branch[1], nop(), nop(), bkpt(0)]);
    unaligned_pc_base.step().unwrap();
    unaligned_pc_base.write_register(0, 0).unwrap();
    unaligned_pc_base.write_memory(ORIGIN + 6, 1, 2).unwrap();
    unaligned_pc_base.step().unwrap();
    assert_eq!(unaligned_pc_base.get_state().pc, ORIGIN + 10);

    let lr_branch = tbh(14, 0);
    let mut lr_relative = loaded(&[lr_branch[0], lr_branch[1], nop(), nop(), bkpt(0)]);
    lr_relative.write_register(0, 1).unwrap();
    lr_relative.write_register(14, 0x300).unwrap();
    lr_relative.write_memory(0x302, 2, 2).unwrap();
    lr_relative.step().unwrap();
    assert_eq!(lr_relative.get_state().pc, ORIGIN + 8);

    let lr_index_branch = tbb(0, 14);
    let mut lr_index = loaded(&[
        lr_index_branch[0],
        lr_index_branch[1],
        nop(),
        nop(),
        bkpt(0),
    ]);
    lr_index.write_register(0, 0x300).unwrap();
    lr_index.write_register(14, 1).unwrap();
    lr_index.write_memory(0x301, 1, 2).unwrap();
    lr_index.step().unwrap();
    assert_eq!(lr_index.get_state().pc, ORIGIN + 8);

    let set_basepri = msr(0, 17);
    let set_faultmask = msr(1, 19);
    let get_basepri = mrs(2, 17);
    let get_faultmask = mrs(3, 19);
    let mut words = Vec::new();
    for encoded in [set_basepri, set_faultmask, get_basepri, get_faultmask] {
        words.extend(encoded);
    }
    let mut masks = loaded(&words);
    masks.write_register(0, 0x80).unwrap();
    masks.write_register(1, 1).unwrap();
    for _ in 0..4 {
        masks.step().unwrap();
    }
    assert_eq!(masks.get_state().basepri, 0x80);
    assert!(masks.get_state().faultmask);
    assert_eq!(masks.read_register(2).unwrap(), 0x80);
    assert_eq!(masks.read_register(3).unwrap(), 1);
}

#[test]
fn run_and_execute_are_bounded_and_transactional() {
    let mut simulator = loaded(&[movs(0, 1), nop()]);
    let before = simulator.get_state();
    assert_eq!(
        simulator.run(2),
        Err(CortexM3Error::StepLimitExceeded { limit: 2 })
    );
    assert_eq!(simulator.get_state(), before);
    assert_eq!(
        simulator.run(cortex_m3_gatelevel::MAX_RETAINED_STEPS + 1),
        Err(CortexM3Error::RetentionLimitExceeded {
            requested: cortex_m3_gatelevel::MAX_RETAINED_STEPS + 1,
            limit: cortex_m3_gatelevel::MAX_RETAINED_STEPS,
        })
    );

    let mut fresh = CortexM3GateLevel::new();
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
        fresh.execute(&[0], 1, cortex_m3_gatelevel::MAX_RETAINED_STEPS + 1,),
        Err(CortexM3Error::RetentionLimitExceeded {
            requested: cortex_m3_gatelevel::MAX_RETAINED_STEPS + 1,
            limit: cortex_m3_gatelevel::MAX_RETAINED_STEPS,
        })
    );
}

#[test]
fn every_error_has_display_text() {
    let errors = [
        CortexM3Error::EmptyProgram,
        CortexM3Error::InvalidRegister { index: 16 },
        CortexM3Error::MisalignedFetch { pc: 1 },
        CortexM3Error::UnknownInstruction16 { raw: 0, pc: 0 },
        CortexM3Error::UnsupportedExceptionReturn {
            target: 0xffff_fff9,
        },
        CortexM3Error::AllocationFailed,
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}
