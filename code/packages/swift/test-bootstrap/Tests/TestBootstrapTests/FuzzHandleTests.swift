import Foundation
import XCTest
@testable import TestBootstrap

/// Budgets, sticky exhaustion, handle retirement and sentinels. Every wait has a
/// hard ceiling so a regressed guarantee FAILS instead of hanging.
final class FuzzHandleTests: XCTestCase {
    private final class Box<T> {
        var value: T
        init(_ v: T) { value = v }
    }

    private func run(_ seeds: [[UInt8]] = [[1]], iterations: Int = 0, step: Int = 3, mem: Int = 100, target: @escaping Target) throws -> FuzzReport {
        try FuzzEngine.run(corpus: seeds, options: FuzzOptions(maxIterations: iterations, stepBudget: step, memBudget: mem), target: target)
    }

    // MARK: the handle on its own ----------------------------------------------------------------

    func testTickReserveAndEdgeBasics() throws {
        let h = Handle(stepBudget: 2, memBudget: 10)
        let cov = Bitmap()
        h.attach(cov)
        h.edge(7); h.edge(7 + 65_536)
        XCTAssertEqual(cov.count, 1)
        try h.tick(); try h.tick()
        XCTAssertEqual(h.stepsUsed, 2)
        XCTAssertThrowsError(try h.tick())
        XCTAssertEqual(h.exhausted?.which, .steps)
        // Non-positive reserves nothing; a fresh handle.
        let g = Handle(stepBudget: 1, memBudget: 10)
        try g.reserve(0); try g.reserve(-5); try g.reserve(10)
        XCTAssertEqual(g.bytesReserved, 10)
        XCTAssertThrowsError(try g.reserve(1))
        XCTAssertEqual(g.exhausted?.which, .memory)
        // The overflow-safe test: n > budget - reserved, even for Int.max.
        let k = Handle(stepBudget: 1, memBudget: 10)
        XCTAssertThrowsError(try k.reserve(Int.max))
        XCTAssertEqual(k.bytesReserved, 0)
        XCTAssertEqual(Handle.edgeId("x"), fnv1a64("x"))
    }

    func testExhaustionIsStickyAndSharedAcrossBothBudgets() throws {
        let h = Handle(stepBudget: 0, memBudget: 10)
        var first: AnyObject?
        do { try h.tick() } catch { first = error as AnyObject }
        XCTAssertNotNil(first)
        // The memory hook now rethrows the SAME steps sentinel, not a new memory one.
        do { try h.reserve(1); XCTFail("must throw") } catch { XCTAssertTrue(error as AnyObject === first) }
        do { try h.tick(); XCTFail("must throw") } catch { XCTAssertTrue(error as AnyObject === first) }
        XCTAssertTrue(h.exhausted === first)
        XCTAssertEqual(h.exhausted?.which, .steps)
        let m = Handle(stepBudget: 5, memBudget: 1)
        XCTAssertThrowsError(try m.reserve(2))
        XCTAssertThrowsError(try m.tick()) // sticky: memory broke first
        XCTAssertEqual(m.exhausted?.which, .memory)
        XCTAssertEqual(m.stepsUsed, 0)
    }

    func testRetiredHandleIsInert() throws {
        let h = Handle(stepBudget: 5, memBudget: 100)
        let cov = Bitmap()
        h.attach(cov)
        h.edge(1)
        try h.tick(); try h.reserve(3)
        h.retire()
        XCTAssertTrue(h.isRetired)
        h.edge(2); h.edge(3)
        XCTAssertEqual(cov.edges(), [1])
        for call in [{ try h.tick() }, { try h.reserve(1) }] {
            do { try call(); XCTFail("must throw") } catch let s as BudgetSentinel { XCTAssertEqual(s.which, .retired) }
        }
        XCTAssertEqual(h.stepsUsed, 1)
        XCTAssertEqual(h.bytesReserved, 3)
        XCTAssertNil(h.exhausted, "retirement is not exhaustion")
        XCTAssertEqual(h.scope("x") { 41 + 1 }, 42)
        XCTAssertEqual(h.frames(for: FuzzSupport.Boom()), [])
    }

