// ============================================================================
// BigInt.swift -- a small, capped, arbitrary-precision integer
// ============================================================================
//
// WHY THIS EXISTS
// ---------------
// The pbt-v1 corpus draws 128-bit integers (`gen_primitive` / `i128`) and the
// generator algebra of VER05 section 3 promises `gen_i128(min, max)` over the
// whole i128 range. Swift has `Int`, `Int64` and `UInt64` but no 128-bit or
// arbitrary-width integer in the standard library (`Int128` only arrived in a
// newer toolchain, and it is not wide enough for the k-word rejection sampler
// of VER05e section 2.1 anyway). The package has zero dependencies, so we write
// the little bignum we need, the way the C lane (VER05i) does.
//
// REPRESENTATION
// --------------
// A sign flag plus a *magnitude* in little-endian base-2^32 limbs:
//
//     value = (-1)^negative * (limbs[0] + limbs[1]*2^32 + limbs[2]*2^64 + ...)
//
// The magnitude is always *normalised*: there is no high zero limb, so zero is
// the empty array, and zero is never negative. Normalisation makes `==` a plain
// field comparison and makes every operation's output canonical.
//
// WHY UInt32 LIMBS?
// -----------------
// A product of two 32-bit limbs plus a carry fits in a `UInt64`, so schoolbook
// multiplication needs no 128-bit intermediate. (The same trick as the C lane.)
//
// CAPS
// ----
// Untrusted input (the corpus file) must not be able to make us allocate or
// loop without bound, so sizes are capped:
//
//   * `apiMaxBits` = 1000 bits (about 300 decimal digits): the largest value a
//     caller-facing entry point accepts (decoded corpus integers, `integer`
//     bounds and spans).
//   * `maxBits` = 1100 bits: the hard ceiling for any arithmetic result. The
//     extra 100 bits are headroom for the rejection sampler, which needs
//     2^(64k) for a k-word draw of a span just under 1000 bits (k = 16 gives
//     2^1024). An operation that would exceed it is a *precondition failure*:
//     the validated entry points make it unreachable, and a silent wrap would be
//     worse than a loud stop.
//
// DIVISION
// --------
// Division is binary long division (shift and subtract), one bit at a time:
// O(bits x limbs), at most ~1100 x 35 steps. It is slow in the abstract and
// completely adequate here, and it is short enough to read and verify.
// `quotientAndRemainder` *truncates toward zero* (like Java's BigInteger and
// like Swift's own `/` and `%`); `shiftedRight` *floors* (like Python's `>>`
// and `//`), which is what the shrinker's `floor(diff / 2)` needs.
// ============================================================================

/// Errors raised by `BigInt.parse`.
public enum BigIntError: Error, Equatable {
    /// The text is not a canonical decimal integer (empty, a sign alone, a
    /// non-digit, a leading zero, or "-0").
    case notCanonical
    /// The text has more than `BigInt.maxDecimalDigits` digits.
    case tooLong
}

public struct BigInt: Equatable, Hashable, Comparable, CustomStringConvertible, ExpressibleByIntegerLiteral {
    /// The widest value (in bits) that an entry point accepts from callers.
    public static let apiMaxBits = 1000
    /// The widest result (in bits) any arithmetic operation may produce.
    public static let maxBits = 1100
    /// The most decimal digits `parse` accepts.
    public static let maxDecimalDigits = 300

    /// True for a negative value; always false for zero.
    private(set) var negative: Bool
    /// Little-endian base-2^32 magnitude, normalised (no high zero limb).
    private(set) var limbs: [UInt32]

    // MARK: construction ------------------------------------------------------

    private init(negative: Bool, limbs: [UInt32]) {
        var l = limbs
        while let last = l.last, last == 0 { l.removeLast() }
        precondition(l.count * 32 - (l.last.map { $0.leadingZeroBitCount } ?? 32) <= BigInt.maxBits,
                     "BigInt result exceeds \(BigInt.maxBits) bits")
        self.limbs = l
        self.negative = l.isEmpty ? false : negative
    }

    public static let zero = BigInt(negative: false, limbs: [])
    public static let one = BigInt(negative: false, limbs: [1])

