// ============================================================================
// Gens.swift -- the generator algebra (VER05 section 3)
// ============================================================================
//
// Primitive generators build typed values from random words; combinators build
// new generators from old ones. The ORDER in which a generator consumes words is
// the compatibility contract between lanes (the pbt-v1 corpus pins it), so each
// generator documents exactly what it draws.
//
// NAMING NOTE
// -----------
// The flat-map combinator is called `andThen`, not `bind`: inside closures a
// Swift `bind` can resolve to the POSIX socket `bind` (recorded in lessons.d as
// "swift-posix-bind-collides-with-sequence-bind").

import Foundation

public enum Gens {
    private static let maxI128 = BigInt.one.shiftedLeft(127) - .one
    private static let minI128 = -BigInt.one.shiftedLeft(127)
    private static let two127 = BigInt.one.shiftedLeft(127)
    private static let two128 = BigInt.one.shiftedLeft(128)
    private static let maxU64 = BigInt(UInt64.max)

    // MARK: primitives --------------------------------------------------------

    /// Draws nothing; always `v`. No shrinking.
    public static func constant<T>(_ v: T) -> Gen<T> { Gen(sampler: { _ in v }) }

    /// `bounded(2) == 1`; shrinks `true` to `false`.
    public static func bool() -> Gen<Bool> {
        Gen(sampler: { rng in try rng.bounded(2) == 1 },
            shrinker: { value, falsifies, maxSteps in
                value && maxSteps > 0 && falsifies(false)
                    ? Shrunk(value: false, steps: 1)
                    : Shrunk(value: value, steps: 0)
            })
    }

    /// Builds a shrinker from the integer core, converting to and from `BigInt`.
    /// The shrink target is the in-range point nearest 0.
    private static func bigShrinker<T>(
        lo: BigInt, hi: BigInt, toBig: @escaping (T) -> BigInt, fromBig: @escaping (BigInt) -> T
    ) -> Shrinker<T> {
        let target = clampZero(lo, hi)
        return { value, falsifies, maxSteps in
            let r = Shrink.shrinkIntCore(initial: toBig(value), target: target, lo: lo, hi: hi,
                                         falsifies: { falsifies(fromBig($0)) }, maxSteps: maxSteps)
            return Shrunk(value: fromBig(r.value), steps: r.steps)
        }
    }

    /// The point of [lo, hi] nearest zero (VER05 section 4.2's default target).
    static func clampZero(_ lo: BigInt, _ hi: BigInt) -> BigInt {
        if lo.signum > 0 { return lo }
        if hi.signum < 0 { return hi }
        return .zero
    }

    /// `bounded(256)`, typed `UInt8`.
    public static func u8() -> Gen<UInt8> {
        Gen(sampler: { rng in UInt8(try rng.bounded(256)) },
            shrinker: bigShrinker(lo: .zero, hi: BigInt(255), toBig: { BigInt(Int($0)) }, fromBig: { UInt8($0.asInt!) }))
    }

    /// `nextU64()`, typed `UInt64`.
    public static func u64() -> Gen<UInt64> {
        Gen(sampler: { rng in rng.nextU64() },
            shrinker: bigShrinker(lo: .zero, hi: maxU64, toBig: { BigInt($0) }, fromBig: { $0.asUInt64! }))
    }

    /// Two words `hi`, `lo`; the value is `(hi << 64) | lo`, read as a signed
    /// 128-bit number (subtract 2^128 from anything at or above 2^127).
    static func sampleI128(_ rng: inout SplitMix64) -> BigInt {
        let hi = BigInt(rng.nextU64())
        let lo = BigInt(rng.nextU64())
        var v = hi.shiftedLeft(64) + lo
        if v >= two127 { v = v - two128 }
        return v
    }

    public static func i128() -> Gen<BigInt> {
        Gen(sampler: { rng in sampleI128(&rng) },
            shrinker: bigShrinker(lo: minI128, hi: maxI128, toBig: { $0 }, fromBig: { $0 }))
    }

