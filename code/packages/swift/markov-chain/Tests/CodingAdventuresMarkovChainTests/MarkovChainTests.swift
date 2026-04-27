// ============================================================================
// MarkovChainTests.swift — XCTest suite for CodingAdventuresMarkovChain
// ============================================================================
//
// This test file covers all 10 spec test cases from DT28, plus additional
// edge-case tests. Each test is documented with the expected behaviour and
// the reasoning behind it.
//
// Test map (spec cases):
//   1.  testConstruction             — empty chain, 0 states
//   2.  testTrainSinglePair          — A→B: probability = 1.0
//   3.  testTrainSequence            — frequency ratios from long sequence
//   4.  testLaplaceSmoothing         — smoothed P(A→C) = 0.25 with 3 states
//   5.  testGenerateLength           — generate returns exactly 10 states
//   6.  testGenerateString           — char-chain starts with seed "th"
//   7.  testStationaryDistributionSumsToOne — ∑π = 1.0
//   8.  testOrder2Chain              — "ab"→"c" is 1.0; regenerates "abcabcabc"
//   9.  testUnknownStateThrows       — nextState("UNKNOWN") throws
//   10. testMultiTrainAccumulation   — two trains accumulate counts
//
// ============================================================================

import XCTest
@testable import CodingAdventuresMarkovChain

final class MarkovChainTests: XCTestCase {

    // -----------------------------------------------------------------------
    // Test 1 — Construction
    // -----------------------------------------------------------------------
    // Spec: `MarkovChain.new()` creates an empty chain with 0 states.
    //
    // An empty chain has no states and no transitions. This verifies that the
    // constructor doesn't pre-populate any data.
    func testConstruction() {
        let chain = MarkovChain()
        XCTAssertEqual(chain.states().count, 0, "Empty chain should have 0 states")
        XCTAssertTrue(chain.transitionMatrix().isEmpty, "Empty chain should have empty transition matrix")
    }

    // -----------------------------------------------------------------------
    // Test 2 — Train Single Pair
    // -----------------------------------------------------------------------
    // Spec: train on [A, B] (order=1). probability(A, B) == 1.0
    //
    // When there's only one observed transition from A, it must be taken with
    // probability 1.0 — no other choice exists.
    func testTrainSinglePair() {
        var chain = MarkovChain(order: 1, smoothing: 0.0)
        chain.train(["A", "B"])

        let prob = chain.probability(from: "A", to: "B")
        XCTAssertEqual(prob, 1.0, accuracy: 1e-9,
            "With one training pair A→B, P(A→B) must be 1.0")

        // There are only 2 states: A and B.
        XCTAssertEqual(chain.states().count, 2)
    }

    // -----------------------------------------------------------------------
    // Test 3 — Train Sequence: Frequency Ratios
    // -----------------------------------------------------------------------
    // Spec: train on [A, B, A, C, A, B, B, A].
    //   probability(A, B) ≈ 0.667
    //   probability(A, C) ≈ 0.333
    //   probability(B, A) ≈ 0.667
    //   probability(B, B) ≈ 0.333
    //
    // Let's count manually:
    //   Sequence:  A B A C A B B A
    //   Transitions from A: A→B (idx0), A→C (idx2), A→B (idx4)  → B:2, C:1
    //   Transitions from B: B→A (idx1), B→A (idx5 wraps), B→B (idx6) → A:2, B:1
    //   Transitions from C: C→A (idx3)
    //
    //   P(A→B) = 2/3 ≈ 0.6667
    //   P(A→C) = 1/3 ≈ 0.3333
    //   P(B→A) = 2/3 ≈ 0.6667
    //   P(B→B) = 1/3 ≈ 0.3333
    func testTrainSequence() {
        var chain = MarkovChain(order: 1, smoothing: 0.0)
        chain.train(["A", "B", "A", "C", "A", "B", "B", "A"])

        XCTAssertEqual(chain.probability(from: "A", to: "B"), 2.0/3.0, accuracy: 1e-6,
            "P(A→B) should be 2/3")
        XCTAssertEqual(chain.probability(from: "A", to: "C"), 1.0/3.0, accuracy: 1e-6,
            "P(A→C) should be 1/3")
        XCTAssertEqual(chain.probability(from: "B", to: "A"), 2.0/3.0, accuracy: 1e-6,
            "P(B→A) should be 2/3")
        XCTAssertEqual(chain.probability(from: "B", to: "B"), 1.0/3.0, accuracy: 1e-6,
            "P(B→B) should be 1/3")
    }

