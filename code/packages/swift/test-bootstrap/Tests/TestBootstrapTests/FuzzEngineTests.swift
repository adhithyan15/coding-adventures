import Foundation
import XCTest
@testable import TestBootstrap

/// The campaign contracts of VER06 section 9.1 and the hardening of VER06l sections 5 and 6.
final class FuzzEngineTests: XCTestCase {
    private final class Box<T> {
        var value: T
        init(_ v: T) { value = v }
    }

    private func opts(_ n: Int = 300, seed: UInt64 = 42, maxLen: Int = 4096, minimize: Int = 2000) -> FuzzOptions {
        FuzzOptions(maxIterations: n, seed: seed, maxInputLen: maxLen, minimizeBudget: minimize)
    }

    /// Three nested byte tests, each reporting an edge; the last crashes. Each stage
    /// is reachable by one 8-bit boundary/arithmetic mutation of the seed, but the
    /// three together in one lucky run are not.
    private func staircase(withEdges: Bool) -> Target {
        return { d, h in
            guard d.count >= 4, d[0] == 0x41 else { return }
            if withEdges { h.edge(1) }
            guard d[1] == 128 else { return }
            if withEdges { h.edge(2) }
            guard d[2] == 255 else { return }
            if withEdges { h.edge(3) }
            guard d[3] == 127 else { return }
            if withEdges { h.edge(4) }
            throw FuzzSupport.Boom()
        }
    }

    // MARK: determinism, budget, honest report -------------------------------------------------

    func testCampaignIsAPureFunctionOfCorpusOptionsAndTarget() throws {
        let a = try FuzzEngine.run(corpus: [[1, 2, 3], [9]], options: opts(), target: staircase(withEdges: true))
        let b = try FuzzEngine.run(corpus: [[1, 2, 3], [9]], options: opts(), target: staircase(withEdges: true))
        XCTAssertEqual(a.description, b.description)
        XCTAssertEqual(a.corpus(), b.corpus())
        XCTAssertEqual(a.crashes.map { $0.signature }, b.crashes.map { $0.signature })
        let c = try FuzzEngine.run(corpus: [[1, 2, 3], [9]], options: opts(seed: 43), target: staircase(withEdges: true))
        XCTAssertNotEqual(a.corpus(), c.corpus())
    }

    func testBudgetIterationsExactCandidatesBoundedEmptyCorpusReplaced() throws {
        let longest = Box(0)
        let seen = Box<[[UInt8]]>([])
        let r = try FuzzEngine.run(corpus: [], options: opts(500, maxLen: 6)) { d, h in
            longest.value = max(longest.value, d.count)
            seen.value.append(d)
            h.edge(UInt64(d.count))
        }
        XCTAssertEqual(r.iterations, 500)
        XCTAssertEqual(seen.value.first, [0], "an empty corpus is replaced by one 00 seed")
        XCTAssertLessThanOrEqual(longest.value, 6)
        // Initial seeds run as given, even past maxInputLen.
        let big = Box(0)
        _ = try FuzzEngine.run(corpus: [[UInt8](repeating: 7, count: 100)], options: opts(0, maxLen: 6)) { d, _ in big.value = d.count }
        XCTAssertEqual(big.value, 100)
    }

    func testSeedsThatCrashTimeOutOrExhaustMemoryAreNeverMutatedAndNeverClean() throws {
        let r = try FuzzEngine.run(corpus: [[1], [2]], options: opts(100)) { _, _ in throw FuzzSupport.Boom() }
        XCTAssertEqual(r.iterations, 0)
        XCTAssertFalse(r.isClean)
        XCTAssertEqual(r.crashes.count, 1)
        XCTAssertEqual(r.crashes[0].hits, 2)
        let t = try FuzzEngine.run(corpus: [[1]], options: FuzzOptions(maxIterations: 50, stepBudget: 0)) { _, h in try h.tick() }
        XCTAssertEqual(t.iterations, 0); XCTAssertEqual(t.timeouts, 1); XCTAssertFalse(t.isClean)
        let o = try FuzzEngine.run(corpus: [[1]], options: FuzzOptions(maxIterations: 50, memBudget: 0)) { _, h in try h.reserve(1) }
        XCTAssertEqual(o.iterations, 0); XCTAssertEqual(o.ooms, 1); XCTAssertFalse(o.isClean)
    }