    public init(_ value: Int) {
        let m = value.magnitude
        self.init(negative: value < 0, limbs: [UInt32(truncatingIfNeeded: m), UInt32(truncatingIfNeeded: UInt64(m) >> 32)])
    }

    /// Lets `x < 100` and `BigInt = 7` read naturally (literals fit an `Int`).
    public init(integerLiteral value: Int) { self.init(value) }

    public init(_ value: UInt64) {
        self.init(negative: false, limbs: [UInt32(truncatingIfNeeded: value), UInt32(truncatingIfNeeded: value >> 32)])
    }

    /// Parses a *canonical* decimal integer: an optional `-`, then digits with
    /// no leading zero (except the single digit "0"), and not "-0". At most
    /// `maxDecimalDigits` digits. This is the same grammar the corpus uses.
    public static func parse(_ text: String) throws -> BigInt {
        let bytes = Array(text.utf8)
        let neg = bytes.first == UInt8(ascii: "-")
        let digits = neg ? Array(bytes.dropFirst()) : bytes
        if digits.isEmpty { throw BigIntError.notCanonical }
        for d in digits where d < 0x30 || d > 0x39 { throw BigIntError.notCanonical }
        if digits[0] == 0x30 && (digits.count > 1 || neg) { throw BigIntError.notCanonical }
        if digits.count > maxDecimalDigits { throw BigIntError.tooLong }
        // Horner: acc = acc * 10 + digit, one digit at a time.
        var mag: [UInt32] = []
        for d in digits {
            mag = magMulSmallAdd(mag, 10, UInt32(d - 0x30))
        }
        return BigInt(negative: neg, limbs: mag)
    }

    // MARK: inspection --------------------------------------------------------

    public var isZero: Bool { limbs.isEmpty }
    public var signum: Int { isZero ? 0 : (negative ? -1 : 1) }
    /// The number of bits in the magnitude (0 for zero).
    public var bitWidth: Int { BigInt.magBits(limbs) }
    public var isEven: Bool { (limbs.first ?? 0) & 1 == 0 }
    public var magnitude: BigInt { BigInt(negative: false, limbs: limbs) }

    /// The low 64 bits of the two's-complement form: how the corpus reduces a
    /// (possibly negative or wider-than-64-bit) decimal seed modulo 2^64.
    public var lowU64: UInt64 {
        let lo = UInt64(limbs.count > 0 ? limbs[0] : 0) | (UInt64(limbs.count > 1 ? limbs[1] : 0) << 32)
        return negative ? (0 &- lo) : lo
    }

    /// The value as a `UInt64`, or nil when negative or too wide.
    public var asUInt64: UInt64? {
        if negative || limbs.count > 2 { return nil }
        return UInt64(limbs.count > 0 ? limbs[0] : 0) | (UInt64(limbs.count > 1 ? limbs[1] : 0) << 32)
    }

    /// The value as an `Int`, or nil when out of range.
    public var asInt: Int? {
        if limbs.count > 2 { return nil }
        let m = UInt64(limbs.count > 0 ? limbs[0] : 0) | (UInt64(limbs.count > 1 ? limbs[1] : 0) << 32)
        if negative {
            if m > UInt64(Int.max) + 1 { return nil }
            return m == UInt64(Int.max) + 1 ? Int.min : -Int(m)
        }
        return m > UInt64(Int.max) ? nil : Int(m)
    }

    // MARK: ordering ----------------------------------------------------------

    public static func < (a: BigInt, b: BigInt) -> Bool { compare(a, b) < 0 }

    /// -1, 0 or 1.
    public static func compare(_ a: BigInt, _ b: BigInt) -> Int {
        if a.negative != b.negative { return a.negative ? -1 : 1 }
        let c = magCompare(a.limbs, b.limbs)
        return a.negative ? -c : c
    }

    // MARK: arithmetic --------------------------------------------------------

    public static prefix func - (a: BigInt) -> BigInt { BigInt(negative: !a.negative, limbs: a.limbs) }

    public static func + (a: BigInt, b: BigInt) -> BigInt {
        if a.negative == b.negative {
            return BigInt(negative: a.negative, limbs: magAdd(a.limbs, b.limbs))
        }
        // Opposite signs: subtract the smaller magnitude from the larger; the
        // result takes the sign of the larger.
        let c = magCompare(a.limbs, b.limbs)
        if c == 0 { return .zero }
        if c > 0 { return BigInt(negative: a.negative, limbs: magSub(a.limbs, b.limbs)) }
        return BigInt(negative: b.negative, limbs: magSub(b.limbs, a.limbs))
    }

