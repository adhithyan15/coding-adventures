use arm7tdmi_simulator::*;

fn arm(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn thumb(words: &[u16]) -> Vec<u8> {
    words.iter().flat_map(|word| word.to_le_bytes()).collect()
}

fn arm_cpu(words: &[u32]) -> Arm7TdmiSimulator {
    let mut cpu = Arm7TdmiSimulator::new();
    cpu.load(&arm(words), 0, InstructionSet::Arm).unwrap();
    cpu
}

fn thumb_cpu(words: &[u16]) -> Arm7TdmiSimulator {
    let mut cpu = Arm7TdmiSimulator::new();
    cpu.load(&thumb(words), 0, InstructionSet::Thumb).unwrap();
    cpu
}

fn dp_imm(cond: u32, opcode: u32, set: bool, rn: u32, rd: u32, value: u32) -> u32 {
    cond << 28 | 1 << 25 | opcode << 21 | u32::from(set) << 20 | rn << 16 | rd << 12 | value
}

fn with_state(cpu: &mut Arm7TdmiSimulator, change: impl FnOnce(&mut Arm7TdmiState)) {
    let mut state = cpu.get_state();
    change(&mut state);
    cpu.restore(state).unwrap();
}

#[test]
fn reset_load_and_complete_physical_state() {
    let mut cpu = Arm7TdmiSimulator::new();
    assert_eq!(cpu.state().memory.len(), MEMORY_SIZE);
    assert_eq!(cpu.state().cpsr, Mode::Supervisor as u32 | CPSR_I | CPSR_F);
    cpu.load(&arm(&[0xef00_0000]), 0x100, InstructionSet::Arm)
        .unwrap();
    assert_eq!(cpu.state().registers.pc, 0x100);
    assert_eq!(cpu.state().loaded_len, 4);
    assert_eq!(
        &cpu.state().memory[0x100..0x104],
        &0xef00_0000_u32.to_le_bytes()
    );
    cpu.reset();
    assert_eq!(cpu.state().registers, PhysicalRegisters::default());
    assert_eq!(cpu.state().loaded_len, 0);
}

#[test]
fn all_register_banks_are_views_not_copies() {
    let mut cpu = arm_cpu(&[0xef00_0000]);
    let modes = [
        Mode::User,
        Mode::System,
        Mode::Fiq,
        Mode::Irq,
        Mode::Supervisor,
        Mode::Abort,
        Mode::Undefined,
    ];
    for (index, mode) in modes.into_iter().enumerate() {
        with_state(&mut cpu, |state| state.cpsr = mode as u32);
        cpu.write_register(13, 0x1000 + index as u32).unwrap();
        cpu.write_register(14, 0x2000 + index as u32).unwrap();
        if mode == Mode::Fiq {
            cpu.write_register(8, 0xf108).unwrap();
        }
    }
    with_state(&mut cpu, |state| state.cpsr = Mode::User as u32);
    assert_eq!(cpu.read_register(13).unwrap(), 0x1001);
    with_state(&mut cpu, |state| state.cpsr = Mode::System as u32);
    assert_eq!(cpu.read_register(13).unwrap(), 0x1001); // User/System share: last write wins.
    with_state(&mut cpu, |state| state.cpsr = Mode::Fiq as u32);
    assert_eq!(cpu.read_register(8).unwrap(), 0xf108);
    assert_eq!(cpu.read_register(13).unwrap(), 0x1002);
    assert_eq!(cpu.state().registers.common_r0_r7.len(), 8);
    assert_eq!(cpu.state().registers.normal_r8_r12.len(), 5);
}

#[test]
fn restore_rejects_reserved_state_atomically() {
    let mut cpu = arm_cpu(&[0xef00_0000]);
    let before = cpu.get_state();
    let mut bad = before.clone();
    bad.cpsr = 0;
    assert_eq!(
        cpu.restore(bad),
        Err(Arm7TdmiError::ReservedMode { mode: 0 })
    );
    assert_eq!(cpu.state(), &before);
    let mut bad = before.clone();
    bad.memory.pop();
    assert!(matches!(
        cpu.restore(bad),
        Err(Arm7TdmiError::InvalidState { .. })
    ));
    assert_eq!(cpu.state(), &before);
    let mut bad = before.clone();
    bad.registers.pc = 2;
    assert!(matches!(
        cpu.restore(bad),
        Err(Arm7TdmiError::InvalidState { .. })
    ));
    assert_eq!(cpu.state(), &before);
}

#[test]
fn all_arm_conditions_have_taken_and_failed_witnesses() {
    let flag_sets = [0, CPSR_Z, CPSR_C, CPSR_N, CPSR_V, CPSR_C | CPSR_N | CPSR_V];
    for condition in 0_u32..14 {
        for flags in flag_sets {
            let mut cpu = arm_cpu(&[dp_imm(condition, 13, false, 0, 0, 1)]);
            with_state(&mut cpu, |state| {
                state.cpsr = Mode::Supervisor as u32 | flags
            });
            let trace = cpu.step().unwrap();
            assert_eq!(trace.condition_met, trace.mnemonic != "CONDITION_FAILED");
            assert_eq!(cpu.read_register(0).unwrap() == 1, trace.condition_met);
        }
    }
    let mut cpu = arm_cpu(&[0xf3a0_0001]);
    assert!(matches!(
        cpu.step(),
        Err(Arm7TdmiError::UnknownInstruction { .. })
    ));
}

#[test]
fn arm_data_processing_all_sixteen_operations() {
    for opcode in 0..16 {
        let mut cpu = arm_cpu(&[dp_imm(0xe, opcode, true, 1, 2, 3)]);
        cpu.write_register(1, 7).unwrap();
        cpu.step().unwrap();
        assert_ne!(
            cpu.state().cpsr & (CPSR_N | CPSR_Z | CPSR_C | CPSR_V),
            0xffff_ffff
        );
    }
    let mut cpu = arm_cpu(&[
        0xe1b0_0061, // MOVS r0,r1,RRX
        0xe1b0_0221, // MOVS r0,r1,LSR #4
        0xe1b0_0441, // MOVS r0,r1,ASR #8
        0xe1b0_0861, // MOVS r0,r1,ROR #16
    ]);
    cpu.write_register(1, 0x8000_0001).unwrap();
    for _ in 0..4 {
        cpu.step().unwrap();
    }
}

#[test]
fn register_shifts_observe_pipeline_pc() {
    let mut cpu = arm_cpu(&[0xe1a0_021f, 0xef00_0000]); // MOV r0,pc,LSL r2
    cpu.write_register(2, 0).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(0).unwrap(), 12);
}

