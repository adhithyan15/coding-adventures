/// Tests for the StateMachine package: DFA and NFA implementations.
///
/// These tests cover:
/// 1. DFA construction and validation
/// 2. DFA processing single events and sequences
/// 3. DFA acceptance checking
/// 4. DFA introspection (reachability, completeness, validation)
/// 5. DFA reset
/// 6. DFA error cases
/// 7. Classic examples (turnstile, binary div-by-3, branch predictor)
/// 8. NFA construction and validation
/// 9. NFA epsilon closure computation
/// 10. NFA processing (non-deterministic branching)
/// 11. NFA acceptance checking
/// 12. NFA subset construction (NFA -> DFA conversion)
/// 13. NFA reset

import XCTest
@testable import StateMachine

// ============================================================
// Helper — reusable DFA definitions
// ============================================================

/// The classic turnstile: insert coin to unlock, push to lock.
///
///     locked --coin--> unlocked
///     locked --push--> locked
///     unlocked --coin--> unlocked
///     unlocked --push--> locked
///
/// Initial: locked. Accepting: {unlocked}.
func makeTurnstile() throws -> DFA {
    try DFA(
        states: ["locked", "unlocked"],
        alphabet: ["coin", "push"],
        transitions: [
            DFA.TransitionRule(from: "locked", on: "coin", to: "unlocked"),
            DFA.TransitionRule(from: "locked", on: "push", to: "locked"),
            DFA.TransitionRule(from: "unlocked", on: "coin", to: "unlocked"),
            DFA.TransitionRule(from: "unlocked", on: "push", to: "locked"),
        ],
        initial: "locked",
        accepting: ["unlocked"]
    )
}

/// DFA that accepts binary strings representing numbers divisible by 3.
///
/// States represent the remainder when divided by 3:
///   r0 = remainder 0 (divisible by 3) — accepting
///   r1 = remainder 1
///   r2 = remainder 2
///
/// Transition logic: new_remainder = (old_remainder * 2 + bit) mod 3
func makeDivBy3() throws -> DFA {
    try DFA(
        states: ["r0", "r1", "r2"],
        alphabet: ["0", "1"],
        transitions: [
            DFA.TransitionRule(from: "r0", on: "0", to: "r0"),  // (0*2+0) mod 3 = 0
            DFA.TransitionRule(from: "r0", on: "1", to: "r1"),  // (0*2+1) mod 3 = 1
            DFA.TransitionRule(from: "r1", on: "0", to: "r2"),  // (1*2+0) mod 3 = 2
            DFA.TransitionRule(from: "r1", on: "1", to: "r0"),  // (1*2+1) mod 3 = 0
            DFA.TransitionRule(from: "r2", on: "0", to: "r1"),  // (2*2+0) mod 3 = 1
            DFA.TransitionRule(from: "r2", on: "1", to: "r2"),  // (2*2+1) mod 3 = 2
        ],
        initial: "r0",
        accepting: ["r0"]
    )
}

/// 2-bit saturating counter branch predictor as a DFA.
///
/// States: SNT (strongly not-taken), WNT (weakly not-taken),
///         WT (weakly taken), ST (strongly taken)
///
/// This is a real hardware pattern used in CPU branch prediction.
/// The accepting states {WT, ST} are the ones that predict "taken".
func makeBranchPredictor() throws -> DFA {
    try DFA(
        states: ["SNT", "WNT", "WT", "ST"],
        alphabet: ["taken", "not_taken"],
        transitions: [
            DFA.TransitionRule(from: "SNT", on: "taken", to: "WNT"),
            DFA.TransitionRule(from: "SNT", on: "not_taken", to: "SNT"),
            DFA.TransitionRule(from: "WNT", on: "taken", to: "WT"),
            DFA.TransitionRule(from: "WNT", on: "not_taken", to: "SNT"),
            DFA.TransitionRule(from: "WT", on: "taken", to: "ST"),
            DFA.TransitionRule(from: "WT", on: "not_taken", to: "WNT"),
            DFA.TransitionRule(from: "ST", on: "taken", to: "ST"),
            DFA.TransitionRule(from: "ST", on: "not_taken", to: "WT"),
        ],
        initial: "WNT",
        accepting: ["WT", "ST"]
    )
}

// ============================================================
// Helper — reusable NFA definitions
// ============================================================

/// NFA that accepts strings containing "ab" as a substring.
///
/// State q0 is the "scanning" state — it loops on any input.
/// When an 'a' is seen, the NFA non-deterministically spawns a
/// branch to q1 (while also staying in q0). If q1 sees 'b', it
/// moves to q2 (accepting). q2 loops on any input.
func makeContainsAb() throws -> NFA {
    try NFA(
        states: ["q0", "q1", "q2"],
        alphabet: ["a", "b"],
        transitions: [
            NFATransitionRule(from: "q0", on: "a", to: ["q0", "q1"]),
            NFATransitionRule(from: "q0", on: "b", to: ["q0"]),
            NFATransitionRule(from: "q1", on: "b", to: ["q2"]),
            NFATransitionRule(from: "q2", on: "a", to: ["q2"]),
            NFATransitionRule(from: "q2", on: "b", to: ["q2"]),
        ],
        initial: "q0",
        accepting: ["q2"]
    )
}