    // MARK: sticky exhaustion through the campaign: swallow-then-... ------------------------------

    func testSwallowThenCallThenReturnIsATimeoutNotAPass() throws {
        let r = try run { _, h in
            for _ in 0..<20 { try? h.tick() } // swallows every error, exhausts at 4
            try? h.reserve(1)                 // still stopped
        }
        XCTAssertEqual(r.timeouts, 1)
        XCTAssertEqual(r.crashes.count, 0)
        XCTAssertEqual(r.ooms, 0)
        XCTAssertEqual(r.admittedSeeds, 0, "a swallowed exhaustion is never admitted")
        XCTAssertFalse(r.isClean)
    }

    func testSwallowThenCallOtherBudgetPropagatesTheSameVerdict() throws {
        // memory breaks first, steps called afterwards: still an OOM.
        let oomFirst = try run(mem: 10) { _, h in
            do { try h.reserve(1000) } catch {}
            try h.tick()
        }
        XCTAssertEqual(oomFirst.ooms, 1); XCTAssertEqual(oomFirst.timeouts, 0); XCTAssertEqual(oomFirst.crashes.count, 0)
        // steps break first, reserve afterwards: still a timeout.
        let stepsFirst = try run(step: 0) { _, h in
            do { try h.tick() } catch {}
            try h.reserve(1)
        }
        XCTAssertEqual(stepsFirst.timeouts, 1); XCTAssertEqual(stepsFirst.ooms, 0); XCTAssertEqual(stepsFirst.crashes.count, 0)
    }

    func testSwallowThenThrowSomethingOrdinaryIsATimeoutNotACrash() throws {
        let r = try run(step: 1) { _, h in
            try? h.tick(); try? h.tick()
            throw FuzzSupport.Boom()
        }
        XCTAssertEqual(r.timeouts, 1)
        XCTAssertEqual(r.crashes.count, 0)
        let o = try run(mem: 1) { _, h in
            try? h.reserve(5)
            throw FuzzSupport.Other()
        }
        XCTAssertEqual(o.ooms, 1); XCTAssertEqual(o.crashes.count, 0)
    }

    func testBudgetsResetPerExecutionAndExhaustionIsPerExecution() throws {
        let calls = Box(0)
        let r = try run([[1], [2], [3], [4]], step: 2) { d, h in
            calls.value += 1
            for _ in 0..<(d[0] == 3 ? 10 : 2) { try h.tick() }
        }
        XCTAssertEqual(calls.value, 4)
        XCTAssertEqual(r.timeouts, 1)
        XCTAssertEqual(r.admittedSeeds, 1, "no edges: only the first passing seed is admitted")
    }

    // MARK: stale handles used from other threads and tasks -------------------------------------------

    func testStaleThreadCannotTouchALaterExecution() throws {
        let gate = DispatchSemaphore(value: 0), done = DispatchSemaphore(value: 0)
        let seen = Box<[String]>([])
        let handles = Box<[Handle]>([])
        let started = Box(false)
        let r = try run([[1], [2]], step: 1000) { d, h in
            handles.value.append(h)
            if d[0] == 1 {
                // Spawn a thread that fires only once execution TWO is under way.
                started.value = true
                Thread.detachNewThread {
                    _ = gate.wait(timeout: .now() + 10)
                    h.edge(4242)
                    do { try h.tick() } catch let s as BudgetSentinel { seen.value.append("tick:\(s.which)") } catch {}
                    do { try h.reserve(1) } catch let s as BudgetSentinel { seen.value.append("reserve:\(s.which)") } catch {}
                    done.signal()
                }
            } else {
                // Execution two: release the stale thread, wait for it, report nothing.
                gate.signal()
                XCTAssertTrue(FuzzSupport.wait(done), "stale thread never finished")
                try h.tick()
            }
        }
        XCTAssertTrue(started.value)
        XCTAssertEqual(seen.value, ["tick:retired", "reserve:retired"])
        XCTAssertEqual(r.totalEdges, 0, "the stale thread's edge must not reach the campaign map")
        XCTAssertEqual(r.coverageTier, FuzzReport.tierBlackBox)
        XCTAssertEqual(handles.value[1].stepsUsed, 1, "execution two's own step count is untouched by the stale thread")
        XCTAssertEqual(handles.value[1].bytesReserved, 0)
        XCTAssertEqual(r.timeouts + r.ooms + r.crashes.count, 0)
    }

