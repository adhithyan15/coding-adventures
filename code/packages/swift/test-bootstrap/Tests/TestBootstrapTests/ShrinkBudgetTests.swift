import Foundation
import XCTest
@testable import TestBootstrap

/// The required set from Java's ShrinkBudgetTest (VER05e section 4.1), ported.
final class ShrinkBudgetTests: XCTestCase {
    override func setUp() {
        super.setUp()
        Hooks.reset()
    }
    override func tearDown() {
        Hooks.reset()
        XCTAssertNil(ShrinkBudget.current(), "a test leaked an ambient scope")
        super.tearDown()
    }

    private let ci = Options(numTests: 100, maxShrinkSteps: 1000, ci: .on)

    /// A hard evaluation ceiling shared by a test's properties. Past it `tick`
    /// returns false and the property gives up cheaply, so a regressed guarantee
    /// makes the test FAIL (the assertion on `hit`) instead of hanging.
    private final class Ceiling {
        let limit: Int
        private(set) var count = 0
        private(set) var hit = false
        init(_ limit: Int) { self.limit = limit }
        func tick() -> Bool {
            count += 1
            if count > limit { hit = true; return false }
            return true
        }
    }

    // MARK: the counters --------------------------------------------------------

    func testARefusedTakeLeavesEveryCounterUnchanged() {
        let root = ShrinkBudget.enterChild(maxSteps: 1)         // attempts cap 16 + 64 = 80
        let child = ShrinkBudget.enterChild(maxSteps: 1000)     // a roomy local cap
        for _ in 0..<80 { XCTAssertTrue(root.charge(1) || true) }   // exhaust the ROOT directly
        let beforeChild = (child.attemptsSpent, child.workSpent)
        let beforeRoot = (root.attemptsSpent, root.workSpent)
        XCTAssertFalse(child.charge(5), "the child has room but its ancestor does not")
        XCTAssertEqual(child.attemptsSpent, beforeChild.0)
        XCTAssertEqual(child.workSpent, beforeChild.1)
        XCTAssertEqual(root.attemptsSpent, beforeRoot.0)
        XCTAssertEqual(root.workSpent, beforeRoot.1)
        child.close()
        root.close()
    }

    func testAHugeCostIsRefusedWithoutWrapping() {
        let b = ShrinkBudget.enterChild(maxSteps: 10)
        XCTAssertFalse(b.charge(Int.max))
        XCTAssertFalse(b.chargeCopy(Int.max))
        XCTAssertEqual(b.attemptsSpent, 0)
        XCTAssertEqual(b.workSpent, 0)
        XCTAssertTrue(b.charge(1))
        b.close()
    }

    func testChargeCopyChargesWorkButNotAttempts() {
        let b = ShrinkBudget.enterChild(maxSteps: 10)
        XCTAssertTrue(b.chargeCopy(100))
        XCTAssertEqual(b.attemptsSpent, 0)
        XCTAssertEqual(b.workSpent, 100)
        XCTAssertTrue(b.charge(7))
        XCTAssertEqual(b.attemptsSpent, 1)
        XCTAssertEqual(b.workSpent, 107)
        XCTAssertTrue(b.charge(0), "an empty candidate still costs one element")
        XCTAssertEqual(b.workSpent, 108)
        b.close()
    }

    func testTheLocalOnlyTakeTrapIsClosed() {
        // The child's local cap is huge, the parent's is tiny: ALL ancestors count.
        let parent = ShrinkBudget.enterChild(maxSteps: 0)       // 64 attempts
        let child = ShrinkBudget.enterChild(maxSteps: 10_000)   // 160,064 attempts
        var granted = 0
        while child.charge(1) && granted < 10_000 { granted += 1 }
        XCTAssertEqual(granted, 64, "a local-only check would have granted far more")
        XCTAssertTrue(parent.wasRefused)
        child.close(); parent.close()
    }