/// NFA with a chain of epsilon transitions: q0 --e--> q1 --e--> q2.
///
/// The only real transition is q2 --a--> q3 (accepting).
/// So the NFA starts in {q0, q1, q2} and accepts exactly "a".
func makeEpsilonChain() throws -> NFA {
    try NFA(
        states: ["q0", "q1", "q2", "q3"],
        alphabet: ["a"],
        transitions: [
            NFATransitionRule(from: "q0", on: EPSILON, to: ["q1"]),
            NFATransitionRule(from: "q1", on: EPSILON, to: ["q2"]),
            NFATransitionRule(from: "q2", on: "a", to: ["q3"]),
        ],
        initial: "q0",
        accepting: ["q3"]
    )
}

/// NFA that accepts "a" or "ab" using epsilon transitions.
///
///     q0 --e--> q1 --a--> q2 (accept)
///     q0 --e--> q3 --a--> q4 --b--> q5 (accept)
func makeAOrAb() throws -> NFA {
    try NFA(
        states: ["q0", "q1", "q2", "q3", "q4", "q5"],
        alphabet: ["a", "b"],
        transitions: [
            NFATransitionRule(from: "q0", on: EPSILON, to: ["q1", "q3"]),
            NFATransitionRule(from: "q1", on: "a", to: ["q2"]),
            NFATransitionRule(from: "q3", on: "a", to: ["q4"]),
            NFATransitionRule(from: "q4", on: "b", to: ["q5"]),
        ],
        initial: "q0",
        accepting: ["q2", "q5"]
    )
}

// ============================================================
// DFA Construction and Validation Tests
// ============================================================

final class DFAConstructionTests: XCTestCase {

    func testConstructValidDFA() throws {
        let t = try makeTurnstile()
        XCTAssertEqual(t.currentState, "locked")
        XCTAssertEqual(t.initial, "locked")
        XCTAssertEqual(t.states, ["locked", "unlocked"])
        XCTAssertEqual(t.alphabet, ["coin", "push"])
        XCTAssertEqual(t.accepting, ["unlocked"])
    }

    func testRejectEmptyStates() {
        XCTAssertThrowsError(
            try DFA(
                states: [],
                alphabet: ["a"],
                transitions: [],
                initial: "q0",
                accepting: []
            )
        ) { error in
            guard let dfaError = error as? DFAError else {
                XCTFail("Expected DFAError"); return
            }
            XCTAssertEqual(dfaError, .emptyStates)
        }
    }

    func testRejectInitialStateNotInStates() {
        XCTAssertThrowsError(
            try DFA(
                states: ["q0", "q1"],
                alphabet: ["a"],
                transitions: [DFA.TransitionRule(from: "q0", on: "a", to: "q1")],
                initial: "q_missing",
                accepting: []
            )
        ) { error in
            guard let dfaError = error as? DFAError else {
                XCTFail("Expected DFAError"); return
            }
            XCTAssertEqual(dfaError, .initialStateNotInStates(state: "q_missing"))
        }
    }

    func testRejectAcceptingStateNotInStates() {
        XCTAssertThrowsError(
            try DFA(
                states: ["q0", "q1"],
                alphabet: ["a"],
                transitions: [DFA.TransitionRule(from: "q0", on: "a", to: "q1")],
                initial: "q0",
                accepting: ["q_missing"]
            )
        ) { error in
            guard let dfaError = error as? DFAError else {
                XCTFail("Expected DFAError"); return
            }
            XCTAssertEqual(dfaError, .acceptingStateNotInStates(state: "q_missing"))
        }
    }

    func testRejectTransitionSourceNotInStates() {
        XCTAssertThrowsError(
            try DFA(
                states: ["q0"],
                alphabet: ["a"],
                transitions: [DFA.TransitionRule(from: "q_bad", on: "a", to: "q0")],
                initial: "q0",
                accepting: []
            )
        ) { error in
            guard let dfaError = error as? DFAError else {
                XCTFail("Expected DFAError"); return
            }
            XCTAssertEqual(dfaError, .transitionSourceNotInStates(source: "q_bad"))
        }
    }

    func testRejectTransitionEventNotInAlphabet() {
        XCTAssertThrowsError(
            try DFA(
                states: ["q0"],
                alphabet: ["a"],
                transitions: [DFA.TransitionRule(from: "q0", on: "b", to: "q0")],
                initial: "q0",
                accepting: []
            )
        ) { error in
            guard let dfaError = error as? DFAError else {
                XCTFail("Expected DFAError"); return
            }
            XCTAssertEqual(dfaError, .transitionEventNotInAlphabet(event: "b"))
        }
    }

    func testRejectTransitionTargetNotInStates() {
        XCTAssertThrowsError(
            try DFA(
                states: ["q0"],
                alphabet: ["a"],
                transitions: [DFA.TransitionRule(from: "q0", on: "a", to: "q_bad")],
                initial: "q0",
                accepting: []
            )
        ) { error in
            guard let dfaError = error as? DFAError else {
                XCTFail("Expected DFAError"); return
            }
            XCTAssertEqual(
                dfaError,
                .transitionTargetNotInStates(target: "q_bad", source: "q0", event: "a")
            )
        }
    }

    func testRejectActionWithoutTransition() {
        XCTAssertThrowsError(
            try DFA(
                states: ["q0"],
                alphabet: ["a"],
                transitions: [DFA.TransitionRule(from: "q0", on: "a", to: "q0")],
                initial: "q0",
                accepting: [],
                actions: [transitionKey("q0", "b"): { _, _, _ in }]
            )
        ) { error in
            guard let dfaError = error as? DFAError else {
                XCTFail("Expected DFAError"); return
            }
            XCTAssertEqual(dfaError, .actionWithoutTransition(source: "q0", event: "b"))
        }
    }

