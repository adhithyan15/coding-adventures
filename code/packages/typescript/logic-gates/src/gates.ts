/**
 * Logic Gates — the foundation of all digital computing.
 *
 * === What is a logic gate? ===
 *
 * A logic gate is the simplest possible decision-making element. It takes
 * one or two inputs, each either 0 or 1, and produces a single output
 * that is also 0 or 1. The output is entirely determined by the inputs —
 * there is no randomness, no hidden state, no memory.
 *
 * In physical hardware, gates are built from transistors — tiny electronic
 * switches etched into silicon. A modern CPU contains billions of transistors
 * organized into billions of gates. But conceptually, every computation a
 * computer performs — from adding numbers to rendering video to running AI
 * models — ultimately reduces to combinations of these simple 0-or-1 operations.
 *
 * This module implements the seven fundamental gates, proves that all of them
 * can be built from a single gate type (NAND), and provides multi-input variants.
 *
 * === Why only 0 and 1? ===
 *
 * Computers use binary (base-2) because transistors are most reliable as
 * on/off switches. A transistor that is "on" (conducting electricity)
 * represents 1. A transistor that is "off" (blocking electricity) represents 0.
 * You could theoretically build a computer using base-3 or base-10, but the
 * error margins for distinguishing between voltage levels would make it
 * unreliable. Binary gives us two clean, easily distinguishable states.
 */

// ---------------------------------------------------------------------------
// The Bit type
// ---------------------------------------------------------------------------
// In TypeScript, we use a union type to represent a binary digit. This gives
// us compile-time safety: the type checker will reject any value that isn't
// literally 0 or 1. At runtime, we still validate because TypeScript types
// are erased after compilation.

export type Bit = 0 | 1;

// ---------------------------------------------------------------------------
// CMOS gate instances — physical transistor models
// ---------------------------------------------------------------------------
// Each of the seven primitive gate functions delegates its digital evaluation
// to a CMOS transistor simulation from @coding-adventures/transistors. Using
// module-level singleton instances avoids allocating a new object on every
// call while still exercising the full transistor physics model.
//
// Default circuit parameters: 3.3 V Vdd, 180 nm CMOS process node.
import {
  CMOSInverter,
  CMOSNand,
  CMOSNor,
  CMOSAnd,
  CMOSOr,
  CMOSXor,
  CMOSXnor,
} from "@coding-adventures/transistors";

const _cmosNot  = new CMOSInverter();
const _cmosNand = new CMOSNand();
const _cmosNor  = new CMOSNor();
const _cmosAnd  = new CMOSAnd();
const _cmosOr   = new CMOSOr();
const _cmosXor  = new CMOSXor();
const _cmosXnor = new CMOSXnor();

// ---------------------------------------------------------------------------
// Input validation
// ---------------------------------------------------------------------------
// Every gate checks that its inputs are valid binary values (0 or 1).
// TypeScript's type system helps at compile time, but since types are erased
// at runtime, we need runtime checks too. This mirrors how real hardware
// enforces voltage thresholds — signals outside the valid range cause
// undefined behavior.

export function validateBit(value: unknown, name: string = "input"): asserts value is Bit {
  if (typeof value !== "number") {
    throw new TypeError(`${name} must be a number, got ${typeof value}`);
  }
  if (value !== 0 && value !== 1) {
    throw new RangeError(`${name} must be 0 or 1, got ${value}`);
  }
}

// ===========================================================================
// THE FOUR FUNDAMENTAL GATES
// ===========================================================================
// These are the building blocks. NOT, AND, OR, and XOR are the four gates
// from which all other gates (and all of digital logic) can be constructed.
//
// Each gate is defined by its "truth table" — an exhaustive listing of
// every possible input combination and the corresponding output. Since each
// input can only be 0 or 1, a two-input gate has exactly 4 possible input
// combinations (2 x 2 = 4), making it easy to verify correctness.

/**
 * The NOT gate (also called an "inverter").
 *
 * NOT is the simplest gate — it has one input and flips it.
 * If the input is 0, the output is 1. If the input is 1, the output is 0.
 *
 * Think of it like a light switch: if the light is off (0), flipping the
 * switch turns it on (1), and vice versa.
 *
 * Truth table:
 *     Input | Output
 *     ------+-------
 *       0   |   1
 *       1   |   0
 *
 * Circuit symbol:
 *     a -->o-- output
 *     (the small circle o means "invert")
 *
 * @example
 * NOT(0) // => 1
 * NOT(1) // => 0
 */
