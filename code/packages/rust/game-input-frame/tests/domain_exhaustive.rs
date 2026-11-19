//! Finite exhaustive coverage of the native identifier and axis-value domains.
//!
//! These loops cover validation domains, not every possible frame sequence.

use game_input_frame::{
    reduce_frame, ActionChange, ActionFrame, AxisActionId, ButtonActionId, FrameError, InputState,
    PlayerId,
};

const SLOT_COUNT: usize = 16;

fn prior_state() -> InputState {
    InputState {
        player_id: PlayerId::new(u32::MAX),
        held_buttons: 0x8000_0000_0000_0005,
        axes: [17; SLOT_COUNT],
    }
}

#[test]
fn all_raw_button_ids_have_the_exact_validity_and_edge_behavior() {
    for raw_id in 0_u16..=255 {
        let previous = prior_state();
        let before = previous.clone();
        let changes = [ActionChange::Button {
            action_id: ButtonActionId::new(raw_id as u8),
            down: true,
        }];
        let before_changes = changes;
        let result = reduce_frame(&previous, u64::MAX, &changes);
        if raw_id < 64 {
            let bit = 1_u64 << raw_id;
            let was_down = before.held_buttons & bit != 0;
            let frame = result.expect("every button slot 0..=63 is valid");
            assert_eq!(frame.player_id.get(), u32::MAX);
            assert_eq!(frame.tick, u64::MAX);
            assert_eq!(frame.held_buttons, before.held_buttons | bit);
            assert_eq!(frame.pressed_buttons, if was_down { 0 } else { bit });
            assert_eq!(frame.released_buttons, 0);
            assert_eq!(frame.axes, before.axes);
        } else {
            assert_eq!(
                result,
                Err(FrameError::InvalidButtonActionId {
                    change_index: 0,
                    action_id: raw_id as u8,
                })
            );
        }
        assert_eq!(previous, before);
        assert_eq!(changes, before_changes);
    }
}

#[test]
fn all_raw_axis_ids_validate_before_the_axis_value() {
    for raw_id in 0_u16..=255 {
        let previous = prior_state();
        let before = previous.clone();
        // The invalid value also pins slot-before-value precedence for every
        // raw identifier; valid slots report the value error, invalid slots
        // report the identifier error.
        let changes = [ActionChange::Axis {
            action_id: AxisActionId::new(raw_id as u8),
            value: i16::MIN,
        }];
        let before_changes = changes;
        let result = reduce_frame(&previous, 23, &changes);
        if raw_id < SLOT_COUNT as u16 {
            assert_eq!(
                result,
                Err(FrameError::InvalidAxisValue {
                    change_index: 0,
                    axis_slot: raw_id as u8,
                    value: i16::MIN,
                })
            );
        } else {
            assert_eq!(
                result,
                Err(FrameError::InvalidAxisActionId {
                    change_index: 0,
                    action_id: raw_id as u8,
                })
            );
        }
        assert_eq!(previous, before);
        assert_eq!(changes, before_changes);
    }
}

#[test]
fn all_axis_values_are_checked_in_every_previous_state_slot() {
    for slot in 0..SLOT_COUNT {
        for raw in i32::from(i16::MIN)..=i32::from(i16::MAX) {
            let value = raw as i16;
            let valid = (-32767..=32767).contains(&raw);
            let mut previous = prior_state();
            previous.axes[slot] = value;
            let before = previous.clone();
            let result = reduce_frame(&previous, 0x0123_4567_89ab_cdef, &[]);
            if valid {
                let frame = result.expect("independent range accepts this prior level");
                assert_eq!(frame.player_id.get(), u32::MAX);
                assert_eq!(frame.tick, 0x0123_4567_89ab_cdef);
                assert_eq!(frame.held_buttons, before.held_buttons);
                assert_eq!(frame.pressed_buttons, 0);
                assert_eq!(frame.released_buttons, 0);
                assert_eq!(frame.axes, before.axes);
            } else {
                assert_eq!(
                    result,
                    Err(FrameError::InvalidPreviousAxisValue {
                        axis_slot: slot as u8,
                        value,
                    })
                );
            }
            assert_eq!(previous, before);
        }
    }
}

#[test]
fn all_axis_values_are_checked_in_every_axis_update_slot() {
    for slot in 0..SLOT_COUNT {
        for raw in i32::from(i16::MIN)..=i32::from(i16::MAX) {
            let value = raw as i16;
            let valid = (-32767..=32767).contains(&raw);
            let previous = prior_state();
            let before = previous.clone();
            let changes = [ActionChange::Axis {
                action_id: AxisActionId::new(slot as u8),
                value,
            }];
            let before_changes = changes;
            let result = reduce_frame(&previous, 0xfedc_ba98_7654_3210, &changes);
            if valid {
                let frame = result.expect("independent range accepts this update");
                let mut expected_axes = before.axes;
                expected_axes[slot] = value;
                assert_eq!(frame.player_id.get(), u32::MAX);
                assert_eq!(frame.tick, 0xfedc_ba98_7654_3210);
                assert_eq!(frame.held_buttons, before.held_buttons);
                assert_eq!(frame.pressed_buttons, 0);
                assert_eq!(frame.released_buttons, 0);
                assert_eq!(frame.axes, expected_axes);
            } else {
                assert_eq!(
                    result,
                    Err(FrameError::InvalidAxisValue {
                        change_index: 0,
                        axis_slot: slot as u8,
                        value,
                    })
                );
            }
            assert_eq!(previous, before);
            assert_eq!(changes, before_changes);
        }
    }
}

#[test]
fn invalid_identifiers_at_event_255_are_exact_and_atomic() {
    let previous = prior_state();
    let before = previous.clone();

    let mut button_changes = [ActionChange::Button {
        action_id: ButtonActionId::new(0),
        down: false,
    }; 256];
    button_changes[255] = ActionChange::Button {
        action_id: ButtonActionId::new(255),
        down: true,
    };
    let before_buttons = button_changes;
    assert_eq!(
        reduce_frame(&previous, 5, &button_changes),
        Err(FrameError::InvalidButtonActionId {
            change_index: 255,
            action_id: 255,
        })
    );
    assert_eq!(previous, before);
    assert_eq!(button_changes, before_buttons);

    let mut axis_changes = [ActionChange::Button {
        action_id: ButtonActionId::new(0),
        down: false,
    }; 256];
    axis_changes[255] = ActionChange::Axis {
        action_id: AxisActionId::new(255),
        value: i16::MIN,
    };
    let before_axes = axis_changes;
    assert_eq!(
        reduce_frame(&previous, 5, &axis_changes),
        Err(FrameError::InvalidAxisActionId {
            change_index: 255,
            action_id: 255,
        })
    );
    assert_eq!(previous, before);
    assert_eq!(axis_changes, before_axes);
}

#[test]
fn successful_exhaustive_cases_return_complete_frames() {
    let previous = InputState {
        player_id: PlayerId::new(9),
        held_buttons: 0,
        axes: [0; SLOT_COUNT],
    };
    let frame = reduce_frame(&previous, u64::MAX, &[]).unwrap();
    assert_eq!(
        frame,
        ActionFrame {
            player_id: PlayerId::new(9),
            tick: u64::MAX,
            held_buttons: 0,
            pressed_buttons: 0,
            released_buttons: 0,
            axes: [0; SLOT_COUNT],
        }
    );
}
