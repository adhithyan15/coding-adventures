// ============================================================================
// CtCompare.swift -- constant-time comparison and selection (CT01), Swift lane
// ============================================================================
//
// Why this exists
// ---------------
// A secret comparison that stops at the first differing byte tells an attacker
// how long the matching prefix was. Controlling one operand (a MAC tag, a
// session token) they can recover the other a byte at a time by timing guesses.
// The defence is to read EVERY byte whatever the data is, fold the differences
// into one accumulator, and look at the accumulator once at the very end:
//
//     a   = 41 42 43 44          a   = 41 42 43 44
//     b   = 41 42 43 44          b   = 41 FF 43 44
//     xor = 00 00 00 00          xor = 00 BD 00 00   (all four bytes were read)
//     acc = 0x00 -> equal        acc = 0xBD          -> different
//
// What this module promises -- and what it does not
// -------------------------------------------------
// The SOURCE has no content-dependent early return, branch, index or loop
// bound; loops run to the public length. It does NOT promise wall-clock
// constant time, and Swift cannot:
//
//   * The Swift optimiser (LLVM) may rewrite, unroll, vectorise or turn the
//     mask arithmetic back into a branch when it can prove a value is 0 or 1;
//     nothing in the language lets the source pin the generated code. Checking
//     the optimised assembly (`swiftc -O -emit-assembly`) is the only evidence,
//     and this package does not claim it has been done for every toolchain.
//   * Arrays are copy-on-write heap buffers managed by reference counting;
//     secret bytes can be copied (for example when an array is mutated while
//     shared) and left behind in freed memory. The package cannot zero them.
//   * Array subscripts are bounds-checked; those checks branch on the PUBLIC
//     index, never on data. Cache and speculation behaviour of the host are
//     outside the model.
//
// The selection bit
// -----------------
// Swift's Bool has no conversion to a number: `b ? 1 : 0` is a branch on the
// secret. So, like the Go, Ruby and Elixir lanes, the bit is an integer
// (UInt8) that must be exactly 0 (pick right) or 1 (pick left), and every
// equality has a bit-returning variant:
//
//     try ctSelectBytes(a, b, choice: ctEqBit(x, y))
//
// converts no boolean.
//
// Wrapping arithmetic
// -------------------
// Swift traps on integer overflow, so every place where the algorithm relies on
// two's-complement wraparound (0 - 1 = 255, 0 - d) uses the wrapping operators
// `&-` and `&+`.
// ============================================================================

/// The ways a CT01 operation can refuse. Both are decided from public
/// information (the lengths, and the range of the choice) before either
/// input's contents are read.
public enum CtCompareError: Error, Equatable {
    /// CT01's `CT_SELECT_LENGTH_MISMATCH`: the inputs differ in length.
    case selectLengthMismatch
    /// The selection bit was neither 0 nor 1.
    case invalidChoice
}

// OR together the XOR of every byte pair. Both arrays have the same length; the
// loop bound is that public length.
private func difference(_ left: [UInt8], _ right: [UInt8]) -> UInt8 {
    var acc: UInt8 = 0
    for i in 0..<left.count {
        acc |= left[i] ^ right[i]
    }
    return acc
}

/// `1` when the byte arrays are equal, else `0`. A length mismatch is `0` and
/// reads neither input's contents; lengths are public.
///
/// For an accumulator `acc` in 0..255, widened to 64 bits, `acc - 1` wraps to
/// all ones exactly when `acc == 0` and is otherwise 0..254 with the top bit
/// clear. A logical shift right by 63 therefore gives 1 for "equal" and 0 for
/// "different", with no comparison and no conditional.
public func ctEqBit(_ left: [UInt8], _ right: [UInt8]) -> UInt8 {
    if left.count != right.count {
        return 0
    }
    let acc = difference(left, right)
    return UInt8(truncatingIfNeeded: (UInt64(acc) &- 1) >> 63)
}

/// Fixed-width companion of ``ctEqBit(_:_:)``. Swift has no fixed-size byte
/// array in this API, so it is the same operation, kept so call sites read the
/// same in every CT01 lane.
public func ctEqFixedBit(_ left: [UInt8], _ right: [UInt8]) -> UInt8 {
    return ctEqBit(left, right)
}

/// Boolean equality without early exit. The `Bool` is the public result.
public func ctEq(_ left: [UInt8], _ right: [UInt8]) -> Bool {
    return ctEqBit(left, right) == 1
}

/// Boolean form of ``ctEqFixedBit(_:_:)``.
public func ctEqFixed(_ left: [UInt8], _ right: [UInt8]) -> Bool {
    return ctEqBit(left, right) == 1
}

/// A fresh copy of `left` when `choice` is 1 and of `right` when it is 0.
///
/// Throws ``CtCompareError/selectLengthMismatch`` for unequal lengths (checked
/// first) and ``CtCompareError/invalidChoice`` for any other `choice`; both are
/// decided before either input's contents are read.
///
/// With `mask = 0xFF` (choice 1) or `0x00` (choice 0):
///
///     out = right ^ ((left ^ right) & mask)
///
/// is `left` or `right` by the same instructions either way. The validity test
/// `choice & ~1 != 0` is a single computation with the same answer for both
/// valid values, so a valid call does identical work whichever bit is secret.
public func ctSelectBytes(_ left: [UInt8], _ right: [UInt8], choice: UInt8) throws -> [UInt8] {
    if left.count != right.count {
        throw CtCompareError.selectLengthMismatch
    }
    if choice & ~1 != 0 {
        throw CtCompareError.invalidChoice
    }
    let mask: UInt8 = 0 &- choice  // 0 -> 0x00, 1 -> 0xFF
    var out = [UInt8](repeating: 0, count: left.count)
    for i in 0..<left.count {
        let l = left[i]
        let r = right[i]
        out[i] = r ^ ((l ^ r) & mask)
    }
    return out
}

/// `1` when the two unsigned 64-bit values are equal, else `0`.
///
/// `d = x ^ y` is zero exactly when the values are equal. `d | (0 - d)` has its
/// top bit set exactly when `d != 0` (for a non-zero `d` either `d` or its
/// two's complement has it set), so shifting that bit down and flipping it
/// gives the equality bit. All 64 bit positions are folded by one fixed
/// operation sequence.
public func ctEqU64Bit(_ x: UInt64, _ y: UInt64) -> UInt8 {
    let d = x ^ y
    let negated = 0 &- d
    return UInt8(truncatingIfNeeded: ((d | negated) >> 63) ^ 1)
}

/// Boolean form of ``ctEqU64Bit(_:_:)``.
public func ctEqU64(_ x: UInt64, _ y: UInt64) -> Bool {
    return ctEqU64Bit(x, y) == 1
}