export function NOT(a: Bit): Bit {
  validateBit(a, "a");
  // Delegate to the CMOS inverter (2 transistors: 1 PMOS + 1 NMOS).
  return _cmosNot.evaluateDigital(a) as Bit;
}

/**
 * The AND gate.
 *
 * AND takes two inputs and outputs 1 ONLY if BOTH inputs are 1.
 * If either input is 0, the output is 0.
 *
 * Think of two switches wired in series (one after the other): electric
 * current can only flow through if both switches are closed (both = 1).
 *
 * Truth table:
 *     A  B  | Output
 *     ------+-------
 *     0  0  |   0      Neither is 1 -> 0
 *     0  1  |   0      Only B is 1 -> 0
 *     1  0  |   0      Only A is 1 -> 0
 *     1  1  |   1      Both are 1  -> 1
 *
 * Circuit symbol:
 *     a --+
 *         |D---- output
 *     b --+
 *
 * @example
 * AND(1, 1) // => 1
 * AND(1, 0) // => 0
 */
export function AND(a: Bit, b: Bit): Bit {
  validateBit(a, "a");
  validateBit(b, "b");
  // Delegate to the CMOS AND gate (NAND + inverter = 6 transistors).
  return _cmosAnd.evaluateDigital(a, b) as Bit;
}

/**
 * The OR gate.
 *
 * OR takes two inputs and outputs 1 if EITHER input is 1 (or both).
 * The output is 0 only when both inputs are 0.
 *
 * Think of two switches wired in parallel (side by side): current flows
 * if either switch is closed.
 *
 * Truth table:
 *     A  B  | Output
 *     ------+-------
 *     0  0  |   0      Neither is 1 -> 0
 *     0  1  |   1      B is 1       -> 1
 *     1  0  |   1      A is 1       -> 1
 *     1  1  |   1      Both are 1   -> 1
 *
 * Circuit symbol:
 *     a --\
 *          \---- output
 *     b --/
 *
 * @example
 * OR(0, 0) // => 0
 * OR(0, 1) // => 1
 */
export function OR(a: Bit, b: Bit): Bit {
  validateBit(a, "a");
  validateBit(b, "b");
  // Delegate to the CMOS OR gate (NOR + inverter = 6 transistors).
  return _cmosOr.evaluateDigital(a, b) as Bit;
}

/**
 * The XOR gate (Exclusive OR).
 *
 * XOR outputs 1 if the inputs are DIFFERENT. Unlike OR, XOR outputs 0
 * when both inputs are 1.
 *
 * The name "exclusive" means: one or the other, but NOT both.
 *
 * Truth table:
 *     A  B  | Output
 *     ------+-------
 *     0  0  |   0      Same      -> 0
 *     0  1  |   1      Different -> 1
 *     1  0  |   1      Different -> 1
 *     1  1  |   0      Same      -> 0
 *
 * Why XOR matters for arithmetic:
 *     In binary addition, 1 + 1 = 10 (that's "one-zero" in binary, which
 *     equals 2 in decimal). The sum digit is 0 and the carry is 1.
 *     Notice that the sum digit (0) is exactly what XOR(1, 1) produces!
 *
 *     0 + 0 = 0  ->  XOR(0, 0) = 0
 *     0 + 1 = 1  ->  XOR(0, 1) = 1
 *     1 + 0 = 1  ->  XOR(1, 0) = 1
 *     1 + 1 = 0  ->  XOR(1, 1) = 0  (carry the 1 separately)
 *
 *     This is why XOR is the key gate in building adder circuits.
 *
 * @example
 * XOR(1, 0) // => 1
 * XOR(1, 1) // => 0
 */
export function XOR(a: Bit, b: Bit): Bit {
  validateBit(a, "a");
  validateBit(b, "b");
  // Delegate to the CMOS XOR gate (4 NAND gates = 16 transistors).
  return _cmosXor.evaluateDigital(a, b) as Bit;
}

