//! Combinational Circuits — building blocks between primitive gates and full arithmetic.
//!
//! # What are combinational circuits?
//!
//! Combinational circuits produce outputs that depend ONLY on the current inputs —
//! no memory, no state, no clock. They are built entirely from the primitive gates
//! defined in [`crate::gates`] (AND, OR, NOT, XOR, etc.).
//!
//! These circuits fill the gap between individual gates and the ALU:
//!
//! ```text
//! Primitive gates (gates.rs)
//!     │
//! Combinational circuits (THIS MODULE)
//!     │  MUX, DEMUX, decoder, encoder, tri-state buffer
//!     │
//! Arithmetic circuits (arithmetic crate)
//!     │  half adder, full adder, ALU
//!     │
//! CPU, FPGA, memory controllers
//!     │  everything above uses these building blocks
//! ```
//!
//! # Why these circuits matter
//!
//! - **MUX (Multiplexer)**: The selector switch of digital logic. A K-input LUT in
//!   an FPGA is literally a 2^K-to-1 MUX with SRAM storing the truth table. CPUs use
//!   MUXes to select between register outputs, ALU inputs, and forwarded values.
//!
//! - **DEMUX (Demultiplexer)**: Routes one signal to one of many destinations.
//!   Used in memory write addressing and bus arbitration.
//!
//! - **Decoder**: Converts binary addresses into one-hot select lines. Every memory
//!   chip has a row decoder that activates exactly one word line based on the address.
//!
//! - **Encoder / Priority Encoder**: The inverse of a decoder. Priority encoders
//!   are the heart of interrupt controllers — when multiple interrupts fire
//!   simultaneously, the priority encoder picks the most important one.
//!
//! - **Tri-state buffer**: Enables shared buses by letting devices "disconnect"
//!   from the wire when they're not talking. Without tri-state buffers, you'd need
//!   separate wires for every device pair.

use crate::gates::{and_gate, not_gate, or_gate};

// ===========================================================================
// Helper — bit validation (reuses the same pattern as gates.rs)
// ===========================================================================

/// Panics (in debug mode) if the value is not 0 or 1.
#[inline]
fn validate_bit(value: u8, name: &str) {
    debug_assert!(
        value == 0 || value == 1,
        "{name} must be 0 or 1, got {value}"
    );
}

/// Validates every element in a slice is 0 or 1.
#[inline]
fn validate_bits(bits: &[u8], name: &str) {
    for (i, &b) in bits.iter().enumerate() {
        validate_bit(b, &format!("{name}[{i}]"));
    }
}

// ===========================================================================
// MULTIPLEXER (MUX) — The Selector Switch
// ===========================================================================
//
// A multiplexer takes N data inputs and a set of select lines, and routes
// exactly one input to the output. Think of it as a railroad switch that
// directs one of several trains onto a single track.
//
// The number of select lines determines how many inputs can be selected:
//   1 select line  -> 2 inputs  (2:1 MUX)
//   2 select lines -> 4 inputs  (4:1 MUX)
//   N select lines -> 2^N inputs (2^N:1 MUX)
//
// Every larger MUX can be built recursively from 2:1 MUXes.
// This recursive structure is exactly how FPGA look-up tables (LUTs) work:
// a 4-input LUT is a 16:1 MUX tree with the truth table stored in SRAM.

