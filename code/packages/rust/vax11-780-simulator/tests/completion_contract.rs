use vax11_780_simulator::{
    MEMORY_SIZE, Psl, REGISTER_PC, REGISTER_SP, VaxError, VaxSimulator, encoding,
};

fn run(program: &[u8]) -> vax11_780_simulator::ExecutionResult {
    VaxSimulator::new().run_checked(program, 128).unwrap()
}

#[test]
fn state_restore_and_checked_loading_are_complete_and_atomic() {
    let mut cpu = VaxSimulator::new();
    cpu.write_register_checked(0, 0x1234_5678).unwrap();
    cpu.write_long_checked(17, 0x89ab_cdef).unwrap();
    cpu.set_psl(Psl {
        n: true,
        z: false,
        v: true,
        c: false,
    });
    let valid = cpu.get_state();
    assert_eq!(valid.memory.len(), MEMORY_SIZE);

    let mut invalid = valid.clone();
    invalid.memory.pop();
    assert!(matches!(
        cpu.restore(&invalid),
        Err(VaxError::InvalidState(_))
    ));
    assert_eq!(cpu.get_state(), valid);

    assert_eq!(
        cpu.load_at_checked(&[0; 8], (MEMORY_SIZE - 4) as u32),
        Err(VaxError::ProgramOutOfRange {
            origin: (MEMORY_SIZE - 4) as u32,
            length: 8,
            memory_size: MEMORY_SIZE,
        })
    );
    assert_eq!(cpu.get_state(), valid);

    cpu.load_at_checked(&[encoding::HALT], 0x100).unwrap();
    let loaded = cpu.get_state();
    assert_eq!(loaded.regs[REGISTER_PC], 0x100);
    assert_eq!(loaded.loaded_origin, 0x100);
    assert_eq!(loaded.loaded_len, 1);
    assert_eq!(loaded.memory[0x100], encoding::HALT);
    assert_eq!(loaded.memory[17], 0);
}

#[test]
fn little_endian_unaligned_memory_and_partial_register_writes_work() {
    let mut cpu = VaxSimulator::new();
    cpu.write_long_checked(3, 0x1122_3344).unwrap();
    assert_eq!(cpu.read_long_checked(3).unwrap(), 0x1122_3344);
    assert_eq!(cpu.read_word_checked(4).unwrap(), 0x2233);
    assert_eq!(cpu.read_byte_checked(6).unwrap(), 0x11);

    cpu.write_register_checked(1, 0xaabb_ccdd).unwrap();
    cpu.write_register_width_checked(1, 1, 0x11).unwrap();
    assert_eq!(cpu.read_register_checked(1).unwrap(), 0xaabb_cc11);
    cpu.write_register_width_checked(1, 2, 0x2233).unwrap();
    assert_eq!(cpu.read_register_checked(1).unwrap(), 0xaabb_2233);
    assert_eq!(
        cpu.read_register_checked(16),
        Err(VaxError::InvalidRegister { index: 16 })
    );
}

#[test]
fn literals_immediates_registers_and_deferred_modes_execute() {
    let program = encoding::assemble(&[
        encoding::movl(encoding::literal(42), encoding::register(0)),
        encoding::movl(encoding::immediate_long(0x1234_5678), encoding::register(1)),
        encoding::movl(encoding::register(1), encoding::register(2)),
        encoding::movl(encoding::register_deferred(3), encoding::register(4)),
        vec![encoding::HALT],
    ]);
    let mut cpu = VaxSimulator::new();
    cpu.load_checked(&program).unwrap();
    cpu.write_register_checked(3, 0x200).unwrap();
    cpu.write_long_checked(0x200, 0xfeed_face).unwrap();
    let result = cpu.run_loaded_checked(16).unwrap();
    assert_eq!(result.final_state.regs[0], 42);
    assert_eq!(result.final_state.regs[1], 0x1234_5678);
    assert_eq!(result.final_state.regs[2], 0x1234_5678);
    assert_eq!(result.final_state.regs[4], 0xfeed_face);
    assert_eq!(result.traces[0].raw, vec![0xd0, 42, 0x50]);
    assert_eq!(result.traces[0].mnemonic, "MOVL");
}

