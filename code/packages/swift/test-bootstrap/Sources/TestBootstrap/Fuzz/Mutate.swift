// ============================================================================
// Mutate.swift -- the fuzz mutator algebra (VER06 section 3, VER06l section 3)
// ============================================================================
//
// A fuzzer improves inputs by *mutating* them: flip a bit, bump an integer,
// cut a block out, paste one in. Every mutator here is a pure function from
// bytes to bytes that matches the shared `fuzz-v1` corpus byte for byte.
//
// TWO ENTRY POINTS PER MUTATOR
// ----------------------------
//   * the `UInt64` form (`bitFlip`, ...) is for engine code, where offsets and
//     counts are unsigned machine integers;
//   * the `BigInt` form (`bitFlipBig`, ...) is for the corpus runner, which may
//     be handed integers of 300 digits and negative ones. It reduces its
//     arguments EXACTLY (offsets wrap with FLOOR modulo, so -1 mod 8 is 7;
//     counts and lengths saturate, a negative one to 0) and then calls the same
//     core, so the two forms cannot drift apart.
//
// NO TRAPS
// --------
// Swift traps on overflow and on a bad index, and a trap kills the process. No
// caller or fixture value may reach one, so: every offset is reduced modulo a
// positive machine length before it indexes anything; every conversion to `Int`
// is either a modulo whose result is provably in range or a saturating clamp;
// and wrapping arithmetic is always spelled out (`&+`, `&*`).

/// A caller mistake or an exhausted cap. Messages are static text (they never
/// contain input bytes).
public enum FuzzError: Error, Equatable {
    /// A bad argument (range, size, null-like input).
    case invalidArgument(String)
    /// A `mutateSequence` call spent its work cap; `workDone` is the number of
    /// bytes it really copied, so a caller keeping an aggregate budget can
    /// charge a FAILED call too.
    case workExhausted(workDone: Int)
}

/// The byte order of the multi-byte mutators.
public enum Endian: Equatable {
    case little, big
}

public enum Mutate {
    /// Hard ceiling on a campaign's iterations, on `mutateSequence` steps and
    /// on the energy base.
    public static let maxIterations = 1_000_000
    /// Hard ceiling on an initial seed and on the size of a mutation sequence.
    public static let maxInputBytes = 1 << 20
    /// Hard ceiling on the cumulative bytes one `mutateSequence` call may copy.
    /// Each step copies the whole buffer, so `steps` alone does not bound the
    /// work (CWE-400): 10^6 steps over 1 MiB would be 10^12 byte copies.
    public static let maxSequenceWork = 1 << 28

    // MARK: interest lists (VER06 section 3.4) ---------------------------------

    private static let interest8: [UInt64] = [0, 1, 127, 128, 255]
    private static let interest16: [UInt64] = [0, 1, 255, 256, 32767, 32768, 65535]
    private static let interest32: [UInt64] = [0, 1, 65535, 65536, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF]
    private static let interest64: [UInt64] = [
        0, 1, 0xFFFF_FFFF, 0x1_0000_0000, 0x7FFF_FFFF_FFFF_FFFF, 0x8000_0000_0000_0000, 0xFFFF_FFFF_FFFF_FFFF,
    ]
    /// The 8-bit arithmetic deltas `mutateSequence` draws from.
    private static let deltas8: [Int64] = [-35, -8, -1, 1, 8, 35]

    /// The boundary values for a width in bytes (1, 2, 4 or 8), else nil.
    public static func interestValues(width: Int) -> [UInt64]? {
        switch width {
        case 1: return interest8
        case 2: return interest16
        case 4: return interest32
        case 8: return interest64
        default: return nil
        }
    }

    /// The 8-bit arithmetic deltas of `mutateSequence`.
    public static func sequenceDeltas() -> [Int64] { deltas8 }

    private static func validWidth(_ w: Int) -> Bool { w == 1 || w == 2 || w == 4 || w == 8 }

    // MARK: integer helpers -----------------------------------------------------

    /// `v mod m` for an unsigned value and a positive length; the result is in
    /// `[0, m)` and therefore fits an `Int`.
    private static func modU(_ v: UInt64, _ m: Int) -> Int {
        Int(v % UInt64(m))
    }

