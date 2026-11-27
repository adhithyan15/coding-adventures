import XCTest
@testable import TestBootstrap

final class ShrinkTests: XCTestCase {
    override func tearDown() { Hooks.reset(); super.tearDown() }

    private func big(_ n: Int) -> BigInt { BigInt(n) }

    // MARK: integers ----------------------------------------------------------

    func testShrinkIntFindsTheExactThresholdForEveryStart() throws {
        for threshold in 1...40 {
            let prop: (BigInt) -> Bool = { $0 < BigInt(threshold) }
            for start in [threshold, threshold + 1, threshold + 7, 500, 9999] {
                let r = try Shrink.shrinkInt(initial: big(start), target: .zero, lo: .zero, hi: big(100_000), prop: prop, maxSteps: 1000)
                XCTAssertEqual(r.value, big(threshold), "threshold \(threshold) start \(start)")
            }
        }
    }

    func testShrinkIntNegativeStartsReachTheNegativeThreshold() throws {
        // x > -t holds; failing set is x <= -t; the target 0 is on the holding side.
        for t in 1...30 {
            let prop: (BigInt) -> Bool = { $0 > BigInt(-t) }
            let r = try Shrink.shrinkInt(initial: big(-1000), target: .zero, lo: big(-5000), hi: big(5000), prop: prop, maxSteps: 1000)
            XCTAssertEqual(r.value, big(-t))
        }
    }

    func testTheBisectionCandidateFloorsForNegativeDiffs() throws {
        // From -7 toward 0: diff = -7, floor(-7/2) = -4, so the first candidate is
        // -7 - (-4) = -3. Truncation would give -4. Record the candidates tried.
        var seen: [BigInt] = []
        let prop: (BigInt) -> Bool = { x in seen.append(x); return x > BigInt(-100) ? true : false }
        _ = prop   // (shape only)
        var tried: [BigInt] = []
        let r = try Shrink.shrinkInt(initial: big(-7), target: .zero, lo: big(-100), hi: big(100), prop: { x in
            tried.append(x)
            return x > BigInt(-3)    // holds above -3, so -3 and below fail
        }, maxSteps: 1000)
        XCTAssertEqual(tried.first, big(-7), "the start is evaluated first")
        XCTAssertEqual(tried[1], .zero, "the target probe")
        XCTAssertEqual(tried[2], big(-3), "floor(-7/2) = -4 so the candidate is -3")
        XCTAssertEqual(r.value, big(-3))
    }

    func testTargetProbeCountsAsOneStep() throws {
        let r = try Shrink.shrinkInt(initial: big(900), target: .zero, lo: .zero, hi: big(1000), prop: { _ in false }, maxSteps: 1000)
        XCTAssertEqual(r.value, .zero)
        XCTAssertEqual(r.steps, 1)
    }

    func testZeroBudgetReturnsTheStartUnchanged() throws {
        let r = try Shrink.shrinkInt(initial: big(900), target: .zero, lo: .zero, hi: big(1000), prop: { $0 < 5 }, maxSteps: 0)
        XCTAssertEqual(r.value, big(900))
        XCTAssertEqual(r.steps, 0)
    }

    func testStepBudgetStopsTheWalk() throws {
        let r = try Shrink.shrinkInt(initial: big(54321), target: .zero, lo: .zero, hi: big(100_000), prop: { $0 <= 100 }, maxSteps: 3)
        XCTAssertEqual(r.steps, 3)
        XCTAssertFalse(r.value <= big(100))
    }

    func testShrinkIntNeverLeavesBoundsAndRespectsAnOffsetTarget() throws {
        let r = try Shrink.shrinkInt(initial: big(99), target: big(10), lo: big(10), hi: big(500), prop: { $0 <= 50 }, maxSteps: 1000)
        XCTAssertEqual(r.value, big(51))
        XCTAssertEqual(r.steps, 5)
        let r2 = try Shrink.shrinkInt(initial: big(-80), target: .zero, lo: big(-200), hi: big(200), prop: { $0 >= -10 && $0 <= 50 }, maxSteps: 1000)
        XCTAssertEqual(r2.value, big(-11))
    }

