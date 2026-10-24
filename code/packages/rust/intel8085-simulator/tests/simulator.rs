use intel8080_simulator::Intel8080Error;
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

#[test]
fn remaining_interrupt_paths_and_bounded_run_contract_are_covered() {
    let mut rst65 = ready_simulator();
    rst65.restart_masks = RestartMasks::unmasked();
    rst65.core.interrupts_enabled = true;
    rst65.request_interrupt(InterruptRequest::Rst65);
    assert_eq!(rst65.step().unwrap().mnemonic, "INT RST6.5");
    assert_eq!(rst65.core.pc, 0x0034);

    let mut rst55 = ready_simulator();
    rst55.restart_masks = RestartMasks::unmasked();
    rst55.core.interrupts_enabled = true;
    rst55.request_interrupt(InterruptRequest::Rst55);
    assert_eq!(rst55.step().unwrap().mnemonic, "INT RST5.5");
    assert_eq!(rst55.core.pc, 0x002C);

    let mut trapped_halt = Intel8085Simulator::new(65_536);
    trapped_halt.load_program(&[0x76]).unwrap();
    trapped_halt.core.regs.sp = 0x1000;
    trapped_halt.step().unwrap();
    trapped_halt.request_interrupt(InterruptRequest::Trap);
    let result = trapped_halt.run_loaded_with_limit(1).unwrap();
    assert_eq!(result.steps, 1);
    assert_eq!(result.pc, 0x0024);
    assert!(!result.halted);

    let mut bounded = Intel8085Simulator::new(65_536);
    bounded.load_program(&[0x00, 0x76]).unwrap();
    let one = bounded.run_loaded_with_limit(1).unwrap();
    assert_eq!(one.steps, 1);
    assert!(!one.halted);
    let completed = bounded.run_loaded().unwrap();
    assert_eq!(completed.steps, 1);
    assert!(completed.halted);
    assert_eq!(
        bounded.step(),
        Err(Intel8085Error::Halted),
        "a post-HLT step without a serviceable interrupt is typed"
    );
}

#[test]
fn short_memory_data_fault_and_oversized_run_are_atomic() {
    let mut data = Intel8085Simulator::new(8);
    data.load_program(&[0x7E]).unwrap(); // MOV A,M
    data.core.regs.h = 1;
    let before = data.snapshot();
    assert_eq!(
        data.step(),
        Err(Intel8085Error::MemoryOutOfRange {
            address: 0x0100,
            memory_size: 8,
        })
    );
    assert_eq!(data.snapshot(), before);

    let mut run = Intel8085Simulator::new(2);
    let before = run.snapshot();
    assert_eq!(
        run.run(&[0, 0, 0], 1),
        Err(Intel8085Error::ProgramOutOfRange {
            origin: 0,
            length: 3,
            memory_size: 2,
        })
    );
    assert_eq!(run.snapshot(), before);
}

#[test]
fn errors_have_stable_messages_and_core_failures_map_without_information_loss() {
    assert_eq!(RestartMasks::default(), RestartMasks::masked());

    let errors = [
        Intel8085Error::ProgramOutOfRange {
            origin: 7,
            length: 2,
            memory_size: 8,
        },
        Intel8085Error::FetchOutsideProgram {
            address: 3,
            origin: 4,
            length: 1,
        },
        Intel8085Error::TruncatedInstruction {
            address: 4,
            expected: 2,
            available: 1,
        },
        Intel8085Error::UnknownOpcode {
            address: 2,
            opcode: 0x08,
        },
        Intel8085Error::Halted,
        Intel8085Error::MemoryOutOfRange {
            address: 0x0100,
            memory_size: 8,
        },
        Intel8085Error::StackOutOfRange {
            address: 0xFFFE,
            memory_size: 32,
        },
        Intel8085Error::UnsupportedInterruptOpcode { opcode: 0 },
    ];
    let rendered = errors.map(|error| error.to_string());
    assert!(rendered[0].contains("program of 2 bytes"));
    assert!(rendered[1].contains("outside installed program"));
    assert!(rendered[2].contains("needs 2 bytes"));
    assert!(rendered[3].contains("unknown opcode 0x08"));
    assert_eq!(rendered[4], "CPU is halted");
    assert!(rendered[5].contains("address 0x0100"));
    assert!(rendered[6].contains("stack address 0xFFFE"));
    assert!(rendered[7].contains("not a RST instruction"));

    let mapped = [
        Intel8080Error::ProgramOutOfRange {
            length: 9,
            memory_size: 8,
        },
        Intel8080Error::TruncatedInstruction {
            address: 7,
            expected: 2,
            available: 1,
        },
        Intel8080Error::UnknownOpcode {
            address: 1,
            opcode: 0x08,
        },
        Intel8080Error::Halted,
        Intel8080Error::MemoryOutOfRange {
            address: 9,
            memory_size: 8,
        },
    ]
    .map(Intel8085Error::from);
    assert_eq!(
        mapped[0],
        Intel8085Error::ProgramOutOfRange {
            origin: 0,
            length: 9,
            memory_size: 8,
        }
    );
    assert_eq!(
        mapped[1],
        Intel8085Error::TruncatedInstruction {
            address: 7,
            expected: 2,
            available: 1,
        }
    );
    assert_eq!(
        mapped[2],
        Intel8085Error::UnknownOpcode {
            address: 1,
            opcode: 0x08,
        }
    );
    assert_eq!(mapped[3], Intel8085Error::Halted);
    assert_eq!(
        mapped[4],
        Intel8085Error::MemoryOutOfRange {
            address: 9,
            memory_size: 8,
        }
    );
}
