/**
 * Combinational Circuits — building blocks between primitive gates and full arithmetic.
 *
 * === What are combinational circuits? ===
 *
 * Combinational circuits produce outputs that depend ONLY on the current inputs --
 * no memory, no state, no clock. They are built entirely from the primitive gates
 * defined in gates.ts (AND, OR, NOT, XOR, etc.).
 *
 * These circuits fill the gap between individual gates and the ALU:
 *
 *     Primitive gates (gates.ts)
 *         |
 *     Combinational circuits (THIS MODULE)
 *         |  MUX, DEMUX, decoder, encoder, tri-state buffer
 *         |
 *     Arithmetic circuits (arithmetic package)
 *         |  half adder, full adder, ALU
 *         |
 *     CPU, FPGA, memory controllers
 *         |  everything above uses these building blocks
 *
 * === Why these circuits matter ===
 *
 * - **MUX (Multiplexer)**: The selector switch of digital logic. A K-input LUT in
 *   an FPGA is literally a 2^K-to-1 MUX with SRAM storing the truth table. CPUs use
 *   MUXes to select between register outputs, ALU inputs, and forwarded values.
 *
 * - **DEMUX (Demultiplexer)**: Routes one signal to one of many destinations.
 *   Used in memory write addressing and bus arbitration.
 *
 * - **Decoder**: Converts binary addresses into one-hot select lines. Every memory
 *   chip has a row decoder that activates exactly one word line based on the address.
 *
 * - **Encoder / Priority Encoder**: The inverse of a decoder. Priority encoders
 *   are the heart of interrupt controllers -- when multiple interrupts fire
 *   simultaneously, the priority encoder picks the most important one.
 *
 * - **Tri-state buffer**: Enables shared buses by letting devices "disconnect"
 *   from the wire when they're not talking. Without tri-state buffers, you'd need
 *   separate wires for every device pair.
 */

import { AND, NOT, OR, type Bit, validateBit } from "./gates.js";

// ===========================================================================
// MULTIPLEXER (MUX) -- The Selector Switch
// ===========================================================================
//
// A multiplexer takes N data inputs and a set of select lines, and routes
// exactly one input to the output. Think of it as a railroad switch that
// directs one of several trains onto a single track.
//
// The number of select lines determines how many inputs can be selected:
//   1 select line  -> 2 inputs  (2:1 MUX)
//   2 select lines -> 4 inputs  (4:1 MUX)
//   3 select lines -> 8 inputs  (8:1 MUX)
//   N select lines -> 2^N inputs (2^N:1 MUX)
//
// Every larger MUX can be built recursively from 2:1 MUXes:
//   4:1  = two 2:1 MUXes feeding a third 2:1 MUX
//   8:1  = two 4:1 MUXes feeding a 2:1 MUX
//   16:1 = two 8:1 MUXes feeding a 2:1 MUX
//
// This recursive structure is exactly how FPGA look-up tables (LUTs) work:
// a 4-input LUT is a 16:1 MUX tree with the truth table stored in SRAM.

/**
 * 2-to-1 Multiplexer -- the simplest selector circuit.
 *
 * Routes one of two data inputs to the output based on a select signal.
 *
 * Circuit:
 *     d0 --+
 *          |---- output
 *     d1 --+
 *           ^
 *     sel --+
 *
 * Built from gates:
 *     output = OR(AND(d0, NOT(sel)), AND(d1, sel))
 *
 * When sel=0, the NOT(sel)=1 enables d0 through the top AND gate.
 * When sel=1, sel itself enables d1 through the bottom AND gate.
 *
 * Truth table:
 *     sel  | output
 *     -----+-------
 *      0   |  d0
 *      1   |  d1
 *
 * @example
 * mux2(0, 1, 0)  // sel=0, select d0=0 -> 0
 * mux2(0, 1, 1)  // sel=1, select d1=1 -> 1
 */
export function mux2(d0: Bit, d1: Bit, sel: Bit): Bit {
  validateBit(d0, "d0");
  validateBit(d1, "d1");
  validateBit(sel, "sel");

  // output = OR(AND(d0, NOT(sel)), AND(d1, sel))
  //
  // When sel=0: NOT(sel)=1, so AND(d0, 1)=d0; AND(d1, 0)=0 -> OR(d0, 0) = d0
  // When sel=1: NOT(sel)=0, so AND(d0, 0)=0; AND(d1, 1)=d1 -> OR(0, d1) = d1
  return OR(AND(d0, NOT(sel)), AND(d1, sel));
}

