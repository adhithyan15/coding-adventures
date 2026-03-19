/**
 * Sequential Logic — memory elements that give circuits the ability to remember.
 *
 * === From Combinational to Sequential ===
 *
 * The gates in gates.ts are "combinational" — their output depends ONLY on the
 * current inputs. They have no memory. If you remove the input, the output
 * disappears. This is like a light switch: the light is on only while the switch
 * is held in the ON position.
 *
 * Sequential logic is fundamentally different. Sequential circuits can REMEMBER
 * their previous state. Even after the input changes, the output can persist.
 * This is what makes computers possible — without memory, there are no variables,
 * no registers, no stored programs, no state machines.
 *
 * === The Key Insight: Feedback ===
 *
 * Memory arises from FEEDBACK — wiring a gate's output back into its own input.
 * When you cross-couple two NOR gates (each feeding its output into the other's
 * input), you create a stable loop that "latches" into one of two states and
 * stays there. This is the SR Latch, the simplest memory element.
 *
 * From this single idea, we build the entire memory hierarchy:
 *
 *     SR Latch          -> raw 1-bit memory (2 cross-coupled NOR gates)
 *     D Latch           -> controlled 1-bit memory (SR + enable signal)
 *     D Flip-Flop       -> edge-triggered 1-bit memory (2 D latches)
 *     Register          -> N-bit word storage (N flip-flops in parallel)
 *     Shift Register    -> serial-to-parallel converter (chained flip-flops)
 *     Counter           -> binary counting (register + incrementer)
 *
 * === Why This Matters for GPUs ===
 *
 * GPUs are built on massive parallelism, and every parallel unit needs its own
 * local storage:
 *
 * - Registers hold intermediate computation values in shader cores
 * - Shift registers align mantissas during floating-point addition
 * - Counters track pipeline stages and warp scheduling
 * - A modern GPU has millions of flip-flops organized into register files
 *
 * This module builds each component from the gates defined in gates.ts,
 * showing exactly how physical memory works at the transistor level.
 */

import { AND, NOR, NOT, XOR, type Bit, validateBit } from "./gates.js";

// ===========================================================================
// Types for Sequential Logic State
// ===========================================================================

/**
 * Internal state of a single D flip-flop (master-slave configuration).
 *
 * A master-slave flip-flop is actually TWO latches chained together:
 *
 *          +----------+     +----------+
 *   Data ->| Master   |---->| Slave    |---> Q
 *          | Latch    |     | Latch    |---> Q_bar
 *   CLK' ->| Enable   |  CLK->| Enable  |
 *          +----------+     +----------+
 *
 * The master captures data when clock is LOW (NOT clock).
 * The slave captures the master's output when clock is HIGH.
 * This two-phase operation prevents data from "racing through"
 * both latches in a single clock cycle.
 */
export interface FlipFlopState {
  masterQ: Bit;
  masterQBar: Bit;
  slaveQ: Bit;
  slaveQBar: Bit;
}

/**
 * Internal state of a binary counter.
 *
 * A counter is a register that increments its own value on each
 * clock pulse. It wraps around when all bits are 1 (like an
 * odometer rolling over from 999 to 000).
 *
 *   Width=4 counter sequence:
 *   0000 -> 0001 -> 0010 -> 0011 -> 0100 -> ... -> 1111 -> 0000
 */
export interface CounterState {
  value: Bit[];
  ffState: FlipFlopState[];
}

// ===========================================================================
// Helper: create a fresh FlipFlopState
// ===========================================================================