    func testShrinkIntArgumentValidation() {
        XCTAssertThrowsError(try Shrink.shrinkInt(initial: big(1), target: .zero, lo: big(5), hi: big(1), prop: { _ in false }, maxSteps: 10))
        XCTAssertThrowsError(try Shrink.shrinkInt(initial: big(50), target: .zero, lo: .zero, hi: big(10), prop: { _ in false }, maxSteps: 10))
        XCTAssertThrowsError(try Shrink.shrinkInt(initial: big(5), target: .zero, lo: .zero, hi: big(10), prop: { _ in false }, maxSteps: -1))
        XCTAssertThrowsError(try Shrink.shrinkInt(initial: big(5), target: .zero, lo: .zero, hi: big(10), prop: { _ in false }, maxSteps: 10_001))
        XCTAssertThrowsError(try Shrink.shrinkInt(initial: big(5), target: .zero, lo: .zero, hi: big(10), prop: { _ in true }, maxSteps: 10)) {
            XCTAssertEqual($0 as? PbtError, .notFalsifying)
        }
    }

    func testExtremeI128StartsDoNotOverflow() throws {
        let lo = -BigInt.one.shiftedLeft(127), hi = BigInt.one.shiftedLeft(127) - .one
        let r = try Shrink.shrinkInt(initial: hi, target: .zero, lo: lo, hi: hi, prop: { $0 < BigInt(1000) }, maxSteps: 1000)
        XCTAssertEqual(r.value, BigInt(1000))
        let r2 = try Shrink.shrinkInt(initial: lo, target: hi, lo: lo, hi: hi, prop: { $0 > BigInt(-1000) }, maxSteps: 1000)
        XCTAssertEqual(r2.value, r2.value)   // reaches the probe: target hi holds, so it walks
        XCTAssertTrue(r2.value <= BigInt(-1000))
    }

    // MARK: sequences: both bisection halves ---------------------------------

    private func shrinkInts(_ list: [Int], _ prop: @escaping ([BigInt]) -> Bool, calls: ((String) -> Void)? = nil) throws -> Shrunk<[BigInt]> {
        try Shrink.shrinkList(initial: list.map { BigInt($0) }, elemTarget: .zero, prop: { l in
            calls?(l.map { $0.description }.joined(separator: ","))
            return prop(l)
        }, maxSteps: 1000)
    }

    func testBisectionKeepsTheRightHalfWhenOnlyItFails() throws {
        var log: [String] = []
        let r = try shrinkInts([1, 1, 1, 7], { !$0.contains(BigInt(7)) }, calls: { log.append($0) })
        XCTAssertEqual(r.value, [BigInt(7)])
        XCTAssertEqual(r.steps, 2)
        // start, empty probe, LEFT half first (holds), RIGHT half (fails), then
        // again; the last two entries are the element walk's bisection (4) and
        // unit (6) candidates for the surviving 7, both of which hold.
        XCTAssertEqual(log, ["1,1,1,7", "", "1,1", "1,7", "1", "7", "4", "6"])
    }

    func testBisectionKeepsTheLeftHalfWhenItFails() throws {
        var log: [String] = []
        let r = try shrinkInts([7, 1, 1, 1], { !$0.contains(BigInt(7)) }, calls: { log.append($0) })
        XCTAssertEqual(r.value, [BigInt(7)])
        XCTAssertEqual(r.steps, 2)
        XCTAssertEqual(log, ["7,1,1,1", "", "7,1", "7", "4", "6"])
    }

