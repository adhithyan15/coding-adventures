import Foundation
import XCTest
@testable import TestBootstrap

/// Regressions for the security review (F1-F7).
final class HardeningTests: XCTestCase {
    override func setUp() { super.setUp(); Hooks.reset() }
    override func tearDown() {
        Hooks.reset()
        XCTAssertNil(ShrinkBudget.current(), "a test leaked an ambient scope")
        super.tearDown()
    }

    private final class Box<T> { var value: T; init(_ v: T) { value = v } }

    // MARK: F1 -- threads and tasks --------------------------------------------

    /// An outer check whose property starts `work` on another thread/task. The outer
    /// budget is small (67 attempts) and the inner check wants 500 cases, so an
    /// inner check charged to the OUTER tree must starve.
    private func outerWithInner(_ spawn: @escaping (_ inner: @escaping () -> Verdict) -> Verdict?) throws -> (outer: Verdict, inner: Verdict?) {
        let innerVerdict = Box<Verdict?>(nil)
        let opts = Options(numTests: 3, maxShrinkSteps: 0, ci: .on)
        let outer = try Runner.check(seed: 1, gen: Gens.bool(), options: opts) { _ in
            innerVerdict.value = spawn {
                (try? Runner.check(seed: 2, gen: Gens.bool(), options: Options(numTests: 500, maxShrinkSteps: 0, ci: .on)) { _ in true })!
            }
            return true
        }
        return (outer, innerVerdict.value)
    }

    func testACapturedScopeChargedFromASpawnedThreadMovesTheRootAndPropagatesStarvation() throws {
        let rootAttempts = Box(0)
        let opts = Options(numTests: 3, maxShrinkSteps: 0, ci: .on)
        let inner = Box<Verdict?>(nil)
        let outer = try Runner.check(seed: 1, gen: Gens.bool(), options: opts) { _ in
            let scope = Runner.currentScope()
            XCTAssertNotNil(scope)
            let before = scope!.node.attemptsSpent
            let done = DispatchSemaphore(value: 0)
            Thread.detachNewThread {
                inner.value = scope!.run {
                    try? Runner.check(seed: 2, gen: Gens.bool(), options: Options(numTests: 500, maxShrinkSteps: 0, ci: .on)) { _ in true }
                }
                done.signal()
            }
            XCTAssertEqual(done.wait(timeout: .now() + 30), .success)
            rootAttempts.value = max(rootAttempts.value, scope!.node.attemptsSpent - before)
            return true
        }
        XCTAssertEqual(inner.value?.failureId, FailureIds.exhausted, "the spawned check was charged to the outer tree and starved")
        XCTAssertGreaterThan(rootAttempts.value, 50, "the tree's counters moved")
        XCTAssertFalse(outer.passed, "the starvation reached the outer root's refused flag")
        XCTAssertEqual(outer.failureId, FailureIds.exhausted)
    }

    func testABareThreadGetsAFreshRootAndCannotStarveTheOuterCheck() throws {
        // The documented limit: without PbtScope.run a spawned thread is unconnected.
        let r = try outerWithInner { run in
            let box = Box<Verdict?>(nil)
            let done = DispatchSemaphore(value: 0)
            Thread.detachNewThread { box.value = run(); done.signal() }
            _ = done.wait(timeout: .now() + 30)
            return box.value
        }
        XCTAssertTrue(r.inner?.passed ?? false, "a fresh root ran all 500 cases")
        XCTAssertTrue(r.outer.passed, "and the outer check never learned of it")
    }

    func testATaskCreatedInsideAPropertyInheritsTheScope() throws {
        let r = try outerWithInner { run in
            let box = Box<Verdict?>(nil)
            let done = DispatchSemaphore(value: 0)
            Task { box.value = run(); done.signal() }
            _ = done.wait(timeout: .now() + 30)
            return box.value
        }
        XCTAssertEqual(r.inner?.failureId, FailureIds.exhausted, "the Task inherited the outer scope")
        XCTAssertEqual(r.outer.failureId, FailureIds.exhausted)
    }

