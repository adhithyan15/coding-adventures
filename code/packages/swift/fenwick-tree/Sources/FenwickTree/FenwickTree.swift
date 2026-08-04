// ============================================================================
// FenwickTree.swift — Binary Indexed Tree (Fenwick Tree)
// ============================================================================
//
// A Fenwick tree stores a running array of `Double` values and answers two
// operations in O(log n):
//
//   • update(index, delta)  — add `delta` to the value at `index`
//   • prefixSum(index)      — sum of the first `index` values
//
// Range sums, point queries, and an order-statistic search (`findKth`) are all
// built on top. The trick is that each 1-based slot `i` stores the sum of the
// `lowbit(i)` values ending at `i`, where `lowbit(i) = i & -i` isolates the
// lowest set bit — so walking up (`i += lowbit(i)`) covers the updates and
// walking down (`i -= lowbit(i)`) accumulates a prefix, each in log-many steps.
//
// This is the Swift port of the `fenwick-tree` package in the coding-adventures
// monorepo; it mirrors the reference behaviour exactly (1-based indices,
// prefixSum(0) == 0). Where the Rust reference returns `Result`, this Swift
// version throws — the idiomatic Swift equivalent.

import Foundation

/// Errors thrown by ``FenwickTree`` operations.
public enum FenwickError: Error, Equatable {
    /// An index fell outside the valid range `[min, max]`.
    case indexOutOfRange(index: Int, min: Int, max: Int)
    /// A range had `left > right`.
    case invalidRange(left: Int, right: Int)
    /// `findKth` was called on a tree of size 0.
    case emptyTree
    /// `findKth` received a target `<= 0`.
    case nonPositiveTarget(target: Double)
    /// `findKth`'s target exceeded the total sum of the tree.
    case targetExceedsTotal(target: Double, total: Double)
}

/// A Binary Indexed Tree over `Double`, with 1-based external indices.
public struct FenwickTree: Equatable {
    private let n: Int
    private var bit: [Double]

    /// Isolate the lowest set bit of `i` (`i & -i`).
    private static func lowbit(_ i: Int) -> Int { i & (-i) }

    /// The largest power of two `<= n` (0 when `n == 0`).
    private static func highestPowerOfTwoAtMost(_ n: Int) -> Int {
        n == 0 ? 0 : 1 << (Int.bitWidth - n.leadingZeroBitCount - 1)
    }

    // ── Construction ─────────────────────────────────────────────────────────

    /// Create an all-zero tree that can hold `size` values.
    public init(size: Int) {
        self.n = max(size, 0)
        self.bit = Array(repeating: 0.0, count: self.n + 1)
    }

    /// Build a tree from `values` in O(n) via the in-place parent-propagation
    /// construction.
    public init(values: [Double]) {
        self.init(size: values.count)
        var i = 1
        while i <= n {
            bit[i] += values[i - 1]
            let parent = i + Self.lowbit(i)
            if parent <= n { bit[parent] += bit[i] }
            i += 1
        }
    }

    // ── Queries and updates ──────────────────────────────────────────────────

    /// Add `delta` to the value at 1-based `index`.
    public mutating func update(_ index: Int, delta: Double) throws {
        try checkIndex(index)
        var current = index
        while current <= n {
            bit[current] += delta
            current += Self.lowbit(current)
        }
    }

    /// Sum of the first `index` values. `prefixSum(0)` is `0`; `index` may range
    /// over `0...count`.
    public func prefixSum(_ index: Int) throws -> Double {
        guard index >= 0 && index <= n else {
            throw FenwickError.indexOutOfRange(index: index, min: 0, max: n)
        }
        var total = 0.0
        var current = index
        while current > 0 {
            total += bit[current]
            current -= Self.lowbit(current)
        }
        return total
    }

    /// Sum of the values in the inclusive 1-based range `left...right`.
    public func rangeSum(_ left: Int, _ right: Int) throws -> Double {
        guard left <= right else {
            throw FenwickError.invalidRange(left: left, right: right)
        }
        try checkIndex(left)
        try checkIndex(right)
        if left == 1 {
            return try prefixSum(right)
        }
        return try prefixSum(right) - prefixSum(left - 1)
    }

    /// The single value stored at 1-based `index`.
    public func pointQuery(_ index: Int) throws -> Double {
        try checkIndex(index)
        return try rangeSum(index, index)
    }

    /// The smallest 1-based index whose prefix sum is `>= target` (an
    /// order-statistic search). Requires strictly positive values for a
    /// meaningful result, matching the reference.
    public func findKth(_ target: Double) throws -> Int {
        guard n != 0 else { throw FenwickError.emptyTree }
        guard target > 0.0 else { throw FenwickError.nonPositiveTarget(target: target) }

        let total = try prefixSum(n)
        guard target <= total else {
            throw FenwickError.targetExceedsTotal(target: target, total: total)
        }

        var index = 0
        var step = Self.highestPowerOfTwoAtMost(n)
        var remaining = target
        while step > 0 {
            let next = index + step
            if next <= n && bit[next] < remaining {
                index = next
                remaining -= bit[index]
            }
            step >>= 1
        }
        return index + 1
    }

    // ── Introspection ────────────────────────────────────────────────────────

    /// The number of values the tree holds.
    public var count: Int { n }

    /// Whether the tree holds no values.
    public var isEmpty: Bool { n == 0 }

    /// A copy of the internal 1-based BIT array (index 0 dropped).
    public var bitArray: [Double] { Array(bit.dropFirst()) }

    private func checkIndex(_ index: Int) throws {
        guard index >= 1 && index <= n else {
            throw FenwickError.indexOutOfRange(index: index, min: 1, max: n)
        }
    }
}

extension FenwickTree: CustomStringConvertible {
    public var description: String { "FenwickTree(n=\(n), bit=\(bitArray))" }
}