    /// FLOOR modulo of an arbitrary integer by a positive machine length.
    private static func modBig(_ v: BigInt, _ m: Int) -> Int {
        let r = v.quotientAndRemainder(dividingBy: BigInt(m)).remainder
        let fixed = r.signum < 0 ? r + BigInt(m) : r
        return fixed.asInt ?? 0 // in [0, m): always representable
    }

    /// Clamps into [0, 2^64): a negative value saturates to 0.
    static func satU64(_ v: BigInt) -> UInt64 {
        if v.signum < 0 { return 0 }
        return v.asUInt64 ?? UInt64.max
    }

    /// `v` modulo 2^64 in two's complement (so -1 is all ones).
    static func low64(_ v: BigInt) -> UInt64 { v.lowU64 }

    /// A UInt64 saturated into a non-negative `Int` (lengths and counts).
    private static func clampInt(_ v: UInt64) -> Int { v > UInt64(Int.max) ? Int.max : Int(v) }

    // MARK: flips, by parity ------------------------------------------------------

    /// Inverts `count` consecutive units (bits or bytes) from `base`, wrapping
    /// around a buffer of n units.
    ///
    /// The window of `count` covers every unit `count / n` times plus the first
    /// `count % n` units (from the wrapped start) once more. A unit flipped an
    /// even number of times is back where it began (XOR twice), so only parity
    /// matters:
    ///
    ///     flips(step) = count / n + (1 if step < count % n else 0)
    ///
    /// That is O(len) work for ANY count, where a loop of `count` single flips
    /// would never finish for count = 10^18 (VER06 section 9.2). Worked
    /// example, 2 bytes = 16 bits, offset 0, count 17: 17 / 16 = 1 (every bit
    /// once, odd) and 17 % 16 = 1 (bit 0 once more, so twice, even): the result
    /// inverts bits 1..15 and leaves bit 0.
    private static func parityFlip(_ data: [UInt8], bits: Bool, base: UInt64, oddFull: Bool, rem: UInt64, n: UInt64) -> [UInt8] {
        var buf = data
        if oddFull {
            for i in buf.indices { buf[i] ^= 0xFF }
        }
        var step: UInt64 = 0
        while step < rem {
            let pos = (base &+ step) % n // base < n and step < n: no wrap in practice
            if bits {
                buf[Int(pos / 8)] ^= UInt8(1) << UInt8(pos % 8)
            } else {
                buf[Int(pos)] ^= 0xFF
            }
            step += 1
        }
        return buf
    }

    /// The number of flippable units (bits or bytes) of a buffer, or nil if
    /// the count does not fit 64 bits (it cannot for a real array).
    private static func units(_ data: [UInt8], bits: Bool) -> UInt64? {
        let n = UInt64(data.count)
        if !bits { return n }
        let (m, overflow) = n.multipliedReportingOverflow(by: 8)
        return overflow ? nil : m
    }

    private static func flip(_ data: [UInt8], bits: Bool, offset: UInt64, count: UInt64) -> [UInt8] {
        guard !data.isEmpty, count != 0, let n = units(data, bits: bits) else { return data }
        return parityFlip(data, bits: bits, base: offset % n, oddFull: ((count / n) & 1) == 1, rem: count % n, n: n)
    }

    private static func flipBig(_ data: [UInt8], bits: Bool, offset: BigInt, count: BigInt) -> [UInt8] {
        guard !data.isEmpty, count.signum > 0, let n = units(data, bits: bits) else { return data }
        let bn = BigInt(n)
        let qr = count.quotientAndRemainder(dividingBy: bn)
        var base = offset.quotientAndRemainder(dividingBy: bn).remainder
        if base.signum < 0 { base = base + bn }
        return parityFlip(data, bits: bits, base: base.asUInt64 ?? 0, oddFull: !qr.quotient.isEven,
                          rem: qr.remainder.asUInt64 ?? 0, n: n)
    }

    /// Inverts `count` consecutive bits from `bitOffset mod (8 * len)`. Bits are
    /// numbered LSB-first within each byte: bit 9 of `00 00` is bit 1 of byte 1,
    /// so `bitFlip(00 00, 9, 1)` is `00 02`. An empty input or a zero count is a
    /// no-op.
    public static func bitFlip(_ data: [UInt8], bitOffset: UInt64, count: UInt64) -> [UInt8] {
        flip(data, bits: true, offset: bitOffset, count: count)
    }
    public static func bitFlipBig(_ data: [UInt8], bitOffset: BigInt, count: BigInt) -> [UInt8] {
        flipBig(data, bits: true, offset: bitOffset, count: count)
    }
    /// XORs `count` consecutive bytes with 0xFF from `byteOffset mod len`.
    public static func byteFlip(_ data: [UInt8], byteOffset: UInt64, count: UInt64) -> [UInt8] {
        flip(data, bits: false, offset: byteOffset, count: count)
    }
    public static func byteFlipBig(_ data: [UInt8], byteOffset: BigInt, count: BigInt) -> [UInt8] {
        flipBig(data, bits: false, offset: byteOffset, count: count)
    }

