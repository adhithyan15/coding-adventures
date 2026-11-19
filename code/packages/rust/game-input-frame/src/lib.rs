//! Deterministic per-player input levels for one caller-assigned simulation tick.
//!
//! The reducer begins after a host has normalized input. It owns no devices,
//! clock, player registry, or allocation: one fixed-size snapshot and at most
//! 256 ordered changes are enough to build a complete frame.

#![deny(unsafe_code)]

mod digital;

/// Number of stable button slots in the v1 profile.
pub const BUTTON_SLOT_COUNT: usize = 64;
/// Number of stable absolute-axis slots in the v1 profile.
pub const AXIS_SLOT_COUNT: usize = 16;
/// Largest ordered change batch accepted by the bounded reducer.
pub const MAX_CHANGES: usize = 256;

/// An opaque player key. The full `u32` range is valid and preserved verbatim.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PlayerId(u32);

impl PlayerId {
    /// Wrap any player key without allocating or reserving it.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Recover the exact key supplied by the caller.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// A raw button slot. Values outside `0..=63` remain representable so the
/// reducer can report them as ordered input errors instead of rejecting them
/// during construction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ButtonActionId(u8);

impl ButtonActionId {
    pub const fn new(value: u8) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// A raw axis slot. Values outside `0..=15` are retained for reducer errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AxisActionId(u8);

impl AxisActionId {
    pub const fn new(value: u8) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// The prior logical levels for one player. Axis slots are absolute levels;
/// every element must be in the symmetric range `-32767..=32767`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputState {
    pub player_id: PlayerId,
    pub held_buttons: u64,
    pub axes: [i16; AXIS_SLOT_COUNT],
}

/// One already-normalized logical action level update.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionChange {
    /// Set a stable button slot to the requested level.
    Button {
        action_id: ButtonActionId,
        down: bool,
    },
    /// Replace an axis slot with an absolute level, not a delta.
    Axis { action_id: AxisActionId, value: i16 },
}

/// The complete logical snapshot after one ordered input batch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionFrame {
    pub player_id: PlayerId,
    pub tick: u64,
    pub held_buttons: u64,
    pub pressed_buttons: u64,
    pub released_buttons: u64,
    pub axes: [i16; AXIS_SLOT_COUNT],
}

/// A refusal with portable, fixed-width details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrameError {
    /// The v1 bound is fixed, so no host-sized count is needed in the payload.
    TooManyChanges,
    InvalidPreviousAxisValue {
        axis_slot: u8,
        value: i16,
    },
    InvalidButtonActionId {
        change_index: u8,
        action_id: u8,
    },
    InvalidAxisActionId {
        change_index: u8,
        action_id: u8,
    },
    InvalidAxisValue {
        change_index: u8,
        axis_slot: u8,
        value: i16,
    },
}

/// Reduce one caller-ticked batch into a complete action frame.
///
/// Validation has observable precedence: batch length first, then prior axes
/// in ascending slot order, then each change in slice order (slot before value).
/// The fixed arrays and bounded input slice make the work bounded and require
/// no heap allocation. All mutation happens in local copies, so refusal cannot
/// expose a partial frame or alter either borrowed input.
pub fn reduce_frame(
    previous: &InputState,
    tick: u64,
    changes: &[ActionChange],
) -> Result<ActionFrame, FrameError> {
    if changes.len() > MAX_CHANGES {
        return Err(FrameError::TooManyChanges);
    }

    let mut axes = previous.axes;
    for (axis_slot, &value) in axes.iter().enumerate() {
        if !valid_axis_value(value) {
            return Err(FrameError::InvalidPreviousAxisValue {
                axis_slot: axis_slot as u8,
                value,
            });
        }
    }

    let mut held_buttons = previous.held_buttons;
    let mut pressed_buttons = 0_u64;
    let mut released_buttons = 0_u64;

    for (change_index, change) in changes.iter().copied().enumerate() {
        // At most 256 changes are admitted, so every zero-based index fits u8.
        let change_index = change_index as u8;
        match change {
            ActionChange::Button { action_id, down } => {
                let slot = action_id.get();
                if usize::from(slot) >= BUTTON_SLOT_COUNT {
                    return Err(FrameError::InvalidButtonActionId {
                        change_index,
                        action_id: slot,
                    });
                }
                let was_down = digital::bit_is_set(held_buttons, slot);
                if down && !was_down {
                    held_buttons = digital::set_bit(held_buttons, slot);
                    pressed_buttons = digital::set_bit(pressed_buttons, slot);
                } else if !down && was_down {
                    held_buttons = digital::clear_bit(held_buttons, slot);
                    released_buttons = digital::set_bit(released_buttons, slot);
                }
            }
            ActionChange::Axis { action_id, value } => {
                let slot = action_id.get();
                if usize::from(slot) >= AXIS_SLOT_COUNT {
                    return Err(FrameError::InvalidAxisActionId {
                        change_index,
                        action_id: slot,
                    });
                }
                if !valid_axis_value(value) {
                    return Err(FrameError::InvalidAxisValue {
                        change_index,
                        axis_slot: slot,
                        value,
                    });
                }
                axes[usize::from(slot)] = value;
            }
        }
    }

    Ok(ActionFrame {
        player_id: previous.player_id,
        tick,
        held_buttons,
        pressed_buttons,
        released_buttons,
        axes,
    })
}

#[inline]
const fn valid_axis_value(value: i16) -> bool {
    // `i16` already enforces the upper endpoint; only its asymmetric minimum
    // lies outside the v1 symmetric range.
    value != i16::MIN
}
