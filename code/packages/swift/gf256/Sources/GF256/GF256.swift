// GF256.swift
// Part of coding-adventures — an educational computing stack built from
// logic gates up through interpreters and compilers.
//
// ============================================================================
// MARK: - Galois Field GF(2^8) Arithmetic
// ============================================================================
//
// GF(2^8) — also written GF(256) — is the finite field with 256 elements.
// Its elements are the integers 0 through 255, each representing a polynomial
// over GF(2) (a polynomial whose coefficients are 0 or 1):
//
//   The byte 0b1010_0011 (= 163 = 0xA3) represents:
//     1·x⁷ + 0·x⁶ + 1·x⁵ + 0·x⁴ + 0·x³ + 0·x² + 1·x + 1
//     = x⁷ + x⁵ + x + 1
//
// ============================================================================
// Why GF(256)?
// ============================================================================
//
// Finite fields arise naturally wherever we need exact arithmetic that:
//   - Works on fixed-width data (one byte per element)
//   - Never overflows (results stay in 0..255)
//   - Has multiplicative inverses (so we can "divide")
//
// These properties make GF(256) ideal for:
//   - Reed-Solomon error correction (QR codes, CDs, DVDs, hard drives)
//   - AES encryption (SubBytes and MixColumns steps)
//   - Secret sharing schemes (Shamir's secret sharing)
//
// ============================================================================
// Addition in GF(2^8)
// ============================================================================
//
// In any GF(2^k) field, addition is bitwise XOR. Here is why:
//
// Each bit of a byte represents one coefficient of a polynomial over GF(2).
// Polynomial addition over GF(2) is coefficient-wise addition mod 2:
//
//   1 + 1 = 0  (mod 2)
//   1 + 0 = 1
//   0 + 0 = 0
//
// That is exactly XOR! So adding two GF(256) elements is just a ^ b.
//
// Consequence: Every element is its own additive inverse (a + a = 0),
// so subtraction equals addition in GF(2^8). No sign bits needed.
//
// ============================================================================
// Multiplication in GF(2^8)
// ============================================================================
//
// Multiplication is polynomial multiplication modulo the primitive polynomial:
//
//   p(x) = x^8 + x^4 + x^3 + x^2 + 1
//
// In hex: 1_0001_1101 = 0x11D = 285 decimal.
//
// This polynomial is:
//   (1) IRREDUCIBLE over GF(2): it has no polynomial factors of degree 1-7.
//       Irreducibility ensures every non-zero element has an inverse.
//   (2) PRIMITIVE: the element g = 2 (the polynomial x) generates all 255
//       non-zero elements as g^0, g^1, ..., g^254.
//
// Direct polynomial multiplication is O(n²). Instead, we precompute two
// lookup tables (LOG and ALOG) that convert multiplication to addition:
//
//   a × b = ALOG[(LOG[a] + LOG[b]) mod 255]
//
// This reduces multiplication to two table lookups and one addition — O(1).
//
// ============================================================================
// Log/Antilog Table Construction
// ============================================================================
//
// We build the tables at module initialization by repeated multiplication by 2:
//
//   ALOG[0] = 1  (g^0 = 1)
//   ALOG[i] = ALOG[i-1] × 2 (mod p(x))
//
// "Multiply by 2" in GF(2^8) means:
//   1. Left-shift by 1 bit.
//   2. If the result ≥ 256 (bit 8 is set), XOR with 0x11D to reduce
//      modulo the primitive polynomial (this is the modular reduction step).
//
// We also build the inverse LOG table: LOG[ALOG[i]] = i.
//
// ============================================================================
// Table Sizes
// ============================================================================
//
//   ALOG: 256 entries (indices 0..254 are the standard table;
//         index 255 = 1, because the multiplicative group has order 255:
//         g^255 = g^0 = 1. This entry is needed so that inverse(1) works:
//         ALOG[255 - LOG[1]] = ALOG[255 - 0] = ALOG[255] = 1.)
//
//   LOG:  256 entries (index 0 is unused because 0 has no logarithm;
//         index 1..255 hold the discrete logarithm base g.)
//
// ============================================================================

// ============================================================================
// MARK: - Table Construction (module-level, runs at startup)
// ============================================================================

