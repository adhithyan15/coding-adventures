// ============================================================================
// Runner.swift -- the honest runner (VER05 section 7.1, VER05e section 5)
// ============================================================================
//
// VERDICT HONESTY: the runner never says "pass" unless every case was really
// evaluated and held.
//
//   * zero cases evaluated            -> property-exhausted, never a pass;
//   * a generator error               -> property-exhausted;
//   * a property that THROWS          -> property-panic;
//   * a property that returns false   -> shrink, then property-counterexample;
//   * the reported witness is RE-VERIFIED: if the shrinker returns something
//     that no longer fails, the original candidate is reported with 0 steps;
//   * a nested check starved by the budget is exhausted, never a pass;
//   * redacted (CI) verdicts carry no seed, witness or minimal.
//
// TRAPS: Swift cannot catch a trap (array index out of range, arithmetic
// overflow, force-unwrap of nil, `fatalError`, stack overflow). A property that
// traps crashes the whole test process. That is a property of the language, not
// a verdict this engine can produce: write properties that return `false` or
// `throw` rather than trap.

public enum Runner {
    /// Runs `property` over `options.numTests` candidates drawn from
    /// `SplitMix64(seed)`. `property` returns `true` when the property HOLDS; a
    /// thrown error is a property-panic. `shrinker` (optional) overrides the
    /// generator's own, so a defective shrinker can be tested. Invalid options
    /// throw `PbtError.invalidArgument` (no verdict is produced).
    public static func check<T>(
        seed: UInt64, gen: Gen<T>, options: Options = Options(), shrinker: Shrinker<T>? = nil,
        property: @escaping (T) throws -> Bool
    ) throws -> Verdict {
        let o = options
        if o.numTests < 0 || o.numTests > Limits.maxCases {
            throw PbtError.invalidArgument("numTests \(o.numTests) outside [0, \(Limits.maxCases)]")
        }
        if o.maxShrinkSteps < 0 || o.maxShrinkSteps > Limits.maxShrinkSteps {
            throw PbtError.invalidArgument("maxShrinkSteps \(o.maxShrinkSteps) outside [0, \(Limits.maxShrinkSteps)]")
        }
        let redact = o.ci != .off

        if o.numTests == 0 {
            return Verdict(passed: false, failureId: FailureIds.exhausted,
                           message: "no cases were evaluated (numTests is 0); a property that never ran is not a pass",
                           numTests: 0, iterations: 0, shrinkSteps: 0, seed: nil, initial: nil, minimal: nil, digest: nil)
        }

        // A check nested in a property (an ambient scope exists) pays one attempt
        // at entry, one per case after the first and the size of every sampled
        // collection, all charged up the chain: neither nesting depth, branching
        // nor numTests multiplies the outermost budget. A starved nested check
        // reports property-exhausted without evaluating anything. A top-level
        // check always evaluates its first case.
        let outer = ShrinkBudget.current()
        let nested = outer != nil
        if let outer, !outer.charge(1) { return spent(0) }

        // The WHOLE check runs in one scope (case loop included).
        let scope = ShrinkBudget.enterCheck(numTests: o.numTests, maxSteps: o.maxShrinkSteps)
        defer { scope.close() }
        var rng = Hooks.newRng(seed)
        for iteration in 1...o.numTests {
            if nested && iteration > 1 && !scope.charge(1) { return spent(iteration) }
            let candidate: T
            do {
                candidate = try gen.sample(&rng)
            } catch {
                return exhausted(error, iteration, redact)
            }
            if nested && !scope.chargeCopy(sizeOf(candidate)) { return spent(iteration) }
            let holds: Bool
            do {
                holds = try property(candidate)
            } catch {
                return panic(seed, iteration, error, redact)
            }
            if !holds {
                return falsified(seed, iteration, candidate, gen, property, shrinker, o, redact, scope)
            }
        }
        if !nested && scope.wasRefused {
            // A nested check or generator inside this run was starved, so the
            // property was not fully evaluated: never report that as a pass.
            return Verdict(passed: false, failureId: FailureIds.exhausted,
                           message: "a nested check was starved by the shrink budget; the property was not fully evaluated",
                           numTests: o.numTests, iterations: o.numTests, shrinkSteps: 0,
                           seed: nil, initial: nil, minimal: nil, digest: nil)
        }
        return Verdict(passed: true, failureId: nil, message: nil, numTests: o.numTests,
                       iterations: 0, shrinkSteps: 0, seed: nil, initial: nil, minimal: nil, digest: nil)
    }

    /// The enclosing shrink budget was spent: exhausted, never a pass.
    private static func spent(_ iteration: Int) -> Verdict {
        Verdict(passed: false, failureId: FailureIds.exhausted,
                message: "the enclosing shrink budget was spent at iteration \(iteration)",
                numTests: max(0, iteration - 1), iterations: iteration, shrinkSteps: 0,
                seed: nil, initial: nil, minimal: nil, digest: nil)
    }

    private static let walkCap = 10_000

