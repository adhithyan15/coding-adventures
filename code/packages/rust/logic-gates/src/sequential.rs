//! Sequential Logic — memory elements that give circuits the ability to remember.
//!
//! # From Combinational to Sequential
//!
//! The gates in [`crate::gates`] are "combinational" — their output depends ONLY on the
//! current inputs. They have no memory. If you remove the input, the output
//! disappears. This is like a light switch: the light is on only while the switch
//! is held in the ON position.
//!
//! Sequential logic is fundamentally different. Sequential circuits can REMEMBER
//! their previous state. Even after the input changes, the output can persist.
//! This is what makes computers possible — without memory, there are no variables,
//! no registers, no stored programs, no state machines.
//!
//! # The Key Insight: Feedback
//!
//! Memory arises from FEEDBACK — wiring a gate's output back into its own input.
//! When you cross-couple two NOR gates (each feeding its output into the other's
//! input), you create a stable loop that "latches" into one of two states and
//! stays there. This is the SR Latch, the simplest memory element.
//!
//! From this single idea, we build the entire memory hierarchy:
//!
//! ```text
//! SR Latch          -> raw 1-bit memory (2 cross-coupled NOR gates)
//! D Latch           -> controlled 1-bit memory (SR + enable signal)
//! D Flip-Flop       -> edge-triggered 1-bit memory (2 D latches)
//! Register          -> N-bit word storage (N flip-flops in parallel)
//! Shift Register    -> serial-to-parallel converter (chained flip-flops)
//! ```

use crate::gates::{and_gate, nor_gate, not_gate, xor_gate};

// ===========================================================================
// FlipFlopState — internal state for master-slave D flip-flops
// ===========================================================================

/// Internal state of a master-slave D flip-flop.
///
/// The D flip-flop uses two latches internally: a "master" and a "slave".
/// This struct captures the Q and Q_bar outputs of both latches so that
/// state can be preserved across clock cycles.
///
/// In real hardware, this state is held by the voltage levels on the
/// cross-coupled NOR gates inside each latch. In our software simulation,
/// we carry it explicitly between function calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FlipFlopState {
    pub master_q: u8,
    pub master_q_bar: u8,
    pub slave_q: u8,
    pub slave_q_bar: u8,
}

impl Default for FlipFlopState {
    /// Default state: both latches reset (Q=0, Q_bar=1).
    fn default() -> Self {
        Self {
            master_q: 0,
            master_q_bar: 1,
            slave_q: 0,
            slave_q_bar: 1,
        }
    }
}

// ===========================================================================
// SR LATCH — The Simplest Memory Element
// ===========================================================================
//
// The SR (Set-Reset) Latch is where memory begins. It is built from just
// two NOR gates, cross-coupled so that each gate's output feeds into the
// other gate's input. This feedback loop creates two stable states:
//
//     State "Set":   Q=1, Q_bar=0   (the latch remembers a 1)
//     State "Reset": Q=0, Q_bar=1   (the latch remembers a 0)
//
// Circuit diagram:
//
//     Reset --+         +-- Q
//             |  +----+ |
//             +--| NOR |-+
//             |  +----+ |
//             |    ^    |
//             |    |    |
//             |    v    |
//             |  +----+ |
//             +--| NOR |-+
//             |  +----+ |
//     Set   --+         +-- Q_bar