    func testZeroIterationsIsNeverCleanAndACleanRunIsClean() throws {
        let zero = try FuzzEngine.run(corpus: [[1]], options: opts(0)) { _, _ in }
        XCTAssertFalse(zero.isClean)
        let clean = try FuzzEngine.run(corpus: [[1]], options: opts(20)) { _, _ in }
        XCTAssertTrue(clean.isClean)
        XCTAssertEqual(clean.coverageTier, FuzzReport.tierBlackBox)
        XCTAssertEqual(clean.admittedSeeds, 1)
    }

    func testEveryExecutionLandsSomewhere() throws {
        // A target that times out on long inputs and crashes on others still accounts for each run.
        let runs = Box(0)
        let r = try FuzzEngine.run(corpus: [[1]], options: FuzzOptions(maxIterations: 400, stepBudget: 3)) { d, h in
            runs.value += 1
            if d.count > 5 { for _ in 0..<10 { try h.tick() } }
            if d.contains(0xFF) { throw FuzzSupport.Boom() }
            h.edge(UInt64(d.count))
        }
        XCTAssertEqual(runs.value >= 401, true)
        XCTAssertGreaterThan(r.crashes.reduce(0) { $0 + $1.hits } + r.timeouts, 0)
    }

    // MARK: feedback -------------------------------------------------------------------------------------

    func testCoverageFeedbackCracksTheStaircaseThatBlindMutationCannot() throws {
        var guided = 0, blind = 0
        for seed in UInt64(1)...6 {
            let o = FuzzOptions(maxIterations: 6000, seed: seed, maxInputLen: 16)
            let g = try FuzzEngine.run(corpus: [[0x41, 0, 0, 0]], options: o, target: staircase(withEdges: true))
            let b = try FuzzEngine.run(corpus: [[0x41, 0, 0, 0]], options: o, target: staircase(withEdges: false))
            if !g.crashes.isEmpty { guided += 1 }
            if !b.crashes.isEmpty { blind += 1 }
            XCTAssertEqual(g.coverageTier, FuzzReport.tierReported)
            XCTAssertEqual(b.coverageTier, FuzzReport.tierBlackBox)
        }
        XCTAssertGreaterThanOrEqual(guided, 5, "guided \(guided) blind \(blind)")
        XCTAssertLessThanOrEqual(blind, 1, "guided \(guided) blind \(blind)")
    }

    // MARK: dedup and minimisation ---------------------------------------------------------------------------

    func testOneRecordPerSignatureWithHitsAndTheShortestInputKept() throws {
        let r = try FuzzEngine.run(corpus: [[5, 0x7F, 5, 5, 5], [0x7F], [0x7F, 1, 2]], options: opts(300)) { d, _ in
            if d.contains(0x7F) { throw FuzzSupport.Boom() }
        }
        XCTAssertEqual(r.crashes.count, 1)
        XCTAssertEqual(r.crashes[0].hits, 3)
        XCTAssertEqual(r.crashes[0].minimalInput(), [0x7F], "minimised to the one byte that matters")
        XCTAssertTrue(r.crashes[0].minimized)
        XCTAssertEqual(r.uniqueCrashes, 1)
    }

    func testDifferentKindsAndFramesAreDifferentCrashes() throws {
        let r = try FuzzEngine.run(corpus: [[1], [2], [3]], options: opts(0)) { d, h in
            switch d[0] {
            case 1: throw FuzzSupport.Boom()
            case 2: throw FuzzSupport.Other()
            default: try h.scope("x") { throw FuzzSupport.Boom() }
            }
        }
        XCTAssertEqual(r.crashes.count, 3)
        XCTAssertEqual(Set(r.crashes.map { $0.signature }).count, 3)
    }

