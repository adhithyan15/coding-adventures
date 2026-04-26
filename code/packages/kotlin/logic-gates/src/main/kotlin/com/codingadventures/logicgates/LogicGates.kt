// ============================================================================
// LogicGates.kt — The foundation of all digital computing
// ============================================================================
//
// A logic gate is the simplest possible decision-making element. It takes
// one or two inputs, each either 0 or 1, and produces a single output that
// is also 0 or 1. The output is entirely determined by the inputs — there
// is no randomness, no hidden state, no memory.
//
// In physical hardware, gates are built from transistors — tiny electronic
// switches etched into silicon. A modern CPU contains billions of transistors
// organized into billions of gates. But conceptually, every computation a
// computer performs — from adding numbers to rendering video to running AI
// models — ultimately reduces to combinations of these simple 0-or-1 operations.
//
// This module implements the seven fundamental gates, proves that all of them
// can be built from a single gate type (NAND), and provides multi-input variants.
//
// Why only 0 and 1?
// -----------------
// Computers use binary (base-2) because transistors are most reliable as
// on/off switches. A transistor that is "on" (conducting electricity)
// represents 1. A transistor that is "off" (blocking electricity) represents 0.
// You could theoretically build a computer using base-3 or base-10, but the
// error margins for distinguishing between voltage levels would make it
// unreliable. Binary gives us two clean, easily distinguishable states.
//

package com.codingadventures.logicgates

/**
 * The seven fundamental logic gates plus multi-input variants and NAND-derived gates.
 *
 * All inputs and outputs are [Int] values constrained to {0, 1}.
 * Passing any other value throws [IllegalArgumentException].
 *
 * All functions are top-level in the companion `object LogicGates` — this is a
 * pure utility singleton with no mutable state.
 *
 * ## Gate summary
 * ```
 *   NOT (a)   — flip: 0→1, 1→0
 *   AND (a,b) — 1 only if BOTH are 1
 *   OR  (a,b) — 1 if EITHER is 1
 *   XOR (a,b) — 1 if inputs are DIFFERENT
 *   NAND(a,b) — NOT(AND(a,b)) — functionally complete
 *   NOR (a,b) — NOT(OR(a,b))
 *   XNOR(a,b) — NOT(XOR(a,b)) — equality comparator
 * ```
 */
object LogicGates {

    // =========================================================================
    // Input Validation
    // =========================================================================
    //
    // Every gate checks that its inputs are valid binary values (0 or 1).
    // We reject integers outside {0, 1} with a clear error message that
    // includes both the parameter name and the offending value.

    /**
     * Validate that [value] is a binary bit: the integer 0 or 1.
     *
     * @param value the value to check
     * @param name  the parameter name to include in the error message
     * @throws IllegalArgumentException if value is not 0 or 1
     */
    private fun validateBit(value: Int, name: String) {
        require(value == 0 || value == 1) {
            "$name must be 0 or 1, got: $value"
        }
    }

    // =========================================================================
    // THE FOUR FUNDAMENTAL GATES
    // =========================================================================
    //
    // NOT, AND, OR, and XOR are the four gates from which all other gates (and
    // all of digital logic) can be constructed.
    //
    // Each gate is defined by its "truth table" — an exhaustive listing of
    // every possible input combination and the corresponding output.

    // ── NOT ──────────────────────────────────────────────────────────────────
    //
    // The simplest gate: one input, one output. Flips the bit.
    //
    //   Input │ Output
    //   ──────┼───────
    //     0   │   1
    //     1   │   0
    //
    // Circuit symbol:  a ──▷○── output  (the small circle ○ means "invert")

    /**
     * NOT gate (inverter).
     *
     * Flips the input: 0 → 1, 1 → 0.
     *
     * ```
     * NOT(0) = 1
     * NOT(1) = 0
     * ```
     *
     * @param a input bit (0 or 1)
     * @return the complement of a
     */
    fun NOT(a: Int): Int {
        validateBit(a, "a")
        return if (a == 0) 1 else 0
    }

