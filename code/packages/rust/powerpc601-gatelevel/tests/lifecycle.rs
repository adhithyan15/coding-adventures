use coding_adventures_powerpc601_gatelevel::{PowerPc601GateSimulator, PowerPcError};
use powerpc601_simulator::encoding::{assemble, d_form, halt, i_form};

#[test]
fn dff_state_restore_and_direct_access_are_checked() {
    let mut cpu = PowerPc601GateSimulator::new();
    cpu.write_register_checked(7, 0x1234_5678).unwrap();
    cpu.write_byte_checked(0x100, 0xaa).unwrap();
    cpu.write_half_checked(0x102, 0xbeef).unwrap();
    cpu.write_word_checked(0x104, 0x1122_3344).unwrap();
    let saved = cpu.get_state();
    cpu.reset();
    cpu.restore(&saved).unwrap();
    assert_eq!(cpu.read_register_checked(7).unwrap(), 0x1234_5678);
    assert_eq!(cpu.read_byte_checked(0x100).unwrap(), 0xaa);
    assert_eq!(cpu.read_half_checked(0x102).unwrap(), 0xbeef);
    assert_eq!(cpu.read_word_checked(0x104).unwrap(), 0x1122_3344);
    assert_eq!(
        cpu.read_register_checked(32),
        Err(PowerPcError::InvalidRegister { index: 32 })
    );
    assert_eq!(
        cpu.read_word_checked(0x102),
        Err(PowerPcError::MisalignedAccess {
            address: 0x102,
            width: 4,
        })
    );
}

#[test]
fn load_step_and_complete_traces_match_functional_lifecycle() {
    let program = assemble(&[d_form(14, 1, 0, 42), halt()]);
    let mut cpu = PowerPc601GateSimulator::new();
    let result = cpu.run_checked(&program, 4).unwrap();
    assert!(result.halted);
    assert_eq!(result.steps, 2);
    assert_eq!(result.final_state.gpr[1], 42);
    assert_eq!(result.traces[0].state_before.gpr[1], 0);
    assert_eq!(result.traces[0].state_after.gpr[1], 42);
}

#[test]
fn failed_bounded_run_rolls_back_every_dff() {
    let mut cpu = PowerPc601GateSimulator::new();
    cpu.load_checked(&assemble(&[i_form(18, 0, false, false)]))
        .unwrap();
    let before = cpu.get_state();
    assert_eq!(
        cpu.run_loaded_checked(3),
        Err(PowerPcError::StepLimitExceeded { max_steps: 3 })
    );
    assert_eq!(cpu.get_state(), before);
}
