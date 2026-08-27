use coding_adventures_ge225_simulator::{
    assemble_card_io, assemble_controller_select, assemble_controller_status, assemble_fixed,
    assemble_select_x_group, encode_instruction, Simulator,
};

const PROGRAM: i32 = 0o1000;
const READY: u8 = 0o20;

fn simulator_with_program(words: &[i32]) -> Simulator {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator.load_words(words, PROGRAM).unwrap();
    simulator.set_program_counter(PROGRAM).unwrap();
    simulator
}

#[test]
fn selector_and_status_words_match_the_manual_formats() {
    let simulator = Simulator::new(4096).unwrap();

    assert_eq!(assemble_controller_select(3, 0).unwrap(), 0o2500320);
    assert_eq!(assemble_controller_select(3, 2).unwrap(), 0o2540320);
    assert_eq!(
        simulator
            .disassemble_word(assemble_controller_select(3, 2).unwrap())
            .unwrap(),
        "SEL P3,X2"
    );
    assert_eq!(
        assemble_controller_status(6, 0o25, true).unwrap(),
        0o2514625
    );
    assert_eq!(
        assemble_controller_status(6, 0o25, false).unwrap(),
        0o2516625
    );
    assert_eq!(
        simulator
            .disassemble_word(assemble_controller_status(6, 0o25, true).unwrap())
            .unwrap(),
        "BCS 25,P6,SET"
    );

    assert!(assemble_controller_select(-1, 0).is_err());
    assert!(assemble_controller_select(8, 0).is_err());
    assert!(assemble_controller_select(0, 4).is_err());
    assert!(assemble_controller_status(0, 0o17, true).is_err());
    assert!(assemble_controller_status(0, 0o36, true).is_err());

    let mut invalid_error = Simulator::new(4096).unwrap();
    assert!(invalid_error
        .set_controller_error_condition(0, READY, true)
        .is_err());
}

#[test]
fn select_delivers_two_opaque_words_and_skips_them() {
    let mut simulator = simulator_with_program(&[
        assemble_controller_select(2, 0).unwrap(),
        0o1234567,
        0o0765432,
        assemble_fixed("NOP").unwrap(),
    ]);
    simulator
        .set_controller_error_condition(2, 0o25, true)
        .unwrap();

    let trace = simulator.step().unwrap();
    let state = simulator.get_state();

    assert_eq!(trace.address, PROGRAM);
    assert_eq!(state.pc, PROGRAM + 3);
    assert!(state.controller_selector_busy);
    assert_eq!(state.selected_controller, Some(2));
    assert!(!state.controllers[2].ready);
    assert!(!state.controllers[2].error);
    assert_eq!(state.controllers[2].error_conditions, 0);
    assert_eq!(state.controllers[2].conditions & (1_u64 << 0o25), 0);
    assert_eq!(
        simulator.controller_commands(),
        &[coding_adventures_ge225_simulator::ControllerCommand {
            plug: 2,
            select_word: 0o2500220,
            command_word: 0o1234567,
            address_word: 0o0765432,
        }]
    );

    assert!(simulator.advance_controller_selector());
    assert!(!simulator.advance_controller_selector());
    simulator
        .complete_controller(2, 1_u64 << 0o23, false)
        .unwrap();
    let completed = simulator.get_state();
    assert!(completed.controllers[2].ready);
    assert_ne!(completed.controllers[2].conditions & (1_u64 << READY), 0);
    assert_ne!(completed.controllers[2].conditions & (1_u64 << 0o23), 0);
}

#[test]
fn busy_or_offline_selector_alert_halts_without_delivering_a_second_command() {
    let mut simulator = simulator_with_program(&[
        assemble_controller_select(1, 0).unwrap(),
        1,
        2,
        assemble_controller_select(2, 0).unwrap(),
        3,
        4,
    ]);
    simulator.step().unwrap();
    simulator.step().unwrap();

    let state = simulator.get_state();
    assert!(state.halted);
    assert!(state.priority_alarm);
    assert!(state.controller_selector_alarm);
    assert_eq!(state.pc, PROGRAM + 4);
    assert_eq!(simulator.controller_commands().len(), 1);

    simulator.clear_direct_io_alarms();
    assert!(simulator.get_state().halted);
    assert!(simulator.advance_controller_selector());
    simulator.clear_controller_selector_alarm();
    assert!(!simulator.get_state().halted);

    let mut offline = simulator_with_program(&[assemble_controller_select(7, 0).unwrap(), 1, 2]);
    offline.set_controller_online(7, false).unwrap();
    offline.step().unwrap();
    assert!(offline.get_state().controller_selector_alarm);
    assert!(offline.controller_commands().is_empty());
}