    func testAllowEmptyAcceptingSet() throws {
        let dfa = try DFA(
            states: ["q0"],
            alphabet: ["a"],
            transitions: [DFA.TransitionRule(from: "q0", on: "a", to: "q0")],
            initial: "q0",
            accepting: []
        )
        XCTAssertTrue(dfa.accepting.isEmpty)
    }
}

// ============================================================
// DFA Processing Tests
// ============================================================

final class DFAProcessingTests: XCTestCase {

    func testProcessSingleEvent() throws {
        let t = try makeTurnstile()
        let result = try t.process("coin")
        XCTAssertEqual(result, "unlocked")
        XCTAssertEqual(t.currentState, "unlocked")
    }

    func testProcessMultipleEventsSequentially() throws {
        let t = try makeTurnstile()
        try t.process("coin")
        XCTAssertEqual(t.currentState, "unlocked")
        try t.process("push")
        XCTAssertEqual(t.currentState, "locked")
        try t.process("coin")
        XCTAssertEqual(t.currentState, "unlocked")
        try t.process("coin")
        XCTAssertEqual(t.currentState, "unlocked")
    }

    func testBuildTraceFromProcessCalls() throws {
        let t = try makeTurnstile()
        try t.process("coin")
        try t.process("push")

        let trace = t.trace
        XCTAssertEqual(trace.count, 2)
        XCTAssertEqual(trace[0].source, "locked")
        XCTAssertEqual(trace[0].event, "coin")
        XCTAssertEqual(trace[0].target, "unlocked")
        XCTAssertNil(trace[0].actionName)
        XCTAssertEqual(trace[1].source, "unlocked")
        XCTAssertEqual(trace[1].event, "push")
        XCTAssertEqual(trace[1].target, "locked")
        XCTAssertNil(trace[1].actionName)
    }

    func testProcessSequenceAndReturnTrace() throws {
        let t = try makeTurnstile()
        let trace = try t.processSequence(["coin", "push", "coin"])
        XCTAssertEqual(trace.count, 3)
        XCTAssertEqual(trace[0].source, "locked")
        XCTAssertEqual(trace[0].target, "unlocked")
        XCTAssertEqual(trace[1].source, "unlocked")
        XCTAssertEqual(trace[1].target, "locked")
        XCTAssertEqual(trace[2].source, "locked")
        XCTAssertEqual(trace[2].target, "unlocked")
    }

    func testReturnEmptyTraceForEmptySequence() throws {
        let t = try makeTurnstile()
        let trace = try t.processSequence([])
        XCTAssertTrue(trace.isEmpty)
        XCTAssertEqual(t.currentState, "locked")
    }

    func testThrowOnInvalidEvent() throws {
        let t = try makeTurnstile()
        XCTAssertThrowsError(try t.process("kick")) { error in
            guard let dfaError = error as? DFAError else {
                XCTFail("Expected DFAError"); return
            }
            XCTAssertEqual(dfaError, .eventNotInAlphabet(event: "kick"))
        }
    }

    func testThrowOnUndefinedTransition() throws {
        let dfa = try DFA(
            states: ["q0", "q1"],
            alphabet: ["a", "b"],
            transitions: [DFA.TransitionRule(from: "q0", on: "a", to: "q1")],
            initial: "q0",
            accepting: []
        )
        XCTAssertThrowsError(try dfa.process("b")) { error in
            guard let dfaError = error as? DFAError else {
                XCTFail("Expected DFAError"); return
            }
            XCTAssertEqual(dfaError, .noTransition(state: "q0", event: "b"))
        }
    }

    func testHandleSelfLoops() throws {
        let dfa = try DFA(
            states: ["q0"],
            alphabet: ["a"],
            transitions: [DFA.TransitionRule(from: "q0", on: "a", to: "q0")],
            initial: "q0",
            accepting: ["q0"]
        )
        try dfa.process("a")
        XCTAssertEqual(dfa.currentState, "q0")
        try dfa.process("a")
        XCTAssertEqual(dfa.currentState, "q0")
    }

    func testFireActionsWithCorrectArguments() throws {
        // Use nonisolated(unsafe) for the mutable capture in the action closure.
        // This is safe because tests run single-threaded.
        nonisolated(unsafe) var log: [(String, String, String)] = []

        let dfa = try DFA(
            states: ["a", "b"],
            alphabet: ["x"],
            transitions: [
                DFA.TransitionRule(from: "a", on: "x", to: "b"),
                DFA.TransitionRule(from: "b", on: "x", to: "a"),
            ],
            initial: "a",
            accepting: [],
            actions: [
                transitionKey("a", "x"): { source, event, target in
                    log.append((source, event, target))
                },
            ]
        )
        try dfa.process("x")
        XCTAssertEqual(log.count, 1)
        XCTAssertEqual(log[0].0, "a")
        XCTAssertEqual(log[0].1, "x")
        XCTAssertEqual(log[0].2, "b")

        // Action only fires on (a, x), not (b, x)
        try dfa.process("x")
        XCTAssertEqual(log.count, 1)
    }
}

// ============================================================
// DFA Acceptance Tests
// ============================================================

final class DFAAcceptanceTests: XCTestCase {

    func testAcceptSequencesEndingInAcceptingState() throws {
        let t = try makeTurnstile()
        XCTAssertTrue(t.accepts(["coin"]))
        XCTAssertFalse(t.accepts(["coin", "push"]))
        XCTAssertTrue(t.accepts(["coin", "push", "coin"]))
    }

