use microwatt_powerisa_simulator::encoding::{assemble, attn, d_form, i_form, xo_form};
use microwatt_powerisa_simulator::{Error, MicrowattPowerIsaSimulator, State, MEMORY_SIZE};

#[test]
fn failed_load_restore_and_direct_access_are_atomic() {
    let mut cpu = MicrowattPowerIsaSimulator::new();
    cpu.load_program(&assemble(&[attn()]), 0x100).unwrap();
    cpu.write_register(7, 99).unwrap();
    let before = cpu.get_state();
    assert!(matches!(
        cpu.load_program(&[1, 2, 3, 4], 65_536),
        Err(Error::ProgramOutOfRange { .. })
    ));
    assert_eq!(cpu.get_state(), before);
    assert_eq!(
        cpu.write_register(32, 1),
        Err(Error::InvalidRegister { index: 32 })
    );
    assert_eq!(cpu.get_state(), before);
    assert!(matches!(
        cpu.write_word(65_534, 1),
        Err(Error::MisalignedAccess { .. })
    ));
    assert_eq!(cpu.get_state(), before);

    let mut invalid = before.clone();
    invalid.cia = 3;
    assert!(matches!(cpu.restore(&invalid), Err(Error::InvalidState(_))));
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn step_faults_restore_every_byte_and_register() {
    let mut cpu = MicrowattPowerIsaSimulator::new();
    cpu.load_program(&assemble(&[d_form(33, 3, 3, 4)]), 0x100)
        .unwrap();
    cpu.write_register(3, 0x200).unwrap(); // lwzu with RA=RT is reserved
    let before = cpu.get_state();
    assert!(matches!(cpu.step(), Err(Error::IllegalInstruction { .. })));
    assert_eq!(cpu.get_state(), before);

    let mut cpu = MicrowattPowerIsaSimulator::new();
    cpu.load_program(&assemble(&[xo_form(5, 3, 4, false, 491, false)]), 0x100)
        .unwrap();
    cpu.write_register(3, 123).unwrap();
    cpu.write_register(4, 0).unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.step(),
        Err(Error::DivideFault {
            cia: 0x100,
            signed: true
        })
    );
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn failed_run_and_execute_restore_the_call_entry_state() {
    let infinite = assemble(&[i_form(0, false, false)]);
    let mut cpu = MicrowattPowerIsaSimulator::new();
    cpu.load_program(&infinite, 0x100).unwrap();
    cpu.write_register(9, 0xfeed).unwrap();
    let before = cpu.get_state();
    assert_eq!(cpu.run(3), Err(Error::StepLimit { max_steps: 3 }));
    assert_eq!(cpu.get_state(), before);

    let replacement = assemble(&[d_form(14, 9, 0, 1), i_form(0, false, false)]);
    assert_eq!(
        cpu.execute(&replacement, 0x200, 2),
        Err(Error::StepLimit { max_steps: 2 })
    );
    assert_eq!(cpu.get_state(), before);
}

#[test]
fn fetch_precedence_and_exact_truncated_bytes_are_stable() {
    let mut cpu = MicrowattPowerIsaSimulator::new();
    cpu.load_program(&[0xaa, 0xbb], 0x100).unwrap();
    assert_eq!(
        cpu.step(),
        Err(Error::TruncatedInstruction {
            cia: 0x100,
            bytes: vec![0xaa, 0xbb]
        })
    );
    let mut state = cpu.get_state();
    state.cia = 0x102;
    // Restore rejects malformed fetch alignment before it can enter the CPU.
    assert!(matches!(cpu.restore(&state), Err(Error::InvalidState(_))));
}

#[test]
fn branch_may_target_residual_installed_bytes_then_fetch_reports_truncation() {
    let mut bytes = assemble(&[i_form(4, false, false)]);
    bytes.extend_from_slice(&[0xaa, 0xbb]);
    let mut cpu = MicrowattPowerIsaSimulator::new();
    cpu.load_program(&bytes, 0x100).unwrap();
    let trace = cpu.step().unwrap();
    assert_eq!(trace.cia_after, 0x104);
    assert_eq!(
        cpu.step(),
        Err(Error::TruncatedInstruction {
            cia: 0x104,
            bytes: vec![0xaa, 0xbb]
        })
    );
}

#[test]
fn restore_accepts_the_one_past_memory_sentinel_but_not_beyond() {
    let mut cpu = MicrowattPowerIsaSimulator::new();
    let mut state = State {
        cia: MEMORY_SIZE as u64,
        ..State::default()
    };
    cpu.restore(&state).unwrap();
    state.cia += 4;
    assert!(matches!(cpu.restore(&state), Err(Error::InvalidState(_))));
}

#[test]
fn retained_traces_obey_both_count_and_byte_budgets() {
    let mut words = vec![d_form(14, 3, 3, 1); 70];
    words.push(attn());
    let mut cpu = MicrowattPowerIsaSimulator::new();
    let result = cpu.execute(&assemble(&words), 0, 80).unwrap();
    assert_eq!(result.steps, 71);
    assert!(result.traces.len() <= 64);
    assert!(result.traces.len() * std::mem::size_of_val(&result.traces[0]) <= 8 * 1024 * 1024);
}
