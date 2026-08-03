//! Stable C ABI for the first neural-learning Rust execution core.
//!
//! The exported operation is intentionally the same two-input weighted neuron
//! learners can calculate on paper, but it accepts any non-empty bounded input
//! length. The C boundary uses only fixed-width integers, IEEE-754 doubles,
//! pointers, and status codes. No Rust-owned allocation crosses the boundary.

#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::c_char;
use std::mem::{align_of, size_of};
use std::panic::{self, AssertUnwindSafe};

pub const ABI_VERSION_V1: u32 = 0x0001_0000;
const MAX_INPUT_COUNT: u64 = 1 << 20;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok = 0,
    NullPointer = 1,
    EmptyInput = 2,
    BufferTooSmall = 3,
    ValueTooLarge = 4,
    NonFinite = 5,
    Panic = 6,
    OverlappingBuffer = 7,
    MisalignedPointer = 8,
}

#[no_mangle]
pub extern "C" fn neural_learning_abi_version() -> u32 {
    ABI_VERSION_V1
}

#[no_mangle]
pub extern "C" fn neural_learning_status_message_v1(status: u32) -> *const c_char {
    let message: &'static [u8] = match status {
        0 => b"ok\0",
        1 => b"null pointer\0",
        2 => b"input count must be positive\0",
        3 => b"contribution buffer is too small\0",
        4 => b"input count is too large\0",
        5 => b"all inputs and arithmetic results must be finite\0",
        6 => b"Rust panic was contained\0",
        7 => b"mutable output buffers must not overlap other buffers\0",
        8 => b"pointer is not aligned for a double\0",
        _ => b"unknown status\0",
    };
    message.as_ptr().cast()
}

fn aligned<T>(pointer: *const T) -> bool {
    (pointer as usize).is_multiple_of(align_of::<T>())
}

fn byte_range<T>(pointer: *const T, count: usize) -> Option<(usize, usize)> {
    let start = pointer as usize;
    let bytes = count.checked_mul(size_of::<T>())?;
    start.checked_add(bytes).map(|end| (start, end))
}

fn overlaps(left: (usize, usize), right: (usize, usize)) -> bool {
    left.0 < right.1 && right.0 < left.1
}

fn contain_panic(operation: impl FnOnce() -> Status) -> u32 {
    match panic::catch_unwind(AssertUnwindSafe(operation)) {
        Ok(status) => status as u32,
        Err(_) => Status::Panic as u32,
    }
}

unsafe fn weighted_sum(
    inputs: *const f64,
    weights: *const f64,
    input_count: u64,
    bias: f64,
    contributions_out: *mut f64,
    contributions_capacity: u64,
    prediction_out: *mut f64,
) -> Status {
    if inputs.is_null()
        || weights.is_null()
        || contributions_out.is_null()
        || prediction_out.is_null()
    {
        return Status::NullPointer;
    }
    if input_count == 0 {
        return Status::EmptyInput;
    }
    if input_count > MAX_INPUT_COUNT {
        return Status::ValueTooLarge;
    }
    if contributions_capacity < input_count {
        return Status::BufferTooSmall;
    }
    if !aligned(inputs)
        || !aligned(weights)
        || !aligned(contributions_out.cast_const())
        || !aligned(prediction_out.cast_const())
    {
        return Status::MisalignedPointer;
    }

    let count = match usize::try_from(input_count) {
        Ok(count) => count,
        Err(_) => return Status::ValueTooLarge,
    };
    let inputs_range = match byte_range(inputs, count) {
        Some(range) => range,
        None => return Status::ValueTooLarge,
    };
    let weights_range = match byte_range(weights, count) {
        Some(range) => range,
        None => return Status::ValueTooLarge,
    };
    let contributions_range = match byte_range(contributions_out.cast_const(), count) {
        Some(range) => range,
        None => return Status::ValueTooLarge,
    };
    let prediction_range = match byte_range(prediction_out.cast_const(), 1) {
        Some(range) => range,
        None => return Status::ValueTooLarge,
    };
    if overlaps(contributions_range, inputs_range)
        || overlaps(contributions_range, weights_range)
        || overlaps(prediction_range, inputs_range)
        || overlaps(prediction_range, weights_range)
        || overlaps(prediction_range, contributions_range)
    {
        return Status::OverlappingBuffer;
    }
    if !bias.is_finite() {
        return Status::NonFinite;
    }

    let mut prediction = bias;
    for index in 0..count {
        // SAFETY: the caller contract requires readable buffers for `count`
        // aligned doubles; range arithmetic above rules out address overflow.
        let input = unsafe { inputs.add(index).read() };
        // SAFETY: same contract and checks as the input read above.
        let weight = unsafe { weights.add(index).read() };
        let contribution = input * weight;
        prediction += contribution;
        if !input.is_finite()
            || !weight.is_finite()
            || !contribution.is_finite()
            || !prediction.is_finite()
        {
            return Status::NonFinite;
        }
    }

    for index in 0..count {
        // SAFETY: the first pass validated all reads and arithmetic. The caller
        // provides a writable, non-overlapping output buffer of this length.
        let contribution = unsafe { inputs.add(index).read() * weights.add(index).read() };
        unsafe { contributions_out.add(index).write(contribution) };
    }
    // SAFETY: `prediction_out` is a checked, non-overlapping one-double slot.
    unsafe { prediction_out.write(prediction) };
    Status::Ok
}