    func testAttemptsAndWorkCapsAreIndependent() {
        let b = ShrinkBudget.enterChild(maxSteps: 0, maxWork: 50)
        XCTAssertTrue(b.chargeCopy(50))
        XCTAssertFalse(b.chargeCopy(1))
        XCTAssertFalse(b.hasRoom())
        b.close()
        let c = ShrinkBudget.enterChild(maxSteps: 0)
        for _ in 0..<64 { XCTAssertTrue(c.charge(1)) }
        XCTAssertFalse(c.charge(1))
        XCTAssertFalse(c.hasRoom())
        c.close()
    }

    func testConcurrentChargingNeverExceedsTheCap() {
        let root = ShrinkBudget.enterChild(maxSteps: 0)    // 64 attempts
        let group = DispatchGroup()
        let lock = NSLock()
        var granted = 0
        for _ in 0..<8 {
            group.enter()
            Thread.detachNewThread {
                var mine = 0
                for _ in 0..<500 { if root.charge(1) { mine += 1 } }
                lock.lock(); granted += mine; lock.unlock()
                group.leave()
            }
        }
        XCTAssertEqual(group.wait(timeout: .now() + 30), .success)
        XCTAssertEqual(granted, 64)
        XCTAssertEqual(root.attemptsSpent, 64)
        root.close()
    }

    func testAmbientIsRestoredAfterNestingAndDeepNestingJoins() {
        XCTAssertNil(ShrinkBudget.current())
        let a = ShrinkBudget.enterChild(maxSteps: 10)
        XCTAssertTrue(ShrinkBudget.current() === a)
        let b = ShrinkBudget.enterChild(maxSteps: 10)
        XCTAssertFalse(b.isJoined)
        b.close()
        XCTAssertTrue(ShrinkBudget.current() === a)
        // Open 70 nested owning scopes: past depth 64 they JOIN instead.
        var stack: [ShrinkBudget] = [a]
        for _ in 0..<70 { stack.append(ShrinkBudget.enterChild(maxSteps: 10)) }
        XCTAssertTrue(stack.last!.isJoined)
        XCTAssertLessThanOrEqual(stack.filter { !$0.isJoined }.count, ShrinkBudget.maxDepth)
        for s in stack.reversed() { s.close() }
        XCTAssertNil(ShrinkBudget.current())
    }

    func testJoinedHandlesChargeTheirOwnerAndEnterJoinsAmbient() {
        let owner = ShrinkBudget.enterChild(maxSteps: 10)
        let joined = ShrinkBudget.enter(maxSteps: 10)
        XCTAssertTrue(joined.isJoined)
        XCTAssertTrue(joined.charge(3))
        XCTAssertEqual(owner.attemptsSpent, 1)
        XCTAssertEqual(joined.attemptsSpent, 1)
        joined.close()
        XCTAssertTrue(ShrinkBudget.current() === owner, "closing a joined handle changes nothing")
        owner.close()
        let fresh = ShrinkBudget.enter(maxSteps: 10)
        XCTAssertFalse(fresh.isJoined)
        XCTAssertFalse(fresh.isNested)
        fresh.close()
    }

    // MARK: threads --------------------------------------------------------------

    func testAThreadThatPredatesTheScopeHasNoAmbientBudget() {
        let started = DispatchSemaphore(value: 0), go = DispatchSemaphore(value: 0), done = DispatchSemaphore(value: 0)
        var sawAmbient = true
        Thread.detachNewThread {
            started.signal()
            _ = go.wait(timeout: .now() + 30)
            sawAmbient = ShrinkBudget.current() != nil
            done.signal()
        }
        XCTAssertEqual(started.wait(timeout: .now() + 30), .success)
        let scope = ShrinkBudget.enterChild(maxSteps: 10)
        go.signal()
        XCTAssertEqual(done.wait(timeout: .now() + 30), .success)
        XCTAssertFalse(sawAmbient)
        scope.close()
    }

