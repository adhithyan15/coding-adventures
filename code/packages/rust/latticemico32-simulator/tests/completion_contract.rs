use latticemico32_simulator::{
    decode, opcode_class, BusTransactionKind, Lm32Error, Lm32Simulator, Lm32State, StopReason, CFG,
    CFG2, EXCEPTION_BREAKPOINT, EXCEPTION_DATA_BUS, EXCEPTION_DIVIDE_BY_ZERO,
    EXCEPTION_INSTRUCTION_BUS, EXCEPTION_INTERRUPT, EXCEPTION_SYSTEM_CALL, MAX_RETAINED_STEPS,
    MEMORY_SIZE,
};

const fn imm(op: u32, ry: u32, rx: u32, value: u16) -> u32 {
    (op << 26) | (ry << 21) | (rx << 16) | value as u32
}
const fn reg(op: u32, ry: u32, rz: u32, rx: u32) -> u32 {
    (op << 26) | (ry << 21) | (rz << 16) | (rx << 11)
}
const fn branch(op: u32, rx: u32, ry: u32, offset: u16) -> u32 {
    (op << 26) | (rx << 21) | (ry << 16) | offset as u32
}
const fn csr_read(index: u32, destination: u32) -> u32 {
    (0x24 << 26) | (index << 21) | (destination << 11)
}
const fn csr_write(index: u32, source: u32) -> u32 {
    (0x34 << 26) | (index << 21) | (source << 16)
}
const fn branch_register(register: u32) -> u32 {
    (0x30 << 26) | (register << 21)
}
const fn immediate_control(op: u32, offset_words: i32) -> u32 {
    (op << 26) | ((offset_words as u32) & 0x03ff_ffff)
}

fn bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_be_bytes()).collect()
}

fn loaded(words: &[u32]) -> Lm32Simulator {
    let mut cpu = Lm32Simulator::new();
    cpu.load(&bytes(words), 0x100).unwrap();
    cpu
}

fn snapshot(cpu: &Lm32Simulator) -> Lm32State {
    cpu.snapshot().unwrap()
}

#[test]
fn all_primary_opcodes_have_a_stable_classification() {
    for opcode in 0_u8..64 {
        let class = opcode_class(opcode);
        assert!(matches!(
            class,
            "implemented" | "unsupported" | "reserved" | "canonical-only"
        ));
        let raw = u32::from(opcode) << 26;
        match opcode {
            0x27 | 0x35 => assert!(matches!(
                decode(raw),
                Err(latticemico32_simulator::DecodeError::Unsupported)
            )),
            0x2a | 0x2b | 0x33 => assert!(decode(raw).is_err()),
            _ => assert!(decode(raw).is_ok(), "opcode {opcode:#04x}"),
        }
    }
    assert_eq!(opcode_class(64), "out-of-range");
    assert_eq!(decode(0xac00_0002).unwrap().mnemonic, "break");
    assert_eq!(decode(0xac00_0007).unwrap().mnemonic, "scall");
    assert!(decode(0xac00_0003).is_err());
    assert!(decode(reg(0x2d, 1, 2, 3) | 1).is_err());
}

#[test]
fn load_reset_and_restore_cover_complete_state() {
    let mut cpu = loaded(&[imm(0x0d, 0, 1, 7)]);
    cpu.write_register(0, 9).unwrap();
    cpu.write_memory(0xfffc, 4, 0x1234_5678).unwrap();
    cpu.set_interrupt_lines(0x55aa);
    let before_reset = snapshot(&cpu);
    cpu.reset();
    let reset = snapshot(&cpu);
    assert_eq!(reset.pc, 0);
    assert_eq!(reset.gpr, [0; 32]);
    assert_eq!(reset.origin, Some(0x100));
    assert_eq!(reset.length, 4);
    assert_eq!(reset.memory, before_reset.memory);

    let mut restored = before_reset;
    restored.pc = 0x100;
    restored.eba = 0x100;
    restored.deba = 0x200;
    restored.dc_re = true;
    cpu.restore(restored.clone()).unwrap();
    assert_eq!(snapshot(&cpu), restored);

    let mut invalid = restored;
    invalid.memory.pop();
    let before = snapshot(&cpu);
    assert!(matches!(
        cpu.restore(invalid),
        Err(Lm32Error::InvalidState { .. })
    ));
    assert_eq!(snapshot(&cpu), before);
}

