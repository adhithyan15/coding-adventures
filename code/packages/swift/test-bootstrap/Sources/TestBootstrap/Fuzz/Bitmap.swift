// ============================================================================
// Bitmap.swift -- coverage map, admission and the power schedule
// (VER06 section 4, VER06l section 4)
// ============================================================================
//
// Bit `e` set means edge `e` was executed. Edge ids are reduced modulo 65,536 as
// UNSIGNED 64-bit numbers. Collisions are the known price of a fixed-size map;
// AFL pays the same one.

public final class Bitmap {
    /// Number of edges the map distinguishes.
    public static let coverageMapSize = 65_536
    /// Size of the map in bytes.
    public static let coverageBytes = coverageMapSize / 8

    private var map = [UInt8](repeating: 0, count: Bitmap.coverageBytes)
    // The masked ids in the order first seen, beside the map, so listing the
    // edges of one execution costs O(edges hit), not O(8192).
    private var order: [Int] = []

    public init() {}

    /// The slot of an edge id: the low 16 bits are the id modulo 65,536.
    private static func slot(_ edge: UInt64) -> Int { Int(edge & UInt64(coverageMapSize - 1)) }

    /// Marks an edge as executed.
    public func record(_ edge: UInt64) {
        let e = Bitmap.slot(edge)
        let mask = UInt8(1) << UInt8(e & 7)
        if map[e >> 3] & mask == 0 {
            map[e >> 3] |= mask
            order.append(e)
        }
    }

    /// Whether an edge is set.
    public func has(_ edge: UInt64) -> Bool {
        let e = Bitmap.slot(edge)
        return map[e >> 3] & (UInt8(1) << UInt8(e & 7)) != 0
    }

    /// ORs the trace into this (cumulative) map; reports whether any edge was
    /// new and how many were.
    public func admit(_ trace: [Int]) -> (hasNew: Bool, countNew: Int) {
        var fresh = 0
        for e in trace {
            let id = UInt64(truncatingIfNeeded: e) // slots are non-negative
            if !has(id) {
                record(id)
                fresh += 1
            }
        }
        return (fresh > 0, fresh)
    }

    /// Every covered edge in ascending order.
    public func edges() -> [Int] { order.sorted() }

    /// The edges in first-seen order: the campaign's per-execution trace.
    func trace() -> [Int] { order }

    /// The number of distinct covered edges.
    public var count: Int { order.count }

    /// ORs another map into this one.
    public func merge(_ other: Bitmap) {
        for e in other.order { record(UInt64(e)) }
    }

    /// Clears the map in O(edges recorded).
    public func reset() {
        for e in order { map[e >> 3] &= ~(UInt8(1) << UInt8(e & 7)) }
        order.removeAll(keepingCapacity: true)
    }

    /// A copy of the 8,192 bytes of the map (LSB-first within a byte).
    public func raw() -> [UInt8] { map }

    /// The pure novelty check: does the trace hold an edge absent from
    /// `existing`? It counts UNIQUE novel edges (a trace that repeats one novel
    /// edge three times adds one). Ids are compared as full unsigned 64-bit
    /// words, not reduced.
    ///
    ///     corpusAdmit([1 2 3], [2 4]) -> (true, 1)    (edge 4 is new)
    ///     corpusAdmit([1 2 3], [1 2]) -> (false, 0)
    public static func corpusAdmit(existing: [UInt64], trace: [UInt64]) -> (admitted: Bool, newEdges: Int) {
        var have = Set(existing)
        var novel = 0
        for e in trace where have.insert(e).inserted { novel += 1 }
        return (novel > 0, novel)
    }

    /// The mutation budget: `max(1, round(base * (1 + new / total) / sqrt(len)))`.
    ///
    /// Short seeds that found a large share of the coverage get the most energy.
    /// The edge bonus is 1.0 while nothing is covered (total == 0). Rounding is
    /// half AWAY from zero (`floor(x + 0.5)`): round-half-even would turn 2.5
    /// into 2.
    ///
    /// Worked example: base 10, len 4, 3 new of 6 total edges:
    /// 10 * 1.5 / 2 = 7.5 rounds to 8.
    public static func calculateEnergy(base: Int, seedLen: Int, newEdges: Int, totalEdges: Int) -> Int {
        let b = Double(max(1, base))
        let bonus = totalEdges > 0 ? 1.0 + Double(newEdges) / Double(totalEdges) : 1.0
        let factor = Double(max(1, seedLen)).squareRoot()
        let energy = (b * bonus / factor + 0.5).rounded(.down)
        // Clamp BEFORE the Int conversion: no campaign runs more than maxIterations slots.
        let clamped = min(max(energy, 1), Double(Mutate.maxIterations))
        return clamped.isNaN ? 1 : Int(clamped)
    }
}
