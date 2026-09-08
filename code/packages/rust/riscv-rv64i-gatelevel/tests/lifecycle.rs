use riscv_rv64i_gatelevel::{Rv64IGateLevel, FLIP_FLOP_COUNT};
use riscv_rv64i_simulator::encoding::*;
use riscv_rv64i_simulator::{Rv64IError, MEMORY_SIZE, REGISTER_COUNT, RESET_STACK_POINTER};

#[test]
fn topology_reset_load_and_origin_are_exact() {
    assert_eq!(FLIP_FLOP_COUNT, 526_401);
    let mut cpu = Rv64IGateLevel::new();
    let reset = cpu.get_state();
    assert_eq!(reset.registers.len(), REGISTER_COUNT);
    assert_eq!(reset.registers[0], 0);
    assert_eq!(reset.registers[2], RESET_STACK_POINTER);
    assert_eq!(reset.memory, vec![0; MEMORY_SIZE]);

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

    cpu.reset();
    assert_eq!(cpu.get_state(), reset);
}

#[test]
fn restore_round_trips_full_state_and_rejects_invalid_snapshots_atomically() {
    let mut cpu = Rv64IGateLevel::new();
    let bytes = assemble(&[encode_addi(1, 0, 9), encode_ecall()]);
    cpu.load_at_checked(&bytes, 0x80).unwrap();
    cpu.write_register(31, 0xfeed_face_cafe_beef).unwrap();
    cpu.write_byte(0xfffe, 0xa5).unwrap();
    let snapshot = cpu.get_state();
    cpu.reset();
    cpu.restore(&snapshot).unwrap();
    assert_eq!(cpu.get_state(), snapshot);

    for invalid in [
        {
            let mut state = snapshot.clone();
            state.registers[0] = 1;
            state
        },
        {
            let mut state = snapshot.clone();
            state.memory.pop();
            state
        },
        {
            let mut state = snapshot.clone();
            state.pc = 2;
            state
        },
    ] {
        assert!(matches!(
            cpu.restore(&invalid),
            Err(Rv64IError::InvalidState(_))
        ));
        assert_eq!(cpu.get_state(), snapshot);
    }
}

#[test]
fn invalid_loads_and_direct_access_are_checked_and_atomic() {
    let mut cpu = Rv64IGateLevel::new();
    cpu.load_checked(&assemble(&[encode_ecall()])).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.load_at_checked(&[1, 2, 3, 4], 2),
        Err(Rv64IError::MisalignedProgram { origin: 2 })
    );
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.load_at_checked(&[1, 2, 3, 4, 5], 0xfffc),
        Err(Rv64IError::ProgramOutOfRange {
            origin: 0xfffc,
            length: 5,
        })
    );
    assert_eq!(cpu.get_state(), before);

    cpu.write_register(0, u64::MAX).unwrap();
    assert_eq!(cpu.read_register(0), Ok(0));
    assert_eq!(
        cpu.read_register(REGISTER_COUNT),
        Err(Rv64IError::InvalidRegister {
            index: REGISTER_COUNT,
        })
    );
    assert_eq!(
        cpu.write_byte(MEMORY_SIZE as u64, 1),
        Err(Rv64IError::DataOutOfRange {
            address: MEMORY_SIZE as u64,
            width: 1,
        })
    );
}

#[test]
fn traces_halt_and_successful_runs_match_the_contract() {
    let words = [
        encode_addi(1, 0, 0x100),
        encode_addi(3, 0, 42),
        encode_sd(1, 3, 0),
        encode_ld(4, 1, 0),
        encode_zero_halt(),
    ];
    let mut cpu = Rv64IGateLevel::new();
    cpu.load_checked(&assemble(&words)).unwrap();
    let result = cpu.run_checked(words.len()).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, words.len());
    assert_eq!(result.traces[2].mnemonic, "sd");
    assert_eq!(result.traces[3].mnemonic, "ld");
    assert_eq!(result.traces[4].mnemonic, "zero-halt");
    assert_eq!(result.traces[0].pc_before, 0);
    assert_eq!(result.traces[0].pc_after, 4);
    assert_eq!(result.traces[0].raw, words[0]);
    assert_eq!(result.traces[3].state_after.registers[4], 42);
    assert_eq!(result.final_state, cpu.get_state());

    let halted = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(Rv64IError::Halted));
    assert_eq!(cpu.get_state(), halted);
    let already_halted = cpu.run_checked(0).unwrap();
    assert_eq!(already_halted.steps, 0);
    assert!(already_halted.halted);
}

#[test]
fn step_and_run_faults_are_transactional() {
    let cases = [
        (
            assemble(&[encode_ld(3, 0, 1)]),
            Rv64IError::MisalignedData {
                address: 1,
                width: 8,
            },
        ),
        (
            assemble(&[encode_ld(3, 2, 0)]),
            Rv64IError::DataOutOfRange {
                address: MEMORY_SIZE as u64,
                width: 8,
            },
        ),
        (
            assemble(&[encode_jal(1, 2)]),
            Rv64IError::MisalignedFetch { pc: 2 },
        ),
    ];
    for (program, expected) in cases {
        let mut cpu = Rv64IGateLevel::new();
        cpu.load_checked(&program).unwrap();
        if matches!(expected, Rv64IError::DataOutOfRange { .. }) {
            cpu.write_register(2, MEMORY_SIZE as u64).unwrap();
        }
        let before = cpu.get_state();
        assert_eq!(cpu.step_checked(), Err(expected));
        assert_eq!(cpu.get_state(), before);
    }

    let mut cpu = Rv64IGateLevel::new();
    cpu.load_checked(&assemble(&[encode_addi(1, 1, 1), encode_jal(0, -4)]))
        .unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_checked(5),
        Err(Rv64IError::StepLimitExceeded { max_steps: 5 })
    );
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&assemble(&[encode_addi(1, 0, 7), 0xffff_ffff]))
        .unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.run_checked(3),
        Err(Rv64IError::UnknownInstruction { .. })
    ));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn fetch_boundaries_and_every_malformed_decode_family_are_atomic() {
    let mut cpu = Rv64IGateLevel::new();
    cpu.load_checked(&[0x13, 0, 0]).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step_checked(),
        Err(Rv64IError::FetchOutsideProgram { pc: 0 })
    );
    assert_eq!(cpu.get_state(), before);

    let malformed: [u32; 12] = [
        0xffff_ffff,
        0x0000_1067,
        0x0000_2063,
        0x0000_7003,
        0x0000_4023,
        0x0400_1093,
        0x0400_00b3,
        0x0200_109b,
        0x0200_90bb,
        0x0000_008f,
        0x0010_100f,
        0x0020_0073,
    ];
    for raw in malformed {
        let mut cpu = Rv64IGateLevel::new();
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