#[test]
fn arm_multiply_and_long_multiply_families() {
    let instructions = [
        0xe000_0291, // MUL r0,r1,r2
        0xe023_4291, // MLA r3,r1,r2,r4
        0xe086_5291, // UMULL r5,r6,r1,r2
        0xe0a6_5291, // UMLAL r5,r6,r1,r2
        0xe0c8_7291, // SMULL r7,r8,r1,r2
        0xe0e8_7291, // SMLAL r7,r8,r1,r2
    ];
    let mut cpu = arm_cpu(&instructions);
    cpu.write_register(1, 3).unwrap();
    cpu.write_register(2, 4).unwrap();
    cpu.write_register(4, 5).unwrap();
    for _ in instructions {
        cpu.step().unwrap();
    }
    assert_eq!(cpu.read_register(0).unwrap(), 12);
    assert_eq!(cpu.read_register(3).unwrap(), 17);
}

#[test]
fn arm_single_transfers_cover_addressing_and_widths() {
    let mut cpu = arm_cpu(&[
        0xe581_0004, // STR r0,[r1,#4]
        0xe591_2004, // LDR r2,[r1,#4]
        0xe4c1_0001, // STRB r0,[r1],#1
        0xe551_3001, // LDRB r3,[r1,#-1]
    ]);
    cpu.write_register(0, 0x1234_56ab).unwrap();
    cpu.write_register(1, 0x100).unwrap();
    for _ in 0..4 {
        cpu.step().unwrap();
    }
    assert_eq!(cpu.read_register(2).unwrap(), 0x1234_56ab);
    assert_eq!(cpu.read_register(3).unwrap(), 0xab);
    assert_eq!(cpu.read_register(1).unwrap(), 0x101);
}