// ===========================================================================
// COMPOSITE GATES
// ===========================================================================
// These gates are built by combining fundamental gates. They are included
// because they appear frequently in digital circuits and have useful properties.

/**
 * The NAND gate (NOT AND).
 *
 * NAND is the inverse of AND: it outputs 1 in every case EXCEPT when both
 * inputs are 1.
 *
 * Truth table:
 *     A  B  | Output
 *     ------+-------
 *     0  0  |   1
 *     0  1  |   1
 *     1  0  |   1
 *     1  1  |   0      <- the only 0 output
 *
 * Why NAND is special — Functional Completeness:
 *     NAND has a remarkable property: you can build EVERY other gate using
 *     only NAND gates. This means if you had a factory that could only
 *     produce one type of gate, you'd pick NAND — because from NAND alone,
 *     you can construct NOT, AND, OR, XOR, and any other logic function.
 *
 *     This property is called "functional completeness" and it's why real
 *     chip manufacturers often build entire processors from NAND gates —
 *     they're the cheapest and simplest to manufacture.
 *
 *     See the nand_* functions below for proofs of how each gate is built
 *     from NAND.
 *
 * Implementation:
 *     NAND(a, b) = NOT(AND(a, b))
 *
 * @example
 * NAND(1, 1) // => 0
 * NAND(1, 0) // => 1
 */
export function NAND(a: Bit, b: Bit): Bit {
  // Delegate to the CMOS NAND gate (4 transistors — the natural CMOS primitive).
  return _cmosNand.evaluateDigital(a, b) as Bit;
}

/**
 * The NOR gate (NOT OR).
 *
 * NOR is the inverse of OR: it outputs 1 ONLY when both inputs are 0.
 *
 * Truth table:
 *     A  B  | Output
 *     ------+-------
 *     0  0  |   1      <- the only 1 output
 *     0  1  |   0
 *     1  0  |   0
 *     1  1  |   0
 *
 * Like NAND, NOR is also functionally complete — you can build every
 * other gate from NOR alone. (We don't demonstrate this here, but it's
 * a fun exercise!)
 *
 * Implementation:
 *     NOR(a, b) = NOT(OR(a, b))
 *
 * @example
 * NOR(0, 0) // => 1
 * NOR(0, 1) // => 0
 */
export function NOR(a: Bit, b: Bit): Bit {
  // Delegate to the CMOS NOR gate (4 transistors — the other natural CMOS primitive).
  return _cmosNor.evaluateDigital(a, b) as Bit;
}

/**
 * The XNOR gate (Exclusive NOR, also called "equivalence gate").
 *
 * XNOR is the inverse of XOR: it outputs 1 when the inputs are the SAME.
 *
 * Truth table:
 *     A  B  | Output
 *     ------+-------
 *     0  0  |   1      Same      -> 1
 *     0  1  |   0      Different -> 0
 *     1  0  |   0      Different -> 0
 *     1  1  |   1      Same      -> 1
 *
 * Use case:
 *     XNOR is used as an equality comparator. If you want to check whether
 *     two bits are equal, XNOR gives you the answer directly:
 *     XNOR(a, b) = 1 means a and b have the same value.
 *
 * Implementation:
 *     XNOR(a, b) = NOT(XOR(a, b))
 *
 * @example
 * XNOR(1, 1) // => 1
 * XNOR(1, 0) // => 0
 */
export function XNOR(a: Bit, b: Bit): Bit {
  // Delegate to the dedicated CMOS XNOR gate (XOR + Inverter = 8 transistors).
  return _cmosXnor.evaluateDigital(a, b) as Bit;
}

// ===========================================================================
// NAND-DERIVED GATES — Proving Functional Completeness
// ===========================================================================
// The functions below prove that NAND is functionally complete by building
// NOT, AND, OR, and XOR using ONLY the NAND gate. No other gate is used.
//
// This is not just an academic exercise. In real chip manufacturing, the
// ability to build everything from one gate type dramatically simplifies
// the fabrication process. The first commercially successful logic family
// (TTL 7400 series, introduced in 1966) was built around NAND gates.
//
// For each derived gate, we show:
// 1. The construction formula
// 2. A circuit diagram showing how NAND gates are wired
// 3. A proof by truth table that it matches the original gate

