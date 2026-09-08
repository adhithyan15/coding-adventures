use riscv_rv64i_simulator::encoding::*;
use riscv_rv64i_simulator::{
    Rv64IError, Rv64ISimulator, MEMORY_SIZE, REGISTER_COUNT, RESET_STACK_POINTER,
};

fn assert_reset(cpu: &Rv64ISimulator) {
    let state = cpu.get_state();
    assert_eq!(state.registers.len(), REGISTER_COUNT);
    assert_eq!(state.registers[0], 0);
    assert_eq!(state.registers[2], RESET_STACK_POINTER);
    assert_eq!(state.pc, 0);
    assert_eq!(state.memory, vec![0; MEMORY_SIZE]);
    assert!(!state.halted);
    assert_eq!(state.loaded_origin, 0);
    assert_eq!(state.loaded_len, 0);
}

#[test]
fn reset_load_and_origin_metadata_are_exact() {
    let mut cpu = Rv64ISimulator::new();
    assert_reset(&cpu);

    let program = assemble(&[encode_addi(10, 0, 7), encode_ecall()]);
    cpu.load_at_checked(&program, 0x80).unwrap();
    let loaded = cpu.get_state();
    assert_eq!(loaded.pc, 0x80);
    assert_eq!(loaded.loaded_origin, 0x80);
    assert_eq!(loaded.loaded_len, 8);
    assert_eq!(&loaded.memory[0x80..0x88], program);

    cpu.run_checked(2).unwrap();
    cpu.reset();
    assert_reset(&cpu);
}

#[test]
fn invalid_loads_and_restore_are_atomic() {
    let mut cpu = Rv64ISimulator::new();
    cpu.load_checked(&assemble(&[encode_addi(1, 0, 9), encode_ecall()]))
        .unwrap();
    let before = cpu.get_state();

    assert_eq!(
        cpu.load_at_checked(&[1, 2, 3, 4], 2),
        Err(Rv64IError::MisalignedProgram { origin: 2 })
    );
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.load_at_checked(&[1, 2, 3, 4], MEMORY_SIZE as u64),
        Err(Rv64IError::ProgramOutOfRange {
            origin: MEMORY_SIZE as u64,
            length: 4
        })
    );
    assert_eq!(cpu.get_state(), before);

    let mut invalid = before.clone();
    invalid.registers[0] = 1;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(Rv64IError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);

    invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(Rv64IError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn direct_access_is_checked_and_x0_is_immutable() {
    let mut cpu = Rv64ISimulator::new();
    cpu.write_register(0, u64::MAX).unwrap();
    cpu.write_register(31, 0xfeed_face_cafe_beef).unwrap();
    assert_eq!(cpu.read_register(0), Ok(0));
    assert_eq!(cpu.read_register(31), Ok(0xfeed_face_cafe_beef));
    assert_eq!(
        cpu.read_register(REGISTER_COUNT),
        Err(Rv64IError::InvalidRegister {
            index: REGISTER_COUNT
        })
    );
    assert_eq!(
        cpu.write_register(REGISTER_COUNT, 1),
        Err(Rv64IError::InvalidRegister {
            index: REGISTER_COUNT
        })
    );

    cpu.write_byte((MEMORY_SIZE - 1) as u64, 0xa5).unwrap();
    assert_eq!(cpu.read_byte((MEMORY_SIZE - 1) as u64), Ok(0xa5));
    assert_eq!(
        cpu.read_byte(MEMORY_SIZE as u64),
        Err(Rv64IError::DataOutOfRange {
            address: MEMORY_SIZE as u64,
            width: 1
        })
    );
}

#[test]
fn traces_capture_complete_instruction_boundaries() {
    let program = assemble(&[
        encode_addi(1, 0, 0x100),
        encode_addi(3, 0, 42),
        encode_sd(1, 3, 0),
        encode_ld(4, 1, 0),
        encode_ecall(),
    ]);
    let mut cpu = Rv64ISimulator::new();
    cpu.load_checked(&program).unwrap();
    let result = cpu.run_checked(5).unwrap();

    assert!(result.halted);
    assert_eq!(result.steps, 5);
    assert_eq!(result.traces.len(), 5);
    assert_eq!(result.traces[2].mnemonic, "sd");
    assert_eq!(result.traces[3].mnemonic, "ld");
    assert_eq!(result.traces[4].mnemonic, "ecall");
    assert_eq!(result.traces[0].state_before.pc, 0);
    assert_eq!(result.traces[0].state_after.pc, 4);
    assert_eq!(result.traces[3].state_after.registers[4], 42);
    assert_eq!(result.final_state, cpu.get_state());
}

#[test]
fn step_faults_and_step_limit_roll_back_the_whole_machine() {
    let mut cpu = Rv64ISimulator::new();
    let program = assemble(&[
        encode_addi(1, 0, 1),
        encode_addi(1, 1, 1),
        encode_zero_halt(),
    ]);
    cpu.load_checked(&program).unwrap();
    let before_run = cpu.get_state();
    assert_eq!(
        cpu.run_checked(1),
        Err(Rv64IError::StepLimitExceeded { max_steps: 1 })
    );
    assert_eq!(cpu.get_state(), before_run);

    cpu.load_checked(&assemble(&[encode_ld(3, 0, 1), encode_ecall()]))
        .unwrap();
    let before_fault = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(Rv64IError::MisalignedData {
            address: 1,
            width: 8
        })
    );
    assert_eq!(cpu.get_state(), before_fault);
}

