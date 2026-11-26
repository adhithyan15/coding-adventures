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
 * Passing any other value throws [InvalidBitException] (a subclass of
 * [IllegalArgumentException]) naming the first offending parameter.
 *
 * ## Type-confusion note
 * Kotlin has no implicit numeric widening, so `AND(1L, 0)`, `AND('a', 1)`,
 * `AND(true, 0)` and `AND(1.0, 0)` are rejected by the COMPILER: every gate
 * takes plain [Int] parameters and there are no `Number`/`Any` overloads.
 * The only runtime rejection is therefore an out-of-range Int.
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
     * @throws InvalidBitException if value is not 0 or 1
     */
    private fun validateBit(value: Int, name: String) {
        if (value != 0 && value != 1) throw InvalidBitException(name, value)
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
    fun nandNOT(a: Int): Int {
        validateBit(a, "a")
        return NAND(a, a)
    }

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
    fun nandAND(a: Int, b: Int): Int {
        validateBit(a, "a")
        validateBit(b, "b")
        return nandNOT(NAND(a, b))
    }

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
    fun nandOR(a: Int, b: Int): Int {
        // Validate up front so the error names OUR parameters, not an inner gate's.
        validateBit(a, "a")
        validateBit(b, "b")
        return NAND(nandNOT(a), nandNOT(b))
    }

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
        validateBit(a, "a")
        validateBit(b, "b")
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

    /** Maximum number of inputs an N-ary gate accepts (bounds work per call). */
    const val MAX_INPUTS = 65536

    // Snapshot-then-validate (CWE-367 / CWE-400 / CWE-476), in this order:
    //   1. null guard. Kotlin's `vararg Int` is non-null, but Java callers can
    //      pass null; the module is compiled with -Xno-param-assertions so the
    //      compiler's own NullPointerException does not pre-empt this guard and
    //      every caller sees the package's InvalidShapeException.
    //   2. length checks on `inputs.size` BEFORE copying anything.
    //   3. ONE copyOf(); element validation and computation use the copy only,
    //      so a concurrently mutated caller array cannot change the outcome.
    // Shape errors therefore win over bit errors. The first bad element is
    // reported as "inputs[i]"; the name is built only on failure.
    private fun snapshot(gate: String, inputs: IntArray?, min: Int): IntArray {
        if (inputs == null) {
            throw InvalidShapeException("inputs", "$gate input array must not be null")
        }
        val n = inputs.size
        if (n < min) throw InvalidShapeException("inputs", "$gate requires at least $min inputs")
        if (n > MAX_INPUTS) {
            throw InvalidShapeException("inputs", "$gate accepts at most $MAX_INPUTS inputs, got $n")
        }
        val copy = inputs.copyOf()
        for (i in copy.indices) {
            val v = copy[i]
            if (v != 0 && v != 1) throw InvalidBitException("inputs[$i]", v)
        }
        return copy
    }

    /**
     * AND with N inputs. Returns 1 only if ALL inputs are 1.
     *
     * Chains 2-input AND gates: `AND_N(a,b,c) = AND(AND(a,b), c)`.
     *
     * @param inputs two or more bit values (each must be 0 or 1)
     * @return 1 if all inputs are 1, else 0
     * @throws InvalidShapeException if null, fewer than 2, or more than [MAX_INPUTS]
     * @throws InvalidBitException if any element is not 0/1 (parameter `inputs[i]`)
     */
    fun AND_N(vararg inputs: Int): Int {
        val v = snapshot("AND_N", inputs, 2)
        var result = AND(v[0], v[1])
        for (i in 2 until v.size) result = AND(result, v[i])
        return result
    }

    /**
     * OR with N inputs. Returns 1 if ANY input is 1.
     *
     * Chains 2-input OR gates: `OR_N(a,b,c) = OR(OR(a,b), c)`.
     *
     * @param inputs two or more bit values (each must be 0 or 1)
     * @return 1 if any input is 1, else 0
     * @throws InvalidShapeException if null, fewer than 2, or more than [MAX_INPUTS]
     * @throws InvalidBitException if any element is not 0/1 (parameter `inputs[i]`)
     */
    fun OR_N(vararg inputs: Int): Int {
        val v = snapshot("OR_N", inputs, 2)
        var result = OR(v[0], v[1])
        for (i in 2 until v.size) result = OR(result, v[i])
        return result
    }

    /**
     * N-input XOR gate — parity checker.
     *
     * Returns 1 if an ODD number of inputs are 1 (odd parity).
     * Returns 0 if an EVEN number of inputs are 1 (even parity).
     *
     * Unlike AND_N and OR_N, XOR_N accepts 0 or 1 inputs:
     * - `XOR_N()` → 0 (zero ones is even parity; XOR identity is 0)
     * - `XOR_N(a)` → a (single input passes through unchanged)
     *
     * @param inputs any number of bit values (each must be 0 or 1)
     * @return 1 if an odd number of inputs are 1, else 0
     * @throws InvalidShapeException if null or more than [MAX_INPUTS]
     * @throws InvalidBitException if any element is not 0/1 (parameter `inputs[i]`)
     */
    fun XOR_N(vararg inputs: Int): Int {
        val v = snapshot("XOR_N", inputs, 0)
        var result = 0
        for (i in v.indices) result = XOR(result, v[i])
        return result
    }
}

/**
 * Thrown when an N-ary gate receives an input array of the wrong shape:
 * null (from Java callers), too short (AND_N / OR_N need two inputs), or
 * longer than [LogicGates.MAX_INPUTS]. Distinct from [InvalidBitException].
 */
class InvalidShapeException(val parameter: String, message: String) :
    IllegalArgumentException("$parameter: $message") {
    init {
        requireNonNullArg(parameter, "parameter")
        requireNonNullArg(message, "message")
    }
}

/**
 * Thrown when a gate argument is not the integer 0 or 1.
 *
 * Extends [IllegalArgumentException] so existing callers keep working, but
 * carries the rejected [parameter] name and [value] for precise diagnosis.
 */
class InvalidBitException(val parameter: String, val value: Int) :
    IllegalArgumentException("$parameter must be 0 or 1, got: $value") {
    init {
        requireNonNullArg(parameter, "parameter")
    }
}

// The module is compiled with -Xno-param-assertions, so Java callers could
// otherwise smuggle null into non-null Kotlin properties. Constructors guard
// explicitly and throw IllegalArgumentException with a fixed message (the
// argument is typed nullable so the compiler keeps the check).
private fun requireNonNullArg(value: String?, name: String) {
    if (value == null) throw IllegalArgumentException("$name must not be null")
}