    public static func - (a: BigInt, b: BigInt) -> BigInt { a + (-b) }

    public static func * (a: BigInt, b: BigInt) -> BigInt {
        BigInt(negative: a.negative != b.negative, limbs: magMul(a.limbs, b.limbs))
    }

    /// Truncating division (the remainder has the dividend's sign).
    /// Precondition: `divisor` is not zero.
    public func quotientAndRemainder(dividingBy divisor: BigInt) -> (quotient: BigInt, remainder: BigInt) {
        precondition(!divisor.isZero, "BigInt division by zero")
        let (q, r) = BigInt.magDivMod(limbs, divisor.limbs)
        return (BigInt(negative: negative != divisor.negative, limbs: q), BigInt(negative: negative, limbs: r))
    }

    /// Multiplies by 2^n (n >= 0).
    public func shiftedLeft(_ n: Int) -> BigInt {
        precondition(n >= 0)
        if isZero { return self }
        precondition(bitWidth + n <= BigInt.maxBits, "BigInt shift exceeds \(BigInt.maxBits) bits")
        return BigInt(negative: negative, limbs: BigInt.magShiftLeft(limbs, n))
    }

    /// Divides by 2^n (n >= 0), rounding toward negative infinity.
    ///
    ///     (-3) >> 1 == -2        (3) >> 1 == 1
    ///
    /// A negative number's floor is `-(ceil(|x| / 2^n))`, so when any bit is
    /// shifted out of a negative magnitude we round the magnitude *up*.
    public func shiftedRight(_ n: Int) -> BigInt {
        precondition(n >= 0)
        let (shifted, lost) = BigInt.magShiftRight(limbs, n)
        if negative && lost { return BigInt(negative: true, limbs: BigInt.magAdd(shifted, [1])) }
        return BigInt(negative: negative, limbs: shifted)
    }

    // MARK: text --------------------------------------------------------------

    public var description: String {
        if isZero { return "0" }
        var chunks: [UInt32] = []
        var cur = limbs
        while !cur.isEmpty {
            let (q, r) = BigInt.magDivModSmall(cur, 1_000_000_000)
            chunks.append(r)
            cur = q
        }
        var s = negative ? "-" : ""
        s += String(chunks.last!)
        for c in chunks.dropLast().reversed() {
            let part = String(c)
            s += String(repeating: "0", count: 9 - part.count) + part
        }
        return s
    }

    // MARK: magnitude kernels (private, all on normalised [UInt32]) ----------

    private static func trim(_ a: inout [UInt32]) {
        while let last = a.last, last == 0 { a.removeLast() }
    }

    private static func magBits(_ a: [UInt32]) -> Int {
        guard let top = a.last else { return 0 }
        return a.count * 32 - top.leadingZeroBitCount
    }

    private static func magCompare(_ a: [UInt32], _ b: [UInt32]) -> Int {
        if a.count != b.count { return a.count < b.count ? -1 : 1 }
        var i = a.count - 1
        while i >= 0 {
            if a[i] != b[i] { return a[i] < b[i] ? -1 : 1 }
            i -= 1
        }
        return 0
    }

    private static func magAdd(_ a: [UInt32], _ b: [UInt32]) -> [UInt32] {
        var out: [UInt32] = []
        out.reserveCapacity(max(a.count, b.count) + 1)
        var carry: UInt64 = 0
        for i in 0..<max(a.count, b.count) {
            let s = UInt64(i < a.count ? a[i] : 0) + UInt64(i < b.count ? b[i] : 0) + carry
            out.append(UInt32(truncatingIfNeeded: s))
            carry = s >> 32
        }
        if carry != 0 { out.append(UInt32(carry)) }
        return out
    }

    /// a - b, requires a >= b.
    private static func magSub(_ a: [UInt32], _ b: [UInt32]) -> [UInt32] {
        var out = a
        var borrow: Int64 = 0
        for i in 0..<a.count {
            var d = Int64(a[i]) - Int64(i < b.count ? b[i] : 0) - borrow
            if d < 0 { d += 1 << 32; borrow = 1 } else { borrow = 0 }
            out[i] = UInt32(d)
        }
        trim(&out)
        return out
    }