/// Build the LOG and ALOG lookup tables at module initialization time.
///
/// - Returns: A tuple `(log, alog)` where:
///   - `log[x]`  = i such that 2^i = x in GF(256)  (for x in 1..255)
///   - `alog[i]` = 2^i in GF(256)                   (for i in 0..255)
///
/// Both tables use `UInt16` for intermediate values to detect overflow safely.
/// The final stored values fit in `UInt8` but we return `UInt16` for LOG
/// (it stores indices up to 255, which fits in UInt8, but UInt16 keeps the
/// code simple and avoids conversions in the arithmetic).
private func buildTables() -> (log: [UInt16], alog: [UInt8]) {
    var log  = [UInt16](repeating: 0, count: 256)
    var alog = [UInt8](repeating: 0, count: 256)

    // Start with the generator value 1 (= g^0 = 2^0).
    var val: UInt16 = 1

    for i in 0..<255 {
        // Record: g^i = val.
        alog[i] = UInt8(val)
        log[Int(val)] = UInt16(i)

        // Multiply val by 2 (the generator g = x in polynomial terms).
        //   Step 1: shift left 1 bit (multiply by x).
        //   Step 2: if bit 8 is set (value >= 256), reduce modulo p(x)
        //           by XOR-ing with 0x11D.
        val <<= 1
        if val >= 256 {
            // We use UInt16 here so `val` can temporarily hold values in
            // 256..511 before reduction. After XOR with 0x11D (= 285),
            // the result is always in 1..255 (fits in UInt8).
            val ^= 0x11D
        }
    }

    // ALOG[255] = 1: the multiplicative group has order 255,
    // so g^255 = g^0 = 1. Required by inverse(1):
    //   ALOG[255 - LOG[1]] = ALOG[255 - 0] = ALOG[255] = 1  ✓
    alog[255] = 1
    // LOG[0] stays 0 — it is never accessed for valid (non-zero) inputs.

    return (log: log, alog: alog)
}

/// Pre-built LOG and ALOG tables.
///
/// Built once at module load time and used by all GF256 operations.
/// Using a module-level `let` ensures thread safety (immutable after init).
private let tables: (log: [UInt16], alog: [UInt8]) = buildTables()

// ============================================================================
// MARK: - Public Namespace
// ============================================================================
//
// We wrap the GF(256) API in a `public enum GF256` (used as a namespace).
// Swift enums with no cases cannot be instantiated — they are pure namespaces.
// This prevents name collisions with Swift's built-in `+`, `*`, etc. operators
// while keeping call sites clean: `GF256.multiply(a, b)`.

/// GF(2^8) — the Galois field with 256 elements.
///
/// Elements are `UInt8` values (0..255). Arithmetic never overflows:
/// all results are guaranteed to be in 0..255.
///
/// ## Quick Reference
///
///   GF256.add(a, b)       → a XOR b
///   GF256.multiply(a, b)  → ALOG[(LOG[a] + LOG[b]) % 255]
///   GF256.inverse(a)      → ALOG[255 - LOG[a]]
///
public enum GF256 {

    // ========================================================================
    // MARK: - Constants
    // ========================================================================

    /// The additive identity: a + 0 = a for all a.
    public static let zero: UInt8 = 0

    /// The multiplicative identity: a × 1 = a for all a.
    public static let one: UInt8 = 1

    /// The primitive (irreducible) polynomial used for modular reduction.
    ///
    /// p(x) = x^8 + x^4 + x^3 + x^2 + 1
    ///
    /// In binary: 1_0001_1101. In hex: 0x11D. In decimal: 285.
    ///
    /// This polynomial is irreducible over GF(2) — it cannot be factored into
    /// lower-degree polynomials. Irreducibility ensures every non-zero element
    /// has a multiplicative inverse, making GF(256) a field.
    ///
    /// It is also primitive — the element g = 2 generates all 255 non-zero
    /// elements as consecutive powers: g^0, g^1, ..., g^254.
    public static let primitivePoly: UInt16 = 0x11D

    // ========================================================================
    // MARK: - Table Accessors (read-only views)
    // ========================================================================