#[test]
fn image_and_memory_validation_is_atomic() {
    let mut cpu = loaded(&[0xac00_0007]);
    let before = snapshot(&cpu);
    assert_eq!(cpu.load(&[], 0), Err(Lm32Error::EmptyProgram));
    assert!(matches!(
        cpu.load(&[0; 3], 0),
        Err(Lm32Error::UnalignedProgram { .. })
    ));
    assert!(matches!(
        cpu.load(&[0; 4], 0xffff),
        Err(Lm32Error::UnalignedProgram { .. })
    ));
    assert!(matches!(
        cpu.load(&vec![0; MEMORY_SIZE + 4], 0),
        Err(Lm32Error::ProgramTooLarge { .. })
    ));
    assert!(matches!(
        cpu.load(&[0; 8], 0xfffc),
        Err(Lm32Error::ProgramOutOfRange { .. })
    ));
    assert_eq!(snapshot(&cpu), before);
    assert_eq!(cpu.read_memory(0xfffc, 4).unwrap(), 0);
    assert!(matches!(
        cpu.read_memory(1, 2),
        Err(Lm32Error::MisalignedData { .. })
    ));
    assert!(matches!(
        cpu.read_memory(0x1_0000, 1),
        Err(Lm32Error::MemoryOutOfRange { .. })
    ));
    assert!(matches!(
        cpu.write_register(32, 0),
        Err(Lm32Error::InvalidRegister { .. })
    ));
}

#[test]
fn arithmetic_logic_shift_and_extension_matrix() {
    let program = [
        imm(0x0d, 0, 1, 5),
        imm(0x0d, 0, 2, 3),
        reg(0x2d, 1, 2, 3),
        reg(0x32, 1, 2, 4),
        reg(0x22, 1, 2, 5),
        reg(0x23, 1, 2, 6),
        reg(0x31, 1, 2, 7),
        imm(0x08, 3, 8, 6),
        imm(0x0e, 4, 9, 8),
        reg(0x26, 8, 9, 10),
        reg(0x29, 8, 9, 11),
        reg(0x21, 8, 9, 12),
        imm(0x0f, 2, 13, 31),
        imm(0x00, 13, 14, 31),
        imm(0x05, 13, 15, 31),
        reg(0x2c, 15, 0, 16),
        reg(0x37, 15, 0, 17),
    ];
    let mut cpu = loaded(&program);
    for _ in program {
        cpu.step().unwrap();
    }
    let s = snapshot(&cpu);
    assert_eq!(
        (s.gpr[3], s.gpr[4], s.gpr[5], s.gpr[6], s.gpr[7]),
        (8, 2, 15, 1, 2)
    );
    assert_eq!(s.gpr[8], 0);
    assert_eq!(s.gpr[9], 10);
    assert_eq!(s.gpr[10], 10);
    assert_eq!(s.gpr[11], !10);
    assert_eq!(s.gpr[12], !10);
    assert_eq!(s.gpr[13], 0x8000_0000);
    assert_eq!(s.gpr[14], 1);
    assert_eq!(s.gpr[15], 0xffff_ffff);
    assert_eq!(s.gpr[16], 0xffff_ffff);
    assert_eq!(s.gpr[17], 0xffff_ffff);
}

#[test]
fn immediate_high_and_comparison_boundaries() {
    let program = [
        imm(0x1e, 0, 1, 0x8000),
        imm(0x18, 1, 2, 0xffff),
        imm(0x19, 1, 3, 0),
        imm(0x1f, 1, 4, 0),
        imm(0x1a, 1, 5, 0),
        imm(0x1b, 1, 6, 0x8000),
        imm(0x1c, 1, 7, 0xffff),
        imm(0x1d, 1, 8, 0xffff),
        reg(0x39, 1, 1, 9),
        reg(0x3f, 1, 0, 10),
        reg(0x3a, 0, 1, 11),
        reg(0x3b, 1, 1, 12),
        reg(0x3c, 1, 1, 13),
        reg(0x3d, 1, 0, 14),
    ];
    let mut cpu = loaded(&program);
    cpu.write_register(0, 0).unwrap();
    for _ in program {
        cpu.step().unwrap();
    }
    let s = snapshot(&cpu);
    assert_eq!(s.gpr[1], 0x8000_0000);
    assert_eq!(s.gpr[2], 0x8000_0000);
    assert_eq!(&s.gpr[3..=14], &[0, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1]);
}