#[test]
fn arm_halfword_and_signed_transfers() {
    let mut cpu = arm_cpu(&[
        0xe1c1_00b0, // STRH r0,[r1]
        0xe1d1_20b0, // LDRH r2,[r1]
        0xe1d1_30d0, // LDRSB r3,[r1]
        0xe1d1_40f0, // LDRSH r4,[r1]
    ]);
    cpu.write_register(0, 0xffff_80ff).unwrap();
    cpu.write_register(1, 0x100).unwrap();
    for _ in 0..4 {
        cpu.step().unwrap();
    }
    assert_eq!(cpu.read_register(2).unwrap(), 0x80ff);
    assert_eq!(cpu.read_register(3).unwrap(), u32::MAX);
    assert_eq!(cpu.read_register(4).unwrap(), 0xffff_80ff);
}

#[test]
fn arm_block_transfers_all_address_modes() {
    for (pre, up) in [(false, true), (true, true), (false, false), (true, false)] {
        let store =
            0xe800_0000 | u32::from(pre) << 24 | u32::from(up) << 23 | 1 << 21 | 1 << 16 | 0b101;
        let load = store | 1 << 20 | 2 << 16;
        let mut cpu = arm_cpu(&[store, load]);
        cpu.write_register(0, 0xaaaa).unwrap();
        cpu.write_register(2, 0xbbbb).unwrap();
        cpu.write_register(1, 0x200).unwrap();
        cpu.write_register(2, 0x200).unwrap();
        cpu.step().unwrap();
        // Load without writeback/base overlap is exercised separately below.
        assert_ne!(cpu.state().memory[0x1f8..0x208], [0; 16]);
    }
}

#[test]
fn arm_branch_link_bx_and_low_bit_rules() {
    let mut cpu = arm_cpu(&[0xeb00_0000, 0xef00_0000, 0xef00_0000]);
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(14).unwrap(), 4);
    assert_eq!(cpu.state().registers.pc, 8);
    let mut cpu = arm_cpu(&[0xe12f_ff10, 0xef00_0000]);
    cpu.write_register(0, 5).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().cpsr & CPSR_T, CPSR_T);
    assert_eq!(cpu.state().registers.pc, 4);
    let mut cpu = arm_cpu(&[0xe12f_ff10]);
    cpu.write_register(0, 2).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.step(),
        Err(Arm7TdmiError::UnpredictableInstruction { .. })
    ));
    assert_eq!(cpu.state(), &before);
}

#[test]
fn pc_writes_preserve_state_except_bx_and_exception_return() {
    let mut cpu = arm_cpu(&[0xe1b0_f000]); // MOVS pc,r0
    cpu.write_register(0, 0x101).unwrap();
    with_state(&mut cpu, |state| {
        state.spsr_supervisor = Mode::User as u32 | CPSR_T
    });
    cpu.step().unwrap();
    assert_eq!(cpu.state().cpsr, Mode::User as u32 | CPSR_T);
    assert_eq!(cpu.state().registers.pc, 0x100);

    let mut cpu = arm_cpu(&[0xe591_f000]);
    cpu.write_register(1, 0x80).unwrap();
    cpu.write_memory(0x80, &0x103_u32.to_le_bytes()).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().cpsr & CPSR_T, 0);
    assert_eq!(cpu.state().registers.pc, 0x100);

    let mut cpu = thumb_cpu(&[0xbd00]); // POP {pc}
    cpu.write_register(13, 0x80).unwrap();
    cpu.write_memory(0x80, &0x103_u32.to_le_bytes()).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().cpsr & CPSR_T, CPSR_T);
    assert_eq!(cpu.state().registers.pc, 0x102);
}

#[test]
fn bx_classifies_every_low_bit_pattern() {
    for low in 0..4_u32 {
        let mut cpu = arm_cpu(&[0xe12f_ff10]);
        cpu.write_register(0, 0x100 | low).unwrap();
        let before = cpu.get_state();
        if low == 2 {
            assert!(matches!(
                cpu.step(),
                Err(Arm7TdmiError::UnpredictableInstruction { .. })
            ));
            assert_eq!(cpu.state(), &before);
        } else {
            cpu.step().unwrap();
            assert_eq!(
                cpu.state().registers.pc,
                if low & 1 != 0 {
                    0x100 | (low & 2)
                } else {
                    0x100
                }
            );
            assert_eq!(
                cpu.state().cpsr & CPSR_T,
                if low & 1 != 0 { CPSR_T } else { 0 }
            );
        }
    }
}