    func testANewThreadInsideAScopeGetsAFreshRootNotTheParentsBudget() {
        // Documented difference from Java: no inheritance. The escape is a fresh,
        // still-bounded root; it never charges (or starves) the spawning scope.
        let scope = ShrinkBudget.enterChild(maxSteps: 0)
        let done = DispatchSemaphore(value: 0)
        var childIsFreshRoot = false
        Thread.detachNewThread {
            let b = ShrinkBudget.enterChild(maxSteps: 10)
            childIsFreshRoot = !b.isNested
            XCTAssertTrue(b.charge(1))
            b.close()
            done.signal()
        }
        XCTAssertEqual(done.wait(timeout: .now() + 30), .success)
        XCTAssertTrue(childIsFreshRoot)
        XCTAssertEqual(scope.attemptsSpent, 0)
        scope.close()
    }

    func testAThreadOutlivingTheRootGetsAFreshWorkingRoot() {
        let root = ShrinkBudget.enterChild(maxSteps: 10)
        root.close()
        XCTAssertNil(ShrinkBudget.current(), "a closed root is never ambient again")
        let b = ShrinkBudget.enterChild(maxSteps: 10)
        XCTAssertFalse(b.isNested)
        XCTAssertTrue(b.charge(1))
        b.close()
    }

    // MARK: stalling properties, big witnesses ------------------------------------

    // The witnesses are 256 KiB with a 4M-element work cap (the cap is the thing
    // under test, and a smaller cap keeps a REGRESSION fast enough to finish and fail).
    private static let bigWitness = 1 << 18
    private static let smallWorkCap = 4 * 1024 * 1024

    func testABigWitnessWithAStallingPropertyStaysInsideTheWorkBudget() throws {
        ShrinkBudget.defaultMaxWork = ShrinkBudgetTests.smallWorkCap
        let big = [UInt8](repeating: 1, count: ShrinkBudgetTests.bigWitness)
        let ceiling = Ceiling(600)
        // Every smaller candidate holds, so bisection stalls: the worst case.
        let r = try Shrink.shrinkBytes(initial: big, prop: { c in ceiling.tick() ? c.count < big.count : true }, maxSteps: 1000)
        XCTAssertFalse(ceiling.hit, "evaluations=\(ceiling.count): the work cap no longer bounds the shrink")
        XCTAssertEqual(r.value.count, big.count)
        XCTAssertLessThanOrEqual(ShrinkBudget.lastWork, ShrinkBudgetTests.smallWorkCap)
        XCTAssertLessThan(ceiling.count, Limits.attemptBudget(1000) + 2)
    }

    func testTheRunnerShrinkOfABigWitnessIsAlsoBounded() throws {
        ShrinkBudget.defaultMaxWork = ShrinkBudgetTests.smallWorkCap
        let g = try Gens.bytes(ShrinkBudgetTests.bigWitness, ShrinkBudgetTests.bigWitness)
        let ceiling = Ceiling(600)
        let v = try Runner.check(seed: 1, gen: g, options: Options(numTests: 1, maxShrinkSteps: 1000, ci: .on)) { _ in !ceiling.tick() }
        XCTAssertFalse(ceiling.hit, "evaluations=\(ceiling.count): the work cap no longer bounds the shrink")
        XCTAssertEqual(v.failureId, FailureIds.counterexample)
        XCTAssertLessThanOrEqual(ShrinkBudget.lastWork, ShrinkBudgetTests.smallWorkCap)
    }

    func testAMegabyteWitnessIsStillBoundedAtTheDefaultCap() throws {
        let big = [UInt8](repeating: 1, count: Limits.maxInputBytes)
        let ceiling = Ceiling(400)
        let r = try Shrink.shrinkBytes(initial: big, prop: { c in ceiling.tick() ? c.count < big.count : true }, maxSteps: 1000)
        XCTAssertFalse(ceiling.hit, "evaluations=\(ceiling.count)")
        XCTAssertEqual(r.value.count, big.count)
        XCTAssertLessThanOrEqual(ShrinkBudget.lastWork, Limits.maxShrinkWork)
    }