    /// `min + boundedBig(max - min + 1)` over [min, max] inclusive.
    public static func integer(_ min: BigInt, _ max: BigInt) throws -> Gen<BigInt> {
        if min > max { throw PbtError.invalidArgument("integer needs min <= max") }
        if min.bitWidth > BigInt.apiMaxBits || max.bitWidth > BigInt.apiMaxBits || (max - min + .one).bitWidth > BigInt.apiMaxBits {
            throw PbtError.invalidArgument("integer bounds or span wider than \(BigInt.apiMaxBits) bits")
        }
        return Gen(sampler: { rng in try boundedInt(&rng, min, max) },
                   shrinker: bigShrinker(lo: min, hi: max, toBig: { $0 }, fromBig: { $0 }))
    }

    static func boundedInt(_ rng: inout SplitMix64, _ lo: BigInt, _ hi: BigInt) throws -> BigInt {
        let span = hi - lo + .one
        return lo + (try rng.boundedBig(span))
    }

    /// The same draws as `integer`, typed `Int` (spans are computed in `BigInt`,
    /// so the full `Int` range cannot overflow).
    public static func int(_ min: Int, _ max: Int) throws -> Gen<Int> {
        if min > max { throw PbtError.invalidArgument("int needs min <= max") }
        let lo = BigInt(min), hi = BigInt(max)
        return Gen(sampler: { rng in try boundedInt(&rng, lo, hi).asInt! },
                   shrinker: bigShrinker(lo: lo, hi: hi, toBig: { BigInt($0) }, fromBig: { $0.asInt! }))
    }

    private static func checkLengths(_ minLen: Int, _ maxLen: Int, _ limit: Int, _ what: String) throws {
        if minLen < 0 || maxLen < minLen { throw PbtError.invalidArgument("\(what) needs 0 <= minLen <= maxLen") }
        if maxLen > limit { throw PbtError.invalidArgument("\(what) maxLen \(maxLen) exceeds \(limit)") }
    }

    /// `L = minLen + bounded(maxLen - minLen + 1)`, then `L` draws of `bounded(256)`.
    public static func bytes(_ minLen: Int, _ maxLen: Int) throws -> Gen<[UInt8]> {
        try checkLengths(minLen, maxLen, Limits.maxInputBytes, "bytes")
        let span = UInt64(maxLen - minLen) + 1
        return Gen(
            sampler: { rng in
                let n = minLen + Int(try rng.bounded(span))
                var out = [UInt8]()
                out.reserveCapacity(n)
                for _ in 0..<n { out.append(UInt8(try rng.bounded(256))) }
                return out
            },
            shrinker: { value, falsifies, maxSteps in
                // The list conversion (and the array one on the way out) are
                // copies too: charge them first, and do nothing once spent.
                let budget = ShrinkBudget.enter(maxSteps: maxSteps)
                defer { budget.close() }
                if maxSteps <= 0 || !budget.chargeCopy(2 * value.count) { return Shrunk(value: value, steps: 0) }
                return Shrink.shrinkSequence(value, falsifies: { budget.chargeCopy($0.count) && falsifies($0) },
                                             maxSteps: maxSteps, minLen: minLen, shrinkElem: Shrink.byteWalk)
            })
    }

    // MARK: combinators -------------------------------------------------------

    /// Maps the generated value. Drops shrinking (a map cannot be inverted).
    public static func map<T, U>(_ g: Gen<T>, _ f: @escaping (T) throws -> U) -> Gen<U> {
        Gen(sampler: { rng in try f(try g.sample(&rng)) })
    }

