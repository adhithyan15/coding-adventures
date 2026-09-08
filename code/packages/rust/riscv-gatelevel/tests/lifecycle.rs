use riscv_gatelevel::{Rv32IGateLevel, FLIP_FLOP_COUNT};
use riscv_simulator::csr::{CAUSE_ECALL_M_MODE, CSR_MCAUSE, CSR_MEPC, CSR_MTVEC};
use riscv_simulator::encoding::*;
use riscv_simulator::{Rv32IError, RV32I_MEMORY_SIZE};

#[test]
fn topology_reset_load_and_origin_are_exact() {
    assert_eq!(FLIP_FLOP_COUNT, 525_505);
    let mut cpu = Rv32IGateLevel::new();
    cpu.write_byte(17, 0xaa).unwrap();
    cpu.write_register(4, 99).unwrap();
    let bytes = assemble(&[encode_addi(1, 0, 7), encode_ecall()]);
    cpu.load_at_checked(&bytes, 0x100).unwrap();
    let state = cpu.get_state();
    assert_eq!(state.pc, 0x100);
    assert_eq!(state.loaded_origin, 0x100);
    assert_eq!(state.loaded_len, bytes.len());
    assert_eq!(&state.memory[0x100..0x100 + bytes.len()], bytes);
    assert_eq!(state.memory[17], 0);
    assert_eq!(state.registers[4], 0);
    assert_eq!(state.memory.len(), RV32I_MEMORY_SIZE);

    cpu.reset();
    assert_eq!(cpu.get_state(), Rv32IGateLevel::new().get_state());
}