    func testMinimiserReachesTheEmptyInputAndNeverTradesOneCrashForAnother() throws {
        let alwaysA: ([UInt8]) -> Outcome = { _ in .crash(kind: "a", frames: []) }
        let sigA = FuzzSignature.crashSignature(kind: "a", frames: [])
        XCTAssertEqual(try FuzzEngine.minimizeCrashingInput([9], sig: sigA, test: alwaysA, budget: 100), [])
        let flaky: ([UInt8]) -> Outcome = { $0.count == 4 ? .crash(kind: "a", frames: []) : .crash(kind: "b", frames: []) }
        XCTAssertEqual(try FuzzEngine.minimizeCrashingInput([1, 2, 3, 4], sig: sigA, test: flaky, budget: 100), [0, 0, 0, 0].map { _ in 0 }.isEmpty ? [] : [1, 2, 3, 4].map { _ in UInt8(0) })
        XCTAssertThrowsError(try FuzzEngine.minimizeCrashingInput([1], sig: sigA, test: alwaysA, budget: -1))
        XCTAssertThrowsError(try FuzzEngine.minimizeCrashingInput([1], sig: sigA, test: alwaysA, budget: FuzzOptions.maxMinimizeBudget + 1))
        XCTAssertThrowsError(try FuzzEngine.minimizeCrashingInput([UInt8](repeating: 0, count: Mutate.maxInputBytes + 1), sig: sigA, test: alwaysA, budget: 1))
        XCTAssertEqual(try FuzzEngine.minimizeCrashingInput([], sig: sigA, test: alwaysA, budget: 0), [])
    }

    func testMinimisationCountsRejectedCandidatesPerCrashAndCampaignWide() throws {
        // Every candidate of a different length is a DIFFERENT crash, so each is
        // rejected: rejected candidates must still spend the budget.
        let calls = Box(0)
        let exec: ExecFunc = { d, _ in
            calls.value += 1
            return .crash(kind: d.count == 64 ? "orig" : "other", frames: [])
        }
        let seed = [UInt8](repeating: 1, count: 64)
        let r = try FuzzEngine.runOutcomes(corpus: [seed], options: opts(0, minimize: 5), exec: exec)
        XCTAssertEqual(calls.value, 1 + 0, "maxIterations 0: the campaign-wide minimisation allowance is 0")
        XCTAssertFalse(r.crashes[0].minimized)
        calls.value = 0
        let per = try FuzzEngine.runOutcomes(corpus: [seed], options: opts(1000, minimize: 5), exec: exec)
        XCTAssertEqual(calls.value, 1 + 5, "per-crash cap of 5 counts rejected candidates")
        XCTAssertFalse(per.crashes[0].minimized)
        // Campaign-wide: two distinct crashing seeds, total allowance = maxIterations = 3.
        calls.value = 0
        let seed2 = [UInt8](repeating: 2, count: 32)
        let exec2: ExecFunc = { d, _ in
            calls.value += 1
            return .crash(kind: d.count == 64 ? "A" : (d.count == 32 ? "B" : "other"), frames: [])
        }
        let wide = try FuzzEngine.runOutcomes(corpus: [seed, seed2], options: opts(3, minimize: 100), exec: exec2)
        XCTAssertEqual(calls.value, 2 + 3, "the whole campaign minimises at most maxIterations times")
        XCTAssertEqual(wide.crashes.count, 2)
        XCTAssertFalse(wide.crashes[0].minimized); XCTAssertFalse(wide.crashes[1].minimized)
    }

    // MARK: caps ---------------------------------------------------------------------------------------------------

    func testRetainedBytesCapDropsAdmissionsAndCrashInputsWithCounts() throws {
        // Each pass is admitted (a fresh edge per length); the cap admits only a few.
        let r = try FuzzEngine.runWithCap(corpus: [[1, 2, 3, 4]], options: opts(200), target: { d, h in h.edge(UInt64(d.count) &* 977 &+ UInt64(d.first ?? 0)) }, retainedCap: 10)
        XCTAssertGreaterThan(r.admissionsDropped, 0)
        XCTAssertLessThanOrEqual(r.corpus().reduce(0) { $0 + $1.count }, 10)
        // A crash whose input will not fit keeps no input, and is not "minimized".
        let c = try FuzzEngine.runWithCap(corpus: [[1, 2, 3, 4, 5, 6, 7, 8]], options: opts(0, minimize: 0), target: { _, _ in throw FuzzSupport.Boom() }, retainedCap: 4)
        XCTAssertEqual(c.crashes.count, 1)
        XCTAssertTrue(c.crashes[0].inputDropped)
        XCTAssertFalse(c.crashes[0].minimized)
        XCTAssertEqual(c.crashes[0].minimalInput(), [])
        XCTAssertFalse(c.isClean)
        // Under the cap nothing is dropped.
        let ok = try FuzzEngine.run(corpus: [[1]], options: opts(50)) { d, h in h.edge(UInt64(d.count)) }
        XCTAssertEqual(ok.admissionsDropped, 0)
    }