    /// Re-samples up to `maxRetries` times, then fails with `ExhaustedError`
    /// (reported as `property-exhausted`). Shrinking is preserved: a shrink
    /// candidate must also satisfy `pred`.
    public static func filter<T>(
        _ g: Gen<T>, maxRetries: Int = Limits.defaultMaxRetries, _ pred: @escaping (T) -> Bool
    ) throws -> Gen<T> {
        if maxRetries < 1 || maxRetries > Limits.maxFilterRetries {
            throw PbtError.invalidArgument("filter maxRetries \(maxRetries) outside [1, \(Limits.maxFilterRetries)]")
        }
        return Gen(
            sampler: { rng in
                // Every retry is charged whenever a budget exists, so stacked
                // filters cannot multiply uncharged retries. A nested check pays
                // an attempt up the chain; a top-level check root pays one
                // element of WORK (its attempt cap, about numTests + 16*maxSteps
                // + 64, would wrongly exhaust a legitimate rare filter).
                let budget = ShrinkBudget.current()
                let nestedScope = budget?.isNested ?? false
                for _ in 0..<maxRetries {
                    if let budget, !(nestedScope ? budget.charge(1) : budget.chargeCopy(1)) {
                        throw ExhaustedError("the enclosing shrink budget was spent while filtering")
                    }
                    let v = try g.sample(&rng)
                    if pred(v) { return v }
                }
                throw ExhaustedError("filter rejected \(maxRetries) samples")
            },
            shrinker: { value, falsifies, maxSteps in
                // Charge before the user predicate runs: it is user code and may be costly.
                let budget = ShrinkBudget.current()
                return g.shrinkNested(value, { c in (budget?.charge(1) ?? true) && pred(c) && falsifies(c) }, maxSteps)
            })
    }

    /// Flat-map: sample, then sample from `f(value)`. No shrinking.
    public static func andThen<T, U>(_ g: Gen<T>, _ f: @escaping (T) throws -> Gen<U>) -> Gen<U> {
        Gen(sampler: { rng in try f(try g.sample(&rng)).sample(&rng) })
    }

    /// Uniform choice among arms: `bounded(count)`, then that arm.
    public static func oneOf<T>(_ gens: [Gen<T>]) throws -> Gen<T> {
        if gens.isEmpty { throw PbtError.invalidArgument("oneOf needs at least one generator") }
        return Gen(sampler: { rng in try gens[Int(try rng.bounded(UInt64(gens.count)))].sample(&rng) })
    }

    public struct Arm<T> {
        public let weight: UInt64
        public let gen: Gen<T>
        public init(weight: UInt64, gen: Gen<T>) {
            self.weight = weight
            self.gen = gen
        }
    }

    /// Weighted choice: `bounded(total)`; arms in order; a zero-weight arm is
    /// never selected. Total weight is `1 ... 2^32`.
    public static func frequency<T>(_ arms: [Arm<T>]) throws -> Gen<T> {
        if arms.isEmpty { throw PbtError.invalidArgument("frequency needs at least one arm") }
        var total: UInt64 = 0
        for a in arms {
            if a.weight > Limits.maxFrequencyTotal { throw PbtError.invalidArgument("frequency weight outside [0, 2^32]") }
            total += a.weight   // at most count * 2^32: cannot overflow UInt64
            if total > Limits.maxFrequencyTotal { throw PbtError.invalidArgument("frequency total weight exceeds 2^32") }
        }
        if total == 0 { throw PbtError.invalidArgument("frequency total weight must be > 0") }
        let sum = total
        return Gen(sampler: { rng in
            var pick = try rng.bounded(sum)
            for a in arms {
                if pick < a.weight { return try a.gen.sample(&rng) }
                pick -= a.weight
            }
            throw PbtError.invalidArgument("unreachable: pick < total")
        })
    }

