use zpu_simulator::{
    decode, offset5, AccessKind, BusTransaction, Operation, ZpuError, ZpuSimulator, EXTERNAL_WORDS,
    INTERNAL_MEMORY_BYTES, INTERRUPT_VECTOR, RESET_SP,
};

fn loaded(raw: u8) -> ZpuSimulator {
    let mut cpu = ZpuSimulator::new();
    cpu.load(&[raw], 0).unwrap();
    cpu
}

fn expected_operation(raw: u8) -> Operation {
    if raw >= 0x80 {
        Operation::Im
    } else if (0x60..=0x7f).contains(&raw) {
        Operation::LoadSp
    } else if (0x40..=0x5f).contains(&raw) {
        Operation::StoreSp
    } else if (0x20..=0x3f).contains(&raw) {
        Operation::Emulate
    } else if (0x10..=0x1f).contains(&raw) {
        Operation::AddSp
    } else {
        match raw {
            0x00 => Operation::Break,
            0x02 => Operation::PushSp,
            0x04 => Operation::PopPc,
            0x05 => Operation::Add,
            0x06 => Operation::And,
            0x07 => Operation::Or,
            0x08 => Operation::Load,
            0x09 => Operation::Not,
            0x0a => Operation::Flip,
            0x0c => Operation::Store,
            0x0d => Operation::PopSp,
            0x01 | 0x03 | 0x0b | 0x0e | 0x0f => Operation::Nop,
            _ => unreachable!(),
        }
    }
}

#[test]
fn all_256_bytes_have_the_independent_frozen_classification() {
    for raw in 0_u8..=u8::MAX {
        let decoded = decode(raw);
        assert_eq!(decoded.operation, expected_operation(raw), "raw={raw:#04x}");
        if matches!(decoded.operation, Operation::LoadSp | Operation::StoreSp) {
            assert_eq!(decoded.parameter, (raw & 0x0f) | ((!raw) & 0x10));
            assert_eq!(decoded.parameter, offset5(raw));
        }
    }
}

#[test]
fn reset_load_snapshot_restore_and_helper_contracts_are_complete() {
    let mut cpu = ZpuSimulator::new();
    let reset = cpu.snapshot().unwrap();
    assert_eq!(reset.pc, 0);
    assert_eq!(reset.sp, RESET_SP);
    assert_eq!(reset.internal_memory.len(), INTERNAL_MEMORY_BYTES);
    assert_eq!(reset.external_bus_words.len(), EXTERNAL_WORDS);

    assert_eq!(cpu.load(&[], 0), Err(ZpuError::EmptyProgram));
    assert_eq!(cpu.snapshot().unwrap(), reset);
    assert_eq!(
        cpu.load(&[0; 2], 0x7fff),
        Err(ZpuError::ProgramOutOfRange {
            origin: 0x7fff,
            length: 2,
        })
    );
    cpu.load(&[0x0b], 0x123).unwrap();
    assert_eq!(cpu.snapshot().unwrap().pc, 0x123);
    cpu.write_internal_word(0x10, 0x1122_3344).unwrap();
    cpu.write_external_word(0x8000, 0x5566_7788).unwrap();
    assert_eq!(cpu.read_internal_word(0x10).unwrap(), 0x1122_3344);
    assert_eq!(cpu.read_external_word(0x8000).unwrap(), 0x5566_7788);
    assert_eq!(
        cpu.read_internal_word(2),
        Err(ZpuError::MisalignedAddress { address: 2 })
    );
    assert_eq!(
        cpu.read_external_word(0x7ffc),
        Err(ZpuError::AddressOutOfRange { address: 0x7ffc })
    );

    let saved = cpu.snapshot().unwrap();
    cpu.reset();
    cpu.restore(saved.clone()).unwrap();
    assert_eq!(cpu.snapshot().unwrap(), saved);
    let mut invalid = saved;
    invalid.sp = 3;
    assert!(matches!(
        cpu.restore(invalid),
        Err(ZpuError::InvalidState { .. })
    ));
}

