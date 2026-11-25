// ============================================================================
// Handle.swift -- what a fuzz target sees (VER06l sections 4.2 and 5)
// ============================================================================
//
// A target receives ONE `Handle` per execution. It offers:
//
//   * `edge(id)`      report that a program point ran (the coverage feedback);
//   * `tick()`        count one unit of work; throws after `stepBudget` ticks;
//   * `reserve(n)`    declare an allocation of n bytes; throws past `memBudget`;
//   * `scope(l) {}`   push a frame label for crash grouping.
//
// THE HANDLE LIFETIME RULES (a security review of the C# lane found every one)
// ---------------------------------------------------------------------------
//   1. A FRESH handle is made for every execution.
//   2. The executor RETIRES it, under the handle's lock, as soon as the target
//      returns or throws. Afterwards `edge` does nothing, `tick`/`reserve`
//      throw a retired sentinel and `scope` records nothing. A `Thread`, a
//      dispatch block or a Swift `Task` that outlives the target can therefore
//      never touch a later run's budgets or coverage map.
//   3. Exhaustion is STICKY and SHARED: the first breach of either budget stores
//      its sentinel, and every later `tick`/`reserve` rethrows that same object.
//      Swift's `catch` swallows anything, so a target can ignore the error; the
//      executor therefore judges by the handle's state after the run, not by
//      what the target threw.
//   4. A sentinel is recognised ONLY by identity (`===`) with the object the
//      handle stored. A sentinel caught in an earlier run and rethrown in a
//      later one is the crash `stale-budget-sentinel`.

import Foundation

/// The error `tick` and `reserve` throw. An internal `final class`: outside code
/// cannot construct one, and even inside the module only the instance a handle
/// itself stored counts as that handle's budget breach (compared with `===`).
final class BudgetSentinel: Error, @unchecked Sendable {
    enum Which { case steps, memory, retired }

    let which: Which

    init(which: Which) { self.which = which }
}

public final class Handle: @unchecked Sendable {
    /// Deepest nesting of `scope` that records a label (deeper scopes still run).
    static let maxLabelDepth = 4096

    private let gate = NSLock()
    private var cov: Bitmap?
    private var retired = false
    private var steps = 0
    private var reserved = 0
    private var exhaustedSentinel: BudgetSentinel?
    private var labels: [String] = []
    private var snapshot: (frames: [String], depth: Int, kind: String)?

    let stepBudget: Int
    let memBudget: Int
    private let retiredSentinel: BudgetSentinel

    init(stepBudget: Int, memBudget: Int) {
        self.stepBudget = stepBudget
        self.memBudget = memBudget
        self.retiredSentinel = BudgetSentinel(which: .retired)
    }

    // MARK: engine side ---------------------------------------------------------------

    func attach(_ bitmap: Bitmap) {
        gate.lock(); defer { gate.unlock() }
        cov = bitmap
    }

    /// Ends the handle's life (the executor calls it right after the target).
    /// Taking the same lock as `edge` means no write to the coverage map can be
    /// in flight once this returns.
    func retire() {
        gate.lock(); defer { gate.unlock() }
        retired = true
        cov = nil
    }

    var isRetired: Bool {
        gate.lock(); defer { gate.unlock() }
        return retired
    }

    /// The sentinel of the FIRST breach of either budget, if any.
    var exhausted: BudgetSentinel? {
        gate.lock(); defer { gate.unlock() }
        return exhaustedSentinel
    }

    /// The label stack captured where `error` left the nested scopes (innermost
    /// first), or [] when the snapshot belongs to a DIFFERENT throw. Swift errors
    /// are values with no identity, so a snapshot is tagged with the error's
    /// dynamic type: an error caught
    /// outside the scopes and followed by an unrelated one cannot lend its frames.
    /// (Two throws of the same value type remain indistinguishable.)
    func frames(for error: Error) -> [String] {
        gate.lock(); defer { gate.unlock() }
        guard let s = snapshot, s.kind == String(reflecting: type(of: error)) else { return [] }
        return s.frames
    }