    func testADetachedTaskDoesNotInherit() throws {
        let r = try outerWithInner { run in
            let box = Box<Verdict?>(nil)
            let done = DispatchSemaphore(value: 0)
            Task.detached { box.value = run(); done.signal() }
            _ = done.wait(timeout: .now() + 30)
            return box.value
        }
        XCTAssertTrue(r.inner?.passed ?? false)
    }

    func testPbtScopeRunRestoresThePreviousAmbientEvenOnAThrow() throws {
        struct E: Error {}
        XCTAssertNil(Runner.currentScope())
        let a = ShrinkBudget.enterChild(maxSteps: 10)
        let scopeA = Runner.currentScope()!
        a.close()                           // a is closed: usable() skips it
        XCTAssertNil(Runner.currentScope())
        let b = ShrinkBudget.enterChild(maxSteps: 10)
        let scopeB = Runner.currentScope()!
        XCTAssertThrowsError(try scopeB.run { throw E() })
        XCTAssertTrue(ShrinkBudget.current() === b, "the previous ambient is restored")
        _ = scopeA
        b.close()
        XCTAssertNil(ShrinkBudget.current())
    }

    // MARK: F2 -- Canon dictionaries ----------------------------------------------

    func testAHugeDictionaryOfMegabyteValuesStaysInsideTheOutputCap() {
        let big = String(repeating: "x", count: 1 << 20)
        var d: [Int: String] = [:]
        for i in 0..<5000 { d[i] = big }       // one shared COW buffer: no 5 GiB allocation
        let start = Date()
        let r = Canon.render(d)
        XCTAssertLessThanOrEqual(r.utf8.count, Canon.maxOutput + 8)
        XCTAssertTrue(r.hasSuffix("..."), "the cut is marked")
        XCTAssertLessThan(Date().timeIntervalSince(start), 30)
        // Values are truncated individually, so many keys fit.
        XCTAssertTrue(r.contains("\"xxxx"))
    }

    func testANonPairDictionaryMirrorTerminates() {
        struct Endless: CustomReflectable {
            var customMirror: Mirror { Mirror(self, unlabeledChildren: 0..<Int.max, displayStyle: .dictionary) }
        }
        struct EndlessSet: CustomReflectable {
            var customMirror: Mirror { Mirror(self, unlabeledChildren: 0..<Int.max, displayStyle: .set) }
        }
        let start = Date()
        XCTAssertFalse(Canon.render(Endless()).isEmpty)
        XCTAssertFalse(Canon.render(EndlessSet()).isEmpty)
        XCTAssertLessThan(Date().timeIntervalSince(start), 30)
    }

    // MARK: F3 -- traps in the public API ---------------------------------------------

    func testHostileStepBudgetsDoNotTrap() throws {
        XCTAssertEqual(Limits.attemptBudget(Int.max), Limits.attemptFactor * Limits.maxShrinkSteps + Limits.attemptSlack)
        XCTAssertEqual(Limits.attemptBudget(-5), Limits.attemptSlack)
        XCTAssertEqual(Limits.attemptBudget(Int.min), Limits.attemptSlack)
        let ints = try Gens.int(0, 100)
        XCTAssertNoThrow(ints.shrinker!(50, { _ in true }, Int.max))
        XCTAssertEqual(ints.shrinker!(50, { _ in true }, -3).steps, 0)
        XCTAssertNoThrow(try Gens.bytes(0, 8).shrinker!([1, 2, 3], { _ in true }, Int.max))
        XCTAssertNoThrow(try Gens.list(Gens.u8(), 0, 8).shrinker!([1, 2, 3], { _ in true }, Int.max))
        XCTAssertEqual(try Gens.bytes(0, 8).shrinker!([1, 2, 3], { _ in true }, Int.min).steps, 0)
        XCTAssertNoThrow(Gens.tuple2(Gens.u8(), Gens.u8()).shrinker!((9, 9), { _ in true }, Int.max))
        XCTAssertNoThrow(Gens.tuple3(Gens.u8(), Gens.u8(), Gens.bool()).shrinker!((9, 9, true), { _ in true }, Int.max))
        XCTAssertNoThrow(Gens.bool().shrinker!(true, { _ in true }, Int.max))
        XCTAssertThrowsError(try Shrink.shrinkInt(initial: 5, target: .zero, lo: .zero, hi: 9, prop: { _ in false }, maxSteps: Int.max))
        XCTAssertThrowsError(try ints.shrink(5, falsifies: { _ in true }, maxSteps: Int.max))
    }