    /// A list: length `minLen + bounded(span)`, then elements in order. Shrinks by
    /// sequence shrinking, with each element shrunk by its own generator.
    public static func list<T>(_ elem: Gen<T>, _ minLen: Int, _ maxLen: Int) throws -> Gen<[T]> {
        try checkLengths(minLen, maxLen, Limits.maxListLen, "list")
        let span = UInt64(maxLen - minLen) + 1
        return Gen(
            sampler: { rng in
                let n = minLen + Int(try rng.bounded(span))
                var out = [T]()
                out.reserveCapacity(Swift.min(n, 1024))
                for _ in 0..<n { out.append(try elem.sample(&rng)) }
                return out
            },
            shrinker: { value, falsifies, maxSteps in
                Shrink.shrinkSequence(value, falsifies: falsifies, maxSteps: maxSteps, minLen: minLen,
                                      shrinkElem: { v, f, s in elem.shrinkNested(v, f, s) })
            })
    }

    // MARK: tuples ------------------------------------------------------------

    /// A counter shared by every pass of a tuple shrink (the global attempt cap).
    private final class Spent {
        private let lock = NSLock()
        private var n = 0
        func next() -> Int {
            lock.lock(); defer { lock.unlock() }
            n += 1
            return n - 1
        }
    }

    /// One budget of `16 * maxSteps + 64` evaluations shared by every pass and
    /// every component: otherwise each pass would grant each component a fresh
    /// budget (about `8 * maxSteps^2` evaluations in total).
    private static func globalBudget<V>(_ falsifies: @escaping (V) -> Bool, _ maxSteps: Int) -> (V) -> Bool {
        let budget = Limits.attemptBudget(maxSteps)
        let spent = Spent()
        return { v in spent.next() < budget && falsifies(v) }
    }

    public static func tuple2<A, B>(_ ga: Gen<A>, _ gb: Gen<B>) -> Gen<(A, B)> {
        Gen(
            sampler: { rng in
                let a = try ga.sample(&rng)
                let b = try gb.sample(&rng)
                return (a, b)
            },
            shrinker: { value, rawFalsifies, maxSteps in
                let falsifies = globalBudget(rawFalsifies, maxSteps)
                var first = value.0, second = value.1
                var steps = 0
                // Repeat passes until a whole pass makes no progress: shrinking
                // one coordinate can unlock more shrinking of the other.
                var progress = true
                while progress && steps < maxSteps {
                    progress = false
                    if maxSteps - steps > 0 {
                        let fixedB = second
                        let r = ga.shrinkNested(first, { falsifies(($0, fixedB)) }, maxSteps - steps)
                        let n = Shrink.clampAdd(r.steps, maxSteps - steps); if n > 0 { first = r.value; steps += n; progress = true }
                    }
                    if maxSteps - steps > 0 {
                        let fixedA = first
                        let r = gb.shrinkNested(second, { falsifies((fixedA, $0)) }, maxSteps - steps)
                        let n = Shrink.clampAdd(r.steps, maxSteps - steps); if n > 0 { second = r.value; steps += n; progress = true }
                    }
                }
                return Shrunk(value: (first, second), steps: steps)
            })
    }

    public static func tuple3<A, B, C>(_ ga: Gen<A>, _ gb: Gen<B>, _ gc: Gen<C>) -> Gen<(A, B, C)> {
        Gen(
            sampler: { rng in
                let a = try ga.sample(&rng)
                let b = try gb.sample(&rng)
                let c = try gc.sample(&rng)
                return (a, b, c)
            },
            shrinker: { value, rawFalsifies, maxSteps in
                let falsifies = globalBudget(rawFalsifies, maxSteps)
                var first = value.0, second = value.1, third = value.2
                var steps = 0
                var progress = true
                while progress && steps < maxSteps {
                    progress = false
                    if maxSteps - steps > 0 {
                        let fb = second, fc = third
                        let r = ga.shrinkNested(first, { falsifies(($0, fb, fc)) }, maxSteps - steps)
                        let n = Shrink.clampAdd(r.steps, maxSteps - steps); if n > 0 { first = r.value; steps += n; progress = true }
                    }
                    if maxSteps - steps > 0 {
                        let fa = first, fc = third
                        let r = gb.shrinkNested(second, { falsifies((fa, $0, fc)) }, maxSteps - steps)
                        let n = Shrink.clampAdd(r.steps, maxSteps - steps); if n > 0 { second = r.value; steps += n; progress = true }
                    }
                    if maxSteps - steps > 0 {
                        let fa = first, fb = second
                        let r = gc.shrinkNested(third, { falsifies((fa, fb, $0)) }, maxSteps - steps)
                        let n = Shrink.clampAdd(r.steps, maxSteps - steps); if n > 0 { third = r.value; steps += n; progress = true }
                    }
                }
                return Shrunk(value: (first, second, third), steps: steps)
            })
    }