/**
 * 4-to-1 Multiplexer -- selects one of four inputs using 2 select lines.
 *
 * Built from three 2:1 MUXes arranged in a tree:
 *
 *     d0 --+                     sel[0] controls first level
 *          MUX -- r0 --+
 *     d1 --+            |        sel[1] controls second level
 *                        MUX -- output
 *     d2 --+            |
 *          MUX -- r1 --+
 *     d3 --+
 *
 * Truth table:
 *     sel[1] sel[0] | output
 *     ---------------+-------
 *       0      0     |  d0
 *       0      1     |  d1
 *       1      0     |  d2
 *       1      1     |  d3
 *
 * @example
 * mux4(1, 0, 0, 0, [0, 0])  // sel=00, select d0=1 -> 1
 * mux4(0, 0, 0, 1, [1, 1])  // sel=11, select d3=1 -> 1
 */
export function mux4(d0: Bit, d1: Bit, d2: Bit, d3: Bit, sel: Bit[]): Bit {
  validateBit(d0, "d0");
  validateBit(d1, "d1");
  validateBit(d2, "d2");
  validateBit(d3, "d3");

  if (!Array.isArray(sel) || sel.length !== 2) {
    throw new RangeError("sel must be an array of exactly 2 bits");
  }

  for (let i = 0; i < sel.length; i++) {
    validateBit(sel[i], `sel[${i}]`);
  }

  // First level: sel[0] selects within each pair
  const r0 = mux2(d0, d1, sel[0]);
  const r1 = mux2(d2, d3, sel[0]);

  // Second level: sel[1] selects between the two pairs
  return mux2(r0, r1, sel[1]);
}

/**
 * N-to-1 Multiplexer -- selects one of N inputs using log2(N) select lines.
 *
 * N must be a power of 2 (2, 4, 8, 16, 32, 64, ...).
 *
 * Built recursively: split inputs in half, recurse on each half with
 * sel[:-1], then use a 2:1 MUX with sel[-1] to pick between the two halves.
 *
 * This recursive construction is exactly how FPGA look-up tables work:
 * a K-input LUT is a 2^K-to-1 MUX tree.
 *
 * @param inputs - List of N data inputs (N must be power of 2, N >= 2)
 * @param sel - List of log2(N) select bits (LSB first)
 * @returns The selected data input value (0 or 1)
 *
 * @example
 * // 16:1 MUX -- select input 5 (binary 0101)
 * const data = Array(16).fill(0);
 * data[5] = 1;
 * muxN(data, [1, 0, 1, 0])  // sel=0101 LSB-first -> index 5 -> 1
 */
export function muxN(inputs: Bit[], sel: Bit[]): Bit {
  const n = inputs.length;

  if (n < 2) {
    throw new RangeError("inputs must have at least 2 elements");
  }

  // Check power of 2: a number is a power of 2 if it has exactly one bit set
  if ((n & (n - 1)) !== 0) {
    throw new RangeError(`inputs length must be a power of 2, got ${n}`);
  }

  const expectedSelBits = Math.log2(n);
  if (!Array.isArray(sel) || sel.length !== expectedSelBits) {
    throw new RangeError(
      `sel must be an array of ${expectedSelBits} bits for ${n} inputs, got ${Array.isArray(sel) ? sel.length : typeof sel}`,
    );
  }

  for (let i = 0; i < inputs.length; i++) {
    validateBit(inputs[i], `inputs[${i}]`);
  }

  for (let i = 0; i < sel.length; i++) {
    validateBit(sel[i], `sel[${i}]`);
  }

  // Base case: 2:1 MUX
  if (n === 2) {
    return mux2(inputs[0], inputs[1], sel[0]);
  }

  // Recursive case: split in half, recurse, combine with 2:1 MUX
  const half = n / 2;
  const lower = muxNInner(inputs.slice(0, half), sel.slice(0, -1));
  const upper = muxNInner(inputs.slice(half), sel.slice(0, -1));
  return mux2(lower, upper, sel[sel.length - 1]);
}

/**
 * Inner recursive helper for muxN -- skips validation (already done).
 */