    func testOversizedBigIntsAreRefusedNotTrapped() throws {
        let huge = BigInt.one.shiftedLeft(1050)          // inside the 1100-bit ceiling, outside the API cap
        XCTAssertThrowsError(try Shrink.shrinkInt(initial: huge, target: .zero, lo: .zero, hi: huge, prop: { _ in false }, maxSteps: 10))
        XCTAssertThrowsError(try Shrink.shrinkInt(initial: 5, target: huge, lo: .zero, hi: 9, prop: { _ in false }, maxSteps: 10))
        XCTAssertThrowsError(try Shrink.shrinkList(initial: [huge], elemTarget: .zero, prop: { _ in false }, maxSteps: 10))
        XCTAssertThrowsError(try Shrink.shrinkList(initial: [1], elemTarget: huge, prop: { _ in false }, maxSteps: 10))
        XCTAssertThrowsError(try Gens.genBounded(seed: 0, lo: .zero, hi: huge, count: 1))
        XCTAssertThrowsError(try Gens.genBounded(seed: 0, lo: -huge, hi: .zero, count: 1))
        XCTAssertThrowsError(try Gens.integer(-BigInt.one.shiftedLeft(999), BigInt.one.shiftedLeft(999)), "a 1001-bit span")
        XCTAssertNoThrow(try Gens.genBounded(seed: 0, lo: .zero, hi: BigInt.one.shiftedLeft(900), count: 2))
    }

    // MARK: F4 -- bounded recursion -----------------------------------------------------

    func testRecursiveChecksAreRefusedPastTheNestingCap() throws {
        let depth = Box(0), evals = Box(0)
        let ceiling = 50_000
        func rec(_ d: Int) throws -> Bool {
            evals.value += 1
            if evals.value > ceiling { return true }          // hard stop: the assertion below then fails
            depth.value = max(depth.value, d)
            return try Runner.check(seed: 1, gen: Gens.bool(), options: Options(numTests: 1, maxShrinkSteps: 1000, ci: .on)) { _ in try rec(d + 1) }.passed
        }
        let v = try Runner.check(seed: 1, gen: Gens.bool(), options: Options(numTests: 1, maxShrinkSteps: 1000, ci: .on)) { _ in try rec(1) }
        XCTAssertFalse(v.passed)
        XCTAssertLessThan(evals.value, ceiling)
        XCTAssertLessThanOrEqual(depth.value, ShrinkBudget.maxNesting + 1)
    }

    func testRecursiveShrinksAreRefusedPastTheNestingCap() throws {
        let g = try Gens.int(0, 10)
        let maxDepth = Box(0)
        func nest(_ d: Int) -> Int {
            maxDepth.value = max(maxDepth.value, d)
            if d > 5_000 { return -1 }
            return (try? g.shrink(5, falsifies: { _ in _ = nest(d + 1); return true }, maxSteps: 10).steps) ?? -1
        }
        _ = nest(1)
        XCTAssertLessThanOrEqual(maxDepth.value, ShrinkBudget.maxNesting + 2)
        XCTAssertNil(ShrinkBudget.current())
        // Shrink.* entries are refused the same way.
        let m = Box(0)
        func nestInt(_ d: Int) {
            m.value = max(m.value, d)
            if d > 5_000 { return }
            _ = try? Shrink.shrinkInt(initial: 5, target: .zero, lo: .zero, hi: 10, prop: { _ in nestInt(d + 1); return false }, maxSteps: 10)
        }
        nestInt(1)
        XCTAssertLessThanOrEqual(m.value, ShrinkBudget.maxNesting + 2)
    }

