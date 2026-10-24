use intel8085_simulator::{Intel8085Error, Intel8085Simulator, InterruptRequest, RestartMasks};

const UNDOCUMENTED: [u8; 10] = [0x08, 0x10, 0x18, 0x28, 0x38, 0xCB, 0xD9, 0xDD, 0xED, 0xFD];

fn encoded_length(opcode: u8) -> Option<usize> {
    if UNDOCUMENTED.contains(&opcode) {
        None
    } else if opcode & 0xC7 == 0x06 || opcode & 0xC7 == 0xC6 || matches!(opcode, 0xD3 | 0xDB) {
        Some(2)
    } else if opcode & 0xCF == 0x01
        || opcode & 0xC7 == 0xC2
        || opcode & 0xC7 == 0xC4
        || matches!(opcode, 0x22 | 0x2A | 0x32 | 0x3A | 0xC3 | 0xCD)
    {
        Some(3)
    } else {
        Some(1)
    }
}

fn ready_simulator() -> Intel8085Simulator {
    let mut simulator = Intel8085Simulator::new(65_536);
    simulator.load_program(&[0; 256]).unwrap();
    simulator.core.regs.sp = 0x4000;
    simulator
}

#[test]
fn all_first_bytes_have_the_documented_8085_classification_and_width() {
    for opcode in 0u8..=u8::MAX {
        let mut simulator = Intel8085Simulator::new(65_536);
        simulator.load_program(&[opcode, 0, 0]).unwrap();
        simulator.core.regs.sp = 0x4000;
        let before = simulator.snapshot();

        match encoded_length(opcode) {
            Some(length) => {
                let trace = simulator.step().unwrap_or_else(|error| {
                    panic!("documented opcode {opcode:#04X} was rejected: {error}")
                });
                assert_eq!(trace.address, 0, "opcode {opcode:#04X}");
                assert_eq!(trace.raw.len(), length, "opcode {opcode:#04X}");
                assert_eq!(trace.state_before, before, "opcode {opcode:#04X}");
                assert_eq!(trace.state_after, simulator.snapshot());
            }
            None => {
                assert_eq!(
                    simulator.step(),
                    Err(Intel8085Error::UnknownOpcode { address: 0, opcode })
                );
                assert_eq!(simulator.snapshot(), before);
            }
        }
    }
}

#[test]
fn rim_reports_serial_masks_enable_and_pending_restart_requests() {
    let mut simulator = Intel8085Simulator::new(65_536);
    simulator.load_program(&[0x20]).unwrap();
    simulator.serial_input = true;
    simulator.core.interrupts_enabled = true;
    simulator.request_interrupt(InterruptRequest::Rst75);
    simulator.request_interrupt(InterruptRequest::Rst65);
    simulator.request_interrupt(InterruptRequest::Rst55);

    let trace = simulator.step().unwrap();
    assert_eq!(trace.mnemonic, "RIM");
    assert_eq!(trace.raw, [0x20]);
    assert_eq!(simulator.core.regs.a, 0xFF);
    assert_eq!(simulator.pending().restart_bits(), 0b111);
    assert!(simulator.core.interrupts_enabled);
}

#[test]
fn sim_updates_only_enabled_masks_serial_output_and_rst75_reset() {
    let mut simulator = Intel8085Simulator::new(65_536);
    simulator.load_program(&[0x30, 0x30]).unwrap();
    simulator.request_interrupt(InterruptRequest::Rst75);
    simulator.core.regs.a = 0b1101_1001; // SOD=1, SOE=1, R7.5=1, MSE=1, M5.5=1

    let trace = simulator.step().unwrap();
    assert_eq!(trace.mnemonic, "SIM");
    assert!(simulator.serial_output);
    assert_eq!(
        simulator.restart_masks,
        RestartMasks {
            rst75: false,
            rst65: false,
            rst55: true,
        }
    );
    assert!(!simulator.pending().rst75);

    simulator.core.regs.a = 0; // MSE=0 and SOE=0 preserve both groups.
    simulator.step().unwrap();
    assert!(simulator.serial_output);
    assert_eq!(
        simulator.restart_masks,
        RestartMasks {
            rst75: false,
            rst65: false,
            rst55: true,
        }
    );
}

#[test]
fn ei_waits_for_the_following_instruction_before_accepting_an_interrupt() {
    let mut simulator = Intel8085Simulator::new(65_536);
    simulator.load_program(&[0xFB, 0x00, 0x76]).unwrap(); // EI; NOP; HLT
    simulator.core.regs.sp = 0x4000;
    simulator.restart_masks.rst75 = false;
    simulator.request_interrupt(InterruptRequest::Rst75);

    assert_eq!(simulator.step().unwrap().mnemonic, "ei");
    assert!(!simulator.core.interrupts_enabled);
    assert_eq!(simulator.core.pc, 1);

    assert_eq!(simulator.step().unwrap().mnemonic, "nop");
    assert!(simulator.core.interrupts_enabled);
    assert_eq!(simulator.core.pc, 2);

    let interrupt = simulator.step().unwrap();
    assert_eq!(interrupt.mnemonic, "INT RST7.5");
    assert!(interrupt.raw.is_empty());
    assert_eq!(simulator.core.pc, 0x003C);
    assert_eq!(simulator.core.regs.sp, 0x3FFE);
    assert_eq!(simulator.core.mem.read_byte(0x3FFE), 2);
    assert_eq!(simulator.core.mem.read_byte(0x3FFF), 0);
    assert!(!simulator.core.interrupts_enabled);
}