    /// Antilogarithm table: `ALOG[i]` = 2^i in GF(256).
    ///
    /// Maps the exponent (discrete logarithm) back to a field element.
    /// ALOG is a bijection from {0..254} to {1..255} (the non-zero elements).
    ///
    /// Notable entries:
    ///   ALOG[0]  = 1     (2^0 = 1)
    ///   ALOG[1]  = 2     (2^1 = 2)
    ///   ALOG[7]  = 128   (2^7 = 0x80)
    ///   ALOG[8]  = 29    (2^8 mod p(x); 256 XOR 0x11D = 0x1D = 29)
    ///   ALOG[254]= 142   (2^254 = the last distinct non-zero element)
    ///   ALOG[255]= 1     (2^255 = 2^0 = 1; group order is 255)
    public static var ALOG: [UInt8] { tables.alog }

    /// Logarithm table: `LOG[x]` = i such that 2^i = x in GF(256).
    ///
    /// LOG[0] is undefined (there is no power of 2 that equals 0).
    /// For x in 1..255: ALOG[LOG[x]] = x  (LOG and ALOG are inverses).
    public static var LOG: [UInt16] { tables.log }

    // ========================================================================
    // MARK: - Addition and Subtraction
    // ========================================================================

    /// Add two GF(256) elements.
    ///
    /// In a characteristic-2 field, addition is XOR. Each bit represents a
    /// GF(2) polynomial coefficient, and GF(2) addition is 1+1=0 (mod 2),
    /// which is the definition of XOR.
    ///
    /// Properties:
    ///   - a + 0 = a  (0 is the additive identity)
    ///   - a + a = 0  (every element is its own inverse)
    ///   - a + b = b + a  (commutativity)
    ///
    /// Example:
    ///   add(0x53, 0xCA) = 0x53 XOR 0xCA = 0x99
    ///
    /// - Parameters:
    ///   - a: First field element (0..255).
    ///   - b: Second field element (0..255).
    /// - Returns: a + b in GF(256), always in 0..255.
    public static func add(_ a: UInt8, _ b: UInt8) -> UInt8 {
        return a ^ b
    }

    /// Subtract two GF(256) elements.
    ///
    /// In characteristic-2 fields, subtraction equals addition. This is
    /// because -1 ≡ 1 (mod 2): every element is its own additive inverse.
    ///
    /// This means a - b = a + b = a XOR b.
    ///
    /// - Parameters:
    ///   - a: The minuend (0..255).
    ///   - b: The subtrahend (0..255).
    /// - Returns: a - b in GF(256), which equals a XOR b.
    public static func subtract(_ a: UInt8, _ b: UInt8) -> UInt8 {
        return a ^ b
    }

    // ========================================================================
    // MARK: - Multiplication
    // ========================================================================

    /// Multiply two GF(256) elements using logarithm/antilogarithm tables.
    ///
    /// The mathematical identity:
    ///   a × b = g^(log_g(a) + log_g(b))
    ///
    /// where g = 2 is our generator. In table form:
    ///   multiply(a, b) = ALOG[(LOG[a] + LOG[b]) % 255]
    ///
    /// The modulo 255 keeps the exponent within the cyclic group of order 255.
    ///
    /// Special case: if either operand is 0, the result is 0.
    /// (Zero has no logarithm and is not reachable as a power of g.)
    ///
    /// Time complexity: O(1) — two table lookups and one addition.
    ///
    /// Example:
    ///   multiply(2, 3) = ALOG[(LOG[2] + LOG[3]) % 255]
    ///                  = ALOG[(1 + 25) % 255]
    ///                  = ALOG[26]
    ///                  = 6  (because 2 × 3 = 6 in GF(256))
    ///
    /// - Parameters:
    ///   - a: First factor (0..255).
    ///   - b: Second factor (0..255).
    /// - Returns: a × b in GF(256), always in 0..255.
    public static func multiply(_ a: UInt8, _ b: UInt8) -> UInt8 {
        // Zero times anything is zero. (0 has no log, so we special-case it.)
        if a == 0 || b == 0 { return 0 }
        let logA = Int(tables.log[Int(a)])
        let logB = Int(tables.log[Int(b)])
        return tables.alog[(logA + logB) % 255]
    }

    // ========================================================================
    // MARK: - Division
    // ========================================================================