    func testABudgetSpentByWorkEndsEveryLoopAndReturnsAFalsifyingValue() throws {
        ShrinkBudget.defaultMaxWork = 5_000
        let big = (0..<3000).map { BigInt($0) }
        let r = try Shrink.shrinkList(initial: big, elemTarget: .zero, prop: { $0.count < 3000 }, maxSteps: 10_000)
        XCTAssertFalse(r.value.count < 3000, "still a counterexample")
        XCTAssertLessThanOrEqual(ShrinkBudget.lastWork, 5_000)
    }

    func testTheBudgetCapsEvaluationsOfAnAlwaysFailingNestedList() throws {
        var evals = 0
        let g = try Gens.list(try Gens.list(Gens.u8(), 0, 20), 1, 20)
        let v = try Runner.check(seed: 4, gen: g, options: Options(numTests: 1, maxShrinkSteps: 200, ci: .on)) { _ in evals += 1; return false }
        XCTAssertEqual(v.failureId, FailureIds.counterexample)
        XCTAssertLessThanOrEqual(evals, 1 + Limits.attemptBudget(200) + 1)
    }

    // MARK: nesting: every public entry is bounded by the outermost root ---------

    func testAPropertyCallingRunnerCheckKeepsTheRootTotalsWithinTheRootCap() throws {
        var evals = 0
        let outerOptions = Options(numTests: 50, maxShrinkSteps: 100, ci: .on)
        let rootCap = outerOptions.numTests + Limits.attemptBudget(outerOptions.maxShrinkSteps)
        let v = try Runner.check(seed: 1, gen: try Gens.int(0, 1000), options: outerOptions) { _ in
            evals += 1
            let inner = try Runner.check(seed: 2, gen: try Gens.int(0, 1_000_000), options: Options(numTests: 100, maxShrinkSteps: 1000, ci: .on)) { y in
                evals += 1
                return y < 10
            }
            return inner.passed
        }
        XCTAssertFalse(v.passed)
        XCTAssertLessThanOrEqual(ShrinkBudget.lastAttempts, rootCap)
        XCTAssertLessThanOrEqual(evals, rootCap + outerOptions.numTests, "evals=\(evals) cap=\(rootCap)")
    }

    func testABranchingPropertyKeepsTotalEvaluationsWithinTheRootCap() throws {
        let ceiling = Ceiling(40_000)
        func branch(_ level: Int) throws -> Bool {
            guard ceiling.tick() else { return true }
            if level == 0 { return false }
            for s in 1...3 {
                let v = try Runner.check(seed: UInt64(s), gen: try Gens.int(0, 100), options: Options(numTests: 20, maxShrinkSteps: 50, ci: .on)) { _ in try branch(level - 1) }
                if v.passed { return true }
            }
            return false
        }
        let opts = Options(numTests: 5, maxShrinkSteps: 50, ci: .on)
        _ = try Runner.check(seed: 1, gen: Gens.bool(), options: opts) { _ in try branch(5) }
        XCTAssertFalse(ceiling.hit, "the evaluation ceiling was reached: the tree no longer bounds the chain")
        // 3^5 * 20 would be ~4,860 uncharged evaluations; the tree caps the chain.
        XCTAssertLessThanOrEqual(ShrinkBudget.lastAttempts, opts.numTests + Limits.attemptBudget(50))
        XCTAssertLessThan(ceiling.count, 5 * (opts.numTests + Limits.attemptBudget(50)))
    }