    func testCrashSignatureCapCountsDroppedSignatures() throws {
        var seeds: [[UInt8]] = []
        for i in 0..<(FuzzEngine.maxCrashes + 50) { seeds.append([UInt8(i >> 8), UInt8(i & 255)]) }
        let r = try FuzzEngine.runOutcomes(corpus: seeds, options: opts(0, minimize: 0)) { d, _ in .crash(kind: "k\(Int(d[0]) << 8 | Int(d[1]))", frames: []) }
        XCTAssertEqual(r.crashes.count, FuzzEngine.maxCrashes)
        XCTAssertEqual(r.crashesDropped, 50)
        XCTAssertEqual(r.uniqueCrashes, FuzzEngine.maxCrashes + 50)
        // Re-hits of dropped signatures count once: crashesDropped is distinct signatures, not hits.
        let again = try FuzzEngine.runOutcomes(corpus: seeds + seeds + seeds, options: opts(0, minimize: 0)) { d, _ in .crash(kind: "k\(Int(d[0]) << 8 | Int(d[1]))", frames: []) }
        XCTAssertEqual(again.crashesDropped, 50)
        XCTAssertEqual(again.uniqueCrashes, FuzzEngine.maxCrashes + 50)
        XCTAssertFalse(r.isClean)
    }

    func testOptionsAndCorpusAreValidated() {
        let ok = [[UInt8(1)]]
        func rejects(_ o: FuzzOptions, _ c: [[UInt8]] = [[1]], line: UInt = #line) {
            XCTAssertThrowsError(try FuzzEngine.run(corpus: c, options: o) { _, _ in }, line: line)
        }
        rejects(FuzzOptions(maxIterations: -1)); rejects(FuzzOptions(maxIterations: Mutate.maxIterations + 1))
        rejects(FuzzOptions(maxInputLen: Mutate.maxInputBytes + 1)); rejects(FuzzOptions(baseEnergy: -1))
        rejects(FuzzOptions(minimizeBudget: FuzzOptions.maxMinimizeBudget + 1)); rejects(FuzzOptions(stepBudget: FuzzOptions.maxStepBudget + 1))
        rejects(FuzzOptions(memBudget: FuzzOptions.maxMemBudget + 1)); rejects(FuzzOptions(stepBudget: Int.min))
        rejects(FuzzOptions(), [[UInt8]](repeating: [1], count: FuzzEngine.maxInitialCorpus + 1))
        rejects(FuzzOptions(), [[UInt8](repeating: 0, count: Mutate.maxInputBytes + 1)])
        XCTAssertNoThrow(try FuzzEngine.run(corpus: ok, options: FuzzOptions(maxIterations: 0, maxInputLen: 0, baseEnergy: 0, minimizeBudget: 0, stepBudget: 0, memBudget: 0)) { _, _ in })
        XCTAssertNoThrow(try FuzzOptions().validate())
        XCTAssertEqual(FuzzOptions().maxIterations, 1000); XCTAssertEqual(FuzzOptions().seed, 42)
        XCTAssertEqual(FuzzOptions().maxInputLen, 4096); XCTAssertEqual(FuzzOptions().memBudget, 64 << 20)
    }

    // MARK: input isolation, crash kinds ---------------------------------------------------------------------------------

    func testTheTargetGetsAFreshPhysicalCopyOfTheInput() throws {
        let mine: [UInt8] = [1, 2, 3, 4]
        let theirs = Box<UnsafePointer<UInt8>?>(nil)
        let mineAddr = mine.withUnsafeBufferPointer { $0.baseAddress }
        _ = try FuzzEngine.run(corpus: [mine], options: opts(0)) { d, _ in
            d.withUnsafeBufferPointer { theirs.value = $0.baseAddress }
        }
        XCTAssertNotNil(theirs.value)
        XCTAssertNotEqual(theirs.value, mineAddr, "the target must not alias the caller's storage")
        // Writing through an unsafe pointer in the target cannot reach the report's corpus either.
        let r = try FuzzEngine.run(corpus: [[1, 2, 3]], options: opts(10)) { d, h in
            h.edge(UInt64(d.count))
            var c = d
            if !c.isEmpty { c.withUnsafeMutableBufferPointer { $0[0] = 99 } }
        }
        XCTAssertEqual(r.corpus().first, [1, 2, 3])
    }