#[test]
fn immediate_boundaries_and_idim_concatenation_match_the_architecture() {
    for raw in 0x80_u8..=0xff {
        let mut cpu = loaded(raw);
        let trace = cpu.step().unwrap();
        let expected = ((i32::from(raw & 0x7f) << 25) >> 25) as u32;
        assert_eq!(trace.sp_after, RESET_SP - 4);
        assert_eq!(trace.top_after, expected);
        assert!(trace.idim_after);
    }

    let mut cpu = ZpuSimulator::new();
    cpu.load(&[0x81, 0xff, 0x0b], 0).unwrap();
    cpu.step().unwrap();
    let second = cpu.step().unwrap();
    assert_eq!(second.top_after, 0x0000_00ff);
    assert_eq!(second.sp_after, RESET_SP - 4);
    assert!(second.idim_after);
    assert!(!cpu.step().unwrap().idim_after);
}

#[test]
fn stack_parameter_families_cover_every_encoded_offset() {
    for field in 0_u8..32 {
        let load_raw = 0x60 | field;
        let mut load = ZpuSimulator::new();
        load.load(&[load_raw], 0x200).unwrap();
        let address = (((u32::from(RESET_SP) >> 2) + u32::from(offset5(load_raw))) & 0x1fff) << 2;
        load.write_internal_word(address, 0x0102_0000 | u32::from(field))
            .unwrap();
        let trace = load.step().unwrap();
        assert_eq!(trace.top_after, 0x0102_0000 | u32::from(field));
        assert_eq!(trace.internal_access.unwrap().kind, AccessKind::Read);

        let store_raw = 0x40 | field;
        let mut store = ZpuSimulator::new();
        store.load(&[store_raw], 0x200).unwrap();
        store
            .write_internal_word(u32::from(RESET_SP), 0xa500_0000 | u32::from(field))
            .unwrap();
        store.step().unwrap();
        assert_eq!(
            store.read_internal_word(address).unwrap(),
            0xa500_0000 | u32::from(field)
        );
        assert_eq!(store.snapshot().unwrap().sp, 0x7ffc);
    }

    for field in 0_u8..16 {
        let mut add = ZpuSimulator::new();
        add.load(&[0x10 | field], 0x200).unwrap();
        add.write_internal_word(u32::from(RESET_SP), 7).unwrap();
        let address = (((u32::from(RESET_SP) >> 2) + u32::from(field)) & 0x1fff) << 2;
        if field != 0 {
            add.write_internal_word(address, 11).unwrap();
        }
        assert_eq!(
            add.step().unwrap().top_after,
            if field == 0 { 14 } else { 18 }
        );
    }
}

#[test]
fn short_alu_flip_stack_and_control_orders_match() {
    for (raw, expected) in [(0x05, 17), (0x06, 4), (0x07, 13)] {
        let mut cpu = loaded(raw);
        cpu.write_internal_word(u32::from(RESET_SP), 5).unwrap();
        cpu.write_internal_word(u32::from(RESET_SP + 4), 12)
            .unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(trace.top_after, expected);
        assert_eq!(trace.sp_after, 0x7ffc);
    }

    let mut not = loaded(0x09);
    not.write_internal_word(u32::from(RESET_SP), 0x0f0f_0000)
        .unwrap();
    assert_eq!(not.step().unwrap().top_after, 0xf0f0_ffff);
    let mut flip = loaded(0x0a);
    flip.write_internal_word(u32::from(RESET_SP), 0x8000_0001)
        .unwrap();
    assert_eq!(flip.step().unwrap().top_after, 0x8000_0001);

    let mut push_sp = loaded(0x02);
    let push = push_sp.step().unwrap();
    assert_eq!(push.top_after, u32::from(RESET_SP));
    assert_eq!(push.sp_after, RESET_SP - 4);

    let mut pop_sp = loaded(0x0d);
    pop_sp
        .write_internal_word(u32::from(RESET_SP), 0x1237)
        .unwrap();
    assert_eq!(pop_sp.step().unwrap().sp_after, 0x1234);

    let mut emulate = loaded(0x3f);
    let emu = emulate.step().unwrap();
    assert_eq!(emu.pc_after, 31 << 5);
    assert_eq!(emu.top_after, 1);

    let mut pop_pc = loaded(0x04);
    pop_pc
        .write_internal_word(u32::from(RESET_SP), 0xffff_8123)
        .unwrap();
    assert_eq!(pop_pc.step().unwrap().pc_after, 0x0123);
}