    // MARK: arithmetic and boundary values ----------------------------------------

    private static func readUint(_ b: [UInt8], _ at: Int, _ w: Int, _ e: Endian) -> UInt64 {
        var v: UInt64 = 0
        for i in 0..<w {
            let x = UInt64(b[at + i])
            if e == .big { v = (v << 8) | x } else { v |= x << UInt64(8 * i) }
        }
        return v
    }

    /// Stores the low `w` bytes of `v`: that is "v mod 2^(8w)".
    private static func writeUint(_ b: inout [UInt8], _ at: Int, _ w: Int, _ e: Endian, _ v: UInt64) {
        for i in 0..<w {
            let shift = e == .big ? 8 * (w - 1 - i) : 8 * i
            b[at + i] = UInt8(truncatingIfNeeded: v >> UInt64(shift))
        }
    }

    private static func arithAt(_ data: [UInt8], _ at: Int, _ w: Int, _ delta: UInt64, _ e: Endian) -> [UInt8] {
        var buf = data
        writeUint(&buf, at, w, e, readUint(buf, at, w, e) &+ delta) // mod 2^64, then truncated
        return buf
    }

    private static func boundaryAt(_ data: [UInt8], _ at: Int, _ w: Int, _ val: UInt64, _ e: Endian) -> [UInt8] {
        var buf = data
        writeUint(&buf, at, w, e, val)
        return buf
    }

    /// Adds the signed `delta` modulo 2^(8 * width) to the integer at the
    /// offset. `width` is in {1, 2, 4, 8}; any other width, or `len < width`,
    /// returns the input unchanged. The offset wraps over the `len - width + 1`
    /// positions where a whole integer fits.
    ///
    /// Worked example (u16, little-endian): `ff 00` is 0x00ff; adding 1 gives
    /// 0x0100, stored little-endian as `00 01`.
    public static func byteArithmetic(_ data: [UInt8], byteOffset: UInt64, delta: Int64, width: Int, endian: Endian) -> [UInt8] {
        guard validWidth(width), data.count >= width else { return data }
        return arithAt(data, modU(byteOffset, data.count - width + 1), width, UInt64(bitPattern: delta), endian)
    }
    public static func byteArithmeticBig(_ data: [UInt8], byteOffset: BigInt, delta: BigInt, width: Int, endian: Endian) -> [UInt8] {
        guard validWidth(width), data.count >= width else { return data }
        return arithAt(data, modBig(byteOffset, data.count - width + 1), width, low64(delta), endian)
    }

    /// Overwrites `width` bytes with `val mod 2^(8 * width)` at the offset. Same
    /// width and short-buffer rules as `byteArithmetic`. Worked example:
    /// `boundaryInsert(00 00 00 00, 0, 4, 0x80000000, little)` is `00 00 00 80`.
    public static func boundaryInsert(_ data: [UInt8], byteOffset: UInt64, width: Int, val: UInt64, endian: Endian) -> [UInt8] {
        guard validWidth(width), data.count >= width else { return data }
        return boundaryAt(data, modU(byteOffset, data.count - width + 1), width, val, endian)
    }
    public static func boundaryInsertBig(_ data: [UInt8], byteOffset: BigInt, width: Int, val: BigInt, endian: Endian) -> [UInt8] {
        guard validWidth(width), data.count >= width else { return data }
        return boundaryAt(data, modBig(byteOffset, data.count - width + 1), width, low64(val), endian)
    }

    // MARK: blocks and splice -------------------------------------------------------

    private static func deleteAt(_ data: [UInt8], _ at: Int, _ length: UInt64) -> [UInt8] {
        let cut = min(clampInt(length), data.count - at)
        var out = data
        out.removeSubrange(at..<(at + cut))
        return out
    }