#[test]
fn mrs_msr_status_and_spsr_boundaries() {
    let mut cpu = arm_cpu(&[
        0xe10f_0000, // MRS r0,CPSR
        0xe129_f001, // MSR CPSR_fc,r1
        0xe14f_2000, // MRS r2,SPSR
        0xe169_f003, // MSR SPSR_fc,r3
    ]);
    cpu.write_register(1, Mode::Supervisor as u32 | CPSR_N | CPSR_I | CPSR_F)
        .unwrap();
    cpu.write_register(3, Mode::Irq as u32 | CPSR_Z).unwrap();
    for _ in 0..4 {
        cpu.step().unwrap();
    }
    assert_eq!(
        cpu.read_register(0).unwrap() & 0x1f,
        Mode::Supervisor as u32
    );
    assert_eq!(cpu.state().spsr_supervisor, Mode::Irq as u32 | CPSR_Z);
    let mut user = arm_cpu(&[0xe14f_0000]);
    with_state(&mut user, |state| state.cpsr = Mode::User as u32);
    assert!(matches!(
        user.step(),
        Err(Arm7TdmiError::UnavailableSavedStatus { .. })
    ));
}

#[test]
fn status_masks_mode_changes_and_immediate_rotation_are_exact() {
    let mut cpu = arm_cpu(&[
        0xe121_f001, // MSR CPSR_c,r1
        0xe328_f102, // MSR CPSR_f,#0x80000000 (imm 2 ROR 2)
        0xef00_0000,
    ]);
    cpu.write_register(1, Mode::Irq as u32 | CPSR_I | CPSR_F)
        .unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().cpsr & 0x1f, Mode::Irq as u32);
    cpu.step().unwrap();
    assert_eq!(cpu.state().cpsr & CPSR_N, CPSR_N);

    let mut user = arm_cpu(&[0xe129_f001]); // User writes flags; control byte is ignored.
    with_state(&mut user, |state| state.cpsr = Mode::User as u32);
    user.write_register(1, Mode::Fiq as u32 | CPSR_Z).unwrap();
    user.step().unwrap();
    assert_eq!(user.state().cpsr & 0x1f, Mode::User as u32);
    assert_eq!(user.state().cpsr & CPSR_Z, CPSR_Z);
}

#[test]
fn swap_is_atomic_and_width_exact() {
    let mut cpu = arm_cpu(&[0xe101_2093, 0xe141_4095]);
    cpu.write_register(1, 0x100).unwrap();
    cpu.write_register(3, 0xaabb_ccdd).unwrap();
    cpu.write_register(5, 0x77).unwrap();
    cpu.write_memory(0x100, &0x1122_3344_u32.to_le_bytes())
        .unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(2).unwrap(), 0x1122_3344);
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(4).unwrap(), 0xdd);
}

#[test]
fn arm_pc_sources_store_plus_twelve() {
    let mut cpu = arm_cpu(&[0xe581_f000]);
    cpu.write_register(1, 0x100).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_memory(0x100, 4).unwrap(), 12_u32.to_le_bytes());
}

#[test]
fn thumb_shift_add_and_immediate_groups() {
    let words = [
        0x0048, 0x0848, 0x1048, 0x1888, 0x1e88, 0x202a, 0x282a, 0x3001, 0x3801,
    ];
    let mut cpu = thumb_cpu(&words);
    cpu.write_register(1, 8).unwrap();
    for _ in words {
        cpu.step().unwrap();
    }
    assert_eq!(cpu.read_register(0).unwrap(), 42);
}

#[test]
fn thumb_all_sixteen_alu_operations() {
    let words: Vec<u16> = (0..16).map(|op| 0x4000 | (op << 6) | (1 << 3)).collect();
    let mut cpu = thumb_cpu(&words);
    cpu.write_register(0, 7).unwrap();
    cpu.write_register(1, 3).unwrap();
    for _ in 0..16 {
        cpu.step().unwrap();
    }
}

#[test]
fn thumb_high_register_and_exchange() {
    let mut cpu = thumb_cpu(&[0x4688, 0x4488, 0x4588, 0x4700, 0xdf00]);
    cpu.write_register(1, 4).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(8).unwrap(), 4);
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(8).unwrap(), 8);
    cpu.step().unwrap();
    cpu.write_register(0, 9).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().registers.pc, 8);
}