    func testHandleEmptyInputBasedOnInitialState() throws {
        let t = try makeTurnstile()
        XCTAssertFalse(t.accepts([]))  // locked is not accepting

        // DFA where initial IS accepting
        let dfa = try DFA(
            states: ["q0"],
            alphabet: ["a"],
            transitions: [DFA.TransitionRule(from: "q0", on: "a", to: "q0")],
            initial: "q0",
            accepting: ["q0"]
        )
        XCTAssertTrue(dfa.accepts([]))
    }

    func testDoNotModifyStateWhenCheckingAcceptance() throws {
        let t = try makeTurnstile()
        try t.process("coin")
        XCTAssertEqual(t.currentState, "unlocked")

        _ = t.accepts(["push", "push", "push"])
        XCTAssertEqual(t.currentState, "unlocked")  // unchanged
    }

    func testDoNotModifyTraceWhenCheckingAcceptance() throws {
        let t = try makeTurnstile()
        try t.process("coin")
        let traceLen = t.trace.count

        _ = t.accepts(["push", "coin"])
        XCTAssertEqual(t.trace.count, traceLen)  // unchanged
    }

    func testReturnFalseOnUndefinedTransition() throws {
        let dfa = try DFA(
            states: ["q0", "q1"],
            alphabet: ["a", "b"],
            transitions: [DFA.TransitionRule(from: "q0", on: "a", to: "q1")],
            initial: "q0",
            accepting: ["q1"]
        )
        XCTAssertTrue(dfa.accepts(["a"]))
        XCTAssertFalse(dfa.accepts(["b"]))  // no transition, graceful reject
    }

    func testReturnFalseOnInvalidEventInAccepts() throws {
        let t = try makeTurnstile()
        // In Swift version, accepts returns false for invalid events
        // (does not throw, matching the graceful behavior)
        XCTAssertFalse(t.accepts(["kick"]))
    }

    func testBinaryDivisibilityBy3() throws {
        let d = try makeDivBy3()
        // 0 = 0 (div by 3) — empty string starts in r0 which is accepting
        XCTAssertTrue(d.accepts([]))
        // 1 = 1 (not div by 3)
        XCTAssertFalse(d.accepts(["1"]))
        // 10 = 2
        XCTAssertFalse(d.accepts(["1", "0"]))
        // 11 = 3
        XCTAssertTrue(d.accepts(["1", "1"]))
        // 100 = 4
        XCTAssertFalse(d.accepts(["1", "0", "0"]))
        // 110 = 6
        XCTAssertTrue(d.accepts(["1", "1", "0"]))
        // 1001 = 9
        XCTAssertTrue(d.accepts(["1", "0", "0", "1"]))
        // 1100 = 12
        XCTAssertTrue(d.accepts(["1", "1", "0", "0"]))
        // 1111 = 15
        XCTAssertTrue(d.accepts(["1", "1", "1", "1"]))
        // 10000 = 16
        XCTAssertFalse(d.accepts(["1", "0", "0", "0", "0"]))
    }

    func testDivBy3AllNumbersUpTo31() throws {
        let d = try makeDivBy3()
        for n in 0..<32 {
            let expected = n % 3 == 0
            if n == 0 {
                XCTAssertTrue(d.accepts([]), "0 should be divisible by 3")
            } else {
                let binary = String(n, radix: 2)
                let bits = binary.map(String.init)
                XCTAssertEqual(
                    d.accepts(bits), expected,
                    "\(n) (binary: \(binary)) should\(expected ? "" : " not") be divisible by 3"
                )
            }
        }
    }
}

// ============================================================
// Branch Predictor Tests
// ============================================================

final class BranchPredictorDFATests: XCTestCase {

    func testStartInWNT() throws {
        let bp = try makeBranchPredictor()
        XCTAssertEqual(bp.currentState, "WNT")
    }

    func testWarmUpToStronglyTaken() throws {
        let bp = try makeBranchPredictor()
        try bp.process("taken")
        XCTAssertEqual(bp.currentState, "WT")
        try bp.process("taken")
        XCTAssertEqual(bp.currentState, "ST")
    }

    func testSaturateAtST() throws {
        let bp = try makeBranchPredictor()
        try bp.processSequence(["taken", "taken", "taken", "taken"])
        XCTAssertEqual(bp.currentState, "ST")
    }

    func testSaturateAtSNT() throws {
        let bp = try makeBranchPredictor()
        try bp.processSequence(["not_taken", "not_taken", "not_taken"])
        XCTAssertEqual(bp.currentState, "SNT")
    }

    func testExhibitHysteresis() throws {
        let bp = try makeBranchPredictor()
        try bp.processSequence(["taken", "taken"])
        XCTAssertEqual(bp.currentState, "ST")

        try bp.process("not_taken")
        XCTAssertEqual(bp.currentState, "WT")
        XCTAssertTrue(bp.accepting.contains("WT"))  // still predicts taken
    }

    func testHandleLoopPattern() throws {
        let bp = try makeBranchPredictor()
        let pattern = Array(repeating: "taken", count: 9) + ["not_taken"]
        try bp.processSequence(pattern)
        XCTAssertEqual(bp.currentState, "WT")
        XCTAssertTrue(bp.accepting.contains(bp.currentState))
    }

    func testPredictViaAcceptingStates() throws {
        let bp = try makeBranchPredictor()
        // WNT is not accepting (predicts not-taken)
        XCTAssertFalse(bp.accepting.contains(bp.currentState))

        // After one 'taken': WT is accepting (predicts taken)
        try bp.process("taken")
        XCTAssertTrue(bp.accepting.contains(bp.currentState))
    }
}