    // MARK: corpus entry points (pbt-v1) --------------------------------------

    private static func checkCount(_ count: Int) throws {
        if count < 0 || count > Limits.maxGenCount {
            throw PbtError.invalidArgument("count \(count) outside [0, \(Limits.maxGenCount)]")
        }
    }

    /// A primitive output of the corpus (`bool`, `u8`, `u64`, `i128`).
    public enum Primitive: Equatable {
        case bool(Bool)
        case int(BigInt)
    }

    /// `count` draws of primitive `typeName` from `SplitMix64(seed)`.
    public static func genPrimitive(seed: UInt64, typeName: String, count: Int) throws -> [Primitive] {
        try checkCount(count)
        var rng = Hooks.newRng(seed)
        var out = [Primitive]()
        out.reserveCapacity(Swift.min(count, 1024))
        for _ in 0..<count {
            switch typeName {
            case "bool": out.append(.bool(try rng.bounded(2) == 1))
            case "u8": out.append(.int(BigInt(try rng.bounded(256))))
            case "u64": out.append(.int(BigInt(rng.nextU64())))
            case "i128": out.append(.int(sampleI128(&rng)))
            default: throw PbtError.invalidArgument("unknown primitive type \"\(typeName)\"")
            }
        }
        return out
    }

    /// `count` unbiased draws over [lo, hi], with the generation budget enforced
    /// here: each draw is an integer about as wide as the span, so `count x
    /// width` bounds what the returned list can hold.
    public static func genBounded(seed: UInt64, lo: BigInt, hi: BigInt, count: Int) throws -> [BigInt] {
        if lo > hi { throw PbtError.invalidArgument("need lo <= hi") }
        try Shrink.checkBits([lo, hi], "genBounded bound")
        try checkCount(count)
        let width = Swift.max(1, ((hi - lo).bitWidth + 7) / 8)
        if count * width > Limits.maxGenBytesTotal {
            throw PbtError.invalidArgument("count \(count) x \(width)-byte span exceeds the generation budget of \(Limits.maxGenBytesTotal) bytes")
        }
        var rng = Hooks.newRng(seed)
        var out = [BigInt]()
        out.reserveCapacity(Swift.min(count, 1024))
        for _ in 0..<count { out.append(try boundedInt(&rng, lo, hi)) }
        return out
    }

    /// `count` byte strings of length in [minLen, maxLen].
    public static func genBytes(seed: UInt64, minLen: Int, maxLen: Int, count: Int) throws -> [[UInt8]] {
        if minLen < 0 || maxLen < minLen || maxLen > Limits.maxInputBytes {
            throw PbtError.invalidArgument("need 0 <= minLen <= maxLen <= \(Limits.maxInputBytes)")
        }
        try checkCount(count)   // applies even when maxLen == 0: empty strings still cost a slot each
        if maxLen > 0 && count > Limits.maxGenBytesTotal / maxLen {
            throw PbtError.invalidArgument("count \(count) x maxLen \(maxLen) exceeds the total of \(Limits.maxGenBytesTotal) bytes")
        }
        let gen = try bytes(minLen, maxLen)
        var rng = Hooks.newRng(seed)
        var out = [[UInt8]]()
        out.reserveCapacity(Swift.min(count, 1024))
        for _ in 0..<count { out.append(try gen.sample(&rng)) }
        return out
    }
}