#[test]
fn autoincrement_autodecrement_absolute_displacement_and_index_modes_execute() {
    let program = encoding::assemble(&[
        encoding::movl(encoding::autoincrement(0), encoding::register(4)),
        encoding::movl(encoding::autodecrement(1), encoding::register(5)),
        encoding::movl(encoding::absolute(0x300), encoding::register(6)),
        encoding::movl(encoding::byte_displacement(2, 8), encoding::register(7)),
        encoding::movl(
            encoding::indexed(3, encoding::register_deferred(2)),
            encoding::register(8),
        ),
        encoding::movl(encoding::autoincrement_deferred(9), encoding::register(10)),
        vec![encoding::HALT],
    ]);
    let mut cpu = VaxSimulator::new();
    cpu.load_checked(&program).unwrap();
    cpu.write_register_checked(0, 0x200).unwrap();
    cpu.write_register_checked(1, 0x208).unwrap();
    cpu.write_register_checked(2, 0x220).unwrap();
    cpu.write_register_checked(3, 2).unwrap();
    cpu.write_register_checked(9, 0x250).unwrap();
    cpu.write_long_checked(0x200, 11).unwrap();
    cpu.write_long_checked(0x204, 12).unwrap();
    cpu.write_long_checked(0x228, 13).unwrap();
    cpu.write_long_checked(0x300, 14).unwrap();
    cpu.write_long_checked(0x250, 0x300).unwrap();
    let result = cpu.run_loaded_checked(16).unwrap();
    assert_eq!(result.final_state.regs[0], 0x204);
    assert_eq!(result.final_state.regs[1], 0x204);
    assert_eq!(result.final_state.regs[4..=8], [11, 12, 14, 13, 13]);
    assert_eq!(result.final_state.regs[9], 0x254);
    assert_eq!(result.final_state.regs[10], 14);
}

#[test]
fn byte_word_long_arithmetic_logic_and_flags_execute() {
    let program = encoding::assemble(&[
        encoding::instruction(
            0xc1,
            &[
                encoding::literal(7),
                encoding::literal(5),
                encoding::register(0),
            ],
        ),
        encoding::instruction(
            0xc3,
            &[
                encoding::literal(2),
                encoding::register(0),
                encoding::register(1),
            ],
        ),
        encoding::instruction(
            0xc5,
            &[
                encoding::literal(3),
                encoding::register(1),
                encoding::register(2),
            ],
        ),
        encoding::instruction(
            0xc9,
            &[
                encoding::literal(1),
                encoding::register(2),
                encoding::register(3),
            ],
        ),
        encoding::instruction(
            0xcb,
            &[
                encoding::literal(8),
                encoding::register(3),
                encoding::register(4),
            ],
        ),
        encoding::instruction(
            0xcd,
            &[
                encoding::literal(3),
                encoding::register(4),
                encoding::register(5),
            ],
        ),
        encoding::instruction(0x91, &[encoding::literal(26), encoding::register(5)]),
        vec![encoding::HALT],
    ]);
    let result = run(&program);
    assert_eq!(&result.final_state.regs[..6], &[12, 10, 30, 31, 23, 20]);
    assert_eq!(
        result.final_state.psl,
        Psl {
            n: false,
            z: false,
            v: false,
            c: false
        }
    );
}