    // -----------------------------------------------------------------------
    // Test 4 — Laplace Smoothing
    // -----------------------------------------------------------------------
    // Spec: MarkovChain(order: 1, smoothing: 1.0, states: ["A","B","C"])
    //       train(["A","B"])
    //       probability(from: "A", to: "C") == 0.25
    //
    // With smoothing=1.0 and 3 pre-declared states:
    //   Raw counts from A: {B: 1}
    //   Smoothed counts:   {A: 0+1=1, B: 1+1=2, C: 0+1=1}
    //   Total smoothed:    1 + 2 + 1 = 4
    //   P(A→C) = 1/4 = 0.25
    //
    // Laplace smoothing prevents zero-probability transitions by giving every
    // unseen (state, target) pair a pseudo-count of α. This is crucial for
    // generation: without it, the chain could get "stuck" with no outgoing
    // transitions from a state that appears at the end of the training sequence.
    func testLaplaceSmoothing() {
        var chain = MarkovChain(order: 1, smoothing: 1.0, states: ["A", "B", "C"])
        chain.train(["A", "B"])

        let prob = chain.probability(from: "A", to: "C")
        XCTAssertEqual(prob, 0.25, accuracy: 1e-9,
            "With Laplace smoothing and 3 states, P(A→C) should be 1/4")

        // Also verify P(A→B) = 2/4 = 0.5 and P(A→A) = 1/4 = 0.25
        XCTAssertEqual(chain.probability(from: "A", to: "B"), 0.5, accuracy: 1e-9,
            "P(A→B) with Laplace should be 2/4 = 0.5")
        XCTAssertEqual(chain.probability(from: "A", to: "A"), 0.25, accuracy: 1e-9,
            "P(A→A) with Laplace should be 1/4 = 0.25")
    }

    // -----------------------------------------------------------------------
    // Test 5 — Generate Length
    // -----------------------------------------------------------------------
    // Spec: generate(A, 10) returns a list of exactly 10 states.
    //
    // The length contract must hold regardless of which states are sampled.
    // We train a simple chain first so that "A" has known transitions.
    func testGenerateLength() throws {
        var chain = MarkovChain(order: 1, smoothing: 1.0)
        chain.train(["A", "B", "C", "A", "B", "A"])

        let sequence = try chain.generate(start: "A", length: 10)
        XCTAssertEqual(sequence.count, 10,
            "generate(start:length:) must return exactly `length` states")
        XCTAssertEqual(sequence[0], "A",
            "The first element must be the start state")
    }

    // -----------------------------------------------------------------------
    // Test 6 — Generate String
    // -----------------------------------------------------------------------
    // Spec: generate_string("th", 50) on a character chain trained on English
    //       text returns a 50-char string starting with "th".
    //
    // We train on a repeated phrase to ensure "th" is a known context,
    // then verify length and prefix.
    func testGenerateString() throws {
        var chain = MarkovChain(order: 2, smoothing: 1.0)
        let corpus = String(repeating: "the quick brown fox jumps over the lazy dog ", count: 10)
        chain.trainString(corpus)

        let result = try chain.generateString(seed: "th", length: 50)
        XCTAssertEqual(result.count, 50,
            "generateString should return exactly `length` characters")
        XCTAssertTrue(result.hasPrefix("th"),
            "Output must start with the seed characters")
    }

    // -----------------------------------------------------------------------
    // Test 7 — Stationary Distribution Sums to 1
    // -----------------------------------------------------------------------
    // Spec: for any ergodic chain, sum(stationary_distribution().values) ≈ 1.0
    //
    // We create a 3-state ergodic chain (every state reachable from every
    // other) and confirm that the power-iteration result sums to 1.0.
    //
    // An ergodic chain has a unique stationary distribution π where π·T = π.
    // Power iteration computes it by repeatedly multiplying an initial
    // uniform vector by T until the vector stops changing.
    func testStationaryDistributionSumsToOne() throws {
        var chain = MarkovChain(order: 1, smoothing: 0.1)
        // Train a chain that visits all 3 states, ensuring ergodicity.
        chain.train(["A", "B", "C", "A", "B", "A", "C", "B", "C", "A"])

        let pi = try chain.stationaryDistribution()
        let total = pi.values.reduce(0, +)
        XCTAssertEqual(total, 1.0, accuracy: 1e-6,
            "Stationary distribution must sum to 1.0")
        XCTAssertEqual(pi.count, chain.states().count,
            "Distribution must have an entry for every state")
    }

    // -----------------------------------------------------------------------
    // Test 8 — Order-2 Chain
    // -----------------------------------------------------------------------
    // Spec: train on Array("abcabcabc").map{String($0)} with order=2.
    //       The context "ab" should transition to 'c' with probability 1.0.
    //       generateString(seed:"ab", length:9) == "abcabcabc"
    //
    // With order=2, the context "ab" (stored as "a\0b") only ever leads to
    // "c" in the training data "abcabcabc". So P(ab→c) = 1.0.
    //
    // The generation then deterministically reproduces the pattern: starting
    // with "ab", we always go to c, then context="bc"→a, then "ca"→b, etc.
    func testOrder2Chain() throws {
        var chain = MarkovChain(order: 2, smoothing: 0.0)
        chain.train(Array("abcabcabc").map { String($0) })

        let result = try chain.generateString(seed: "ab", length: 9)
        XCTAssertEqual(result, "abcabcabc",
            "Order-2 chain on 'abcabcabc' must regenerate the same sequence")
    }