    /// Divide a by b in GF(256).
    ///
    /// Using the identity:
    ///   a / b = a × b^(-1) = g^(log(a) - log(b))
    ///         = ALOG[(LOG[a] - LOG[b] + 255) % 255]
    ///
    /// The `+ 255` before the modulo ensures the result is non-negative when
    /// LOG[a] < LOG[b]. Without it, Swift's `%` would give a non-negative
    /// result for Int (unlike JavaScript), but the explicit `+ 255` makes
    /// the intent clear.
    ///
    /// Special case: a = 0 → result is 0 (0 ÷ b = 0 for any non-zero b).
    ///
    /// - Precondition: `b != 0`. Division by zero is undefined in any field.
    /// - Parameters:
    ///   - a: The dividend (0..255).
    ///   - b: The divisor (1..255; must not be 0).
    /// - Returns: a / b in GF(256), always in 0..255.
    public static func divide(_ a: UInt8, _ b: UInt8) -> UInt8 {
        precondition(b != 0, "GF256: division by zero")
        if a == 0 { return 0 }
        let logA = Int(tables.log[Int(a)])
        let logB = Int(tables.log[Int(b)])
        return tables.alog[(logA - logB + 255) % 255]
    }

    // ========================================================================
    // MARK: - Exponentiation
    // ========================================================================

    /// Raise a GF(256) element to a non-negative integer power.
    ///
    /// Uses the logarithm table:
    ///   base^exp = ALOG[(LOG[base] × exp) % 255]
    ///
    /// The modulo 255 reflects the order of the multiplicative group: every
    /// non-zero element satisfies g^255 = 1 (Fermat's little theorem for
    /// finite fields).
    ///
    /// Special cases:
    ///   - 0^0 = 1 by convention (standard in most numeric libraries)
    ///   - 0^n = 0 for n > 0
    ///   - a^0 = 1 for any a ≠ 0
    ///
    /// We use `UInt32` for `exp` to support large exponents safely. The
    /// intermediate product `LOG[base] × exp` could overflow Int if exp were
    /// extremely large, so we compute the modulo first.
    ///
    /// Example:
    ///   power(2, 8) = ALOG[(LOG[2] × 8) % 255]
    ///              = ALOG[(1 × 8) % 255]
    ///              = ALOG[8]
    ///              = 29   (2^8 mod p(x) = 0x1D = 29)
    ///
    /// - Parameters:
    ///   - base: The base element (0..255).
    ///   - exp: The exponent (non-negative).
    /// - Returns: base^exp in GF(256), always in 0..255.
    public static func power(_ base: UInt8, _ exp: UInt32) -> UInt8 {
        if base == 0 { return exp == 0 ? 1 : 0 }
        if exp == 0 { return 1 }
        let logBase = Int(tables.log[Int(base)])
        // Compute (logBase × exp) % 255 using modular arithmetic to avoid overflow.
        // Since logBase ≤ 254 and exp ≤ UInt32.max, their product could overflow
        // Int on 32-bit platforms. We reduce exp mod 255 first (the group order).
        let expMod = Int(exp % 255)
        let exponent = (logBase * expMod) % 255
        return tables.alog[exponent]
    }

    // ========================================================================
    // MARK: - Inverse
    // ========================================================================

    /// Compute the multiplicative inverse of a GF(256) element.
    ///
    /// The inverse of `a` satisfies: a × inverse(a) = 1.
    ///
    /// By the cyclic group property of the multiplicative group of order 255:
    ///   a × a^(-1) = 1 = g^0 = g^255
    ///   log(a) + log(a^(-1)) ≡ 0 (mod 255)
    ///   log(a^(-1)) = 255 - log(a)
    ///   a^(-1) = ALOG[255 - LOG[a]]
    ///
    /// Example:
    ///   inverse(2) = ALOG[255 - LOG[2]]
    ///              = ALOG[255 - 1]
    ///              = ALOG[254]
    ///              = 142  (2^254 mod p(x) = 0x8E = 142)
    ///   Verify: multiply(2, 142) = 1  ✓
    ///
    /// This operation is fundamental to AES (SubBytes step) and Reed-Solomon
    /// decoding (polynomial GCD over GF(256)).
    ///
    /// - Precondition: `a != 0`. Zero has no multiplicative inverse.
    /// - Parameter a: A non-zero GF(256) element (1..255).
    /// - Returns: The multiplicative inverse of `a` in GF(256).
    public static func inverse(_ a: UInt8) -> UInt8 {
        precondition(a != 0, "GF256: zero has no multiplicative inverse")
        let logA = Int(tables.log[Int(a)])
        return tables.alog[255 - logA]
    }
}