/// SR Latch — the fundamental 1-bit memory element.
///
/// Built from two NOR gates feeding back into each other. The feedback
/// creates two stable states that persist even after inputs are removed.
///
/// # Truth table
///
/// ```text
/// S  R  | Q    Q_bar  | Action
/// ------+-------------+----------------------------------
/// 0  0  | Q    Q_bar  | Hold -- remember previous state
/// 1  0  | 1    0      | Set -- store a 1
/// 0  1  | 0    1      | Reset -- store a 0
/// 1  1  | 0    0      | Invalid -- both outputs forced low
/// ```
///
/// # Why S=1, R=1 is "invalid"
///
/// Both NOR gates receive a 1 input, so both output 0. This means
/// Q = Q_bar = 0, which violates the invariant that Q and Q_bar
/// should be complements. We still compute it because that IS what
/// the gates produce, but the caller should avoid this combination.
///
/// # Example
///
/// ```
/// use logic_gates::sequential::sr_latch;
/// let (q, q_bar) = sr_latch(1, 0, 0, 1); // Set the latch
/// assert_eq!((q, q_bar), (1, 0));
/// let (q, q_bar) = sr_latch(0, 0, q, q_bar); // Hold
/// assert_eq!((q, q_bar), (1, 0));
/// ```
pub fn sr_latch(set: u8, reset: u8, mut q: u8, mut q_bar: u8) -> (u8, u8) {
    // --- Feedback simulation ---
    // We iterate because the two NOR gates depend on each other's outputs.
    // Each iteration computes both gates using the previous iteration's
    // outputs. We stop when the outputs stabilize (no change between
    // iterations). For an SR latch, this always converges within 2-3
    // iterations.
    let max_iterations = 10;
    for _ in 0..max_iterations {
        // Q_new     = NOR(Reset, Q_bar_current)
        // Q_bar_new = NOR(Set,   Q_current)
        let new_q = nor_gate(reset, q_bar);
        let new_q_bar = nor_gate(set, q);

        // Check for convergence
        if new_q == q && new_q_bar == q_bar {
            break;
        }

        q = new_q;
        q_bar = new_q_bar;
    }

    (q, q_bar)
}

// ===========================================================================
// D LATCH — Controlled Memory
// ===========================================================================
//
// The SR latch has a problem: the caller must carefully manage Set and Reset
// to avoid the invalid S=R=1 state. The D Latch solves this by deriving S
// and R from a single data input D, using a NOT gate to guarantee that S
// and R are always complementary.
//
// An "enable" signal controls WHEN the latch listens to the data input:
//   - Enable = 1: the latch is "transparent" -- output follows input
//   - Enable = 0: the latch is "opaque" -- output holds its last value
//
// Circuit:
//     S = AND(Data, Enable)
//     R = AND(NOT(Data), Enable)
//
// S and R can NEVER both be 1 at the same time. Problem solved!

/// D Latch — data latch with enable control.
///
/// When enable=1, the output transparently follows the data input.
/// When enable=0, the output holds its previous value regardless of data.
///
/// # Truth table
///
/// ```text
/// D  E  | Q    Q_bar  | Action
/// ------+-------------+----------------------------------
/// X  0  | Q    Q_bar  | Hold -- latch is opaque
/// 0  1  | 0    1      | Store 0 -- transparent
/// 1  1  | 1    0      | Store 1 -- transparent
/// ```
///
/// # Example
///
/// ```
/// use logic_gates::sequential::d_latch;
/// let (q, qb) = d_latch(1, 1, 0, 1); // Enable=1, store 1
/// assert_eq!((q, qb), (1, 0));
/// let (q, qb) = d_latch(0, 0, q, qb); // Enable=0, hold
/// assert_eq!((q, qb), (1, 0));
/// ```
pub fn d_latch(data: u8, enable: u8, q: u8, q_bar: u8) -> (u8, u8) {
    // Derive Set and Reset from Data and Enable
    let set = and_gate(data, enable);
    let reset = and_gate(not_gate(data), enable);
    sr_latch(set, reset, q, q_bar)
}

// ===========================================================================
// D FLIP-FLOP — Edge-Triggered Memory
// ===========================================================================
//
// The D Latch is transparent whenever Enable is high. In a synchronous
// circuit, this creates race conditions: data can ripple through multiple
// latches in a single clock cycle.
//
// The D Flip-Flop solves this with a MASTER-SLAVE configuration:
// two D latches connected in series, with opposite enable signals.
//
//   When Clock=0: Master is transparent (captures data), Slave holds
//   When Clock=1: Master holds, Slave is transparent (outputs master's value)
//
// The result: data is effectively captured at the RISING EDGE of the clock
// (the transition from 0 to 1).