#[test]
fn move_unary_zero_extend_address_and_stack_operations_execute() {
    let program = encoding::assemble(&[
        encoding::instruction(
            0xd0,
            &[encoding::immediate_long(0xffff_80ff), encoding::register(0)],
        ),
        encoding::instruction(0x9a, &[encoding::register(0), encoding::register(1)]),
        encoding::instruction(0x9b, &[encoding::register(0), encoding::register(2)]),
        encoding::instruction(0xd2, &[encoding::register(1), encoding::register(3)]),
        encoding::instruction(0xd4, &[encoding::register(4)]),
        encoding::instruction(0xd6, &[encoding::register(4)]),
        encoding::instruction(0xd7, &[encoding::register(4)]),
        encoding::instruction(0xde, &[encoding::absolute(0x345), encoding::register(5)]),
        encoding::instruction(0xdd, &[encoding::register(5)]),
        vec![encoding::HALT],
    ]);
    let mut cpu = VaxSimulator::new();
    cpu.load_checked(&program).unwrap();
    cpu.write_register_checked(REGISTER_SP, 0x400).unwrap();
    let result = cpu.run_loaded_checked(32).unwrap();
    assert_eq!(result.final_state.regs[1], 0xff);
    assert_eq!(result.final_state.regs[2], 0x80ff);
    assert_eq!(result.final_state.regs[3], !0xff);
    assert_eq!(result.final_state.regs[4], 0);
    assert_eq!(result.final_state.regs[5], 0x345);
    assert_eq!(result.final_state.regs[REGISTER_SP], 0x3fc);
    assert_eq!(
        &result.final_state.memory[0x3fc..0x400],
        &[0x45, 0x03, 0, 0]
    );
}

#[test]
fn branches_subroutines_and_returns_execute() {
    let program = encoding::assemble(&[
        encoding::instruction(0xd4, &[encoding::register(0)]),
        encoding::branch_byte(0x13, 3),
        encoding::movl(encoding::literal(1), encoding::register(1)),
        encoding::branch_byte(0x10, 4),
        encoding::movl(encoding::literal(2), encoding::register(1)),
        vec![encoding::HALT],
        encoding::movl(encoding::literal(7), encoding::register(2)),
        vec![0x05],
    ]);
    let mut cpu = VaxSimulator::new();
    cpu.load_checked(&program).unwrap();
    cpu.write_register_checked(REGISTER_SP, 0x500).unwrap();
    let result = cpu.run_loaded_checked(32).unwrap();
    assert_eq!(result.final_state.regs[1], 2);
    assert_eq!(result.final_state.regs[2], 7);
    assert_eq!(result.final_state.regs[REGISTER_SP], 0x500);
}

#[test]
fn typed_faults_and_run_limits_are_atomic() {
    let cases = [
        (
            vec![0xfc],
            VaxError::UnsupportedTwoByteOpcode {
                prefix: 0xfc,
                pc: 0,
            },
        ),
        (
            vec![0xff],
            VaxError::UnsupportedTwoByteOpcode {
                prefix: 0xff,
                pc: 0,
            },
        ),
        (
            vec![0xd0, 0x8f, 1],
            VaxError::TruncatedInstruction { pc: 0 },
        ),
        (
            vec![0xd0, 0x8f, 0, 0, 0, 0, 0x00],
            VaxError::InvalidDestinationMode {
                specifier: 0,
                pc: 0,
            },
        ),
    ];
    for (program, expected) in cases {
        let mut cpu = VaxSimulator::new();
        cpu.load_checked(&program).unwrap();
        let before = cpu.get_state();
        assert_eq!(cpu.step_checked(), Err(expected));
        assert_eq!(cpu.get_state(), before);
    }

    let mut cpu = VaxSimulator::new();
    let divide = encoding::assemble(&[
        encoding::instruction(
            0xc7,
            &[
                encoding::literal(0),
                encoding::literal(4),
                encoding::register(0),
            ],
        ),
        vec![encoding::HALT],
    ]);
    cpu.load_checked(&divide).unwrap();
    let before = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(VaxError::DivideByZero { pc: 0 }));
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&[encoding::NOP, 0x11, 0xfd]).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_loaded_checked(3),
        Err(VaxError::StepLimitExceeded { max_steps: 3 })
    );
    assert_eq!(cpu.get_state(), before);

    cpu.load_checked(&[encoding::HALT]).unwrap();
    cpu.step_checked().unwrap();
    let halted = cpu.get_state();
    assert_eq!(cpu.step_checked(), Err(VaxError::Halted));
    assert_eq!(cpu.get_state(), halted);
}