    // ── AND ──────────────────────────────────────────────────────────────────
    //
    // Outputs 1 ONLY if BOTH inputs are 1.
    //
    //   A  B  │ Output
    //   ──────┼───────
    //   0  0  │   0      Neither is 1 → 0
    //   0  1  │   0      Only B is 1  → 0
    //   1  0  │   0      Only A is 1  → 0
    //   1  1  │   1      Both are 1   → 1  ✓
    //
    // Analogy: two switches wired in series. Both must be closed for current
    // to flow.

    /**
     * AND gate.
     *
     * Outputs 1 only if BOTH inputs are 1.
     *
     * ```
     * AND(0, 0) = 0
     * AND(0, 1) = 0
     * AND(1, 0) = 0
     * AND(1, 1) = 1
     * ```
     *
     * @param a first input bit
     * @param b second input bit
     * @return 1 if a == 1 and b == 1, else 0
     */
    fun AND(a: Int, b: Int): Int {
        validateBit(a, "a")
        validateBit(b, "b")
        return if (a == 1 && b == 1) 1 else 0
    }

    // ── OR ───────────────────────────────────────────────────────────────────
    //
    // Outputs 1 if EITHER input is 1 (or both).
    //
    //   A  B  │ Output
    //   ──────┼───────
    //   0  0  │   0
    //   0  1  │   1      B is 1     → 1  ✓
    //   1  0  │   1      A is 1     → 1  ✓
    //   1  1  │   1      Both are 1 → 1  ✓
    //
    // Analogy: two switches wired in parallel. Either can be closed.

    /**
     * OR gate.
     *
     * Outputs 1 if EITHER input is 1 (or both).
     *
     * ```
     * OR(0, 0) = 0
     * OR(0, 1) = 1
     * OR(1, 0) = 1
     * OR(1, 1) = 1
     * ```
     *
     * @param a first input bit
     * @param b second input bit
     * @return 1 if a == 1 or b == 1, else 0
     */
    fun OR(a: Int, b: Int): Int {
        validateBit(a, "a")
        validateBit(b, "b")
        return if (a == 1 || b == 1) 1 else 0
    }

    // ── XOR ──────────────────────────────────────────────────────────────────
    //
    // Exclusive OR: outputs 1 if inputs are DIFFERENT. Unlike OR, XOR outputs
    // 0 when both inputs are 1.
    //
    //   A  B  │ Output
    //   ──────┼───────
    //   0  0  │   0      Same      → 0
    //   0  1  │   1      Different → 1  ✓
    //   1  0  │   1      Different → 1  ✓
    //   1  1  │   0      Same      → 0
    //
    // Why XOR matters for arithmetic:
    //   In binary addition, 1+1 = 10 (binary). The sum digit is 0 — exactly
    //   what XOR(1,1) produces. XOR gives the sum digit; AND gives the carry.
    //   This is why XOR is the key gate in half-adder and full-adder circuits.

    /**
     * XOR gate (Exclusive OR).
     *
     * Outputs 1 if the inputs are DIFFERENT.
     *
     * ```
     * XOR(0, 0) = 0
     * XOR(0, 1) = 1
     * XOR(1, 0) = 1
     * XOR(1, 1) = 0
     * ```
     *
     * @param a first input bit
     * @param b second input bit
     * @return 1 if a != b, else 0
     */
    fun XOR(a: Int, b: Int): Int {
        validateBit(a, "a")
        validateBit(b, "b")
        return if (a != b) 1 else 0
    }

    // =========================================================================
    // COMPOSITE GATES
    // =========================================================================
    //
    // Built from the fundamental gates. Included because they appear frequently
    // in digital circuits and have special properties.

    // ── NAND ─────────────────────────────────────────────────────────────────
    //
    // NAND = NOT(AND). Outputs 1 in every case EXCEPT when both inputs are 1.
    //
    //   A  B  │ Output
    //   ──────┼───────
    //   0  0  │   1
    //   0  1  │   1
    //   1  0  │   1
    //   1  1  │   0      ← the only 0 output
    //
    // Why NAND is special — Functional Completeness:
    //   NAND has a remarkable property: you can build EVERY other gate using
    //   ONLY NAND gates. If you had a factory that could only produce one type
    //   of gate, you'd pick NAND — from NAND alone you can construct NOT, AND,
    //   OR, XOR, and any other logic function. This is called "functional
    //   completeness." Real chip manufacturers often build processors from NAND
    //   gates because they're the cheapest to manufacture.
    //
    //   See nandNOT, nandAND, nandOR, nandXOR below for proofs.

