use coding_adventures_ge225_simulator::{
    assemble_card_io, assemble_controller_select, assemble_controller_status, assemble_fixed,
    assemble_select_x_group, encode_instruction, Simulator as Functional,
};
use ge225_gatelevel::{ControllerCommand, Ge225GateLevel, MIN_MEMORY_WORDS};

const PROGRAM: i32 = 0o1000;
const READY: u8 = 0o20;

fn machine(words: &[i32]) -> Ge225GateLevel {
    let mut gate = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    gate.load_words(words, PROGRAM as usize).unwrap();
    gate.set_program_counter(PROGRAM).unwrap();
    gate
}

#[test]
fn selector_delivers_two_words_and_completion_updates_status() {
    let mut gate = machine(&[
        assemble_controller_select(2, 0).unwrap(),
        0o1234567,
        0o0765432,
        assemble_fixed("NOP").unwrap(),
    ]);
    gate.set_controller_error_condition(2, 0o25, true).unwrap();
    gate.step().unwrap();
    let state = gate.get_state();
    assert_eq!(state.pc, PROGRAM + 3);
    assert!(state.controller_selector_busy);
    assert_eq!(state.selected_controller, Some(2));
    assert!(!state.controllers[2].ready);
    assert!(!state.controllers[2].error);
    assert_eq!(state.controllers[2].error_conditions, 0);
    assert_eq!(
        gate.controller_commands(),
        &[ControllerCommand {
            plug: 2,
            select_word: 0o2500220,
            command_word: 0o1234567,
            address_word: 0o0765432,
        }]
    );
    assert!(gate.advance_controller_selector());
    assert!(!gate.advance_controller_selector());
    gate.complete_controller(2, 1_u64 << 0o23, false).unwrap();
    let state = gate.get_state();
    assert!(state.controllers[2].ready);
    assert_ne!(state.controllers[2].conditions & (1_u64 << READY), 0);
    assert_ne!(state.controllers[2].conditions & (1_u64 << 0o23), 0);
}

#[test]
fn busy_and_offline_selector_alarm_without_delivering() {
    let mut gate = machine(&[
        assemble_controller_select(1, 0).unwrap(),
        1,
        2,
        assemble_controller_select(2, 0).unwrap(),
        3,
        4,
    ]);
    gate.step().unwrap();
    gate.step().unwrap();
    let state = gate.get_state();
    assert!(state.halted && state.priority_alarm && state.controller_selector_alarm);
    assert_eq!(state.pc, PROGRAM + 4);
    assert_eq!(gate.controller_commands().len(), 1);
    gate.clear_direct_io_alarms();
    assert!(gate.get_state().halted);
    assert!(gate.advance_controller_selector());
    gate.clear_controller_selector_alarm();
    assert!(!gate.get_state().halted);

    let mut offline = machine(&[assemble_controller_select(7, 0).unwrap(), 1, 2]);
    offline.set_controller_online(7, false).unwrap();
    offline.step().unwrap();
    assert!(offline.get_state().controller_selector_alarm);
    assert!(offline.controller_commands().is_empty());
}