/// 2-to-1 Multiplexer — the simplest selector circuit.
///
/// Routes one of two data inputs to the output based on a select signal.
///
/// # Circuit
///
/// ```text
/// d0 --+
///      |---- output
/// d1 --+
///       ^
/// sel --+
/// ```
///
/// Built from gates:
///     `output = OR(AND(d0, NOT(sel)), AND(d1, sel))`
///
/// When sel=0, NOT(sel)=1 enables d0 through the top AND gate.
/// When sel=1, sel itself enables d1 through the bottom AND gate.
///
/// # Truth table
///
/// ```text
/// sel | output
/// ----+-------
///  0  |  d0
///  1  |  d1
/// ```
///
/// # Example
///
/// ```
/// use logic_gates::combinational::mux2;
/// assert_eq!(mux2(0, 1, 0), 0); // sel=0 -> d0
/// assert_eq!(mux2(0, 1, 1), 1); // sel=1 -> d1
/// ```
#[inline]
pub fn mux2(d0: u8, d1: u8, sel: u8) -> u8 {
    validate_bit(d0, "d0");
    validate_bit(d1, "d1");
    validate_bit(sel, "sel");

    // output = OR(AND(d0, NOT(sel)), AND(d1, sel))
    or_gate(and_gate(d0, not_gate(sel)), and_gate(d1, sel))
}

/// 4-to-1 Multiplexer — selects one of four inputs using 2 select lines.
///
/// Built from three 2:1 MUXes arranged in a tree:
///
/// ```text
/// d0 --+               sel[0] controls first level
///      MUX -- r0 --+
/// d1 --+            |   sel[1] controls second level
///                   MUX -- output
/// d2 --+            |
///      MUX -- r1 --+
/// d3 --+
/// ```
///
/// # Truth table
///
/// ```text
/// sel[1] sel[0] | output
/// ---------------+-------
///   0      0     |  d0
///   0      1     |  d1
///   1      0     |  d2
///   1      1     |  d3
/// ```
///
/// # Panics
///
/// Panics if `sel` does not have exactly 2 elements.
///
/// # Example
///
/// ```
/// use logic_gates::combinational::mux4;
/// assert_eq!(mux4(1, 0, 0, 0, &[0, 0]), 1); // sel=00 -> d0
/// assert_eq!(mux4(0, 0, 0, 1, &[1, 1]), 1); // sel=11 -> d3
/// ```
pub fn mux4(d0: u8, d1: u8, d2: u8, d3: u8, sel: &[u8]) -> u8 {
    validate_bit(d0, "d0");
    validate_bit(d1, "d1");
    validate_bit(d2, "d2");
    validate_bit(d3, "d3");
    assert!(sel.len() == 2, "sel must have exactly 2 elements, got {}", sel.len());
    validate_bits(sel, "sel");

    // First level: sel[0] selects within each pair
    let r0 = mux2(d0, d1, sel[0]);
    let r1 = mux2(d2, d3, sel[0]);

    // Second level: sel[1] selects between the two pairs
    mux2(r0, r1, sel[1])
}

/// N-to-1 Multiplexer — selects one of N inputs using log2(N) select lines.
///
/// N must be a power of 2 (2, 4, 8, 16, 32, 64, ...).
///
/// Built recursively: split inputs in half, recurse on each half with
/// `sel[..sel.len()-1]`, then use a 2:1 MUX with the last select bit
/// to pick between the two halves.
///
/// This recursive construction is exactly how FPGA look-up tables work:
/// a K-input LUT is a 2^K-to-1 MUX tree.
///
/// # Panics
///
/// Panics if:
/// - `inputs` has fewer than 2 elements
/// - `inputs` length is not a power of 2
/// - `sel` length does not equal log2(inputs.len())
///
/// # Example
///
/// ```
/// use logic_gates::combinational::mux_n;
/// // 16:1 MUX — select input 5 (binary 0101)
/// let mut data = vec![0u8; 16];
/// data[5] = 1;
/// assert_eq!(mux_n(&data, &[1, 0, 1, 0]), 1); // sel=0101 LSB-first -> index 5
/// ```
pub fn mux_n(inputs: &[u8], sel: &[u8]) -> u8 {
    let n = inputs.len();

    assert!(n >= 2, "inputs must have at least 2 elements, got {n}");

    // Check power of 2: a number is a power of 2 if it has exactly one bit set
    assert!(
        n & (n - 1) == 0,
        "inputs length must be a power of 2, got {n}"
    );

    let expected_sel_bits = (n as f64).log2() as usize;
    assert!(
        sel.len() == expected_sel_bits,
        "sel must have {expected_sel_bits} bits for {n} inputs, got {}",
        sel.len()
    );

    validate_bits(inputs, "inputs");
    validate_bits(sel, "sel");

    // Delegate to the inner recursive function (skips validation)
    mux_n_inner(inputs, sel)
}