/**
 * NOT built entirely from NAND gates.
 *
 * Construction:
 *     NOT(a) = NAND(a, a)
 *
 * Why this works:
 *     NAND outputs 0 only when both inputs are 1.
 *     If we feed the same value to both inputs:
 *     - NAND(0, 0) = 1  (neither is 1, so NOT 0 = 1)
 *     - NAND(1, 1) = 0  (both are 1, so NOT 1 = 0)
 *
 * Circuit:
 *     a --+--+
 *         |  |D--o-- output
 *         +--+
 *     (both inputs of the NAND come from the same wire)
 *
 * @example
 * nandNot(0) // => 1
 * nandNot(1) // => 0
 */
export function nandNot(a: Bit): Bit {
  return NAND(a, a);
}

/**
 * AND built entirely from NAND gates.
 *
 * Construction:
 *     AND(a, b) = NOT(NAND(a, b)) = NAND(NAND(a, b), NAND(a, b))
 *
 * Why this works:
 *     NAND is the opposite of AND. So if we invert NAND's output (using
 *     our nandNot trick above), we get AND back.
 *
 * Circuit (2 NAND gates):
 *     a --+
 *         |D--o--+--+
 *     b --+      |  |D--o-- output
 *                +--+
 *     Gate 1: NAND(a, b)
 *     Gate 2: NAND(result, result) = NOT(result) = AND(a, b)
 *
 * @example
 * nandAnd(1, 1) // => 1
 * nandAnd(1, 0) // => 0
 */
export function nandAnd(a: Bit, b: Bit): Bit {
  return nandNot(NAND(a, b));
}

/**
 * OR built entirely from NAND gates.
 *
 * Construction:
 *     OR(a, b) = NAND(NOT(a), NOT(b)) = NAND(NAND(a,a), NAND(b,b))
 *
 * Why this works (De Morgan's Law):
 *     De Morgan's Law states: NOT(A AND B) = (NOT A) OR (NOT B)
 *     Rearranging: A OR B = NOT(NOT(A) AND NOT(B)) = NAND(NOT(A), NOT(B))
 *
 *     This is a fundamental identity in Boolean algebra, discovered by
 *     Augustus De Morgan in the 1800s — long before electronic computers
 *     existed!
 *
 * Circuit (3 NAND gates):
 *     a --+--+
 *         |  |D--o--+
 *         +--+      |
 *                   |D--o-- output
 *     b --+--+      |
 *         |  |D--o--+
 *         +--+
 *     Gate 1: NAND(a, a) = NOT(a)
 *     Gate 2: NAND(b, b) = NOT(b)
 *     Gate 3: NAND(NOT(a), NOT(b)) = OR(a, b)
 *
 * @example
 * nandOr(0, 1) // => 1
 * nandOr(0, 0) // => 0
 */
export function nandOr(a: Bit, b: Bit): Bit {
  return NAND(nandNot(a), nandNot(b));
}

/**
 * XOR built entirely from NAND gates.
 *
 * Construction:
 *     Let N = NAND(a, b)
 *     XOR(a, b) = NAND(NAND(a, N), NAND(b, N))
 *
 * Why this works:
 *     This is the most complex NAND construction. It uses 4 NAND gates.
 *     The intermediate value N = NAND(a, b) is reused twice, which is
 *     why XOR is more "expensive" in hardware than AND or OR.
 *
 *     Proof by truth table:
 *     a=0, b=0: N=NAND(0,0)=1, NAND(0,1)=1, NAND(0,1)=1, NAND(1,1)=0
 *     a=0, b=1: N=NAND(0,1)=1, NAND(0,1)=1, NAND(1,1)=0, NAND(1,0)=1
 *     a=1, b=0: N=NAND(1,0)=1, NAND(1,1)=0, NAND(0,1)=1, NAND(0,1)=1
 *     a=1, b=1: N=NAND(1,1)=0, NAND(1,0)=1, NAND(1,0)=1, NAND(1,1)=0
 *
 * Circuit (4 NAND gates):
 *     a --+----------+
 *         |          |D--o-- wire1 --+
 *         |   +--+ D--o-- N --+      |D--o-- output
 *     b --+---+              |      |
 *         |                  |D--o--+
 *         +------------------+
 *                           wire2
 *
 *     Gate 1: N = NAND(a, b)
 *     Gate 2: wire1 = NAND(a, N)
 *     Gate 3: wire2 = NAND(b, N)
 *     Gate 4: output = NAND(wire1, wire2)
 *
 * @example
 * nandXor(1, 0) // => 1
 * nandXor(1, 1) // => 0
 */