#[test]
fn invalid_load_and_restore_are_atomic() {
    let mut cpu = Rv32IGateLevel::new();
    cpu.load_checked(&assemble(&[encode_ecall()])).unwrap();
    cpu.write_register(2, 0x1234).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.load_at_checked(&[1, 2, 3, 4], 2),
        Err(Rv32IError::MisalignedProgram { origin: 2 })
    );
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.load_at_checked(&[1, 2, 3, 4, 5], 0xfffc),
        Err(Rv32IError::ProgramOutOfRange {
            origin: 0xfffc,
            length: 5,
        })
    );
    assert_eq!(cpu.get_state(), before);

    let mut invalid = before.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(Rv32IError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
    let mut invalid = before.clone();
    invalid.registers[0] = 1;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(Rv32IError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
    let mut invalid = before.clone();
    invalid.pc = 2;
    assert!(matches!(
        cpu.restore(&invalid),
        Err(Rv32IError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn direct_access_is_checked_and_x0_is_hardwired() {
    let mut cpu = Rv32IGateLevel::new();
    assert_eq!(
        cpu.read_register(32),
        Err(Rv32IError::InvalidRegister { index: 32 })
    );
    assert_eq!(
        cpu.write_register(99, 1),
        Err(Rv32IError::InvalidRegister { index: 99 })
    );
    cpu.write_register(0, u32::MAX).unwrap();
    assert_eq!(cpu.read_register(0), Ok(0));
    cpu.write_byte(0xffff, 0xa5).unwrap();
    assert_eq!(cpu.read_byte(0xffff), Ok(0xa5));
    assert_eq!(
        cpu.write_byte(0x1_0000, 1),
        Err(Rv32IError::DataOutOfRange {
            address: 0x1_0000,
            width: 1,
        })
    );
}

#[test]
fn fetch_decode_data_and_csr_faults_are_step_atomic() {
    let cases = [
        (
            assemble(&[0xffff_ffff]),
            Rv32IError::UnknownInstruction {
                raw: 0xffff_ffff,
                pc: 0,
            },
        ),
        (
            assemble(&[(1 << 25) | encode_add(1, 2, 3)]),
            Rv32IError::UnknownInstruction {
                raw: (1 << 25) | encode_add(1, 2, 3),
                pc: 0,
            },
        ),
        (
            assemble(&[encode_lw(1, 0, 2)]),
            Rv32IError::MisalignedData {
                address: 2,
                width: 4,
            },
        ),
        (
            assemble(&[encode_lw(1, 2, 0)]),
            Rv32IError::DataOutOfRange {
                address: 0x1_0000,
                width: 4,
            },
        ),
    ];
    for (bytes, expected) in cases {
        let mut cpu = Rv32IGateLevel::new();
        cpu.load_checked(&bytes).unwrap();
        if matches!(expected, Rv32IError::DataOutOfRange { .. }) {
            cpu.write_register(2, 0x1_0000).unwrap();
        }
        let before = cpu.get_state();
        assert_eq!(cpu.step_checked(), Err(expected));
        assert_eq!(cpu.get_state(), before);
    }

    let mut cpu = Rv32IGateLevel::new();
    cpu.load_checked(&[0x13, 0, 0]).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(Rv32IError::FetchOutsideProgram { pc: 0 })
    );
    assert_eq!(cpu.get_state(), before);

    let mut cpu = Rv32IGateLevel::new();
    cpu.load_checked(&assemble(&[encode_csrrw(1, 0xfff, 2)]))
        .unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(Rv32IError::UnsupportedCsr {
            address: 0xfff,
            pc: 0,
        })
    );
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn traces_are_complete_and_runs_are_transactional() {
    let mut cpu = Rv32IGateLevel::new();
    let bytes = assemble(&[encode_addi(1, 0, 9), encode_ecall()]);
    cpu.load_checked(&bytes).unwrap();
    let trace = cpu.step_checked().unwrap();
    assert_eq!(trace.pc_before, 0);
    assert_eq!(trace.pc_after, 4);
    assert_eq!(trace.raw, encode_addi(1, 0, 9));
    assert_eq!(trace.mnemonic, "addi");
    assert_eq!(trace.state_before.registers[1], 0);
    assert_eq!(trace.state_after.registers[1], 9);
    cpu.step_checked().unwrap();
    let halted = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(Rv32IError::Halted));
    assert_eq!(cpu.get_state(), halted);

    let mut cpu = Rv32IGateLevel::new();
    cpu.load_checked(&assemble(&[encode_addi(1, 1, 1), encode_jal(0, -4)]))
        .unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_checked(5),
        Err(Rv32IError::StepLimitExceeded { max_steps: 5 })
    );
    assert_eq!(cpu.get_state(), before);

    let mut cpu = Rv32IGateLevel::new();
    cpu.load_checked(&assemble(&[encode_addi(1, 0, 7), 0xffff_ffff]))
        .unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.run_checked(3),
        Err(Rv32IError::UnknownInstruction { .. })
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn trap_csrs_and_mret_follow_the_functional_contract() {
    let mut cpu = Rv32IGateLevel::new();
    let mut state = cpu.get_state();
    state.loaded_origin = 0;
    state.loaded_len = 0x104;
    state.csr_mtvec = 0x100;
    state.csr_mstatus = 1 << 3;
    state.memory[..4].copy_from_slice(&encode_ecall().to_le_bytes());
    state.memory[0x100..0x104].copy_from_slice(&encode_mret().to_le_bytes());
    cpu.restore(&state).unwrap();

    assert_eq!(cpu.step_checked().unwrap().pc_after, 0x100);
    let trapped = cpu.get_state();
    assert_eq!(trapped.csr_mepc, 0);
    assert_eq!(trapped.csr_mcause, CAUSE_ECALL_M_MODE);
    assert_eq!(trapped.csr_mstatus & (1 << 3), 0);
    assert_eq!(cpu.step_checked().unwrap().pc_after, 0);
    assert_ne!(cpu.get_state().csr_mstatus & (1 << 3), 0);
    assert_eq!(CSR_MTVEC, 0x305);
    assert_eq!(CSR_MEPC, 0x341);
    assert_eq!(CSR_MCAUSE, 0x342);
}