    func testGenShrinkAndShrinkEntriesChargeOneAttemptOnEntry() throws {
        let scope = ShrinkBudget.enterChild(maxSteps: 0)          // 64 attempts
        for _ in 0..<64 { _ = scope.charge(1) }                   // spent
        let r = try Gens.int(0, 10).shrink(7, falsifies: { _ in true }, maxSteps: 10)
        XCTAssertEqual(r.steps, 0)
        XCTAssertEqual(r.value, 7)
        scope.close()
        let fresh = try Gens.int(0, 10).shrink(7, falsifies: { _ in true }, maxSteps: 10)
        XCTAssertEqual(fresh.value, 0)
    }

    // MARK: F5 -- canonical rendering ------------------------------------------------------

    func testSetsRenderIdenticallyWhateverTheirIterationOrder() {
        var a = Set<Int>(); a.reserveCapacity(4)
        var b = Set<Int>(); b.reserveCapacity(5000)
        for i in 0..<200 { a.insert(i * 7); b.insert(i * 7) }
        XCTAssertEqual(Canon.render(a), Canon.render(b))
        XCTAssertEqual(Canon.render(Set(["b", "a", "c"])), "[\"a\",\"b\",\"c\"]")
        XCTAssertEqual(Canon.render(["b": 1, "a": 2]), "{\"a\":2,\"b\":1}")
    }

    func testUnicodeFormattingAndC1ControlsAreEscaped() {
        let s = "a\u{80}b\u{9f}c\u{2028}d\u{2029}e\u{202A}\u{202B}\u{202C}\u{202D}\u{202E}f\u{a0}g"
        let r = Canon.render(s)
        for esc in ["\\u0080", "\\u009f", "\\u2028", "\\u2029", "\\u202a", "\\u202b", "\\u202c", "\\u202d", "\\u202e"] {
            XCTAssertTrue(r.contains(esc), esc)
        }
        for raw in ["\u{80}", "\u{9f}", "\u{2028}", "\u{2029}", "\u{202E}"] { XCTAssertFalse(r.contains(raw)) }
        XCTAssertTrue(r.contains("\u{a0}"), "U+00A0 is ordinary text")
    }

    // MARK: F6 -- byte-exact keys -----------------------------------------------------------

    func testNfcAndNfdKeysAreDistinctAndExactDuplicatesAreStillRefused() throws {
        let nfc = "\u{e9}", nfd = "e\u{301}"
        XCTAssertEqual(nfc, nfd, "Swift's == treats them as equal; the parser must not")
        let doc = "{\"\(nfc)\":1,\"\(nfd)\":2}"
        let j = try StrictJson.parse(Array(doc.utf8))
        guard case .number(let one)? = j[nfc], case .number(let two)? = j[nfd] else { return XCTFail("lookup must be byte-exact") }
        XCTAssertEqual(one, "1")
        XCTAssertEqual(two, "2")
        XCTAssertThrowsError(try StrictJson.parse(Array("{\"\(nfc)\":1,\"\(nfc)\":2}".utf8)))
        XCTAssertThrowsError(try StrictJson.parse(Array("{\"\(nfd)\":1,\"\(nfd)\":2}".utf8)))
    }