#[test]
fn controller_status_senses_device_condition_bits() {
    let mut set = machine(&[
        assemble_controller_status(4, 0o23, true).unwrap(),
        assemble_fixed("NOP").unwrap(),
    ]);
    set.set_controller_condition(4, 0o23, true).unwrap();
    set.step().unwrap();
    assert_eq!(set.get_state().pc, PROGRAM + 1);

    let mut clear = machine(&[
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
fn selector_automatic_modification_selects_another_plug() {
    let mut gate = machine(&[assemble_controller_select(1, 1).unwrap(), 0o11, 0o22]);
    gate.write_word(1, 0o100).unwrap();
    gate.step().unwrap();
    assert_eq!(gate.get_state().selected_controller, Some(2));
    assert_eq!(gate.controller_commands()[0].plug, 2);
    assert_eq!(gate.get_state().ir, 0o2520220);
}

fn install_return_handler(gate: &mut Ge225GateLevel, with_pbk: bool) {
    let mut handler = vec![assemble_fixed("SET_PST").unwrap()];
    if with_pbk {
        handler.push(assemble_fixed("SET_PBK").unwrap());
    }
    handler.push(encode_instruction(0o26, 1, 0).unwrap());
    gate.load_words(&handler, 0o204).unwrap();
}

#[test]
fn api_latches_transition_and_vectors_through_group_32() {
    let mut gate = machine(&[
        assemble_select_x_group(3).unwrap(),
        assemble_fixed("SET_PST").unwrap(),
        assemble_fixed("NOP").unwrap(),
    ]);
    install_return_handler(&mut gate, false);
    gate.step().unwrap();
    gate.set_controller_api_enabled(2, true).unwrap();
    gate.set_controller_ready(2, false).unwrap();
    gate.set_controller_ready(2, true).unwrap();
    assert_eq!(gate.highest_priority_pending_controller(), Some(2));
    gate.step().unwrap();
    let interrupt = gate.step().unwrap();
    let state = gate.get_state();
    assert_eq!(interrupt.pc_before, 0o204);
    assert_eq!(gate.read_word(0o201).unwrap(), PROGRAM + 2);
    assert_eq!(state.selected_x_group, 32);
    assert!(state.priority_mode && state.priority_return_armed);
    assert!(state.automatic_interrupt_mode);
    let returned = gate.step().unwrap();
    assert_eq!(returned.pc_before, 0o205);
    let state = gate.get_state();
    assert_eq!(state.pc, PROGRAM + 2);
    assert_eq!(state.selected_x_group, 3);
    assert!(!state.priority_mode && !state.priority_return_armed);
    assert_eq!(state.pending_controller_interrupts, 0);
}

#[test]
fn api_defers_new_transition_until_priority_return() {
    let mut gate = machine(&[
        assemble_fixed("SET_PST").unwrap(),
        assemble_fixed("NOP").unwrap(),
    ]);
    install_return_handler(&mut gate, false);
    gate.set_controller_api_enabled(5, true).unwrap();
    gate.set_controller_ready(5, false).unwrap();
    gate.set_controller_ready(5, true).unwrap();
    gate.step().unwrap();
    gate.step().unwrap();
    gate.set_controller_ready(1, false).unwrap();
    gate.set_controller_api_enabled(1, true).unwrap();
    gate.set_controller_ready(1, true).unwrap();
    assert_eq!(gate.highest_priority_pending_controller(), Some(1));
    gate.step().unwrap();
    assert!(!gate.get_state().priority_mode);
    assert_eq!(gate.step().unwrap().pc_before, PROGRAM + 1);
    assert_eq!(gate.step().unwrap().pc_before, 0o204);
    assert!(gate.get_state().priority_mode);
}

#[test]
fn priority_return_can_leave_interrupts_disabled() {
    let mut gate = machine(&[
        assemble_fixed("SET_PST").unwrap(),
        assemble_fixed("NOP").unwrap(),
    ]);
    install_return_handler(&mut gate, true);
    gate.set_controller_api_enabled(0, true).unwrap();
    gate.set_controller_ready(0, false).unwrap();
    gate.set_controller_ready(0, true).unwrap();
    gate.step().unwrap();
    gate.step().unwrap();
    gate.step().unwrap();
    let state = gate.get_state();
    assert!(state.priority_mode && state.priority_return_armed);
    assert!(!state.automatic_interrupt_mode);
    gate.step().unwrap();
    let state = gate.get_state();
    assert_eq!(state.pc, PROGRAM + 1);
    assert!(!state.priority_mode && !state.automatic_interrupt_mode);
}

#[test]
fn bru_target_access_is_never_interruptible() {
    let branch = encode_instruction(0o26, 0, (PROGRAM + 1) & 0x1fff).unwrap();
    let mut gate = machine(&[assemble_fixed("SET_PST").unwrap(), branch]);
    gate.step().unwrap();
    gate.step().unwrap();
    gate.set_controller_api_enabled(0, true).unwrap();
    gate.set_controller_ready(0, false).unwrap();
    gate.set_controller_ready(0, true).unwrap();
    for _ in 0..3 {
        assert_eq!(gate.step().unwrap().pc_before, PROGRAM + 1);
        assert!(!gate.get_state().priority_mode);
    }
    assert_eq!(gate.read_word(0o201).unwrap(), 0);
}

#[test]
fn card_ready_transitions_participate_in_api() {
    let mut reader = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    reader.set_card_reader_api_enabled(true);
    reader.queue_card_reader_record(&[1, 2, 3]).unwrap();
    assert!(reader.get_state().card_reader_interrupt_pending);

    let mut punch = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    punch.set_card_punch_online(false);
    punch.set_card_punch_api_enabled(true);
    punch.set_card_punch_online(true);
    assert!(punch.get_state().card_punch_interrupt_pending);

    let mut completed = machine(&[
        assemble_card_io("WCD", 0o400, 0).unwrap(),
        assemble_fixed("NOP").unwrap(),
    ]);
    completed.set_card_punch_api_enabled(true);
    completed.step().unwrap();
    assert!(completed.get_state().card_punch_interrupt_pending);
}

#[test]
fn command_capture_bound_and_memory_edges_are_atomic() {
    let mut gate = machine(&[assemble_controller_select(0, 0).unwrap(), 1, 2]);
    for _ in 0..64 {
        gate.set_program_counter(PROGRAM).unwrap();
        gate.step().unwrap();
        assert!(gate.advance_controller_selector());
        gate.complete_controller(0, 0, false).unwrap();
    }
    gate.set_program_counter(PROGRAM).unwrap();
    let before = gate.get_state();
    assert!(gate.step().is_err());
    assert_eq!(gate.get_state(), before);
    assert_eq!(gate.controller_commands().len(), 64);

    let mut edge = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    edge.load_words(&[assemble_controller_select(0, 0).unwrap(), 1, 2], 4093)
        .unwrap();
    edge.set_program_counter(4093).unwrap();
    let before = edge.get_state();
    assert!(edge.step().is_err());
    assert_eq!(edge.get_state(), before);
    assert!(edge.controller_commands().is_empty());
}

#[test]
fn reset_clears_all_selector_and_api_state() {
    let mut gate = machine(&[assemble_controller_select(3, 0).unwrap(), 1, 2]);
    gate.set_controller_api_enabled(5, true).unwrap();
    gate.set_controller_ready(5, false).unwrap();
    gate.set_controller_ready(5, true).unwrap();
    gate.set_card_reader_api_enabled(true);
    gate.queue_card_reader_record(&[1]).unwrap();
    gate.step().unwrap();
    gate.reset();
    let state = gate.get_state();
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
    assert!(gate.controller_commands().is_empty());
}

#[test]
fn controller_status_skip_past_memory_is_atomic() {
    let mut gate = Ge225GateLevel::new(MIN_MEMORY_WORDS).unwrap();
    gate.write_word(4094, assemble_controller_status(0, 0o21, true).unwrap())
        .unwrap();
    gate.set_program_counter(4094).unwrap();
    let before = gate.get_state();
    assert!(gate.step().is_err());
    assert_eq!(gate.get_state(), before);
}

#[test]
fn selector_and_status_sequence_matches_functional_oracle() {
    let program = [
        assemble_controller_select(2, 0).unwrap(),
        0o1234567,
        0o0765432,
        assemble_controller_status(2, 0o23, true).unwrap(),
        assemble_fixed("NOP").unwrap(),
    ];
    let mut gate = machine(&program);
    let mut functional = Functional::new(MIN_MEMORY_WORDS as i32).unwrap();
    functional.load_words(&program, PROGRAM).unwrap();
    functional.set_program_counter(PROGRAM).unwrap();

    gate.step().unwrap();
    functional.step().unwrap();
    assert_eq!(gate.get_state().pc, functional.get_state().pc);
    assert_eq!(
        gate.get_state().controller_selector_busy,
        functional.get_state().controller_selector_busy
    );
    assert_eq!(
        gate.controller_commands()[0].plug,
        functional.controller_commands()[0].plug
    );
    assert!(gate.advance_controller_selector());
    assert!(functional.advance_controller_selector());
    gate.complete_controller(2, 1_u64 << 0o23, false).unwrap();
    functional
        .complete_controller(2, 1_u64 << 0o23, false)
        .unwrap();
    gate.step().unwrap();
    functional.step().unwrap();
    let gate_state = gate.get_state();
    let functional_state = functional.get_state();
    assert_eq!(gate_state.pc, functional_state.pc);
    assert_eq!(gate_state.ir, functional_state.ir);
    assert_eq!(
        gate_state.controllers[2].ready,
        functional_state.controllers[2].ready
    );
    assert_eq!(
        gate_state.controllers[2].conditions,
        functional_state.controllers[2].conditions
    );
}