function muxNInner(inputs: Bit[], sel: Bit[]): Bit {
  const n = inputs.length;
  if (n === 2) {
    return mux2(inputs[0], inputs[1], sel[0]);
  }

  const half = n / 2;
  const lower = muxNInner(inputs.slice(0, half), sel.slice(0, -1));
  const upper = muxNInner(inputs.slice(half), sel.slice(0, -1));
  return mux2(lower, upper, sel[sel.length - 1]);
}

// ===========================================================================
// DEMULTIPLEXER (DEMUX) -- The Inverse of MUX
// ===========================================================================
//
// A demultiplexer takes one data input and routes it to one of N outputs.
// The select lines determine which output receives the data; all other
// outputs are 0.
//
// Think of it as an address decoder that also carries data: the decoder
// picks which output line is active, and the data signal determines
// whether that line is 0 or 1.

/**
 * 1-to-N Demultiplexer -- routes one input to one of N outputs.
 *
 * The selected output receives the data value; all other outputs are 0.
 *
 * Built from a decoder + AND gates:
 *     1. Decoder converts sel bits into one-hot (exactly one output = 1)
 *     2. AND each decoder output with the data input
 *
 * 1-to-4 DEMUX truth table:
 *     sel[1] sel[0]  data | y0  y1  y2  y3
 *     ---------------------+------------------
 *       0      0      0   |  0   0   0   0
 *       0      0      1   |  1   0   0   0
 *       0      1      0   |  0   0   0   0
 *       0      1      1   |  0   1   0   0
 *       1      0      0   |  0   0   0   0
 *       1      0      1   |  0   0   1   0
 *       1      1      0   |  0   0   0   0
 *       1      1      1   |  0   0   0   1
 *
 * @param data - The data bit to route (0 or 1)
 * @param sel - List of select bits (LSB first), length = log2(nOutputs)
 * @param nOutputs - Number of outputs (must be power of 2, >= 2)
 * @returns List of nOutputs bits. Exactly one equals data, rest are 0.
 *
 * @example
 * demuxN(1, [1, 0], 4)  // sel=01, route data=1 to output 1 -> [0, 1, 0, 0]
 */
export function demuxN(data: Bit, sel: Bit[], nOutputs: number): Bit[] {
  validateBit(data, "data");

  if (nOutputs < 2 || (nOutputs & (nOutputs - 1)) !== 0) {
    throw new RangeError(
      `nOutputs must be a power of 2 >= 2, got ${nOutputs}`,
    );
  }

  const expectedSelBits = Math.log2(nOutputs);
  if (!Array.isArray(sel) || sel.length !== expectedSelBits) {
    throw new RangeError(
      `sel must be an array of ${expectedSelBits} bits for ${nOutputs} outputs`,
    );
  }

  for (let i = 0; i < sel.length; i++) {
    validateBit(sel[i], `sel[${i}]`);
  }

  // Use decoder to get one-hot output, then AND each with data
  const decoded = decoder(sel);
  return decoded.map((d) => AND(d, data));
}

// ===========================================================================
// DECODER -- Binary to One-Hot
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

/**
 * N-to-2^N Decoder -- converts binary input to one-hot output.
 *
 * For an N-bit input, produces 2^N outputs where exactly one is 1.
 * The output at index i is 1 when the input represents the binary
 * value i.
 *
 * 2-to-4 Decoder truth table:
 *     A1  A0  | Y0  Y1  Y2  Y3
 *     --------+------------------
 *      0   0  |  1   0   0   0
 *      0   1  |  0   1   0   0
 *      1   0  |  0   0   1   0
 *      1   1  |  0   0   0   1
 *
 * Built from AND and NOT gates:
 *     Y0 = AND(NOT(A1), NOT(A0))
 *     Y1 = AND(NOT(A1), A0)
 *     Y2 = AND(A1, NOT(A0))
 *     Y3 = AND(A1, A0)
 *
 * @param inputs - List of N input bits (LSB first). N >= 1.
 * @returns List of 2^N bits, exactly one of which is 1 (one-hot encoding).
 *
 * @example
 * decoder([1, 0])  // input = 01 (binary) -> output index 1 -> [0, 1, 0, 0]
 * decoder([0, 0, 0])  // input = 000 -> output index 0 -> [1, 0, 0, 0, 0, 0, 0, 0]
 */