    /// An approximate size of a sampled value in elements (work): a collection
    /// counts its length plus, one level deeper, the length of each element that
    /// is itself a collection or string (visiting at most `walkCap` elements).
    /// Generation work is approximate by design; a scalar counts 0.
    static func sizeOf(_ v: Any) -> Int {
        let n = shallow(v)
        if n == 0 { return 0 }
        if v is [UInt8] || v is String { return n }
        var total = n
        var visited = 0
        for child in Mirror(reflecting: v).children {
            if visited >= walkCap { break }
            visited += 1
            total += shallow(child.value)
        }
        return total
    }

    private static func shallow(_ v: Any) -> Int {
        if let b = v as? [UInt8] { return b.count }
        if let s = v as? String { return s.utf8.count }
        let m = Mirror(reflecting: v)
        switch m.displayStyle {
        case .collection, .set, .dictionary: return m.children.count
        default: return 0
        }
    }

    private static func exhausted(_ e: Error, _ iteration: Int, _ redact: Bool) -> Verdict {
        let text: String
        if e is ExhaustedError {
            text = "filter precondition rejected all candidates at iteration \(iteration)"
        } else if redact {
            text = "generator error at iteration \(iteration) (\(String(reflecting: type(of: e))))"
        } else {
            text = "generator error at iteration \(iteration): \(clip(Canon.render(e), 200))"
        }
        return Verdict(passed: false, failureId: FailureIds.exhausted, message: text,
                       numTests: iteration - 1, iterations: iteration, shrinkSteps: 0,
                       seed: nil, initial: nil, minimal: nil, digest: nil)
    }

    private static func panic(_ seed: UInt64, _ iteration: Int, _ e: Error, _ redact: Bool) -> Verdict {
        if redact {
            return Verdict(passed: false, failureId: FailureIds.panic,
                           message: "property evaluation threw at iteration \(iteration) (\(String(reflecting: type(of: e))))",
                           numTests: iteration, iterations: iteration, shrinkSteps: 0,
                           seed: nil, initial: nil, minimal: nil, digest: nil)
        }
        return Verdict(passed: false, failureId: FailureIds.panic,
                       message: "property evaluation threw at iteration \(iteration): \(clip(Canon.render(e), 200))",
                       numTests: iteration, iterations: iteration, shrinkSteps: 0,
                       seed: seed, initial: nil, minimal: nil, digest: nil)
    }

    private static func falsified<T>(
        _ seed: UInt64, _ iteration: Int, _ candidate: T, _ gen: Gen<T>,
        _ property: @escaping (T) throws -> Bool, _ shrinker: Shrinker<T>?, _ o: Options,
        _ redact: Bool, _ checkScope: ShrinkBudget
    ) -> Verdict {
        // A candidate the property throws on is not a counterexample to the
        // claim; it is a different failure, so it is never accepted as "smaller".
        let falsifies: (T) -> Bool = { v in
            guard let holds = try? property(v) else { return false }
            return !holds
        }
        var minimal = candidate
        var steps = 0
        // The shrink scope is a child of the check scope (itself a child of any
        // enclosing shrink): every evaluation is charged to the whole chain.
        let shrunk: Shrunk<T>
        do {
            let scope = ShrinkBudget.enterChild(maxSteps: o.maxShrinkSteps)
            defer { scope.close() }
            let budgeted = scope.charged(falsifies)
            if let shrinker {
                shrunk = shrinker(candidate, budgeted, o.maxShrinkSteps)
            } else {
                shrunk = gen.shrinkNested(candidate, budgeted, o.maxShrinkSteps)
            }
        }
        // Re-verify: the witness we report must really fail the property. One
        // attempt is charged to the check scope (not the spent shrink scope); if
        // the chain refuses it, the shrunk value is not adopted.
        if checkScope.charge(1) && shrunk.steps >= 0 && shrunk.steps <= o.maxShrinkSteps && falsifies(shrunk.value) {
            minimal = shrunk.value
            steps = shrunk.steps
        }

        if redact {
            let preimage = "iter:\(iteration);steps:\(steps);shrunk:\(Canon.render(minimal))"
            let hex = String(fnv1a64(preimage), radix: 16)
            let digest = String(repeating: "0", count: 16 - hex.count) + hex
            return Verdict(passed: false, failureId: FailureIds.counterexample,
                           message: "property falsified after \(iteration) iterations (shrunk in \(steps) steps, repro-fnv: \(digest))",
                           numTests: iteration, iterations: iteration, shrinkSteps: steps,
                           seed: nil, initial: nil, minimal: nil, digest: digest)
        }
        return Verdict(passed: false, failureId: FailureIds.counterexample,
                       message: "property falsified at iteration \(iteration) (seed=\(seed)): initial=\(Canon.render(candidate)), minimal=\(Canon.render(minimal)) after \(steps) shrink steps",
                       numTests: iteration, iterations: iteration, shrinkSteps: steps,
                       seed: seed, initial: candidate, minimal: minimal, digest: nil)
    }
}