#[test]
fn big_endian_memory_sign_extension_and_self_modification() {
    let program = [
        imm(0x0d, 0, 1, 0x200),
        imm(0x0d, 0, 2, 0x80ff),
        imm(0x0c, 1, 2, 0),
        imm(0x03, 1, 2, 2),
        imm(0x16, 1, 2, 4),
        imm(0x04, 1, 3, 0),
        imm(0x10, 1, 4, 0),
        imm(0x07, 1, 5, 2),
        imm(0x0b, 1, 6, 2),
        imm(0x0a, 1, 7, 4),
        0,
    ];
    let mut cpu = loaded(&program);
    for _ in 0..10 {
        cpu.step().unwrap();
    }
    let s = snapshot(&cpu);
    assert_eq!(
        &s.memory[0x200..0x208],
        &[0xff, 0, 0x80, 0xff, 0xff, 0xff, 0x80, 0xff]
    );
    assert_eq!(s.gpr[3], 0xffff_ffff);
    assert_eq!(s.gpr[4], 0xff);
    assert_eq!(s.gpr[5], 0xffff_80ff);
    assert_eq!(s.gpr[6], 0x80ff);
    assert_eq!(s.gpr[7], 0xffff_80ff);

    let replacement = imm(0x0d, 0, 9, 77);
    cpu.write_memory(s.pc, 4, replacement).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(9).unwrap(), 77);
}

#[test]
fn data_alignment_rejects_and_bus_range_enters_exception() {
    let mut misaligned = loaded(&[imm(0x0a, 1, 2, 0)]);
    misaligned.write_register(1, 1).unwrap();
    let before = snapshot(&misaligned);
    assert!(matches!(
        misaligned.step(),
        Err(Lm32Error::MisalignedData { .. })
    ));
    assert_eq!(snapshot(&misaligned), before);

    let mut bus = loaded(&[imm(0x0a, 1, 2, 0)]);
    bus.write_register(1, 0x1_0000).unwrap();
    let trace = bus.step().unwrap();
    assert_eq!(trace.exception.unwrap().id, EXCEPTION_DATA_BUS);
    assert_eq!(trace.transactions[1].kind, BusTransactionKind::DataRead);
    assert!(!trace.transactions[1].success);
    assert_eq!(trace.state_after.gpr[30], 0x100);
}

#[test]
fn branch_predicates_calls_and_returns_cover_both_directions() {
    for (op, a, b, expected) in [
        (0x11, 1, 1, true),
        (0x17, 1, 2, true),
        (0x12, u32::MAX, 0, false),
        (0x13, u32::MAX, u32::MAX, true),
        (0x15, 2, 1, true),
        (0x14, 1, 2, false),
    ] {
        let mut cpu = loaded(&[branch(op, 1, 2, 2), 0, 0]);
        cpu.write_register(1, a).unwrap();
        cpu.write_register(2, b).unwrap();
        let trace = cpu.step().unwrap();
        assert_eq!(trace.branch, Some((expected, 0x108)));
        assert_eq!(trace.pc_after, if expected { 0x108 } else { 0x104 });
    }

    let mut calli = loaded(&[immediate_control(0x3e, 2), 0, 0]);
    calli.step().unwrap();
    assert_eq!(calli.read_register(29).unwrap(), 0x104);
    assert_eq!(snapshot(&calli).pc, 0x108);

    let mut call = loaded(&[(0x36 << 26) | (1 << 21)]);
    call.write_register(1, 0x200).unwrap();
    call.step().unwrap();
    assert_eq!(snapshot(&call).pc, 0x200);
    assert_eq!(call.read_register(29).unwrap(), 0x104);

    let mut bad = loaded(&[branch_register(1)]);
    bad.write_register(1, 3).unwrap();
    let before = snapshot(&bad);
    assert!(matches!(
        bad.step(),
        Err(Lm32Error::MisalignedControlFlow { .. })
    ));
    assert_eq!(snapshot(&bad), before);
}