    func testStaleTaskCannotTouchALaterExecution() throws {
        let gate = DispatchSemaphore(value: 0), done = DispatchSemaphore(value: 0)
        let seen = Box<[String]>([])
        let r = try run([[1], [2]], step: 1000) { d, h in
            if d[0] == 1 {
                Task.detached {
                    _ = gate.wait(timeout: .now() + 10)
                    h.edge(9999)
                    do { try h.tick() } catch let s as BudgetSentinel { seen.value.append("tick:\(s.which)") } catch {}
                    done.signal()
                }
            } else {
                gate.signal()
                XCTAssertTrue(FuzzSupport.wait(done), "stale task never finished")
            }
        }
        XCTAssertEqual(seen.value, ["tick:retired"])
        XCTAssertEqual(r.totalEdges, 0)
        XCTAssertEqual(r.timeouts + r.ooms + r.crashes.count, 0)
    }

    func testRacingThreadDuringTheRunNeverCorruptsTheMapAfterRetirement() throws {
        // A thread hammering `edge` while the target returns: after the run the map
        // must not change any more (retirement takes the same lock as edge).
        let stop = Box(false)
        let handle = Box<Handle?>(nil)
        let fin = DispatchSemaphore(value: 0)
        let counts = Box<[Int]>([])
        var cov: Bitmap?
        _ = try FuzzEngine.runOutcomes(corpus: [[1]], options: FuzzOptions(maxIterations: 0)) { data, bitmap in
            let h = Handle(stepBudget: 10, memBudget: 10)
            h.attach(bitmap)
            cov = bitmap
            handle.value = h
            Thread.detachNewThread {
                var i: UInt64 = 0
                while !stop.value && i < 2_000_000 { h.edge(i % 5000); i += 1 }
                fin.signal()
            }
            Thread.sleep(forTimeInterval: 0.01)
            return FuzzEngine.runTarget({ _, _ in }, data, h)
        }
        let after = cov!.count
        Thread.sleep(forTimeInterval: 0.05)
        counts.value = [after, cov!.count]
        stop.value = true
        XCTAssertTrue(FuzzSupport.wait(fin))
        XCTAssertEqual(counts.value[0], counts.value[1], "the map changed after retirement")
        XCTAssertTrue(handle.value!.isRetired)
    }

    // MARK: sentinels --------------------------------------------------------------------------------------

    func testAForgedSentinelIsACrashNotABudgetVerdict() throws {
        for which in [BudgetSentinel.Which.steps, .memory, .retired] {
            let r = try run { _, h in throw BudgetSentinel(which: which) }
            XCTAssertEqual(r.timeouts, 0); XCTAssertEqual(r.ooms, 0)
            XCTAssertEqual(r.crashes.count, 1)
            XCTAssertEqual(r.crashes.first?.kind, "stale-budget-sentinel")
            XCTAssertEqual(r.crashes.first?.signature, "8a20b3bf")
        }
    }

    func testASentinelCaughtInOneRunAndRethrownInTheNextIsStale() throws {
        let kept = Box<Error?>(nil)
        let r = try run([[1], [2]], step: 0) { d, h in
            if d[0] == 1 {
                do { try h.tick() } catch { kept.value = error } // swallow; exhausted -> a timeout
            } else {
                throw kept.value!
            }
        }
        XCTAssertEqual(r.timeouts, 1)
        XCTAssertEqual(r.crashes.count, 1)
        XCTAssertEqual(r.crashes.first?.kind, "stale-budget-sentinel")
    }