    /// Deletes up to `length` bytes at `offset mod len`, clamped at the end.
    public static func blockDelete(_ data: [UInt8], offset: UInt64, length: UInt64) -> [UInt8] {
        guard !data.isEmpty, length != 0 else { return data }
        return deleteAt(data, modU(offset, data.count), length)
    }
    public static func blockDeleteBig(_ data: [UInt8], offset: BigInt, length: BigInt) -> [UInt8] {
        guard !data.isEmpty, length.signum > 0 else { return data }
        return deleteAt(data, modBig(offset, data.count), satU64(length))
    }

    private static func insertAt(_ data: [UInt8], _ at: Int, _ ins: ArraySlice<UInt8>) -> [UInt8] {
        var out = data
        out.insert(contentsOf: ins, at: at)
        return out
    }

    /// Inserts at `offset mod (len + 1)`; an empty input becomes the insert.
    public static func blockInsert(_ data: [UInt8], offset: UInt64, insert: [UInt8]) -> [UInt8] {
        if data.isEmpty { return insert }
        return insertAt(data, modU(offset, data.count + 1), insert[...])
    }
    public static func blockInsertBig(_ data: [UInt8], offset: BigInt, insert: [UInt8]) -> [UInt8] {
        if data.isEmpty { return insert }
        return insertAt(data, modBig(offset, data.count + 1), insert[...])
    }

    private static func duplicateAt(_ data: [UInt8], _ src: Int, _ length: UInt64, _ dst: Int) -> [UInt8] {
        let n = min(clampInt(length), data.count - src)
        return insertAt(data, dst, data[src..<(src + n)])
    }

    /// Copies up to `length` bytes from `srcOffset mod len` and inserts them at
    /// `dstOffset mod (len + 1)` (the end is a valid insertion point); the copy
    /// is clamped at the end of the source.
    public static func blockDuplicate(_ data: [UInt8], srcOffset: UInt64, length: UInt64, dstOffset: UInt64) -> [UInt8] {
        guard !data.isEmpty, length != 0 else { return data }
        return duplicateAt(data, modU(srcOffset, data.count), length, modU(dstOffset, data.count + 1))
    }
    public static func blockDuplicateBig(_ data: [UInt8], srcOffset: BigInt, length: BigInt, dstOffset: BigInt) -> [UInt8] {
        guard !data.isEmpty, length.signum > 0 else { return data }
        return duplicateAt(data, modBig(srcOffset, data.count), satU64(length), modBig(dstOffset, data.count + 1))
    }

    /// `target[:ts] + donor[ds:]` with both splits clamped into their buffers.
    public static func splice(_ target: [UInt8], _ donor: [UInt8], targetSplit: UInt64, donorSplit: UInt64) -> [UInt8] {
        let t = min(clampInt(targetSplit), target.count)
        let d = min(clampInt(donorSplit), donor.count)
        var out = [UInt8]()
        out.reserveCapacity(t + (donor.count - d))
        out.append(contentsOf: target[0..<t])
        out.append(contentsOf: donor[d...])
        return out
    }
    public static func spliceBig(_ target: [UInt8], _ donor: [UInt8], targetSplit: BigInt, donorSplit: BigInt) -> [UInt8] {
        splice(target, donor, targetSplit: satU64(targetSplit), donorSplit: satU64(donorSplit))
    }

    /// The length `splice` must produce, computed independently for the runner's invariant.
    static func spliceLength(targetLen: Int, donorLen: Int, targetSplit: UInt64, donorSplit: UInt64) -> Int {
        min(clampInt(targetSplit), targetLen) + donorLen - min(clampInt(donorSplit), donorLen)
    }

    // MARK: mutateSequence -------------------------------------------------------------

    /// A mutated buffer and the bytes of buffer copying the sequence did.
    struct Counted {
        let data: [UInt8]
        let work: Int
    }