#[test]
fn control_flow_alignment_and_post_halt_are_typed() {
    let mut cpu = Rv64ISimulator::new();
    cpu.load_checked(&assemble(&[encode_jal(1, 2), encode_ecall()]))
        .unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(Rv64IError::MisalignedFetch { pc: 2 })
    );
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&assemble(&[encode_beq(0, 0, 2), encode_ecall()]))
        .unwrap();
    assert_eq!(
        cpu.step_checked(),
        Err(Rv64IError::MisalignedFetch { pc: 2 })
    );

    cpu.load_checked(&assemble(&[encode_zero_halt()])).unwrap();
    let trace = cpu.step_checked().unwrap();
    assert_eq!(trace.mnemonic, "zero-halt");
    assert!(trace.state_after.halted);
    let halted = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(Rv64IError::Halted));
    assert_eq!(cpu.get_state(), halted);
    let already_halted = cpu.run_checked(0).unwrap();
    assert_eq!(already_halted.steps, 0);
    assert!(already_halted.halted);
    assert_eq!(already_halted.final_state, halted);
}

#[test]
fn fetch_outside_install_and_unknown_decode_are_atomic() {
    let mut cpu = Rv64ISimulator::new();
    cpu.load_at_checked(&assemble(&[encode_addi(1, 0, 1)]), 0x40)
        .unwrap();
    cpu.step_checked().unwrap();
    let exhausted = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(Rv64IError::FetchOutsideProgram { pc: 0x44 })
    );
    assert_eq!(cpu.get_state(), exhausted);

    cpu.load_checked(&0xffff_ffff_u32.to_le_bytes()).unwrap();
    let unknown = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(Rv64IError::UnknownInstruction {
            raw: 0xffff_ffff,
            pc: 0
        })
    );
    assert_eq!(cpu.get_state(), unknown);
}