#[test]
fn csr_surface_dc_remap_and_manual_alignment_are_exact() {
    let program = [
        csr_read(6, 1),
        csr_read(10, 2),
        csr_write(0, 3),
        csr_write(1, 4),
        csr_write(7, 5),
        csr_write(9, 6),
        csr_write(8, 7),
        csr_read(0, 8),
        csr_read(1, 9),
        csr_read(7, 10),
        csr_read(9, 11),
    ];
    let mut cpu = loaded(&program);
    cpu.write_register(3, 7).unwrap();
    cpu.write_register(4, 0x55aa).unwrap();
    cpu.write_register(5, 0x1200).unwrap();
    cpu.write_register(6, 0x2380).unwrap();
    cpu.write_register(7, 2).unwrap();
    for _ in program {
        cpu.step().unwrap();
    }
    let s = snapshot(&cpu);
    assert_eq!(s.gpr[1], CFG);
    assert_eq!(s.gpr[2], CFG2);
    assert_eq!(s.gpr[8], 7);
    assert_eq!(s.gpr[9], 0x55aa);
    assert_eq!(s.gpr[10], 0x1200);
    assert_eq!(s.gpr[11], 0);
    assert!(s.dc_re); // misaligned DEBA write stores zero
    assert!(matches!(
        loaded(&[csr_read(8, 1)]).step(),
        Err(Lm32Error::UnsupportedCsr { .. })
    ));
    assert!(matches!(
        loaded(&[csr_write(6, 1)]).step(),
        Err(Lm32Error::UnsupportedCsr { .. })
    ));
}

#[test]
fn exceptions_save_pc_and_apply_debug_remapping() {
    for dc_re in [false, true] {
        let mut cpu = loaded(&[0xac00_0007]);
        let mut s = snapshot(&cpu);
        s.ie = true;
        s.eie = false;
        s.eba = 0x1000;
        s.deba = 0x2000;
        s.dc_re = dc_re;
        cpu.restore(s).unwrap();
        let trace = cpu.step().unwrap();
        let ex = trace.exception.unwrap();
        assert_eq!(ex.id, EXCEPTION_SYSTEM_CALL);
        assert_eq!(ex.base, if dc_re { 0x2000 } else { 0x1000 });
        assert_eq!(trace.state_after.gpr[30], 0x100);
        assert!(trace.state_after.eie);
        assert!(!trace.state_after.ie);
    }
    let mut debug = loaded(&[0xac00_0002]);
    let mut s = snapshot(&debug);
    s.ie = true;
    s.dc_re = false;
    s.eba = 0x1000;
    s.deba = 0x2000;
    debug.restore(s).unwrap();
    let ex = debug.step().unwrap().exception.unwrap();
    assert_eq!(ex.id, EXCEPTION_BREAKPOINT);
    assert_eq!(ex.base, 0x2000);
    assert_eq!(snapshot(&debug).gpr[31], 0x100);
    assert!(snapshot(&debug).bie);

    let mut div = loaded(&[reg(0x23, 1, 2, 3)]);
    div.write_register(1, 9).unwrap();
    let ex = div.step().unwrap().exception.unwrap();
    assert_eq!(ex.id, EXCEPTION_DIVIDE_BY_ZERO);
}

#[test]
fn eret_and_bret_restore_saved_enable_bits() {
    let mut eret = loaded(&[branch_register(30)]);
    let mut s = snapshot(&eret);
    s.gpr[30] = 0x200;
    s.eie = true;
    eret.restore(s).unwrap();
    let trace = eret.step().unwrap();
    assert_eq!(trace.mnemonic, "eret");
    assert!(snapshot(&eret).ie);
    assert_eq!(snapshot(&eret).pc, 0x200);

    let mut bret = loaded(&[branch_register(31)]);
    let mut s = snapshot(&bret);
    s.gpr[31] = 0x300;
    s.bie = true;
    bret.restore(s).unwrap();
    let trace = bret.step().unwrap();
    assert_eq!(trace.mnemonic, "bret");
    assert!(snapshot(&bret).ie);
    assert_eq!(snapshot(&bret).pc, 0x300);
}