#[test]
fn internal_and_external_load_store_are_big_endian_and_transactional() {
    let mut internal_load = loaded(0x08);
    internal_load
        .write_internal_word(u32::from(RESET_SP), 0x0000_0103)
        .unwrap();
    internal_load
        .write_internal_word(0x100, 0x1122_3344)
        .unwrap();
    let trace = internal_load.step().unwrap();
    assert_eq!(trace.top_after, 0x1122_3344);
    assert_eq!(trace.internal_access.unwrap().address, 0x100);

    let mut external_load = loaded(0x08);
    external_load
        .write_internal_word(u32::from(RESET_SP), 0xffff_8003)
        .unwrap();
    external_load
        .write_external_word(0x8000, 0xaabb_ccdd)
        .unwrap();
    assert_eq!(
        external_load.step().unwrap().bus_transaction,
        Some(BusTransaction::Read {
            address: 0x8000,
            value: 0xaabb_ccdd,
        })
    );

    for (address, external) in [(0x0103_u32, false), (0x8003, true)] {
        let mut store = loaded(0x0c);
        store
            .write_internal_word(u32::from(RESET_SP), address)
            .unwrap();
        store
            .write_internal_word(u32::from(RESET_SP + 4), 0xdead_beef)
            .unwrap();
        let trace = store.step().unwrap();
        assert_eq!(trace.sp_after, 0);
        if external {
            assert_eq!(store.read_external_word(0x8000).unwrap(), 0xdead_beef);
            assert_eq!(
                trace.bus_transaction,
                Some(BusTransaction::Write {
                    address: 0x8000,
                    value: 0xdead_beef,
                })
            );
        } else {
            assert_eq!(store.read_internal_word(0x100).unwrap(), 0xdead_beef);
        }
    }
}

#[test]
fn interrupt_is_preflighted_masked_by_idim_and_level_rearmed() {
    let mut cpu = ZpuSimulator::new();
    let mut image = vec![0x0b; 0x22];
    image[0] = 0x81;
    cpu.load(&image, 0).unwrap();
    cpu.set_interrupt_level(true);
    let accepted = cpu.step().unwrap();
    assert_eq!(accepted.raw, None);
    assert_eq!(accepted.pc_after, INTERRUPT_VECTOR);
    assert_eq!(accepted.top_after, 0);
    assert!(accepted.in_interrupt_after);

    let sustained = cpu.step().unwrap();
    assert_eq!(sustained.raw, Some(0x0b));
    cpu.set_interrupt_level(false);
    assert!(!cpu.step().unwrap().in_interrupt_after);

    let mut masked = ZpuSimulator::new();
    masked.load(&[0x81, 0x82, 0x0b], 0).unwrap();
    masked.step().unwrap();
    masked.set_interrupt_level(true);
    assert_eq!(masked.step().unwrap().raw, Some(0x82));
    assert!(masked.snapshot().unwrap().idim);

    let mut bad_fetch = loaded(0x0b);
    bad_fetch.step().unwrap();
    bad_fetch.set_interrupt_level(true);
    let before = bad_fetch.snapshot().unwrap();
    assert_eq!(
        bad_fetch.step(),
        Err(ZpuError::FetchOutsideProgram { pc: 1 })
    );
    assert_eq!(bad_fetch.snapshot().unwrap(), before);
}