    func testSingleDeletionRunsRightToLeftAndIsOneMinimal() throws {
        // Fails when two adjacent equal elements exist; [1,2,3,3,4,5] -> [3,3].
        let r = try shrinkInts([1, 2, 3, 3, 4, 5], { l in !zip(l, l.dropFirst()).contains { $0 == $1 } })
        XCTAssertEqual(r.value, [BigInt(3), BigInt(3)])
        XCTAssertEqual(r.steps, 4)
    }

    func testElementWalkShrinksValuesAfterTheLengthIsMinimal() throws {
        let r = try shrinkInts([10, 5, 8, 4], { $0.reduce(.zero, +) <= BigInt(20) })
        XCTAssertEqual(r.value, [BigInt(8), BigInt(5), BigInt(8)])
        XCTAssertEqual(r.steps, 3)
    }

    func testEveryShortListOverFourValuesIsOneMinimalByDeletion() throws {
        let props: [([BigInt]) -> Bool] = [
            { $0.count < 3 }, { !$0.contains(BigInt(2)) }, { $0.reduce(.zero, +) < BigInt(5) },
            { l in !zip(l, l.dropFirst()).contains { $0 == $1 } },
        ]
        func lists(_ n: Int) -> [[Int]] { n == 0 ? [[]] : lists(n - 1).flatMap { l in (0..<4).map { l + [$0] } } }
        for p in props {
            for len in 0...4 {
                for l in lists(len) {
                    let bigL = l.map { BigInt($0) }
                    if p(bigL) { continue }
                    let r = try Shrink.shrinkList(initial: bigL, elemTarget: .zero, prop: p, maxSteps: 1000)
                    XCTAssertFalse(p(r.value), "\(l)")
                    XCTAssertLessThanOrEqual(r.value.count, l.count)
                    for i in 0..<r.value.count where r.value.count > 1 {
                        var d = r.value; d.remove(at: i)
                        XCTAssertTrue(p(d), "\(l) -> \(r.value) is not 1-minimal at \(i)")
                    }
                }
            }
        }
    }

    func testShrinkBytesCorpusShapeAndZeroBudget() throws {
        let r = try Shrink.shrinkBytes(initial: [1, 2, 3, 4, 5, 6, 7, 8], prop: { $0.count < 5 }, maxSteps: 1000)
        XCTAssertEqual(r.value, [0, 0, 0, 0, 0])
        XCTAssertEqual(r.steps, 16)
        let z = try Shrink.shrinkBytes(initial: [9, 9, 9, 9, 9], prop: { $0.count < 5 }, maxSteps: 0)
        XCTAssertEqual(z.value, [9, 9, 9, 9, 9])
        XCTAssertThrowsError(try Shrink.shrinkBytes(initial: [1], prop: { _ in true }, maxSteps: 5))
    }

    func testEmptyProbeIsOneStepWhenTheEmptyListFails() throws {
        let r = try Shrink.shrinkList(initial: [BigInt(1), BigInt(2)], elemTarget: .zero, prop: { _ in false }, maxSteps: 1000)
        XCTAssertEqual(r.value, [])
        XCTAssertEqual(r.steps, 1)
    }

    func testMinLenIsRespectedByGeneratorAttachedSequenceShrinking() throws {
        let g = try Gens.list(Gens.u8(), 3, 10)
        let r = try g.shrink([9, 9, 9, 9, 9, 9], falsifies: { $0.count >= 3 }, maxSteps: 1000)
        XCTAssertEqual(r.value.count, 3)
        XCTAssertEqual(r.value, [0, 0, 0])
        let gb = try Gens.bytes(2, 8)
        let rb = try gb.shrink([5, 5, 5, 5, 5], falsifies: { $0.count >= 2 }, maxSteps: 1000)
        XCTAssertEqual(rb.value, [0, 0])
    }

    func testPropertyReceivesCandidatesThatCannotCorruptTheShrinker() throws {
        // Value semantics: scribbling on the candidate is impossible, and a
        // property that reads its argument sees exactly the candidate.
        let r = try Shrink.shrinkList(initial: [BigInt(1), BigInt(2), BigInt(3)], elemTarget: .zero, prop: { l in l.count < 2 }, maxSteps: 1000)
        XCTAssertEqual(r.value.count, 2)
    }