/// Inner recursive helper for `mux_n` — skips validation (already done by caller).
fn mux_n_inner(inputs: &[u8], sel: &[u8]) -> u8 {
    let n = inputs.len();

    // Base case: 2:1 MUX
    if n == 2 {
        return mux2(inputs[0], inputs[1], sel[0]);
    }

    // Recursive case: split in half, recurse, combine with 2:1 MUX
    let half = n / 2;
    let lower = mux_n_inner(&inputs[..half], &sel[..sel.len() - 1]);
    let upper = mux_n_inner(&inputs[half..], &sel[..sel.len() - 1]);
    mux2(lower, upper, sel[sel.len() - 1])
}

// ===========================================================================
// DEMULTIPLEXER (DEMUX) — The Inverse of MUX
// ===========================================================================
//
// A demultiplexer takes one data input and routes it to one of N outputs.
// The select lines determine which output receives the data; all other
// outputs are 0.
//
// Think of it as an address decoder that also carries data: the decoder
// picks which output line is active, and the data signal determines
// whether that line is 0 or 1.

/// 1-to-N Demultiplexer — routes one input to one of N outputs.
///
/// The selected output receives the data value; all other outputs are 0.
///
/// Built from a decoder + AND gates:
/// 1. Decoder converts sel bits into one-hot (exactly one output = 1)
/// 2. AND each decoder output with the data input
///
/// # 1-to-4 DEMUX truth table
///
/// ```text
/// sel[1] sel[0]  data | y0  y1  y2  y3
/// ---------------------+-----------------
///   0      0      0   |  0   0   0   0
///   0      0      1   |  1   0   0   0
///   0      1      0   |  0   0   0   0
///   0      1      1   |  0   1   0   0
///   1      0      0   |  0   0   0   0
///   1      0      1   |  0   0   1   0
///   1      1      0   |  0   0   0   0
///   1      1      1   |  0   0   0   1
/// ```
///
/// # Panics
///
/// Panics if `n_outputs` is not a power of 2 >= 2, or if `sel` length
/// does not equal log2(n_outputs).
///
/// # Example
///
/// ```
/// use logic_gates::combinational::demux;
/// assert_eq!(demux(1, &[1, 0], 4), vec![0, 1, 0, 0]); // sel=01 -> output 1
/// ```
pub fn demux(data: u8, sel: &[u8], n_outputs: usize) -> Vec<u8> {
    validate_bit(data, "data");

    assert!(
        n_outputs >= 2 && (n_outputs & (n_outputs - 1)) == 0,
        "n_outputs must be a power of 2 >= 2, got {n_outputs}"
    );

    let expected_sel_bits = (n_outputs as f64).log2() as usize;
    assert!(
        sel.len() == expected_sel_bits,
        "sel must have {expected_sel_bits} bits for {n_outputs} outputs, got {}",
        sel.len()
    );

    validate_bits(sel, "sel");

    // Use decoder to get one-hot output, then AND each with data
    let decoded = decoder(sel);
    decoded.iter().map(|&d| and_gate(d, data)).collect()
}

// ===========================================================================
// DECODER — Binary to One-Hot
// ===========================================================================
//
// A decoder converts an N-bit binary input into a one-hot output:
// exactly one of 2^N output lines is 1, the rest are 0.
//
// It's essentially a DEMUX with data hardwired to 1.
//
// Decoders are fundamental to memory addressing: the row decoder in an SRAM
// chip takes the address bits and activates exactly one word line, enabling
// read/write access to that row of cells.
//
// Construction: each output Y_i is an AND of all N input bits (or their
// complements), corresponding to the binary representation of i.
//
// Example for 2-to-4:
//   Y0 = AND(NOT(A1), NOT(A0))  -- active when input = 00
//   Y1 = AND(NOT(A1), A0)       -- active when input = 01
//   Y2 = AND(A1, NOT(A0))       -- active when input = 10
//   Y3 = AND(A1, A0)            -- active when input = 11