/// Compute an identity-activated weighted neuron and expose its paper trace.
///
/// # Safety
///
/// Each non-null pointer must address live, aligned storage for its declared
/// length. Inputs must be readable and outputs writable for the duration of
/// the call. Mutable outputs must not overlap inputs or one another.
#[no_mangle]
pub unsafe extern "C" fn neural_learning_weighted_sum_f64_v1(
    inputs: *const f64,
    weights: *const f64,
    input_count: u64,
    bias: f64,
    contributions_out: *mut f64,
    contributions_capacity: u64,
    prediction_out: *mut f64,
) -> u32 {
    contain_panic(|| unsafe {
        weighted_sum(
            inputs,
            weights,
            input_count,
            bias,
            contributions_out,
            contributions_capacity,
            prediction_out,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;
    use std::ptr;

    #[test]
    fn reports_version_and_static_status_messages() {
        assert_eq!(neural_learning_abi_version(), ABI_VERSION_V1);
        let message = unsafe { CStr::from_ptr(neural_learning_status_message_v1(0)) };
        assert_eq!(message.to_str().unwrap(), "ok");
    }

    #[test]
    fn contains_panics_without_touching_caller_outputs() {
        let contributions = [91.0, 92.0];
        let prediction = 93.0;
        let status = contain_panic(|| panic!("forced panic at the ABI boundary"));
        assert_eq!(status, Status::Panic as u32);
        assert_eq!(contributions, [91.0, 92.0]);
        assert_eq!(prediction, 93.0);
    }

    #[test]
    fn matches_the_hand_calculation() {
        let inputs = [2.0, -1.0];
        let weights = [0.5, -0.25];
        let mut contributions = [0.0; 2];
        let mut prediction = 0.0;
        let status = unsafe {
            neural_learning_weighted_sum_f64_v1(
                inputs.as_ptr(),
                weights.as_ptr(),
                2,
                0.1,
                contributions.as_mut_ptr(),
                2,
                &mut prediction,
            )
        };
        assert_eq!(status, Status::Ok as u32);
        assert_eq!(contributions, [1.0, 0.25]);
        assert_eq!(prediction, 1.35);
    }

    #[test]
    fn rejects_bad_arguments_without_writing_outputs() {
        let inputs = [2.0, -1.0];
        let weights = [0.5, -0.25];
        let mut contributions = [91.0, 92.0];
        let mut prediction = 93.0;
        let short = unsafe {
            neural_learning_weighted_sum_f64_v1(
                inputs.as_ptr(),
                weights.as_ptr(),
                2,
                0.1,
                contributions.as_mut_ptr(),
                1,
                &mut prediction,
            )
        };
        assert_eq!(short, Status::BufferTooSmall as u32);
        assert_eq!(contributions, [91.0, 92.0]);
        assert_eq!(prediction, 93.0);

        let null = unsafe {
            neural_learning_weighted_sum_f64_v1(
                ptr::null(),
                weights.as_ptr(),
                2,
                0.1,
                contributions.as_mut_ptr(),
                2,
                &mut prediction,
            )
        };
        assert_eq!(null, Status::NullPointer as u32);
    }

    #[test]
    fn rejects_non_finite_and_overlapping_buffers() {
        let inputs = [f64::INFINITY, -1.0];
        let weights = [0.5, -0.25];
        let mut contributions = [0.0; 2];
        let mut prediction = 0.0;
        let non_finite = unsafe {
            neural_learning_weighted_sum_f64_v1(
                inputs.as_ptr(),
                weights.as_ptr(),
                2,
                0.1,
                contributions.as_mut_ptr(),
                2,
                &mut prediction,
            )
        };
        assert_eq!(non_finite, Status::NonFinite as u32);

        let mut aliased = [2.0, -1.0];
        let overlap = unsafe {
            neural_learning_weighted_sum_f64_v1(
                aliased.as_ptr(),
                weights.as_ptr(),
                2,
                0.1,
                aliased.as_mut_ptr(),
                2,
                &mut prediction,
            )
        };
        assert_eq!(overlap, Status::OverlappingBuffer as u32);
    }
}