/// D Flip-Flop — edge-triggered 1-bit memory using master-slave design.
///
/// To simulate a rising edge (0->1 transition), call twice:
///   1. First with clock=0 (master absorbs data)
///   2. Then with clock=1 (slave outputs what master captured)
///
/// # Example
///
/// ```
/// use logic_gates::sequential::{d_flip_flop, FlipFlopState};
/// let mut state = FlipFlopState::default();
/// // Clock low: master absorbs data=1
/// let (q, q_bar) = d_flip_flop(1, 0, &mut state);
/// // Clock high: slave outputs master's value
/// let (q, q_bar) = d_flip_flop(1, 1, &mut state);
/// assert_eq!(q, 1);
/// ```
pub fn d_flip_flop(data: u8, clock: u8, state: &mut FlipFlopState) -> (u8, u8) {
    // Master latch: enabled when clock is LOW (NOT clock)
    //   When clock=0, NOT(clock)=1, so master is transparent -- absorbs data
    //   When clock=1, NOT(clock)=0, so master holds its value
    let not_clock = not_gate(clock);
    let (mq, mqb) = d_latch(data, not_clock, state.master_q, state.master_q_bar);
    state.master_q = mq;
    state.master_q_bar = mqb;

    // Slave latch: enabled when clock is HIGH (clock directly)
    //   When clock=1, slave is transparent -- outputs master's stored value
    //   When clock=0, slave holds its value
    let (sq, sqb) = d_latch(mq, clock, state.slave_q, state.slave_q_bar);
    state.slave_q = sq;
    state.slave_q_bar = sqb;

    (sq, sqb)
}

// ===========================================================================
// REGISTER — N-Bit Word Storage
// ===========================================================================
//
// A register is simply N flip-flops arranged in parallel, one per bit.
// All flip-flops share the same clock signal, so they all capture their
// data at the same instant.
//
//     Bit 0:  Data[0] --| D-FF |-- Out[0]
//     Bit 1:  Data[1] --| D-FF |-- Out[1]
//     ...
//     Bit N:  Data[N] --| D-FF |-- Out[N]
//                          |
//     Clock ---------------+ (shared by all flip-flops)

/// N-bit register — stores a binary word on the clock signal.
///
/// Each bit position has its own D flip-flop. All flip-flops share
/// the same clock, so the entire word is captured simultaneously.
///
/// # Example
///
/// ```
/// use logic_gates::sequential::{register, FlipFlopState};
/// let mut state: Vec<FlipFlopState> = (0..4).map(|_| FlipFlopState::default()).collect();
/// // Clock low: flip-flops absorb data
/// let out = register(&[1, 0, 1, 1], 0, &mut state);
/// // Clock high: flip-flops output stored data
/// let out = register(&[1, 0, 1, 1], 1, &mut state);
/// assert_eq!(out, vec![1, 0, 1, 1]);
/// ```
pub fn register(data: &[u8], clock: u8, state: &mut [FlipFlopState]) -> Vec<u8> {
    assert_eq!(
        data.len(),
        state.len(),
        "data length {} does not match state length {}",
        data.len(),
        state.len()
    );
    assert!(!data.is_empty(), "data must not be empty");

    let mut output = Vec::with_capacity(data.len());
    for (i, &bit) in data.iter().enumerate() {
        let (q, _q_bar) = d_flip_flop(bit, clock, &mut state[i]);
        output.push(q);
    }
    output
}