/// N-to-2^N Decoder — converts binary input to one-hot output.
///
/// For an N-bit input, produces 2^N outputs where exactly one is 1.
/// The output at index i is 1 when the input represents the binary value i.
///
/// # 2-to-4 Decoder truth table
///
/// ```text
/// A1  A0  | Y0  Y1  Y2  Y3
/// --------+-----------------
///  0   0  |  1   0   0   0
///  0   1  |  0   1   0   0
///  1   0  |  0   0   1   0
///  1   1  |  0   0   0   1
/// ```
///
/// # Panics
///
/// Panics if `inputs` is empty.
///
/// # Example
///
/// ```
/// use logic_gates::combinational::decoder;
/// assert_eq!(decoder(&[1, 0]), vec![0, 1, 0, 0]); // input=01 -> output index 1
/// assert_eq!(decoder(&[0, 0, 0]), vec![1, 0, 0, 0, 0, 0, 0, 0]); // input=000 -> index 0
/// ```
pub fn decoder(inputs: &[u8]) -> Vec<u8> {
    assert!(!inputs.is_empty(), "inputs must be non-empty");
    validate_bits(inputs, "inputs");

    let n = inputs.len();
    let n_outputs = 1usize << n; // 2^n

    // Precompute complements once
    let complements: Vec<u8> = inputs.iter().map(|&b| not_gate(b)).collect();

    let mut outputs = Vec::with_capacity(n_outputs);
    for i in 0..n_outputs {
        // Output i is the AND of all input bits where the bit corresponding
        // to the binary representation of i is taken directly, and the rest
        // are complemented.
        //
        // For i=5 (binary 101) with 3 inputs [A0, A1, A2]:
        //   Y5 = AND(A0, NOT(A1), A2)
        //   because 5 in binary is: bit0=1, bit1=0, bit2=1
        let mut result: u8 = 1;
        for bit_pos in 0..n {
            if (i >> bit_pos) & 1 == 1 {
                // This bit position is 1 in i's binary representation
                result = and_gate(result, inputs[bit_pos]);
            } else {
                // This bit position is 0 — use the complement
                result = and_gate(result, complements[bit_pos]);
            }
        }
        outputs.push(result);
    }

    outputs
}

// ===========================================================================
// ENCODER — One-Hot to Binary
// ===========================================================================
//
// The inverse of a decoder: takes a one-hot input (exactly one bit is 1)
// and produces the binary index of that bit.
//
// If input bit 5 is active (out of 8 inputs), the encoder outputs [1,0,1]
// (the binary representation of 5, LSB first).