// ============================================================
// DFA Reset Tests
// ============================================================

final class DFAResetTests: XCTestCase {

    func testReturnToInitialState() throws {
        let t = try makeTurnstile()
        try t.process("coin")
        XCTAssertEqual(t.currentState, "unlocked")

        t.reset()
        XCTAssertEqual(t.currentState, "locked")
    }

    func testClearTrace() throws {
        let t = try makeTurnstile()
        try t.processSequence(["coin", "push", "coin"])
        XCTAssertEqual(t.trace.count, 3)

        t.reset()
        XCTAssertTrue(t.trace.isEmpty)
    }
}

// ============================================================
// DFA Introspection Tests
// ============================================================

final class DFAIntrospectionTests: XCTestCase {

    func testFindAllReachableStates() throws {
        let t = try makeTurnstile()
        XCTAssertEqual(t.reachableStates(), ["locked", "unlocked"])
    }

    func testExcludeUnreachableStates() throws {
        let dfa = try DFA(
            states: ["q0", "q1", "q_dead"],
            alphabet: ["a"],
            transitions: [
                DFA.TransitionRule(from: "q0", on: "a", to: "q1"),
                DFA.TransitionRule(from: "q1", on: "a", to: "q0"),
            ],
            initial: "q0",
            accepting: []
        )
        XCTAssertEqual(dfa.reachableStates(), ["q0", "q1"])
    }

    func testDetectCompleteDFA() throws {
        let t = try makeTurnstile()
        XCTAssertTrue(t.isComplete())
    }

    func testDetectIncompleteDFA() throws {
        let dfa = try DFA(
            states: ["q0", "q1"],
            alphabet: ["a", "b"],
            transitions: [DFA.TransitionRule(from: "q0", on: "a", to: "q1")],
            initial: "q0",
            accepting: []
        )
        XCTAssertFalse(dfa.isComplete())
    }

    func testValidateCleanDFA() throws {
        let t = try makeTurnstile()
        XCTAssertTrue(t.validate().isEmpty)
    }

    func testReportUnreachableStates() throws {
        let dfa = try DFA(
            states: ["q0", "q1", "q_dead"],
            alphabet: ["a"],
            transitions: [
                DFA.TransitionRule(from: "q0", on: "a", to: "q1"),
                DFA.TransitionRule(from: "q1", on: "a", to: "q0"),
                DFA.TransitionRule(from: "q_dead", on: "a", to: "q_dead"),
            ],
            initial: "q0",
            accepting: []
        )
        let warnings = dfa.validate()
        XCTAssertTrue(warnings.contains { $0.contains("Unreachable") })
        XCTAssertTrue(warnings.contains { $0.contains("q_dead") })
    }

    func testReportUnreachableAcceptingStates() throws {
        let dfa = try DFA(
            states: ["q0", "q_dead"],
            alphabet: ["a"],
            transitions: [
                DFA.TransitionRule(from: "q0", on: "a", to: "q0"),
                DFA.TransitionRule(from: "q_dead", on: "a", to: "q_dead"),
            ],
            initial: "q0",
            accepting: ["q_dead"]
        )
        let warnings = dfa.validate()
        XCTAssertTrue(warnings.contains { $0.contains("Unreachable accepting") })
    }

    func testReportMissingTransitions() throws {
        let dfa = try DFA(
            states: ["q0", "q1"],
            alphabet: ["a", "b"],
            transitions: [DFA.TransitionRule(from: "q0", on: "a", to: "q1")],
            initial: "q0",
            accepting: []
        )
        let warnings = dfa.validate()
        XCTAssertTrue(warnings.contains { $0.contains("Missing transitions") })
    }
}

// ============================================================
// DFA Edge Cases
// ============================================================

final class DFAEdgeCaseTests: XCTestCase {

    func testSingleStateSelfLoop() throws {
        let dfa = try DFA(
            states: ["q0"],
            alphabet: ["a"],
            transitions: [DFA.TransitionRule(from: "q0", on: "a", to: "q0")],
            initial: "q0",
            accepting: ["q0"]
        )
        XCTAssertTrue(dfa.accepts(["a", "a", "a"]))
        XCTAssertTrue(dfa.accepts([]))
    }

    func testLargeAlphabet() throws {
        let alphabet = Set((97...122).map { String(UnicodeScalar($0)) })
        var transitions: [DFA.TransitionRule] = []
        for c in alphabet {
            transitions.append(DFA.TransitionRule(from: "q0", on: c, to: "q1"))
            transitions.append(DFA.TransitionRule(from: "q1", on: c, to: "q0"))
        }
        let dfa = try DFA(
            states: ["q0", "q1"],
            alphabet: alphabet,
            transitions: transitions,
            initial: "q0",
            accepting: ["q1"]
        )
        XCTAssertTrue(dfa.accepts(["a"]))
        XCTAssertFalse(dfa.accepts(["a", "b"]))
        XCTAssertTrue(dfa.accepts(["x", "y", "z"]))
    }

    func testTraceReturnsCopy() throws {
        let t = try makeTurnstile()
        try t.process("coin")
        let t1 = t.trace
        let t2 = t.trace
        XCTAssertEqual(t1, t2)
    }
}

// ============================================================
// NFA Construction Tests
// ============================================================

final class NFAConstructionTests: XCTestCase {