    private static func magMul(_ a: [UInt32], _ b: [UInt32]) -> [UInt32] {
        if a.isEmpty || b.isEmpty { return [] }
        var out = [UInt32](repeating: 0, count: a.count + b.count)
        for i in 0..<a.count {
            var carry: UInt64 = 0
            for j in 0..<b.count {
                // 32x32 -> 64 bits, plus the existing limb, plus the carry:
                // at most (2^32-1)^2 + 2(2^32-1) = 2^64 - 1, so it never wraps.
                let t = UInt64(a[i]) * UInt64(b[j]) + UInt64(out[i + j]) + carry
                out[i + j] = UInt32(truncatingIfNeeded: t)
                carry = t >> 32
            }
            out[i + b.count] = UInt32(carry)
        }
        trim(&out)
        return out
    }

    private static func magMulSmallAdd(_ a: [UInt32], _ m: UInt32, _ add: UInt32) -> [UInt32] {
        var out: [UInt32] = []
        var carry = UInt64(add)
        for limb in a {
            let t = UInt64(limb) * UInt64(m) + carry
            out.append(UInt32(truncatingIfNeeded: t))
            carry = t >> 32
        }
        if carry != 0 { out.append(UInt32(carry)) }
        return out
    }

    private static func magDivModSmall(_ a: [UInt32], _ d: UInt32) -> ([UInt32], UInt32) {
        var q = [UInt32](repeating: 0, count: a.count)
        var rem: UInt64 = 0
        var i = a.count - 1
        while i >= 0 {
            let cur = (rem << 32) | UInt64(a[i])
            q[i] = UInt32(cur / UInt64(d))
            rem = cur % UInt64(d)
            i -= 1
        }
        trim(&q)
        return (q, UInt32(rem))
    }

    private static func magShiftLeft(_ a: [UInt32], _ n: Int) -> [UInt32] {
        if a.isEmpty { return a }
        let words = n / 32, bits = n % 32
        var out = [UInt32](repeating: 0, count: words)
        if bits == 0 {
            out.append(contentsOf: a)
        } else {
            var carry: UInt32 = 0
            for limb in a {
                out.append((limb << UInt32(bits)) | carry)
                carry = limb >> UInt32(32 - bits)
            }
            if carry != 0 { out.append(carry) }
        }
        return out
    }

    /// Returns the shifted magnitude and whether any nonzero bit fell off.
    private static func magShiftRight(_ a: [UInt32], _ n: Int) -> ([UInt32], Bool) {
        let words = n / 32, bits = n % 32
        if words >= a.count { return ([], !a.isEmpty) }
        var lost = false
        for i in 0..<words where a[i] != 0 { lost = true }
        var out: [UInt32] = []
        for i in words..<a.count {
            var v = a[i] >> UInt32(bits)
            if bits != 0 && i + 1 < a.count { v |= a[i + 1] << UInt32(32 - bits) }
            out.append(v)
        }
        if bits != 0 && (a[words] & ((1 << UInt32(bits)) - 1)) != 0 { lost = true }
        trim(&out)
        return (out, lost)
    }

    /// Binary long division of magnitudes: for each dividend bit from the top,
    /// shift it into the remainder and subtract the divisor when it fits.
    private static func magDivMod(_ a: [UInt32], _ b: [UInt32]) -> ([UInt32], [UInt32]) {
        if magCompare(a, b) < 0 { return ([], a) }
        if b.count == 1 {
            let (q, r) = magDivModSmall(a, b[0])
            return (q, r == 0 ? [] : [r])
        }
        var q = [UInt32](repeating: 0, count: a.count)
        var rem: [UInt32] = []
        var i = magBits(a) - 1
        while i >= 0 {
            rem = magShiftLeft(rem, 1)
            if (a[i / 32] >> UInt32(i % 32)) & 1 == 1 {
                if rem.isEmpty { rem = [1] } else { rem[0] |= 1 }
            }
            if magCompare(rem, b) >= 0 {
                rem = magSub(rem, b)
                q[i / 32] |= 1 << UInt32(i % 32)
            }
            i -= 1
        }
        trim(&q)
        return (q, rem)
    }
}