#[test]
fn every_malformed_decode_family_fails_atomically() {
    let malformed: [u32; 12] = [
        0xffff_ffff,
        0x0000_1067, // JALR with reserved funct3.
        0x0000_2063, // Reserved branch funct3.
        0x0000_7003, // Reserved load width.
        0x0000_4023, // Reserved store width.
        0x0400_1093, // SLLI with reserved funct6.
        0x0400_00b3, // ADD-family register op with reserved funct7.
        0x0200_109b, // SLLIW with reserved funct7.
        0x0200_90bb, // M word op with reserved funct3.
        0x0000_008f, // FENCE with reserved nonzero rd.
        0x0010_100f, // FENCE.I with reserved immediate.
        0x0020_0073, // Unsupported SYSTEM immediate.
    ];
    for raw in malformed {
        let mut cpu = Rv64ISimulator::new();
        cpu.load_checked(&raw.to_le_bytes()).unwrap();
        let before = cpu.get_state();
        assert_eq!(
            cpu.step_checked(),
            Err(Rv64IError::UnknownInstruction { raw, pc: 0 }),
            "raw {raw:08x}"
        );
        assert_eq!(cpu.get_state(), before, "raw {raw:08x}");
    }
}

#[test]
fn every_structured_encoder_reaches_its_decode_family() {
    let words = [
        encode_lui(5, 0x81234),
        encode_auipc(5, 0x81234),
        encode_jal(5, 0),
        encode_jalr(5, 0, 0),
        encode_beq(0, 0, 0),
        encode_bne(0, 0, 0),
        encode_blt(0, 0, 0),
        encode_bge(0, 0, 0),
        encode_bltu(0, 0, 0),
        encode_bgeu(0, 0, 0),
        encode_lb(5, 0, 0),
        encode_lh(5, 0, 0),
        encode_lw(5, 0, 0),
        encode_ld(5, 0, 0),
        encode_lbu(5, 0, 0),
        encode_lhu(5, 0, 0),
        encode_lwu(5, 0, 0),
        encode_sb(0, 0, 0),
        encode_sh(0, 0, 0),
        encode_sw(0, 0, 0),
        encode_sd(0, 0, 0),
        encode_addi(5, 0, -1),
        encode_slti(5, 0, -1),
        encode_sltiu(5, 0, -1),
        encode_xori(5, 0, -1),
        encode_ori(5, 0, -1),
        encode_andi(5, 0, -1),
        encode_slli(5, 0, 63),
        encode_srli(5, 0, 63),
        encode_srai(5, 0, 63),
        encode_add(5, 0, 0),
        encode_sub(5, 0, 0),
        encode_sll(5, 0, 0),
        encode_slt(5, 0, 0),
        encode_sltu(5, 0, 0),
        encode_xor(5, 0, 0),
        encode_srl(5, 0, 0),
        encode_sra(5, 0, 0),
        encode_or(5, 0, 0),
        encode_and(5, 0, 0),
        encode_addiw(5, 0, -1),
        encode_slliw(5, 0, 31),
        encode_srliw(5, 0, 31),
        encode_sraiw(5, 0, 31),
        encode_addw(5, 0, 0),
        encode_subw(5, 0, 0),
        encode_sllw(5, 0, 0),
        encode_srlw(5, 0, 0),
        encode_sraw(5, 0, 0),
        encode_mul(5, 0, 0),
        encode_mulh(5, 0, 0),
        encode_mulhsu(5, 0, 0),
        encode_mulhu(5, 0, 0),
        encode_div(5, 0, 0),
        encode_divu(5, 0, 0),
        encode_rem(5, 0, 0),
        encode_remu(5, 0, 0),
        encode_mulw(5, 0, 0),
        encode_divw(5, 0, 0),
        encode_divuw(5, 0, 0),
        encode_remw(5, 0, 0),
        encode_remuw(5, 0, 0),
        encode_fence(),
        encode_fence_i(),
        encode_ecall(),
        encode_ebreak(),
        encode_zero_halt(),
    ];
    for raw in words {
        let mut cpu = Rv64ISimulator::new();
        cpu.load_checked(&raw.to_le_bytes()).unwrap();
        cpu.step_checked()
            .unwrap_or_else(|error| panic!("raw {raw:08x}: {error}"));
    }
}