/// 2^N-to-N Encoder — converts one-hot input to binary output.
///
/// Exactly one input bit must be 1. The output is the binary
/// representation of the index of that active bit (LSB first).
///
/// # 4-to-2 Encoder truth table
///
/// ```text
/// I0  I1  I2  I3  | A0  A1
/// ----------------+--------
///  1   0   0   0  |  0   0
///  0   1   0   0  |  1   0
///  0   0   1   0  |  0   1
///  0   0   0   1  |  1   1
/// ```
///
/// # Panics
///
/// Panics if:
/// - `inputs` length is not a power of 2 >= 2
/// - Not exactly one bit is set (not one-hot)
///
/// # Example
///
/// ```
/// use logic_gates::combinational::encoder;
/// assert_eq!(encoder(&[0, 0, 1, 0]), vec![0, 1]); // I2 active -> binary 10 -> [0, 1]
/// ```
pub fn encoder(inputs: &[u8]) -> Vec<u8> {
    let n_inputs = inputs.len();

    assert!(
        n_inputs >= 2 && (n_inputs & (n_inputs - 1)) == 0,
        "inputs length must be a power of 2 >= 2, got {n_inputs}"
    );

    validate_bits(inputs, "inputs");

    // Validate one-hot: exactly one bit must be 1
    let active_count: u8 = inputs.iter().sum();
    assert!(
        active_count == 1,
        "inputs must be one-hot (exactly one bit = 1), got {active_count} active bits"
    );

    let n_output_bits = (n_inputs as f64).log2() as usize;

    // Find the active index
    let active_index = inputs.iter().position(|&b| b == 1).unwrap();

    // Convert to binary (LSB first)
    let mut output = Vec::with_capacity(n_output_bits);
    for bit_pos in 0..n_output_bits {
        output.push(((active_index >> bit_pos) & 1) as u8);
    }

    output
}

// ===========================================================================
// PRIORITY ENCODER — Multiple Inputs, Highest Wins
// ===========================================================================
//
// A regular encoder requires exactly one active input (one-hot). In real
// systems, multiple signals can be active simultaneously — for example,
// multiple interrupt lines firing at the same time.
//
// The priority encoder solves this: it outputs the binary index of the
// HIGHEST-PRIORITY active input. Priority is determined by index — the
// highest index has the highest priority.
//
// It also outputs a "valid" flag that indicates whether ANY input is active.
// This distinguishes "no input active" from "input 0 is active" (both would
// produce output 00 without the valid flag).

/// Priority encoder — encodes the highest-priority active input.
///
/// When multiple inputs are active, the one with the highest index wins.
/// A "valid" output indicates whether any input is active at all.
///
/// # 4-to-2 Priority Encoder truth table
///
/// ```text
/// I0  I1  I2  I3  | A0  A1  Valid
/// ----------------+---------------
///  0   0   0   0  |  0   0    0     No input active
///  1   0   0   0  |  0   0    1     I0 wins (only one)
///  X   1   0   0  |  1   0    1     I1 wins over I0
///  X   X   1   0  |  0   1    1     I2 wins over I0,I1
///  X   X   X   1  |  1   1    1     I3 always wins
/// ```
///
/// # Panics
///
/// Panics if `inputs` length is not a power of 2 >= 2.
///
/// # Returns
///
/// `(binary_output, valid)` where:
/// - `binary_output`: `Vec<u8>` of N bits (LSB first) — index of highest active input
/// - `valid`: 1 if any input is active, 0 if all inputs are 0
///
/// # Example
///
/// ```
/// use logic_gates::combinational::priority_encoder;
/// let (bits, valid) = priority_encoder(&[1, 0, 1, 0]); // I0 and I2 active, I2 wins
/// assert_eq!(bits, vec![0, 1]);
/// assert_eq!(valid, 1);
///
/// let (bits, valid) = priority_encoder(&[0, 0, 0, 0]); // No input active
/// assert_eq!(bits, vec![0, 0]);
/// assert_eq!(valid, 0);
/// ```
pub fn priority_encoder(inputs: &[u8]) -> (Vec<u8>, u8) {
    let n_inputs = inputs.len();

    assert!(
        n_inputs >= 2 && (n_inputs & (n_inputs - 1)) == 0,
        "inputs length must be a power of 2 >= 2, got {n_inputs}"
    );

    validate_bits(inputs, "inputs");

    let n_output_bits = (n_inputs as f64).log2() as usize;

    // Scan from highest index to lowest — first active input wins
    let mut highest_active: Option<usize> = None;
    for i in (0..n_inputs).rev() {
        if inputs[i] == 1 {
            highest_active = Some(i);
            break;
        }
    }

    // Valid flag: 1 if any input was active
    let valid: u8 = if highest_active.is_some() { 1 } else { 0 };

    // Convert active index to binary (LSB first)
    // If no input is active, output all zeros
    let index = highest_active.unwrap_or(0);
    let mut output = Vec::with_capacity(n_output_bits);
    for bit_pos in 0..n_output_bits {
        output.push(((index >> bit_pos) & 1) as u8);
    }

    (output, valid)
}