    func testTheHandlesOwnSentinelRethrownWithinTheRunIsAVerdict() throws {
        let t = try run(step: 0) { _, h in do { try h.tick() } catch { throw error } }
        XCTAssertEqual(t.timeouts, 1); XCTAssertEqual(t.crashes.count, 0)
        let o = try run(mem: 0) { _, h in try h.reserve(1) }
        XCTAssertEqual(o.ooms, 1); XCTAssertEqual(o.crashes.count, 0)
    }

    // MARK: scope / frames ---------------------------------------------------------------------------------------

    func testScopeSnapshotsTheLabelStackInnermostFirstWhereTheErrorLeft() throws {
        let r = try run { _, h in
            try h.scope("outer") { try h.scope("inner") { throw FuzzSupport.Boom() } }
        }
        XCTAssertEqual(r.crashes.first?.frames, ["inner", "outer"])
        XCTAssertEqual(r.crashes.first?.signature, FuzzSignature.crashSignature(kind: r.crashes[0].kind, frames: ["inner", "outer"]))
    }

    func testAHandledErrorLeavesNoStaleFrames() throws {
        let r = try run { _, h in
            try h.scope("outer") {
                do { try h.scope("inner") { throw FuzzSupport.Other() } } catch {} // handled inside `outer`
            }
            throw FuzzSupport.Boom()
        }
        XCTAssertEqual(r.crashes.first?.frames, [], "the handled error's snapshot must be discarded")
    }

    func testScopeLabelsAreSanitisedCollapsedAndBounded() throws {
        let r = try run { _, h in
            func go(_ n: Int) throws { if n == 0 { throw FuzzSupport.Boom() }; try h.scope("rec:\n\u{e9}") { try go(n - 1) } }
            try go(300) // 300 identical labels
        }
        XCTAssertEqual(r.crashes.first?.frames, ["rec___" + "_"])
        let deep = try run { _, h in
            func go(_ n: Int) throws { if n == 0 { throw FuzzSupport.Boom() }; try h.scope("f\(n % 100)") { try go(n - 1) } }
            try go(6000) // deeper than the 4,096 recorded scopes; the body still runs
        }
        XCTAssertLessThanOrEqual(deep.crashes.first?.frames.count ?? 999, 64)
        XCTAssertEqual(deep.crashes.count, 1)
        XCTAssertFalse(deep.crashes[0].frames.contains { $0.contains(":") })
    }

    func testACaughtScopeErrorLendsNoFramesToALaterUnrelatedError() throws {
        let r = try run { _, h in
            do { try h.scope("a") { throw FuzzSupport.Other() } } catch {}
            throw FuzzSupport.Boom()
        }
        XCTAssertEqual(r.crashes.first?.frames, [], "B must not inherit A's snapshot")
        // The same error escaping its scopes keeps its own frames.
        let s = try run { _, h in try h.scope("a") { throw FuzzSupport.Boom() } }
        XCTAssertEqual(s.crashes.first?.frames, ["a"])
    }

    func testLabelsAreTruncatedAtPushTime() throws {
        let r = try run { _, h in try h.scope(String(repeating: "L", count: 5_000_000)) { throw FuzzSupport.Boom() } }
        XCTAssertEqual(r.crashes.first?.frames, [String(repeating: "L", count: 128)])
    }

    func testBudgetSentinelsDoNotLeaveFrames() throws {
        let r = try run(step: 0) { _, h in try h.scope("a") { try h.tick() } }
        XCTAssertEqual(r.timeouts, 1)
        let h = Handle(stepBudget: 0, memBudget: 1)
        XCTAssertThrowsError(try h.scope("x") { try h.tick() })
        XCTAssertEqual(h.frames(for: FuzzSupport.Boom()), [])
    }
}