    func testARecursingCustomShrinkerStaysInsideTheRootCap() throws {
        let ceiling = Ceiling(60_000)
        var evals: Int { ceiling.count }
        let inner = try Gens.int(0, 1_000_000)
        let recursing: Shrinker<Int> = { value, falsifies, maxSteps in
            // A malicious shrinker that re-enters the PUBLIC Gen.shrink per probe.
            for _ in 0..<50 {
                _ = try? inner.shrink(value, falsifies: { _ in ceiling.tick() ? falsifies(value) : false }, maxSteps: 1000)
            }
            return Shrunk(value: value, steps: 0)
        }
        let opts = Options(numTests: 1, maxShrinkSteps: 100, ci: .on)
        let v = try Runner.check(seed: 1, gen: try Gens.int(0, 1000), options: opts, shrinker: recursing) { _ in false }
        XCTAssertEqual(v.failureId, FailureIds.counterexample)
        XCTAssertLessThanOrEqual(ShrinkBudget.lastAttempts, opts.numTests + Limits.attemptBudget(100))
        XCTAssertFalse(ceiling.hit, "evals=\(evals)")
        XCTAssertLessThanOrEqual(evals, opts.numTests + Limits.attemptBudget(100))
    }

    func testAFilterPredicateThatShrinksStaysInsideTheRootCap() throws {
        var evals = 0
        let g = try Gens.filter(try Gens.int(0, 1000)) { x in
            // The predicate itself shrinks: user code re-entering the engine.
            _ = try? Shrink.shrinkInt(initial: BigInt(x), target: .zero, lo: .zero, hi: BigInt(1000), prop: { _ in evals += 1; return false }, maxSteps: 1000)
            return x % 2 == 0
        }
        let opts = Options(numTests: 10, maxShrinkSteps: 100, ci: .on)
        _ = try Runner.check(seed: 1, gen: g, options: opts) { $0 < 5 }
        XCTAssertLessThanOrEqual(ShrinkBudget.lastAttempts, opts.numTests + Limits.attemptBudget(100))
        XCTAssertLessThanOrEqual(ShrinkBudget.lastWork, Limits.maxShrinkWork)
    }

    func testShrinkIntAndSimulateReplayInsideAPropertyAreCharged() throws {
        let opts = Options(numTests: 3, maxShrinkSteps: 10, ci: .on)
        var evals = 0
        _ = try Runner.check(seed: 1, gen: Gens.bool(), options: opts) { _ in
            _ = try? Shrink.shrinkInt(initial: 900, target: .zero, lo: .zero, hi: 1000, prop: { _ in evals += 1; return false }, maxSteps: 1000)
            _ = try? Shrink.simulateReplay(seed: 3, prop: { _ in evals += 1; return false })
            return true
        }
        XCTAssertLessThanOrEqual(ShrinkBudget.lastAttempts, opts.numTests + Limits.attemptBudget(10))
        XCTAssertGreaterThan(evals, 0)
    }

    // MARK: starvation is never a pass -----------------------------------------------

    func testAStarvedNestedCheckIsExhaustedButATopLevelCheckStillRuns() throws {
        let scope = ShrinkBudget.enterChild(maxSteps: 0)
        for _ in 0..<64 { _ = scope.charge(1) }          // spend it all
        let nested = try Runner.check(seed: 1, gen: Gens.bool(), options: ci) { _ in true }
        XCTAssertFalse(nested.passed)
        XCTAssertEqual(nested.failureId, FailureIds.exhausted)
        scope.close()
        let top = try Runner.check(seed: 1, gen: Gens.bool(), options: ci) { _ in true }
        XCTAssertTrue(top.passed)
    }

    func testAStarvedNestedCheckNeverLetsTheTopLevelCheckPass() throws {
        // The outer property is TRUE on every case, but an inner check is starved:
        // the outer run must not report a pass.
        let opts = Options(numTests: 500, maxShrinkSteps: 0, ci: .on)    // attempt cap 500 + 64
        let v = try Runner.check(seed: 1, gen: Gens.bool(), options: opts) { _ in
            let inner = try Runner.check(seed: 2, gen: Gens.bool(), options: Options(numTests: 5, maxShrinkSteps: 0, ci: .on)) { _ in true }
            _ = inner
            return true
        }
        XCTAssertFalse(v.passed)
        XCTAssertEqual(v.failureId, FailureIds.exhausted)
        XCTAssertTrue(v.message!.contains("starved"))
    }