// ===========================================================================
// TRI-STATE BUFFER — Three Output States
// ===========================================================================
//
// Normal gates have two possible outputs: 0 or 1. A tri-state buffer adds
// a third state: HIGH-IMPEDANCE (Z), which means the output is electrically
// disconnected — as if the wire were cut.
//
// This is essential for shared buses. In a computer, the data bus connects
// the CPU, memory, and I/O devices on the same wires. Only one device can
// drive the bus at a time. Tri-state buffers let each device disconnect
// when it's not its turn, preventing electrical conflicts.
//
// In FPGAs, tri-state buffers appear in I/O blocks where pins can be
// configured as inputs (high-Z) or outputs (driven).
//
// We represent high-impedance as None in Rust (Option<u8>):
//   - enable=1: output = Some(data) (0 or 1)
//   - enable=0: output = None (high-Z, disconnected)

/// Tri-state buffer — output can be 0, 1, or high-impedance (None).
///
/// When enabled, the buffer passes the data input through to the output.
/// When disabled, the output is high-impedance (`None`) — electrically
/// disconnected from the wire.
///
/// # Truth table
///
/// ```text
/// data  enable | output
/// -------------+-------
///   0      0   |  None    (high-Z, disconnected)
///   1      0   |  None    (high-Z, disconnected)
///   0      1   |  Some(0) (driving low)
///   1      1   |  Some(1) (driving high)
/// ```
///
/// # Example
///
/// ```
/// use logic_gates::combinational::tri_state;
/// assert_eq!(tri_state(1, 1), Some(1)); // Enabled -> pass data
/// assert_eq!(tri_state(1, 0), None);    // Disabled -> high-Z
/// assert_eq!(tri_state(0, 1), Some(0)); // Enabled -> pass data
/// ```
#[inline]
pub fn tri_state(data: u8, enable: u8) -> Option<u8> {
    validate_bit(data, "data");
    validate_bit(enable, "enable");

    if enable == 0 {
        None
    } else {
        Some(data)
    }
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- MUX2 ---
    #[test]
    fn test_mux2_sel0_returns_d0() {
        assert_eq!(mux2(0, 1, 0), 0);
        assert_eq!(mux2(1, 0, 0), 1);
    }

    #[test]
    fn test_mux2_sel1_returns_d1() {
        assert_eq!(mux2(0, 1, 1), 1);
        assert_eq!(mux2(1, 0, 1), 0);
    }

    // --- MUX4 ---
    #[test]
    fn test_mux4_all_selections() {
        assert_eq!(mux4(1, 0, 0, 0, &[0, 0]), 1); // sel=00 -> d0
        assert_eq!(mux4(0, 1, 0, 0, &[1, 0]), 1); // sel=01 -> d1
        assert_eq!(mux4(0, 0, 1, 0, &[0, 1]), 1); // sel=10 -> d2
        assert_eq!(mux4(0, 0, 0, 1, &[1, 1]), 1); // sel=11 -> d3
    }

    // --- MUX_N ---
    #[test]
    fn test_mux_n_2_inputs() {
        assert_eq!(mux_n(&[1, 0], &[0]), 1);
        assert_eq!(mux_n(&[1, 0], &[1]), 0);
    }

    #[test]
    fn test_mux_n_4_inputs() {
        assert_eq!(mux_n(&[1, 0, 0, 0], &[0, 0]), 1);
        assert_eq!(mux_n(&[0, 0, 0, 1], &[1, 1]), 1);
    }

    #[test]
    fn test_mux_n_16_inputs() {
        let mut data = vec![0u8; 16];
        data[5] = 1;
        // 5 in binary LSB-first: 1, 0, 1, 0
        assert_eq!(mux_n(&data, &[1, 0, 1, 0]), 1);
    }

    // --- DEMUX ---
    #[test]
    fn test_demux_route_to_each_output() {
        assert_eq!(demux(1, &[0, 0], 4), vec![1, 0, 0, 0]);
        assert_eq!(demux(1, &[1, 0], 4), vec![0, 1, 0, 0]);
        assert_eq!(demux(1, &[0, 1], 4), vec![0, 0, 1, 0]);
        assert_eq!(demux(1, &[1, 1], 4), vec![0, 0, 0, 1]);
    }

    #[test]
    fn test_demux_data_zero() {
        assert_eq!(demux(0, &[1, 0], 4), vec![0, 0, 0, 0]);
    }

    // --- DECODER ---
    #[test]
    fn test_decoder_1bit() {
        assert_eq!(decoder(&[0]), vec![1, 0]);
        assert_eq!(decoder(&[1]), vec![0, 1]);
    }

    #[test]
    fn test_decoder_2bit() {
        assert_eq!(decoder(&[0, 0]), vec![1, 0, 0, 0]);
        assert_eq!(decoder(&[1, 0]), vec![0, 1, 0, 0]);
        assert_eq!(decoder(&[0, 1]), vec![0, 0, 1, 0]);
        assert_eq!(decoder(&[1, 1]), vec![0, 0, 0, 1]);
    }

    #[test]
    fn test_decoder_3bit() {
        assert_eq!(decoder(&[0, 0, 0]), vec![1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(decoder(&[1, 1, 1]), vec![0, 0, 0, 0, 0, 0, 0, 1]);
    }

    // --- ENCODER ---
    #[test]
    fn test_encoder_4_inputs() {
        assert_eq!(encoder(&[1, 0, 0, 0]), vec![0, 0]); // index 0
        assert_eq!(encoder(&[0, 1, 0, 0]), vec![1, 0]); // index 1
        assert_eq!(encoder(&[0, 0, 1, 0]), vec![0, 1]); // index 2
        assert_eq!(encoder(&[0, 0, 0, 1]), vec![1, 1]); // index 3
    }

    #[test]
    fn test_encoder_8_inputs() {
        assert_eq!(encoder(&[0, 0, 0, 0, 0, 1, 0, 0]), vec![1, 0, 1]); // index 5
    }

    // --- PRIORITY ENCODER ---
    #[test]
    fn test_priority_encoder_single_active() {
        let (bits, valid) = priority_encoder(&[0, 0, 1, 0]);
        assert_eq!(bits, vec![0, 1]);
        assert_eq!(valid, 1);
    }

    #[test]
    fn test_priority_encoder_multiple_active() {
        // I0 and I2 active — I2 (highest) wins
        let (bits, valid) = priority_encoder(&[1, 0, 1, 0]);
        assert_eq!(bits, vec![0, 1]); // index 2
        assert_eq!(valid, 1);
    }

    #[test]
    fn test_priority_encoder_all_active() {
        // All active — I3 (highest) wins
        let (bits, valid) = priority_encoder(&[1, 1, 1, 1]);
        assert_eq!(bits, vec![1, 1]); // index 3
        assert_eq!(valid, 1);
    }

    #[test]
    fn test_priority_encoder_none_active() {
        let (bits, valid) = priority_encoder(&[0, 0, 0, 0]);
        assert_eq!(bits, vec![0, 0]);
        assert_eq!(valid, 0);
    }

    // --- TRI-STATE BUFFER ---
    #[test]
    fn test_tri_state_enabled() {
        assert_eq!(tri_state(0, 1), Some(0));
        assert_eq!(tri_state(1, 1), Some(1));
    }

    #[test]
    fn test_tri_state_disabled() {
        assert_eq!(tri_state(0, 0), None);
        assert_eq!(tri_state(1, 0), None);
    }
}