    func testConstructValidNFA() throws {
        let nfa = try makeContainsAb()
        XCTAssertEqual(nfa.states, ["q0", "q1", "q2"])
        XCTAssertEqual(nfa.alphabet, ["a", "b"])
        XCTAssertEqual(nfa.initial, "q0")
        XCTAssertEqual(nfa.accepting, ["q2"])
    }

    func testRejectEmptyStates() {
        XCTAssertThrowsError(
            try NFA(
                states: [],
                alphabet: ["a"],
                transitions: [],
                initial: "q0",
                accepting: []
            )
        ) { error in
            guard let nfaError = error as? NFAError else {
                XCTFail("Expected NFAError"); return
            }
            XCTAssertEqual(nfaError, .emptyStates)
        }
    }

    func testRejectEpsilonInAlphabet() {
        XCTAssertThrowsError(
            try NFA(
                states: ["q0"],
                alphabet: ["a", ""],
                transitions: [],
                initial: "q0",
                accepting: []
            )
        ) { error in
            guard let nfaError = error as? NFAError else {
                XCTFail("Expected NFAError"); return
            }
            XCTAssertEqual(nfaError, .epsilonInAlphabet)
        }
    }

    func testRejectInitialNotInStates() {
        XCTAssertThrowsError(
            try NFA(
                states: ["q0"],
                alphabet: ["a"],
                transitions: [],
                initial: "q_bad",
                accepting: []
            )
        ) { error in
            guard let nfaError = error as? NFAError else {
                XCTFail("Expected NFAError"); return
            }
            XCTAssertEqual(nfaError, .initialStateNotInStates(state: "q_bad"))
        }
    }

    func testRejectAcceptingNotSubset() {
        XCTAssertThrowsError(
            try NFA(
                states: ["q0"],
                alphabet: ["a"],
                transitions: [],
                initial: "q0",
                accepting: ["q_bad"]
            )
        ) { error in
            guard let nfaError = error as? NFAError else {
                XCTFail("Expected NFAError"); return
            }
            XCTAssertEqual(nfaError, .acceptingStateNotInStates(state: "q_bad"))
        }
    }

    func testRejectTransitionSourceNotInStates() {
        XCTAssertThrowsError(
            try NFA(
                states: ["q0"],
                alphabet: ["a"],
                transitions: [NFATransitionRule(from: "q_bad", on: "a", to: ["q0"])],
                initial: "q0",
                accepting: []
            )
        ) { error in
            guard let nfaError = error as? NFAError else {
                XCTFail("Expected NFAError"); return
            }
            XCTAssertEqual(nfaError, .transitionSourceNotInStates(source: "q_bad"))
        }
    }

    func testRejectTransitionEventNotInAlphabetAndNotEpsilon() {
        XCTAssertThrowsError(
            try NFA(
                states: ["q0"],
                alphabet: ["a"],
                transitions: [NFATransitionRule(from: "q0", on: "z", to: ["q0"])],
                initial: "q0",
                accepting: []
            )
        ) { error in
            guard let nfaError = error as? NFAError else {
                XCTFail("Expected NFAError"); return
            }
            XCTAssertEqual(nfaError, .transitionEventNotInAlphabet(event: "z"))
        }
    }

    func testRejectTransitionTargetNotInStates() {
        XCTAssertThrowsError(
            try NFA(
                states: ["q0"],
                alphabet: ["a"],
                transitions: [NFATransitionRule(from: "q0", on: "a", to: ["q_bad"])],
                initial: "q0",
                accepting: []
            )
        ) { error in
            guard let nfaError = error as? NFAError else {
                XCTFail("Expected NFAError"); return
            }
            XCTAssertEqual(
                nfaError,
                .transitionTargetNotInStates(target: "q_bad", source: "q0", event: "a")
            )
        }
    }
}

// ============================================================
// NFA Epsilon Closure Tests
// ============================================================

final class NFAEpsilonClosureTests: XCTestCase {

    func testReturnInputSetWhenNoEpsilonTransitions() throws {
        let nfa = try makeContainsAb()
        XCTAssertEqual(nfa.epsilonClosure(["q0"]), ["q0"])
    }

    func testFollowSingleEpsilonTransition() throws {
        let nfa = try NFA(
            states: ["q0", "q1"],
            alphabet: ["a"],
            transitions: [NFATransitionRule(from: "q0", on: EPSILON, to: ["q1"])],
            initial: "q0",
            accepting: []
        )
        XCTAssertEqual(nfa.epsilonClosure(["q0"]), ["q0", "q1"])
    }

    func testFollowChainedEpsilons() throws {
        let nfa = try makeEpsilonChain()
        XCTAssertEqual(nfa.epsilonClosure(["q0"]), ["q0", "q1", "q2"])
    }

    func testHandleEpsilonCyclesWithoutInfiniteLoop() throws {
        let nfa = try NFA(
            states: ["q0", "q1"],
            alphabet: ["a"],
            transitions: [
                NFATransitionRule(from: "q0", on: EPSILON, to: ["q1"]),
                NFATransitionRule(from: "q1", on: EPSILON, to: ["q0"]),
            ],
            initial: "q0",
            accepting: []
        )
        XCTAssertEqual(nfa.epsilonClosure(["q0"]), ["q0", "q1"])
    }

    func testFollowBranchingEpsilons() throws {
        let nfa = try makeAOrAb()
        XCTAssertEqual(nfa.epsilonClosure(["q0"]), ["q0", "q1", "q3"])
    }

    func testComputeClosureForMultipleStates() throws {
        let nfa = try makeEpsilonChain()
        let result = nfa.epsilonClosure(["q0", "q3"])
        XCTAssertEqual(result, ["q0", "q1", "q2", "q3"])
    }