#[test]
fn interrupts_latch_mask_clear_and_reassert_for_every_line() {
    for line in 0..32 {
        let bit = 1_u32 << line;
        let mut cpu = loaded(&[imm(0x0d, 0, 1, 1)]);
        let mut s = snapshot(&cpu);
        s.ie = true;
        s.im = bit;
        s.eba = 0x1000;
        cpu.restore(s).unwrap();
        cpu.set_interrupt_lines(bit);
        let trace = cpu.step().unwrap();
        let sample = trace.interrupt.unwrap();
        assert_eq!(sample.newly_latched, bit);
        assert_eq!(sample.accepted, bit);
        assert_eq!(trace.exception.unwrap().id, EXCEPTION_INTERRUPT);
        assert_eq!(trace.transactions.len(), 0);
    }

    let mut cpu = loaded(&[csr_write(2, 1), imm(0x0d, 0, 2, 1)]);
    let mut s = snapshot(&cpu);
    s.ip = 1;
    s.gpr[1] = 1;
    s.interrupt_lines = 1;
    cpu.restore(s).unwrap();
    cpu.step().unwrap();
    assert_eq!(snapshot(&cpu).ip, 0); // clear occurs after boundary re-latch
    cpu.step().unwrap();
    assert_eq!(snapshot(&cpu).ip, 1); // held line re-latches next boundary
}

#[test]
fn instruction_bus_error_and_fetch_contract_are_distinct() {
    let mut no_image = Lm32Simulator::new();
    assert_eq!(no_image.step(), Err(Lm32Error::NoProgramLoaded));
    let mut outside = loaded(&[0]);
    let mut s = snapshot(&outside);
    s.pc = 0x104;
    outside.restore(s).unwrap();
    assert_eq!(
        outside.step(),
        Err(Lm32Error::FetchOutsideProgram { pc: 0x104 })
    );

    let mut bus = loaded(&[0]);
    let mut s = snapshot(&bus);
    s.pc = 0xffff_fffc;
    bus.restore(s).unwrap();
    let trace = bus.step().unwrap();
    assert_eq!(trace.exception.unwrap().id, EXCEPTION_INSTRUCTION_BUS);
    assert!(!trace.transactions[0].success);
}

#[test]
fn run_and_execute_are_bounded_and_atomic() {
    let mut cpu = loaded(&[imm(0x0d, 0, 1, 1), 0xac00_0007]);
    let result = cpu.run(2).unwrap();
    assert_eq!(result.reason, StopReason::Exception(EXCEPTION_SYSTEM_CALL));
    assert_eq!(result.steps, 2);
    assert_eq!(result.final_state.gpr[1], 1);

    let mut exhausted = loaded(&[immediate_control(0x38, 0)]);
    let before = snapshot(&exhausted);
    assert_eq!(
        exhausted.run(2),
        Err(Lm32Error::StepLimitExceeded { limit: 2 })
    );
    assert_eq!(snapshot(&exhausted), before);
    assert_eq!(exhausted.run(0), Err(Lm32Error::ZeroStepBudget));
    assert!(matches!(
        exhausted.run(MAX_RETAINED_STEPS + 1),
        Err(Lm32Error::RetentionLimitExceeded { .. })
    ));

    let mut execute = loaded(&[0xac00_0002]);
    let original = snapshot(&execute);
    let result = execute.execute(&bytes(&[0xac00_0007]), 0x200, 1).unwrap();
    assert_eq!(result.reason, StopReason::Exception(EXCEPTION_SYSTEM_CALL));
    assert_eq!(snapshot(&execute).origin, Some(0x200));
    let error = execute
        .execute(&bytes(&[immediate_control(0x38, 0)]), 0x300, 1)
        .unwrap_err();
    assert_eq!(error.source, Lm32Error::StepLimitExceeded { limit: 1 });
    assert_ne!(snapshot(&execute), original);
    assert_eq!(snapshot(&execute).origin, Some(0x200));
}