// ===========================================================================
// SHIFT REGISTER — Serial-to-Parallel Conversion
// ===========================================================================
//
// A shift register is a chain of flip-flops where each one's output feeds
// into the next one's input. On each clock cycle, every bit shifts one
// position (left or right), and a new bit enters from the serial input.
//
// Right shift:
//     serial_in -> [FF_0] -> [FF_1] -> ... -> [FF_N-1] -> serial_out
//
// Left shift:
//     serial_out <- [FF_0] <- [FF_1] <- ... <- [FF_N-1] <- serial_in
//
// Why shift registers matter for floating-point arithmetic:
//   When adding two floating-point numbers, their mantissas must be aligned.
//   This alignment is done by shifting, which is built from shift registers.

/// Shift register — shifts bits through a chain of flip-flops.
///
/// On each clock cycle, bits shift one position and a new bit enters
/// from the serial input. The bit that falls off the end becomes the
/// serial output.
///
/// Returns `(parallel_out, serial_out)` where:
/// - `parallel_out`: current value of all bit positions
/// - `serial_out`: the bit that was shifted out
///
/// # Example
///
/// ```
/// use logic_gates::sequential::{shift_register, FlipFlopState};
/// let mut state: Vec<FlipFlopState> = (0..4).map(|_| FlipFlopState::default()).collect();
/// let (out, sout) = shift_register(1, 0, &mut state, "right");
/// let (out, sout) = shift_register(1, 1, &mut state, "right");
/// assert_eq!(out, vec![1, 0, 0, 0]);
/// ```
pub fn shift_register(
    serial_in: u8,
    clock: u8,
    state: &mut [FlipFlopState],
    direction: &str,
) -> (Vec<u8>, u8) {
    assert!(
        direction == "right" || direction == "left",
        "direction must be 'right' or 'left', got '{direction}'"
    );
    let width = state.len();
    assert!(width >= 1, "shift register width must be >= 1");

    // Read current parallel output before shifting (from slave_q of each FF)
    let current_values: Vec<u8> = state.iter().map(|s| s.slave_q).collect();

    // Determine data inputs based on shift direction
    let serial_out;
    let data_inputs: Vec<u8>;

    if direction == "right" {
        // serial_in -> FF[0] -> FF[1] -> ... -> FF[N-1] -> serial_out
        serial_out = current_values[width - 1];
        let mut inputs = vec![serial_in];
        inputs.extend_from_slice(&current_values[..width - 1]);
        data_inputs = inputs;
    } else {
        // serial_out <- FF[0] <- FF[1] <- ... <- FF[N-1] <- serial_in
        serial_out = current_values[0];
        let mut inputs = current_values[1..].to_vec();
        inputs.push(serial_in);
        data_inputs = inputs;
    }

    // Clock all flip-flops with their new data inputs
    let mut parallel_out = Vec::with_capacity(width);
    for i in 0..width {
        let (q, _q_bar) = d_flip_flop(data_inputs[i], clock, &mut state[i]);
        parallel_out.push(q);
    }

    (parallel_out, serial_out)
}

// ===========================================================================
// COUNTER — Binary Counting (helper, uses XOR and AND from gates)
// ===========================================================================
// A counter increments its stored value on each clock cycle. It combines
// storage (register) with arithmetic (chain of half-adders starting with
// carry_in=1).

/// Binary counter state, wrapping a register and its current value.
#[derive(Debug, Clone)]
pub struct CounterState {
    /// Current count as a list of bits (index 0 = LSB).
    pub value: Vec<u8>,
    /// Flip-flop states for the underlying register.
    pub ff_state: Vec<FlipFlopState>,
}

impl CounterState {
    /// Create a new counter state with the given bit width, initialized to zero.
    pub fn new(width: usize) -> Self {
        assert!(width >= 1, "counter width must be >= 1");
        Self {
            value: vec![0; width],
            ff_state: (0..width).map(|_| FlipFlopState::default()).collect(),
        }
    }
}