    func testCrashKindIsTheTypeNameAndTheMessageIsNeverRead() throws {
        let touched = Box(false)
        struct Hostile: Error, CustomStringConvertible, LocalizedError, CustomDebugStringConvertible {
            let flag: Box<Bool>
            var description: String { flag.value = true; return "SECRET-INPUT-BYTES" }
            var debugDescription: String { flag.value = true; return "SECRET" }
            var errorDescription: String? { flag.value = true; return "SECRET" }
            var localizedDescription: String { flag.value = true; return "SECRET" }
        }
        let r = try FuzzEngine.run(corpus: [[1]], options: opts(0)) { _, _ in throw Hostile(flag: touched) }
        XCTAssertFalse(touched.value, "the error's description must never be called")
        XCTAssertEqual(r.crashes.count, 1)
        let kind = r.crashes[0].kind
        XCTAssertTrue(kind.contains("Hostile"), kind)
        XCTAssertFalse(kind.contains("SECRET"))
        XCTAssertFalse(kind.contains(":")); XCTAssertLessThanOrEqual(kind.utf8.count, 64)
        // A top-level error type has a stable, readable kind.
        let b = try FuzzEngine.run(corpus: [[1]], options: opts(0)) { _, _ in throw FuzzSupport.Boom() }
        XCTAssertTrue(b.crashes[0].kind.hasSuffix("Boom"), b.crashes[0].kind)
        // An enum error and an NSError are crashes with their own kinds.
        let n = try FuzzEngine.run(corpus: [[1]], options: opts(0)) { _, _ in throw NSError(domain: "SECRET-DOMAIN", code: 1) }
        XCTAssertFalse(n.crashes[0].kind.contains("SECRET"))
        let e = try FuzzEngine.run(corpus: [[1]], options: opts(0)) { _, _ in throw FuzzError.invalidArgument("SECRET") }
        XCTAssertTrue(e.crashes[0].kind.contains("FuzzError"))
    }

    func testLocalErrorTypesHaveARunStableKind() throws {
        func make() -> String {
            struct LocalBoom: Error {}
            let r = try! FuzzEngine.run(corpus: [[1]], options: opts(0)) { _, _ in throw LocalBoom() }
            return r.crashes[0].kind
        }
        let a = make(), b = make()
        XCTAssertEqual(a, b)
        XCTAssertFalse(a.contains("$"), "an address leaked into the kind: \(a)")
    }

    // MARK: privacy ----------------------------------------------------------------------------------------------------------------

    func testNoRenderingOfAnOptionReportOrRecordShowsSeedsOrInputBytes() throws {
        let seed: UInt64 = 777_000_111
        let o = FuzzOptions(maxIterations: 30, seed: seed)
        let r = try FuzzEngine.run(corpus: [[0xDE, 0xAD, 0xBE, 0xEF]], options: o) { d, h in
            h.edge(UInt64(d.count))
            if d.count >= 4 && d[0] == 0xDE { throw FuzzSupport.Boom() }
        }
        XCTAssertFalse(r.crashes.isEmpty)
        let renderings: [String] = [
            o.description, o.debugDescription, String(describing: o), String(reflecting: o), Canon.render(o), dumped(o),
            r.description, r.debugDescription, String(describing: r), String(reflecting: r), Canon.render(r), dumped(r),
            r.crashes[0].description, String(reflecting: r.crashes[0]), Canon.render(r.crashes[0]), dumped(r.crashes[0]),
            "\(r.crashes)", String(describing: r.crashes),
        ] + FuzzSignature.tapFailures(r, target: "t", firstTest: 1)
        for s in renderings {
            XCTAssertFalse(s.contains("777000111"), s)
            XCTAssertFalse(s.lowercased().contains("deadbeef"), s)
            XCTAssertFalse(s.contains("222, 173"), s)
            XCTAssertFalse(s.contains("[222"), s)
        }
        XCTAssertEqual(r.crashes[0].minimalInput().first, 0xDE, "the input is reachable only through the explicit accessor")
    }

    private func dumped(_ v: Any) -> String {
        var s = ""
        dump(v, to: &s)
        return s
    }
}