#[test]
fn deterministic_allocation_failures_preserve_entry_state() {
    let mut cpu = loaded(&[imm(0x0d, 0, 1, 1), 0xac00_0007]);
    let before = snapshot(&cpu);
    cpu.fail_allocation_after(Some(0));
    assert_eq!(cpu.step(), Err(Lm32Error::AllocationFailed));
    assert_eq!(snapshot(&cpu), before);

    cpu.fail_after_candidate_steps(Some(0));
    assert_eq!(cpu.run(2), Err(Lm32Error::AllocationFailed));
    assert_eq!(snapshot(&cpu), before);

    cpu.fail_after_candidate_steps(Some(1));
    let looping = bytes(&[imm(0x0d, 0, 1, 1), immediate_control(0x38, -1)]);
    assert_eq!(
        cpu.execute(&looping, 0x200, 3).unwrap_err().source,
        Lm32Error::AllocationFailed
    );
    assert_eq!(snapshot(&cpu), before);
}

#[test]
fn mutable_r0_aliases_and_overlaps_are_not_special_cased() {
    let mut cpu = loaded(&[
        imm(0x0d, 0, 0, 1), // addi alias depends on old mutable r0
        reg(0x2d, 0, 0, 0), // overlap captures both sources first
        imm(0x09, 0, 1, 0), // not alias: xnori r1,r0,0
        reg(0x2d, 0, 0, 2), // mv alias only if software maintains r0=0
    ]);
    cpu.write_register(0, 5).unwrap();
    for _ in 0..4 {
        cpu.step().unwrap();
    }
    let s = snapshot(&cpu);
    assert_eq!(s.gpr[0], 12);
    assert_eq!(s.gpr[1], !12);
    assert_eq!(s.gpr[2], 24);
}

#[test]
fn malformed_unsupported_and_csr_faults_are_atomic() {
    for word in [0x9c00_0000, 0xd400_0000, 0xa800_0000, 0xcc00_0000] {
        let mut cpu = loaded(&[word]);
        let before = snapshot(&cpu);
        let error = cpu.step().unwrap_err();
        if matches!(word >> 26, 0x27 | 0x35) {
            assert!(matches!(error, Lm32Error::UnsupportedInstruction { .. }));
        } else {
            assert!(matches!(error, Lm32Error::UnknownInstruction { .. }));
        }
        assert_eq!(snapshot(&cpu), before);
    }

    let mut csr = loaded(&[csr_read(31, 1)]);
    let before = snapshot(&csr);
    assert!(matches!(
        csr.step(),
        Err(Lm32Error::UnsupportedCsr {
            index: 31,
            write: false
        })
    ));
    assert_eq!(snapshot(&csr), before);
}

#[test]
fn csr_reserved_bits_w1c_and_bit_eight_alignment_rules() {
    let mut cpu = loaded(&[
        csr_write(8, 1),
        csr_write(7, 2),
        csr_write(9, 3),
        csr_write(2, 4),
    ]);
    let mut s = snapshot(&cpu);
    s.gpr[1] = 0xffff_fffd; // DC stores only RE=0; every other bit is ignored.
    s.gpr[2] = 0x0100; // Manual-visible EBA bit 8 is retained.
    s.gpr[3] = 0x0300;
    s.gpr[4] = 0x00ff_00ff;
    s.ip = u32::MAX;
    cpu.restore(s).unwrap();
    for _ in 0..4 {
        cpu.step().unwrap();
    }
    let s = snapshot(&cpu);
    assert!(!s.dc_re);
    assert_eq!(s.eba, 0x100);
    assert_eq!(s.deba, 0x300);
    assert_eq!(s.ip, 0xff00_ff00);
}