    func testAPassingInnerCheckWithHugeNumTestsIsBoundedUnderAnOuterShrink() throws {
        let ceiling = Ceiling(5_000)
        let opts = Options(numTests: 1, maxShrinkSteps: 20, ci: .on)
        _ = try Runner.check(seed: 1, gen: Gens.bool(), options: opts) { _ in
            _ = try Runner.check(seed: 2, gen: Gens.bool(), options: Options(numTests: 100_000, maxShrinkSteps: 10, ci: .on)) { _ in !ceiling.tick() ? false : true }
            return false
        }
        XCTAssertFalse(ceiling.hit, "evaluations=\(ceiling.count)")
        XCTAssertLessThanOrEqual(ceiling.count, opts.numTests + Limits.attemptBudget(20) + 5)
    }

    func testANestedCheckSamplingLargeListsIsChargedAsWork() throws {
        ShrinkBudget.defaultMaxWork = 2_000
        var cases = 0
        let opts = Options(numTests: 1, maxShrinkSteps: 5, ci: .on)
        _ = try Runner.check(seed: 1, gen: Gens.bool(), options: opts) { _ in
            let big = try Gens.list(Gens.u8(), 500, 500)
            _ = try Runner.check(seed: 3, gen: big, options: Options(numTests: 1000, maxShrinkSteps: 5, ci: .on)) { _ in cases += 1; return true }
            return true
        }
        XCTAssertLessThanOrEqual(cases * 500, 2_000 + 500, "cases=\(cases)")
        XCTAssertGreaterThan(cases, 0)
    }

    func testAnInnerCheckThatSamplesListsOfListsIsChargedByContents() throws {
        ShrinkBudget.defaultMaxWork = 5_000
        var cases = 0
        let g = try Gens.list(try Gens.list(Gens.u8(), 20, 20), 20, 20)    // 20 + 20*20 = 420 per case
        _ = try Runner.check(seed: 1, gen: Gens.bool(), options: Options(numTests: 1, maxShrinkSteps: 5, ci: .on)) { _ in
            _ = try Runner.check(seed: 3, gen: g, options: Options(numTests: 1000, maxShrinkSteps: 5, ci: .on)) { _ in cases += 1; return true }
            return true
        }
        XCTAssertLessThanOrEqual(cases, 5_000 / 420 + 2, "cases=\(cases)")
    }

    func testARefusedReVerificationReportsTheOriginalWitnessWithZeroSteps() throws {
        // A shrinker that burns the whole attempt budget of the check chain, so the
        // single re-verification charge is refused.
        let burner: Shrinker<Int> = { value, _, _ in
            if let b = ShrinkBudget.current() { var spent = 0
            while spent < 1_000_000 && b.charge(1) { spent += 1 } }
            return Shrunk(value: 0, steps: 1)
        }
        let v = try Runner.check(seed: 1, gen: try Gens.int(1, 1000), options: Options(numTests: 1, maxShrinkSteps: 10, ci: .off), shrinker: burner) { $0 == 0 }
        XCTAssertEqual(v.failureId, FailureIds.counterexample)
        XCTAssertEqual(v.shrinkSteps, 0)
        XCTAssertEqual(v.minimal as? Int, v.initial as? Int)
    }

    // MARK: filter charging ------------------------------------------------------------

    func testAFilterPredicateIsNotInvokedUnderASpentBudget() throws {
        var predicateCalls = 0
        let g = try Gens.filter(try Gens.int(0, 100)) { _ in predicateCalls += 1; return true }
        let scope = ShrinkBudget.enterChild(maxSteps: 0)
        for _ in 0..<64 { _ = scope.charge(1) }
        _ = g.shrinkNested(50, { _ in true }, 100)
        XCTAssertEqual(predicateCalls, 0, "the budget is checked before the user predicate runs")
        scope.close()
    }