#[test]
fn interrupt_priority_stack_order_and_halt_release_are_exact() {
    let mut simulator = ready_simulator();
    simulator.core.regs.sp = 0x0100;
    simulator.core.pc = 0x0012;
    simulator.restart_masks = RestartMasks::unmasked();
    simulator.core.interrupts_enabled = true;
    simulator.request_interrupt(InterruptRequest::Rst55);
    simulator.request_interrupt(InterruptRequest::Rst65);
    simulator.request_interrupt(InterruptRequest::Rst75);
    simulator.request_interrupt(InterruptRequest::Trap);

    let trap = simulator.step().unwrap();
    assert_eq!(trap.mnemonic, "INT TRAP");
    assert_eq!(simulator.core.pc, 0x0024);
    assert_eq!(simulator.core.regs.sp, 0x00FE);
    assert_eq!(simulator.core.mem.read_byte(0x00FE), 0x12);
    assert_eq!(simulator.core.mem.read_byte(0x00FF), 0x00);

    simulator.core.interrupts_enabled = true;
    let rst75 = simulator.step().unwrap();
    assert_eq!(rst75.mnemonic, "INT RST7.5");
    assert_eq!(simulator.core.pc, 0x003C);

    let mut halted = Intel8085Simulator::new(65_536);
    halted.load_program(&[0x76]).unwrap();
    halted.core.regs.sp = 0x0200;
    halted.step().unwrap();
    assert!(halted.core.halted);
    halted.request_interrupt(InterruptRequest::Trap);
    assert_eq!(halted.step().unwrap().mnemonic, "INT TRAP");
    assert!(!halted.core.halted);
}

#[test]
fn intr_accepts_restart_acknowledgements_and_rejects_other_bytes_atomically() {
    let mut simulator = ready_simulator();
    simulator.core.regs.sp = 0x3000;
    simulator.core.interrupts_enabled = true;
    simulator.request_interrupt(InterruptRequest::Intr(0xDF)); // RST 3
    assert_eq!(simulator.step().unwrap().mnemonic, "INT INTR/RST3");
    assert_eq!(simulator.core.pc, 0x0018);

    let mut invalid = ready_simulator();
    invalid.core.interrupts_enabled = true;
    invalid.request_interrupt(InterruptRequest::Intr(0x00));
    let before = invalid.snapshot();
    assert_eq!(
        invalid.step(),
        Err(Intel8085Error::UnsupportedInterruptOpcode { opcode: 0x00 })
    );
    assert_eq!(invalid.snapshot(), before);
}

#[test]
fn load_fetch_stack_and_transaction_failures_are_typed_and_atomic() {
    let mut simulator = Intel8085Simulator::new(8);
    assert_eq!(
        simulator.load_program_at(&[1, 2], 7),
        Err(Intel8085Error::ProgramOutOfRange {
            origin: 7,
            length: 2,
            memory_size: 8,
        })
    );

    simulator.load_program_at(&[0x3E], 4).unwrap(); // truncated MVI A
    simulator.core.pc = 4;
    let truncated = simulator.snapshot();
    assert_eq!(
        simulator.step(),
        Err(Intel8085Error::TruncatedInstruction {
            address: 4,
            expected: 2,
            available: 1,
        })
    );
    assert_eq!(simulator.snapshot(), truncated);

    simulator.core.pc = 3;
    let outside = simulator.snapshot();
    assert_eq!(
        simulator.step(),
        Err(Intel8085Error::FetchOutsideProgram {
            address: 3,
            origin: 4,
            length: 1,
        })
    );
    assert_eq!(simulator.snapshot(), outside);

    let mut stack = Intel8085Simulator::new(32);
    stack.load_program(&[0]).unwrap();
    stack.core.regs.sp = 0;
    stack.request_interrupt(InterruptRequest::Trap);
    let stack_before = stack.snapshot();
    assert_eq!(
        stack.step(),
        Err(Intel8085Error::StackOutOfRange {
            address: 0xFFFE,
            memory_size: 32,
        })
    );
    assert_eq!(stack.snapshot(), stack_before);

    let mut transactional = Intel8085Simulator::new(65_536);
    transactional.core.regs.a = 0xA5;
    transactional.serial_input = true;
    let transaction_before = transactional.snapshot();
    assert_eq!(
        transactional.run(&[0x3E, 7, 0x08], 10),
        Err(Intel8085Error::UnknownOpcode {
            address: 2,
            opcode: 0x08,
        })
    );
    assert_eq!(transactional.snapshot(), transaction_before);
}

#[test]
fn reset_runs_and_snapshots_cover_the_complete_owned_state() {
    let program = [0x3E, 0xC0, 0x30, 0x76]; // MVI A,C0; SIM; HLT
    let mut simulator = Intel8085Simulator::new(65_536);
    simulator.serial_input = true;
    simulator.core.input_ports[7] = 0xA5;
    let first = simulator.run(&program, 10).unwrap();
    let owned = simulator.snapshot();
    assert!(first.halted);
    assert_eq!(first.steps, 3);
    assert_eq!(first.final_state, owned);
    assert!(owned.serial_output);
    assert!(owned.serial_input);
    assert_eq!(owned.loaded_origin, 0);
    assert_eq!(owned.loaded_len, program.len());

    simulator.core.regs.a = 0;
    simulator.serial_output = false;
    simulator.core.mem.write_byte(0, 0);
    assert_eq!(owned.core.regs.a, 0xC0);
    assert!(owned.serial_output);
    assert_eq!(owned.core.memory[0], 0x3E);

    simulator.reset();
    let reset = simulator.snapshot();
    assert!(reset.core.memory.iter().all(|byte| *byte == 0));
    assert_eq!(reset.core.input_ports, [0; 256]);
    assert_eq!(reset.core.output_ports, [0; 256]);
    assert_eq!(reset.restart_masks, RestartMasks::masked());
    assert_eq!(reset.loaded_len, 0);
    assert!(!reset.serial_input);
    assert!(!reset.serial_output);
}