export function decoder(inputs: Bit[]): Bit[] {
  if (!Array.isArray(inputs) || inputs.length < 1) {
    throw new RangeError("inputs must be a non-empty array of bits");
  }

  for (let i = 0; i < inputs.length; i++) {
    validateBit(inputs[i], `inputs[${i}]`);
  }

  const n = inputs.length;
  const nOutputs = 1 << n; // 2^n

  // Precompute complements once
  const complements: Bit[] = inputs.map((b) => NOT(b));

  const outputs: Bit[] = [];
  for (let i = 0; i < nOutputs; i++) {
    // Output i is the AND of all input bits where the bit corresponding
    // to the binary representation of i is taken directly, and the rest
    // are complemented.
    //
    // For i=5 (binary 101) with 3 inputs [A0, A1, A2]:
    //   Y5 = AND(A0, NOT(A1), A2)
    //   because 5 in binary is: bit0=1, bit1=0, bit2=1
    let result: Bit = 1;
    for (let bitPos = 0; bitPos < n; bitPos++) {
      if ((i >> bitPos) & 1) {
        // This bit position is 1 in i's binary representation
        result = AND(result, inputs[bitPos]);
      } else {
        // This bit position is 0 -- use the complement
        result = AND(result, complements[bitPos]);
      }
    }
    outputs.push(result);
  }

  return outputs;
}

// ===========================================================================
// ENCODER -- One-Hot to Binary
// ===========================================================================
//
// The inverse of a decoder: takes a one-hot input (exactly one bit is 1)
// and produces the binary index of that bit.
//
// If input bit 5 is active (out of 8 inputs), the encoder outputs 101
// (the binary representation of 5).

/**
 * 2^N-to-N Encoder -- converts one-hot input to binary output.
 *
 * Exactly one input bit must be 1. The output is the binary
 * representation of the index of that active bit.
 *
 * 4-to-2 Encoder truth table:
 *     I0  I1  I2  I3  | A1  A0
 *     ----------------+--------
 *      1   0   0   0  |  0   0
 *      0   1   0   0  |  0   1
 *      0   0   1   0  |  1   0
 *      0   0   0   1  |  1   1
 *
 * @param inputs - List of 2^N bits in one-hot encoding (exactly one must be 1).
 *                 Length must be a power of 2, >= 2.
 * @returns List of N bits representing the binary index of the active input (LSB first).
 *
 * @example
 * encoder([0, 0, 1, 0])  // I2 active -> binary 10 -> [0, 1]
 */
export function encoder(inputs: Bit[]): Bit[] {
  const nInputs = inputs.length;

  if (nInputs < 2 || (nInputs & (nInputs - 1)) !== 0) {
    throw new RangeError(
      `inputs length must be a power of 2 >= 2, got ${nInputs}`,
    );
  }

  for (let i = 0; i < inputs.length; i++) {
    validateBit(inputs[i], `inputs[${i}]`);
  }

  // Validate one-hot: exactly one bit must be 1
  const activeCount = inputs.reduce((sum: number, b) => sum + b, 0 as number);
  if (activeCount !== 1) {
    throw new RangeError(
      `inputs must be one-hot (exactly one bit = 1), got ${activeCount} active bits`,
    );
  }

  const nOutputBits = Math.log2(nInputs);

  // Find the active index
  const activeIndex = inputs.indexOf(1 as Bit);

  // Convert to binary (LSB first)
  const output: Bit[] = [];
  for (let bitPos = 0; bitPos < nOutputBits; bitPos++) {
    output.push(((activeIndex >> bitPos) & 1) as Bit);
  }

  return output;
}

// ===========================================================================
// PRIORITY ENCODER -- Multiple Inputs, Highest Wins
// ===========================================================================
//
// A regular encoder requires exactly one active input (one-hot). In real
// systems, multiple signals can be active simultaneously -- for example,
// multiple interrupt lines firing at the same time.
//
// The priority encoder solves this: it outputs the binary index of the
// HIGHEST-PRIORITY active input. Priority is determined by index -- the
// highest index has the highest priority.
//
// It also outputs a "valid" flag that indicates whether ANY input is active.
// This distinguishes "no input active" from "input 0 is active" (both would
// produce output 00 without the valid flag).