#[test]
fn controller_status_branches_use_device_supplied_condition_bits() {
    let mut asserted = simulator_with_program(&[
        assemble_controller_status(4, 0o23, true).unwrap(),
        assemble_fixed("NOP").unwrap(),
    ]);
    asserted.set_controller_condition(4, 0o23, true).unwrap();
    asserted.step().unwrap();
    assert_eq!(asserted.get_state().pc, PROGRAM + 1);

    let mut clear = simulator_with_program(&[
        assemble_controller_status(4, 0o23, false).unwrap(),
        assemble_fixed("NOP").unwrap(),
    ]);
    clear.set_controller_condition(4, 0o23, true).unwrap();
    clear.step().unwrap();
    assert_eq!(clear.get_state().pc, PROGRAM + 2);

    clear.set_controller_condition(4, READY, false).unwrap();
    assert!(!clear.get_state().controllers[4].ready);
}

#[test]
fn select_automatic_modification_can_choose_another_plug() {
    let mut simulator =
        simulator_with_program(&[assemble_controller_select(1, 1).unwrap(), 0o11, 0o22]);
    simulator.write_word(1, 0o100).unwrap();

    simulator.step().unwrap();

    assert_eq!(simulator.get_state().selected_controller, Some(2));
    assert_eq!(simulator.controller_commands()[0].plug, 2);
    assert_eq!(simulator.get_state().ir, 0o2520220);
}

#[test]
fn api_latches_a_disabled_transition_and_uses_group_32_vectoring() {
    let mut simulator = simulator_with_program(&[
        assemble_select_x_group(3).unwrap(),
        assemble_fixed("SET_PST").unwrap(),
        assemble_fixed("NOP").unwrap(),
    ]);
    simulator
        .load_words(
            &[
                assemble_fixed("SET_PST").unwrap(),
                encode_instruction(0o26, 1, 0).unwrap(),
            ],
            0o204,
        )
        .unwrap();
    simulator.step().unwrap();
    simulator.set_controller_api_enabled(2, true).unwrap();
    simulator.set_controller_ready(2, false).unwrap();
    simulator.set_controller_ready(2, true).unwrap();
    assert_eq!(simulator.highest_priority_pending_controller(), Some(2));

    simulator.step().unwrap();
    assert!(simulator.get_state().automatic_interrupt_mode);
    let interrupt_trace = simulator.step().unwrap();
    let interrupted = simulator.get_state();
    assert_eq!(interrupt_trace.address, 0o204);
    assert_eq!(simulator.read_word(0o201).unwrap(), PROGRAM + 2);
    assert_eq!(interrupted.selected_x_group, 32);
    assert!(interrupted.priority_mode);
    assert!(interrupted.priority_return_armed);
    assert!(interrupted.automatic_interrupt_mode);

    let return_trace = simulator.step().unwrap();
    let returned = simulator.get_state();
    assert_eq!(return_trace.address, 0o205);
    assert_eq!(returned.pc, PROGRAM + 2);
    assert_eq!(returned.selected_x_group, 3);
    assert!(!returned.priority_mode);
    assert!(!returned.priority_return_armed);
    assert!(returned.automatic_interrupt_mode);
    assert_eq!(returned.pending_controller_interrupts, 0);
}

#[test]
fn api_defers_new_transitions_until_priority_mode_returns() {
    let mut simulator = simulator_with_program(&[
        assemble_fixed("SET_PST").unwrap(),
        assemble_fixed("NOP").unwrap(),
    ]);
    simulator
        .load_words(
            &[
                assemble_fixed("SET_PST").unwrap(),
                encode_instruction(0o26, 1, 0).unwrap(),
            ],
            0o204,
        )
        .unwrap();
    simulator.set_controller_api_enabled(5, true).unwrap();
    simulator.set_controller_ready(5, false).unwrap();
    simulator.set_controller_ready(5, true).unwrap();
    simulator.step().unwrap();
    simulator.step().unwrap();

    simulator.set_controller_ready(1, false).unwrap();
    simulator.set_controller_api_enabled(1, true).unwrap();
    simulator.set_controller_ready(1, true).unwrap();
    assert_eq!(simulator.highest_priority_pending_controller(), Some(1));

    simulator.step().unwrap();
    assert!(!simulator.get_state().priority_mode);
    let branch_target = simulator.step().unwrap();
    assert_eq!(branch_target.address, PROGRAM + 1);
    let second_interrupt = simulator.step().unwrap();
    assert_eq!(second_interrupt.address, 0o204);
    assert!(simulator.get_state().priority_mode);
}

#[test]
fn priority_return_can_disable_further_interrupts_with_set_pbk() {
    let mut simulator = simulator_with_program(&[
        assemble_fixed("SET_PST").unwrap(),
        assemble_fixed("NOP").unwrap(),
    ]);
    simulator
        .load_words(
            &[
                assemble_fixed("SET_PST").unwrap(),
                assemble_fixed("SET_PBK").unwrap(),
                encode_instruction(0o26, 1, 0).unwrap(),
            ],
            0o204,
        )
        .unwrap();
    simulator.set_controller_api_enabled(0, true).unwrap();
    simulator.set_controller_ready(0, false).unwrap();
    simulator.set_controller_ready(0, true).unwrap();

    simulator.step().unwrap();
    simulator.step().unwrap();
    simulator.step().unwrap();
    assert!(simulator.get_state().priority_mode);
    assert!(simulator.get_state().priority_return_armed);
    assert!(!simulator.get_state().automatic_interrupt_mode);
    simulator.step().unwrap();

    let returned = simulator.get_state();
    assert_eq!(returned.pc, PROGRAM + 1);
    assert!(!returned.priority_mode);
    assert!(!returned.automatic_interrupt_mode);
}