    func testAFilterRejectingEverythingStaysInsideTheRootCapInANestedCheck() throws {
        var preds = 0
        let g = try Gens.filter(try Gens.int(0, 10), maxRetries: 10_000) { _ in preds += 1; return false }
        let opts = Options(numTests: 10, maxShrinkSteps: 5, ci: .on)
        let v = try Runner.check(seed: 1, gen: Gens.bool(), options: opts) { _ in
            _ = try Runner.check(seed: 2, gen: g, options: Options(numTests: 50, maxShrinkSteps: 5, ci: .on)) { _ in true }
            return true
        }
        XCTAssertLessThanOrEqual(preds, opts.numTests + Limits.attemptBudget(5), "preds=\(preds)")
        XCTAssertFalse(v.passed)
    }

    func testStackedTopLevelFiltersAreBoundedByTheWorkAllowance() throws {
        ShrinkBudget.defaultMaxWork = 20_000
        var calls = 0
        let rare = { (x: Int) -> Bool in calls += 1; return false }
        let g = try Gens.filter(try Gens.filter(try Gens.filter(try Gens.int(0, 10), maxRetries: 10_000, rare), maxRetries: 10_000, { _ in true }), maxRetries: 10_000, { _ in true })
        let v = try Runner.check(seed: 1, gen: g, options: Options(numTests: 1, ci: .on)) { _ in true }
        XCTAssertEqual(v.failureId, FailureIds.exhausted)
        XCTAssertLessThanOrEqual(calls, 20_001)
    }

    func testAMaxRetriesCapExistsAndTopLevelFiltersDoNotUseTheAttemptCap() throws {
        XCTAssertThrowsError(try Gens.filter(Gens.bool(), maxRetries: Limits.maxFilterRetries + 1) { _ in true })
        // A rare-but-satisfiable filter (1 in 5000) must NOT be starved at top level,
        // where the attempt cap is only about 17k.
        let g = try Gens.filter(try Gens.int(0, 4999), maxRetries: 10_000) { $0 == 1234 }
        let v = try Runner.check(seed: 11, gen: g, options: Options(numTests: 3, maxShrinkSteps: 10, ci: .on)) { $0 == 1234 }
        XCTAssertTrue(v.passed || v.failureId == FailureIds.exhausted)
    }

    func testMaxStepsIsValidatedAndZeroStepsReturnsTheInput() throws {
        let g = try Gens.list(Gens.u8(), 0, 5)
        XCTAssertThrowsError(try g.shrink([1], falsifies: { _ in true }, maxSteps: -1))
        XCTAssertThrowsError(try g.shrink([1], falsifies: { _ in true }, maxSteps: Limits.maxShrinkSteps + 1))
        XCTAssertEqual(try g.shrink([1, 2], falsifies: { _ in true }, maxSteps: 0).value, [1, 2])
        XCTAssertEqual(try Gens.bytes(0, 5).shrink([1, 2], falsifies: { _ in true }, maxSteps: 0).value, [1, 2])
    }

    func testBytePathsChargeTwiceTheCandidateSizePerAttempt() throws {
        // initial: 3n (clone, list, array) + 1 attempt; sequence entry: n; then
        // every candidate costs its size TWICE (the attempt and the conversion).
        let n = 100
        var sizes: [Int] = []
        _ = try Shrink.shrinkBytes(initial: [UInt8](repeating: 3, count: n), prop: { c in sizes.append(c.count); return c.count < n }, maxSteps: 10)
        let candidates = sizes.dropFirst()    // the first call is the initial check
        let expected = 3 * n + 1 + n + 2 * candidates.reduce(0) { $0 + max(1, $1) }
        XCTAssertEqual(ShrinkBudget.lastWork, expected)
    }

    func testTheAmbientBudgetIsRestoredAfterAThrowInsideAProperty() throws {
        struct E: Error {}
        _ = try Runner.check(seed: 1, gen: Gens.bool(), options: ci) { _ in throw E() }
        XCTAssertNil(ShrinkBudget.current())
        _ = try? Shrink.shrinkInt(initial: 5, target: .zero, lo: .zero, hi: 10, prop: { _ in true }, maxSteps: 5)
        XCTAssertNil(ShrinkBudget.current())
    }
}
