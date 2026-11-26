// ============================================================================
// FuzzEngine.swift -- executor, classifier, minimiser and the campaign
// (VER06 sections 5 and 9.1, VER06l sections 5 and 6)
// ============================================================================
//
// A campaign is a pure function of (corpus, options, target): triage every seed,
// then repeatedly pick a seed, give it an energy-scaled number of mutated
// children, run each child and either admit it (new coverage), record it (crash,
// timeout, OOM) or drop it (redundant). Nothing here reads a clock, a file or the
// environment, and nothing starts a thread or a task.

/// The code under test. It receives its own copy of the input and a `Handle`.
/// Any error that escapes is a crash (the engine's own budget sentinels are the
/// exception: they are a timeout or an OOM).
public typealias Target = (_ data: [UInt8], _ h: Handle) throws -> Void

/// The code under test, as outcomes: runs `data`, reports edges into `cov`.
public typealias ExecFunc = (_ data: [UInt8], _ cov: Bitmap) -> Outcome

public enum FuzzEngine {
    /// Distinct crash signatures a report holds; further novel ones only increment `crashesDropped`.
    public static let maxCrashes = 1000
    /// Distinct over-cap signatures remembered so `crashesDropped` counts signatures, not hits.
    static let maxDroppedSignatures = 100_000
    /// Seeds accepted by one campaign.
    public static let maxInitialCorpus = 10_000
    /// Bytes of corpus seeds plus crash inputs one campaign retains (VER06l section 6).
    public static let maxRetainedBytes = 256 << 20
    /// Crash kind of a sentinel that escaped into a run it does not belong to.
    static let staleSentinel = "stale-budget-sentinel"

    // MARK: the executor -------------------------------------------------------------

    /// Runs a bounded coverage-guided campaign against `target`. The coverage
    /// tier is `caller-reported` when the target called `edge` at least once
    /// during the campaign, `black-box-uninstrumented` otherwise.
    public static func run(corpus: [[UInt8]], options: FuzzOptions, target: @escaping Target) throws -> FuzzReport {
        try options.validate()
        return try runWithCap(corpus: corpus, options: options, target: target, retainedCap: maxRetainedBytes)
    }

    static func runWithCap(corpus: [[UInt8]], options: FuzzOptions, target: @escaping Target, retainedCap: Int) throws -> FuzzReport {
        return try runOutcomes(corpus: corpus, options: options, retainedCap: retainedCap) { data, cov in
            // A FRESH handle per execution, retired by `runTarget`: a stale thread
            // or task can never touch a later run's budgets or coverage map.
            let h = Handle(stepBudget: options.stepBudget, memBudget: options.memBudget)
            h.attach(cov)
            return runTarget(target, data, h)
        }
    }

    /// Runs ONE execution and judges it.
    ///
    /// The order matters:
    ///   1. the input is physically copied (an engine failure here is not the target's);
    ///   2. the target runs inside the only `do/catch`;
    ///   3. the handle is RETIRED (from here no stale thread can change it);
    ///   4. only then is the handle's exhaustion state read: a target that
    ///      swallowed the sentinel and returned, or then threw something
    ///      ordinary, is still a timeout or an OOM, never a pass or a crash.
    static func runTarget(_ target: Target, _ data: [UInt8], _ h: Handle) -> Outcome {
        let copy = data.withUnsafeBufferPointer { Array($0) }
        var thrown: Error?
        do {
            try target(copy, h)
        } catch {
            thrown = error
        }
        h.retire()
        if let ex = h.exhausted { return ex.which == .steps ? .timeout : .oom }
        guard let t = thrown else { return .pass }
        return classify(t, h)
    }

    /// Turns a thrown error into an outcome.
    ///
    /// Only the error's DYNAMIC TYPE NAME is read (`String(reflecting:)` of the
    /// metatype). `description`, `localizedDescription` and every other member of
    /// the error are never called: a message may carry input bytes, and a hostile
    /// conformance could run arbitrary code.
    static func classify(_ error: Error, _ h: Handle) -> Outcome {
        if let b = error as? BudgetSentinel {
            // Our own sentinel is recognised by IDENTITY with the handle's stored one.
            if let ex = h.exhausted, b === ex { return ex.which == .steps ? .timeout : .oom }
            // One caught in an earlier run (or built elsewhere) and rethrown now.
            return .crash(kind: staleSentinel, frames: [])
        }
        return .crash(kind: FuzzSignature.normaliseKind(String(reflecting: type(of: error))), frames: h.frames(for: error))
    }

    // MARK: minimisation (VER06 section 5.3) -------------------------------------------

    private static func sameCrash(_ o: Outcome, _ sig: String) -> Bool {
        if case .crash(let kind, let frames) = o { return FuzzSignature.crashSignature(kind: kind, frames: frames) == sig }
        return false
    }