#[test]
fn bru_target_access_is_not_interruptible() {
    let branch = encode_instruction(0o26, 0, (PROGRAM + 1) & 0x1fff).unwrap();
    let mut simulator = simulator_with_program(&[assemble_fixed("SET_PST").unwrap(), branch]);
    simulator.step().unwrap();
    simulator.step().unwrap();
    simulator.set_controller_api_enabled(0, true).unwrap();
    simulator.set_controller_ready(0, false).unwrap();
    simulator.set_controller_ready(0, true).unwrap();

    for _ in 0..3 {
        let trace = simulator.step().unwrap();
        assert_eq!(trace.address, PROGRAM + 1);
        assert!(!simulator.get_state().priority_mode);
    }
    assert_eq!(simulator.read_word(0o201).unwrap(), 0);
}

#[test]
fn card_reader_and_punch_ready_transitions_participate_in_api() {
    let mut reader = Simulator::new(4096).unwrap();
    reader.set_card_reader_api_enabled(true);
    reader.queue_card_reader_record(&[1, 2, 3]).unwrap();
    assert!(reader.get_state().card_reader_interrupt_pending);

    let mut punch = Simulator::new(4096).unwrap();
    punch.set_card_punch_online(false);
    punch.set_card_punch_api_enabled(true);
    punch.set_card_punch_online(true);
    assert!(punch.get_state().card_punch_interrupt_pending);

    let mut completed_punch = simulator_with_program(&[
        assemble_card_io("WCD", 0o400, 0).unwrap(),
        assemble_fixed("NOP").unwrap(),
    ]);
    completed_punch.set_card_punch_api_enabled(true);
    completed_punch.step().unwrap();
    assert!(completed_punch.get_state().card_punch_interrupt_pending);
}

#[test]
fn controller_command_capture_is_bounded_and_atomic() {
    let mut simulator = simulator_with_program(&[assemble_controller_select(0, 0).unwrap(), 1, 2]);
    for _ in 0..64 {
        simulator.set_program_counter(PROGRAM).unwrap();
        simulator.step().unwrap();
        assert!(simulator.advance_controller_selector());
        simulator.complete_controller(0, 0, false).unwrap();
    }
    assert_eq!(simulator.controller_commands().len(), 64);

    simulator.set_program_counter(PROGRAM).unwrap();
    let before = simulator.get_state();
    let error = simulator.step().unwrap_err();
    assert!(error.contains("command capture is full"));
    assert_eq!(simulator.get_state(), before);
    assert_eq!(simulator.controller_commands().len(), 64);
}

#[test]
fn reset_clears_selector_and_api_state() {
    let mut simulator = simulator_with_program(&[assemble_controller_select(3, 0).unwrap(), 1, 2]);
    simulator.set_controller_api_enabled(5, true).unwrap();
    simulator.set_controller_ready(5, false).unwrap();
    simulator.set_controller_ready(5, true).unwrap();
    simulator.set_controller_api_enabled(1, true).unwrap();
    simulator.set_controller_ready(1, false).unwrap();
    simulator.set_controller_ready(1, true).unwrap();
    simulator.set_card_reader_api_enabled(true);
    simulator.queue_card_reader_record(&[1]).unwrap();
    simulator.step().unwrap();
    assert_eq!(simulator.highest_priority_pending_controller(), Some(1));
    assert!(!simulator.controller_commands().is_empty());

    simulator.reset();

    let state = simulator.get_state();
    assert!(!state.controller_selector_busy);
    assert_eq!(state.selected_controller, None);
    assert_eq!(state.pending_controller_interrupts, 0);
    assert!(!state.card_reader_api_enabled);
    assert!(!state.card_reader_interrupt_pending);
    assert!(!state.priority_mode);
    assert!(state.controllers.iter().all(|controller| {
        controller.online
            && controller.ready
            && !controller.error
            && controller.error_conditions == 0
            && !controller.api_enabled
    }));
    assert!(simulator.controller_commands().is_empty());
}

#[test]
fn select_rejects_a_command_block_without_a_valid_continuation_atomically() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator
        .load_words(&[assemble_controller_select(0, 0).unwrap(), 1, 2], 4093)
        .unwrap();
    simulator.set_program_counter(4093).unwrap();
    let before = simulator.get_state();

    let error = simulator.step().unwrap_err();

    assert!(error.contains("address out of range"));
    assert_eq!(simulator.get_state(), before);
    assert!(simulator.controller_commands().is_empty());
}

#[test]
fn controller_status_skip_past_memory_fails_atomically() {
    let mut simulator = Simulator::new(4096).unwrap();
    simulator
        .write_word(4094, assemble_controller_status(0, 0o21, true).unwrap())
        .unwrap();
    simulator.set_program_counter(4094).unwrap();
    let before = simulator.get_state();

    let error = simulator.step().unwrap_err();

    assert!(error.contains("address out of range: 4096"));
    assert_eq!(simulator.get_state(), before);
}