#[test]
fn masked_disabled_and_priority_interrupt_boundaries_are_explicit() {
    let mut masked = loaded(&[imm(0x0d, 0, 1, 1)]);
    let mut s = snapshot(&masked);
    s.ie = true;
    s.im = 2;
    masked.restore(s).unwrap();
    masked.set_interrupt_lines(1);
    let trace = masked.step().unwrap();
    assert!(trace.exception.is_none());
    assert_eq!(trace.state_after.ip, 1);

    let mut disabled = loaded(&[0xac00_0007]);
    let mut s = snapshot(&disabled);
    s.im = 1;
    s.ie = false;
    disabled.restore(s).unwrap();
    disabled.set_interrupt_lines(1);
    let trace = disabled.step().unwrap();
    assert_eq!(trace.exception.unwrap().id, EXCEPTION_SYSTEM_CALL);
    assert_eq!(trace.state_after.ip, 1);

    let mut priority = loaded(&[0xffff_ffff]);
    let mut s = snapshot(&priority);
    s.im = 1;
    s.ie = true;
    priority.restore(s).unwrap();
    priority.set_interrupt_lines(1);
    let trace = priority.step().unwrap();
    assert_eq!(trace.exception.unwrap().id, EXCEPTION_INTERRUPT);
    assert!(trace.raw.is_none());
}

#[test]
fn store_bus_error_and_instruction_bus_remap_use_dc_re() {
    let mut store = loaded(&[imm(0x16, 1, 2, 0)]);
    let mut s = snapshot(&store);
    s.gpr[1] = 0x1_0000;
    s.gpr[2] = 0xfeed_beef;
    s.dc_re = true;
    s.deba = 0x2000;
    store.restore(s).unwrap();
    let trace = store.step().unwrap();
    let ex = trace.exception.unwrap();
    assert_eq!(ex.id, EXCEPTION_DATA_BUS);
    assert_eq!(ex.base, 0x2000);
    assert_eq!(trace.transactions[1].kind, BusTransactionKind::DataWrite);
    assert!(!trace.transactions[1].success);

    let mut fetch = loaded(&[0]);
    let mut s = snapshot(&fetch);
    s.pc = 0xffff_fffc;
    s.dc_re = true;
    s.deba = 0x3000;
    fetch.restore(s).unwrap();
    let ex = fetch.step().unwrap().exception.unwrap();
    assert_eq!(ex.id, EXCEPTION_INSTRUCTION_BUS);
    assert_eq!(ex.base, 0x3000);
}

#[test]
fn negative_immediate_control_flow_and_pc_wrap_are_exact() {
    let mut backward = loaded(&[immediate_control(0x38, -1)]);
    let trace = backward.step().unwrap();
    assert_eq!(trace.pc_after, 0xfc);
    assert_eq!(trace.branch, Some((true, 0xfc)));

    let mut wrap = loaded(&[branch_register(1)]);
    wrap.write_register(1, 0xffff_fffc).unwrap();
    wrap.step().unwrap();
    assert_eq!(snapshot(&wrap).pc, 0xffff_fffc);
}

#[test]
fn allocation_preflights_cover_snapshot_load_and_late_step_trace() {
    let mut cpu = loaded(&[imm(0x0d, 0, 1, 1)]);
    let before = snapshot(&cpu);
    cpu.fail_allocation_after(Some(0));
    assert_eq!(cpu.snapshot(), Err(Lm32Error::AllocationFailed));
    cpu.fail_allocation_after(Some(0));
    assert_eq!(
        cpu.load(&bytes(&[0xac00_0007]), 0x200),
        Err(Lm32Error::AllocationFailed)
    );
    assert_eq!(snapshot(&cpu), before);

    // Clone, transaction-vector, and post-effect trace-state checkpoints.
    for checkpoint in 0..4 {
        let mut probe = loaded(&[imm(0x0d, 0, 1, 1)]);
        let before = snapshot(&probe);
        probe.fail_allocation_after(Some(checkpoint));
        if probe.step() == Err(Lm32Error::AllocationFailed) {
            assert_eq!(snapshot(&probe), before);
        }
    }
}

#[test]
fn invalid_snapshot_metadata_and_bases_are_rejected() {
    let mut cpu = loaded(&[0]);
    for mutate in 0..4 {
        let mut invalid = snapshot(&cpu);
        match mutate {
            0 => invalid.origin = None,
            1 => invalid.length = 0,
            2 => invalid.eba = 1,
            _ => invalid.pc = 2,
        }
        let before = snapshot(&cpu);
        assert!(matches!(
            cpu.restore(invalid),
            Err(Lm32Error::InvalidState { .. })
        ));
        assert_eq!(snapshot(&cpu), before);
    }
}