    /// Shrinks `data` while it keeps the exact signature. Every candidate
    /// execution, accepted or rejected, is counted in `spent`, and the budget is
    /// checked before each one, so no stream of rejected candidates can run past it.
    static func minimize(_ data: [UInt8], _ sig: String, _ test: ([UInt8]) -> Outcome, _ budget: Int)
        -> (data: [UInt8], spent: Int, finished: Bool) {
        var cur = data
        var spent = 0
        if cur.isEmpty { return (cur, 0, true) }
        // Pass 1: delete blocks, halving the block size. Starting at max(1, len/2)
        // (not len/2) is what makes the empty input reachable from a 1-byte crasher.
        var chunk = max(1, cur.count / 2)
        while chunk > 0 {
            var offset = 0
            while offset < cur.count {
                if spent >= budget { return (cur, spent, false) }
                let end = offset + min(chunk, cur.count - offset)
                var candidate = cur
                candidate.removeSubrange(offset..<end)
                spent += 1
                if sameCrash(test(candidate), sig) {
                    cur = candidate // same offset again: the tail slid left
                } else {
                    offset += chunk
                }
            }
            chunk /= 2
        }
        // Pass 2: zero individual non-essential bytes.
        var i = 0
        while i < cur.count {
            if cur[i] != 0 {
                if spent >= budget { return (cur, spent, false) }
                let original = cur[i]
                cur[i] = 0
                spent += 1
                if !sameCrash(test(cur), sig) { cur[i] = original }
            }
            i += 1
        }
        return (cur, spent, true)
    }

    /// Shrinks `data` while it keeps the EXACT crash signature: pass 1 deletes
    /// blocks, pass 2 zeroes bytes. A candidate that crashes differently is
    /// rejected, so minimisation never trades one bug for another. `budget` caps
    /// the executions of `test`; it must be in [0, `FuzzOptions.maxMinimizeBudget`]
    /// and `data` at most `Mutate.maxInputBytes`.
    public static func minimizeCrashingInput(_ data: [UInt8], sig: String, test: ([UInt8]) -> Outcome, budget: Int) throws -> [UInt8] {
        if budget < 0 || budget > FuzzOptions.maxMinimizeBudget {
            throw FuzzError.invalidArgument("budget must be in [0, \(FuzzOptions.maxMinimizeBudget)]")
        }
        if data.count > Mutate.maxInputBytes { throw FuzzError.invalidArgument("data exceeds \(Mutate.maxInputBytes) bytes") }
        return minimize(data, sig, test, budget).data
    }

    // MARK: the campaign ----------------------------------------------------------------

    private struct SeedEntry {
        let data: [UInt8]
        let newEdges: Int
        let generation: Int
    }

    private final class Campaign {
        let exec: ExecFunc
        let opts: FuzzOptions
        let retainedCap: Int
        var rng: SplitMix64
        let step = Bitmap()
        let cumulative = Bitmap()
        var corpus: [SeedEntry] = []
        var crashes: [CrashRecord] = []
        var bySig: [String: Int] = [:]
        var timeouts = 0
        var ooms = 0
        var dropped = 0
        /// Distinct signatures refused by the cap (bounded: past it each counts once, an upper bound).
        var droppedSigs = Set<String>()
        var admissionsDropped = 0
        var minimizeLeft: Int
        var retained = 0

        init(exec: @escaping ExecFunc, opts: FuzzOptions, retainedCap: Int) {
            self.exec = exec
            self.opts = opts
            self.retainedCap = retainedCap
            self.minimizeLeft = opts.maxIterations
            self.rng = SplitMix64(seed: opts.seed)
        }

        /// A non-throwing bounded draw (n is never 0 where it is used).
        func draw(_ n: Int) -> Int { Int((try? rng.bounded(UInt64(max(n, 1)))) ?? 0) }

        /// Runs one input; the edges of the run are left in `step`.
        func execute(_ data: [UInt8]) -> Outcome {
            step.reset()
            return exec(data, step)
        }

        /// Puts one non-passing execution in its bucket.
        func file(_ output: Outcome, _ data: [UInt8]) {
            switch output {
            case .timeout: timeouts += 1
            case .oom: ooms += 1
            case .crash(let kind, let frames): recordCrash(kind, frames, data)
            case .pass: break
            }
        }

        /// Puts a passing input in the corpus iff it covered a new edge. The first
        /// passing seed is always admitted, even with no coverage, so there is
        /// something to mutate in black-box mode.
        func admit(_ data: [UInt8], generation: Int) {
            let a = cumulative.admit(step.trace())
            if a.hasNew || (generation == 0 && corpus.isEmpty) {
                if data.count > retainedCap - retained {
                    admissionsDropped += 1
                    return
                }
                retained += data.count
                corpus.append(SeedEntry(data: data, newEdges: a.countNew, generation: generation))
            }
        }