/**
 * Priority encoder -- encodes the highest-priority active input.
 *
 * When multiple inputs are active, the one with the highest index wins.
 * A "valid" output indicates whether any input is active at all.
 *
 * 4-to-2 Priority Encoder truth table:
 *     I0  I1  I2  I3  | A1  A0  Valid
 *     ----------------+-------------
 *      0   0   0   0  |  0   0    0     No input active
 *      1   0   0   0  |  0   0    1     I0 wins (only one)
 *      X   1   0   0  |  0   1    1     I1 wins over I0
 *      X   X   1   0  |  1   0    1     I2 wins over I0,I1
 *      X   X   X   1  |  1   1    1     I3 always wins
 *
 * @param inputs - List of 2^N input bits. Length must be a power of 2, >= 2.
 * @returns [binaryOutput, valid] where:
 *   - binaryOutput: List of N bits (LSB first) -- index of highest active input
 *   - valid: 1 if any input is active, 0 if all inputs are 0
 *
 * @example
 * priorityEncoder([1, 0, 1, 0])  // I0 and I2 active, I2 wins -> [[0, 1], 1]
 * priorityEncoder([0, 0, 0, 0])  // No input active -> [[0, 0], 0]
 */
export function priorityEncoder(inputs: Bit[]): [Bit[], Bit] {
  const nInputs = inputs.length;

  if (nInputs < 2 || (nInputs & (nInputs - 1)) !== 0) {
    throw new RangeError(
      `inputs length must be a power of 2 >= 2, got ${nInputs}`,
    );
  }

  for (let i = 0; i < inputs.length; i++) {
    validateBit(inputs[i], `inputs[${i}]`);
  }

  const nOutputBits = Math.log2(nInputs);

  // Scan from highest index to lowest -- first active input wins
  let highestActive = -1;
  for (let i = nInputs - 1; i >= 0; i--) {
    if (inputs[i] === 1) {
      highestActive = i;
      break;
    }
  }

  // Valid flag: 1 if any input was active
  const valid: Bit = highestActive === -1 ? 0 : 1;

  // Convert active index to binary (LSB first)
  // If no input is active, output all zeros
  const index = Math.max(highestActive, 0);
  const output: Bit[] = [];
  for (let bitPos = 0; bitPos < nOutputBits; bitPos++) {
    output.push(((index >> bitPos) & 1) as Bit);
  }

  return [output, valid];
}

// ===========================================================================
// TRI-STATE BUFFER -- Three Output States
// ===========================================================================
//
// Normal gates have two possible outputs: 0 or 1. A tri-state buffer adds
// a third state: HIGH-IMPEDANCE (Z), which means the output is electrically
// disconnected -- as if the wire were cut.
//
// This is essential for shared buses. In a computer, the data bus connects
// the CPU, memory, and I/O devices on the same wires. Only one device can
// drive the bus at a time. Tri-state buffers let each device disconnect
// when it's not its turn, preventing electrical conflicts.
//
// In FPGAs, tri-state buffers appear in I/O blocks where pins can be
// configured as inputs (high-Z) or outputs (driven).
//
// We represent high-impedance as null in TypeScript:
//   - enable=1: output = data (0 or 1)
//   - enable=0: output = null (high-Z, disconnected)

/**
 * Tri-state buffer -- output can be 0, 1, or high-impedance (null).
 *
 * When enabled, the buffer passes the data input through to the output.
 * When disabled, the output is high-impedance (null) -- electrically
 * disconnected from the wire.
 *
 * Truth table:
 *     data  enable | output
 *     -------------+-------
 *       0      0   |  null    (high-Z, disconnected)
 *       1      0   |  null    (high-Z, disconnected)
 *       0      1   |   0      (driving low)
 *       1      1   |   1      (driving high)
 *
 * @param data - The data bit to pass through (0 or 1)
 * @param enable - When 1, buffer is active. When 0, output is high-Z (null).
 * @returns data value (Bit) when enabled, null when disabled (high-impedance).
 *
 * @example
 * triState(1, 1)  // Enabled -> pass data through -> 1
 * triState(1, 0)  // Disabled -> high-impedance -> null
 * triState(0, 1)  // Enabled -> pass data through -> 0
 */
export function triState(data: Bit, enable: Bit): Bit | null {
  validateBit(data, "data");
  validateBit(enable, "enable");

  if (enable === 0) {
    return null;
  }

  return data;
}
