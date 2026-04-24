package com.codingadventures.gf256;

/**
 * Galois Field GF(2^8) arithmetic.
 *
 * <p>GF(256) is the finite field with exactly 256 elements: the integers 0..255.
 * Arithmetic in this field is very different from ordinary integer arithmetic:
 *
 * <ul>
 *   <li>Addition is XOR (characteristic-2 field: 1 + 1 = 0).</li>
 *   <li>Subtraction equals addition (every element is its own inverse).</li>
 *   <li>Multiplication is done via precomputed logarithm/antilogarithm tables
 *       (O(1) lookup instead of bit-level polynomial long multiplication).</li>
 * </ul>
 *
 * <p>Applications:
 * <ul>
 *   <li>Reed-Solomon error correction — QR codes, CDs, hard drives, deep-space probes</li>
 *   <li>AES encryption — SubBytes and MixColumns steps use GF(2^8)</li>
 * </ul>
 *
 * <h2>The Primitive Polynomial</h2>
 *
 * <p>The elements of GF(2^8) are polynomials over GF(2) of degree ≤ 7.
 * Multiplication requires reduction modulo an irreducible degree-8 polynomial.
 * We use:
 * <pre>
 *   p(x) = x^8 + x^4 + x^3 + x^2 + 1  =  0x11D  =  285
 * </pre>
 * This polynomial is <em>primitive</em>: the element g = 2 (the polynomial x)
 * generates the entire multiplicative group of order 255. That is,
 * g^0, g^1, ..., g^254 are exactly the 255 non-zero elements of GF(256).
 *
 * <h2>Log / Antilog Table Construction</h2>
 *
 * <p>We precompute two tables at class load time:
 * <ul>
 *   <li>{@code EXP[i]} = g^i mod p(x) — the antilogarithm (exponentiation) table</li>
 *   <li>{@code LOG[x]} = i such that g^i = x — the discrete logarithm table</li>
 * </ul>
 *
 * <p>Construction: start with value = 1. Each step, multiply by 2 (left shift 1 bit).
 * If the result overflows a byte (bit 8 set), XOR with 0x11D to reduce modulo p(x).
 * This is the "Russian peasant" or "shift-and-XOR" method.
 *
 * <p>The EXP table has 512 entries (indices 0..510): entries 0..254 are the standard
 * cycle, and entries 255..509 duplicate 0..254. This doubling lets the multiply
 * formula {@code EXP[(LOG[a] + LOG[b]) % 255]} avoid an extra bounds check — the
 * sum can be at most 254+254 = 508, which stays within the doubled table.
 *
 * <h2>Performance</h2>
 *
 * <p>All operations are O(1) table lookups. The table is 512 + 256 = 768 bytes,
 * far smaller than a CPU cache line cluster, so lookups are very fast.
 */
public final class GF256 {

    /**
     * The primitive (irreducible) polynomial used for modular reduction.
     *
     * <p>{@code p(x) = x^8 + x^4 + x^3 + x^2 + 1}
     * Binary: 1_0001_1101 = 0x11D = 285.
     *
     * <p>This polynomial cannot be factored into two polynomials of lower degree
     * over GF(2). Its irreducibility ensures every non-zero element has a
     * multiplicative inverse, making GF(256) a field. Its primitivity ensures
     * the element 2 generates all 255 non-zero elements.
     */
    public static final int PRIMITIVE_POLY = 0x11D;

    // =========================================================================
    // Log / Antilog Tables
    // =========================================================================
    //
    // EXP_TABLE[i] = 2^i mod p(x), for i in 0..511.
    //   - Entries 0..254: the standard antilogarithm cycle.
    //   - Entries 255..509: a copy of 0..254 (allows multiply without mod 255).
    //   - Entry 510 (= EXP[255]) = 1, since g^255 = g^0 = 1.
    //
    // LOG_TABLE[x] = i such that 2^i = x, for x in 1..255.
    //   LOG_TABLE[0] = -1 (sentinel; zero has no logarithm).

    /** Antilogarithm table. EXP_TABLE[i] = 2^i in GF(256). Length = 512. */
    static final int[] EXP_TABLE = new int[512];

    /** Logarithm table. LOG_TABLE[x] = discrete log base 2 of x in GF(256). */
    static final int[] LOG_TABLE = new int[256];

    static {
        int x = 1;
        for (int i = 0; i < 255; i++) {
            EXP_TABLE[i] = x;
            LOG_TABLE[x] = i;

            // Multiply x by the generator g = 2 (shift left 1 bit).
            x <<= 1;
            // If bit 8 is set, the polynomial overflowed degree 8.
            // Reduce modulo the primitive polynomial by XOR-ing with 0x11D.
            if ((x & 0x100) != 0) {
                x ^= PRIMITIVE_POLY;
            }
        }
        // Copy entries 0..254 into 255..509 so multiply can index without
        // needing a bounds-checked (sum % 255) that would branch.
        System.arraycopy(EXP_TABLE, 0, EXP_TABLE, 255, 255);
        // Sentinel: zero has no discrete logarithm.
        LOG_TABLE[0] = -1;
    }

    // Private constructor: this class is purely a namespace for static utilities.
    private GF256() {}

    // =========================================================================
    // Field Operations
    // =========================================================================