        private func recordCrash(_ kind: String, _ frames: [String], _ data: [UInt8]) {
            let sig = FuzzSignature.crashSignature(kind: kind, frames: frames)
            if let at = bySig[sig] {
                let rec = crashes[at]
                rec.hit()
                if !rec.inputDropped && data.count < rec.minimalLength {
                    retained -= rec.minimalLength - data.count
                    rec.replaceMinimal(data)
                }
                return
            }
            if crashes.count >= FuzzEngine.maxCrashes {
                if droppedSigs.contains(sig) { return }
                if droppedSigs.count < FuzzEngine.maxDroppedSignatures { droppedSigs.insert(sig) }
                dropped += 1
                return
            }
            let allowance = min(opts.minimizeBudget, minimizeLeft)
            let m = FuzzEngine.minimize(data, sig, { self.execute($0) }, allowance)
            minimizeLeft -= m.spent
            var kept = m.data
            var droppedInput = false
            if kept.count > retainedCap - retained {
                kept = []
                droppedInput = true
            } else {
                retained += kept.count
            }
            bySig[sig] = crashes.count
            crashes.append(CrashRecord(signature: sig, kind: kind, frames: frames, minimal: kept,
                                       minimized: m.finished && !droppedInput, inputDropped: droppedInput))
        }
    }

    /// Runs a bounded coverage-guided campaign over `exec`.
    ///
    /// Contracts (VER06 section 9.1), each pinned by a test:
    ///   * a pure function of (corpus, options, exec): same inputs, same report;
    ///   * with a live corpus `iterations == options.maxIterations` exactly, and
    ///     no mutated candidate exceeds `options.maxInputLen` (initial seeds run
    ///     as given, up to `Mutate.maxInputBytes` each);
    ///   * an empty corpus is replaced by one `00` seed; seeds that crash, time
    ///     out or exhaust memory are never mutated, so a corpus of crashers
    ///     fuzzes nothing (iterations == 0);
    ///   * every execution lands in crashes, timeouts, ooms or the corpus;
    ///     `isClean` needs at least one iteration and no failure at all;
    ///   * one `CrashRecord` per signature; a novel crash is minimised.
    ///
    /// Per iteration the stream draws: `bounded(len(corpus))` for the seed; per
    /// energy slot `1 + bounded(4)` steps, one `nextU64` sub-seed, then, if
    /// `len(corpus) > 1`, `bounded(4)` and on 0 a donor index and two split draws
    /// (as the Go, Rust, Java and C# lanes).
    public static func runOutcomes(corpus initialCorpus: [[UInt8]], options opts: FuzzOptions, exec: @escaping ExecFunc) throws -> FuzzReport {
        try runOutcomes(corpus: initialCorpus, options: opts, retainedCap: maxRetainedBytes, exec: exec)
    }

    static func runOutcomes(corpus initialCorpus: [[UInt8]], options opts: FuzzOptions, retainedCap: Int,
                            exec: @escaping ExecFunc) throws -> FuzzReport {
        try opts.validate()
        if initialCorpus.count > maxInitialCorpus {
            throw FuzzError.invalidArgument("initial corpus exceeds \(maxInitialCorpus) seeds")
        }
        for (i, s) in initialCorpus.enumerated() where s.count > Mutate.maxInputBytes {
            throw FuzzError.invalidArgument("initial corpus seed \(i) exceeds \(Mutate.maxInputBytes) bytes")
        }
        let seeds = initialCorpus.isEmpty ? [[UInt8(0)]] : initialCorpus
        let c = Campaign(exec: exec, opts: opts, retainedCap: retainedCap)

        // 1. Triage: run every seed once.
        for data in seeds {
            let output = c.execute(data)
            if output == .pass { c.admit(data, generation: 0) } else { c.file(output, data) }
        }

        // 2. The mutation loop.
        var iterations = 0
        while iterations < opts.maxIterations && !c.corpus.isEmpty {
            let seed = c.corpus[c.draw(c.corpus.count)]
            let energy = Bitmap.calculateEnergy(base: opts.baseEnergy, seedLen: seed.data.count,
                                                newEdges: seed.newEdges, totalEdges: c.cumulative.count)
            var slot = 0
            while slot < energy && iterations < opts.maxIterations {
                slot += 1
                iterations += 1
                let steps = 1 + c.draw(4)
                let sub = c.rng.nextU64()
                // steps <= 4 and the seed is at most 1 MiB, so neither cap can trip.
                var candidate = (try? Mutate.mutateSequence(seed.data, seed: sub, steps: steps, maxLen: Mutate.maxInputBytes)) ?? seed.data
                if c.corpus.count > 1 && c.draw(4) == 0 {
                    let donor = c.corpus[c.draw(c.corpus.count)].data
                    let ts = UInt64(c.draw(candidate.count + 1))
                    let ds = UInt64(c.draw(donor.count + 1))
                    candidate = Mutate.splice(candidate, donor, targetSplit: ts, donorSplit: ds)
                }
                if candidate.count > opts.maxInputLen { candidate.removeSubrange(opts.maxInputLen...) }
                let output = c.execute(candidate)
                if output == .pass { c.admit(candidate, generation: seed.generation + 1) } else { c.file(output, candidate) }
            }
        }

        let total = c.cumulative.count
        return FuzzReport(iterations: iterations, admittedSeeds: c.corpus.count, totalEdges: total, crashes: c.crashes,
                          timeouts: c.timeouts, ooms: c.ooms, crashesDropped: c.dropped,
                          admissionsDropped: c.admissionsDropped,
                          coverageTier: total > 0 ? FuzzReport.tierReported : FuzzReport.tierBlackBox,
                          corpus: c.corpus.map { $0.data })
    }
}