    func testNfcAndNfdCaseIdsAreNotDuplicates() throws {
        var cases = try Support.cases()
        cases[0] = Support.setting(cases[0], "id", .string("pbt/\u{e9}"))
        cases[1] = Support.setting(cases[1], "id", .string("pbt/e\u{301}"))
        XCTAssertFalse(Corpus.evaluate(cases).problems.contains { $0.contains("duplicate or missing case id") })
        cases[1] = Support.setting(cases[1], "id", .string("pbt/\u{e9}"))
        XCTAssertTrue(Corpus.evaluate(cases).problems.contains { $0.contains("duplicate or missing case id") })
    }

    // MARK: F7 -- JSON depth ------------------------------------------------------------------

    func testJsonDepthIsExactly64Containers() {
        func nest(_ n: Int) -> [UInt8] { Array((String(repeating: "[", count: n) + String(repeating: "]", count: n)).utf8) }
        XCTAssertNoThrow(try StrictJson.parse(nest(64)))
        XCTAssertThrowsError(try StrictJson.parse(nest(65)))
        XCTAssertNoThrow(try StrictJson.parse(Array((String(repeating: "[", count: 63) + "[1]" + String(repeating: "]", count: 63)).utf8)))
    }

    // MARK: round-2 LOW items ---------------------------------------------------------

    /// A string whose equality and hash are by Unicode scalars, so a Set can hold
    /// the NFC and NFD spellings as two elements (Swift's own String would merge them).
    private struct Raw: Hashable {
        let s: String
        static func == (a: Raw, b: Raw) -> Bool { a.s.unicodeScalars.elementsEqual(b.s.unicodeScalars) }
        func hash(into h: inout Hasher) { for u in s.unicodeScalars { h.combine(u.value) } }
    }

    func testNfcAndNfdSetElementsRenderIdenticallyAcrossRuns() {
        let nfc = Raw(s: "\u{e9}"), nfd = Raw(s: "e\u{301}")
        let all = [nfd, Raw(s: "a"), nfc, Raw(s: "z")]
        var first: String?
        for _ in 0..<20 {
            var set = Set<Raw>()
            set.reserveCapacity(Int.random(in: 1...500))
            for x in all.shuffled() { set.insert(x) }
            XCTAssertEqual(set.count, 4, "both spellings are present")
            let r = Canon.render(set)
            if first == nil { first = r }
            XCTAssertEqual(r, first)
        }
        XCTAssertEqual(first, "[{\"a\"},{\"e\u{301}\"},{\"z\"},{\"\u{e9}\"}]", "UTF-8 byte order: a, e+U+0301, z, U+00E9")
    }

    func testNestedShrinkersReturningHostileStepCountsDoNotTrap() throws {
        for hostile in [Int.max, -1, Int.min] {
            let evil = Gen<Int>(sampler: { rng in Int(try rng.bounded(100)) },
                                shrinker: { v, _, _ in Shrunk(value: 0, steps: hostile) })
            let lists = try Gens.list(evil, 1, 4)
            let v1 = try Runner.check(seed: 1, gen: lists, options: Options(numTests: 5, ci: .on)) { _ in false }
            XCTAssertEqual(v1.failureId, FailureIds.counterexample)
            XCTAssertLessThanOrEqual(v1.shrinkSteps, 1000)
            let t2 = Gens.tuple2(evil, evil)
            let v2 = try Runner.check(seed: 1, gen: t2, options: Options(numTests: 5, ci: .on)) { _ in false }
            XCTAssertEqual(v2.failureId, FailureIds.counterexample)
            XCTAssertLessThanOrEqual(v2.shrinkSteps, 1000)
            let t3 = Gens.tuple3(evil, evil, evil)
            XCTAssertNoThrow(try Runner.check(seed: 1, gen: t3, options: Options(numTests: 5, ci: .on)) { _ in false })
            XCTAssertNoThrow(try Gens.list(evil, 1, 3).shrink([5, 6], falsifies: { _ in true }, maxSteps: 1000))
        }
    }

    func testTheNestingCapIs64() {
        XCTAssertEqual(ShrinkBudget.maxNesting, 64)
    }
}