    /**
     * Add two GF(256) elements.
     *
     * <p>In a characteristic-2 field, addition is XOR. Each bit is a GF(2)
     * coefficient, and GF(2) satisfies 1 + 1 = 0 (mod 2). No carry, no overflow.
     *
     * <p>Examples:
     * <pre>
     *   add(0x53, 0xCA) = 0x53 ^ 0xCA = 0x99
     *   add(x, x)       = 0  for all x   (every element is its own additive inverse)
     * </pre>
     *
     * @param a first field element (0..255)
     * @param b second field element (0..255)
     * @return a XOR b
     */
    public static int add(int a, int b) {
        return a ^ b;
    }

    /**
     * Subtract two GF(256) elements.
     *
     * <p>In characteristic 2, subtraction equals addition: {@code -1 = 1}, so
     * negation is the identity function. This means XOR is both add and subtract.
     *
     * <p>This identity simplifies Reed-Solomon algorithms: syndrome computation,
     * Berlekamp-Massey, and Forney all use "minus" internally but it costs nothing
     * extra here.
     *
     * @param a the minuend (0..255)
     * @param b the subtrahend (0..255)
     * @return a XOR b
     */
    public static int sub(int a, int b) {
        return a ^ b;
    }

    /**
     * Multiply two GF(256) elements using the logarithm/antilogarithm tables.
     *
     * <p>Mathematical identity: {@code a * b = g^(log(a) + log(b))}
     * where g = 2 is the generator. This turns multiplication into two table
     * lookups and one addition — O(1) with no loop.
     *
     * <p>Special case: zero times anything is zero. Zero has no logarithm
     * (it is not reachable as any power of g), so we handle it explicitly.
     *
     * <p>The doubled EXP table (512 entries) means we never need a conditional
     * mod-255: the sum LOG[a] + LOG[b] is at most 254+254 = 508, safely within
     * the 512-entry table.
     *
     * <p>Examples:
     * <pre>
     *   mul(2, 2) = 4     (g^1 * g^1 = g^2 = 4)
     *   mul(0x53, 0x8C) = 1   (multiplicative inverses under 0x11D)
     * </pre>
     *
     * @param a first factor (0..255)
     * @param b second factor (0..255)
     * @return a * b in GF(256)
     */
    public static int mul(int a, int b) {
        if (a == 0 || b == 0) return 0;
        return EXP_TABLE[LOG_TABLE[a] + LOG_TABLE[b]];
    }

    /**
     * Divide a by b in GF(256).
     *
     * <p>{@code a / b = g^(log(a) - log(b))} in the cyclic group of order 255.
     * The {@code +255} before the modulo ensures the exponent is non-negative
     * when {@code LOG[a] < LOG[b]} (Java's {@code %} can return negative values
     * for negative operands, so we normalize into the range [0, 254]).
     *
     * <p>Special case: {@code 0 / b = 0} for any non-zero b.
     *
     * @param a the dividend (0..255)
     * @param b the divisor (1..255); must not be zero
     * @return a / b in GF(256)
     * @throws ArithmeticException if b is 0 (division by zero is undefined)
     */
    public static int div(int a, int b) {
        if (b == 0) throw new ArithmeticException("Division by zero in GF(256)");
        if (a == 0) return 0;
        return EXP_TABLE[((LOG_TABLE[a] - LOG_TABLE[b]) % 255 + 255) % 255];
    }

    /**
     * Raise a GF(256) element to a non-negative integer power.
     *
     * <p>Uses the logarithm table: {@code a^n = g^(log(a) * n mod 255)}.
     * The modulo 255 reflects Fermat's little theorem for finite fields:
     * every non-zero element satisfies {@code a^255 = 1}.
     *
     * <p>Special cases:
     * <ul>
     *   <li>{@code pow(0, 0) = 1} (convention, consistent with Math.pow)</li>
     *   <li>{@code pow(0, n) = 0} for n &gt; 0</li>
     *   <li>{@code pow(a, 0) = 1} for all a &ne; 0</li>
     * </ul>
     *
     * @param a the base (0..255)
     * @param n the exponent (must be &ge; 0)
     * @return a^n in GF(256)
     * @throws ArithmeticException if n is negative
     */
    public static int pow(int a, int n) {
        if (n < 0) throw new ArithmeticException("Exponent must be non-negative in GF(256)");
        if (n == 0) return 1;
        if (a == 0) return 0;
        return EXP_TABLE[((LOG_TABLE[a] * n) % 255 + 255) % 255];
    }

    /**
     * Compute the multiplicative inverse of a GF(256) element.
     *
     * <p>The inverse of {@code a} satisfies: {@code a * inv(a) = 1}.
     *
     * <p>Derivation: since the multiplicative group is cyclic of order 255,
     * {@code a * a^{-1} = 1 = g^0 = g^{255}}, so
     * {@code log(a) + log(a^{-1}) ≡ 0 (mod 255)}, which gives
     * {@code a^{-1} = ALOG[255 - LOG[a]]}.
     *
     * <p>This operation is fundamental to Reed-Solomon decoding (Forney algorithm)
     * and to AES SubBytes (the S-box is defined via GF(2^8) inverse).
     *
     * @param a the field element to invert (1..255)
     * @return the multiplicative inverse of a
     * @throws ArithmeticException if a is 0 (zero has no multiplicative inverse)
     */
    public static int inv(int a) {
        if (a == 0) throw new ArithmeticException("Zero has no multiplicative inverse in GF(256)");
        return EXP_TABLE[255 - LOG_TABLE[a]];
    }
}