function freshFlipFlopState(): FlipFlopState {
  return { masterQ: 0, masterQBar: 1, slaveQ: 0, slaveQBar: 1 };
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
// Once the latch enters one of these states, it STAYS there even after
// the input that caused it is removed. This is memory.
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
//
// The cross-coupling (each NOR feeds into the other) is what creates the
// feedback loop. In software, we simulate this by iterating until the
// outputs stabilize.

/**
 * SR Latch — the fundamental 1-bit memory element.
 *
 * Built from two NOR gates feeding back into each other. The feedback
 * creates two stable states that persist even after inputs are removed.
 *
 * Truth table:
 *     S  R  | Q    Q_bar  | Action
 *     ------+-------------+----------------------------------
 *     0  0  | Q    Q_bar  | Hold — remember previous state
 *     1  0  | 1    0      | Set — store a 1
 *     0  1  | 0    1      | Reset — store a 0
 *     1  1  | 0    0      | Invalid — both outputs forced low
 *
 * Why S=1, R=1 is "invalid":
 *     Both NOR gates receive a 1 input, so both output 0. This means
 *     Q = Q_bar = 0, which violates the invariant that Q and Q_bar
 *     should be complements. In real hardware, releasing both inputs
 *     simultaneously leads to a race condition — the circuit may
 *     oscillate or settle unpredictably. We still compute it (returning
 *     [0, 0]) because that IS what the gates produce, but the caller
 *     should avoid this combination.
 *
 * How the feedback simulation works:
 *     In real hardware, the two NOR gates evaluate continuously and
 *     simultaneously. In software, we simulate this by computing both
 *     gates in a loop until the outputs stop changing (convergence).
 *     For an SR latch, this always converges in at most 2 iterations.
 *
 * @example
 * srLatch(1, 0)           // Set the latch -> [1, 0]
 * srLatch(0, 0, 1, 0)     // Hold — remembers the 1 -> [1, 0]
 * srLatch(0, 1, 1, 0)     // Reset — back to 0 -> [0, 1]
 */
export function srLatch(
  set: Bit,
  reset: Bit,
  q: Bit = 0,
  qBar: Bit = 1,
): [Bit, Bit] {
  validateBit(set, "set");
  validateBit(reset, "reset");
  validateBit(q, "q");
  validateBit(qBar, "qBar");

  // --- Feedback simulation ---
  // We iterate because the two NOR gates depend on each other's outputs.
  // Each iteration computes both gates using the previous iteration's
  // outputs. We stop when the outputs stabilize (no change between
  // iterations). For an SR latch, this always converges within 2-3
  // iterations because there are no oscillating states (except the
  // invalid S=R=1 case, which converges to (0,0) in 1-2 steps).

  const maxIterations = 10; // Safety limit; real convergence happens in 2-3
  for (let i = 0; i < maxIterations; i++) {
    // Compute new outputs from current state
    //   Q_new     = NOR(Reset, Q_bar_current)
    //   Q_bar_new = NOR(Set,   Q_current)
    const newQ = NOR(reset, qBar);
    const newQBar = NOR(set, q);

    // Check for convergence — have the outputs stabilized?
    if (newQ === q && newQBar === qBar) {
      break;
    }

    // Update state for next iteration
    q = newQ;
    qBar = newQBar;
  }

  return [q, qBar];
}

// ===========================================================================
// D LATCH — Controlled Memory
// ===========================================================================
//
// The SR latch has a problem: the caller must carefully manage Set and Reset
// to avoid the invalid S=R=1 state. The D Latch solves this by deriving S
// and R from a single data input D, using a NOT gate to guarantee that S
// and R are always complementary (never both 1 at the same time).
//
// An "enable" signal controls WHEN the latch listens to the data input:
//   - Enable = 1: the latch is "transparent" — output follows input
//   - Enable = 0: the latch is "opaque" — output holds its last value
//
// Circuit diagram:
//
//                     +----------+
//     Data --+--------| AND      |-- Set --+
//            |        |          |         |    +----------+
//            |   +----+          |         +----| SR Latch |-- Q
//            |   |    +----------+         |    |          |
//     Enable-+---+                         |    |          |-- Q_bar
//            |   |    +----------+         |    +----------+
//            |   +----| AND      |-- Reset-+
//            |        |          |
//            +-->o----+          |
//              NOT    +----------+
//
//     S = AND(Data, Enable)
//     R = AND(NOT(Data), Enable)
//
// Notice: if Data=0 then S=0, R=Enable. If Data=1 then S=Enable, R=0.
// S and R can NEVER both be 1 at the same time. Problem solved!

/**
 * D Latch — data latch with enable control.
 *
 * When enable=1, the output transparently follows the data input.
 * When enable=0, the output holds its previous value regardless of data.
 *
 * This is the workhorse of level-sensitive storage. The "D" stands for
 * "Data" — there is only one data input, eliminating the invalid state
 * problem of the SR latch.
 *
 * Truth table:
 *     D  E  | Q    Q_bar  | Action
 *     ------+-------------+----------------------------------
 *     X  0  | Q    Q_bar  | Hold — latch is opaque
 *     0  1  | 0    1      | Store 0 — transparent
 *     1  1  | 1    0      | Store 1 — transparent
 *
 * (X means "don't care" — the value doesn't matter)
 *
 * Why not just use the D latch everywhere?
 *     The D latch is "level-sensitive" — it passes data through the entire
 *     time Enable is high. This causes problems in pipelines where you
 *     want data captured at a precise INSTANT, not during a whole interval.
 *     That's why we need the D Flip-Flop (see below).
 *
 * @example
 * dLatch(1, 1)           // Enable=1, store the 1 -> [1, 0]
 * dLatch(0, 0, 1, 0)     // Enable=0, hold the 1 -> [1, 0]
 * dLatch(0, 1, 1, 0)     // Enable=1, now store the 0 -> [0, 1]
 */
export function dLatch(
  data: Bit,
  enable: Bit,
  q: Bit = 0,
  qBar: Bit = 1,
): [Bit, Bit] {
  validateBit(data, "data");
  validateBit(enable, "enable");
  validateBit(q, "q");
  validateBit(qBar, "qBar");

  // Derive Set and Reset from Data and Enable
  //   S = AND(Data, Enable)       — set when data=1 and enabled
  //   R = AND(NOT(Data), Enable)  — reset when data=0 and enabled
  const set = AND(data, enable);
  const reset = AND(NOT(data), enable);

  // Feed into the SR latch with current state
  return srLatch(set, reset, q, qBar);
}

// ===========================================================================
// D FLIP-FLOP — Edge-Triggered Memory
// ===========================================================================
//
// The D Latch is transparent whenever Enable is high. In a synchronous
// circuit (where everything runs off a shared clock), this transparency
// creates race conditions: data can ripple through multiple latches in
// a single clock cycle if the clock stays high long enough.
//
// The D Flip-Flop solves this with a MASTER-SLAVE configuration:
// two D latches connected in series, with opposite enable signals.
//
//     +-----------------------------------------------------+
//     |                                                     |
//     |  Clock=0: Master transparent    Clock=1: Slave transparent
//     |  (absorbs new data)             (outputs stored data)
//     |                                                     |
//     |         +------------+          +------------+      |
//     |  Data --| D Latch    |----------| D Latch    |-- Q  |
//     |         | (Master)   |          | (Slave)    |      |
//     |  CLK' --| Enable     |   CLK ---| Enable     |      |
//     |         +------------+          +------------+      |
//     |                                                     |
//     |  CLK' = NOT(CLK)                                    |
//     +-----------------------------------------------------+
//
// How it works:
//   1. When Clock=0: Master is transparent (captures data), Slave holds
//   2. When Clock=1: Master holds, Slave is transparent (outputs master's value)
//
// The result: data is effectively captured at the RISING EDGE of the clock
// (the transition from 0 to 1). During the entire high period, new data
// cannot pass through because the master is holding. This is "edge-triggered"
// behavior.
//
// Why edge-triggering matters:
//   In a GPU, thousands of operations happen per clock cycle. Edge-triggering
//   ensures every flip-flop samples its input at exactly the same instant,
//   preventing data races between pipeline stages. Without this, a pipeline
//   would be chaos — data from stage 3 could leak into stage 5 before
//   stage 4 finishes processing.

/**
 * D Flip-Flop — captures data on the clock signal, master-slave design.
 *
 * The master-slave configuration creates edge-like behavior:
 * - When clock=0: master latch is transparent (absorbs data)
 * - When clock=1: slave latch is transparent (outputs master's captured value)
 *
 * To simulate a rising edge (0 -> 1 transition), call twice:
 *   1. First with clock=0 (master absorbs data)
 *   2. Then with clock=1 (slave outputs what master captured)
 *
 * @returns [Q, Q_bar, internalState] — the flip-flop output and internal
 *   state for passing back to the next call
 *
 * @example
 * // Clock low: master absorbs data=1
 * let [q, qBar, state] = dFlipFlop(1, 0);
 * // Clock high: slave outputs master's value
 * [q, qBar, state] = dFlipFlop(1, 1, state);
 * // q === 1
 */
export function dFlipFlop(
  data: Bit,
  clock: Bit,
  state?: FlipFlopState,
): [Bit, Bit, FlipFlopState] {
  validateBit(data, "data");
  validateBit(clock, "clock");

  if (!state) {
    state = freshFlipFlopState();
  }

  // Master latch: enabled when clock is LOW (NOT clock)
  //   When clock=0, NOT(clock)=1, so master is transparent — absorbs data
  //   When clock=1, NOT(clock)=0, so master holds its value
  const notClock = NOT(clock);
  const [masterQ, masterQBar] = dLatch(
    data,
    notClock,
    state.masterQ,
    state.masterQBar,
  );

  // Slave latch: enabled when clock is HIGH (clock directly)
  //   When clock=1, slave is transparent — outputs master's stored value
  //   When clock=0, slave holds its value
  const [slaveQ, slaveQBar] = dLatch(
    masterQ,
    clock,
    state.slaveQ,
    state.slaveQBar,
  );

  const newState: FlipFlopState = {
    masterQ,
    masterQBar,
    slaveQ,
    slaveQBar,
  };

  return [slaveQ, slaveQBar, newState];
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
//     Bit 2:  Data[2] --| D-FF |-- Out[2]
//     ...
//     Bit N:  Data[N] --| D-FF |-- Out[N]
//                         |
//     Clock --------------+ (shared by all flip-flops)
//
// Registers are the workhorses of any processor:
//   - x86 CPUs have 16 general-purpose 64-bit registers (RAX, RBX, ...)
//   - GPUs have thousands of 32-bit registers per streaming multiprocessor
//   - A GPU's register file can be several megabytes in total
//
// Each 32-bit register is literally 32 D flip-flops side by side.

/**
 * N-bit register — stores a binary word on the clock signal.
 *
 * Each bit position has its own D flip-flop. All flip-flops share
 * the same clock, so the entire word is captured simultaneously.
 *
 * @param data - List of bits to store, one per flip-flop position.
 * @param clock - Clock signal shared by all flip-flops.
 * @param state - Internal flip-flop states from previous call (undefined for initial).
 * @param width - Expected register width. If given, data must match.
 * @returns [outputBits, newState] — output bits and internal state for chaining
 *
 * @example
 * // Clock low: flip-flops absorb data
 * let [out, state] = register([1, 0, 1, 1], 0);
 * // Clock high: flip-flops output stored data
 * [out, state] = register([1, 0, 1, 1], 1, state);
 * // out === [1, 0, 1, 1]
 */
export function register(
  data: Bit[],
  clock: Bit,
  state?: FlipFlopState[],
  width?: number,
): [Bit[], FlipFlopState[]] {
  validateBit(clock, "clock");

  if (!Array.isArray(data)) {
    throw new TypeError("data must be an array of bits");
  }

  if (data.length === 0) {
    throw new RangeError("data must not be empty");
  }

  for (let i = 0; i < data.length; i++) {
    validateBit(data[i], `data[${i}]`);
  }

  if (width !== undefined && data.length !== width) {
    throw new RangeError(
      `data length ${data.length} does not match width ${width}`,
    );
  }

  const n = data.length;

  // Initialize state if this is the first call
  if (!state) {
    state = Array.from({ length: n }, () => freshFlipFlopState());
  }

  if (state.length !== n) {
    throw new RangeError(
      `state length ${state.length} does not match data length ${n}`,
    );
  }

  // Run each flip-flop independently with the shared clock
  const outputBits: Bit[] = [];
  const newState: FlipFlopState[] = [];

  for (let i = 0; i < n; i++) {
    const [q, , ffState] = dFlipFlop(data[i], clock, state[i]);
    outputBits.push(q);
    newState.push(ffState);
  }

  return [outputBits, newState];
}

// ===========================================================================
// SHIFT REGISTER — Serial-to-Parallel Conversion
// ===========================================================================
//
// A shift register is a chain of flip-flops where each one's output feeds
// into the next one's input. On each clock cycle, every bit shifts one
// position (left or right), and a new bit enters from the serial input.
//
// Right shift (direction="right"):
//
//     serial_in -> [FF_0] -> [FF_1] -> [FF_2] -> ... -> [FF_N-1] -> serial_out
//
//     Each clock cycle:
//       FF_0 gets serial_in
//       FF_1 gets old FF_0
//       FF_2 gets old FF_1
//       ...
//       serial_out = old FF_N-1
//
// Left shift (direction="left"):
//
//     serial_out <- [FF_0] <- [FF_1] <- [FF_2] <- ... <- [FF_N-1] <- serial_in
//
// Why shift registers matter for floating-point arithmetic:
//   When adding two floating-point numbers, their mantissas must be aligned
//   to the same exponent. If one number has exponent 5 and the other has
//   exponent 3, the smaller number's mantissa must be shifted RIGHT by 2
//   positions. This is done by a barrel shifter, which is built from
//   multiplexers and shift registers.
//
//   Example: 1.5 x 2^5 + 1.25 x 2^3
//   Step 1: Shift 1.25's mantissa right by 2: 0.0125 x 2^5
//   Step 2: Add aligned mantissas: 1.5 + 0.3125 = 1.8125 x 2^5

/**
 * Shift register — shifts bits through a chain of flip-flops.
 *
 * On each clock cycle, bits shift one position and a new bit enters
 * from the serial input. The bit that falls off the end becomes the
 * serial output.
 *
 * @param serialIn - Bit to feed into the first position
 * @param clock - Clock signal
 * @param state - Internal state from previous call (undefined for initial)
 * @param width - Number of bit positions (default 8)
 * @param direction - "right" shifts bits toward higher indices,
 *                    "left" shifts bits toward lower indices
 * @returns [parallelOut, serialOut, newState]
 *
 * @example
 * // Shift in a 1 from the right
 * let [out, sout, st] = shiftRegister(1, 0, undefined, 4);
 * [out, sout, st] = shiftRegister(1, 1, st, 4);
 * // out === [1, 0, 0, 0] — first 1 entered position 0
 */
export function shiftRegister(
  serialIn: Bit,
  clock: Bit,
  state?: FlipFlopState[],
  width: number = 8,
  direction: "right" | "left" = "right",
): [Bit[], Bit, FlipFlopState[]] {
  validateBit(serialIn, "serialIn");
  validateBit(clock, "clock");

  if (direction !== "right" && direction !== "left") {
    throw new RangeError(
      `direction must be 'right' or 'left', got '${direction}'`,
    );
  }

  if (width < 1) {
    throw new RangeError(`width must be >= 1, got ${width}`);
  }

  // Initialize state: all flip-flops start at 0
  if (!state) {
    state = Array.from({ length: width }, () => freshFlipFlopState());
  }

  if (state.length !== width) {
    throw new RangeError(
      `state length ${state.length} does not match width ${width}`,
    );
  }

  // Read current parallel output before shifting (from slaveQ of each FF)
  const currentValues: Bit[] = state.map((s) => s.slaveQ);

  // Determine the data inputs for each flip-flop based on shift direction
  //
  // Right shift: serial_in -> FF[0] -> FF[1] -> ... -> FF[N-1] -> serial_out
  //   FF[0] gets serial_in
  //   FF[i] gets current value of FF[i-1]
  //   serial_out = current value of FF[N-1]
  //
  // Left shift: serial_out <- FF[0] <- FF[1] <- ... <- FF[N-1] <- serial_in
  //   FF[N-1] gets serial_in
  //   FF[i] gets current value of FF[i+1]
  //   serial_out = current value of FF[0]

  let serialOut: Bit;
  let dataInputs: Bit[];

  if (direction === "right") {
    serialOut = currentValues[width - 1];
    dataInputs = [serialIn, ...currentValues.slice(0, width - 1)];
  } else {
    // left
    serialOut = currentValues[0];
    dataInputs = [...currentValues.slice(1), serialIn];
  }

  // Clock all flip-flops with their new data inputs
  const newState: FlipFlopState[] = [];
  const parallelOut: Bit[] = [];

  for (let i = 0; i < width; i++) {
    const [q, , ffState] = dFlipFlop(dataInputs[i], clock, state[i]);
    parallelOut.push(q);
    newState.push(ffState);
  }

  return [parallelOut, serialOut, newState];
}

// ===========================================================================
// COUNTER — Binary Counting
// ===========================================================================
//
// A counter is a register that increments its stored value on each clock
// cycle. It combines storage (register) with arithmetic (incrementer).
//
// The incrementer is built from a chain of half-adders:
//
//     Bit 0: sum = XOR(bit, carry_in=1)    carry_out = AND(bit, carry_in=1)
//     Bit 1: sum = XOR(bit, carry_out_0)   carry_out = AND(bit, carry_out_0)
//     Bit 2: sum = XOR(bit, carry_out_1)   carry_out = AND(bit, carry_out_1)
//     ...
//
// Starting with carry_in=1 means we add 1 to the current value — that's
// incrementing!
//
// When the counter reaches its maximum value (all 1s), the next increment
// wraps around to 0 (overflow). For an 8-bit counter, this means:
//     255 + 1 = 0 (with carry out)
//
// In GPUs, counters are used for:
//   - Pipeline stage tracking (which stage is each instruction in?)
//   - Warp schedulers (round-robin selection of thread warps)
//   - Performance counters (how many instructions executed?)
//   - Loop iteration counting in shader programs

/**
 * Binary counter — increments on each clock cycle.
 *
 * Combines a register with an incrementer circuit (chain of half-adders
 * starting with carry_in=1).
 *
 * @param clock - Clock signal
 * @param reset - When 1, counter resets to all zeros
 * @param state - Internal state from previous call (undefined for initial).
 * @param width - Number of bits (default 8, max count = 2^width - 1)
 * @returns [countBits, newState] — current counter value as bits (index 0 = LSB)
 *
 * @example
 * let [bits, st] = counter(0, 0, undefined, 4);  // Initialize
 * [bits, st] = counter(1, 0, st, 4);              // Tick 1
 * // bits === [1, 0, 0, 0] = decimal 1
 */
export function counter(
  clock: Bit,
  reset: Bit = 0,
  state?: CounterState,
  width: number = 8,
): [Bit[], CounterState] {
  validateBit(clock, "clock");
  validateBit(reset, "reset");

  if (width < 1) {
    throw new RangeError(`width must be >= 1, got ${width}`);
  }

  // Initialize state
  if (!state) {
    state = {
      value: Array.from({ length: width }, () => 0 as Bit),
      ffState: Array.from({ length: width }, () => freshFlipFlopState()),
    };
  }

  const currentValue: Bit[] = [...state.value];
  let ffState: FlipFlopState[] = [...state.ffState];

  // Reset: force all bits to 0
  let nextValue: Bit[];
  if (reset === 1) {
    nextValue = Array.from({ length: width }, () => 0 as Bit);
  } else {
    // Increment: add 1 using a chain of half-adders
    // A half-adder computes: sum = XOR(a, b), carry = AND(a, b)
    // We chain them with carry_in=1 (adding 1 to the current value)
    nextValue = [];
    let carry: Bit = 1; // carry_in = 1 means "add 1"
    for (let i = 0; i < width; i++) {
      const bit = currentValue[i];
      // Half-adder: sum and carry
      const sumBit = XOR(bit, carry);
      carry = AND(bit, carry);
      nextValue.push(sumBit);
    }
    // If carry=1 after the last bit, the counter overflows to 0
    // (which is exactly what nextValue already contains — all the
    // XOR results with the carry propagated through)
  }

  // Store the new value in the register
  const [output, newFfState] = register(nextValue, clock, ffState, width);

  // Only update the stored value when the register captures it (clock=1).
  // On clock=0, the master latch is transparent but slave holds — so the
  // counter value hasn't actually changed yet.
  const storedValue = clock === 1 ? nextValue : currentValue;

  const newState: CounterState = {
    value: storedValue,
    ffState: newFfState,
  };

  return [output, newState];
}