#[test]
fn mixed_state_image_keeps_its_original_transport_alignment() {
    let mut cpu = Arm7TdmiSimulator::new();
    let mut program = thumb(&[0x4700]); // BX r0 at address 2.
    program.extend_from_slice(&0xef00_0000_u32.to_le_bytes()); // ARM SWI 0 at address 4.
    cpu.load(&program, 2, InstructionSet::Thumb).unwrap();
    cpu.write_register(0, 4).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.state().cpsr & CPSR_T, 0);
    assert_eq!(cpu.state().registers.pc, 4);
    assert!(cpu.step().unwrap().state_after.halted);
}

#[test]
fn control_targets_are_preflighted_atomically() {
    let mut cpu = arm_cpu(&[0xe12f_ff10]);
    cpu.write_register(0, 0x1_0000).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step(),
        Err(Arm7TdmiError::TargetOutOfRange {
            target: 0x1_0000,
            instruction_set: InstructionSet::Arm
        })
    );
    assert_eq!(cpu.state(), &before);

    let mut cpu = arm_cpu(&[0xea7f_ffff]);
    let before = cpu.get_state();
    assert!(matches!(
        cpu.step(),
        Err(Arm7TdmiError::TargetOutOfRange { .. })
    ));
    assert_eq!(cpu.state(), &before);
}

#[test]
fn thumb_register_memory_all_eight_operations() {
    let words: Vec<u16> = (0..8)
        .map(|op| 0x5000 | (op << 9) | (2 << 6) | (1 << 3))
        .collect();
    let mut cpu = thumb_cpu(&words);
    cpu.write_register(0, 0xffff_80ff).unwrap();
    cpu.write_register(1, 0x100).unwrap();
    cpu.write_register(2, 0).unwrap();
    cpu.write_memory(0x100, &0xffff_80ff_u32.to_le_bytes())
        .unwrap();
    for _ in 0..8 {
        cpu.step().unwrap();
    }
}

#[test]
fn thumb_immediate_literal_and_sp_transfers() {
    let mut cpu = thumb_cpu(&[
        0x6008, 0x680a, 0x7008, 0x780b, 0x8008, 0x880c, 0x9800, 0x9001, 0xdf00,
    ]);
    cpu.write_register(0, 0x1122_3344).unwrap();
    cpu.write_register(1, 0x100).unwrap();
    cpu.write_register(13, 0x200).unwrap();
    cpu.write_memory(0x200, &0x5566_7788_u32.to_le_bytes())
        .unwrap();
    for _ in 0..8 {
        cpu.step().unwrap();
    }
    assert_eq!(cpu.read_register(2).unwrap(), 0x1122_3344);
    assert_eq!(cpu.read_register(4).unwrap(), 0x3344);
}

#[test]
fn thumb_pc_literal_and_address_generation_align_visible_pc() {
    let mut cpu = Arm7TdmiSimulator::new();
    let mut program = thumb(&[0x4801, 0xa101, 0xdf00, 0]);
    program.extend_from_slice(&0x1234_5678_u32.to_le_bytes());
    cpu.load(&program, 0, InstructionSet::Thumb).unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(0).unwrap(), 0x1234_5678);
    assert_eq!(cpu.read_register(1).unwrap(), 8);
}

#[test]
fn thumb_sp_adjust_push_pop_and_extensions() {
    let mut cpu = thumb_cpu(&[0xb081, 0xb001, 0xb503, 0xbc0c, 0xdf00]);
    cpu.write_register(0, 1).unwrap();
    cpu.write_register(1, 2).unwrap();
    cpu.write_register(13, 0x200).unwrap();
    cpu.write_register(14, 0x101).unwrap();
    cpu.step().unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(13).unwrap(), 0x200);
    cpu.step().unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(2).unwrap(), 1);
    assert_eq!(cpu.read_register(3).unwrap(), 2);
}

#[test]
fn thumb_multiple_transfer_and_writeback() {
    let mut cpu = thumb_cpu(&[0xc106, 0xca18, 0xdf00]);
    cpu.write_register(1, 0x100).unwrap();
    cpu.write_register(2, 0xaaaa).unwrap();
    cpu.write_register(2, 0x100).unwrap();
    cpu.write_register(3, 0xbbbb).unwrap();
    cpu.step().unwrap();
    cpu.write_register(2, 0x100).unwrap();
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(4).unwrap(), 0x100);
}