    /**
     * NAND gate (NOT AND).
     *
     * Functionally complete: any logic function can be built from NAND alone.
     *
     * ```
     * NAND(0, 0) = 1
     * NAND(0, 1) = 1
     * NAND(1, 0) = 1
     * NAND(1, 1) = 0
     * ```
     *
     * @param a first input bit
     * @param b second input bit
     * @return NOT(AND(a, b))
     */
    fun NAND(a: Int, b: Int): Int = NOT(AND(a, b))

    // ── NOR ──────────────────────────────────────────────────────────────────
    //
    // NOR = NOT(OR). Outputs 1 ONLY when both inputs are 0.
    //
    //   A  B  │ Output
    //   ──────┼───────
    //   0  0  │   1      ← the only 1 output
    //   0  1  │   0
    //   1  0  │   0
    //   1  1  │   0
    //
    // Like NAND, NOR is also functionally complete.

    /**
     * NOR gate (NOT OR).
     *
     * ```
     * NOR(0, 0) = 1
     * NOR(0, 1) = 0
     * NOR(1, 0) = 0
     * NOR(1, 1) = 0
     * ```
     *
     * @param a first input bit
     * @param b second input bit
     * @return NOT(OR(a, b))
     */
    fun NOR(a: Int, b: Int): Int = NOT(OR(a, b))

    // ── XNOR ─────────────────────────────────────────────────────────────────
    //
    // XNOR = NOT(XOR). Outputs 1 when inputs are the SAME (equality comparator).
    //
    //   A  B  │ Output
    //   ──────┼───────
    //   0  0  │   1      Same      → 1  ✓
    //   0  1  │   0      Different → 0
    //   1  0  │   0      Different → 0
    //   1  1  │   1      Same      → 1  ✓
    //
    // Use case: XNOR(a, b) = 1 means a and b have the same value.

    /**
     * XNOR gate (Exclusive NOR — equivalence gate).
     *
     * Outputs 1 when both inputs are the SAME. Acts as a single-bit equality comparator.
     *
     * ```
     * XNOR(0, 0) = 1
     * XNOR(0, 1) = 0
     * XNOR(1, 0) = 0
     * XNOR(1, 1) = 1
     * ```
     *
     * @param a first input bit
     * @param b second input bit
     * @return NOT(XOR(a, b))
     */
    fun XNOR(a: Int, b: Int): Int = NOT(XOR(a, b))

    // =========================================================================
    // NAND-DERIVED GATES — Proving Functional Completeness
    // =========================================================================
    //
    // The functions below prove that NAND is functionally complete by building
    // NOT, AND, OR, and XOR using ONLY the NAND gate. No other gate is used.
    //
    // This is not just an academic exercise. The first commercially successful
    // logic family (TTL 7400 series, introduced 1966) was built around NAND
    // gates because they're the simplest to manufacture in silicon.

    /**
     * NOT built entirely from NAND gates.
     *
     * Construction: `NOT(a) = NAND(a, a)`
     *
     * Why it works: NAND(0,0)=1 and NAND(1,1)=0 — feeding the same wire
     * to both inputs of a NAND is equivalent to NOT.
     *
     * @param a input bit
     * @return NOT(a) using only NAND
     */
    fun nandNOT(a: Int): Int = NAND(a, a)

    /**
     * AND built entirely from NAND gates.
     *
     * Construction: `AND(a,b) = NAND(NAND(a,b), NAND(a,b)) = NOT(NAND(a,b))`
     *
     * NAND is the opposite of AND, so inverting NAND's output gives AND.
     * Uses 2 NAND gates.
     *
     * @param a first input bit
     * @param b second input bit
     * @return AND(a, b) using only NAND
     */
    fun nandAND(a: Int, b: Int): Int = nandNOT(NAND(a, b))