    var stepsUsed: Int {
        gate.lock(); defer { gate.unlock() }
        return steps
    }

    var bytesReserved: Int {
        gate.lock(); defer { gate.unlock() }
        return reserved
    }

    // MARK: target side ----------------------------------------------------------------

    /// A stable edge id derived from a name (FNV-1a), so a campaign is
    /// deterministic across runs and machines.
    public static func edgeId(_ label: String) -> UInt64 { fnv1a64(label) }

    /// Reports that the target executed the program point with this id (reduced mod 65,536).
    public func edge(_ id: UInt64) {
        gate.lock(); defer { gate.unlock() }
        cov?.record(id)
    }

    /// Counts one unit of work. After `stepBudget` ticks the execution is a
    /// timeout: `tick` throws a private sentinel and keeps throwing on every
    /// later call. A target that loops without ever ticking cannot be stopped
    /// (the engine starts no thread), so CI needs an external wall-clock limit.
    public func tick() throws {
        // The whole body runs under the lock, the retired check included: a stale
        // thread racing `retire()` either finishes before it or is refused after
        // it, and cannot update the counters of a handle the executor has read.
        gate.lock(); defer { gate.unlock() }
        if retired { throw retiredSentinel }
        if let e = exhaustedSentinel { throw e }
        if steps >= stepBudget {
            let s = BudgetSentinel(which: .steps)
            exhaustedSentinel = s
            throw s
        }
        steps += 1
    }

    /// Declares an allocation of `n` bytes about to be made on behalf of the
    /// input. When the running total of one execution exceeds `memBudget` the
    /// execution is an out-of-memory outcome. A non-positive `n` reserves
    /// nothing. Call it before `[UInt8](repeating: 0, count: n)` sized by input
    /// data. The overflow-safe test is `n > budget - reserved`.
    public func reserve(_ n: Int) throws {
        gate.lock(); defer { gate.unlock() }
        if retired { throw retiredSentinel }
        if let e = exhaustedSentinel { throw e }
        if n <= 0 { return }
        if n > memBudget - reserved {
            let s = BudgetSentinel(which: .memory)
            exhaustedSentinel = s
            throw s
        }
        reserved += n
    }

    /// Pushes `label` as a frame while `body` runs, for crash grouping (Swift
    /// errors carry no stack trace; see VER06l section 5.1). When an error
    /// leaves the scope, the FIRST scope to see it snapshots the label stack
    /// (innermost first); a scope that exits normally discards a snapshot taken
    /// inside it (the error was handled). On a retired handle the body just runs.
    /// Meant for single-threaded use.
    public func scope<T>(_ label: String, _ body: () throws -> T) rethrows -> T {
        let depth = push(label)
        do {
            let r = try body()
            pop(depth, failed: false, error: nil)
            return r
        } catch {
            pop(depth, failed: true, error: error)
            throw error
        }
    }

    private func push(_ label: String) -> Int? {
        gate.lock(); defer { gate.unlock() }
        if retired || labels.count >= Handle.maxLabelDepth { return nil }
        // Truncate at push time: a hostile megabyte label is stored as 128 bytes.
        labels.append(String(decoding: Array(label.utf8.prefix(FuzzSignature.maxFrameLen)), as: UTF8.self))
        return labels.count
    }

    private func pop(_ depth: Int?, failed: Bool, error: Error?) {
        guard let depth else { return }
        gate.lock(); defer { gate.unlock() }
        if !retired {
            if failed {
                if snapshot == nil, let error, !(error is BudgetSentinel) {
                    var out: [String] = []
                    var prev: String?
                    for raw in labels.reversed() where raw != prev {
                        prev = raw
                        out.append(FuzzSignature.normaliseFrame(raw))
                        if out.count >= FuzzSignature.maxFrames * 2 { break }
                    }
                    snapshot = (FuzzSignature.collapse(out), depth, String(reflecting: type(of: error)))
                }
            } else if let s = snapshot, s.depth >= depth {
                snapshot = nil
            }
        }
        if labels.count >= depth { labels.removeSubrange((depth - 1)...) }
    }
}