export function nandXor(a: Bit, b: Bit): Bit {
  const nandAB = NAND(a, b);
  return NAND(NAND(a, nandAB), NAND(b, nandAB));
}

/**
 * NOR built entirely from NAND gates.
 *
 * Construction:
 *     NOR(a, b) = NOT(OR(a, b))
 *               = NAND_NOT(NAND_OR(a, b))
 *               = nandNot(nandOr(a, b))
 *
 * We first build OR from NAND (3 gates), then invert it (1 more gate).
 * Total: 4 NAND gates.
 *
 * @example
 * nandNor(0, 0) // => 1
 * nandNor(0, 1) // => 0
 */
export function nandNor(a: Bit, b: Bit): Bit {
  return nandNot(nandOr(a, b));
}

/**
 * XNOR built entirely from NAND gates.
 *
 * Construction:
 *     XNOR(a, b) = NOT(XOR(a, b))
 *                = nandNot(nandXor(a, b))
 *
 * We first build XOR from NAND (4 gates), then invert it (1 more gate).
 * Total: 5 NAND gates.
 *
 * @example
 * nandXnor(1, 1) // => 1
 * nandXnor(1, 0) // => 0
 */
export function nandXnor(a: Bit, b: Bit): Bit {
  return nandNot(nandXor(a, b));
}

// ===========================================================================
// MULTI-INPUT GATES
// ===========================================================================
// In practice, you often need to AND or OR more than two values together.
// For example, "are ALL four conditions true?" requires a 4-input AND.
//
// Multi-input gates work by chaining 2-input gates. For AND:
//   AND_N(a, b, c, d) = AND(AND(AND(a, b), c), d)
//
// TypeScript's Array.reduce does exactly this: it takes an array and
// repeatedly applies a 2-argument function from left to right.

/**
 * AND with N inputs. Returns 1 only if ALL inputs are 1.
 *
 * This chains 2-input AND gates together using reduce:
 *     andN(a, b, c, d) = AND(AND(AND(a, b), c), d)
 *
 * In hardware, this would be a chain of AND gates:
 *     a --+
 *         |D-- r1 --+
 *     b --+         |D-- r2 --+
 *              c ---+         |D-- output
 *                       d ---+
 *
 * @example
 * andN(1, 1, 1, 1) // => 1
 * andN(1, 1, 0, 1) // => 0
 */
export function andN(...inputs: Bit[]): Bit {
  if (inputs.length < 2) {
    throw new RangeError("andN requires at least 2 inputs");
  }
  let result = inputs[0];
  for (let i = 1; i < inputs.length; i++) {
    result = AND(result, inputs[i]);
  }
  return result;
}

/**
 * OR with N inputs. Returns 1 if ANY input is 1.
 *
 * This chains 2-input OR gates together using reduce:
 *     orN(a, b, c, d) = OR(OR(OR(a, b), c), d)
 *
 * @example
 * orN(0, 0, 0, 0) // => 0
 * orN(0, 0, 1, 0) // => 1
 */
export function orN(...inputs: Bit[]): Bit {
  if (inputs.length < 2) {
    throw new RangeError("orN requires at least 2 inputs");
  }
  let result = inputs[0];
  for (let i = 1; i < inputs.length; i++) {
    result = OR(result, inputs[i]);
  }
  return result;
}