    /**
     * OR built entirely from NAND gates.
     *
     * Construction (De Morgan's Law):
     * `OR(a,b) = NAND(NAND(a,a), NAND(b,b)) = NAND(NOT(a), NOT(b))`
     *
     * De Morgan: `NOT(NOT(A) AND NOT(B)) = A OR B`. Uses 3 NAND gates.
     *
     * @param a first input bit
     * @param b second input bit
     * @return OR(a, b) using only NAND
     */
    fun nandOR(a: Int, b: Int): Int = NAND(nandNOT(a), nandNOT(b))

    /**
     * XOR built entirely from NAND gates.
     *
     * Construction:
     * ```
     * N = NAND(a, b)
     * XOR(a,b) = NAND(NAND(a, N), NAND(b, N))
     * ```
     *
     * Uses 4 NAND gates. The intermediate value N is reused twice.
     *
     * @param a first input bit
     * @param b second input bit
     * @return XOR(a, b) using only NAND
     */
    fun nandXOR(a: Int, b: Int): Int {
        val nandAb = NAND(a, b)
        return NAND(NAND(a, nandAb), NAND(b, nandAb))
    }

    // =========================================================================
    // MULTI-INPUT GATES
    // =========================================================================
    //
    // In practice, you often need to AND or OR more than two values together.
    // Multi-input gates work by chaining 2-input gates from left to right:
    //
    //   AND_N(a, b, c, d) = AND(AND(AND(a, b), c), d)
    //
    // Each varargs method requires at least 2 inputs.

    /**
     * AND with N inputs. Returns 1 only if ALL inputs are 1.
     *
     * Chains 2-input AND gates: `AND_N(a,b,c) = AND(AND(a,b), c)`.
     *
     * @param inputs two or more bit values (each must be 0 or 1)
     * @return 1 if all inputs are 1, else 0
     * @throws IllegalArgumentException if fewer than 2 inputs, or any is not 0/1
     */
    fun AND_N(vararg inputs: Int): Int {
        require(inputs.size >= 2) { "AND_N requires at least 2 inputs" }
        var result = AND(inputs[0], inputs[1])
        for (i in 2 until inputs.size) {
            result = AND(result, inputs[i])
        }
        return result
    }

    /**
     * OR with N inputs. Returns 1 if ANY input is 1.
     *
     * Chains 2-input OR gates: `OR_N(a,b,c) = OR(OR(a,b), c)`.
     *
     * @param inputs two or more bit values (each must be 0 or 1)
     * @return 1 if any input is 1, else 0
     * @throws IllegalArgumentException if fewer than 2 inputs, or any is not 0/1
     */
    fun OR_N(vararg inputs: Int): Int {
        require(inputs.size >= 2) { "OR_N requires at least 2 inputs" }
        var result = OR(inputs[0], inputs[1])
        for (i in 2 until inputs.size) {
            result = OR(result, inputs[i])
        }
        return result
    }

    /**
     * N-input XOR gate — parity checker.
     *
     * Returns 1 if an ODD number of inputs are 1 (odd parity).
     * Returns 0 if an EVEN number of inputs are 1 (even parity).
     *
     * This is how 8-bit parity flags are computed in hardware: a chain
     * of XOR gates reduces 8 bits to a single parity bit.
     *
     * Unlike AND_N and OR_N, XOR_N accepts 0 or 1 inputs:
     * - `XOR_N()` → 0 (zero ones is even parity; XOR identity is 0)
     * - `XOR_N(a)` → a (single input passes through unchanged)
     *
     * @param bits any number of bit values (each must be 0 or 1)
     * @return 1 if an odd number of inputs are 1, else 0
     * @throws IllegalArgumentException if any input is not 0/1
     */
    fun XOR_N(vararg bits: Int): Int {
        for (i in bits.indices) validateBit(bits[i], "bit[$i]")
        if (bits.isEmpty()) return 0
        var result = bits[0]
        for (i in 1 until bits.size) {
            result = XOR(result, bits[i])
        }
        return result
    }
}