    func testReturnEmptySetForEmptyInput() throws {
        let nfa = try makeEpsilonChain()
        XCTAssertEqual(nfa.epsilonClosure([]), Set<String>())
    }
}

// ============================================================
// NFA Processing Tests
// ============================================================

final class NFAProcessingTests: XCTestCase {

    func testStartInEpsilonClosureOfInitialState() throws {
        let nfa = try makeEpsilonChain()
        XCTAssertEqual(nfa.currentStates, ["q0", "q1", "q2"])
    }

    func testProcessDeterministicCase() throws {
        let nfa = try makeContainsAb()
        try nfa.process("b")
        XCTAssertEqual(nfa.currentStates, ["q0"])
    }

    func testHandleNonDeterministicBranching() throws {
        let nfa = try makeContainsAb()
        try nfa.process("a")
        XCTAssertEqual(nfa.currentStates, ["q0", "q1"])
    }

    func testLetDeadPathsVanish() throws {
        let nfa = try makeContainsAb()
        try nfa.process("a")  // {q0, q1}
        try nfa.process("a")  // q0->{q0,q1}, q1 has no 'a' -> dies
        XCTAssertEqual(nfa.currentStates, ["q0", "q1"])
    }

    func testReachAcceptingState() throws {
        let nfa = try makeContainsAb()
        try nfa.process("a")
        try nfa.process("b")
        XCTAssertTrue(nfa.currentStates.contains("q2"))
    }

    func testProcessThroughEpsilonChain() throws {
        let nfa = try makeEpsilonChain()
        try nfa.process("a")
        XCTAssertEqual(nfa.currentStates, ["q3"])
    }

    func testThrowOnInvalidEvent() throws {
        let nfa = try makeContainsAb()
        XCTAssertThrowsError(try nfa.process("c")) { error in
            guard let nfaError = error as? NFAError else {
                XCTFail("Expected NFAError"); return
            }
            XCTAssertEqual(nfaError, .eventNotInAlphabet(event: "c"))
        }
    }

    func testReturnTraceFromProcessSequence() throws {
        let nfa = try makeContainsAb()
        let trace = try nfa.processSequence(["a", "b"])
        XCTAssertEqual(trace.count, 2)

        let (before, event, after) = trace[0]
        XCTAssertEqual(event, "a")
        XCTAssertTrue(before.contains("q0"))
        XCTAssertTrue(after.contains("q1"))

        let (_, event2, after2) = trace[1]
        XCTAssertEqual(event2, "b")
        XCTAssertTrue(after2.contains("q2"))
    }
}

// ============================================================
// NFA Acceptance Tests
// ============================================================

final class NFAAcceptanceTests: XCTestCase {

    func testAcceptStringsContainingAb() throws {
        let nfa = try makeContainsAb()
        XCTAssertTrue(nfa.accepts(["a", "b"]))
        XCTAssertTrue(nfa.accepts(["b", "a", "b"]))
        XCTAssertTrue(nfa.accepts(["a", "a", "b"]))
        XCTAssertTrue(nfa.accepts(["a", "b", "a", "b"]))
    }

    func testRejectStringsNotContainingAb() throws {
        let nfa = try makeContainsAb()
        XCTAssertFalse(nfa.accepts(["a"]))
        XCTAssertFalse(nfa.accepts(["b"]))
        XCTAssertFalse(nfa.accepts(["b", "a"]))
        XCTAssertFalse(nfa.accepts(["b", "b", "b"]))
        XCTAssertFalse(nfa.accepts([]))
    }

    func testAcceptAAndAbWithEpsilonNFA() throws {
        let nfa = try makeAOrAb()
        XCTAssertTrue(nfa.accepts(["a"]))
        XCTAssertTrue(nfa.accepts(["a", "b"]))
    }

    func testRejectInvalidStringsForAOrAbNFA() throws {
        let nfa = try makeAOrAb()
        XCTAssertFalse(nfa.accepts([]))
        XCTAssertFalse(nfa.accepts(["b"]))
        XCTAssertFalse(nfa.accepts(["a", "a"]))
        XCTAssertFalse(nfa.accepts(["a", "b", "a"]))
    }

    func testAcceptSingleAViaEpsilonChain() throws {
        let nfa = try makeEpsilonChain()
        XCTAssertTrue(nfa.accepts(["a"]))
    }

    func testRejectEmptyAndMultiCharForEpsilonChain() throws {
        let nfa = try makeEpsilonChain()
        XCTAssertFalse(nfa.accepts([]))
        XCTAssertFalse(nfa.accepts(["a", "a"]))
    }

    func testDoNotModifyStateWhenCheckingAcceptance() throws {
        let nfa = try makeContainsAb()
        let original = nfa.currentStates
        _ = nfa.accepts(["a", "b", "a"])
        XCTAssertEqual(nfa.currentStates, original)
    }

    func testRejectEarlyWhenNFAReachesEmptyStateSet() throws {
        let nfa = try NFA(
            states: ["q0", "q1"],
            alphabet: ["a", "b"],
            transitions: [NFATransitionRule(from: "q0", on: "a", to: ["q1"])],
            initial: "q0",
            accepting: ["q1"]
        )
        XCTAssertFalse(nfa.accepts(["b"]))
        XCTAssertFalse(nfa.accepts(["b", "a"]))
    }
}

// ============================================================
// Subset Construction Tests (NFA -> DFA)
// ============================================================

final class SubsetConstructionTests: XCTestCase {

