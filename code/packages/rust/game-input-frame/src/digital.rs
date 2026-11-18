//! Small, pure operations on the button-level bit field.
//!
//! A bit mask is a row of 64 yes/no lights. Slot zero is the rightmost light;
//! a shift moves the one-light marker to the slot we want to inspect. Keeping
//! these operations scalar and explicit makes their bit-level meaning easy to
//! inspect and prove independently of the record-based reducer.

/// Return whether a button slot is currently held.
#[inline]
pub fn bit_is_set(mask: u64, slot: u8) -> bool {
    (mask & (1_u64 << slot)) != 0
}

/// Turn on one button's light while preserving every other slot.
#[inline]
pub fn set_bit(mask: u64, slot: u8) -> u64 {
    mask | (1_u64 << slot)
}

/// Turn off one button's light while preserving every other slot.
#[inline]
pub fn clear_bit(mask: u64, slot: u8) -> u64 {
    mask & !(1_u64 << slot)
}