#[test]
fn break_run_execute_and_limits_are_atomic() {
    let mut cpu = ZpuSimulator::new();
    cpu.load(&[0x0b, 0x00, 0x0b], 0).unwrap();
    let result = cpu.run(2).unwrap();
    assert!(result.break_asserted);
    assert_eq!(result.steps, 2);
    assert_eq!(cpu.run(1).unwrap().steps, 0);
    assert!(cpu.step().unwrap().break_before);

    let mut limit = loaded(0x0b);
    let before = limit.snapshot().unwrap();
    assert_eq!(limit.run(0), Err(ZpuError::StepLimit { limit: 0 }));
    assert_eq!(limit.snapshot().unwrap(), before);
    assert_eq!(
        limit.run(65),
        Err(ZpuError::TraceLimit {
            requested: 65,
            limit: 64
        })
    );
    assert_eq!(limit.snapshot().unwrap(), before);

    let mut execute = ZpuSimulator::new();
    execute.write_internal_word(0, 0xfeed_face).unwrap();
    let before = execute.snapshot().unwrap();
    assert!(execute.execute(&[0x0b], 0, 1).is_err());
    assert_eq!(execute.snapshot().unwrap(), before);
}

#[test]
fn deterministic_allocation_failures_preserve_exact_entry_state() {
    assert_eq!(
        ZpuSimulator::try_new_with_allocation_failure(Some(0)).unwrap_err(),
        ZpuError::AllocationFailed
    );
    assert_eq!(
        ZpuSimulator::try_new_with_allocation_failure(Some(1)).unwrap_err(),
        ZpuError::AllocationFailed
    );
    assert_eq!(
        ZpuSimulator::try_new().unwrap().snapshot().unwrap().sp,
        RESET_SP
    );

    let snapshot = loaded(0x0b);
    for checkpoint in 0..=1 {
        let before = snapshot.snapshot().unwrap();
        snapshot.fail_allocation_after(Some(checkpoint));
        assert_eq!(snapshot.snapshot(), Err(ZpuError::AllocationFailed));
        snapshot.fail_allocation_after(None);
        assert_eq!(snapshot.snapshot().unwrap(), before);
    }

    let mut cpu = loaded(0x0b);
    let before = cpu.snapshot().unwrap();
    for checkpoint in 0..=4 {
        cpu.fail_allocation_after(Some(checkpoint));
        assert_eq!(cpu.step(), Err(ZpuError::AllocationFailed));
        cpu.fail_allocation_after(None);
        assert_eq!(cpu.snapshot().unwrap(), before);
    }

    let mut reserve = ZpuSimulator::new();
    reserve.load(&[0x0b, 0x00], 0).unwrap();
    let before = reserve.snapshot().unwrap();
    reserve.fail_allocation_after(Some(2));
    assert_eq!(reserve.run(2), Err(ZpuError::AllocationFailed));
    reserve.fail_allocation_after(None);
    assert_eq!(reserve.snapshot().unwrap(), before);

    let mut run = ZpuSimulator::new();
    run.load(&[0x0b, 0x00], 0).unwrap();
    let before = run.snapshot().unwrap();
    run.fail_candidate_after(Some(0));
    assert_eq!(run.run(2), Err(ZpuError::AllocationFailed));
    assert_eq!(run.snapshot().unwrap(), before);

    let mut execute = loaded(0x02);
    let before = execute.snapshot().unwrap();
    execute.fail_candidate_after(Some(0));
    assert_eq!(
        execute.execute(&[0x0b, 0x00], 0, 2).unwrap_err().source,
        ZpuError::AllocationFailed
    );
    assert_eq!(execute.snapshot().unwrap(), before);
}