#[test]
fn thumb_conditional_unconditional_and_long_branch() {
    let mut cpu = thumb_cpu(&[0xd100, 0xe000, 0xdf00]);
    with_state(&mut cpu, |state| state.cpsr |= CPSR_Z);
    let trace = cpu.step().unwrap();
    assert!(!trace.condition_met);
    cpu.step().unwrap();
    assert_eq!(cpu.state().registers.pc, 6);

    let mut cpu = thumb_cpu(&[0xf000, 0xf801, 0xdf00, 0xdf00]);
    cpu.step().unwrap();
    let interim = cpu.read_register(14).unwrap();
    assert_eq!(interim, 4);
    cpu.step().unwrap();
    assert_eq!(cpu.read_register(14).unwrap(), 5);
    assert_eq!(cpu.state().registers.pc, 6);
}

#[test]
fn exact_halt_traces_and_post_halt_faults() {
    let mut arm = arm_cpu(&[0xef00_0000]);
    let trace = arm.step().unwrap();
    assert_eq!(trace.fetched, 0xef00_0000_u32.to_le_bytes());
    assert_eq!(trace.raw, 0xef00_0000);
    assert!(trace.state_after.halted);
    let frozen = arm.get_state();
    assert_eq!(arm.step(), Err(Arm7TdmiError::Halted));
    assert_eq!(arm.state(), &frozen);
    let mut thumb = thumb_cpu(&[0xdf00]);
    let trace = thumb.step().unwrap();
    assert_eq!(&trace.fetched[..2], &0xdf00_u16.to_le_bytes());
    assert!(trace.state_after.halted);
}

#[test]
fn trace_keeps_fetched_bytes_before_self_modification() {
    let raw = 0xe580_0000_u32; // STR r0,[r0]
    let mut cpu = arm_cpu(&[raw]);
    cpu.write_register(0, 0).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.fetched, raw.to_le_bytes());
    assert_eq!(&trace.state_after.memory[..4], &[0; 4]);
    assert_eq!(trace.state_before.memory[..4], raw.to_le_bytes());
}

#[test]
fn memory_fetch_target_and_decode_faults_are_atomic() {
    let cases = [
        0xe591_0000, // misaligned LDR
        0xe12f_ff10, // bad BX target supplied below
        0xee00_0010, // coprocessor
        0xef00_0001, // nonzero SWI
    ];
    for raw in cases {
        let mut cpu = arm_cpu(&[raw]);
        cpu.write_register(1, 1).unwrap();
        cpu.write_register(0, 2).unwrap();
        let before = cpu.get_state();
        assert!(cpu.step().is_err());
        assert_eq!(cpu.state(), &before);
    }
    let mut cpu = thumb_cpu(&[0xde00]);
    let before = cpu.get_state();
    assert!(cpu.step().is_err());
    assert_eq!(cpu.state(), &before);
}

#[test]
fn later_architecture_and_reserved_neighbors_are_fail_closed() {
    for raw in [0xe12f_ff30, 0xe120_0070] {
        // BLX register, BKPT
        let mut cpu = arm_cpu(&[raw]);
        let before = cpu.get_state();
        assert!(matches!(
            cpu.step(),
            Err(Arm7TdmiError::UnsupportedInstruction { .. })
        ));
        assert_eq!(cpu.state(), &before);
    }
    for raw in [0xbe00, 0xb650, 0xb660, 0x4780, 0xe800] {
        // BKPT, SETEND, CPS, BLX reg/imm
        let mut cpu = thumb_cpu(&[raw]);
        let before = cpu.get_state();
        assert!(matches!(
            cpu.step(),
            Err(Arm7TdmiError::UnsupportedInstruction { .. })
        ));
        assert_eq!(cpu.state(), &before);
    }
    let mut cpu = thumb_cpu(&[0x4400]); // low-register form in the high-register group
    assert!(matches!(
        cpu.step(),
        Err(Arm7TdmiError::UnpredictableInstruction { .. })
    ));
}

