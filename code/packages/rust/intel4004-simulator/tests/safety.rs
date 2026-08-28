use intel4004_simulator::{
    encode_fim, encode_hlt, encode_jun, encode_ldm, Intel4004Error, Intel4004Simulator,
};

#[test]
fn load_reset_and_snapshot_are_deterministic() {
    let mut simulator = Intel4004Simulator::new(8);
    simulator.run(&[encode_ldm(9), encode_hlt()], 8).unwrap();
    let completed = simulator.snapshot();
    assert_eq!(completed.accumulator, 9);

    let before_error = simulator.snapshot();
    assert_eq!(
        simulator.load_program(&[0; 9]),
        Err(Intel4004Error::ProgramTooLarge {
            bytes: 9,
            capacity: 8,
        })
    );
    assert_eq!(simulator.snapshot(), before_error);

    simulator.load_program(&[encode_hlt()]).unwrap();
    assert_eq!(simulator.memory, vec![encode_hlt(), 0, 0, 0, 0, 0, 0, 0]);
    simulator.reset();
    let reset_once = simulator.snapshot();
    simulator.reset();
    assert_eq!(simulator.snapshot(), reset_once);
}

#[test]
fn fetch_decode_and_target_errors_are_atomic() {
    let mut truncated = Intel4004Simulator::new(1);
    truncated.load_program(&[encode_fim(0, 0).0]).unwrap();
    let before = truncated.snapshot();
    assert_eq!(
        truncated.step(),
        Err(Intel4004Error::TruncatedInstruction { address: 0 })
    );
    assert_eq!(truncated.snapshot(), before);

    let mut unknown = Intel4004Simulator::new(2);
    unknown.load_program(&[0x02, encode_hlt()]).unwrap();
    let before = unknown.snapshot();
    assert_eq!(
        unknown.step(),
        Err(Intel4004Error::UnknownOpcode {
            address: 0,
            opcode: 0x02,
        })
    );
    assert_eq!(unknown.snapshot(), before);

    let mut jump = Intel4004Simulator::new(2);
    let (first, second) = encode_jun(0xFFF);
    jump.load_program(&[first, second]).unwrap();
    let before = jump.snapshot();
    assert_eq!(
        jump.step(),
        Err(Intel4004Error::AddressOutOfRange {
            address: 0xFFF,
            capacity: 2,
        })
    );
    assert_eq!(jump.snapshot(), before);
}

#[test]
fn indirect_fetch_and_invalid_legacy_state_fail_closed() {
    let mut indirect = Intel4004Simulator::new(16);
    indirect.load_program(&[0x30]).unwrap();
    indirect.registers[0] = 0xF;
    indirect.registers[1] = 0xF;
    let before = indirect.snapshot();
    assert_eq!(
        indirect.step(),
        Err(Intel4004Error::AddressOutOfRange {
            address: 0xFF,
            capacity: 16,
        })
    );
    assert_eq!(indirect.snapshot(), before);

    let mut corrupted = Intel4004Simulator::new(16);
    corrupted.load_program(&[encode_hlt()]).unwrap();
    corrupted.ram_bank = 4;
    let before = corrupted.snapshot();
    assert_eq!(
        corrupted.step(),
        Err(Intel4004Error::InvalidState("RAM selector"))
    );
    assert_eq!(corrupted.snapshot(), before);
}

#[test]
fn bounded_execution_stops_at_the_requested_limit() {
    let mut simulator = Intel4004Simulator::new(16);
    let traces = simulator.run(&[encode_ldm(1)], 7).unwrap();
    assert_eq!(traces.len(), 7);
    assert!(!simulator.halted);
    assert_eq!(simulator.pc, 7);

    let before = simulator.snapshot();
    assert_eq!(simulator.step().unwrap().address, 7);
    assert_ne!(simulator.snapshot(), before);
}

#[test]
fn every_typed_error_is_stable_and_public_state_checks_are_atomic() {
    let errors = [
        Intel4004Error::ProgramTooLarge {
            bytes: 2,
            capacity: 1,
        },
        Intel4004Error::AddressOutOfRange {
            address: 2,
            capacity: 1,
        },
        Intel4004Error::TruncatedInstruction { address: 0 },
        Intel4004Error::UnknownOpcode {
            address: 0,
            opcode: 0xFF,
        },
        Intel4004Error::Halted,
        Intel4004Error::InvalidState("register file"),
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }

    let mut no_rom = Intel4004Simulator::new(0);
    let before = no_rom.snapshot();
    assert_eq!(
        no_rom.step(),
        Err(Intel4004Error::AddressOutOfRange {
            address: 0,
            capacity: 0,
        })
    );
    assert_eq!(no_rom.snapshot(), before);

    let mut invalid_registers = Intel4004Simulator::new(1);
    invalid_registers.load_program(&[encode_hlt()]).unwrap();
    invalid_registers.registers.pop();
    let before = invalid_registers.snapshot();
    assert_eq!(
        invalid_registers.step(),
        Err(Intel4004Error::InvalidState("register file"))
    );
    assert_eq!(invalid_registers.snapshot(), before);

    let mut invalid_stack = Intel4004Simulator::new(1);
    invalid_stack.load_program(&[encode_hlt()]).unwrap();
    invalid_stack.stack_pointer = 3;
    let before = invalid_stack.snapshot();
    assert_eq!(
        invalid_stack.step(),
        Err(Intel4004Error::InvalidState("stack pointer"))
    );
    assert_eq!(invalid_stack.snapshot(), before);
}
