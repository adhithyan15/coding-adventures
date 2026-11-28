// ============================================================================
// RankStatistics.swift — Exact, distribution-free statistics for medians
// ============================================================================
//
// A median interval is selected by an exact binomial-tail inequality. Swift's
// standard library has no arbitrary-precision integer, so the private
// BigNatural below stores little-endian base-2^32 limbs. Its operations are
// deliberately small: addition, multiplication by a small integer, exact
// division by a small integer, and comparison. No floating probability can
// round a rank across the cutoff.
//
// ============================================================================

import Foundation

public enum RankStatisticsError: Error, Equatable, LocalizedError {
    case emptySample
    case tooManyObservations(Int)
    case invalidConfidence(Int)
    case nonFiniteSample(Int)
    case arithmeticInvariant

    public var errorDescription: String? {
        switch self {
        case .emptySample:
            return "values must not be empty"
        case .tooManyObservations(let count):
            return "sample has \(count) observations; maximum is 10000"
        case .invalidConfidence(let confidence):
            return "confidence must be from 1 through 99, got \(confidence)"
        case .nonFiniteSample(let index):
            return "sample at index \(index) must be finite"
        case .arithmeticInvariant:
            return "exact binomial recurrence lost divisibility"
        }
    }
}

public struct MedianInterval: Equatable {
    public let low: Double
    public let high: Double

    public init(low: Double, high: Double) {
        self.low = low
        self.high = high
    }
}

private struct BigNatural {
    // Least-significant limb first. Zero is represented by [0].
    private var limbs: [UInt32]

    init(_ value: UInt32 = 0) {
        limbs = [value]
    }

    mutating func add(_ other: BigNatural) {
        let count = max(limbs.count, other.limbs.count)
        if limbs.count < count {
            limbs.append(contentsOf: repeatElement(0, count: count - limbs.count))
        }

        var carry: UInt64 = 0
        for index in 0..<count {
            let rhs = index < other.limbs.count ? UInt64(other.limbs[index]) : 0
            let sum = UInt64(limbs[index]) + rhs + carry
            limbs[index] = UInt32(truncatingIfNeeded: sum)
            carry = sum >> 32
        }
        if carry != 0 {
            limbs.append(UInt32(carry))
        }
    }

    mutating func multiply(by factor: UInt32) {
        var carry: UInt64 = 0
        for index in limbs.indices {
            let product = UInt64(limbs[index]) * UInt64(factor) + carry
            limbs[index] = UInt32(truncatingIfNeeded: product)
            carry = product >> 32
        }
        if carry != 0 {
            limbs.append(UInt32(carry))
        }
    }

    mutating func divideExactly(by divisor: UInt32) throws {
        precondition(divisor > 0)
        var remainder: UInt64 = 0
        for index in limbs.indices.reversed() {
            let numerator = (remainder << 32) | UInt64(limbs[index])
            limbs[index] = UInt32(numerator / UInt64(divisor))
            remainder = numerator % UInt64(divisor)
        }
        guard remainder == 0 else {
            throw RankStatisticsError.arithmeticInvariant
        }
        while limbs.count > 1, limbs.last == 0 {
            limbs.removeLast()
        }
    }

    func compared(to other: BigNatural) -> ComparisonResult {
        if limbs.count != other.limbs.count {
            return limbs.count < other.limbs.count ? .orderedAscending : .orderedDescending
        }
        for index in limbs.indices.reversed() where limbs[index] != other.limbs[index] {
            return limbs[index] < other.limbs[index] ? .orderedAscending : .orderedDescending
        }
        return .orderedSame
    }
}

private func sortedFinite(_ values: [Double]) throws -> [Double] {
    guard !values.isEmpty else {
        throw RankStatisticsError.emptySample
    }
    for (index, value) in values.enumerated() where !value.isFinite {
        throw RankStatisticsError.nonFiniteSample(index)
    }
    return values.sorted()
}

/// Returns the sample median without changing the input array.
///
/// The even-sample midpoint first adds its two central values. This preserves
/// subnormal rounding; if that sum overflows, halving each operand is safe.
public func median(_ values: [Double]) throws -> Double {
    let sorted = try sortedFinite(values)
    let middle = sorted.count / 2
    guard sorted.count.isMultiple(of: 2) else {
        return sorted[middle]
    }

    let low = sorted[middle - 1]
    let high = sorted[middle]
    let sum = low + high
    return sum.isFinite ? sum / 2 : low / 2 + high / 2
}

/// Returns the exact distribution-free interval for a population median.
///
/// The 10,000-observation bound caps the work performed by the exact
/// multi-limb binomial recurrence. `nil` means no confidence interval qualifies.
public func medianInterval(_ values: [Double], confidencePercent: Int) throws -> MedianInterval? {
    guard !values.isEmpty else {
        throw RankStatisticsError.emptySample
    }
    guard values.count <= 10_000 else {
        throw RankStatisticsError.tooManyObservations(values.count)
    }
    guard (1...99).contains(confidencePercent) else {
        throw RankStatisticsError.invalidConfidence(confidencePercent)
    }
    let sorted = try sortedFinite(values)
    let count = sorted.count

    var choose = BigNatural(1)
    var cumulative = BigNatural()
    var powerOfTwo = BigNatural(1)
    for _ in 0..<count {
        powerOfTwo.multiply(by: 2)
    }

    var bestRank = 0
    for zeroBasedRank in 0..<count {
        if zeroBasedRank > 0 {
            choose.multiply(by: UInt32(count - zeroBasedRank + 1))
            try choose.divideExactly(by: UInt32(zeroBasedRank))
        }
        cumulative.add(choose)

        var left = cumulative
        left.multiply(by: 200)
        var right = powerOfTwo
        right.multiply(by: UInt32(100 - confidencePercent))
        if left.compared(to: right) == .orderedDescending {
            break
        }
        bestRank = zeroBasedRank + 1
    }

    guard bestRank > 0 else {
        return nil
    }
    return MedianInterval(low: sorted[bestRank - 1], high: sorted[count - bestRank])
}