/// Binary counter — increments on each clock cycle.
///
/// Uses a chain of half-adders (XOR for sum, AND for carry) starting with
/// carry_in=1 to add 1 to the current value each cycle.
///
/// # Example
///
/// ```
/// use logic_gates::sequential::{counter, CounterState};
/// let mut state = CounterState::new(4);
/// let bits = counter(0, 0, &mut state); // Initialize
/// let bits = counter(0, 1, &mut state); // Tick 1
/// assert_eq!(bits, vec![1, 0, 0, 0]);  // decimal 1
/// ```
pub fn counter(clock: u8, reset: u8, state: &mut CounterState) -> Vec<u8> {
    let width = state.value.len();

    let next_value = if reset == 1 {
        vec![0; width]
    } else {
        // Increment: add 1 using a chain of half-adders
        let mut result = Vec::with_capacity(width);
        let mut carry: u8 = 1; // carry_in = 1 means "add 1"
        for i in 0..width {
            let bit = state.value[i];
            let sum_bit = xor_gate(bit, carry);
            carry = and_gate(bit, carry);
            result.push(sum_bit);
        }
        result
    };

    // Store the new value in the register
    let output = register(&next_value, clock, &mut state.ff_state);

    // Only update the stored value when the register captures it (clock=1)
    if clock == 1 {
        state.value = next_value;
    }

    output
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- SR Latch ---
    #[test]
    fn test_sr_latch_set() {
        let (q, qb) = sr_latch(1, 0, 0, 1);
        assert_eq!((q, qb), (1, 0));
    }

    #[test]
    fn test_sr_latch_reset() {
        let (q, qb) = sr_latch(0, 1, 1, 0);
        assert_eq!((q, qb), (0, 1));
    }

    #[test]
    fn test_sr_latch_hold() {
        // Set first, then hold
        let (q, qb) = sr_latch(1, 0, 0, 1);
        let (q, qb) = sr_latch(0, 0, q, qb);
        assert_eq!((q, qb), (1, 0));
    }

    #[test]
    fn test_sr_latch_invalid() {
        let (q, qb) = sr_latch(1, 1, 0, 1);
        assert_eq!((q, qb), (0, 0));
    }

    // --- D Latch ---
    #[test]
    fn test_d_latch_transparent() {
        let (q, qb) = d_latch(1, 1, 0, 1);
        assert_eq!((q, qb), (1, 0));
    }

    #[test]
    fn test_d_latch_hold() {
        let (q, qb) = d_latch(1, 1, 0, 1);
        let (q, qb) = d_latch(0, 0, q, qb);
        assert_eq!((q, qb), (1, 0)); // Holds the 1
    }

    // --- D Flip-Flop ---
    #[test]
    fn test_d_flip_flop_rising_edge() {
        let mut state = FlipFlopState::default();
        // Clock low: master absorbs data=1
        d_flip_flop(1, 0, &mut state);
        // Clock high: slave outputs
        let (q, _qb) = d_flip_flop(1, 1, &mut state);
        assert_eq!(q, 1);
    }

    // --- Register ---
    #[test]
    fn test_register_store_and_retrieve() {
        let mut state: Vec<FlipFlopState> =
            (0..4).map(|_| FlipFlopState::default()).collect();
        register(&[1, 0, 1, 1], 0, &mut state);
        let out = register(&[1, 0, 1, 1], 1, &mut state);
        assert_eq!(out, vec![1, 0, 1, 1]);
    }

    // --- Shift Register ---
    #[test]
    fn test_shift_register_right() {
        let mut state: Vec<FlipFlopState> =
            (0..4).map(|_| FlipFlopState::default()).collect();
        shift_register(1, 0, &mut state, "right");
        let (out, _sout) = shift_register(1, 1, &mut state, "right");
        assert_eq!(out, vec![1, 0, 0, 0]);
    }

    // --- Counter ---
    #[test]
    fn test_counter_counts_up() {
        let mut state = CounterState::new(4);
        // Tick 1: 0 -> 1
        counter(0, 0, &mut state);
        let bits = counter(0, 1, &mut state);
        assert_eq!(bits, vec![1, 0, 0, 0]); // decimal 1
    }
}
