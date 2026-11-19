use game_input_frame::{
    reduce_frame, ActionChange, ActionFrame, AxisActionId, ButtonActionId, FrameError, InputState,
    PlayerId, MAX_CHANGES,
};

fn state(player: u32, held_buttons: u64) -> InputState {
    InputState {
        player_id: PlayerId::new(player),
        held_buttons,
        axes: [0; 16],
    }
}

fn button(slot: u8, down: bool) -> ActionChange {
    ActionChange::Button {
        action_id: ButtonActionId::new(slot),
        down,
    }
}

fn axis(slot: u8, value: i16) -> ActionChange {
    ActionChange::Axis {
        action_id: AxisActionId::new(slot),
        value,
    }
}

#[test]
fn full_width_ids_masks_ticks_and_all_button_slots_are_preserved() {
    let previous = state(u32::MAX, 0);
    let changes: Vec<_> = (0..64).map(|slot| button(slot, true)).collect();
    let frame = reduce_frame(&previous, u64::MAX, &changes).unwrap();
    assert_eq!(frame.player_id.get(), u32::MAX);
    assert_eq!(frame.tick, u64::MAX);
    assert_eq!(frame.held_buttons, u64::MAX);
    assert_eq!(frame.pressed_buttons, u64::MAX);
    assert_eq!(frame.released_buttons, 0);
}

#[test]
fn empty_batch_preserves_levels_and_has_no_edges() {
    let mut previous = state(23, (1 << 0) | (1 << 63));
    previous.axes[0] = -32767;
    previous.axes[15] = 32767;
    let frame = reduce_frame(&previous, 0, &[]).unwrap();
    assert_eq!(
        frame,
        ActionFrame {
            player_id: PlayerId::new(23),
            tick: 0,
            held_buttons: previous.held_buttons,
            pressed_buttons: 0,
            released_buttons: 0,
            axes: previous.axes,
        }
    );
}

#[test]
fn duplicates_are_noops_and_taps_record_both_transition_edges() {
    let previous = state(1, 0);
    let frame = reduce_frame(
        &previous,
        4,
        &[
            button(5, false),
            button(5, true),
            button(5, true),
            button(5, false),
            button(5, true),
        ],
    )
    .unwrap();
    assert_eq!(frame.held_buttons, 1 << 5);
    assert_eq!(frame.pressed_buttons, 1 << 5);
    assert_eq!(frame.released_buttons, 1 << 5);

    let previous_down = state(1, 1 << 5);
    let frame = reduce_frame(
        &previous_down,
        5,
        &[button(5, true), button(5, false), button(5, false)],
    )
    .unwrap();
    assert_eq!(frame.held_buttons, 0);
    assert_eq!(frame.pressed_buttons, 0);
    assert_eq!(frame.released_buttons, 1 << 5);
}

#[test]
fn axes_accept_symmetric_extrema_carry_forward_and_last_update_wins() {
    let mut previous = state(8, 0);
    previous.axes[3] = 12;
    let frame = reduce_frame(
        &previous,
        9,
        &[axis(0, -32767), axis(3, 32767), axis(3, -42)],
    )
    .unwrap();
    assert_eq!(frame.axes[0], -32767);
    assert_eq!(frame.axes[1], 0);
    assert_eq!(frame.axes[3], -42);
    assert_eq!(frame.axes[15], 0);
}

#[test]
fn count_boundaries_are_exact_and_257_precedes_invalid_state_and_change() {
    let noops = vec![button(0, false); MAX_CHANGES - 1];
    assert!(reduce_frame(&state(0, 0), 1, &noops).is_ok());
    let at_limit = vec![button(0, false); MAX_CHANGES];
    assert!(reduce_frame(&state(0, 0), 1, &at_limit).is_ok());

    let mut over_limit = vec![button(0, false); MAX_CHANGES + 1];
    over_limit[MAX_CHANGES] = button(64, true);
    let mut invalid = state(0, 0);
    invalid.axes[0] = i16::MIN;
    assert_eq!(
        reduce_frame(&invalid, 1, &over_limit),
        Err(FrameError::TooManyChanges)
    );
}

#[test]
fn last_admitted_index_and_raw_invalid_ids_are_reported_portably() {
    let mut changes = vec![button(0, false); MAX_CHANGES - 1];
    changes.push(button(255, true));
    assert_eq!(
        reduce_frame(&state(0, 0), 0, &changes),
        Err(FrameError::InvalidButtonActionId {
            change_index: 255,
            action_id: 255
        })
    );
    assert_eq!(
        reduce_frame(&state(0, 0), 0, &[axis(16, i16::MIN)]),
        Err(FrameError::InvalidAxisActionId {
            change_index: 0,
            action_id: 16
        })
    );
}

#[test]
fn validation_precedence_is_state_slot_then_change_order_slot_then_value() {
    let mut previous = state(0, 0);
    previous.axes[9] = i16::MIN;
    previous.axes[3] = i16::MIN;
    assert_eq!(
        reduce_frame(&previous, 0, &[button(64, true)]),
        Err(FrameError::InvalidPreviousAxisValue {
            axis_slot: 3,
            value: i16::MIN
        })
    );

    let previous = state(0, 0);
    assert_eq!(
        reduce_frame(&previous, 0, &[axis(16, i16::MIN)]),
        Err(FrameError::InvalidAxisActionId {
            change_index: 0,
            action_id: 16
        })
    );
    assert_eq!(
        reduce_frame(&previous, 0, &[axis(15, i16::MIN)]),
        Err(FrameError::InvalidAxisValue {
            change_index: 0,
            axis_slot: 15,
            value: i16::MIN
        })
    );
    assert_eq!(
        reduce_frame(
            &previous,
            0,
            &[button(0, true), axis(15, i16::MIN), button(64, true)]
        ),
        Err(FrameError::InvalidAxisValue {
            change_index: 1,
            axis_slot: 15,
            value: i16::MIN
        })
    );
}

#[test]
fn refused_and_successful_reductions_leave_borrowed_inputs_unchanged() {
    let previous = state(1, 1 << 63);
    let changes = [button(63, false), axis(0, i16::MIN)];
    let previous_before = previous.clone();
    let changes_before = changes;
    assert!(reduce_frame(&previous, 9, &changes).is_err());
    assert_eq!(previous, previous_before);
    assert_eq!(changes, changes_before);
    assert!(reduce_frame(&previous, 9, &changes[..1]).is_ok());
    assert_eq!(previous, previous_before);
    assert_eq!(changes, changes_before);
}