    // -----------------------------------------------------------------------
    // Test 9 — Unknown State Throws
    // -----------------------------------------------------------------------
    // Spec: calling next_state on an unseen state raises an error.
    //
    // The chain should never silently return a random result for a state it
    // has never seen. Throwing an error lets the caller decide how to handle
    // the situation (fall back to a default, signal an error, etc.).
    func testUnknownStateThrows() {
        var chain = MarkovChain(order: 1, smoothing: 0.0)
        chain.train(["A", "B"])

        XCTAssertThrowsError(try chain.nextState("UNKNOWN")) { error in
            guard case MarkovError.unknownState(let state) = error else {
                XCTFail("Expected MarkovError.unknownState, got \(error)")
                return
            }
            XCTAssertEqual(state, "UNKNOWN",
                "Error should name the unknown state")
        }
    }

    // -----------------------------------------------------------------------
    // Test 10 — Multi-Train Accumulation
    // -----------------------------------------------------------------------
    // Spec: calling `train` twice accumulates counts before re-normalising,
    //       so probabilities reflect the combined training data.
    //
    // If we train twice on [A, B] (total: A→B appears 2 times) versus once
    // on [A, B, A, C] (A→B once, A→C once), the probabilities should differ.
    //
    // Here we verify that:
    //   • Training twice on [A, B] gives P(A→B) = 1.0 (only A→B seen)
    //   • Training once on [A, B] then once on [A, C] gives P(A→B) ≈ P(A→C) ≈ 0.5
    func testMultiTrainAccumulation() {
        // First scenario: train twice on [A, B]
        var chain1 = MarkovChain(order: 1, smoothing: 0.0)
        chain1.train(["A", "B"])
        chain1.train(["A", "B"])
        XCTAssertEqual(chain1.probability(from: "A", to: "B"), 1.0, accuracy: 1e-9,
            "After two trains of A→B only, P(A→B) should still be 1.0")

        // Second scenario: train on [A, B] then [A, C]
        var chain2 = MarkovChain(order: 1, smoothing: 0.0)
        chain2.train(["A", "B"])
        chain2.train(["A", "C"])
        // Now A→B: 1 time, A→C: 1 time → both 0.5
        XCTAssertEqual(chain2.probability(from: "A", to: "B"), 0.5, accuracy: 1e-9,
            "After training A→B and A→C once each, P(A→B) should be 0.5")
        XCTAssertEqual(chain2.probability(from: "A", to: "C"), 0.5, accuracy: 1e-9,
            "After training A→B and A→C once each, P(A→C) should be 0.5")
    }

    // -----------------------------------------------------------------------
    // Additional: Probability Rows Sum to 1
    // -----------------------------------------------------------------------
    // Each row of the transition matrix must sum to 1.0 (it's a probability
    // distribution over next states). This invariant must hold after training.
    func testProbabilityRowsSumToOne() {
        var chain = MarkovChain(order: 1, smoothing: 0.5)
        chain.train(["A", "B", "C", "A", "B", "B", "C", "A"])

        for (_, row) in chain.transitionMatrix() {
            let rowSum = row.values.reduce(0, +)
            XCTAssertEqual(rowSum, 1.0, accuracy: 1e-9,
                "Each transition row must sum to 1.0")
        }
    }

    // -----------------------------------------------------------------------
    // Additional: Seed Too Short Throws
    // -----------------------------------------------------------------------
    // generateString with a seed shorter than `order` must throw seedTooShort.
    func testSeedTooShortThrows() {
        var chain = MarkovChain(order: 3, smoothing: 1.0)
        chain.trainString("abcabcabcabc")

        XCTAssertThrowsError(try chain.generateString(seed: "ab", length: 10)) { error in
            guard case MarkovError.seedTooShort(let required, let got) = error else {
                XCTFail("Expected MarkovError.seedTooShort, got \(error)")
                return
            }
            XCTAssertEqual(required, 3, "Required depth should be the chain's order")
            XCTAssertEqual(got, 2, "Got should reflect the seed length")
        }
    }

    // -----------------------------------------------------------------------
    // Additional: Pre-declared States Appear in states()
    // -----------------------------------------------------------------------
    // States declared in the constructor appear in states() even before training.
    func testPreDeclaredStates() {
        let chain = MarkovChain(order: 1, smoothing: 0.0, states: ["X", "Y", "Z"])
        XCTAssertEqual(chain.states(), ["X", "Y", "Z"],
            "Pre-declared states must appear in states() before training")
    }

    // -----------------------------------------------------------------------
    // Additional: Zero-Length Generate
    // -----------------------------------------------------------------------
    func testZeroLengthGenerate() throws {
        var chain = MarkovChain()
        chain.train(["A", "B"])
        let result = try chain.generate(start: "A", length: 0)
        XCTAssertEqual(result.count, 0, "generate with length 0 must return empty array")
    }
}