    func testConvertDeterministicNFACleanly() throws {
        let nfa = try NFA(
            states: ["q0", "q1"],
            alphabet: ["a", "b"],
            transitions: [
                NFATransitionRule(from: "q0", on: "a", to: ["q1"]),
                NFATransitionRule(from: "q0", on: "b", to: ["q0"]),
                NFATransitionRule(from: "q1", on: "a", to: ["q0"]),
                NFATransitionRule(from: "q1", on: "b", to: ["q1"]),
            ],
            initial: "q0",
            accepting: ["q1"]
        )
        let dfa = try nfa.toDfa()
        XCTAssertEqual(dfa.states.count, 2)
        XCTAssertTrue(dfa.accepts(["a"]))
        XCTAssertFalse(dfa.accepts(["a", "a"]))
        XCTAssertTrue(dfa.accepts(["a", "b"]))
    }

    func testConvertContainsAbNFAToEquivalentDFA() throws {
        let nfa = try makeContainsAb()
        let dfa = try nfa.toDfa()

        let testCases: [([String], Bool)] = [
            (["a", "b"], true),
            (["b", "a", "b"], true),
            (["a", "a", "b"], true),
            (["a"], false),
            (["b"], false),
            (["b", "a"], false),
            ([], false),
        ]
        for (events, expected) in testCases {
            XCTAssertEqual(
                dfa.accepts(events), expected,
                "DFA should\(expected ? "" : " not") accept \(events)"
            )
        }
    }

    func testConvertEpsilonNFACorrectly() throws {
        let nfa = try makeAOrAb()
        let dfa = try nfa.toDfa()

        XCTAssertTrue(dfa.accepts(["a"]))
        XCTAssertTrue(dfa.accepts(["a", "b"]))
        XCTAssertFalse(dfa.accepts([]))
        XCTAssertFalse(dfa.accepts(["b"]))
        XCTAssertFalse(dfa.accepts(["a", "a"]))
    }

    func testConvertEpsilonChainNFACorrectly() throws {
        let nfa = try makeEpsilonChain()
        let dfa = try nfa.toDfa()

        XCTAssertTrue(dfa.accepts(["a"]))
        XCTAssertFalse(dfa.accepts([]))
        XCTAssertFalse(dfa.accepts(["a", "a"]))
    }

    func testProduceValidDFAFromConversion() throws {
        let nfa = try makeContainsAb()
        let dfa = try nfa.toDfa()
        let warnings = dfa.validate()
        for w in warnings {
            XCTAssertFalse(w.contains("Unreachable"), "Converted DFA should have no unreachable states")
        }
    }

    func testLanguageEquivalenceForAllStringsUpToLength4() throws {
        // NFA for "ends with 'ab'"
        let nfa = try NFA(
            states: ["q0", "q1", "q2"],
            alphabet: ["a", "b"],
            transitions: [
                NFATransitionRule(from: "q0", on: "a", to: ["q0", "q1"]),
                NFATransitionRule(from: "q0", on: "b", to: ["q0"]),
                NFATransitionRule(from: "q1", on: "b", to: ["q2"]),
            ],
            initial: "q0",
            accepting: ["q2"]
        )
        let dfa = try nfa.toDfa()

        // Generate all strings of a,b up to length 4
        func genStrings(_ alpha: [String], maxLen: Int) -> [[String]] {
            var result: [[String]] = [[]]
            for len in 1...maxLen {
                let newStrs = result
                    .filter { $0.count == len - 1 }
                    .flatMap { s in alpha.map { c in s + [c] } }
                result.append(contentsOf: newStrs)
            }
            return result
        }

        for s in genStrings(["a", "b"], maxLen: 4) {
            XCTAssertEqual(
                nfa.accepts(s), dfa.accepts(s),
                "NFA and DFA should agree on \(s)"
            )
        }
    }
}

// ============================================================
// NFA Reset Tests
// ============================================================

final class NFAResetTests: XCTestCase {

    func testReturnToEpsilonClosureOfInitial() throws {
        let nfa = try makeContainsAb()
        try nfa.process("a")
        XCTAssertTrue(nfa.currentStates.contains("q1"))

        nfa.reset()
        XCTAssertEqual(nfa.currentStates, ["q0"])
    }

    func testRecomputeEpsilonClosureOnReset() throws {
        let nfa = try makeEpsilonChain()
        try nfa.process("a")
        XCTAssertEqual(nfa.currentStates, ["q3"])

        nfa.reset()
        XCTAssertEqual(nfa.currentStates, ["q0", "q1", "q2"])
    }
}

// ============================================================
// StateSetName Tests
// ============================================================

final class StateSetNameTests: XCTestCase {

    func testDeterministicNaming() {
        let name = stateSetName(["q2", "q0", "q1"])
        XCTAssertEqual(name, "{q0,q1,q2}")
    }

    func testSingleState() {
        XCTAssertEqual(stateSetName(["q0"]), "{q0}")
    }

    func testEmptySet() {
        XCTAssertEqual(stateSetName([]), "{}")
    }
}

// ============================================================
// TransitionKey Tests
// ============================================================

final class TransitionKeyTests: XCTestCase {

    func testBasicKey() {
        let key = transitionKey("locked", "coin")
        XCTAssertEqual(key, "locked\0coin")
    }

    func testKeyUniqueness() {
        let k1 = transitionKey("q0", "a")
        let k2 = transitionKey("q0", "b")
        let k3 = transitionKey("q1", "a")
        XCTAssertNotEqual(k1, k2)
        XCTAssertNotEqual(k1, k3)
    }
}