/**
 * XOR with N inputs — a parity checker (reduces a sequence of bits via XOR).
 *
 * Returns 1 if an ODD number of inputs are 1 (odd parity).
 * Returns 0 if an EVEN number of inputs are 1 (even parity).
 *
 * This is the gate basis for the Intel 8008's Parity flag:
 *     P = NOT(xorN(...result_bits))
 * P=1 means even parity (even number of 1-bits in the result).
 *
 * Hardware analogy: chain of 2-input XOR gates:
 *     xorN(a, b, c, d) = XOR(XOR(XOR(a, b), c), d)
 *
 * This is also called a "parity tree" in digital logic. It forms the
 * spine of error-detection codes (like parity bits in DRAM, or CRC).
 *
 * Truth table for 3 inputs:
 *     A  B  C  | # of 1s | Output
 *     ---------+---------+-------
 *     0  0  0  |    0    |   0    (even)
 *     0  0  1  |    1    |   1    (odd)
 *     0  1  0  |    1    |   1    (odd)
 *     0  1  1  |    2    |   0    (even)
 *     1  0  0  |    1    |   1    (odd)
 *     1  0  1  |    2    |   0    (even)
 *     1  1  0  |    2    |   0    (even)
 *     1  1  1  |    3    |   1    (odd)
 *
 * @param bits - Two or more Bit values (0 or 1).
 * @returns 1 if odd number of 1s, 0 if even number of 1s.
 *
 * @example
 * xorN(0, 0)          // => 0  (zero 1-bits = even)
 * xorN(1, 0)          // => 1  (one 1-bit = odd)
 * xorN(1, 1)          // => 0  (two 1-bits = even)
 * xorN(1, 1, 1)       // => 1  (three 1-bits = odd)
 * xorN(1, 0, 1, 0)    // => 0  (two 1-bits = even)
 */
export function xorN(...inputs: Bit[]): Bit {
  if (inputs.length === 0) {
    return 0 as Bit;
  }
  if (inputs.length === 1) {
    return inputs[0];
  }
  let result = inputs[0];
  for (let i = 1; i < inputs.length; i++) {
    result = XOR(result, inputs[i]);
  }
  return result;
}

// ===========================================================================
// MULTIPLEXER AND DEMULTIPLEXER
// ===========================================================================
// These are selector circuits that route data based on a control signal.
// They are fundamental to building more complex components like ALUs
// and memory systems.

/**
 * Multiplexer (MUX) — a 2-to-1 data selector.
 *
 * A MUX selects one of two inputs based on a selector signal:
 *   - sel=0: output = a
 *   - sel=1: output = b
 *
 * Think of it as a railroad switch: the selector controls which track
 * (input) connects to the output.
 *
 * Truth table:
 *     sel | Output
 *     ----+-------
 *      0  |   a
 *      1  |   b
 *
 * Implementation:
 *     MUX(a, b, sel) = OR(AND(a, NOT(sel)), AND(b, sel))
 *
 * Circuit:
 *     a ----AND---+
 *           |     |
 *     NOT(sel)    OR---- output
 *           |     |
 *     b ----AND---+
 *           |
 *         sel
 *
 * Why MUX matters:
 *     The ALU uses multiplexers to select which operation's result
 *     to output. If the opcode says "add", the MUX routes the adder's
 *     output to the ALU output, ignoring the outputs of AND, OR, etc.
 *
 * @example
 * mux(0, 1, 0) // => 0  (selects a)
 * mux(0, 1, 1) // => 1  (selects b)
 */
export function mux(a: Bit, b: Bit, sel: Bit): Bit {
  validateBit(a, "a");
  validateBit(b, "b");
  validateBit(sel, "sel");
  return OR(AND(a, NOT(sel)), AND(b, sel));
}

/**
 * Demultiplexer (DMUX) — a 1-to-2 data distributor.
 *
 * A DMUX takes one input and routes it to one of two outputs based on
 * a selector signal. The unselected output gets 0.
 *
 *   - sel=0: output_a = input, output_b = 0
 *   - sel=1: output_a = 0,     output_b = input
 *
 * Think of it as the reverse of a MUX: instead of selecting which input
 * to use, it selects which output to send to.
 *
 * Implementation:
 *     output_a = AND(input, NOT(sel))
 *     output_b = AND(input, sel)
 *
 * @example
 * dmux(1, 0) // => [1, 0]  (input goes to a)
 * dmux(1, 1) // => [0, 1]  (input goes to b)
 */
export function dmux(input: Bit, sel: Bit): [Bit, Bit] {
  validateBit(input, "input");
  validateBit(sel, "sel");
  return [AND(input, NOT(sel)), AND(input, sel)];
}