    /// Applies `steps` seeded mutations, each chosen by one `bounded(7)` draw:
    ///
    ///     draw  step
    ///     0     bit flip, count 1
    ///     1     byte flip, count 1
    ///     2     byte arithmetic, width 1, delta from (-35 -8 -1 1 8 35)
    ///     3     boundary insert, width 1, value from (0 1 127 128 255)
    ///     4     block delete (length 1..len/2); skipped if len <= 1
    ///     5     block insert of one random byte
    ///     6     block duplicate (length 1..len/2); skipped if len <= 1
    ///
    /// An empty buffer is refilled to `00` before every step. The draw order is
    /// the cross-lane contract: the same (data, seed, steps) gives the same bytes
    /// in every language. Draws for a skipped step are still consumed.
    ///
    /// `maxLen` is VER06 section 3.5's "maximum length budget": a block insert or
    /// duplicate that would grow the buffer past it is skipped AFTER its draws
    /// are consumed, so the stream stays aligned and memory stays bounded.
    ///
    /// A negative `steps` runs none. `steps` above `maxIterations`, an input above
    /// `maxInputBytes` and a call whose copying would pass `maxSequenceWork`
    /// throw (the last as `FuzzError.workExhausted`).
    public static func mutateSequence(_ data: [UInt8], seed: UInt64, steps: Int, maxLen: Int) throws -> [UInt8] {
        try counted(data, rng: SplitMix64(seed: seed), steps: steps, maxLen: maxLen, workCap: maxSequenceWork).data
    }

    /// `mutateSequence` over a caller-built stream and an explicit work cap, also
    /// reporting the REAL work (the corpus runner charges it to its aggregate
    /// budget: block inserts and duplicates grow the buffer, so steps x initial
    /// length would undercount).
    static func counted(_ data: [UInt8], seed: UInt64, steps: Int, maxLen: Int, workCap: Int) throws -> Counted {
        try counted(data, rng: SplitMix64(seed: seed), steps: steps, maxLen: maxLen, workCap: workCap)
    }

    /// A non-throwing bounded draw: `bounded(0)` cannot happen, and maps to 0.
    private static func draw(_ rng: inout SplitMix64, _ n: UInt64) -> UInt64 {
        (try? rng.bounded(max(n, 1))) ?? 0
    }

    static func counted(_ data: [UInt8], rng rngIn: SplitMix64, steps: Int, maxLen: Int, workCap: Int) throws -> Counted {
        if steps > maxIterations { throw FuzzError.invalidArgument("steps exceeds maxIterations") }
        if data.count > maxInputBytes { throw FuzzError.invalidArgument("input exceeds maxInputBytes") }
        var rng = rngIn
        var cur = data
        var work = cur.count // the initial copy is work too
        if work > workCap { throw FuzzError.workExhausted(workDone: 0) }
        var i = 0
        while i < steps {
            i += 1
            if cur.isEmpty { cur = [0] }
            // Refuse BEFORE the step when even its lower bound passes the cap.
            if work > workCap - cur.count { throw FuzzError.workExhausted(workDone: work) }
            let n = cur.count
            switch draw(&rng, 7) {
            case 0:
                cur = flip(cur, bits: true, offset: draw(&rng, UInt64(n) * 8), count: 1)
            case 1:
                cur = flip(cur, bits: false, offset: draw(&rng, UInt64(n)), count: 1)
            case 2:
                let delta = deltas8[Int(draw(&rng, UInt64(deltas8.count)))]
                cur = arithAt(cur, Int(draw(&rng, UInt64(n))), 1, UInt64(bitPattern: delta), .little)
            case 3:
                let val = interest8[Int(draw(&rng, UInt64(interest8.count)))]
                cur = boundaryAt(cur, Int(draw(&rng, UInt64(n))), 1, val, .little)
            case 4:
                if n > 1 {
                    let length = 1 + draw(&rng, UInt64(max(1, n / 2)))
                    cur = deleteAt(cur, Int(draw(&rng, UInt64(n))), length)
                }
            case 5:
                let ins = UInt8(truncatingIfNeeded: draw(&rng, 256))
                let at = Int(draw(&rng, UInt64(n) + 1))
                if n < maxLen { cur = insertAt(cur, at, [ins][...]) }
            default: // 6
                if n > 1 {
                    let length = 1 + draw(&rng, UInt64(max(1, n / 2)))
                    let src = Int(draw(&rng, UInt64(n)))
                    let dst = Int(draw(&rng, UInt64(n) + 1))
                    let grow = min(clampInt(length), n - src)
                    if n + grow <= maxLen { cur = duplicateAt(cur, src, length, dst) }
                }
            }
            // Charge what the step really produced (at least 1).
            let (w, overflow) = work.addingReportingOverflow(max(cur.count, 1))
            if overflow || w > workCap { throw FuzzError.workExhausted(workDone: overflow ? Int.max : w) }
            work = w
        }
        return Counted(data: cur, work: work)
    }
}