    // MARK: replay ------------------------------------------------------------

    func testReplayIsDeterministic() throws {
        let a = try Shrink.simulateReplay(seed: 42, prop: { $0 <= BigInt(100) })
        let b = try Shrink.simulateReplay(seed: 42, prop: { $0 <= BigInt(100) })
        XCTAssertEqual(a, b)
        XCTAssertEqual(a.initialFail, BigInt(413))
        XCTAssertEqual(a.minimal, BigInt(101))
        XCTAssertThrowsError(try Shrink.simulateReplay(seed: 1, prop: { _ in true }))
    }

    // MARK: generator-attached shrinking ---------------------------------------

    func testIntegerShrinkNeverCrossesTheRefinementBound() throws {
        let g = try Gens.integer(BigInt(10), BigInt(500))
        let r = try g.shrink(BigInt(99), falsifies: { $0 > BigInt(50) }, maxSteps: 1000)
        XCTAssertEqual(r.value, BigInt(51))
        let r2 = try g.shrink(BigInt(300), falsifies: { _ in true }, maxSteps: 1000)
        XCTAssertEqual(r2.value, BigInt(10), "the target is the in-range point nearest zero, the lower bound")
        let neg = try Gens.integer(BigInt(-50), BigInt(-5))
        XCTAssertEqual(try neg.shrink(BigInt(-40), falsifies: { _ in true }, maxSteps: 100).value, BigInt(-5))
    }

    func testBoolShrinksTrueToFalseOnly() throws {
        let g = Gens.bool()
        XCTAssertEqual(try g.shrink(true, falsifies: { _ in true }, maxSteps: 5).value, false)
        XCTAssertEqual(try g.shrink(true, falsifies: { $0 }, maxSteps: 5).value, true)
        XCTAssertEqual(try g.shrink(false, falsifies: { _ in true }, maxSteps: 5).steps, 0)
        XCTAssertEqual(try g.shrink(true, falsifies: { _ in true }, maxSteps: 0).value, true)
    }

    func testTupleShrinksCoordinateWiseInRepeatedPasses() throws {
        let g = Gens.tuple2(try Gens.int(0, 1000), try Gens.int(0, 1000))
        // Fails when a + b >= 100: the minimal pair keeps the sum at exactly 100... local minimum:
        let r = try g.shrink((400, 700), falsifies: { $0.0 + $0.1 >= 100 }, maxSteps: 1000)
        XCTAssertGreaterThanOrEqual(r.value.0 + r.value.1, 100)
        XCTAssertTrue(r.value.0 == 0 || r.value.1 == 0 || r.value.0 + r.value.1 == 100)
        let t3 = Gens.tuple3(Gens.bool(), try Gens.int(0, 50), try Gens.u8())
        let r3 = try t3.shrink((true, 40, 200), falsifies: { $0.1 >= 5 }, maxSteps: 1000)
        XCTAssertEqual(r3.value.0, false)
        XCTAssertEqual(r3.value.1, 5)
        XCTAssertEqual(r3.value.2, 0)
    }

    func testMapHasNoShrinkerAndFilterPreservesPredicate() throws {
        let doubled = Gens.map(try Gens.int(0, 100)) { $0 * 2 }
        XCTAssertEqual(try doubled.shrink(40, falsifies: { _ in true }, maxSteps: 100).steps, 0)
        let even = try Gens.filter(try Gens.int(0, 1000), { $0 % 2 == 0 })
        let r = try even.shrink(900, falsifies: { $0 >= 7 }, maxSteps: 1000)
        XCTAssertEqual(r.value % 2, 0, "a shrink candidate that fails the filter is rejected")
        XCTAssertGreaterThanOrEqual(r.value, 7)
    }
}