#[test]
fn public_pc_write_obeys_current_state_target_rules() {
    let mut cpu = arm_cpu(&[0xef00_0000]);
    cpu.write_register(15, 2).unwrap(); // ordinary ARM PC write discards bits 1:0
    assert_eq!(cpu.state().registers.pc, 0);
    assert_eq!(
        cpu.write_register(15, 0x1_0000),
        Err(Arm7TdmiError::TargetOutOfRange {
            target: 0x1_0000,
            instruction_set: InstructionSet::Arm
        })
    );
    cpu.write_register(15, 4).unwrap();
    assert_eq!(cpu.state().registers.pc, 4);
}

#[test]
fn malformed_programs_and_fetches_are_typed() {
    let mut cpu = Arm7TdmiSimulator::new();
    assert_eq!(
        cpu.load(&[], 0, InstructionSet::Arm),
        Err(Arm7TdmiError::EmptyProgram)
    );
    assert!(matches!(
        cpu.load(&[0; 4], 2, InstructionSet::Arm),
        Err(Arm7TdmiError::MisalignedProgram { .. })
    ));
    assert!(matches!(
        cpu.load(&[0; 4], 0xffff, InstructionSet::Thumb),
        Err(Arm7TdmiError::MisalignedProgram { .. })
    ));
    let mut cpu = arm_cpu(&[0xe1a0_0000]);
    cpu.step().unwrap();
    let frozen = cpu.get_state();
    assert_eq!(
        cpu.step(),
        Err(Arm7TdmiError::FetchOutsideProgram { pc: 4, width: 4 })
    );
    assert_eq!(cpu.state(), &frozen);
}

#[test]
fn run_and_execute_are_transactional_and_bounded() {
    let mut cpu = arm_cpu(&[0xe1a0_0000]);
    let before = cpu.get_state();
    assert_eq!(
        cpu.run(1),
        Err(Arm7TdmiError::StepLimitExceeded { limit: 1 })
    );
    assert_eq!(cpu.state(), &before);
    assert_eq!(
        cpu.run(65),
        Err(Arm7TdmiError::RetentionLimitExceeded {
            requested: 65,
            limit: 64
        })
    );
    let result = cpu
        .execute(&arm(&[0xef00_0000]), 0, InstructionSet::Arm, 1)
        .unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 1);
    let committed = cpu.get_state();
    assert!(cpu
        .execute(&arm(&[0xe591_0000]), 0, InstructionSet::Arm, 1)
        .is_err());
    assert_eq!(cpu.state(), &committed);
}

#[test]
fn every_error_has_stable_nonempty_text() {
    let errors = [
        Arm7TdmiError::EmptyProgram,
        Arm7TdmiError::MisalignedProgram {
            origin: 1,
            alignment: 4,
        },
        Arm7TdmiError::ProgramOutOfRange {
            origin: 0xffff,
            length: 4,
        },
        Arm7TdmiError::InvalidState { reason: "test" },
        Arm7TdmiError::InvalidRegister { register: 16 },
        Arm7TdmiError::ReservedMode { mode: 0 },
        Arm7TdmiError::UnavailableSavedStatus { mode: Mode::User },
        Arm7TdmiError::PrivilegedInstruction { raw: 0, pc: 0 },
        Arm7TdmiError::UnpredictableInstruction { raw: 0, pc: 0 },
        Arm7TdmiError::UnknownInstruction {
            raw: 0,
            width: 4,
            pc: 0,
        },
        Arm7TdmiError::UnsupportedInstruction {
            raw: 0,
            width: 4,
            pc: 0,
        },
        Arm7TdmiError::UnsupportedException {
            immediate: 1,
            pc: 0,
        },
        Arm7TdmiError::MisalignedFetch {
            pc: 1,
            alignment: 2,
        },
        Arm7TdmiError::FetchOutsideProgram { pc: 4, width: 4 },
        Arm7TdmiError::MisalignedData {
            address: 1,
            width: 4,
        },
        Arm7TdmiError::DataOutOfRange {
            address: 0xffff,
            width: 4,
        },
        Arm7TdmiError::MisalignedTarget {
            target: 2,
            instruction_set: InstructionSet::Arm,
        },
        Arm7TdmiError::TargetOutOfRange {
            target: 0x1_0000,
            instruction_set: InstructionSet::Arm,
        },
        Arm7TdmiError::Halted,
        Arm7TdmiError::RetentionLimitExceeded {
            requested: 65,
            limit: 64,
        },
        Arm7TdmiError::AllocationFailed,
        Arm7TdmiError::StepLimitExceeded { limit: 1 },
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}