// ============================================================================
// MARK: - GF256Field — parameterizable field factory
// ============================================================================
//
// The GF256 namespace above is fixed to the Reed-Solomon polynomial 0x11D.
// AES uses 0x11B. GF256Field accepts any primitive polynomial and builds
// its own independent log/antilog tables.
//
// Usage:
//   let aes = GF256Field(polynomial: 0x11B)
//   aes.multiply(0x53, 0x8C)  // → 1   (AES GF(2^8) inverses)
//   aes.multiply(0x57, 0x83)  // → 0xC1 (FIPS 197 Appendix B)

/// A GF(2^8) field parameterized by an arbitrary primitive polynomial.
///
/// The `GF256` enum is fixed to the Reed-Solomon polynomial `0x11D`.
/// `GF256Field` lets you work with any polynomial — most notably `0x11B`
/// for AES.
///
/// Tables are built once in `init` and cached as stored properties.
/// All operations are O(1).
public struct GF256Field {

    /// The primitive (irreducible) polynomial for this field.
    public let polynomial: UInt16

    // Precomputed lookup tables for this polynomial.
    private let fieldLog:  [UInt16]
    private let fieldAlog: [UInt8]

    // ========================================================================
    // MARK: - Initialization
    // ========================================================================

    /// Create a GF(2^8) field for the given primitive polynomial.
    ///
    /// - Parameter polynomial: The degree-8 irreducible polynomial as a
    ///   `UInt16`, e.g. `0x11B` for AES or `0x11D` for Reed-Solomon.
    public init(polynomial: UInt16) {
        self.polynomial = polynomial

        var log  = [UInt16](repeating: 0, count: 256)
        var alog = [UInt8](repeating: 0, count: 256)

        var val: UInt16 = 1
        for i in 0..<255 {
            alog[i] = UInt8(val)
            log[Int(val)] = UInt16(i)
            val <<= 1
            if val >= 256 {
                val ^= polynomial
            }
        }
        // g^255 = 1 (the multiplicative group has order 255).
        alog[255] = 1

        self.fieldLog  = log
        self.fieldAlog = alog
    }

    // ========================================================================
    // MARK: - Operations
    // ========================================================================

    /// Add two elements: `a XOR b`.
    /// Addition is polynomial-independent; included for API symmetry.
    public func add(_ a: UInt8, _ b: UInt8) -> UInt8 { a ^ b }

    /// Subtract two elements: `a XOR b` (identical to add in GF(2^8)).
    public func subtract(_ a: UInt8, _ b: UInt8) -> UInt8 { a ^ b }

    /// Multiply two elements using this field's log/antilog tables.
    public func multiply(_ a: UInt8, _ b: UInt8) -> UInt8 {
        if a == 0 || b == 0 { return 0 }
        let logA = Int(fieldLog[Int(a)])
        let logB = Int(fieldLog[Int(b)])
        return fieldAlog[(logA + logB) % 255]
    }

    /// Divide `a` by `b`.
    ///
    /// - Precondition: `b != 0`.
    public func divide(_ a: UInt8, _ b: UInt8) -> UInt8 {
        precondition(b != 0, "GF256Field: division by zero")
        if a == 0 { return 0 }
        let logA = Int(fieldLog[Int(a)])
        let logB = Int(fieldLog[Int(b)])
        return fieldAlog[(logA - logB + 255) % 255]
    }

    /// Raise `base` to a non-negative integer power.
    public func power(_ base: UInt8, _ exp: UInt32) -> UInt8 {
        if base == 0 { return exp == 0 ? 1 : 0 }
        if exp == 0  { return 1 }
        let logBase = Int(fieldLog[Int(base)])
        let expMod  = Int(exp % 255)
        return fieldAlog[(logBase * expMod) % 255]
    }

    /// Compute the multiplicative inverse of `a`.
    ///
    /// - Precondition: `a != 0`.
    public func inverse(_ a: UInt8) -> UInt8 {
        precondition(a != 0, "GF256Field: zero has no multiplicative inverse")
        return fieldAlog[255 - Int(fieldLog[Int(a)])]
    }
}
